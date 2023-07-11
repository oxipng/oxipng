use crate::colors::{BitDepth, ColorType};
use crate::deflate;
use crate::error::PngError;
use crate::filters::*;
use crate::headers::*;
use crate::interlace::{deinterlace_image, interlace_image, Interlacing};
use crate::Options;
use bitvec::bitarr;
use libdeflater::{CompressionLvl, Compressor};
use log::warn;
use rgb::ComponentSlice;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::iter::Iterator;
use std::path::Path;
use std::sync::Arc;

pub(crate) mod scan_lines;

use self::scan_lines::ScanLines;

/// Compression level to use for the Brute filter strategy
const BRUTE_LEVEL: i32 = 1; // 1 is fastest, 2-4 are not useful, 5 is slower but more effective
/// Number of lines to compress with the Brute filter strategy
const BRUTE_LINES: usize = 4; // Values over 8 are generally not useful

#[derive(Debug, Clone)]
pub struct PngImage {
    /// The headers stored in the IHDR chunk
    pub ihdr: IhdrData,
    /// The uncompressed, unfiltered data from the IDAT chunk
    pub data: Vec<u8>,
}

/// Contains all data relevant to a PNG image
#[derive(Debug, Clone)]
pub struct PngData {
    /// Uncompressed image data
    pub raw: Arc<PngImage>,
    /// The filtered and compressed data of the IDAT chunk
    pub idat_data: Vec<u8>,
    /// All non-critical chunks from the PNG are stored here
    pub aux_chunks: Vec<Chunk>,
}

impl PngData {
    /// Create a new `PngData` struct by opening a file
    #[inline]
    pub fn new(filepath: &Path, opts: &Options) -> Result<Self, PngError> {
        let byte_data = Self::read_file(filepath)?;

        Self::from_slice(&byte_data, opts)
    }

    pub fn read_file(filepath: &Path) -> Result<Vec<u8>, PngError> {
        let file = match File::open(filepath) {
            Ok(f) => f,
            Err(_) => return Err(PngError::new("Failed to open file for reading")),
        };
        let file_len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
        let mut reader = BufReader::new(file);
        // Check file for PNG header
        let mut header = [0; 8];
        if reader.read_exact(&mut header).is_err() {
            return Err(PngError::new("Not a PNG file: too small"));
        }
        if !file_header_is_valid(&header) {
            return Err(PngError::new("Invalid PNG header detected"));
        }
        // Read raw png data into memory
        let mut byte_data: Vec<u8> = Vec::with_capacity(file_len);
        byte_data.extend_from_slice(&header);
        match reader.read_to_end(&mut byte_data) {
            Ok(_) => (),
            Err(_) => return Err(PngError::new("Failed to read from file")),
        }
        Ok(byte_data)
    }

    /// Create a new `PngData` struct by reading a slice
    pub fn from_slice(byte_data: &[u8], opts: &Options) -> Result<Self, PngError> {
        let mut byte_offset: usize = 0;
        // Test that png header is valid
        let header = byte_data.get(0..8).ok_or(PngError::TruncatedData)?;
        if !file_header_is_valid(header) {
            return Err(PngError::NotPNG);
        }
        byte_offset += 8;

        // Read the data chunks
        let mut idat_data: Vec<u8> = Vec::new();
        let mut key_chunks: FxHashMap<[u8; 4], Vec<u8>> = FxHashMap::default();
        let mut aux_chunks: Vec<Chunk> = Vec::new();
        while let Some(chunk) = parse_next_chunk(byte_data, &mut byte_offset, opts.fix_errors)? {
            match &chunk.name {
                b"IDAT" => {
                    if idat_data.is_empty() {
                        // Keep track of where the first IDAT sits relative to other chunks
                        aux_chunks.push(Chunk {
                            name: chunk.name,
                            data: Vec::new(),
                        })
                    }
                    idat_data.extend_from_slice(chunk.data);
                }
                b"IHDR" | b"PLTE" | b"tRNS" => {
                    key_chunks.insert(chunk.name, chunk.data.to_owned());
                }
                _ => {
                    if opts.strip.keep(&chunk.name) {
                        aux_chunks.push(Chunk {
                            name: chunk.name,
                            data: chunk.data.to_owned(),
                        })
                    } else if chunk.name == *b"acTL" {
                        warn!(
                            "Stripping animation data from APNG - image will become standard PNG"
                        );
                    }
                }
            }
        }

        // Parse the chunks into our PngData
        if idat_data.is_empty() {
            return Err(PngError::ChunkMissing("IDAT"));
        }
        let ihdr_chunk = match key_chunks.remove(b"IHDR") {
            Some(ihdr) => ihdr,
            None => return Err(PngError::ChunkMissing("IHDR")),
        };
        let ihdr = parse_ihdr_chunk(
            &ihdr_chunk,
            key_chunks.remove(b"PLTE"),
            key_chunks.remove(b"tRNS"),
        )?;
        let raw_data = deflate::inflate(idat_data.as_ref(), ihdr.raw_data_size())?;

        // Reject files with incorrect width/height or truncated data
        if raw_data.len() != ihdr.raw_data_size() {
            return Err(PngError::TruncatedData);
        }

        let mut raw = PngImage {
            ihdr,
            data: raw_data,
        };
        raw.data = raw.unfilter_image()?;
        // Return the PngData
        Ok(Self {
            idat_data,
            raw: Arc::new(raw),
            aux_chunks,
        })
    }

    /// Return an estimate of the output size which can help with evaluation of very small data
    pub fn estimated_output_size(&self) -> usize {
        self.idat_data.len() + self.raw.key_chunks_size()
    }

    /// Format the `PngData` struct into a valid PNG bytestream
    pub fn output(&self) -> Vec<u8> {
        // PNG header
        let mut output = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        // IHDR
        let mut ihdr_data = Vec::with_capacity(13);
        ihdr_data.write_all(&self.raw.ihdr.width.to_be_bytes()).ok();
        ihdr_data
            .write_all(&self.raw.ihdr.height.to_be_bytes())
            .ok();
        ihdr_data.write_all(&[self.raw.ihdr.bit_depth as u8]).ok();
        ihdr_data
            .write_all(&[self.raw.ihdr.color_type.png_header_code()])
            .ok();
        ihdr_data.write_all(&[0]).ok(); // Compression -- deflate
        ihdr_data.write_all(&[0]).ok(); // Filter method -- 5-way adaptive filtering
        ihdr_data.write_all(&[self.raw.ihdr.interlaced as u8]).ok();
        write_png_block(b"IHDR", &ihdr_data, &mut output);
        // Ancillary chunks - split into those that come before IDAT and those that come after
        let mut aux_split = self.aux_chunks.split(|c| &c.name == b"IDAT");
        let aux_pre = aux_split.next().unwrap();
        for chunk in aux_pre
            .iter()
            .filter(|c| !(&c.name == b"bKGD" || &c.name == b"hIST" || &c.name == b"tRNS"))
        {
            write_png_block(&chunk.name, &chunk.data, &mut output);
        }
        // Palette and transparency
        match &self.raw.ihdr.color_type {
            ColorType::Indexed { palette } => {
                let mut palette_data = Vec::with_capacity(palette.len() * 3);
                for px in palette {
                    palette_data.extend_from_slice(px.rgb().as_slice());
                }
                write_png_block(b"PLTE", &palette_data, &mut output);
                if let Some(last_trns) = palette.iter().rposition(|px| px.a != 255) {
                    let trns_data: Vec<_> = palette[0..=last_trns].iter().map(|px| px.a).collect();
                    write_png_block(b"tRNS", &trns_data, &mut output);
                }
            }
            ColorType::Grayscale {
                transparent_shade: Some(trns),
            } => {
                // Transparency pixel - 2 byte u16
                write_png_block(b"tRNS", &trns.to_be_bytes(), &mut output);
            }
            ColorType::RGB {
                transparent_color: Some(trns),
            } => {
                // Transparency pixel - 6 byte RGB16
                let trns_data: Vec<_> = trns.iter().flat_map(|c| c.to_be_bytes()).collect();
                write_png_block(b"tRNS", &trns_data, &mut output);
            }
            _ => {}
        }
        // Special ancillary chunks that need to come after PLTE but before IDAT
        for chunk in aux_pre
            .iter()
            .filter(|c| &c.name == b"bKGD" || &c.name == b"hIST" || &c.name == b"tRNS")
        {
            write_png_block(&chunk.name, &chunk.data, &mut output);
        }
        // IDAT data
        write_png_block(b"IDAT", &self.idat_data, &mut output);
        // Ancillary chunks that come after IDAT
        for aux_post in aux_split {
            for chunk in aux_post {
                write_png_block(&chunk.name, &chunk.data, &mut output);
            }
        }
        // Stream end
        write_png_block(b"IEND", &[], &mut output);

        output
    }
}

impl PngImage {
    /// Convert the image to the specified interlacing type
    /// Returns true if the interlacing was changed, false otherwise
    /// The `interlace` parameter specifies the *new* interlacing mode
    /// Assumes that the data has already been de-filtered
    #[inline]
    #[must_use]
    pub fn change_interlacing(&self, interlace: Interlacing) -> Option<PngImage> {
        if interlace == self.ihdr.interlaced {
            return None;
        }

        Some(if interlace == Interlacing::Adam7 {
            // Convert progressive to interlaced data
            interlace_image(self)
        } else {
            // Convert interlaced to progressive data
            deinterlace_image(self)
        })
    }

    /// Return the number of channels in the image, based on color type
    #[inline]
    pub fn channels_per_pixel(&self) -> usize {
        self.ihdr.color_type.channels_per_pixel() as usize
    }

    /// Return the number of bytes per channel in the image
    #[inline]
    pub fn bytes_per_channel(&self) -> usize {
        match self.ihdr.bit_depth {
            BitDepth::Sixteen => 2,
            // Depths lower than 8 will round up to 1 byte
            _ => 1,
        }
    }

    /// Calculate the size of the PLTE and tRNS chunks
    pub fn key_chunks_size(&self) -> usize {
        match &self.ihdr.color_type {
            ColorType::Indexed { palette } => {
                let plte = 12 + palette.len() * 3;
                if let Some(trns) = palette.iter().rposition(|p| p.a != 255) {
                    plte + 12 + trns + 1
                } else {
                    plte
                }
            }
            ColorType::Grayscale { transparent_shade } if transparent_shade.is_some() => 12 + 2,
            ColorType::RGB { transparent_color } if transparent_color.is_some() => 12 + 6,
            _ => 0,
        }
    }

    /// Return an iterator over the scanlines of the image
    #[inline]
    pub fn scan_lines(&self, has_filter: bool) -> ScanLines<'_> {
        ScanLines::new(self, has_filter)
    }

    /// Reverse all filters applied on the image, returning an unfiltered IDAT bytestream
    fn unfilter_image(&self) -> Result<Vec<u8>, PngError> {
        let mut unfiltered = Vec::with_capacity(self.data.len());
        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        let mut last_line: Vec<u8> = Vec::new();
        let mut last_pass = None;
        let mut unfiltered_buf = Vec::new();
        for line in self.scan_lines(true) {
            if last_pass != line.pass {
                last_line.clear();
                last_pass = line.pass;
            }
            last_line.resize(line.data.len(), 0);
            let filter = RowFilter::try_from(line.filter).map_err(|_| PngError::InvalidData)?;
            filter.unfilter_line(bpp, line.data, &last_line, &mut unfiltered_buf)?;
            unfiltered.extend_from_slice(&unfiltered_buf);
            std::mem::swap(&mut last_line, &mut unfiltered_buf);
            unfiltered_buf.clear();
        }
        Ok(unfiltered)
    }

    /// Apply the specified filter type to all rows in the image
    pub fn filter_image(&self, filter: RowFilter, optimize_alpha: bool) -> Vec<u8> {
        let mut filtered = Vec::with_capacity(self.data.len());
        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        // If alpha optimization is enabled, determine how many bytes of alpha there are per pixel
        let alpha_bytes = if optimize_alpha && self.ihdr.color_type.has_alpha() {
            self.bytes_per_channel()
        } else {
            0
        };

        let mut prev_line = Vec::new();
        let mut prev_pass: Option<u8> = None;
        let mut f_buf = Vec::new();
        for line in self.scan_lines(false) {
            if prev_pass != line.pass || line.data.len() != prev_line.len() {
                prev_line = vec![0; line.data.len()];
            }
            // Alpha optimisation may alter the line data, so we need a mutable copy of it
            let mut line_data = line.data.to_vec();

            if filter <= RowFilter::Paeth {
                // Standard filters
                let filter = if prev_pass == line.pass || filter <= RowFilter::Sub {
                    filter
                } else {
                    RowFilter::None
                };
                filter.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                filtered.extend_from_slice(&f_buf);
                prev_line = line_data;
            } else {
                // Heuristic filter selection strategies
                let mut best_line = Vec::new();
                let mut best_line_raw = Vec::new();
                // Avoid vertical filtering on first line of each interlacing pass
                let try_filters = if prev_pass == line.pass {
                    RowFilter::STANDARD.iter()
                } else {
                    RowFilter::SINGLE_LINE.iter()
                };
                match filter {
                    RowFilter::MinSum => {
                        // MSAD algorithm mentioned in libpng reference docs
                        // http://www.libpng.org/pub/png/book/chapter09.html
                        let mut best_size = usize::MAX;
                        for f in try_filters {
                            f.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                            let size = f_buf.iter().fold(0, |acc, &x| {
                                let signed = x as i8;
                                acc + signed.unsigned_abs() as usize
                            });
                            if size < best_size {
                                best_size = size;
                                std::mem::swap(&mut best_line, &mut f_buf);
                                best_line_raw = line_data.clone();
                            }
                        }
                    }
                    RowFilter::Entropy => {
                        // Shannon entropy algorithm, from LodePNG
                        // https://github.com/lvandeve/lodepng
                        let mut best_size = i32::MIN;
                        for f in try_filters {
                            f.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                            let mut counts = vec![0; 0x100];
                            for &i in f_buf.iter() {
                                counts[i as usize] += 1;
                            }
                            let size = counts.into_iter().fold(0, |acc, x| {
                                if x == 0 {
                                    return acc;
                                }
                                acc + ilog2i(x)
                            }) as i32;
                            if size > best_size {
                                best_size = size;
                                std::mem::swap(&mut best_line, &mut f_buf);
                                best_line_raw = line_data.clone();
                            }
                        }
                    }
                    RowFilter::Bigrams => {
                        // Count distinct bigrams, from pngwolf
                        // https://bjoern.hoehrmann.de/pngwolf/
                        let mut best_size = usize::MAX;
                        for f in try_filters {
                            f.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                            let mut set = bitarr![0; 0x10000];
                            for pair in f_buf.windows(2) {
                                let bigram = (pair[0] as usize) << 8 | pair[1] as usize;
                                set.set(bigram, true);
                            }
                            let size = set.count_ones();
                            if size < best_size {
                                best_size = size;
                                std::mem::swap(&mut best_line, &mut f_buf);
                                best_line_raw = line_data.clone();
                            }
                        }
                    }
                    RowFilter::BigEnt => {
                        // Bigram entropy, combined from Entropy and Bigrams filters
                        let mut best_size = i32::MIN;
                        // FxHasher is the fastest rust hasher currently available for this purpose
                        let mut counts = FxHashMap::<u16, u32>::default();
                        for f in try_filters {
                            f.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                            counts.clear();
                            for pair in f_buf.windows(2) {
                                let bigram = (pair[0] as u16) << 8 | pair[1] as u16;
                                counts.entry(bigram).and_modify(|e| *e += 1).or_insert(1);
                            }
                            let size = counts.values().fold(0, |acc, &x| acc + ilog2i(x)) as i32;
                            if size > best_size {
                                best_size = size;
                                std::mem::swap(&mut best_line, &mut f_buf);
                                best_line_raw = line_data.clone();
                            }
                        }
                    }
                    RowFilter::Brute => {
                        // Brute force by compressing each filter attempt
                        // Similar to that of LodePNG but includes some previous lines for context
                        let mut best_size = usize::MAX;
                        let line_start = filtered.len();
                        filtered.resize(filtered.len() + line.data.len() + 1, 0);
                        let mut compressor =
                            Compressor::new(CompressionLvl::new(BRUTE_LEVEL).unwrap());
                        let limit = filtered.len().min((line.data.len() + 1) * BRUTE_LINES);
                        let capacity = compressor.zlib_compress_bound(limit);
                        let mut dest = vec![0; capacity];

                        for f in try_filters {
                            f.filter_line(bpp, &mut line_data, &prev_line, &mut f_buf, alpha_bytes);
                            filtered[line_start..].copy_from_slice(&f_buf);
                            let size = compressor
                                .zlib_compress(&filtered[filtered.len() - limit..], &mut dest)
                                .unwrap_or(usize::MAX);
                            if size < best_size {
                                best_size = size;
                                std::mem::swap(&mut best_line, &mut f_buf);
                                best_line_raw = line_data.clone();
                            }
                        }
                        filtered.resize(line_start, 0);
                    }
                    _ => unreachable!(),
                }
                filtered.extend_from_slice(&best_line);
                prev_line = best_line_raw;
            }

            prev_pass = line.pass;
        }
        filtered
    }
}

fn write_png_block(key: &[u8], chunk: &[u8], output: &mut Vec<u8>) {
    let mut chunk_data = Vec::with_capacity(chunk.len() + 4);
    chunk_data.extend_from_slice(key);
    chunk_data.extend_from_slice(chunk);
    output.reserve(chunk_data.len() + 8);
    output.extend_from_slice(&(chunk_data.len() as u32 - 4).to_be_bytes());
    let crc = deflate::crc32(&chunk_data);
    output.append(&mut chunk_data);
    output.extend_from_slice(&crc.to_be_bytes());
}

// Integer approximation for i * log2(i) - much faster than float calculations
fn ilog2i(i: u32) -> u32 {
    let log = 32 - i.leading_zeros() - 1;
    i * log + ((i - (1 << log)) << 1)
}
