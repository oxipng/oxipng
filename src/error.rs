use std::{error::Error, fmt};

use crate::colors::{BitDepth, ColorType};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PngError {
    APNGOutOfOrder,
    C2PAMetadataPreventsChanges,
    ChunkMissing(&'static str),
    CRCMismatch([u8; 4]),
    DeflatedDataTooLong(usize),
    IncorrectDataLength(usize, usize),
    InflatedDataTooLong(usize),
    InvalidData,
    InvalidDepthForType(BitDepth, ColorType),
    NotPNG,
    TruncatedData,
    Other(Box<str>),
}

impl Error for PngError {}

impl fmt::Display for PngError {
    #[inline]
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PngError::APNGOutOfOrder => f.write_str("APNG chunks are out of order"),
            PngError::C2PAMetadataPreventsChanges => f.write_str(
                "The image contains C2PA manifest that would be invalidated by any file changes",
            ),
            PngError::ChunkMissing(s) => write!(f, "Chunk {s} missing or empty"),
            PngError::CRCMismatch(ref c) => write!(
                f,
                "CRC mismatch in {} chunk; May be recoverable by using --fix",
                String::from_utf8_lossy(c)
            ),
            PngError::DeflatedDataTooLong(_) => f.write_str("Deflated data too long"),
            PngError::IncorrectDataLength(l1, l2) => write!(
                f,
                "Data length {l1} does not match the expected length {l2}"
            ),
            PngError::InflatedDataTooLong(_) => f.write_str("Inflated data too long"),
            PngError::InvalidData => f.write_str("Invalid data found; unable to read PNG file"),
            PngError::InvalidDepthForType(d, ref c) => {
                write!(f, "Invalid bit depth {d} for color type {c}")
            }
            PngError::NotPNG => f.write_str("Invalid header detected; Not a PNG file"),
            PngError::TruncatedData => {
                f.write_str("Missing data in the file; the file is truncated")
            }
            PngError::Other(ref s) => f.write_str(s),
        }
    }
}

impl PngError {
    #[cold]
    #[must_use]
    pub fn new(description: &str) -> PngError {
        PngError::Other(description.into())
    }
}
