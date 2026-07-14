use std::{fs, fs::File, io::prelude::*};

use oxipng::*;

#[test]
fn optimize_from_memory() {
    let mut in_file = File::open("tests/files/fully_optimized.png").unwrap();
    let mut in_file_buf: Vec<u8> = Vec::new();
    in_file.read_to_end(&mut in_file_buf).unwrap();

    let result = oxipng::optimize_from_memory(&in_file_buf, &Options::default());
    assert!(result.is_ok());
}

#[test]
fn optimize_from_memory_corrupted() {
    let mut in_file = File::open("tests/files/corrupted_header.png").unwrap();
    let mut in_file_buf: Vec<u8> = Vec::new();
    in_file.read_to_end(&mut in_file_buf).unwrap();

    let result = oxipng::optimize_from_memory(&in_file_buf, &Options::default());
    assert!(result.is_err());
}

#[test]
fn optimize_from_memory_apng() {
    let mut in_file = File::open("tests/files/apng_file.png").unwrap();
    let mut in_file_buf: Vec<u8> = Vec::new();
    in_file.read_to_end(&mut in_file_buf).unwrap();

    let result = oxipng::optimize_from_memory(&in_file_buf, &Options::default());
    assert!(result.is_ok());
}

#[test]
fn optimize() {
    let result = oxipng::optimize(
        &"tests/files/fully_optimized.png".into(),
        &OutFile::None,
        &Options::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn skip_c2pa() {
    let result = oxipng::optimize(
        &"tests/files/c2pa-signed.png".into(),
        &OutFile::None,
        &Options {
            strip: StripChunks::Keep(indexset! {*b"caBX"}),
            ..Options::default()
        },
    );
    assert!(matches!(result, Err(PngError::ChunkPreventsChanges(_))));
}

#[test]
fn optimize_corrupted() {
    let result = oxipng::optimize(
        &"tests/files/corrupted_header.png".into(),
        &OutFile::None,
        &Options::default(),
    );
    assert!(result.is_err());
}

#[test]
fn optimize_apng() {
    let result = oxipng::optimize(
        &"tests/files/apng_file.png".into(),
        &OutFile::None,
        &Options::from_preset(0),
    );
    assert!(result.is_ok());
}

#[test]
fn optimize_srgb_icc() {
    let file = fs::read("tests/files/badsrgb.png").unwrap();
    let mut opts = Options::default();

    let result = oxipng::optimize_from_memory(&file, &opts);
    assert!(result.unwrap().len() > 1000);

    opts.strip = StripChunks::Safe;
    let result = oxipng::optimize_from_memory(&file, &opts);
    assert!(result.unwrap().len() < 1000);
}
