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

#[test]
fn filter_segmented() {
    test_it_converts(
        "tests/files/rgb_8_should_be_rgb_8.png",
        FilterStrategy::Segmented,
        RGB,
        BitDepth::Eight,
        RGB,
        BitDepth::Eight,
    );
}

/// Filter with Segmented, then check that the data decodes to the original image and that
/// the returned filters reproduce the same output.
fn check_segmented(image: &PngImage, optimize_alpha: bool) {
    let (filtered, used) = image.filter_image(FilterStrategy::Segmented, optimize_alpha);
    let FilterStrategy::Predefined(filters) = &used else {
        panic!("expected predefined filters, got {used}");
    };
    assert_eq!(filters.len(), image.scan_lines(false).count());
    assert_eq!(filtered.len(), image.ihdr.raw_data_size());

    let (refiltered, _) = image.filter_image(used.clone(), optimize_alpha);
    assert!(refiltered == filtered);

    let compressed = deflate(&filtered, 1, None).unwrap();
    let decoded = PngImage::new(image.ihdr.clone(), &compressed).unwrap();
    if optimize_alpha {
        // Only the colour of fully transparent pixels may change
        assert!(decoded.data != image.data);
        let bytes = image.bytes_per_channel();
        let bpp = bytes * image.channels_per_pixel();
        for (a, b) in image.data.chunks(bpp).zip(decoded.data.chunks(bpp)) {
            if a[bpp - bytes..].iter().any(|&x| x != 0) {
                assert_eq!(a, b);
            }
        }
    } else {
        assert!(decoded.data == image.data);
    }
}

fn test_segmented_round_trip(input: &str, optimize_alpha: bool) {
    let png = PngData::new(Path::new(input), &oxipng::Options::default()).unwrap();
    // Large enough for several sections
    assert!(png.raw.ihdr.raw_data_size() > 4 * 65536);
    check_segmented(&png.raw, optimize_alpha);
}

#[test]
fn filter_segmented_round_trip() {
    test_segmented_round_trip(
        "tests/files/rgba_16_should_be_grayscale_alpha_16.png",
        false,
    );
}

#[test]
fn filter_segmented_round_trip_interlaced() {
    test_segmented_round_trip(
        "tests/files/interlaced_rgba_16_should_be_grayscale_alpha_16.png",
        false,
    );
}

#[test]
fn filter_segmented_round_trip_alpha() {
    test_segmented_round_trip("tests/files/filter_0_for_rgba_8.png", true);
}

#[test]
fn filter_segmented_small_interlaced() {
    // Small sizes leave some interlacing passes empty or with a single line
    let rgb = PngData::new(
        Path::new("tests/files/rgb_8_should_be_rgb_8.png"),
        &oxipng::Options::default(),
    )
    .unwrap();
    assert!(!rgb.raw.ihdr.interlaced);
    for width in 1..=9_u32 {
        for height in 1..=9 {
            let mut seed: u32 = width * 31 + height;
            let data = (0..width * height * 3)
                .map(|_| {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (seed >> 16) as u8
                })
                .collect();
            let mut ihdr = rgb.raw.ihdr.clone();
            ihdr.width = width;
            ihdr.height = height;
            let image = PngImage { ihdr, data };
            check_segmented(&image, false);
            check_segmented(
                &interlace::changed_interlacing(&image, true).unwrap(),
                false,
            );
        }
    }
}
