mod deflater;
use crate::AtomicMin;
use crate::{PngError, PngResult};
pub use deflater::crc32;
pub use deflater::deflate;
pub use deflater::inflate;
use std::{fmt, fmt::Display};

#[cfg(feature = "zopfli")]
use std::num::NonZeroU8;
#[cfg(feature = "zopfli")]
mod zopfli_oxipng;
#[cfg(feature = "zopfli")]
pub use zopfli_oxipng::deflate as zopfli_deflate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// DEFLATE algorithms supported by oxipng
pub enum Deflaters {
    /// Use libdeflater.
    Libdeflater {
        /// Which compression level to use on the file (1-12)
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

impl Deflaters {
    pub(crate) fn deflate(self, data: &[u8], max_size: &AtomicMin) -> PngResult<Vec<u8>> {
        let compressed = match self {
            Self::Libdeflater { compression } => deflate(data, compression, max_size)?,
            #[cfg(feature = "zopfli")]
            Self::Zopfli { iterations } => zopfli_deflate(data, iterations)?,
        };
        if let Some(max) = max_size.get() {
            if compressed.len() > max {
                return Err(PngError::DeflatedDataTooLong(max));
            }
        }
        Ok(compressed)
    }
}

impl Display for Deflaters {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Libdeflater { compression } => Display::fmt(compression, f),
            #[cfg(feature = "zopfli")]
            Self::Zopfli { .. } => Display::fmt("zopfli", f),
        }
    }
}
