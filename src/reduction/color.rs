use std::hash::{BuildHasherDefault, Hash};

use indexmap::IndexSet;
use rgb::{ComponentMap, ComponentSlice, FromSlice, RGB, RGBA, alt::Gray};
use rustc_hash::FxHasher;

use crate::{
    colors::{BitDepth, ColorType},
    headers::IhdrData,
    png::PngImage,
};

type FxIndexSet<V> = IndexSet<V, BuildHasherDefault<FxHasher>>;

/// Maximum size difference between indexed and channels to consider a candidate for evaluation
pub const INDEXED_MAX_DIFF: usize = 20000;

fn build_palette<T>(
    iter: impl IntoIterator<Item = T>,
    reduced: &mut Vec<u8>,
) -> Option<FxIndexSet<T>>
where
    T: Eq + Hash,
{
    let mut palette = FxIndexSet::default();
    palette.reserve(257);
    for pixel in iter {
        let (idx, _) = palette.insert_full(pixel);
        if idx == 256 {
            return None;
        }
        reduced.push(idx as u8);
    }
    Some(palette)
}

#[must_use]
pub fn reduced_to_indexed(png: &PngImage, allow_grayscale: bool) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Eight {
        return None;
    }
    if matches!(png.ihdr.color_type, ColorType::Indexed { .. }) {
        return None;
    }
    if !allow_grayscale && png.ihdr.color_type.is_gray() {
        return None;
    }

    let mut raw_data = Vec::with_capacity(png.data.len() / png.channels_per_pixel());
    let palette: Vec<_> = match png.ihdr.color_type {
        ColorType::Grayscale { transparent_shade } => {
            let pmap = build_palette(png.data.as_gray().iter().copied(), &mut raw_data)?;
            // Convert the Gray16 transparency to Gray8
            let transparency_pixel = transparent_shade.map(|t| Gray::from(t as u8));
            pmap.into_iter()
                .map(|px| {
                    RGB::from(px).with_alpha(if Some(px) == transparency_pixel {
                        0
                    } else {
                        255
                    })
                })
                .collect()
        }
        ColorType::RGB { transparent_color } => {
            let pmap = build_palette(png.data.as_rgb().iter().copied(), &mut raw_data)?;
            // Convert the RGB16 transparency to RGB8
            let transparency_pixel = transparent_color.map(|t| t.map(|c| c as u8));
            pmap.into_iter()
                .map(|px| {
                    px.with_alpha(if Some(px) == transparency_pixel {
                        0
                    } else {
                        255
                    })
                })
                .collect()
        }
        ColorType::GrayscaleAlpha => {
            let pmap = build_palette(png.data.as_gray_alpha().iter().copied(), &mut raw_data)?;
            pmap.into_iter().map(RGBA::from).collect()
        }
        ColorType::RGBA => {
            let pmap = build_palette(png.data.as_rgba().iter().copied(), &mut raw_data)?;
            pmap.into_iter().collect()
        }
        _ => return None,
    };

    Some(PngImage {
        data: raw_data,
        ihdr: IhdrData {
            color_type: ColorType::Indexed { palette },
            ..png.ihdr
        },
    })
}

#[must_use]
pub fn reduced_rgb_to_grayscale(png: &PngImage) -> Option<PngImage> {
    if !png.ihdr.color_type.is_rgb() {
        return None;
    }

    let mut reduced = Vec::with_capacity(png.data.len());
    let byte_depth = png.bytes_per_channel();
    let bpp = png.channels_per_pixel() * byte_depth;
    let last_color = 2 * byte_depth;
    for pixel in png.data.chunks_exact(bpp) {
        if byte_depth == 1 {
            if pixel[0] != pixel[1] || pixel[1] != pixel[2] {
                return None;
            }
        } else if pixel[0..2] != pixel[2..4] || pixel[2..4] != pixel[4..6] {
            return None;
        }
        reduced.extend_from_slice(&pixel[last_color..]);
    }

    let color_type = match png.ihdr.color_type {
        ColorType::RGB { transparent_color } => ColorType::Grayscale {
            // Copy the transparent component if it is also gray
            transparent_shade: transparent_color
                .filter(|t| t.r == t.g && t.g == t.b)
                .map(|t| t.r),
        },
        _ => ColorType::GrayscaleAlpha,
    };

    Some(PngImage {
        data: reduced,
        ihdr: IhdrData {
            color_type,
            ..png.ihdr
        },
    })
}

/// Attempt to convert indexed to a different color type, returning the resulting image if successful
#[must_use]
pub fn indexed_to_channels(
    png: &PngImage,
    allow_grayscale: bool,
    optimize_alpha: bool,
) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Eight {
        return None;
    }
    let mut palette = match &png.ihdr.color_type {
        ColorType::Indexed { palette } => palette.clone(),
        _ => return None,
    };

    // Ensure fully transparent colors are black, which can help with grayscale conversion
    if optimize_alpha {
        for color in &mut palette {
            if color.a == 0 {
                color.r = 0;
                color.g = 0;
                color.b = 0;
            }
        }
    }

    // Determine which channels are required
    let is_gray = if allow_grayscale {
        palette.iter().all(|c| c.r == c.g && c.g == c.b)
    } else {
        false
    };
    let has_alpha = palette.iter().any(|c| c.a != 255);
    let color_type = match (is_gray, has_alpha) {
        (false, true) => ColorType::RGBA,
        (false, false) => ColorType::RGB {
            transparent_color: None,
        },
        (true, true) => ColorType::GrayscaleAlpha,
        (true, false) => ColorType::Grayscale {
            transparent_shade: None,
        },
    };

    // Don't proceed if output would be too much larger
    let out_size = color_type.channels_per_pixel() as usize * png.data.len();
    if out_size - png.data.len() > INDEXED_MAX_DIFF {
        return None;
    }

    // Construct the new data
    let black = RGBA::new(0, 0, 0, 255);
    let ch_start = if is_gray { 2 } else { 0 };
    let ch_end = if has_alpha { 3 } else { 2 };
    let mut data = Vec::with_capacity(out_size);
    for b in &png.data {
        let color = palette.get(*b as usize).unwrap_or(&black);
        data.extend_from_slice(&color.as_slice()[ch_start..=ch_end]);
    }

    Some(PngImage {
        ihdr: IhdrData {
            color_type,
            ..png.ihdr
        },
        data,
    })
}
