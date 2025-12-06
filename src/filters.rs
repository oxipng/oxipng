use std::{fmt, fmt::Display, mem::transmute};

/// Filtering strategy for use in [`Options`][crate::Options]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum FilterStrategy {
    /// Same filter for all rows
    Basic(RowFilter),
    /// Minimum sum of absolute differences
    MinSum,
    /// Shannon entropy
    Entropy,
    /// Count of distinct bigrams
    Bigrams,
    /// Shannon entropy of bigrams
    BigEnt,
    /// Deflate compression
    Brute {
        /// The number of lines to compress at once
        num_lines: usize,
        /// The compression level to use (1-12)
        level: u8,
    },
    /// Predefined filter for each row
    Predefined(Vec<RowFilter>),
}

impl FilterStrategy {
    pub const NONE: Self = Self::Basic(RowFilter::None);
    pub const SUB: Self = Self::Basic(RowFilter::Sub);
    pub const UP: Self = Self::Basic(RowFilter::Up);
    pub const AVERAGE: Self = Self::Basic(RowFilter::Average);
    pub const PAETH: Self = Self::Basic(RowFilter::Paeth);
}

impl Display for FilterStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(filter) => filter.fmt(f),
            Self::MinSum => "MinSum".fmt(f),
            Self::Entropy => "Entropy".fmt(f),
            Self::Bigrams => "Bigrams".fmt(f),
            Self::BigEnt => "BigEnt".fmt(f),
            Self::Brute { .. } => "Brute".fmt(f),
            Self::Predefined(_) => "Predefined".fmt(f),
        }
    }
}

/// PNG delta filters
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum RowFilter {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl TryFrom<u8> for RowFilter {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 4 {
            return Err(());
        }
        unsafe { transmute(value as i8) }
    }
}

impl Display for RowFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(
            match self {
                Self::None => "None",
                Self::Sub => "Sub",
                Self::Up => "Up",
                Self::Average => "Average",
                Self::Paeth => "Paeth",
            },
            f,
        )
    }
}

impl RowFilter {
    pub(crate) const ALL: [Self; 5] = [Self::None, Self::Sub, Self::Up, Self::Average, Self::Paeth];
    pub(crate) const SINGLE_LINE: [Self; 2] = [Self::None, Self::Sub];

    pub(crate) fn filter_line(
        self,
        bpp: usize,
        data: &mut [u8],
        prev_line: &[u8],
        buf: &mut Vec<u8>,
        alpha_bytes: usize,
    ) {
        assert!(data.len() >= bpp);
        assert_eq!(data.len(), prev_line.len());

        if alpha_bytes != 0 {
            self.optimize_alpha(bpp, data, prev_line, bpp - alpha_bytes);
        }

        buf.clear();
        buf.reserve(data.len() + 1);
        buf.push(self as u8);
        match self {
            Self::None => {
                buf.extend_from_slice(data);
            }
            Self::Sub => {
                buf.extend_from_slice(&data[0..bpp]);
                buf.extend(
                    data.iter()
                        .skip(bpp)
                        .zip(data.iter())
                        .map(|(cur, last)| cur.wrapping_sub(*last)),
                );
            }
            Self::Up => {
                buf.extend(
                    data.iter()
                        .zip(prev_line.iter())
                        .map(|(cur, last)| cur.wrapping_sub(*last)),
                );
            }
            Self::Average => {
                for (i, byte) in data.iter().enumerate() {
                    buf.push(byte.wrapping_sub(i.checked_sub(bpp).map_or_else(
                        || prev_line[i] >> 1,
                        |x| ((u16::from(data[x]) + u16::from(prev_line[i])) >> 1) as u8,
                    )));
                }
            }
            Self::Paeth => {
                for (i, byte) in data.iter().enumerate() {
                    buf.push(byte.wrapping_sub(i.checked_sub(bpp).map_or_else(
                        || prev_line[i],
                        |x| paeth_predictor(data[x], prev_line[i], prev_line[x]),
                    )));
                }
            }
        }
    }

    // Optimize fully transparent pixels of a scanline such that they will be zeroed when filtered
    fn optimize_alpha(self, bpp: usize, data: &mut [u8], prev_line: &[u8], color_bytes: usize) {
        if self == Self::None {
            // Assume transparent pixels already set to 0
            return;
        }

        let mut pixels: Vec<_> = data.chunks_exact_mut(bpp).collect();
        let prev_pixels: Vec<_> = prev_line.chunks_exact(bpp).collect();
        for i in 0..pixels.len() {
            if pixels[i].iter().skip(color_bytes).all(|b| *b == 0) {
                // If the first pixel in the row is transparent, find the next non-transparent pixel and pretend
                // it is the previous one. This can help improve effectiveness of the Sub and Paeth filters.
                let prev = match i {
                    0 => pixels
                        .iter()
                        .position(|px| px.iter().skip(color_bytes).any(|b| *b != 0))
                        .unwrap_or(i),
                    _ => i - 1,
                };

                // These assertions help eliminate a few bounds checks in the slice accesses below
                assert!(prev < pixels.len());
                assert!(i < prev_pixels.len());

                match self {
                    Self::None => unreachable!(),
                    Self::Sub => {
                        // The code below is roughly equivalent to pixels[i][0..color_bytes].copy_from_slice(&pixels[prev][0..color_bytes]),
                        // if such a thing was possible to do without violating Rust aliasing rules. See:
                        // https://users.rust-lang.org/t/problem-borrowing-two-elements-of-vec-mutably/21446/2

                        if prev < i {
                            let (pixels_head, pixels_tail) = pixels.split_at_mut(prev + 1);
                            pixels_tail[i - prev - 1][0..color_bytes]
                                .copy_from_slice(&pixels_head[prev][0..color_bytes]);
                        } else if prev > i {
                            let (pixels_head, pixels_tail) = pixels.split_at_mut(i + 1);
                            pixels_head[i][0..color_bytes]
                                .copy_from_slice(&pixels_tail[prev - i - 1][0..color_bytes]);
                        } else {
                            // If prev == i, we'd be copying the pixels onto themselves, which is useless
                        }
                    }
                    Self::Up => {
                        pixels[i][0..color_bytes].copy_from_slice(&prev_pixels[i][0..color_bytes]);
                    }
                    Self::Average => {
                        for j in 0..color_bytes {
                            pixels[i][j] = match i {
                                0 => prev_pixels[i][j] >> 1,
                                _ => {
                                    ((u16::from(pixels[i - 1][j]) + u16::from(prev_pixels[i][j]))
                                        >> 1) as u8
                                }
                            };
                        }
                    }
                    Self::Paeth => {
                        for j in 0..color_bytes {
                            pixels[i][j] = match i {
                                0 => pixels[prev][j].min(prev_pixels[i][j]),
                                _ => paeth_predictor(
                                    pixels[i - 1][j],
                                    prev_pixels[i][j],
                                    prev_pixels[i - 1][j],
                                ),
                            };
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn unfilter_line(
        self,
        bpp: usize,
        data: &[u8],
        prev_line: &[u8],
        buf: &mut Vec<u8>,
    ) {
        buf.clear();
        buf.reserve(data.len());
        assert!(data.len() >= bpp);
        assert_eq!(data.len(), prev_line.len());
        match self {
            Self::None => {
                buf.extend_from_slice(data);
            }
            Self::Sub => {
                for (i, &cur) in data.iter().enumerate() {
                    let prev_byte = i.checked_sub(bpp).and_then(|x| buf.get(x).copied());
                    buf.push(prev_byte.map_or(cur, |b| cur.wrapping_add(b)));
                }
            }
            Self::Up => {
                buf.extend(
                    data.iter()
                        .zip(prev_line)
                        .map(|(&cur, &last)| cur.wrapping_add(last)),
                );
            }
            Self::Average => {
                for (i, (&cur, &last)) in data.iter().zip(prev_line).enumerate() {
                    let prev_byte = i.checked_sub(bpp).and_then(|x| buf.get(x).copied());
                    buf.push(cur.wrapping_add(prev_byte.map_or_else(
                        || last >> 1,
                        |b| ((u16::from(b) + u16::from(last)) >> 1) as u8,
                    )));
                }
            }
            Self::Paeth => {
                for (i, (&cur, &up)) in data.iter().zip(prev_line).enumerate() {
                    buf.push(
                        match i
                            .checked_sub(bpp)
                            .map(|x| (buf.get(x).copied(), prev_line.get(x).copied()))
                        {
                            Some((Some(left), Some(left_up))) => {
                                cur.wrapping_add(paeth_predictor(left, up, left_up))
                            }
                            _ => cur.wrapping_add(up),
                        },
                    );
                }
            }
        }
    }
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = i32::from(a) + i32::from(b) - i32::from(c);
    let pa = (p - i32::from(a)).abs();
    let pb = (p - i32::from(b)).abs();
    let pc = (p - i32::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
