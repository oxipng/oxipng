use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use oxipng::{internal_tests::*, *};

const GRAYSCALE: u8 = 0;
const RGB: u8 = 2;
const INDEXED: u8 = 3;
const GRAYSCALE_ALPHA: u8 = 4;
const RGBA: u8 = 6;

fn get_opts(input: &Path) -> (OutFile, oxipng::Options) {
    let options = oxipng::Options {
        force: true,
        fast_evaluation: false,
        filters: indexset! {FilterStrategy::NONE},
        interlace: None,
        ..Default::default()
    };
    (OutFile::from_path(input.with_extension("out.png")), options)
}

fn test_it_converts(
    input: &str,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) {
    let input = PathBuf::from(input);
    let (output, opts) = get_opts(&input);
    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_in);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in);
    assert!(png.raw.ihdr.interlaced);

    match oxipng::optimize(&InFile::Path(input), &output, &opts) {
        Ok(_) => (),
        Err(x) => panic!("{}", x),
    }
    let output = output.path().unwrap();
    assert!(output.exists());

    let png = match PngData::new(output, &opts) {
        Ok(x) => x,
        Err(x) => {
            remove_file(output).ok();
            panic!("{}", x)
        }
    };

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_out);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_out);

    remove_file(output).ok();
}

#[test]
fn interlaced_rgba_16_should_be_rgba_16() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_rgba_16.png",
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgba_16_should_be_rgba_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_rgba_8.png",
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_8_should_be_rgba_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_rgba_8.png",
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_16_should_be_rgb_16() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_rgb_16.png",
        RGBA,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgba_16_should_be_rgb_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_rgb_8.png",
        RGBA,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_8_should_be_rgb_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_rgb_8.png",
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_16_should_be_palette_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_palette_8.png",
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_palette_8.png",
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_16_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_palette_4.png",
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_rgba_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_palette_4.png",
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_rgba_16_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_palette_2.png",
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_rgba_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_palette_2.png",
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_rgba_16_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_palette_1.png",
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_rgba_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_palette_1.png",
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_rgba_16_should_be_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_grayscale_alpha_16.png",
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgba_16_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_grayscale_alpha_8.png",
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_8_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_grayscale_alpha_8.png",
        RGBA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_grayscale_16.png",
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgba_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_16_should_be_grayscale_8.png",
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgba_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_rgba_8_should_be_grayscale_8.png",
        RGBA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_16_should_be_rgb_16() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_rgb_16.png",
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgb_16_should_be_rgb_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_rgb_8.png",
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_8_should_be_rgb_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_rgb_8.png",
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_16_should_be_palette_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_palette_8.png",
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_palette_8.png",
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_16_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_palette_4.png",
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_rgb_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_palette_4.png",
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_rgb_16_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_palette_2.png",
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_rgb_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_palette_2.png",
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_rgb_16_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_palette_1.png",
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_rgb_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_palette_1.png",
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_rgb_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_grayscale_16.png",
        RGB,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_rgb_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_grayscale_8.png",
        RGB,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_rgb_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_grayscale_8.png",
        RGB,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_palette_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_palette_8.png",
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_palette_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_palette_4.png",
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_palette_4_should_be_palette_4() {
    test_it_converts(
        "tests/files/interlaced_palette_4_should_be_palette_4.png",
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlaced_palette_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_palette_2.png",
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_palette_4_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_palette_4_should_be_palette_2.png",
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_palette_2_should_be_palette_2() {
    test_it_converts(
        "tests/files/interlaced_palette_2_should_be_palette_2.png",
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlaced_palette_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_palette_1.png",
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_palette_4_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_palette_4_should_be_palette_1.png",
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_palette_2_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_palette_2_should_be_palette_1.png",
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_palette_1_should_be_palette_1() {
    test_it_converts(
        "tests/files/interlaced_palette_1_should_be_palette_1.png",
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlaced_palette_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_grayscale_8.png",
        INDEXED,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_alpha_16_should_be_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_16_should_be_grayscale_alpha_16.png",
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_grayscale_alpha_16_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_16_should_be_grayscale_alpha_8.png",
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_alpha_8_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_8_should_be_grayscale_alpha_8.png",
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_alpha_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_16_should_be_grayscale_16.png",
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_grayscale_alpha_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_16_should_be_grayscale_8.png",
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_alpha_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_alpha_8_should_be_grayscale_8.png",
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/interlaced_grayscale_16_should_be_grayscale_16.png",
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlaced_grayscale_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_16_should_be_grayscale_8.png",
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_grayscale_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/interlaced_grayscale_8_should_be_grayscale_8.png",
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_small_files() {
    test_it_converts(
        "tests/files/interlaced_small_files.png",
        INDEXED,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_vertical_filters() {
    test_it_converts(
        "tests/files/interlaced_vertical_filters.png",
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlaced_odd_width() {
    test_it_converts(
        "tests/files/interlaced_odd_width.png",
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}
