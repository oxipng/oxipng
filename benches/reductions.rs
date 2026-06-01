#![feature(test)]

extern crate oxipng;
extern crate test;

use std::path::PathBuf;

use oxipng::{internal_tests::*, *};
use test::Bencher;

#[bench]
fn reductions_16_to_8_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_16_should_be_rgb_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_16_to_8(&png.raw, false));
}

#[bench]
fn reductions_16_to_8_bits_scaled(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_16_should_be_rgb_16.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_16_to_8(&png.raw, true));
}

#[bench]
fn reductions_8_to_4_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_4.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_8_to_2_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_2.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_8_to_1_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_1.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_grayscale_8_to_4_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_8_should_be_grayscale_4.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_grayscale_8_to_2_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_8_should_be_grayscale_2.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_grayscale_8_to_1_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_8_should_be_grayscale_1.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| bit_depth::reduced_bit_depth_8_or_less(&png.raw));
}

#[bench]
fn reductions_rgba_to_rgb_16(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgba_16_should_be_rgb_16.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_rgba_to_rgb_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgba_8_should_be_rgb_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_rgba_to_rgb_trns_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgba_8_should_be_rgb_trns_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_rgba_to_grayscale_alpha_16(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/rgba_16_should_be_grayscale_alpha_16.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_rgb_to_grayscale(&png.raw));
}

#[bench]
fn reductions_rgba_to_grayscale_alpha_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/rgba_8_should_be_grayscale_alpha_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_rgb_to_grayscale(&png.raw));
}

#[bench]
fn reductions_rgb_to_grayscale_16(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/rgb_16_should_be_grayscale_16.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_rgb_to_grayscale(&png.raw));
}

#[bench]
fn reductions_rgb_to_grayscale_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_8_should_be_grayscale_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_rgb_to_grayscale(&png.raw));
}

#[bench]
fn reductions_rgba_to_palette_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgba_8_should_be_palette_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_to_indexed(&png.raw, true));
}

#[bench]
fn reductions_rgb_to_palette_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_8_should_be_palette_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_to_indexed(&png.raw, true));
}

#[bench]
fn reductions_grayscale_alpha_to_grayscale_16(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_alpha_16_should_be_grayscale_16.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_grayscale_alpha_to_grayscale_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_alpha_8_should_be_grayscale_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_grayscale_alpha_to_grayscale_trns_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_alpha_8_should_be_grayscale_trns_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::reduced_alpha_channel(&png.raw, true));
}

#[bench]
fn reductions_grayscale_8_to_palette_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/grayscale_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::reduced_to_indexed(&png.raw, true));
}

#[bench]
fn reductions_palette_8_to_grayscale_8(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_grayscale_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| color::indexed_to_channels(&png.raw, true, false));
}

#[bench]
fn reductions_palette_duplicate_reduction(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_should_be_reduced_with_dupes.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::reduced_palette(&png.raw, false));
}

#[bench]
fn reductions_palette_unused_reduction(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_should_be_reduced_with_unused.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::reduced_palette(&png.raw, false));
}

#[bench]
fn reductions_palette_full_reduction(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_should_be_reduced_with_both.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::reduced_palette(&png.raw, false));
}

#[bench]
fn reductions_palette_sort(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::sorted_palette(&png.raw));
}

#[bench]
fn reductions_palette_sort_mzeng(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::sorted_palette_mzeng(&png.raw));
}

#[bench]
fn reductions_palette_sort_ezeng(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::sorted_palette_ezeng(&png.raw, 0));
}

#[bench]
fn reductions_palette_sort_ezeng_with_swaps(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::sorted_palette_ezeng(&png.raw, 50));
}

#[bench]
fn reductions_palette_sort_battiato(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_8_should_be_palette_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| palette::sorted_palette_battiato(&png.raw));
}

#[bench]
fn reductions_alpha(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgba_8_reduce_alpha.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| alpha::cleaned_alpha_channel(&png.raw));
}
