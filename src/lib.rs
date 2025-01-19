#![cfg_attr(not(feature = "zopfli"), allow(unreachable_patterns))]

#[cfg(feature = "parallel")]
extern crate rayon;

#[cfg(not(feature = "parallel"))]
mod rayon;

use std::{
    fs::File,
    io::{BufWriter, Read, Write, stdin, stdout},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub use indexmap::{IndexSet, indexset};
use log::{debug, info, trace, warn};
use rayon::prelude::*;
pub use rgb::{RGB16, RGBA8};
use rustc_hash::FxHashMap;

#[cfg(feature = "zopfli")]
pub use crate::deflate::ZopfliOptions;
pub use crate::{
    colors::{BitDepth, ColorType},
    deflate::Deflater,
    error::PngError,
    filters::{FilterStrategy, RowFilter},
    headers::StripChunks,
    options::{InFile, Options, OutFile},
};
use crate::{
    evaluate::{Candidate, Evaluator},
    headers::*,
    png::{PngData, PngImage},
    reduction::*,
};

mod apng;
mod atomicmin;
mod colors;
mod deflate;
mod display_chunks;
mod error;
mod evaluate;
mod filters;
mod headers;
mod options;
mod png;
mod reduction;
#[cfg(feature = "sanity-checks")]
mod sanity_checks;

/// Private to oxipng; don't use outside tests and benches
#[doc(hidden)]
pub mod internal_tests {
    #[cfg(feature = "sanity-checks")]
    pub use crate::sanity_checks::*;
    pub use crate::{deflate::*, png::*, reduction::*};
}

pub type PngResult<T> = Result<T, PngError>;
pub type OptimizationResult = PngResult<(usize, usize)>;

#[derive(Debug)]
/// A raw image definition which can be used to create an optimized png
pub struct RawImage {
    png: Arc<PngImage>,
    aux_chunks: Vec<Chunk>,
}

impl RawImage {
    /// Construct a new raw image definition
    ///
    /// * `width` - The width of the image in pixels
    /// * `height` - The height of the image in pixels
    /// * `color_type` - The color type of the image
    /// * `bit_depth` - The bit depth of the image
    /// * `data` - The raw pixel data of the image
    pub fn new(
        width: u32,
        height: u32,
        color_type: ColorType,
        bit_depth: BitDepth,
        data: Vec<u8>,
    ) -> PngResult<Self> {
        // Validate bit depth
        let valid_depth = match color_type {
            ColorType::Grayscale { .. } => true,
            ColorType::Indexed { .. } => (bit_depth as u8) <= 8,
            _ => (bit_depth as u8) >= 8,
        };
        if !valid_depth {
            return Err(PngError::InvalidDepthForType(bit_depth, color_type));
        }

        // Validate data length
        let bpp = bit_depth as usize * color_type.channels_per_pixel() as usize;
        let row_bytes = (bpp * width as usize).div_ceil(8);
        let expected_len = row_bytes * height as usize;
        if data.len() != expected_len {
            return Err(PngError::IncorrectDataLength(data.len(), expected_len));
        }

        Ok(Self {
            png: Arc::new(PngImage {
                ihdr: IhdrData {
                    width,
                    height,
                    color_type,
                    bit_depth,
                    interlaced: false,
                },
                data,
            }),
            aux_chunks: Vec::new(),
        })
    }

    /// Add a png chunk, such as "iTXt", to be included in the output
    pub fn add_png_chunk(&mut self, name: [u8; 4], data: Vec<u8>) {
        self.aux_chunks.push(Chunk { name, data });
    }

    /// Add an ICC profile for the image
    pub fn add_icc_profile(&mut self, data: &[u8]) {
        // Compress with fastest compression level - will be recompressed during optimization
        let deflater = Deflater::Libdeflater { compression: 1 };
        if let Ok(iccp) = make_iccp(data, deflater, None) {
            self.aux_chunks.push(iccp);
        }
    }

    /// Create an optimized png from the raw image data using the options provided
    pub fn create_optimized_png(&self, opts: &Options) -> PngResult<Vec<u8>> {
        let mut opts = opts.to_owned();
        let mut aux_chunks: Vec<_> = self
            .aux_chunks
            .iter()
            .filter(|c| opts.strip.keep(&c.name))
            .cloned()
            .collect();
        preprocess_chunks(&mut aux_chunks, &mut opts);

        let deadline = Arc::new(Deadline::new(opts.timeout));
        let Some(result) = optimize_raw(self.png.clone(), &opts, deadline, None) else {
            return Err(PngError::new("Failed to optimize input data"));
        };

        let mut png = PngData {
            raw: result.image,
            idat_data: result.idat_data.unwrap(),
            aux_chunks,
            frames: Vec::new(),
        };
        postprocess_chunks(&mut png.aux_chunks, &png.raw.ihdr, &self.png.ihdr);

        Ok(png.output())
    }
}

/// Perform optimization on the input file using the options provided
///
/// Returns the original and optimized file sizes
pub fn optimize(input: &InFile, output: &OutFile, opts: &Options) -> OptimizationResult {
    // Read in the file and try to decode as PNG.
    info!("Processing: {input}");

    let deadline = Arc::new(Deadline::new(opts.timeout));

    let in_data = match *input {
        InFile::Path(ref input_path) => PngData::read_file(input_path)?,
        InFile::StdIn => {
            let mut data = Vec::new();
            stdin()
                .read_to_end(&mut data)
                .map_err(|e| PngError::ReadFailed("stdin".into(), e))?;
            data
        }
    };

    let mut png = PngData::from_slice(&in_data, opts)?;

    // Run the optimizer on the decoded PNG.
    let mut optimized_output = optimize_png(&mut png, &in_data, opts, deadline)?;

    let in_length = in_data.len();

    if is_fully_optimized(in_length, optimized_output.len(), opts) {
        match (output, input) {
            // If output path is None, it also means same as the input path
            (OutFile::Path { path, .. }, InFile::Path(input_path))
                if path.as_ref().is_none_or(|p| p == input_path) =>
            {
                info!("Could not optimize further, no change written: {input}");
                return Ok((in_length, in_length));
            }
            _ => {
                optimized_output = in_data;
            }
        }
    }

    let savings = if in_length >= optimized_output.len() {
        format!(
            "{} bytes ({:.2}% smaller)",
            optimized_output.len(),
            (in_length - optimized_output.len()) as f64 / in_length as f64 * 100_f64
        )
    } else {
        format!(
            "{} bytes ({:.2}% larger)",
            optimized_output.len(),
            (optimized_output.len() - in_length) as f64 / in_length as f64 * 100_f64
        )
    };

    match (output, input) {
        (OutFile::None, _) => {
            info!("{savings}: Dry run, no output");
        }
        (&OutFile::StdOut, _) | (&OutFile::Path { path: None, .. }, &InFile::StdIn) => {
            let mut buffer = BufWriter::new(stdout());
            buffer
                .write_all(&optimized_output)
                .map_err(|e| PngError::WriteFailed("stdout".into(), e))?;
            info!("{savings}: stdout");
        }
        (
            OutFile::Path {
                path,
                preserve_attrs,
            },
            _,
        ) => {
            let input_metadata = if *preserve_attrs {
                input.path().and_then(|in_path| {
                    let meta = in_path.metadata();
                    if let Err(e) = &meta {
                        warn!("Unable to read metadata from {in_path:?}: {e}");
                    }
                    meta.ok()
                })
            } else {
                None
            };

            let output_path = path
                .as_ref()
                .map_or_else(|| input.path().unwrap(), PathBuf::as_path);
            let out_file = File::create(output_path)
                .map_err(|err| PngError::WriteFailed(output_path.display().to_string(), err))?;

            let mut buffer = BufWriter::new(&out_file);
            buffer
                .write_all(&optimized_output)
                // flush BufWriter so IO errors don't get swallowed silently on close() by drop!
                .and_then(|()| buffer.flush())
                .map_err(|e| PngError::WriteFailed(output_path.display().to_string(), e))?;
            // force drop and thereby closing of file handle before modifying any timestamp
            std::mem::drop(buffer);

            if let Some(metadata_input) = &input_metadata {
                let set_time = metadata_input
                    .modified()
                    .and_then(|m| out_file.set_modified(m));
                if let Err(e) = set_time {
                    warn!("Unable to set modification time on {output_path:?}: {e}");
                }
                let set_perm = out_file.set_permissions(metadata_input.permissions());
                if let Err(e) = set_perm {
                    warn!("Unable to set permissions on {output_path:?}: {e}");
                }
            }
            info!("{}: {}", savings, output_path.display());
        }
    }
    Ok((in_length, optimized_output.len()))
}

/// Perform optimization on the input file using the options provided, where the file is already
/// loaded in-memory
pub fn optimize_from_memory(data: &[u8], opts: &Options) -> PngResult<Vec<u8>> {
    // Read in the file and try to decode as PNG.
    info!("Processing from memory");

    let deadline = Arc::new(Deadline::new(opts.timeout));

    let original_size = data.len();
    let mut png = PngData::from_slice(data, opts)?;

    // Run the optimizer on the decoded PNG.
    let optimized_output = optimize_png(&mut png, data, opts, deadline)?;

    if is_fully_optimized(original_size, optimized_output.len(), opts) {
        info!("Image already optimized");
        Ok(data.to_vec())
    } else {
        Ok(optimized_output)
    }
}

/// Perform optimization on the input PNG object using the options provided
fn optimize_png(
    png: &mut PngData,
    original_data: &[u8],
    opts: &Options,
    deadline: Arc<Deadline>,
) -> PngResult<Vec<u8>> {
    // Print png info
    let file_original_size = original_data.len();
    let idat_original_size = png.idat_data.len();
    let raw = png.raw.clone();
    debug!(
        "    {}x{} pixels, PNG format",
        raw.ihdr.width, raw.ihdr.height
    );
    report_format(&raw, &raw.ihdr.color_type.to_string());
    debug!("    IDAT size = {idat_original_size} bytes");
    debug!("    File size = {file_original_size} bytes");

    let mut opts = opts.to_owned();
    preprocess_chunks(&mut png.aux_chunks, &mut opts);

    let max_size = if opts.force {
        None
    } else {
        Some(png.raw.estimated_output_size(&png.idat_data))
    };
    if let Some(result) = optimize_raw(raw.clone(), &opts, deadline.clone(), max_size) {
        png.raw = result.image;
        png.idat_data = result.idat_data.unwrap();
        recompress_frames(png, &opts, deadline, result.filter)?;
        postprocess_chunks(&mut png.aux_chunks, &png.raw.ihdr, &raw.ihdr);
    }

    let output = png.output();

    if idat_original_size >= png.idat_data.len() {
        debug!(
            "    IDAT size = {} bytes ({} bytes decrease)",
            png.idat_data.len(),
            idat_original_size - png.idat_data.len()
        );
    } else {
        debug!(
            "    IDAT size = {} bytes ({} bytes increase)",
            png.idat_data.len(),
            png.idat_data.len() - idat_original_size
        );
    }
    if file_original_size >= output.len() {
        debug!(
            "    file size = {} bytes ({} bytes = {:.2}% decrease)",
            output.len(),
            file_original_size - output.len(),
            (file_original_size - output.len()) as f64 / file_original_size as f64 * 100_f64
        );
    } else {
        debug!(
            "    file size = {} bytes ({} bytes = {:.2}% increase)",
            output.len(),
            output.len() - file_original_size,
            (output.len() - file_original_size) as f64 / file_original_size as f64 * 100_f64
        );
    }

    if opts.interlace == Some(true) && !png.raw.ihdr.interlaced {
        warn!(
            "Interlacing was not enabled as it would result in a larger file. To override this, use `--force`."
        );
    }

    #[cfg(feature = "sanity-checks")]
    assert!(sanity_checks::validate_output(&output, original_data));

    Ok(output)
}

/// Perform optimization on the input image data using the options provided
fn optimize_raw(
    image: Arc<PngImage>,
    opts: &Options,
    deadline: Arc<Deadline>,
    max_size: Option<usize>,
) -> Option<Candidate> {
    // Phase 1: Transformation evaluations

    // Libdeflate has four algorithms: 0 = 'uncompressed', 1-4 = 'greedy', 5-7 = 'lazy', 8-9 = 'lazy2', 10-12 = 'near-optimal'
    // 5 is the minimumm required for a decent evaluation result (o0 and o1)
    // 7 is only slightly slower than 5 and improves evaluation of filters in 'fast' mode (o2)
    // 8 is a little slower but not noticeably when used only for reductions (o3 and higher)
    // 9 is not appreciably better than 8
    // 10 and higher are quite slow - good for filters but only good for reductions if matching the main zc level
    let compression = match opts.deflater {
        Deflater::Libdeflater { compression } if compression < 11 => 5.min(compression),
        _ if opts.filters.len() <= 4 => 7,
        _ => 8,
    };
    let eval_deflater = Deflater::Libdeflater { compression };
    // If only one or two filters are given, use them for evaluations
    let eval_filters = if opts.filters.len() <= 2 {
        opts.filters.clone()
    } else {
        // None and Bigrams work well together, especially for alpha reductions
        indexset! {FilterStrategy::NONE, FilterStrategy::Bigrams}
    };
    // If phase 3 is not needed, we need to retain the IDAT in the evaluation phases instead
    let do_phase_3 = opts.max_trials > 0 && opts.deflater != eval_deflater;
    // This will collect all versions of images and pick one that compresses best
    let eval = Evaluator::new(
        deadline.clone(),
        eval_filters.clone(),
        eval_deflater,
        false,
        opts.tolerance,
        !do_phase_3,
    );
    let new_image = perform_reductions(image.clone(), opts, &deadline, &eval);
    let mut candidates = eval.get_candidates();

    // Return early if idat recoding is off and no transformations occurred
    if !opts.idat_recoding && candidates.is_empty() && new_image.is_same_format(&image) {
        return None;
    }

    // Phase 2: Filter evaluations

    let mut filters = opts.filters.clone();
    if !candidates.is_empty() {
        // Some filters have already been evaluated, we don't need to try them again
        filters = filters.difference(&eval_filters).cloned().collect();
    }

    if !filters.is_empty() && !deadline.passed() {
        // Evaluate the remaining filters
        let eval = Evaluator::new(
            deadline.clone(),
            filters.clone(),
            eval_deflater,
            opts.optimize_alpha,
            opts.tolerance,
            !do_phase_3,
        );
        if candidates.is_empty() {
            trace!("Evaluating {} filters", filters.len());
            eval.try_image(new_image, 0);
        } else {
            // The results could include multiple filter evaluations of the same image
            // Filter out duplicates based on their `nth` value
            let distinct = candidates
                .iter()
                .take(opts.max_trials.max(1))
                .map(|c| (c.nth, c))
                .collect::<FxHashMap<_, _>>();
            trace!(
                "Evaluating {} filters for {} candidates",
                filters.len(),
                distinct.len()
            );
            for (nth, candidate) in distinct {
                eval.try_image_with_name(candidate.image.clone(), nth, &candidate.name);
            }
            eval.inject_candidates(candidates);
        }
        candidates = eval.get_candidates();
    }

    // Phase 3: Main compression trials

    let result = if do_phase_3 && !deadline.passed() {
        // Collect distinct candidates up to max_trials
        // Filter out duplicates where the filter strategy chose the same row filters as a previous one
        let mut distinct: Vec<Candidate> = Vec::new();
        for a in candidates {
            if distinct.len() < opts.max_trials
                && !distinct.iter().any(|b| {
                    a.nth == b.nth
                        && a.estimated_output_size == b.estimated_output_size
                        && a.filter_used.matches(&b.filter_used)
                })
            {
                distinct.push(a);
            }
        }
        debug!(
            "Compressing {} candidates with {}",
            distinct.len(),
            opts.deflater
        );
        Evaluator::recompress_candidates(
            distinct,
            deadline,
            opts.deflater,
            opts.optimize_alpha,
            max_size,
        )?
    } else {
        candidates.into_iter().next()?
    };

    if result.idat_data.is_some()
        && max_size.is_none_or(|max_size| result.estimated_output_size < max_size)
    {
        debug!("Found better result:");
        if !result.image.is_same_format(&image) {
            report_format(&result.image, &result.name);
        }
        debug!("    {}, f = {}", opts.deflater, result.filter);
        return Some(result);
    }
    None
}

#[derive(Debug)]
struct DeadlineImp {
    start: Instant,
    timeout: Duration,
    print_message: AtomicBool,
}

/// Keep track of processing timeout
#[doc(hidden)]
#[derive(Debug)]
pub struct Deadline {
    imp: Option<DeadlineImp>,
}

impl Deadline {
    #[must_use]
    pub fn new(timeout: Option<Duration>) -> Self {
        Self {
            imp: timeout.map(|timeout| DeadlineImp {
                start: Instant::now(),
                timeout,
                print_message: AtomicBool::new(true),
            }),
        }
    }

    /// True if the timeout has passed, and no new work should be done.
    ///
    /// If the verbose option is on, it also prints a timeout message once.
    pub fn passed(&self) -> bool {
        if let Some(imp) = &self.imp {
            let elapsed = imp.start.elapsed();
            if elapsed > imp.timeout {
                if match imp.print_message.compare_exchange(
                    true,
                    false,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(x) | Err(x) => x,
                } {
                    warn!("Timed out after {} second(s)", elapsed.as_secs());
                }
                return true;
            }
        }
        false
    }
}

/// Display the format of the image data
fn report_format(png: &PngImage, color_type: &str) {
    let interlaced = if png.ihdr.interlaced {
        "interlaced"
    } else {
        "non-interlaced"
    };
    debug!(
        "    {}-bit {}, {}",
        png.ihdr.bit_depth, color_type, interlaced
    );
}

/// Recompress the additional frames of an APNG
fn recompress_frames(
    png: &mut PngData,
    opts: &Options,
    deadline: Arc<Deadline>,
    filter: FilterStrategy,
) -> PngResult<()> {
    if !opts.idat_recoding || png.frames.is_empty() {
        return Ok(());
    }
    // Ensure we don't try to recompress frames with a predefined filter
    debug_assert!(!matches!(filter, FilterStrategy::Predefined { .. }));
    png.frames
        .par_iter_mut()
        .with_max_len(1)
        .enumerate()
        .try_for_each(|(i, frame)| {
            if deadline.passed() {
                return Ok(());
            }
            let mut ihdr = png.raw.ihdr.clone();
            ihdr.width = frame.width;
            ihdr.height = frame.height;
            let image = PngImage::new(ihdr, &frame.data)?;
            let (filtered, _) = image.filter_image(filter.clone(), opts.optimize_alpha);
            let max_size = Some(frame.data.len() - 1);
            if let Ok(data) = opts.deflater.deflate(&filtered, max_size) {
                debug!(
                    "Recompressed fdAT #{:<2}: {} ({} bytes decrease)",
                    i,
                    data.len(),
                    frame.data.len() - data.len()
                );
                frame.data = data;
            }
            Ok(())
        })
}

/// Check if an image was already optimized prior to oxipng's operations
const fn is_fully_optimized(original_size: usize, optimized_size: usize, opts: &Options) -> bool {
    original_size <= optimized_size && !opts.force
}
