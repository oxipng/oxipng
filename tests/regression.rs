use std::{
    fs::remove_file,
    path::{Path, PathBuf},
    sync::Arc,
};

use oxipng::{internal_tests::*, *};

const GRAYSCALE: u8 = 0;
const INDEXED: u8 = 3;
const GRAYSCALE_ALPHA: u8 = 4;
const RGBA: u8 = 6;

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
    custom: Option<(OutFile, oxipng::Options)>,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) -> PngImage {
    let input = PathBuf::from(input);
    let (output, opts) = custom.unwrap_or_else(|| get_opts(&input));
    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(
        png.raw.ihdr.color_type.png_header_code(),
        color_type_in,
        "test file is broken"
    );
    assert_eq!(png.raw.ihdr.bit_depth, bit_depth_in, "test file is broken");

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

    assert_eq!(
        png.raw.ihdr.color_type.png_header_code(),
        color_type_out,
        "optimized to wrong color type"
    );
    assert_eq!(
        png.raw.ihdr.bit_depth, bit_depth_out,
        "optimized to wrong bit depth"
    );
    if let ColorType::Indexed { palette } = &png.raw.ihdr.color_type {
        assert!(palette.len() <= 1 << (png.raw.ihdr.bit_depth as u8));
    }

    remove_file(output).ok();
    Arc::try_unwrap(png.raw).unwrap()
}

#[test]
fn issue_42() {
    let input = "tests/files/issue-42.png";
    let (output, mut opts) = get_opts(Path::new(input));
    opts.interlace = Some(true);
    test_it_converts(
        input,
        Some((output, opts)),
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn issue_56() {
    test_it_converts(
        "tests/files/issue-56.png",
        None,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn issue_58() {
    test_it_converts(
        "tests/files/issue-58.png",
        None,
        INDEXED,
        BitDepth::Four,
        INDEXED,
        BitDepth::Four,
    );
}

#[test]
fn issue_59() {
    test_it_converts(
        "tests/files/issue-59.png",
        None,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn issue_60() {
    test_it_converts(
        "tests/files/issue-60.png",
        None,
        RGBA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn issue_89() {
    test_it_converts(
        "tests/files/issue-89.png",
        None,
        RGBA,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn issue_140() {
    test_it_converts(
        "tests/files/issue-140.png",
        None,
        GRAYSCALE,
        BitDepth::Two,
        GRAYSCALE,
        BitDepth::Two,
    );
}

#[test]
fn issue_171() {
    test_it_converts(
        "tests/files/issue-171.png",
        None,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn issue_175() {
    test_it_converts(
        "tests/files/issue-175.png",
        None,
        GRAYSCALE,
        BitDepth::One,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn issue_182() {
    let input = "tests/files/issue-182.png";
    let (output, mut opts) = get_opts(Path::new(input));
    opts.interlace = Some(true);

    test_it_converts(
        input,
        Some((output, opts)),
        GRAYSCALE,
        BitDepth::One,
        GRAYSCALE,
        BitDepth::One,
    );
}
