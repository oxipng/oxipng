use libdeflater::{CompressionLvl, Compressor};
use rayon::prelude::*;

use super::PngImage;
use crate::filters::{FilterStrategy, RowFilter};
#[cfg(not(feature = "parallel"))]
use crate::rayon;

/// The strategies whose filters are combined.
const CANDIDATES: [FilterStrategy; 10] = [
    FilterStrategy::NONE,
    FilterStrategy::SUB,
    FilterStrategy::UP,
    FilterStrategy::AVERAGE,
    FilterStrategy::PAETH,
    FilterStrategy::MinSum,
    FilterStrategy::Entropy,
    FilterStrategy::Bigrams,
    FilterStrategy::BigEnt,
    FilterStrategy::Brute {
        num_lines: 4,
        level: 1,
    },
];
/// Filters are chosen for sections of about this many bytes.
const SECTION_SIZE: usize = 64 * 1024;
/// Each section is compressed after the deflate window of data already chosen.
const WINDOW_SIZE: usize = 32 * 1024;
/// The libdeflate level that judges each section.
const LEVEL: i32 = 7;

impl PngImage {
    /// Filter the image section by section with the filters of whichever candidate strategy
    /// compresses that section smallest, following the data already chosen.
    pub(crate) fn filter_segmented(&self, optimize_alpha: bool) -> (Vec<u8>, FilterStrategy) {
        let lines: Vec<_> = self.scan_lines(false).collect();

        // The filter for each line chosen by each candidate strategy
        let candidates: Vec<Vec<RowFilter>> = CANDIDATES
            .par_iter()
            .map(|strategy| match strategy {
                FilterStrategy::Basic(filter) => vec![*filter; lines.len()],
                _ => match self.filter_image(strategy.clone(), optimize_alpha).1 {
                    FilterStrategy::Predefined(filters) => filters,
                    _ => unreachable!(),
                },
            })
            .collect();

        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        let alpha_bytes = if optimize_alpha && self.ihdr.color_type.has_alpha() {
            self.bytes_per_channel()
        } else {
            0
        };
        let mut output = Vec::with_capacity(self.ihdr.raw_data_size());
        let mut chosen = Vec::with_capacity(lines.len());
        // The previous line as the filters see it (alpha optimization may alter it)
        let mut prev_line = Vec::new();
        let mut start = 0;
        while start < lines.len() {
            let mut end = start;
            let mut size = 0;
            while end < lines.len() && size < SECTION_SIZE {
                size += lines[end].data.len() + 1;
                end += 1;
            }
            let section = |c: usize| &candidates[c][start..end];
            // Skip candidates whose filters equal an earlier one in this section
            let distinct: Vec<usize> = (0..candidates.len())
                .filter(|&c| !(0..c).any(|other| section(other) == section(c)))
                .collect();
            let context = output.len().saturating_sub(WINDOW_SIZE);

            // Each candidate's section after the context, its compressed size and its last line.
            // The context is the same for every candidate, so the sizes can be compared.
            let (_, best, data, prev) = distinct
                .par_iter()
                .map(|&c| {
                    let mut data = output[context..].to_vec();
                    let mut prev = prev_line.clone();
                    let mut line_data = Vec::new();
                    for (i, &filter) in (start..end).zip(section(c)) {
                        let line = &lines[i];
                        if i == 0 || lines[i - 1].pass != line.pass {
                            prev.clear();
                            prev.resize(line.data.len(), 0);
                        }
                        line_data.clear();
                        line_data.extend_from_slice(line.data);
                        filter.filter_line(bpp, &mut line_data, &prev, &mut data, alpha_bytes);
                        std::mem::swap(&mut prev, &mut line_data);
                    }
                    let size = if distinct.len() > 1 {
                        deflated_size(&data)
                    } else {
                        0
                    };
                    (size, c, data, prev)
                })
                // On equal size the earlier candidate wins
                .min_by_key(|&(size, c, ..)| (size, c))
                .unwrap();
            output.extend_from_slice(&data[output.len() - context..]);
            prev_line = prev;
            chosen.extend_from_slice(section(best));
            start = end;
        }
        (output, FilterStrategy::Predefined(chosen))
    }
}

/// The size of the data compressed with the judging level.
fn deflated_size(data: &[u8]) -> usize {
    let mut compressor = Compressor::new(CompressionLvl::new(LEVEL).unwrap());
    let mut compressed = vec![0; compressor.deflate_compress_bound(data.len())];
    compressor
        .deflate_compress(data, &mut compressed)
        .expect("the buffer has the bound size")
}
