#![feature(test)]

extern crate oxipng;
extern crate test;

use std::path::PathBuf;

use oxipng::{internal_tests::*, *};
use test::Bencher;

#[bench]
fn interlacing_16_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_16_should_be_rgb_16.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(true));
}

#[bench]
fn interlacing_8_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from("tests/files/rgb_8_should_be_rgb_8.png"));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(true));
}

#[bench]
fn interlacing_4_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_4_should_be_palette_4.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(true));
}

#[bench]
fn interlacing_2_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_2_should_be_palette_2.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(true));
}

#[bench]
fn interlacing_1_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/palette_1_should_be_palette_1.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(true));
}

#[bench]
fn deinterlacing_16_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/interlaced_rgb_16_should_be_rgb_16.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(false));
}

#[bench]
fn deinterlacing_8_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/interlaced_rgb_8_should_be_rgb_8.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(false));
}

#[bench]
fn deinterlacing_4_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/interlaced_palette_4_should_be_palette_4.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(false));
}

#[bench]
fn deinterlacing_2_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/interlaced_palette_2_should_be_palette_2.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(false));
}

#[bench]
fn deinterlacing_1_bits(b: &mut Bencher) {
    let input = test::black_box(PathBuf::from(
        "tests/files/interlaced_palette_1_should_be_palette_1.png",
    ));
    let png = PngData::new(&input, &Options::default()).unwrap();

    b.iter(|| png.raw.change_interlacing(false));
}
