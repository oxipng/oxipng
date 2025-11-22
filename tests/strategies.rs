use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use oxipng::{internal_tests::*, *};

const GRAYSCALE: u8 = 0;
const RGB: u8 = 2;
const INDEXED: u8 = 3;
const RGBA: u8 = 6;

fn get_opts(input: &Path) -> (OutFile, oxipng::Options) {
    let options = oxipng::Options {
        force: true,
        ..Default::default()
    };
    (OutFile::from_path(input.with_extension("out.png")), options)
}

fn test_it_converts(
    input: &str,
    filter: FilterStrategy,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) {
    let input = PathBuf::from(input);

    let (output, mut opts) = get_opts(&input);
    let png = PngData::new(&input, &opts).unwrap();
    opts.filters = indexset! {filter};
    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_in);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in);

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
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert!(palette.len() <= 1 << (png.raw.ihdr.bit_depth as u8));
    }

    remove_file(output).ok();
}

#[test]
fn filter_minsum() {
    test_it_converts(
        "tests/files/rgb_16_should_be_rgb_16.png",
        FilterStrategy::MinSum,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_entropy() {
    test_it_converts(
        "tests/files/rgb_8_should_be_rgb_8.png",
        FilterStrategy::Entropy,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_bigrams() {
    test_it_converts(
        "tests/files/rgba_8_should_be_rgba_8.png",
        FilterStrategy::Bigrams,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_bigent() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_grayscale_8.png",
        FilterStrategy::BigEnt,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_brute() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_8.png",
        FilterStrategy::Brute {
            num_lines: 4,
            level: 1,
        },
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}
