use log::warn;
use rustc_hash::FxHashMap;
use std::{fs, path::Path, sync::Arc};

use crate::{
    Options, PngResult,
    apng::*,
    colors::{BitDepth, ColorType},
    deflate,
    error::PngError,
    filters::*,
    headers::*,
    interlace::{deinterlace_image, interlace_image},
};

pub(crate) mod scan_lines;

use self::scan_lines::ScanLines;

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
    /// APNG frames
    pub frames: Vec<Frame>,
}

impl PngData {
    /// Create a new `PngData` struct by opening a file
    #[inline]
    pub fn new(filepath: &Path, opts: &Options) -> PngResult<Self> {
        let byte_data = Self::read_file(filepath)?;

        Self::from_slice(&byte_data, opts)
    }

    pub fn read_file(filepath: &Path) -> PngResult<Vec<u8>> {
        fs::read(filepath).map_err(|e| PngError::ReadFailed(filepath.display().to_string(), e))
    }

    /// Create a new `PngData` struct by reading a slice
    pub fn from_slice(byte_data: &[u8], opts: &Options) -> PngResult<Self> {
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
        let mut frames: Vec<Frame> = Vec::new();
        let mut sequence_number = 0;
        while let Some(chunk) = parse_next_chunk(byte_data, &mut byte_offset, opts.fix_errors)? {
            match &chunk.name {
                b"IDAT" => {
                    if idat_data.is_empty() {
                        // Keep track of where the first IDAT sits relative to other chunks
                        aux_chunks.push(Chunk {
                            name: chunk.name,
                            data: Vec::new(),
                        });
                    }
                    idat_data.extend_from_slice(chunk.data);
                }
                b"IHDR" | b"PLTE" | b"tRNS" => {
                    key_chunks.insert(chunk.name, chunk.data.to_owned());
                }
                _ if opts.strip.keep(&chunk.name) => {
                    if chunk.name == *b"caBX" || chunk.name == *b"iDOT" {
                        // caBX (C2PA manifest) and iDOT (data offsets) are necessarily invalidated
                        // by changes to the file. If these chunks have been explicitly kept then
                        // return an error, otherwise strip them automatically.
                        if matches!(opts.strip, StripChunks::Keep(_)) {
                            return Err(PngError::ChunkPreventsChanges(chunk.name));
                        }
                        warn!(
                            "Stripping {} chunk which will be invalidated by file changes",
                            std::str::from_utf8(&chunk.name).unwrap()
                        );
                        continue;
                    }
                    if chunk.name == *b"fcTL" || chunk.name == *b"fdAT" {
                        // Validate the sequence number
                        if read_be_u32(&chunk.data[0..4]) != sequence_number {
                            return Err(PngError::APNGOutOfOrder);
                        }
                        sequence_number += 1;
                        if chunk.name == *b"fcTL" && !idat_data.is_empty() {
                            // Only create a Frame if it's after the IDAT (else store it as an aux chunk)
                            frames.push(Frame::from_fctl_data(chunk.data)?);
                            continue;
                        } else if chunk.name == *b"fdAT" {
                            // Append the data to the last frame
                            frames
                                .last_mut()
                                .ok_or(PngError::APNGOutOfOrder)?
                                .data
                                .extend_from_slice(&chunk.data[4..]);
                            continue;
                        }
                    }
                    // Regular ancillary chunk
                    aux_chunks.push(Chunk {
                        name: chunk.name,
                        data: chunk.data.to_owned(),
                    });
                }
                b"acTL" => {
                    warn!("Stripping animation data from APNG - image will become standard PNG");
                }
                _ => (),
            }
        }

        // Parse the chunks into our PngData
        if idat_data.is_empty() {
            return Err(PngError::ChunkMissing("IDAT"));
        }
        let Some(ihdr_chunk) = key_chunks.remove(b"IHDR") else {
            return Err(PngError::ChunkMissing("IHDR"));
        };
        let ihdr = parse_ihdr_chunk(
            &ihdr_chunk,
            key_chunks.remove(b"PLTE"),
            key_chunks.remove(b"tRNS"),
        )?;
        if let Some(max) = opts.max_decompressed_size {
            if ihdr.raw_data_size() > max {
                return Err(PngError::InflatedDataTooLong(max));
            }
        }

        let raw = PngImage::new(ihdr, &idat_data)?;

        // Return the PngData
        Ok(Self {
            idat_data,
            raw: Arc::new(raw),
            aux_chunks,
            frames,
        })
    }

    /// Format the `PngData` struct into a valid PNG bytestream
    #[must_use]
    pub fn output(&self) -> Vec<u8> {
        // PNG header
        let mut output = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        // IHDR
        let mut ihdr_data = Vec::with_capacity(13);
        ihdr_data.extend_from_slice(&self.raw.ihdr.width.to_be_bytes());
        ihdr_data.extend_from_slice(&self.raw.ihdr.height.to_be_bytes());
        ihdr_data.extend_from_slice(&[self.raw.ihdr.bit_depth as u8]);
        ihdr_data.extend_from_slice(&[
            self.raw.ihdr.color_type.png_header_code(),
            0, // Compression -- deflate
            0, // Filter method -- 5-way adaptive filtering
            self.raw.ihdr.interlaced as u8,
        ]);
        write_png_block(b"IHDR", &ihdr_data, &mut output);
        // Ancillary chunks - split into those that come before IDAT and those that come after
        let mut aux_split = self.aux_chunks.split(|c| &c.name == b"IDAT");
        let aux_pre = aux_split.next().unwrap();
        // Many chunks need to be before PLTE, so write all except those that explicitly need to be after
        // Note: the PNG spec does not say that fcTL needs to be after PLTE, but some decoders expect
        //       that (see issue #625)
        for chunk in aux_pre
            .iter()
            .filter(|c| !matches!(&c.name, b"bKGD" | b"hIST" | b"tRNS" | b"fcTL"))
        {
            write_png_block(&chunk.name, &chunk.data, &mut output);
        }
        // Palette and transparency
        match &self.raw.ihdr.color_type {
            ColorType::Indexed { palette } => {
                let mut palette_data = Vec::with_capacity(palette.len() * 3);
                for px in palette {
                    palette_data.extend_from_slice(px.rgb().as_ref());
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
                let trns_data: Vec<_> = trns.iter().flat_map(u16::to_be_bytes).collect();
                write_png_block(b"tRNS", &trns_data, &mut output);
            }
            _ => {}
        }
        // Special ancillary chunks that need to come after PLTE but before IDAT
        let mut sequence_number = 0;
        for chunk in aux_pre
            .iter()
            .filter(|c| matches!(&c.name, b"bKGD" | b"hIST" | b"tRNS" | b"fcTL"))
        {
            write_png_block(&chunk.name, &chunk.data, &mut output);
            if &chunk.name == b"fcTL" {
                sequence_number += 1;
            }
        }
        // IDAT data
        write_png_block(b"IDAT", &self.idat_data, &mut output);
        // APNG frames
        for frame in self.frames.iter() {
            write_png_block(b"fcTL", &frame.fctl_data(sequence_number), &mut output);
            write_png_block(b"fdAT", &frame.fdat_data(sequence_number + 1), &mut output);
            sequence_number += 2;
        }
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
    pub fn new(ihdr: IhdrData, compressed_data: &[u8]) -> PngResult<Self> {
        let raw_data = deflate::inflate(compressed_data, ihdr.raw_data_size())?;

        // Reject files with incorrect width/height or truncated data
        if raw_data.len() != ihdr.raw_data_size() {
            return Err(PngError::TruncatedData);
        }

        let mut image = Self {
            ihdr,
            data: raw_data,
        };
        image.data = image.unfilter_image()?;
        Ok(image)
    }

    /// Enable or disable interlacing
    /// Returns the new image if the interlacing was changed, None otherwise
    /// Assumes that the data has already been de-filtered
    #[inline]
    #[must_use]
    pub fn change_interlacing(&self, interlace: bool) -> Option<Self> {
        if interlace == self.ihdr.interlaced {
            return None;
        }

        Some(if interlace {
            // Convert progressive to interlaced data
            interlace_image(self)
        } else {
            // Convert interlaced to progressive data
            deinterlace_image(self)
        })
    }

    /// Return the number of channels in the image, based on color type
    #[inline]
    #[must_use]
    pub const fn channels_per_pixel(&self) -> usize {
        self.ihdr.color_type.channels_per_pixel() as usize
    }

    /// Return the number of bytes per channel in the image
    #[inline]
    #[must_use]
    pub const fn bytes_per_channel(&self) -> usize {
        match self.ihdr.bit_depth {
            BitDepth::Sixteen => 2,
            // Depths lower than 8 will round up to 1 byte
            _ => 1,
        }
    }

    /// Calculate the size of the PLTE and tRNS chunks
    #[must_use]
    pub fn key_chunks_size(&self) -> usize {
        match &self.ihdr.color_type {
            ColorType::Indexed { palette } => {
                let plte = 12 + palette.len() * 3;
                palette
                    .iter()
                    .rposition(|p| p.a != 255)
                    .map_or(plte, |trns| plte + 12 + trns + 1)
            }
            ColorType::Grayscale { transparent_shade } if transparent_shade.is_some() => 12 + 2,
            ColorType::RGB { transparent_color } if transparent_color.is_some() => 12 + 6,
            _ => 0,
        }
    }

    /// Return an estimate of the output size which can help with evaluation of very small data
    #[must_use]
    pub fn estimated_output_size(&self, idat_data: &[u8]) -> usize {
        idat_data.len() + self.key_chunks_size()
    }

    /// Return an iterator over the scanlines of the image
    #[inline]
    #[must_use]
    pub fn scan_lines(&self, has_filter: bool) -> ScanLines<'_> {
        ScanLines::new(self, has_filter)
    }

    /// Reverse all filters applied on the image, returning an unfiltered IDAT bytestream
    fn unfilter_image(&self) -> PngResult<Vec<u8>> {
        let mut unfiltered = Vec::with_capacity(self.data.len());
        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        let mut prev_line: Vec<u8> = Vec::new();
        let mut prev_pass = None;
        for line in self.scan_lines(true) {
            if prev_pass != line.pass || prev_line.is_empty() {
                prev_line = vec![0; line.data.len()];
                prev_pass = line.pass;
            }
            let offset = unfiltered.len();
            let filter = RowFilter::try_from(line.filter).map_err(|()| PngError::InvalidData)?;
            filter.unfilter_line(bpp, line.data, &prev_line, &mut unfiltered);
            prev_line.clone_from_slice(&unfiltered[offset..]);
        }
        Ok(unfiltered)
    }

    /// Apply the specified filter type to all rows in the image
    #[must_use]
    pub fn filter_image(
        &self,
        strategy: FilterStrategy,
        optimize_alpha: bool,
    ) -> (Vec<u8>, FilterStrategy) {
        let mut output = Vec::with_capacity(self.ihdr.raw_data_size());
        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        // If alpha optimization is enabled, determine how many bytes of alpha there are per pixel
        let alpha_bytes = if optimize_alpha && self.ihdr.color_type.has_alpha() {
            self.bytes_per_channel()
        } else {
            0
        };

        let mut prev_line = Vec::new();
        let mut prev_pass: Option<u8> = None;
        // For heuristic strategies, keep track of the actual filter used for each line
        let mut filters_used = Vec::new();
        let mut strategy_evaluator = strategy.evaluator();
        for (i, line) in self.scan_lines(false).enumerate() {
            if prev_pass != line.pass || prev_line.is_empty() {
                prev_line = vec![0; line.data.len()];
                prev_pass = line.pass;
            }
            // Alpha optimisation may alter the line data, so we need a mutable copy of it
            let mut line_data = line.data.to_vec();

            if let FilterStrategy::Basic(filter) = strategy {
                // Standard filters
                filter.filter_line(bpp, &mut line_data, &prev_line, &mut output, alpha_bytes);
                prev_line = line_data;
                continue;
            } else if let FilterStrategy::Predefined(lines) = &strategy {
                // Predefined filter for each line
                let filter = lines.get(i).unwrap_or(&RowFilter::None);
                filter.filter_line(bpp, &mut line_data, &prev_line, &mut output, alpha_bytes);
                prev_line = line_data;
                continue;
            }

            // Heuristic filter selection strategies

            let mut best_filter = RowFilter::None;
            if line_data.iter().all(|&x| x == 0) {
                // Assume None if the line is all zeros
                best_filter.filter_line(bpp, &mut line_data, &prev_line, &mut output, alpha_bytes);
                prev_line = line_data;
                filters_used.push(best_filter);
                continue;
            }

            let line_len = line.data.len() + 1;
            let mut best_line = vec![0; line_len];
            let mut best_line_raw = Vec::with_capacity(line.data.len());
            let offset = output.len();
            let evaluator = strategy_evaluator.as_mut().unwrap();
            evaluator.reset(line_len);
            for f in RowFilter::ALL {
                f.filter_line(bpp, &mut line_data, &prev_line, &mut output, alpha_bytes);
                if evaluator.evaluate(&output, offset) {
                    best_line.clone_from_slice(&output[offset..]);
                    best_line_raw.clone_from(&line_data);
                    best_filter = f;
                }
                output.truncate(offset);
            }
            output.extend_from_slice(&best_line);
            prev_line = best_line_raw;
            filters_used.push(best_filter);
        }

        if filters_used.is_empty() {
            (output, strategy)
        } else {
            (output, FilterStrategy::Predefined(filters_used))
        }
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
