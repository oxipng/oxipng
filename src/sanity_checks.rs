use std::io::Cursor;

use image::{codecs::png::PngDecoder, *};
use log::{error, warn};

#[cfg(not(feature = "parallel"))]
use crate::rayon;

/// Validate that the output png data still matches the original image
pub fn validate_output(output: &[u8], original_data: &[u8]) -> bool {
    let (old_frames, new_frames) = rayon::join(
        || load_png_image_from_memory(original_data),
        || load_png_image_from_memory(output),
    );

    match (new_frames, old_frames) {
        (Err(new_err), _) => {
            error!("Failed to read output image for validation: {}", new_err);
            false
        }
        (_, Err(old_err)) => {
            // The original image might be invalid if, for example, there is a CRC error,
            // and we set fix_errors to true. In that case, all we can do is check that the
            // new image is decodable.
            warn!("Failed to read input image for validation: {}", old_err);
            true
        }
        (Ok(new_frames), Ok(old_frames)) if new_frames.len() != old_frames.len() => false,
        (Ok(new_frames), Ok(old_frames)) => {
            for (a, b) in old_frames.iter().zip(new_frames) {
                if !images_equal(a, &b) {
                    return false;
                }
            }
            true
        }
    }
}

/// Loads a PNG image from memory to frames of [RgbaImage]
fn load_png_image_from_memory(png_data: &[u8]) -> Result<Vec<RgbaImage>, image::ImageError> {
    let decoder = PngDecoder::new(Cursor::new(png_data))?;
    if decoder.is_apng()? {
        decoder
            .apng()?
            .into_frames()
            .map(|f| f.map(image::Frame::into_buffer))
            .collect()
    } else {
        DynamicImage::from_decoder(decoder).map(|i| vec![i.into_rgba8()])
    }
}

/// Compares images pixel by pixel for equivalent content
fn images_equal(old_png: &RgbaImage, new_png: &RgbaImage) -> bool {
    let a = old_png.pixels().filter(|x| {
        let p = x.channels();
        !(p.len() == 4 && p[3] == 0)
    });
    let b = new_png.pixels().filter(|x| {
        let p = x.channels();
        !(p.len() == 4 && p[3] == 0)
    });
    a.eq(b)
}
