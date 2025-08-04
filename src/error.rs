use std::{error::Error, fmt};

use crate::colors::{BitDepth, ColorType};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PngError {
    DeflatedDataTooLong(usize),
    TimedOut,
    NotPNG,
    APNGOutOfOrder,
    InvalidData,
    TruncatedData,
    ChunkMissing(&'static str),
    InvalidDepthForType(BitDepth, ColorType),
    IncorrectDataLength(usize, usize),
    C2PAMetadataPreventsChanges,
    Other(Box<str>),
}

impl Error for PngError {}

impl fmt::Display for PngError {
    #[inline]
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PngError::DeflatedDataTooLong(_) => f.write_str("deflated data too long"),
            PngError::TimedOut => f.write_str("timed out"),
            PngError::NotPNG => f.write_str("Invalid header detected; Not a PNG file"),
            PngError::InvalidData => f.write_str("Invalid data found; unable to read PNG file"),
            PngError::TruncatedData => {
                f.write_str("Missing data in the file; the file is truncated")
            }
            PngError::APNGOutOfOrder => f.write_str("APNG chunks are out of order"),
            PngError::ChunkMissing(s) => write!(f, "Chunk {s} missing or empty"),
            PngError::InvalidDepthForType(d, ref c) => {
                write!(f, "Invalid bit depth {d} for color type {c}")
            }
            PngError::IncorrectDataLength(l1, l2) => write!(
                f,
                "Data length {l1} does not match the expected length {l2}"
            ),
            PngError::C2PAMetadataPreventsChanges => f.write_str(
                "The image contains C2PA manifest that would be invalidated by any file changes",
            ),
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
