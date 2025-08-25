use std::sync::Arc;

use crate::{ColorType, Deadline, Deflaters, Options, evaluate::Evaluator, png::PngImage};

pub mod alpha;
use crate::alpha::*;
pub mod bit_depth;
use crate::bit_depth::*;
pub mod color;
use crate::color::*;
pub mod palette;
use crate::palette::*;

pub(crate) fn perform_reductions(
    mut png: Arc<PngImage>,
    opts: &Options,
    deadline: &Deadline,
    eval: &Evaluator,
) -> Arc<PngImage> {
    let mut evaluation_added = false;

    // At low compression levels, skip some transformations which are less likely to be effective
    // This currently affects optimization presets 0-2
    let cheap = match opts.deflate {
        Deflaters::Libdeflater { compression } => compression < 12 && opts.fast_evaluation,
        _ => false,
    };

    // Interlacing must be processed first in order to evaluate the rest correctly
    if let Some(interlacing) = opts.interlace {
        if let Some(reduced) = png.change_interlacing(interlacing) {
            png = Arc::new(reduced);
        }
    }

    // If alpha optimization is enabled, clean the alpha channel before continuing
    // This can allow some color type reductions which may not have been possible otherwise
    if opts.optimize_alpha && !deadline.passed() {
        if let Some(reduced) = cleaned_alpha_channel(&png) {
            png = Arc::new(reduced);
        }
    }

    // Attempt to reduce 16-bit to 8-bit
    // This is just removal of bytes and does not need to be evaluated
    if opts.bit_depth_reduction && !deadline.passed() {
        if let Some(reduced) = reduced_bit_depth_16_to_8(&png, opts.scale_16) {
            png = Arc::new(reduced);
        }
    }

    // Attempt to reduce RGB to grayscale
    // This is just removal of bytes and does not need to be evaluated
    if opts.color_type_reduction && opts.grayscale_reduction && !deadline.passed() {
        if let Some(reduced) = reduced_rgb_to_grayscale(&png) {
            png = Arc::new(reduced);
        }
    }

    // Attempt to expand the bit depth to 8
    // This does need to be evaluated but will be done so later when it gets reduced again
    if opts.bit_depth_reduction && !deadline.passed() {
        if let Some(reduced) = expanded_bit_depth_to_8(&png) {
            png = Arc::new(reduced);
        }
    }

    // Now retain the current png for the evaluator baseline
    // It will only be entered into the evaluator if there are also others to evaluate
    let mut baseline = png.clone();

    // Attempt to reduce and sort the palette
    if opts.palette_reduction && !deadline.passed() {
        if let Some(reduced) = reduced_palette(&png, opts.optimize_alpha) {
            png = Arc::new(reduced);
            // If the palette was reduced but the data is unchanged then this should become the baseline
            if png.data == baseline.data {
                baseline = png.clone();
            }
        }
        if let Some(reduced) = sorted_palette(&png) {
            png = Arc::new(reduced);
        }
        // If either action changed the data then enter this into the evaluator
        if !Arc::ptr_eq(&png, &baseline) {
            eval.try_image_with_description(png.clone(), "Indexed (luma sort)");
            evaluation_added = true;
        }
    }

    // Attempt alpha removal
    if opts.color_type_reduction && !deadline.passed() {
        if let Some(reduced) = reduced_alpha_channel(&png, opts.optimize_alpha) {
            png = Arc::new(reduced);
            // For small differences, if a tRNS chunk is required then enter this into the evaluator
            // Otherwise it is mostly just removal of bytes and should become the baseline
            if png.ihdr.color_type.has_trns() && baseline.data.len() - png.data.len() <= 1000 {
                eval.try_image(png.clone());
                evaluation_added = true;
            } else {
                baseline = png.clone();
            }
        }
    }

    // Attempt to convert from indexed to channels
    // This may give a better result due to dropping the PLTE chunk
    if !cheap && opts.color_type_reduction && !deadline.passed() {
        if let Some(reduced) =
            indexed_to_channels(&png, opts.grayscale_reduction, opts.optimize_alpha)
        {
            // This result should not be passed on to subsequent reductions
            eval.try_image(Arc::new(reduced));
            evaluation_added = true;
        }
    }

    // Attempt to reduce to indexed
    // Keep the existing `png` var in case it is grayscale - we can test both for depth reduction later
    let mut indexed = None;
    if opts.color_type_reduction && !deadline.passed() {
        if let Some(reduced) = reduced_to_indexed(&png, opts.grayscale_reduction) {
            // Make sure the palette gets sorted (but don't bother evaluating both results)
            let new = Arc::new(sorted_palette(&reduced).unwrap_or(reduced));
            // For relatively small differences, enter this into the evaluator
            // Otherwise we're confident enough for it to become the baseline
            if png.data.len() - new.data.len() <= INDEXED_MAX_DIFF {
                eval.try_image_with_description(new.clone(), "Indexed (luma sort)");
                evaluation_added = true;
            } else {
                baseline = new.clone();
            }
            indexed = Some(new);
        }
    }

    // Attempt additional palette sorting techniques
    if !cheap && opts.palette_reduction {
        // Collect a list of palettes so we can avoid evaluating the same one twice
        let mut palettes = Vec::new();
        if let ColorType::Indexed { palette } = &baseline.ihdr.color_type {
            palettes.push(palette.clone());
        }
        // Make sure we use the `indexed` var as input if it exists
        // This one doesn't need to be kept in the palette list as the sorters will fail if there's no change
        let input = indexed.as_ref().unwrap_or(&png);

        // Attempt to sort the palette using the battiato method
        if !deadline.passed() {
            if let Some(reduced) = sorted_palette_battiato(input) {
                if let ColorType::Indexed { palette } = &reduced.ihdr.color_type {
                    if !palettes.contains(palette) {
                        palettes.push(palette.clone());
                        eval.try_image_with_description(
                            Arc::new(reduced),
                            "Indexed (battiato sort)",
                        );
                        evaluation_added = true;
                    }
                }
            }
        }

        // Attempt to sort the palette using the mzeng method
        if !deadline.passed() {
            if let Some(reduced) = sorted_palette_mzeng(input) {
                if let ColorType::Indexed { palette } = &reduced.ihdr.color_type {
                    if !palettes.contains(palette) {
                        palettes.push(palette.clone());
                        eval.try_image_with_description(Arc::new(reduced), "Indexed (mzeng sort)");
                        evaluation_added = true;
                    }
                }
            }
        }
    }

    // Attempt to reduce to a lower bit depth
    if opts.bit_depth_reduction && !deadline.passed() {
        // First try the `png` var
        let reduced = reduced_bit_depth_8_or_less(&png);
        // Then try the `indexed` var, unless we're doing cheap evaluations and already have a reduction
        if (!cheap || reduced.is_none()) && !deadline.passed() {
            if let Some(indexed) = indexed.and_then(|png| reduced_bit_depth_8_or_less(&png)) {
                // Only evaluate this if it's different from the first result (which must be grayscale if it exists)
                if reduced.as_ref().is_none_or(|r| r.data != indexed.data) {
                    eval.try_image(Arc::new(indexed));
                    evaluation_added = true;
                }
            }
        }
        // Enter the first result into the evaluator
        if let Some(reduced) = reduced {
            eval.try_image(Arc::new(reduced));
            evaluation_added = true;
        }
    }

    if evaluation_added {
        eval.try_image(baseline.clone());
    }
    baseline
}
