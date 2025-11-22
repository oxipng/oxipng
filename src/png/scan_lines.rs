use crate::png::PngImage;

/// An iterator over the scan lines of a PNG image
#[derive(Debug, Clone)]
pub struct ScanLines<'a> {
    iter: ScanLineRanges,
    /// A reference to the PNG image being iterated upon
    raw_data: &'a [u8],
    /// Whether the raw data contains filter bytes
    has_filter: bool,
}

impl<'a> ScanLines<'a> {
    #[must_use]
    pub fn new(png: &'a PngImage, has_filter: bool) -> Self {
        Self {
            iter: ScanLineRanges::new(png, has_filter),
            raw_data: &png.data,
            has_filter,
        }
    }
}

impl<'a> Iterator for ScanLines<'a> {
    type Item = ScanLine<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (len, pass, num_pixels) = self.iter.next()?;
        debug_assert!(self.raw_data.len() >= len);
        debug_assert!(!self.has_filter || len > 1);
        // The data length should always be correct here but this check assures
        // the compiler that it doesn't need to account for a potential panic
        if self.raw_data.len() < len {
            return None;
        }
        let (data, rest) = self.raw_data.split_at(len);
        self.raw_data = rest;
        let (&filter, data) = if self.has_filter {
            data.split_first().unwrap()
        } else {
            (&0, data)
        };
        Some(ScanLine {
            filter,
            data,
            pass,
            num_pixels,
        })
    }
}

#[derive(Debug, Clone)]
/// An iterator over the scan line locations of a PNG image
struct ScanLineRanges {
    /// Current pass number, and 0-indexed row within the pass
    pass: Option<(u8, u32)>,
    bits_per_pixel: usize,
    width: u32,
    height: u32,
    left: usize,
    has_filter: bool,
}

impl ScanLineRanges {
    pub fn new(png: &PngImage, has_filter: bool) -> Self {
        Self {
            bits_per_pixel: png.ihdr.bpp(),
            width: png.ihdr.width,
            height: png.ihdr.height,
            left: png.data.len(),
            pass: if png.ihdr.interlaced {
                Some((1, 0))
            } else {
                None
            },
            has_filter,
        }
    }
}

impl Iterator for ScanLineRanges {
    type Item = (usize, Option<u8>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        let (pixels_per_line, current_pass) = if let Some(ref mut pass) = self.pass {
            // Scanlines for interlaced PNG files
            // Handle edge cases for images smaller than 5 pixels in either direction
            // No extra case needed for skipping pass 7 as this is already handled by the
            // self.left == 0 check above
            if self.width < 5 && pass.0 == 2 {
                pass.0 = 3;
                pass.1 = 4;
            }
            // Intentionally keep these separate so that they can be applied one after another
            if self.height < 5 && pass.0 == 3 {
                pass.0 = 4;
                pass.1 = 0;
            }
            if self.width < 3 && pass.0 == 4 {
                pass.0 = 5;
                pass.1 = 2;
            }
            if self.height < 3 && pass.0 == 5 {
                pass.0 = 6;
                pass.1 = 0;
            }
            if self.width == 1 && pass.0 == 6 {
                pass.0 = 7;
                pass.1 = 1;
            }
            let (pixels_factor, y_steps) = match pass.0 {
                1 | 2 => (8, 8),
                3 => (4, 8),
                4 => (4, 4),
                5 => (2, 4),
                6 => (2, 2),
                7 => (1, 2),
                _ => unreachable!(),
            };
            let mut pixels_per_line = self.width / pixels_factor;
            // Determine whether to add pixels if there is a final, incomplete 8x8 block
            let gap = self.width % pixels_factor;
            match pass.0 {
                1 | 3 | 5 if gap > 0 => {
                    pixels_per_line += 1;
                }
                2 if gap >= 5 => {
                    pixels_per_line += 1;
                }
                4 if gap >= 3 => {
                    pixels_per_line += 1;
                }
                6 if gap >= 2 => {
                    pixels_per_line += 1;
                }
                _ => (),
            }
            let current_pass = Some(pass.0);
            if pass.1 + y_steps >= self.height {
                pass.0 += 1;
                pass.1 = match pass.0 {
                    3 => 4,
                    5 => 2,
                    7 => 1,
                    _ => 0,
                };
            } else {
                pass.1 += y_steps;
            }
            (pixels_per_line, current_pass)
        } else {
            // Standard, non-interlaced PNG scanlines
            (self.width, None)
        };
        let bits_per_line = pixels_per_line as usize * self.bits_per_pixel;
        let mut len = bits_per_line.div_ceil(8);
        if self.has_filter {
            len += 1;
        }
        self.left = self.left.checked_sub(len)?;
        Some((len, current_pass, pixels_per_line as usize))
    }
}

#[derive(Debug, Clone)]
/// A scan line in a PNG image
pub struct ScanLine<'a> {
    /// The filter type used to encode the current scan line (0-4)
    pub filter: u8,
    /// The byte data for the current scan line, encoded with the filter specified in the `filter` field
    pub data: &'a [u8],
    /// The current pass if the image is interlaced
    pub pass: Option<u8>,
    /// The number of pixels in the current scan line
    pub num_pixels: usize,
}
