use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use oxipng::{internal_tests::*, *};

const GRAYSCALE_ALPHA: u8 = 4;
const RGBA: u8 = 6;

fn get_opts(input: &Path) -> (OutFile, oxipng::Options) {
    let options = oxipng::Options {
        force: true,
        optimize_alpha: true,
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
fn alpha_filter_0_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_0_for_rgba_16.png",
        FilterStrategy::NONE,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_1_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_1_for_rgba_16.png",
        FilterStrategy::SUB,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_2_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_2_for_rgba_16.png",
        FilterStrategy::UP,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_3_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_3_for_rgba_16.png",
        FilterStrategy::AVERAGE,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_4_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_4_for_rgba_16.png",
        FilterStrategy::PAETH,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_5_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_5_for_rgba_16.png",
        FilterStrategy::MinSum,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_0_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_0_for_rgba_8.png",
        FilterStrategy::NONE,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_1_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_1_for_rgba_8.png",
        FilterStrategy::SUB,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_2_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_2_for_rgba_8.png",
        FilterStrategy::UP,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_3_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_3_for_rgba_8.png",
        FilterStrategy::AVERAGE,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_4_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_4_for_rgba_8.png",
        FilterStrategy::PAETH,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_5_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_5_for_rgba_8.png",
        FilterStrategy::MinSum,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_0_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_alpha_16.png",
        FilterStrategy::NONE,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_1_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_alpha_16.png",
        FilterStrategy::SUB,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_2_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_alpha_16.png",
        FilterStrategy::UP,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_3_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_alpha_16.png",
        FilterStrategy::AVERAGE,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_4_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_alpha_16.png",
        FilterStrategy::PAETH,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_5_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_5_for_grayscale_alpha_16.png",
        FilterStrategy::MinSum,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn alpha_filter_0_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_alpha_8.png",
        FilterStrategy::NONE,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_1_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_alpha_8.png",
        FilterStrategy::SUB,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_2_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_alpha_8.png",
        FilterStrategy::UP,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_3_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_alpha_8.png",
        FilterStrategy::AVERAGE,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_4_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_alpha_8.png",
        FilterStrategy::PAETH,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn alpha_filter_5_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_5_for_grayscale_alpha_8.png",
        FilterStrategy::MinSum,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}
