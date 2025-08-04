mod deflater;
#[cfg(feature = "zopfli")]
use std::num::NonZeroU8;
use std::{fmt, fmt::Display};

pub use deflater::{crc32, deflate, inflate};

use crate::{PngError, PngResult};
#[cfg(feature = "zopfli")]
mod zopfli_oxipng;
#[cfg(feature = "zopfli")]
pub use zopfli_oxipng::deflate as zopfli_deflate;

/// DEFLATE algorithms supported by oxipng (for use in [`Options`][crate::Options])
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deflater {
    /// Use libdeflater.
    Libdeflater {
        /// Which compression level to use on the file (0-12)
        compression: u8,
    },
    #[cfg(feature = "zopfli")]
    /// Use the better but slower Zopfli implementation
    Zopfli {
        /// The number of compression iterations to do. 15 iterations are fine
        /// for small files, but bigger files will need to be compressed with
        /// less iterations, or else they will be too slow.
        iterations: NonZeroU8,
    },
}

impl Deflater {
    pub(crate) fn deflate(self, data: &[u8], max_size: Option<usize>) -> PngResult<Vec<u8>> {
        let compressed = match self {
            Self::Libdeflater { compression } => deflate(data, compression, max_size)?,
            #[cfg(feature = "zopfli")]
            Self::Zopfli { iterations } => zopfli_deflate(data, iterations)?,
        };
        if let Some(max) = max_size {
            if compressed.len() > max {
                return Err(PngError::DeflatedDataTooLong(max));
            }
        }
        Ok(compressed)
    }
}

impl Display for Deflater {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Libdeflater { compression } => write!(f, "zc = {compression}"),
            #[cfg(feature = "zopfli")]
            Self::Zopfli { iterations } => write!(f, "zopfli, zi = {iterations}"),
        }
    }
}
