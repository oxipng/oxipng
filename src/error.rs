use std::{error::Error, fmt};

use crate::colors::{BitDepth, ColorType};

#[derive(Debug)]
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
    ReadFailed(String, std::io::Error),
    TruncatedData,
    WriteFailed(String, std::io::Error),
    Other(Box<str>),
}

impl Error for PngError {}

impl fmt::Display for PngError {
    #[inline]
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::APNGOutOfOrder => f.write_str("APNG chunks are out of order"),
            Self::C2PAMetadataPreventsChanges => f.write_str(
                "The image contains C2PA manifest that would be invalidated by any file changes",
            ),
            Self::ChunkMissing(s) => write!(f, "Chunk {s} missing or empty"),
            Self::CRCMismatch(ref c) => write!(
                f,
                "CRC mismatch in {} chunk; May be recoverable by using --fix",
                String::from_utf8_lossy(c)
            ),
            Self::DeflatedDataTooLong(_) => f.write_str("Deflated data too long"),
            Self::IncorrectDataLength(l1, l2) => write!(
                f,
                "Data length {l1} does not match the expected length {l2}"
            ),
            Self::InflatedDataTooLong(max) => write!(
                f,
                "Inflated data would exceed the maximum size ({max} bytes)"
            ),
            Self::InvalidData => f.write_str("Invalid data found; unable to read PNG file"),
            Self::InvalidDepthForType(d, ref c) => {
                write!(f, "Invalid bit depth {d} for color type {c}")
            }
            Self::NotPNG => f.write_str("Invalid header detected; Not a PNG file"),
            Self::ReadFailed(ref s, ref e) => write!(f, "Failed to read from {s}: {e}"),
            Self::TruncatedData => f.write_str("Missing data in the file; the file is truncated"),
            Self::WriteFailed(ref s, ref e) => write!(f, "Failed to write to {s}: {e}"),
            Self::Other(ref s) => f.write_str(s),
        }
    }
}

impl PngError {
    #[cold]
    #[must_use]
    pub fn new(description: &str) -> Self {
        Self::Other(description.into())
    }
}
