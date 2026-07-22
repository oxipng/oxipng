use crate::{headers::IhdrData, png::PngImage, reduction::bit_depth::*};

/// Enable or disable interlacing, returning the new image if it was changed
#[must_use]
pub fn changed_interlacing(png: &PngImage, interlace: bool) -> Option<PngImage> {
    if interlace == png.ihdr.interlaced {
        return None;
    }

    // Performing the transformation at the bit level can be complex and inefficient, so we only
    // directly support 8-bit and higher. A low depth image would normally already be expanded to 8
    // when we run this, but if depth reductions were disabled we can just expand it temporarily
    // and then revert back again afterward (it's still very fast).
    let orig_depth = png.ihdr.bit_depth;
    let expanded = expanded_bit_depth_to_8(png);
    let png = expanded.as_ref().unwrap_or(png);
    let data = if interlace {
        interlace_bytes(png)
    } else {
        deinterlace_bytes(png)
    };
    let mut new = PngImage {
        data,
        ihdr: IhdrData {
            color_type: png.ihdr.color_type.clone(),
            interlaced: interlace,
            ..png.ihdr
        },
    };
    // Reduce back to original depth
    if new.ihdr.bit_depth != orig_depth {
        new = reduced_bit_depth_forced(&new, orig_depth);
    }
    Some(new)
}

/// Interlace by bytes, for images with at least 8bpp
fn interlace_bytes(png: &PngImage) -> Vec<u8> {
    let bytes_per_pixel = png.ihdr.bpp() / 8;
    match bytes_per_pixel {
        1 => interlace_bytes_const::<1>(png),
        2 => interlace_bytes_const::<2>(png),
        3 => interlace_bytes_const::<3>(png),
        4 => interlace_bytes_const::<4>(png),
        6 => interlace_bytes_const::<6>(png),
        8 => interlace_bytes_const::<8>(png),
        _ => unreachable!(),
    }
}

// Delegate function with const generics for performance
fn interlace_bytes_const<const BPP: usize>(png: &PngImage) -> Vec<u8> {
    let mut passes: Vec<Vec<u8>> = vec![Vec::new(); 7];
    for (y, line) in png.scan_lines(false).enumerate() {
        for (x, pixel) in line.data.as_chunks::<BPP>().0.iter().enumerate() {
            // Copy pixels into interlaced passes
            match (y % 8, x % 8) {
                (0, 0) => passes[0].extend_from_slice(pixel),
                (0, 4) => passes[1].extend_from_slice(pixel),
                (4, 0 | 4) => passes[2].extend_from_slice(pixel),
                (0 | 4, 2 | 6) => passes[3].extend_from_slice(pixel),
                (2 | 6, _) if x % 2 == 0 => passes[4].extend_from_slice(pixel),
                _ if y % 2 == 0 => passes[5].extend_from_slice(pixel),
                _ => passes[6].extend_from_slice(pixel),
            }
        }
    }
    passes.concat()
}

/// Deinterlace by bytes, for images with at least 8bpp
fn deinterlace_bytes(png: &PngImage) -> Vec<u8> {
    let bytes_per_pixel = png.ihdr.bpp() / 8;
    match bytes_per_pixel {
        1 => deinterlace_bytes_const::<1>(png),
        2 => deinterlace_bytes_const::<2>(png),
        3 => deinterlace_bytes_const::<3>(png),
        4 => deinterlace_bytes_const::<4>(png),
        6 => deinterlace_bytes_const::<6>(png),
        8 => deinterlace_bytes_const::<8>(png),
        _ => unreachable!(),
    }
}

// Delegate function with const generics for performance
fn deinterlace_bytes_const<const BPP: usize>(png: &PngImage) -> Vec<u8> {
    let bytes_per_pixel = BPP;
    let bytes_per_line = bytes_per_pixel * png.ihdr.width as usize;
    // Initialize the output data
    let mut data: Vec<u8> = vec![0; bytes_per_line * png.ihdr.height as usize];
    let mut current_pass = 1;
    let mut pass_constants = interlaced_constants(current_pass);
    let mut current_y: usize = pass_constants.y_shift as usize;
    for line in png.scan_lines(false) {
        for (i, pixel) in line.data.as_chunks::<BPP>().0.iter().enumerate() {
            let current_x = pass_constants.x_shift as usize + i * pass_constants.x_step as usize;
            // Copy this byte into the output line
            let index = current_y * bytes_per_line + current_x * bytes_per_pixel;
            data[index..(index + BPP)].copy_from_slice(pixel);
        }
        // Calculate the next line and move to next pass if necessary
        current_y += pass_constants.y_step as usize;
        if current_y >= png.ihdr.height as usize {
            if current_pass == 7 {
                break;
            }
            current_pass += 1;
            if current_pass == 2 && png.ihdr.width <= 4 {
                current_pass += 1;
            }
            if current_pass == 3 && png.ihdr.height <= 4 {
                current_pass += 1;
            }
            if current_pass == 4 && png.ihdr.width <= 2 {
                current_pass += 1;
            }
            if current_pass == 5 && png.ihdr.height <= 2 {
                current_pass += 1;
            }
            if current_pass == 6 && png.ihdr.width == 1 {
                current_pass += 1;
            }
            if current_pass == 7 && png.ihdr.height == 1 {
                break;
            }
            pass_constants = interlaced_constants(current_pass);
            current_y = pass_constants.y_shift as usize;
        }
    }
    data
}

#[derive(Clone, Copy)]
struct InterlacedConstants {
    x_shift: u8,
    y_shift: u8,
    x_step: u8,
    y_step: u8,
}

const fn interlaced_constants(pass: u8) -> InterlacedConstants {
    match pass {
        1 => InterlacedConstants {
            x_shift: 0,
            y_shift: 0,
            x_step: 8,
            y_step: 8,
        },
        2 => InterlacedConstants {
            x_shift: 4,
            y_shift: 0,
            x_step: 8,
            y_step: 8,
        },
        3 => InterlacedConstants {
            x_shift: 0,
            y_shift: 4,
            x_step: 4,
            y_step: 8,
        },
        4 => InterlacedConstants {
            x_shift: 2,
            y_shift: 0,
            x_step: 4,
            y_step: 4,
        },
        5 => InterlacedConstants {
            x_shift: 0,
            y_shift: 2,
            x_step: 2,
            y_step: 4,
        },
        6 => InterlacedConstants {
            x_shift: 1,
            y_shift: 0,
            x_step: 2,
            y_step: 2,
        },
        7 => InterlacedConstants {
            x_shift: 0,
            y_shift: 1,
            x_step: 1,
            y_step: 2,
        },
        _ => unreachable!(),
    }
}
