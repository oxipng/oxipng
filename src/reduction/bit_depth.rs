use crate::{
    colors::{BitDepth, ColorType},
    headers::IhdrData,
    png::PngImage,
};

/// Attempt to reduce a 16-bit image to 8-bit, returning the reduced image if successful
#[must_use]
pub fn reduced_bit_depth_16_to_8(png: &PngImage, force_scale: bool) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Sixteen {
        return None;
    }

    if force_scale {
        return scaled_bit_depth_16_to_8(png);
    }

    // Reduce from 16 to 8 bits per channel per pixel
    if png.data.chunks_exact(2).any(|pair| pair[0] != pair[1]) {
        // Can't reduce
        return None;
    }

    Some(PngImage {
        data: png.data.chunks_exact(2).map(|pair| pair[0]).collect(),
        ihdr: IhdrData {
            color_type: png.ihdr.color_type.clone(),
            bit_depth: BitDepth::Eight,
            ..png.ihdr
        },
    })
}

/// Forcibly reduce a 16-bit image to 8-bit by scaling, returning the reduced image if successful
#[must_use]
pub fn scaled_bit_depth_16_to_8(png: &PngImage) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Sixteen {
        return None;
    }

    // Reduce from 16 to 8 bits per channel per pixel by scaling when necessary
    let data = png
        .data
        .chunks_exact(2)
        .map(|pair| {
            if pair[0] == pair[1] {
                return pair[0];
            }
            // See: http://www.libpng.org/pub/png/spec/1.2/PNG-Decoders.html#D.Sample-depth-rescaling
            // This allows values such as 0x00FF to be rounded to 0x01 rather than truncated to 0x00
            let val = f32::from(u16::from_be_bytes([pair[0], pair[1]]));
            (val * (255.0 / 65535.0)).round() as u8
        })
        .collect();

    Some(PngImage {
        data,
        ihdr: IhdrData {
            color_type: png.ihdr.color_type.clone(),
            bit_depth: BitDepth::Eight,
            ..png.ihdr
        },
    })
}

/// Attempt to reduce an 8-bit image to a lower bit depth, returning the reduced image if successful
#[must_use]
pub fn reduced_bit_depth_8_or_less(png: &PngImage) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Eight || png.channels_per_pixel() != 1 {
        return None;
    }

    let minimum_bits = if let ColorType::Indexed { palette } = &png.ihdr.color_type {
        // We can easily determine minimum depth by the palette size
        match palette.len() {
            0..=2 => 1,
            3..=4 => 2,
            5..=16 => 4,
            _ => return None,
        }
    } else {
        let mut minimum_bits = 1;

        // Finding minimum depth for grayscale is much more complicated
        let mut mask = 1;
        let mut divisions = 1..8;
        for &b in &png.data {
            if b == 0 || b == 255 {
                continue;
            }
            'try_depth: loop {
                // Align the first pixel division with the mask
                let mut byte = b.rotate_left(minimum_bits as u32);
                // Each potential division of this pixel must be identical to successfully reduce
                let compare = byte & mask;
                for _ in divisions.clone() {
                    // Align the next division with the mask
                    byte = byte.rotate_left(minimum_bits as u32);
                    if byte & mask != compare {
                        // This depth is not possible, try the next one up
                        minimum_bits <<= 1;
                        if minimum_bits == 8 {
                            return None;
                        }
                        mask = (1 << minimum_bits) - 1;
                        divisions = 1..(8 / minimum_bits);
                        continue 'try_depth;
                    }
                }
                break;
            }
        }

        minimum_bits
    };

    let mut reduced = Vec::with_capacity(png.data.len());
    let mask = (1 << minimum_bits) - 1;
    for line in png.scan_lines(false) {
        // Loop over the data in chunks that will produce 1 byte of output
        for chunk in line.data.chunks(8 / minimum_bits) {
            let mut new_byte = 0;
            let mut shift = 8;
            for byte in chunk {
                shift -= minimum_bits;
                // Take the low bits of the pixel and shift them into the output byte
                new_byte |= (byte & mask) << shift;
            }
            reduced.push(new_byte);
        }
    }

    // If the image is grayscale we also need to reduce the transparency pixel
    let color_type = if let ColorType::Grayscale {
        transparent_shade: Some(trans),
    } = png.ihdr.color_type
    {
        let reduced_trans = (trans & 0xFF) >> (8 - minimum_bits);
        // Verify the reduction is valid by restoring back to original bit depth
        let mut check = reduced_trans;
        let mut bits = minimum_bits;
        while bits < 8 {
            check = (check << bits) | check;
            bits <<= 1;
        }
        // If the transparency doesn't fit the new bit depth it is therefore unused - set it to None
        ColorType::Grayscale {
            transparent_shade: if trans == check {
                Some(reduced_trans)
            } else {
                None
            },
        }
    } else {
        png.ihdr.color_type.clone()
    };

    Some(PngImage {
        data: reduced,
        ihdr: IhdrData {
            color_type,
            bit_depth: (minimum_bits as u8).try_into().unwrap(),
            ..png.ihdr
        },
    })
}

/// Expand a 1/2/4-bit image to 8-bit, returning the expanded image if successful
#[must_use]
pub fn expanded_bit_depth_to_8(png: &PngImage) -> Option<PngImage> {
    let bit_depth = png.ihdr.bit_depth as u32;
    if bit_depth >= 8 {
        return None;
    }
    // Calculate the current number of pixels per byte
    let ppb = 8 / bit_depth;
    let is_gray = matches!(png.ihdr.color_type, ColorType::Grayscale { .. });

    let mut reduced = Vec::with_capacity((png.ihdr.width * png.ihdr.height) as usize);
    let mut length = 0;
    let mask = (1 << bit_depth) - 1;
    for line in png.scan_lines(false) {
        for &(mut byte) in line.data {
            // Loop over each pixel in the byte
            for _ in 0..ppb {
                // Align the current pixel with the mask
                byte = byte.rotate_left(bit_depth);
                let mut val = byte & mask;
                if is_gray {
                    // Expand gray by repeating the bits
                    let mut bits = bit_depth;
                    while bits < 8 {
                        val = (val << bits) | val;
                        bits <<= 1;
                    }
                }
                reduced.push(val);
            }
        }
        // Trim any overflow
        length += line.num_pixels;
        reduced.truncate(length);
    }

    // If the image is grayscale we also need to expand the transparency pixel
    let color_type = if let ColorType::Grayscale {
        transparent_shade: Some(mut trans),
    } = png.ihdr.color_type
    {
        let mut bits = bit_depth;
        while bits < 8 {
            trans = (trans << bits) | trans;
            bits <<= 1;
        }
        ColorType::Grayscale {
            transparent_shade: Some(trans),
        }
    } else {
        png.ihdr.color_type.clone()
    };

    Some(PngImage {
        data: reduced,
        ihdr: IhdrData {
            color_type,
            bit_depth: BitDepth::Eight,
            ..png.ihdr
        },
    })
}
