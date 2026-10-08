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

const ALL_STRATEGIES: [FilterStrategy; 10] = [
    FilterStrategy::NONE,
    FilterStrategy::SUB,
    FilterStrategy::UP,
    FilterStrategy::AVERAGE,
    FilterStrategy::PAETH,
    FilterStrategy::MinSum,
    FilterStrategy::Entropy,
    FilterStrategy::Bigrams,
    FilterStrategy::BigEnt,
    FilterStrategy::Brute {
        num_lines: 4,
        level: 1,
    },
];

/// Choose the filters by section, check the result's shape, and that the filtered image decodes
/// to the original (where alpha optimization may only change fully transparent pixels)
fn test_sections(
    image: &PngImage,
    strategies: &[FilterStrategy],
    optimize_alpha: bool,
) -> Vec<usize> {
    let strategies = IndexSet::from_iter(strategies.iter().cloned());
    let sections = image
        .filter_by_sections(&strategies, optimize_alpha, 7)
        .expect("several sections");
    assert_eq!(sections.filters.len(), image.scan_lines(false).count());
    assert!(sections.sections > 1);
    assert_eq!(
        sections.won.iter().sum::<usize>(),
        image.ihdr.raw_data_size()
    );

    let predefined = FilterStrategy::Predefined(sections.filters);
    let (filtered, _) = image.filter_image(predefined, optimize_alpha);
    let compressed = deflate(&filtered, 1, None).unwrap();
    let decoded = PngImage::new(image.ihdr.clone(), &compressed).unwrap();
    let bytes = image.bytes_per_channel();
    let bpp = bytes * image.channels_per_pixel();
    for (a, b) in image.data.chunks(bpp).zip(decoded.data.chunks(bpp)) {
        if !optimize_alpha || a[bpp - bytes..].iter().any(|&x| x != 0) {
            assert_eq!(a, b);
        }
    }
    sections.won
}

fn load(input: &str) -> PngImage {
    let png = PngData::new(Path::new(input), &oxipng::Options::default()).unwrap();
    (*png.raw).clone()
}

#[test]
fn filter_by_sections() {
    let image = load("tests/files/rgba_16_should_be_grayscale_alpha_16.png");
    test_sections(&image, &ALL_STRATEGIES, false);
    let image = load("tests/files/interlaced_rgba_16_should_be_grayscale_alpha_16.png");
    test_sections(&image, &ALL_STRATEGIES, false);
    let image = load("tests/files/filter_0_for_rgba_8.png");
    test_sections(&image, &ALL_STRATEGIES, true);
}

#[test]
fn filter_by_sections_predefined() {
    // Predefined filters that are too short (the rest is None) or too long, and a duplicate
    let image = load("tests/files/filter_0_for_rgba_8.png");
    let lines = image.scan_lines(false).count();
    let mut strategies = ALL_STRATEGIES.to_vec();
    strategies.extend([
        FilterStrategy::Predefined(vec![RowFilter::Paeth; 3]),
        FilterStrategy::Predefined(vec![RowFilter::Up; lines + 5]),
        FilterStrategy::Predefined(vec![RowFilter::Sub; lines]),
    ]);
    let won = test_sections(&image, &strategies, false);
    // The duplicate of Sub never wins: on equal size the earlier strategy does
    assert_eq!(won.last(), Some(&0));
}

#[test]
fn filter_by_sections_single_section() {
    // Two lines of 200 KiB make a single section: nothing to choose by section
    let mut image = load("tests/files/rgb_8_should_be_rgb_8.png");
    image.ihdr.width = 200 * 1024 / 3;
    image.ihdr.height = 2;
    image.data = (0..image.ihdr.width * 2 * 3)
        .map(|i| (i % 251) as u8)
        .collect();
    assert!(image.ihdr.raw_data_size() > 3 * 128 * 1024);
    let strategies = IndexSet::from_iter(ALL_STRATEGIES);
    assert!(image.filter_by_sections(&strategies, false, 7).is_none());
}
