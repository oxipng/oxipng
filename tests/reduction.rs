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
        ..Default::default()
    };
    (OutFile::from_path(input.with_extension("out.png")), options)
}

fn test_it_converts(
    input: &str,
    optimize_alpha: bool,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) {
    let input = PathBuf::from(input);
    let (output, mut opts) = get_opts(&input);
    opts.optimize_alpha = optimize_alpha;
    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), color_type_in);
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in, "test file is broken");
    assert!(!png.raw.ihdr.interlaced);

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
fn rgba_16_should_be_rgba_16() {
    test_it_converts(
        "tests/files/rgba_16_should_be_rgba_16.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgba_16_should_be_rgba_8() {
    test_it_converts(
        "tests/files/rgba_16_should_be_rgba_8.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_should_be_rgba_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_rgba_8.png",
        false,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_16_should_be_rgb_16() {
    test_it_converts(
        "tests/files/rgba_16_should_be_rgb_16.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgba_16_should_be_rgb_8() {
    test_it_converts(
        "tests/files/rgba_16_should_be_rgb_8.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_should_be_rgb_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_rgb_8.png",
        false,
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_16_should_be_rgb_trns_16() {
    test_it_converts(
        "tests/files/rgba_16_should_be_rgb_trns_16.png",
        true,
        RGBA,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgba_8_should_be_rgb_trns_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_rgb_trns_8.png",
        true,
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_16_should_be_palette_8() {
    test_it_converts(
        "tests/files/rgba_16_should_be_palette_8.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_palette_8.png",
        false,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_16_should_be_palette_4() {
    test_it_converts(
        "tests/files/rgba_16_should_be_palette_4.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn rgba_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/rgba_8_should_be_palette_4.png",
        false,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn rgba_16_should_be_palette_2() {
    test_it_converts(
        "tests/files/rgba_16_should_be_palette_2.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn rgba_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/rgba_8_should_be_palette_2.png",
        false,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn rgba_16_should_be_palette_1() {
    test_it_converts(
        "tests/files/rgba_16_should_be_palette_1.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn rgba_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/rgba_8_should_be_palette_1.png",
        false,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn rgba_16_should_be_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/rgba_16_should_be_grayscale_alpha_16.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgba_16_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/rgba_16_should_be_grayscale_alpha_8.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_grayscale_alpha_8.png",
        false,
        RGBA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/rgba_16_should_be_grayscale_16.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgba_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/rgba_16_should_be_grayscale_8.png",
        false,
        RGBA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/rgba_8_should_be_grayscale_8.png",
        false,
        RGBA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_16_should_be_rgb_16() {
    test_it_converts(
        "tests/files/rgb_16_should_be_rgb_16.png",
        false,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgb_16_should_be_rgb_8() {
    test_it_converts(
        "tests/files/rgb_16_should_be_rgb_8.png",
        false,
        RGB,
        BitDepth::Sixteen,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_8_should_be_rgb_8() {
    test_it_converts(
        "tests/files/rgb_8_should_be_rgb_8.png",
        false,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_16_should_be_palette_8() {
    test_it_converts(
        "tests/files/rgb_16_should_be_palette_8.png",
        false,
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/rgb_8_should_be_palette_8.png",
        false,
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_trns_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/rgb_trns_8_should_be_palette_8.png",
        false,
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_16_should_be_palette_4() {
    test_it_converts(
        "tests/files/rgb_16_should_be_palette_4.png",
        false,
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn rgb_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/rgb_8_should_be_palette_4.png",
        false,
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn rgb_16_should_be_palette_2() {
    test_it_converts(
        "tests/files/rgb_16_should_be_palette_2.png",
        false,
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn rgb_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/rgb_8_should_be_palette_2.png",
        false,
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn rgb_16_should_be_palette_1() {
    test_it_converts(
        "tests/files/rgb_16_should_be_palette_1.png",
        false,
        RGB,
        BitDepth::Sixteen,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn rgb_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/rgb_8_should_be_palette_1.png",
        false,
        RGB,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn rgb_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/rgb_16_should_be_grayscale_16.png",
        false,
        RGB,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn rgb_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/rgb_16_should_be_grayscale_8.png",
        false,
        RGB,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn rgb_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/rgb_8_should_be_grayscale_8.png",
        false,
        RGB,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn palette_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_8.png",
        false,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn palette_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_4.png",
        false,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn palette_4_should_be_palette_4() {
    test_it_converts(
        "tests/files/palette_4_should_be_palette_4.png",
        false,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn palette_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_2.png",
        false,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn palette_4_should_be_palette_2() {
    test_it_converts(
        "tests/files/palette_4_should_be_palette_2.png",
        false,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn palette_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/palette_8_should_be_grayscale_8.png",
        false,
        INDEXED,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn palette_2_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/palette_2_should_be_grayscale_alpha_8.png",
        true,
        INDEXED,
        BitDepth::Two,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn palette_8_should_be_rgb() {
    test_it_converts(
        "tests/files/palette_8_should_be_rgb.png",
        false,
        INDEXED,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn palette_8_should_be_rgba() {
    test_it_converts(
        "tests/files/palette_8_should_be_rgba.png",
        false,
        INDEXED,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn palette_2_should_be_palette_2() {
    test_it_converts(
        "tests/files/palette_2_should_be_palette_2.png",
        false,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn palette_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/palette_8_should_be_palette_1.png",
        false,
        INDEXED,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn palette_4_should_be_palette_1() {
    test_it_converts(
        "tests/files/palette_4_should_be_palette_1.png",
        false,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn palette_2_should_be_palette_1() {
    test_it_converts(
        "tests/files/palette_2_should_be_palette_1.png",
        false,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn palette_1_should_be_palette_1() {
    test_it_converts(
        "tests/files/palette_1_should_be_palette_1.png",
        false,
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn grayscale_alpha_16_should_be_grayscale_alpha_16() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_should_be_grayscale_alpha_16.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
    );
}

#[test]
fn grayscale_alpha_16_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_should_be_grayscale_alpha_8.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_8_should_be_grayscale_alpha_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_should_be_grayscale_alpha_8.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_should_be_grayscale_16.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn grayscale_alpha_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_should_be_grayscale_8.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_should_be_grayscale_8.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_should_be_palette_8.png",
        false,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_16_should_be_grayscale_16() {
    test_it_converts(
        "tests/files/grayscale_16_should_be_grayscale_16.png",
        false,
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn grayscale_16_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/grayscale_16_should_be_grayscale_8.png",
        false,
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_16_should_be_grayscale_1() {
    test_it_converts(
        "tests/files/grayscale_16_should_be_grayscale_1.png",
        false,
        GRAYSCALE,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn grayscale_8_should_be_grayscale_8() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_grayscale_8.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_8_should_be_grayscale_4() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_grayscale_4.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Four,
    );
}

#[test]
fn grayscale_8_should_be_grayscale_2() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_grayscale_2.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Two,
    );
}

#[test]
fn grayscale_4_should_be_grayscale_2() {
    test_it_converts(
        "tests/files/grayscale_4_should_be_grayscale_2.png",
        false,
        GRAYSCALE,
        BitDepth::Four,
        GRAYSCALE,
        BitDepth::Two,
    );
}

#[test]
fn grayscale_8_should_be_grayscale_1() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_grayscale_1.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn grayscale_4_should_be_grayscale_1() {
    test_it_converts(
        "tests/files/grayscale_4_should_be_grayscale_1.png",
        false,
        GRAYSCALE,
        BitDepth::Four,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn grayscale_2_should_be_grayscale_1() {
    test_it_converts(
        "tests/files/grayscale_2_should_be_grayscale_1.png",
        false,
        GRAYSCALE,
        BitDepth::Two,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn grayscale_8_should_be_palette_8() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_palette_8.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_8_should_be_palette_4() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_palette_4.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn grayscale_8_should_be_palette_2() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_palette_2.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Two,
    );
}

#[test]
fn grayscale_8_should_be_palette_1() {
    test_it_converts(
        "tests/files/grayscale_8_should_be_palette_1.png",
        false,
        GRAYSCALE,
        BitDepth::Eight,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn grayscale_alpha_16_should_be_grayscale_trns_16() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_should_be_grayscale_trns_16.png",
        true,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE,
        BitDepth::Sixteen,
    );
}

#[test]
fn grayscale_alpha_8_should_be_grayscale_trns_8() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_should_be_grayscale_trns_8.png",
        true,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_8_should_be_grayscale_trns_1() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_should_be_grayscale_trns_1.png",
        true,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn grayscale_trns_8_should_be_grayscale_1() {
    test_it_converts(
        "tests/files/grayscale_trns_8_should_be_grayscale_1.png",
        true,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn small_files() {
    test_it_converts(
        "tests/files/small_files.png",
        false,
        INDEXED,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn palette_should_be_reduced_with_dupes() {
    let input = PathBuf::from("tests/files/palette_should_be_reduced_with_dupes.png");
    let (output, opts) = get_opts(&input);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 43);
    }

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

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 35);
    }

    remove_file(output).ok();
}

#[test]
fn palette_should_be_reduced_with_unused() {
    let input = PathBuf::from("tests/files/palette_should_be_reduced_with_unused.png");
    let (output, opts) = get_opts(&input);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 35);
    }

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

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 33);
    }

    remove_file(output).ok();
}

#[test]
fn palette_should_be_reduced_with_bkgd() {
    let input = PathBuf::from("tests/files/palette_should_be_reduced_with_bkgd.png");
    let (output, opts) = get_opts(&input);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::One);
    assert_eq!(&png.aux_chunks[0].name, b"bKGD");
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 3);
    }

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

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::One);
    assert_ne!(&png.aux_chunks[0].name, b"bKGD");
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 2);
    }

    remove_file(output).ok();
}

#[test]
fn palette_should_be_reduced_with_both() {
    let input = PathBuf::from("tests/files/palette_should_be_reduced_with_both.png");
    let (output, opts) = get_opts(&input);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 43);
    }

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

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 33);
    }

    remove_file(output).ok();
}

#[test]
fn palette_should_be_reduced_with_missing() {
    let input = PathBuf::from("tests/files/palette_should_be_reduced_with_missing.png");
    let (output, opts) = get_opts(&input);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 2);
    }

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

    assert_eq!(png.raw.ihdr.color_type.png_header_code(), INDEXED);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Two);
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert_eq!(palette.len(), 3);
    }

    remove_file(output).ok();
}

#[test]
fn rgba_16_reduce_alpha() {
    test_it_converts(
        "tests/files/rgba_16_reduce_alpha.png",
        true,
        RGBA,
        BitDepth::Sixteen,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn rgba_8_reduce_alpha() {
    test_it_converts(
        "tests/files/rgba_8_reduce_alpha.png",
        true,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_16_reduce_alpha() {
    test_it_converts(
        "tests/files/grayscale_alpha_16_reduce_alpha.png",
        true,
        GRAYSCALE_ALPHA,
        BitDepth::Sixteen,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn grayscale_alpha_8_reduce_alpha() {
    test_it_converts(
        "tests/files/grayscale_alpha_8_reduce_alpha.png",
        true,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}
