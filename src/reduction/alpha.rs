use rgb::RGB16;

use crate::{
    colors::{BitDepth, ColorType},
    headers::IhdrData,
    png::PngImage,
};

/// Clean the alpha channel by setting the color of all fully transparent pixels to black
#[must_use]
pub fn cleaned_alpha_channel(png: &PngImage) -> Option<PngImage> {
    if !png.ihdr.color_type.has_alpha() {
        return None;
    }
    let byte_depth = png.bytes_per_channel();
    let bpp = png.channels_per_pixel() * byte_depth;
    let colored_bytes = bpp - byte_depth;

    let mut reduced = Vec::with_capacity(png.data.len());
    for pixel in png.data.chunks_exact(bpp) {
        if pixel.iter().skip(colored_bytes).all(|b| *b == 0) {
            reduced.resize(reduced.len() + bpp, 0);
        } else {
            reduced.extend_from_slice(pixel);
        }
    }

    Some(PngImage {
        data: reduced,
        ihdr: png.ihdr.clone(),
    })
}

#[must_use]
pub fn reduced_alpha_channel(png: &PngImage, optimize_alpha: bool) -> Option<PngImage> {
    if !png.ihdr.color_type.has_alpha() {
        return None;
    }
    let byte_depth = png.bytes_per_channel();
    let bpp = png.channels_per_pixel() * byte_depth;
    let colored_bytes = bpp - byte_depth;

    // If alpha optimisation is enabled, see if the image contains only fully opaque and fully transparent pixels.
    // In case this occurs, we want to try and find an unused color we can use for the tRNS chunk.
    // Rather than an exhaustive search, we will just keep track of 256 shades of gray, which should cover many cases.
    let mut has_transparency = false;
    let mut used_colors = vec![false; 256];

    for pixel in png.data.chunks_exact(bpp) {
        if optimize_alpha && pixel.iter().skip(colored_bytes).all(|b| *b == 0) {
            // Fully transparent, we may be able to reduce with tRNS
            has_transparency = true;
        } else if pixel.iter().skip(colored_bytes).any(|b| *b != 255) {
            // Partially transparent, the image is not reducible
            return None;
        } else if optimize_alpha && pixel.iter().take(colored_bytes).all(|b| *b == pixel[0]) {
            // Opaque shade of gray, we can't use this color for tRNS
            used_colors[pixel[0] as usize] = true;
        }
    }

    let transparency_pixel = if has_transparency {
        // For grayscale, start by checking 4 specific values in the hope that we may reduce depth
        let unused = match png.ihdr.color_type {
            ColorType::GrayscaleAlpha => [0x00, 0xFF, 0x55, 0xAA]
                .into_iter()
                .find(|&v| !used_colors[v as usize]),
            _ => None,
        }
        .or_else(|| used_colors.iter().position(|&u| !u).map(|v| v as u8));
        // If no unused color was found we will have to fail here
        Some(unused?)
    } else {
        None
    };

    let mut raw_data = Vec::with_capacity(png.data.len());
    for pixel in png.data.chunks_exact(bpp) {
        match transparency_pixel {
            Some(trns) if pixel.iter().skip(colored_bytes).all(|b| *b == 0) => {
                raw_data.resize(raw_data.len() + colored_bytes, trns);
            }
            _ => raw_data.extend_from_slice(&pixel[0..colored_bytes]),
        }
    }

    // Construct the color type with appropriate transparency data
    let transparent = transparency_pixel.map(|trns| match png.ihdr.bit_depth {
        BitDepth::Sixteen => (u16::from(trns) << 8) | u16::from(trns),
        _ => u16::from(trns),
    });
    let target_color_type = match png.ihdr.color_type {
        ColorType::GrayscaleAlpha => ColorType::Grayscale {
            transparent_shade: transparent,
        },
        _ => ColorType::RGB {
            transparent_color: transparent.map(|t| RGB16::new(t, t, t)),
        },
    };

    Some(PngImage {
        data: raw_data,
        ihdr: IhdrData {
            color_type: target_color_type,
            ..png.ihdr
        },
    })
}
