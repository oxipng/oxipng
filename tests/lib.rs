use std::{
    fs,
    fs::File,
    io::prelude::*,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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
    assert!(matches!(result, Err(PngError::C2PAMetadataPreventsChanges)));
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

fn temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oxipng-{name}-{}-{nanos}.png", std::process::id()))
}

#[test]
fn min_gain_bytes_skips_in_place_write() {
    let input_data = fs::read("tests/files/verbose_mode.png").unwrap();
    let opts = Options::default();
    let optimized = oxipng::optimize_from_memory(&input_data, &opts).unwrap();
    assert!(
        optimized.len() < input_data.len(),
        "fixture should produce measurable savings"
    );
    let savings = input_data.len() - optimized.len();

    let input_path = temp_path("min-gain-bytes-input");
    fs::write(&input_path, &input_data).unwrap();
    let mut permissions = fs::metadata(&input_path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&input_path, permissions).unwrap();

    let result = oxipng::optimize_with(
        &InFile::Path(input_path.clone()),
        &OutFile::Path {
            path: None,
            preserve_attrs: false,
        },
        &opts,
        Some(MinGain::Bytes(savings + 1)),
    )
    .unwrap();
    assert_eq!(result, (input_data.len(), input_data.len()));
    assert_eq!(fs::read(&input_path).unwrap(), input_data);

    fs::remove_file(&input_path).ok();
}

#[test]
fn min_gain_percentage_threshold_behavior() {
    let input_data = fs::read("tests/files/verbose_mode.png").unwrap();
    let opts = Options::default();
    let optimized = oxipng::optimize_from_memory(&input_data, &opts).unwrap();
    assert!(
        optimized.len() < input_data.len(),
        "fixture should produce measurable savings"
    );
    let savings_ratio = (input_data.len() - optimized.len()) as f64 / input_data.len() as f64;
    let high_threshold = MinGain::ratio((savings_ratio + 0.001).min(1.0)).unwrap();
    let low_threshold = MinGain::ratio(savings_ratio / 2.0).unwrap();

    let input_path = temp_path("min-gain-percent-input");
    let high_output = temp_path("min-gain-percent-high");
    let low_output = temp_path("min-gain-percent-low");
    fs::write(&input_path, &input_data).unwrap();

    let high_result = oxipng::optimize_with(
        &InFile::Path(input_path.clone()),
        &OutFile::from_path(high_output.clone()),
        &opts,
        Some(high_threshold),
    )
    .unwrap();
    assert_eq!(high_result, (input_data.len(), input_data.len()));
    assert_eq!(fs::read(&high_output).unwrap(), input_data);

    let low_result = oxipng::optimize_with(
        &InFile::Path(input_path.clone()),
        &OutFile::from_path(low_output.clone()),
        &opts,
        Some(low_threshold),
    )
    .unwrap();
    assert_eq!(low_result.0, input_data.len());
    assert!(low_result.1 < input_data.len());
    assert_eq!(fs::read(&low_output).unwrap().len(), low_result.1);

    fs::remove_file(&input_path).ok();
    fs::remove_file(&high_output).ok();
    fs::remove_file(&low_output).ok();
}
