use indexmap::IndexSet;
use oxipng::internal_tests::*;
use oxipng::*;
use std::fs::remove_file;
use std::path::Path;
use std::path::PathBuf;

const GRAYSCALE: u8 = 0;
const RGB: u8 = 2;
const INDEXED: u8 = 3;
const GRAYSCALE_ALPHA: u8 = 4;
const RGBA: u8 = 6;

fn get_opts(input: &Path) -> (OutFile, oxipng::Options) {
    let mut options = oxipng::Options {
        force: true,
        ..Default::default()
    };
    let mut filter = IndexSet::new();
    filter.insert(RowFilter::None);
    options.filter = filter;

    (OutFile::from_path(input.with_extension("out.png")), options)
}

fn test_it_converts(
    input: &str,
    custom: Option<(OutFile, oxipng::Options)>,
    color_type_in: u8,
    bit_depth_in: BitDepth,
    color_type_out: u8,
    bit_depth_out: BitDepth,
) {
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
}

#[test]
fn issue_29() {
    test_it_converts(
        "tests/files/issue-29.png",
        None,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn issue_42() {
    let input = PathBuf::from("tests/files/issue_42.png");
    let (output, mut opts) = get_opts(&input);
    opts.interlace = Some(Interlacing::Adam7);

    let png = PngData::new(&input, &opts).unwrap();

    assert_eq!(png.raw.ihdr.interlaced, Interlacing::None);
    assert_eq!(png.raw.ihdr.color_type, ColorType::GrayscaleAlpha);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);

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

    assert_eq!(png.raw.ihdr.interlaced, Interlacing::Adam7);
    assert_eq!(png.raw.ihdr.color_type, ColorType::GrayscaleAlpha);
    assert_eq!(png.raw.ihdr.bit_depth, BitDepth::Eight);

    remove_file(output).ok();
}

#[test]
fn issue_52_01() {
    test_it_converts(
        "tests/files/issue-52-01.png",
        None,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn issue_52_02() {
    test_it_converts(
        "tests/files/issue-52-02.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn issue_52_03() {
    test_it_converts(
        "tests/files/issue-52-03.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGBA,
        BitDepth::Eight,
    );
}

#[test]
fn issue_52_04() {
    test_it_converts(
        "tests/files/issue-52-04.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn issue_52_05() {
    test_it_converts(
        "tests/files/issue-52-05.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn issue_52_06() {
    test_it_converts(
        "tests/files/issue-52-06.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGBA,
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
fn issue_80() {
    test_it_converts(
        "tests/files/issue-80.png",
        None,
        INDEXED,
        BitDepth::Two,
        INDEXED,
        BitDepth::One,
    );
}

#[test]
fn issue_82() {
    test_it_converts(
        "tests/files/issue-82.png",
        None,
        INDEXED,
        BitDepth::Four,
        INDEXED,
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
fn issue_92_filter_0() {
    test_it_converts(
        "tests/files/issue-92.png",
        None,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn issue_92_filter_5() {
    let input = "tests/files/issue-92.png";
    let (_, mut opts) = get_opts(Path::new(input));
    opts.filter = [RowFilter::MinSum].iter().cloned().collect();
    let output = OutFile::from_path(Path::new(input).with_extension("-f5-out.png"));

    test_it_converts(
        input,
        Some((output, opts)),
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::Eight,
    );
}

#[test]
fn issue_113() {
    let input = "tests/files/issue-113.png";
    let (output, mut opts) = get_opts(Path::new(input));
    opts.interlace = Some(Interlacing::Adam7);
    opts.optimize_alpha = true;
    test_it_converts(
        input,
        Some((output, opts)),
        RGBA,
        BitDepth::Eight,
        GRAYSCALE_ALPHA,
        BitDepth::Eight,
    );
}

#[test]
fn issue_129() {
    let input = "tests/files/issue-129.png";
    test_it_converts(input, None, RGB, BitDepth::Eight, INDEXED, BitDepth::Eight);
}

#[test]
fn issue_133() {
    let input = "tests/files/issue-133.png";
    let (output, mut opts) = get_opts(Path::new(input));
    opts.optimize_alpha = true;
    test_it_converts(
        input,
        Some((output, opts)),
        RGBA,
        BitDepth::Eight,
        RGBA,
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
fn issue_141() {
    test_it_converts(
        "tests/files/issue-141.png",
        None,
        RGBA,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

#[test]
fn issue_153() {
    test_it_converts(
        "tests/files/issue-153.png",
        None,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn issue_159() {
    test_it_converts(
        "tests/files/issue-159.png",
        None,
        INDEXED,
        BitDepth::One,
        INDEXED,
        BitDepth::One,
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
    opts.interlace = Some(Interlacing::Adam7);

    test_it_converts(
        input,
        Some((output, opts)),
        GRAYSCALE,
        BitDepth::One,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn issue_195() {
    test_it_converts(
        "tests/files/issue-195.png",
        None,
        RGBA,
        BitDepth::Eight,
        INDEXED,
        BitDepth::Eight,
    );
}

#[test]
fn issue_426_01() {
    test_it_converts(
        "tests/files/issue-426-01.png",
        None,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::One,
    );
}

#[test]
fn issue_426_02() {
    test_it_converts(
        "tests/files/issue-426-02.png",
        None,
        GRAYSCALE,
        BitDepth::Eight,
        GRAYSCALE,
        BitDepth::One,
    );
}
