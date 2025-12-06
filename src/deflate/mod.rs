mod deflater;
use std::{fmt, fmt::Display};

pub use deflater::{crc32, deflate, inflate};

use crate::{PngError, PngResult};

#[cfg(feature = "zopfli")]
mod zopfli_oxipng;
#[cfg(feature = "zopfli")]
pub use zopfli::Options as ZopfliOptions;
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
    Zopfli(ZopfliOptions),
}

impl Deflater {
    pub(crate) fn deflate(self, data: &[u8], max_size: Option<usize>) -> PngResult<Vec<u8>> {
        let compressed = match self {
            Self::Libdeflater { compression } => deflate(data, compression, max_size)?,
            #[cfg(feature = "zopfli")]
            Self::Zopfli(options) => zopfli_deflate(data, options)?,
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
            Self::Zopfli(options) => write!(f, "zopfli, zi = {}", options.iteration_count),
        }
    }
}
