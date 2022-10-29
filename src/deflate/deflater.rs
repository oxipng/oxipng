use crate::atomicmin::AtomicMin;
use crate::{PngError, PngResult};
use libdeflater::{CompressionError, CompressionLvl, Compressor};

pub fn deflate(data: &[u8], level: u8, max_size: &AtomicMin) -> PngResult<Vec<u8>> {
    let mut compressor = Compressor::new(CompressionLvl::new(level.into()).unwrap());
    // If adhering to a max_size we need to include at least 9 extra bytes of slack space (as specified in docs).
    let capacity = max_size
        .get()
        .unwrap_or_else(|| compressor.zlib_compress_bound(data.len()))
        + 9;
    let mut dest = vec![0; capacity];
    let len = compressor
        .zlib_compress(data, &mut dest)
        .map_err(|err| match err {
            CompressionError::InsufficientSpace => PngError::DeflatedDataTooLong(capacity),
        })?;
    if let Some(max) = max_size.get() {
        if len > max {
            return Err(PngError::DeflatedDataTooLong(max));
        }
    }
    dest.truncate(len);
    Ok(dest)
}
