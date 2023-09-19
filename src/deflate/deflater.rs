use crate::atomicmin::AtomicMin;
use crate::{PngError, PngResult};
use libdeflater::*;

pub fn deflate(data: &[u8], level: u8, max_size: &AtomicMin) -> PngResult<Vec<u8>> {
    let mut compressor = Compressor::new(CompressionLvl::new(level.into()).unwrap());
    let capacity = max_size
        .get()
        .unwrap_or_else(|| compressor.zlib_compress_bound(data.len()));
    let mut dest = vec![0; capacity];
    let len = compressor
        .zlib_compress(data, &mut dest)
        .map_err(|err| match err {
            CompressionError::InsufficientSpace => PngError::DeflatedDataTooLong(capacity),
        })?;
    dest.truncate(len);
    Ok(dest)
}

pub fn inflate(data: &[u8], out_size: usize) -> PngResult<Vec<u8>> {
    let mut decompressor = Decompressor::new();
    let mut dest = vec![0; out_size];
    let len = decompressor
        .zlib_decompress(data, &mut dest)
        .map_err(|err| match err {
            DecompressionError::BadData => PngError::InvalidData,
            DecompressionError::InsufficientSpace => PngError::new("inflated data too long"),
        })?;
    dest.truncate(len);
    Ok(dest)
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = Crc::new();
    crc.update(data);
    crc.sum()
}
