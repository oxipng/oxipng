use libdeflater::{CompressionLvl, Compressor};
use std::{fmt, fmt::Display};

use crate::RowFilter;

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

    /// For heuristic strategies, get an evaluator to determine the best filter for each row.
    pub(crate) fn evaluator(&self) -> Option<Box<dyn StrategyEvaluator>> {
        match self {
            Self::MinSum => Some(Box::new(MinSumEvaluator::new())),
            Self::Entropy => Some(Box::new(EntropyEvaluator::new())),
            Self::Bigrams => Some(Box::new(BigramsEvaluator::new())),
            Self::BigEnt => Some(Box::new(BigEntEvaluator::new())),
            Self::Brute { num_lines, level } => {
                Some(Box::new(BruteEvaluator::new(*num_lines, *level)))
            }
            _ => None,
        }
    }
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

pub(crate) trait StrategyEvaluator {
    /// Reset any state for a new line, if necessary.
    fn reset(&mut self, _line_len: usize) {}
    /// Evaluate the output of a filter attempt, returning true if it's the best so far.
    fn evaluate(&mut self, output: &[u8], offset: usize) -> bool;
}

// MSAD algorithm mentioned in libpng reference docs
// http://www.libpng.org/pub/png/book/chapter09.html
struct MinSumEvaluator {
    best_size: usize,
}
impl MinSumEvaluator {
    const fn new() -> Self {
        Self {
            best_size: usize::MAX,
        }
    }
}
impl StrategyEvaluator for MinSumEvaluator {
    fn reset(&mut self, _line_len: usize) {
        self.best_size = usize::MAX;
    }
    fn evaluate(&mut self, output: &[u8], offset: usize) -> bool {
        let size = output[offset..].iter().fold(0, |acc, &x| {
            let signed = x as i8;
            acc + signed.unsigned_abs() as usize
        });
        if size < self.best_size {
            self.best_size = size;
            return true;
        }
        false
    }
}

// Shannon entropy algorithm, from LodePNG
// https://github.com/lvandeve/lodepng
struct EntropyEvaluator {
    best_size: i32,
}
impl EntropyEvaluator {
    const fn new() -> Self {
        Self {
            best_size: i32::MIN,
        }
    }
}
impl StrategyEvaluator for EntropyEvaluator {
    fn reset(&mut self, _line_len: usize) {
        self.best_size = i32::MIN;
    }
    fn evaluate(&mut self, output: &[u8], offset: usize) -> bool {
        // SIMD-friendly histogram construction
        let mut hist0 = [0_u32; 0x100];
        let mut hist1 = [0_u32; 0x100];
        let mut hist2 = [0_u32; 0x100];
        let mut hist3 = [0_u32; 0x100];
        let (chunks, remainder) = output[offset..].as_chunks::<4>();
        for chunk in chunks {
            hist0[chunk[0] as usize] += 1;
            hist1[chunk[1] as usize] += 1;
            hist2[chunk[2] as usize] += 1;
            hist3[chunk[3] as usize] += 1;
        }
        for &i in remainder {
            hist0[i as usize] += 1;
        }

        let size = (0..0x100).fold(0, |acc, i| {
            let x = hist0[i] + hist1[i] + hist2[i] + hist3[i];
            if x == 0 {
                return acc;
            }
            acc + ilog2i(x)
        }) as i32;
        if size > self.best_size {
            self.best_size = size;
            return true;
        }
        false
    }
}

// Count distinct bigrams, from pngwolf
// https://bjoern.hoehrmann.de/pngwolf/
struct BigramsEvaluator {
    seen: [bool; 0x10000],
    touched: Vec<u16>,
    best_size: usize,
}
impl BigramsEvaluator {
    const fn new() -> Self {
        Self {
            seen: [false; 0x10000],
            touched: Vec::new(),
            best_size: usize::MAX,
        }
    }
}
impl StrategyEvaluator for BigramsEvaluator {
    fn reset(&mut self, _line_len: usize) {
        self.best_size = usize::MAX;
    }
    fn evaluate(&mut self, output: &[u8], offset: usize) -> bool {
        // Clear only the entries that were touched
        for &idx in &self.touched {
            self.seen[idx as usize] = false;
        }
        self.touched.clear();
        let mut count = 0;
        for pair in output[offset..].windows(2) {
            let bigram = ((pair[0] as usize) << 8) | pair[1] as usize;
            if !self.seen[bigram] {
                count += 1;
                if count >= self.best_size {
                    return false;
                }
                self.seen[bigram] = true;
                self.touched.push(bigram as u16);
            }
        }
        self.best_size = count;
        true
    }
}

// Bigram entropy, combined from Entropy and Bigrams filters
struct BigEntEvaluator {
    seen: [u32; 0x10000],
    touched: Vec<u16>,
    best_size: i32,
}
impl BigEntEvaluator {
    const fn new() -> Self {
        Self {
            seen: [0; 0x10000],
            touched: Vec::new(),
            best_size: i32::MIN,
        }
    }
}
impl StrategyEvaluator for BigEntEvaluator {
    fn reset(&mut self, _line_len: usize) {
        self.best_size = i32::MIN;
    }
    fn evaluate(&mut self, output: &[u8], offset: usize) -> bool {
        for pair in output[offset..].windows(2) {
            let bigram = ((pair[0] as usize) << 8) | pair[1] as usize;
            if self.seen[bigram] == 0 {
                self.touched.push(bigram as u16);
            }
            self.seen[bigram] += 1;
        }
        let mut size = 0;
        for &idx in &self.touched {
            size += ilog2i(self.seen[idx as usize]) as i32;
            self.seen[idx as usize] = 0;
        }
        self.touched.clear();
        if size > self.best_size {
            self.best_size = size;
            return true;
        }
        false
    }
}

// Brute force by compressing each filter attempt
// Similar to that of LodePNG but includes some previous lines for context
struct BruteEvaluator {
    num_lines: usize,
    compressor: Compressor,
    limit: usize,
    buffer: Vec<u8>,
}
impl BruteEvaluator {
    fn new(num_lines: usize, level: u8) -> Self {
        let compressor = Compressor::new(CompressionLvl::new(level.into()).unwrap());
        Self {
            num_lines,
            compressor,
            limit: 0,
            buffer: Vec::new(),
        }
    }
}
impl StrategyEvaluator for BruteEvaluator {
    fn reset(&mut self, line_len: usize) {
        self.limit = line_len * self.num_lines;
        let capacity = self.compressor.deflate_compress_bound(self.limit);
        self.buffer.resize(capacity, 0);
    }
    fn evaluate(&mut self, output: &[u8], _offset: usize) -> bool {
        let offset = output.len().saturating_sub(self.limit);
        // Rely on the compressor to fail if the output is larger than our best size (the buffer size)
        let Ok(size) = self
            .compressor
            .deflate_compress(&output[offset..], &mut self.buffer)
        else {
            return false;
        };
        self.buffer.truncate(size - 1);
        true
    }
}

// Integer approximation for i * log2(i) - much faster than float calculations
const fn ilog2i(i: u32) -> u32 {
    let log = 32 - i.leading_zeros() - 1;
    i * log + ((i - (1 << log)) << 1)
}
