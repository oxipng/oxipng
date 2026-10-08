use indexmap::IndexSet;
use rayon::prelude::*;

use super::PngImage;
#[cfg(not(feature = "parallel"))]
use crate::rayon;
use crate::{
    deflate,
    filters::{FilterStrategy, RowFilter},
};

/// Filters are chosen for sections of about this many bytes.
/// With the context, a trial is about one of libdeflate's blocks (at most 300 000 bytes).
const SECTION_SIZE: usize = 256 * 1024;
/// Each section is compressed after the deflate window of data already chosen.
const WINDOW_SIZE: usize = 32 * 1024;

/// Filters chosen section by section from several strategies
#[derive(Debug)]
pub struct SectionFilters {
    /// The filter for each line
    pub filters: Vec<RowFilter>,
    /// The number of sections
    pub sections: usize,
    /// For each strategy, how many bytes of the image it won
    pub won: Vec<usize>,
}

impl SectionFilters {
    /// The index of the strategy that won the most bytes
    #[must_use]
    pub fn dominant(&self) -> usize {
        let max = self.won.iter().max().copied().unwrap_or(0);
        self.won.iter().position(|&bytes| bytes == max).unwrap_or(0)
    }
}

impl PngImage {
    /// Choose the filters section by section: each section gets the filters of whichever
    /// strategy compresses it smallest at the given level, following the data already chosen.
    ///
    /// Returns `None` if the image is a single section or one strategy wins every section,
    /// since the result would then be that strategy's.
    #[must_use]
    pub fn filter_by_sections(
        &self,
        strategies: &IndexSet<FilterStrategy>,
        optimize_alpha: bool,
        level: u8,
    ) -> Option<SectionFilters> {
        let lines: Vec<_> = self.scan_lines(false).collect();

        // Sections end at line boundaries; a short rest joins the last section
        let mut bounds = vec![0];
        let mut size = 0;
        for (i, line) in lines.iter().enumerate() {
            size += line.data.len() + 1;
            if size >= SECTION_SIZE {
                bounds.push(i + 1);
                size = 0;
            }
        }
        if size > 0 {
            if size < SECTION_SIZE / 2 && bounds.len() > 1 {
                bounds.pop();
            }
            bounds.push(lines.len());
        }
        if bounds.len() < 3 {
            return None;
        }

        // The filter for each line chosen by each strategy
        let candidates: Vec<Vec<RowFilter>> = strategies
            .par_iter()
            .map(|strategy| {
                let mut filters = match strategy {
                    FilterStrategy::Basic(filter) => vec![*filter; lines.len()],
                    FilterStrategy::Predefined(filters) => filters.clone(),
                    _ => match self.filter_image(strategy.clone(), optimize_alpha).1 {
                        FilterStrategy::Predefined(filters) => filters,
                        _ => unreachable!(),
                    },
                };
                // Like filter_image, lines without a predefined filter get None
                filters.resize(lines.len(), RowFilter::None);
                filters
            })
            .collect();

        let bpp = self.bytes_per_channel() * self.channels_per_pixel();
        let alpha_bytes = if optimize_alpha && self.ihdr.color_type.has_alpha() {
            self.bytes_per_channel()
        } else {
            0
        };
        let mut chosen = Vec::with_capacity(lines.len());
        let mut won = vec![0; candidates.len()];
        // The last bytes already chosen, and the previous line as the filters see it
        // (alpha optimization may alter it)
        let mut context = Vec::new();
        let mut prev_line = Vec::new();
        for section in bounds.windows(2) {
            let (start, end) = (section[0], section[1]);
            let section_bytes: usize = lines[start..end].iter().map(|l| l.data.len() + 1).sum();
            let filters = |c: usize| &candidates[c][start..end];
            // Skip candidates whose filters equal an earlier one in this section
            let distinct: Vec<usize> = (0..candidates.len())
                .filter(|&c| (0..c).all(|other| filters(other) != filters(c)))
                .collect();

            // Each candidate's section after the context, its compressed size and its last line.
            // The section is filtered again rather than taken from the strategy's own output,
            // since with alpha optimization a line depends on the filters chosen before it.
            // The context is the same for every candidate, so the sizes compare the sections
            // (up to how the context itself is coded).
            let (_, best, data, prev) = distinct
                .par_iter()
                .map(|&c| {
                    let mut data = Vec::with_capacity(context.len() + section_bytes);
                    data.extend_from_slice(&context);
                    let mut prev = prev_line.clone();
                    // As in filter_image, but reusing the line buffers
                    let mut line_data = Vec::new();
                    for (i, &filter) in (start..end).zip(filters(c)) {
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
                        deflate::deflate(&data, level, None)
                            .expect("the output buffer has the bound size")
                            .len()
                    } else {
                        0
                    };
                    (size, c, data, prev)
                })
                // On equal size the earlier candidate wins
                .min_by_key(|&(size, c, ..)| (size, c))
                .expect("the first candidate is always distinct");
            won[best] += data.len() - context.len();
            context = data[data.len().saturating_sub(WINDOW_SIZE)..].to_vec();
            prev_line = prev;
            chosen.extend_from_slice(filters(best));
        }

        let result = SectionFilters {
            filters: chosen,
            sections: bounds.len() - 1,
            won,
        };
        // If one strategy won everything, the filters are that strategy's
        let total: usize = result.won.iter().sum();
        (result.won[result.dominant()] < total).then_some(result)
    }
}
