use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use oxipng::{internal_tests::*, *};

const RGB: u8 = 2;
const INDEXED: u8 = 3;

fn get_opts(input: &Path) -> (OutFile, oxipng::Options) {
    let options = oxipng::Options {
        force: true,
        filters: indexset! {FilterStrategy::NONE},
        ..Default::default()
    };
    (OutFile::from_path(input.with_extension("out.png")), options)
}

fn test_it_converts(
    input: &str,
    interlace: bool,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) {
    let input = PathBuf::from(input);
    let (output, mut opts) = get_opts(&input);
    let png = PngData::new(&input, &opts).unwrap();
    opts.interlace = Some(interlace);
    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_in);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in);
    assert_eq!(png.raw.ihdr.interlaced, !interlace);

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
fn deinterlace_rgb_16() {
    test_it_converts(
        "tests/files/interlaced_rgb_16_should_be_rgb_16.png",
        false,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn deinterlace_rgb_8() {
    test_it_converts(
        "tests/files/interlaced_rgb_8_should_be_rgb_8.png",
        false,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn deinterlace_palette_8() {
    test_it_converts(
        "tests/files/interlaced_palette_8_should_be_palette_8.png",
        false,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn deinterlace_palette_4() {
    test_it_converts(
        "tests/files/interlaced_palette_4_should_be_palette_4.png",
        false,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn deinterlace_palette_2() {
    test_it_converts(
        "tests/files/interlaced_palette_2_should_be_palette_2.png",
        false,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn deinterlace_palette_1() {
    test_it_converts(
        "tests/files/interlaced_palette_1_should_be_palette_1.png",
        false,
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn interlace_rgb_16() {
    test_it_converts(
        "tests/files/rgb_16_should_be_rgb_16.png",
        true,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn interlace_rgb_8() {
    test_it_converts(
        "tests/files/rgb_8_should_be_rgb_8.png",
        true,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn interlace_palette_8() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_8.png",
        true,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn interlace_palette_4() {
    test_it_converts(
        "tests/files/palette_4_should_be_palette_4.png",
        true,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn interlace_palette_2() {
    test_it_converts(
        "tests/files/palette_2_should_be_palette_2.png",
        true,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn interlace_palette_1() {
    test_it_converts(
        "tests/files/palette_1_should_be_palette_1.png",
        true,
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}
