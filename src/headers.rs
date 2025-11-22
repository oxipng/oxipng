use indexmap::IndexSet;
use log::{debug, trace, warn};
use rgb::{RGB16, RGBA8};

use crate::{
    Deflater, Options, PngResult,
    colors::{BitDepth, ColorType},
    deflate::{crc32, inflate},
    display_chunks::DISPLAY_CHUNKS,
    error::PngError,
};

#[derive(Debug, Clone)]
/// Headers from the IHDR chunk of the image
pub struct IhdrData {
    /// The width of the image in pixels
    pub width: u32,
    /// The height of the image in pixels
    pub height: u32,
    /// The color type of the image
    pub color_type: ColorType,
    /// The bit depth of the image
    pub bit_depth: BitDepth,
    /// Whether the image is interlaced
    pub interlaced: bool,
}

impl IhdrData {
    /// Bits per pixel
    #[must_use]
    #[inline]
    pub const fn bpp(&self) -> usize {
        self.bit_depth as usize * self.color_type.channels_per_pixel() as usize
    }

    /// Byte length of IDAT that is correct for this IHDR
    #[must_use]
    pub const fn raw_data_size(&self) -> usize {
        let w = self.width as usize;
        let h = self.height as usize;
        let bpp = self.bpp();

        const fn bitmap_size(bpp: usize, w: usize, h: usize) -> usize {
            (w * bpp).div_ceil(8) * h
        }

        if self.interlaced {
            let mut size = bitmap_size(bpp, (w + 7) >> 3, (h + 7) >> 3) + ((h + 7) >> 3);
            if w > 4 {
                size += bitmap_size(bpp, (w + 3) >> 3, (h + 7) >> 3) + ((h + 7) >> 3);
            }
            size += bitmap_size(bpp, (w + 3) >> 2, (h + 3) >> 3) + ((h + 3) >> 3);
            if w > 2 {
                size += bitmap_size(bpp, (w + 1) >> 2, (h + 3) >> 2) + ((h + 3) >> 2);
            }
            size += bitmap_size(bpp, (w + 1) >> 1, (h + 1) >> 2) + ((h + 1) >> 2);
            if w > 1 {
                size += bitmap_size(bpp, w >> 1, (h + 1) >> 1) + ((h + 1) >> 1);
            }
            size + bitmap_size(bpp, w, h >> 1) + (h >> 1)
        } else {
            bitmap_size(bpp, w, h) + h
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: [u8; 4],
    pub data: Vec<u8>,
}

/// [`Options`][crate::Options] to use when stripping chunks (metadata)
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum StripChunks {
    /// None
    ///
    /// ...except caBX chunk if it contains a C2PA.org signature.
    None,
    /// Remove specific chunks
    Strip(IndexSet<[u8; 4]>),
    /// Remove all chunks that won't affect image display
    Safe,
    /// Remove all non-critical chunks except these
    Keep(IndexSet<[u8; 4]>),
    /// All non-critical chunks
    All,
}

impl StripChunks {
    pub(crate) fn keep(&self, name: &[u8; 4]) -> bool {
        match &self {
            Self::None => true,
            Self::Keep(names) => names.contains(name),
            Self::Strip(names) => !names.contains(name),
            Self::Safe => DISPLAY_CHUNKS.contains(name),
            Self::All => false,
        }
    }
}

#[inline]
pub fn file_header_is_valid(bytes: &[u8]) -> bool {
    let expected_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    *bytes == expected_header
}

#[derive(Debug, Clone, Copy)]
pub struct RawChunk<'a> {
    pub name: [u8; 4],
    pub data: &'a [u8],
}

impl RawChunk<'_> {
    // Is it a chunk for C2PA/CAI JUMBF metadata
    pub(crate) fn is_c2pa(&self) -> bool {
        if self.name == *b"caBX" {
            if let Some((b"jumb", data)) = parse_jumbf_box(self.data) {
                if let Some((b"jumd", data)) = parse_jumbf_box(data) {
                    if data.get(..4) == Some(b"c2pa") {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn parse_jumbf_box(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if data.len() < 8 {
        return None;
    }
    let (len, rest) = data.split_at(4);
    let len = read_be_u32(len) as usize;
    if len < 8 || len > data.len() {
        return None;
    }
    let (box_name, data) = rest.split_at(4);
    let data = data.get(..len - 8)?;
    Some((box_name, data))
}

pub fn parse_next_chunk<'a>(
    byte_data: &'a [u8],
    byte_offset: &mut usize,
    fix_errors: bool,
) -> PngResult<Option<RawChunk<'a>>> {
    let length = read_be_u32(
        byte_data
            .get(*byte_offset..*byte_offset + 4)
            .ok_or(PngError::TruncatedData)?,
    );
    if byte_data.len() < *byte_offset + 12 + length as usize {
        return Err(PngError::TruncatedData);
    }
    *byte_offset += 4;

    let chunk_start = *byte_offset;
    let chunk_name = &byte_data[chunk_start..chunk_start + 4];
    if chunk_name == b"IEND" {
        // End of data
        return Ok(None);
    }
    *byte_offset += 4;

    let data = &byte_data[*byte_offset..*byte_offset + length as usize];
    *byte_offset += length as usize;
    let crc = read_be_u32(&byte_data[*byte_offset..*byte_offset + 4]);
    *byte_offset += 4;

    let chunk_bytes = &byte_data[chunk_start..chunk_start + 4 + length as usize];
    if !fix_errors && crc32(chunk_bytes) != crc {
        return Err(PngError::CRCMismatch(chunk_name.try_into().unwrap()));
    }

    let name: [u8; 4] = chunk_name.try_into().unwrap();
    Ok(Some(RawChunk { name, data }))
}

pub fn parse_ihdr_chunk(
    byte_data: &[u8],
    palette_data: Option<Vec<u8>>,
    trns_data: Option<Vec<u8>>,
) -> PngResult<IhdrData> {
    // This eliminates bounds checks for the rest of the function
    let interlaced = byte_data.get(12).copied().ok_or(PngError::TruncatedData)?;
    Ok(IhdrData {
        color_type: match byte_data[9] {
            0 => ColorType::Grayscale {
                transparent_shade: trns_data
                    .filter(|t| t.len() >= 2)
                    .map(|t| read_be_u16(&t[0..2])),
            },
            2 => ColorType::RGB {
                transparent_color: trns_data.filter(|t| t.len() >= 6).map(|t| RGB16 {
                    r: read_be_u16(&t[0..2]),
                    g: read_be_u16(&t[2..4]),
                    b: read_be_u16(&t[4..6]),
                }),
            },
            3 => ColorType::Indexed {
                palette: palette_to_rgba(palette_data, trns_data).unwrap_or_default(),
            },
            4 => ColorType::GrayscaleAlpha,
            6 => ColorType::RGBA,
            _ => return Err(PngError::InvalidData),
        },
        bit_depth: byte_data[8].try_into()?,
        width: read_be_u32(&byte_data[0..4]),
        height: read_be_u32(&byte_data[4..8]),
        interlaced: match interlaced {
            0 => false,
            1 => true,
            _ => return Err(PngError::InvalidData),
        },
    })
}

/// Construct an RGBA palette from the raw palette and transparency data
fn palette_to_rgba(
    palette_data: Option<Vec<u8>>,
    trns_data: Option<Vec<u8>>,
) -> Result<Vec<RGBA8>, PngError> {
    let palette_data = palette_data.ok_or(PngError::ChunkMissing("PLTE"))?;
    let mut palette: Vec<_> = palette_data
        .chunks_exact(3)
        .map(|color| RGBA8::new(color[0], color[1], color[2], 255))
        .collect();

    if let Some(trns_data) = trns_data {
        for (color, trns) in palette.iter_mut().zip(trns_data) {
            color.a = trns;
        }
    }
    Ok(palette)
}

#[inline]
pub fn read_be_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes(bytes.try_into().unwrap())
}

#[inline]
pub fn read_be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().unwrap())
}

/// Extract and decompress the ICC profile from an iCCP chunk
pub fn extract_icc(iccp: &Chunk, max_size: Option<usize>) -> Option<Vec<u8>> {
    // Skip (useless) profile name
    let mut data = iccp.data.as_slice();
    loop {
        let (&n, rest) = data.split_first()?;
        data = rest;
        if n == 0 {
            break;
        }
    }

    let (&compression_method, compressed_data) = data.split_first()?;
    if compression_method != 0 {
        return None; // The profile is supposed to be compressed (method 0)
    }
    // The decompressed size is unknown so we have to guess the required buffer size
    let mut out_size = compressed_data.len() * 2 + 1000;
    if let Some(max) = max_size {
        out_size = out_size.min(max);
    }
    match inflate(compressed_data, out_size) {
        Ok(icc) => Some(icc),
        Err(e) => {
            // Log the error so we can know if the buffer size needs to be adjusted
            warn!("Failed to decompress icc: {e}");
            None
        }
    }
}

/// Make an iCCP chunk by compressing the ICC profile
pub fn make_iccp(icc: &[u8], deflater: Deflater, max_size: Option<usize>) -> PngResult<Chunk> {
    let mut compressed = deflater.deflate(icc, max_size)?;
    let mut data = Vec::with_capacity(compressed.len() + 5);
    data.extend(b"icc"); // Profile name - generally unused, can be anything
    data.extend([0, 0]); // Null separator, zlib compression method
    data.append(&mut compressed);
    Ok(Chunk {
        name: *b"iCCP",
        data,
    })
}

/// If the profile is sRGB, extracts the rendering intent value from it
pub fn srgb_rendering_intent(icc_data: &[u8]) -> Option<u8> {
    let rendering_intent = *icc_data.get(67)?;

    // The known profiles are the same as in libpng's `png_sRGB_checks`.
    // The Profile ID header of ICC has a fixed layout,
    // and is supposed to contain MD5 of profile data at this offset
    match icc_data.get(84..100)? {
        b"\x29\xf8\x3d\xde\xaf\xf2\x55\xae\x78\x42\xfa\xe4\xca\x83\x39\x0d"
        | b"\xc9\x5b\xd6\x37\xe9\x5d\x8a\x3b\x0d\xf3\x8f\x99\xc1\x32\x03\x89"
        | b"\xfc\x66\x33\x78\x37\xe2\x88\x6b\xfd\x72\xe9\x83\x82\x28\xf1\xb8"
        | b"\x34\x56\x2a\xbf\x99\x4c\xcd\x06\x6d\x2c\x57\x21\xd0\xd6\x8c\x5d" => {
            Some(rendering_intent)
        }
        b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" => {
            // Known-bad profiles are identified by their CRC
            match (crc32(icc_data), icc_data.len()) {
                (0x5d51_29ce, 3024) | (0x182e_a552, 3144) | (0xf29e_526d, 3144) => {
                    Some(rendering_intent)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Process aux chunks and potentially adjust options before optimizing
pub fn preprocess_chunks(aux_chunks: &mut Vec<Chunk>, opts: &mut Options) {
    let has_srgb = aux_chunks.iter().any(|c| &c.name == b"sRGB");
    // Grayscale conversion should not be performed if the image is not in the sRGB colorspace
    // An sRGB profile would need to be stripped on conversion, so disallow if stripping is disabled
    let mut allow_grayscale = !has_srgb || opts.strip != StripChunks::None;

    if let Some(iccp_idx) = aux_chunks.iter().position(|c| &c.name == b"iCCP") {
        allow_grayscale = false;
        // See if we can replace an iCCP chunk with an sRGB chunk
        let may_replace_iccp = opts.strip != StripChunks::None && opts.strip.keep(b"sRGB");
        if may_replace_iccp && has_srgb {
            // Files aren't supposed to have both chunks, so we chose to honor sRGB
            trace!("Removing iCCP chunk due to conflict with sRGB chunk");
            aux_chunks.remove(iccp_idx);
            allow_grayscale = true;
        } else if let Some(icc) = extract_icc(&aux_chunks[iccp_idx], opts.max_decompressed_size) {
            let intent = if may_replace_iccp {
                srgb_rendering_intent(&icc)
            } else {
                None
            };
            // sRGB-like profile can be replaced with an sRGB chunk with the same rendering intent
            if let Some(intent) = intent {
                trace!("Replacing iCCP chunk with equivalent sRGB chunk");
                aux_chunks[iccp_idx] = Chunk {
                    name: *b"sRGB",
                    data: vec![intent],
                };
                allow_grayscale = true;
            } else if opts.idat_recoding {
                // Try recompressing the profile
                let cur_len = aux_chunks[iccp_idx].data.len();
                if let Ok(iccp) = make_iccp(&icc, opts.deflater, Some(cur_len - 1)) {
                    debug!(
                        "Recompressed iCCP chunk: {} ({} bytes decrease)",
                        iccp.data.len(),
                        cur_len - iccp.data.len()
                    );
                    aux_chunks[iccp_idx] = iccp;
                }
            }
        }
    }

    if !allow_grayscale && opts.grayscale_reduction {
        debug!("Disabling grayscale reduction due to presence of sRGB or iCCP chunk");
        opts.grayscale_reduction = false;
    }

    // Check for APNG by presence of acTL chunk
    if aux_chunks.iter().any(|c| &c.name == b"acTL") {
        warn!("APNG detected, disabling all reductions");
        opts.interlace = None;
        opts.bit_depth_reduction = false;
        opts.color_type_reduction = false;
        opts.palette_reduction = false;
        opts.grayscale_reduction = false;
    }
}

/// Perform cleanup of certain aux chunks after optimization has been completed
pub fn postprocess_chunks(aux_chunks: &mut Vec<Chunk>, ihdr: &IhdrData, orig_ihdr: &IhdrData) {
    // If the depth/color type has changed, some chunks may be invalid and should be dropped
    // While these could potentially be converted, they have no known use case today and are
    // generally more trouble than they're worth
    if orig_ihdr.bit_depth != ihdr.bit_depth || orig_ihdr.color_type != ihdr.color_type {
        aux_chunks.retain(|c| {
            let invalid = &c.name == b"bKGD" || &c.name == b"sBIT" || &c.name == b"hIST";
            if invalid {
                warn!(
                    "Removing {} chunk as it no longer matches the image data",
                    std::str::from_utf8(&c.name).unwrap()
                );
            }
            !invalid
        });
    }

    // Remove any sRGB or iCCP chunks if the image was converted to or from grayscale
    if orig_ihdr.color_type.is_gray() != ihdr.color_type.is_gray() {
        aux_chunks.retain(|c| {
            let invalid = &c.name == b"sRGB" || &c.name == b"iCCP";
            if invalid {
                trace!(
                    "Removing {} chunk as it no longer matches the color type",
                    std::str::from_utf8(&c.name).unwrap()
                );
            }
            !invalid
        });
    }

    // Remove iDOT which will necessarily be invalid after successful optimization
    aux_chunks.retain(|c| {
        let invalid = &c.name == b"iDOT";
        if invalid {
            trace!(
                "Removing {} chunk as it no longer matches the IDAT",
                std::str::from_utf8(&c.name).unwrap()
            );
        }
        !invalid
    });
}
