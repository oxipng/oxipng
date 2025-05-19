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
        filter: indexset! {FilterStrategy::Basic(RowFilter::None)},
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
    opts.filter = IndexSet::new();
    opts.filter.insert(filter);
    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_in);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in);

    match oxipng::optimize(&InFile::Path(input), &output, &opts) {
        Ok(_) => (),
        Err(x) => panic!("{}", x),
    };
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
fn filter_0_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_0_for_rgba_16.png",
        FilterStrategy::Basic(RowFilter::None),
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_1_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_1_for_rgba_16.png",
        FilterStrategy::Basic(RowFilter::Sub),
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_2_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_2_for_rgba_16.png",
        FilterStrategy::Basic(RowFilter::Up),
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_3_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_3_for_rgba_16.png",
        FilterStrategy::Basic(RowFilter::Average),
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_4_for_rgba_16() {
    test_it_converts(
        "tests/files/filter_4_for_rgba_16.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_5_for_rgba_16() {
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
fn filter_0_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_0_for_rgba_8.png",
        FilterStrategy::Basic(RowFilter::None),
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_1_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_1_for_rgba_8.png",
        FilterStrategy::Basic(RowFilter::Sub),
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_2_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_2_for_rgba_8.png",
        FilterStrategy::Basic(RowFilter::Up),
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_3_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_3_for_rgba_8.png",
        FilterStrategy::Basic(RowFilter::Average),
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_4_for_rgba_8() {
    test_it_converts(
        "tests/files/filter_4_for_rgba_8.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_5_for_rgba_8() {
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
fn filter_0_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_0_for_rgb_16.png",
        FilterStrategy::Basic(RowFilter::None),
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_1_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_1_for_rgb_16.png",
        FilterStrategy::Basic(RowFilter::Sub),
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_2_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_2_for_rgb_16.png",
        FilterStrategy::Basic(RowFilter::Up),
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_3_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_3_for_rgb_16.png",
        FilterStrategy::Basic(RowFilter::Average),
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_4_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_4_for_rgb_16.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_5_for_rgb_16() {
    test_it_converts(
        "tests/files/filter_5_for_rgb_16.png",
        FilterStrategy::MinSum,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_0_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_0_for_rgb_8.png",
        FilterStrategy::Basic(RowFilter::None),
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_1_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_1_for_rgb_8.png",
        FilterStrategy::Basic(RowFilter::Sub),
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_2_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_2_for_rgb_8.png",
        FilterStrategy::Basic(RowFilter::Up),
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_3_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_3_for_rgb_8.png",
        FilterStrategy::Basic(RowFilter::Average),
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_4_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_4_for_rgb_8.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_5_for_rgb_8() {
    test_it_converts(
        "tests/files/filter_5_for_rgb_8.png",
        FilterStrategy::MinSum,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn filter_0_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_alpha_16.png",
        FilterStrategy::Basic(RowFilter::None),
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_1_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_alpha_16.png",
        FilterStrategy::Basic(RowFilter::Sub),
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_2_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_alpha_16.png",
        FilterStrategy::Basic(RowFilter::Up),
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_3_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_alpha_16.png",
        FilterStrategy::Basic(RowFilter::Average),
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_4_for_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_alpha_16.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_5_for_grayscale_alpha_16() {
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
fn filter_0_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_alpha_8.png",
        FilterStrategy::Basic(RowFilter::None),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_1_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_alpha_8.png",
        FilterStrategy::Basic(RowFilter::Sub),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_2_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_alpha_8.png",
        FilterStrategy::Basic(RowFilter::Up),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_3_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_alpha_8.png",
        FilterStrategy::Basic(RowFilter::Average),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_4_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_alpha_8.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_5_for_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/filter_5_for_grayscale_alpha_8.png",
        FilterStrategy::MinSum,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn filter_0_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_16.png",
        FilterStrategy::Basic(RowFilter::None),
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_1_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_16.png",
        FilterStrategy::Basic(RowFilter::Sub),
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_2_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_16.png",
        FilterStrategy::Basic(RowFilter::Up),
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_3_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_16.png",
        FilterStrategy::Basic(RowFilter::Average),
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_4_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_16.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_5_for_grayscale_16() {
    test_it_converts(
        "tests/files/filter_5_for_grayscale_16.png",
        FilterStrategy::MinSum,
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn filter_0_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_0_for_grayscale_8.png",
        FilterStrategy::Basic(RowFilter::None),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_1_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_1_for_grayscale_8.png",
        FilterStrategy::Basic(RowFilter::Sub),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_2_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_2_for_grayscale_8.png",
        FilterStrategy::Basic(RowFilter::Up),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_3_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_3_for_grayscale_8.png",
        FilterStrategy::Basic(RowFilter::Average),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_4_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_4_for_grayscale_8.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_5_for_grayscale_8() {
    test_it_converts(
        "tests/files/filter_5_for_grayscale_8.png",
        FilterStrategy::MinSum,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn filter_0_for_palette_4() {
    test_it_converts(
        "tests/files/filter_0_for_palette_4.png",
        FilterStrategy::Basic(RowFilter::None),
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_1_for_palette_4() {
    test_it_converts(
        "tests/files/filter_1_for_palette_4.png",
        FilterStrategy::Basic(RowFilter::Sub),
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_2_for_palette_4() {
    test_it_converts(
        "tests/files/filter_2_for_palette_4.png",
        FilterStrategy::Basic(RowFilter::Up),
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_3_for_palette_4() {
    test_it_converts(
        "tests/files/filter_3_for_palette_4.png",
        FilterStrategy::Basic(RowFilter::Average),
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_4_for_palette_4() {
    test_it_converts(
        "tests/files/filter_4_for_palette_4.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_5_for_palette_4() {
    test_it_converts(
        "tests/files/filter_5_for_palette_4.png",
        FilterStrategy::MinSum,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn filter_0_for_palette_2() {
    test_it_converts(
        "tests/files/filter_0_for_palette_2.png",
        FilterStrategy::Basic(RowFilter::None),
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_1_for_palette_2() {
    test_it_converts(
        "tests/files/filter_1_for_palette_2.png",
        FilterStrategy::Basic(RowFilter::Sub),
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_2_for_palette_2() {
    test_it_converts(
        "tests/files/filter_2_for_palette_2.png",
        FilterStrategy::Basic(RowFilter::Up),
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_3_for_palette_2() {
    test_it_converts(
        "tests/files/filter_3_for_palette_2.png",
        FilterStrategy::Basic(RowFilter::Average),
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_4_for_palette_2() {
    test_it_converts(
        "tests/files/filter_4_for_palette_2.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_5_for_palette_2() {
    test_it_converts(
        "tests/files/filter_5_for_palette_2.png",
        FilterStrategy::MinSum,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn filter_0_for_palette_1() {
    test_it_converts(
        "tests/files/filter_0_for_palette_1.png",
        FilterStrategy::Basic(RowFilter::None),
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn filter_1_for_palette_1() {
    test_it_converts(
        "tests/files/filter_1_for_palette_1.png",
        FilterStrategy::Basic(RowFilter::Sub),
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn filter_2_for_palette_1() {
    test_it_converts(
        "tests/files/filter_2_for_palette_1.png",
        FilterStrategy::Basic(RowFilter::Up),
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn filter_3_for_palette_1() {
    test_it_converts(
        "tests/files/filter_3_for_palette_1.png",
        FilterStrategy::Basic(RowFilter::Average),
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn filter_4_for_palette_1() {
    test_it_converts(
        "tests/files/filter_4_for_palette_1.png",
        FilterStrategy::Basic(RowFilter::Paeth),
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn filter_5_for_palette_1() {
    test_it_converts(
        "tests/files/filter_5_for_palette_1.png",
        FilterStrategy::MinSum,
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}
