use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum PngError {
    DeflatedDataTooLong(usize),
    NotPNG,
    APNGNotSupported,
    InvalidData,
    TruncatedData,
    ChunkMissing(&'static str),
    Other(Box<str>),
}

impl Error for PngError {
    // deprecated
    fn description(&self) -> &str {
        ""
    }
}

impl fmt::Display for PngError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PngError::DeflatedDataTooLong(_) => f.write_str("deflated data too long"),
            PngError::NotPNG => f.write_str("Invalid header detected; Not a PNG file"),
            PngError::InvalidData => f.write_str("Invalid data found; unable to read PNG file"),
            PngError::TruncatedData => f.write_str("Missing data in the file; the file is truncated"),
            PngError::APNGNotSupported => f.write_str("APNG files are not (yet) supported"),
            PngError::ChunkMissing(s) => write!(f, "Chunk {} missing or empty", s),
            PngError::Other(ref s) => f.write_str(s),
        }
    }
}

impl PngError {
    #[inline]
    pub fn new(description: &str) -> PngError {
        PngError::Other(description.into())
    }
}
