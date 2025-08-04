use std::{fmt, fmt::Display};

use rgb::{RGB16, RGBA8};

use crate::PngError;

#[derive(Debug, PartialEq, Eq, Clone)]
/// The color type used to represent this image
pub enum ColorType {
    /// Grayscale, with one color channel
    Grayscale {
        /// Optional shade of gray that should be rendered as transparent
        transparent_shade: Option<u16>,
    },
    /// RGB, with three color channels
    RGB {
        /// Optional color value that should be rendered as transparent
        transparent_color: Option<RGB16>,
    },
    /// Indexed, with one byte per pixel representing a color from the palette
    Indexed {
        /// The palette containing the colors used, up to 256 entries
        palette: Vec<RGBA8>,
    },
    /// Grayscale + Alpha, with two color channels
    GrayscaleAlpha,
    /// RGBA, with four color channels
    RGBA,
}

impl Display for ColorType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grayscale { .. } => write!(f, "Grayscale"),
            Self::RGB { .. } => write!(f, "RGB"),
            Self::Indexed { palette } => write!(f, "Indexed ({} colors)", palette.len()),
            Self::GrayscaleAlpha => write!(f, "Grayscale + Alpha"),
            Self::RGBA => write!(f, "RGB + Alpha"),
        }
    }
}

impl ColorType {
    /// Get the code used by the PNG specification to denote this color type
    #[inline]
    #[must_use]
    pub const fn png_header_code(&self) -> u8 {
        match self {
            Self::Grayscale { .. } => 0,
            Self::RGB { .. } => 2,
            Self::Indexed { .. } => 3,
            Self::GrayscaleAlpha => 4,
            Self::RGBA => 6,
        }
    }

    #[inline]
    pub(crate) const fn channels_per_pixel(&self) -> u8 {
        match self {
            Self::Grayscale { .. } | Self::Indexed { .. } => 1,
            Self::GrayscaleAlpha => 2,
            Self::RGB { .. } => 3,
            Self::RGBA => 4,
        }
    }

    #[inline]
    pub(crate) const fn is_rgb(&self) -> bool {
        matches!(self, Self::RGB { .. } | Self::RGBA)
    }

    #[inline]
    pub(crate) const fn is_gray(&self) -> bool {
        matches!(self, Self::Grayscale { .. } | Self::GrayscaleAlpha)
    }

    #[inline]
    pub(crate) const fn has_alpha(&self) -> bool {
        matches!(self, Self::GrayscaleAlpha | Self::RGBA)
    }

    #[inline]
    pub(crate) const fn has_trns(&self) -> bool {
        match self {
            Self::Grayscale { transparent_shade } => transparent_shade.is_some(),
            Self::RGB { transparent_color } => transparent_color.is_some(),
            _ => false,
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// The number of bits to be used per channel per pixel
pub enum BitDepth {
    /// One bit per channel per pixel
    One = 1,
    /// Two bits per channel per pixel
    Two = 2,
    /// Four bits per channel per pixel
    Four = 4,
    /// Eight bits per channel per pixel
    Eight = 8,
    /// Sixteen bits per channel per pixel
    Sixteen = 16,
}

impl TryFrom<u8> for BitDepth {
    type Error = PngError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            4 => Ok(Self::Four),
            8 => Ok(Self::Eight),
            16 => Ok(Self::Sixteen),
            _ => Err(PngError::InvalidData),
        }
    }
}

impl Display for BitDepth {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&(*self as u8).to_string(), f)
    }
}
