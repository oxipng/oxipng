use bitvec::prelude::*;

use crate::{headers::IhdrData, png::PngImage};

#[must_use]
pub fn interlace_image(png: &PngImage) -> PngImage {
    let mut passes: Vec<BitVec<u8, Msb0>> = vec![BitVec::new(); 7];
    let bits_per_pixel = png.ihdr.bpp();
    for (index, line) in png.scan_lines(false).enumerate() {
        let bit_vec = line.data.view_bits::<Msb0>();
        for (i, bit) in bit_vec.iter().by_vals().enumerate() {
            // Avoid moving padded 0's into new image
            if i >= (png.ihdr.width as usize * bits_per_pixel) {
                break;
            }
            // Copy pixels into interlaced passes
            let pix_modulo = (i / bits_per_pixel) % 8;
            match index % 8 {
                0 => match pix_modulo {
                    0 => passes[0].push(bit),
                    4 => passes[1].push(bit),
                    2 | 6 => passes[3].push(bit),
                    _ => passes[5].push(bit),
                },
                4 => match pix_modulo {
                    0 | 4 => passes[2].push(bit),
                    2 | 6 => passes[3].push(bit),
                    _ => passes[5].push(bit),
                },
                2 | 6 => match pix_modulo % 2 {
                    0 => passes[4].push(bit),
                    _ => passes[5].push(bit),
                },
                _ => {
                    passes[6].push(bit);
                }
            }
        }
        // Pad end of line on each pass to get 8 bits per byte
        for pass in &mut passes {
            while pass.len() % 8 != 0 {
                pass.push(false);
            }
        }
    }

    let mut output = Vec::with_capacity(png.data.len());
    for pass in &passes {
        output.extend_from_slice(pass.as_raw_slice());
    }

    PngImage {
        data: output,
        ihdr: IhdrData {
            color_type: png.ihdr.color_type.clone(),
            interlaced: true,
            ..png.ihdr
        },
    }
}

pub fn deinterlace_image(png: &PngImage) -> PngImage {
    PngImage {
        data: match png.ihdr.bpp() {
            8.. => deinterlace_bytes(png),
            _ => deinterlace_bits(png),
        },
        ihdr: IhdrData {
            color_type: png.ihdr.color_type.clone(),
            interlaced: false,
            ..png.ihdr
        },
    }
}

/// Deinterlace by bits, for images with less than 8bpp
fn deinterlace_bits(png: &PngImage) -> Vec<u8> {
    let bits_per_pixel = png.ihdr.bpp();
    let bits_per_line = bits_per_pixel * png.ihdr.width as usize;
    // Initialize each output line with blank data
    let mut lines: Vec<BitVec<u8, Msb0>> =
        vec![bitvec![u8, Msb0; 0; bits_per_line]; png.ihdr.height as usize];
    let mut current_pass = 1;
    let mut pass_constants = interlaced_constants(current_pass);
    let mut current_y: usize = pass_constants.y_shift as usize;
    for line in png.scan_lines(false) {
        let bit_vec = line.data.view_bits::<Msb0>();
        let bits_in_line = (png.ihdr.width - u32::from(pass_constants.x_shift))
            .div_ceil(u32::from(pass_constants.x_step)) as usize
            * bits_per_pixel;
        for (i, bit) in bit_vec.iter().by_vals().enumerate() {
            // Avoid moving padded 0's into new image
            if i >= bits_in_line {
                break;
            }
            let current_x: usize = pass_constants.x_shift as usize
                + (i / bits_per_pixel) * pass_constants.x_step as usize;
            // Copy this bit into the output line
            let index = (i % bits_per_pixel) + current_x * bits_per_pixel;
            lines[current_y].set(index, bit);
        }
        // Calculate the next line and move to next pass if necessary
        current_y += pass_constants.y_step as usize;
        if current_y >= png.ihdr.height as usize {
            if !increment_pass(&mut current_pass, &png.ihdr) {
                break;
            }
            pass_constants = interlaced_constants(current_pass);
            current_y = pass_constants.y_shift as usize;
        }
    }
    let mut output = Vec::with_capacity(png.data.len());
    for line in &mut lines {
        while line.len() % 8 != 0 {
            line.push(false);
        }
        output.extend_from_slice(line.as_raw_slice());
    }
    output
}

/// Deinterlace by bytes, for images with at least 8bpp
fn deinterlace_bytes(png: &PngImage) -> Vec<u8> {
    let bytes_per_pixel = png.ihdr.bpp() / 8;
    let bytes_per_line = bytes_per_pixel * png.ihdr.width as usize;
    // Initialize each output line with some blank data
    let mut lines: Vec<Vec<u8>> = vec![vec![0; bytes_per_line]; png.ihdr.height as usize];
    let mut current_pass = 1;
    let mut pass_constants = interlaced_constants(current_pass);
    let mut current_y: usize = pass_constants.y_shift as usize;
    for line in png.scan_lines(false) {
        for (i, byte) in line.data.iter().enumerate() {
            let current_x: usize = pass_constants.x_shift as usize
                + (i / bytes_per_pixel) * pass_constants.x_step as usize;
            // Copy this byte into the output line
            let index = (i % bytes_per_pixel) + current_x * bytes_per_pixel;
            lines[current_y][index] = *byte;
        }
        // Calculate the next line and move to next pass if necessary
        current_y += pass_constants.y_step as usize;
        if current_y >= png.ihdr.height as usize {
            if !increment_pass(&mut current_pass, &png.ihdr) {
                break;
            }
            pass_constants = interlaced_constants(current_pass);
            current_y = pass_constants.y_shift as usize;
        }
    }
    lines.concat()
}

const fn increment_pass(current_pass: &mut u8, ihdr: &IhdrData) -> bool {
    if *current_pass == 7 {
        return false;
    }
    *current_pass += 1;
    if *current_pass == 2 && ihdr.width <= 4 {
        *current_pass += 1;
    }
    if *current_pass == 3 && ihdr.height <= 4 {
        *current_pass += 1;
    }
    if *current_pass == 4 && ihdr.width <= 2 {
        *current_pass += 1;
    }
    if *current_pass == 5 && ihdr.height <= 2 {
        *current_pass += 1;
    }
    if *current_pass == 6 && ihdr.width == 1 {
        *current_pass += 1;
    }
    if *current_pass == 7 && ihdr.height == 1 {
        return false;
    }
    true
}

#[derive(Clone, Copy)]
struct InterlacedConstants {
    x_shift: u8,
    y_shift: u8,
    x_step: u8,
    y_step: u8,
}

fn interlaced_constants(pass: u8) -> InterlacedConstants {
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
