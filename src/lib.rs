#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(clippy::expl_impl_clone_on_copy)]
#![warn(clippy::float_cmp_const)]
#![warn(clippy::linkedlist)]
#![warn(clippy::map_flatten)]
#![warn(clippy::match_same_arms)]
#![warn(clippy::mem_forget)]
#![warn(clippy::mut_mut)]
#![warn(clippy::mutex_integer)]
#![warn(clippy::needless_continue)]
#![warn(clippy::path_buf_push_overwrite)]
#![warn(clippy::range_plus_one)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::upper_case_acronyms)]
#![cfg_attr(
    not(feature = "zopfli"),
    allow(irrefutable_let_patterns),
    allow(unreachable_patterns)
)]

#[cfg(feature = "parallel")]
extern crate rayon;
#[cfg(not(feature = "parallel"))]
mod rayon;

use crate::atomicmin::AtomicMin;
use crate::evaluate::Evaluator;
use crate::headers::*;
use crate::png::PngData;
use crate::png::PngImage;
use crate::reduction::*;
use log::{debug, info, trace, warn};
use rayon::prelude::*;
use std::borrow::Cow;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use crate::colors::{BitDepth, ColorType};
pub use crate::deflate::Deflaters;
pub use crate::error::PngError;
pub use crate::filters::RowFilter;
pub use crate::headers::StripChunks;
pub use crate::interlace::Interlacing;
pub use indexmap::{indexset, IndexSet};
pub use rgb::{RGB16, RGBA8};

mod atomicmin;
mod colors;
mod deflate;
mod error;
mod evaluate;
mod filters;
mod headers;
mod interlace;
mod png;
mod reduction;
#[cfg(feature = "sanity-checks")]
mod sanity_checks;

/// Private to oxipng; don't use outside tests and benches
#[doc(hidden)]
pub mod internal_tests {
    pub use crate::atomicmin::*;
    pub use crate::deflate::*;
    pub use crate::png::*;
    pub use crate::reduction::*;
    #[cfg(feature = "sanity-checks")]
    pub use crate::sanity_checks::*;
}

#[derive(Clone, Debug)]
pub enum OutFile {
    /// Don't actually write any output, just calculate the best results.
    None,
    /// Write output to a file.
    ///
    /// * `path`: Path to write the output file. `None` means same as input.
    /// * `preserve_attrs`: Ensure the output file has the same permissions & timestamps as the input file.
    Path {
        path: Option<PathBuf>,
        preserve_attrs: bool,
    },
    /// Write to standard output.
    StdOut,
}

impl OutFile {
    /// Construct a new `OutFile` with the given path.
    ///
    /// This is a convenience method for `OutFile::Path { path: Some(path), preserve_attrs: false }`.
    pub fn from_path(path: PathBuf) -> Self {
        OutFile::Path {
            path: Some(path),
            preserve_attrs: false,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match *self {
            OutFile::Path {
                path: Some(ref p), ..
            } => Some(p.as_path()),
            _ => None,
        }
    }
}

/// Where to read images from
#[derive(Clone, Debug)]
pub enum InFile {
    Path(PathBuf),
    StdIn,
}

impl InFile {
    pub fn path(&self) -> Option<&Path> {
        match *self {
            InFile::Path(ref p) => Some(p.as_path()),
            InFile::StdIn => None,
        }
    }
}

impl fmt::Display for InFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            InFile::Path(ref p) => write!(f, "{}", p.display()),
            InFile::StdIn => f.write_str("stdin"),
        }
    }
}

impl<T: Into<PathBuf>> From<T> for InFile {
    fn from(s: T) -> Self {
        InFile::Path(s.into())
    }
}

pub type PngResult<T> = Result<T, PngError>;

#[derive(Clone, Debug)]
/// Options controlling the output of the `optimize` function
pub struct Options {
    /// Attempt to fix errors when decoding the input file rather than returning an `Err`.
    ///
    /// Default: `false`
    pub fix_errors: bool,
    /// Write to output even if there was no improvement in compression.
    ///
    /// Default: `false`
    pub force: bool,
    /// Which RowFilters to try on the file
    ///
    /// Default: `None,Sub,Entropy,Bigrams`
    pub filter: IndexSet<RowFilter>,
    /// Whether to change the interlacing type of the file.
    ///
    /// `None` will not change the current interlacing type.
    ///
    /// `Some(x)` will change the file to interlacing mode `x`.
    ///
    /// Default: `Some(Interlacing::None)`
    pub interlace: Option<Interlacing>,
    /// Whether to allow transparent pixels to be altered to improve compression.
    pub optimize_alpha: bool,
    /// Whether to attempt bit depth reduction
    ///
    /// Default: `true`
    pub bit_depth_reduction: bool,
    /// Whether to attempt color type reduction
    ///
    /// Default: `true`
    pub color_type_reduction: bool,
    /// Whether to attempt palette reduction
    ///
    /// Default: `true`
    pub palette_reduction: bool,
    /// Whether to attempt grayscale reduction
    ///
    /// Default: `true`
    pub grayscale_reduction: bool,
    /// Whether to perform recoding of IDAT and other compressed chunks
    ///
    /// If any type of reduction is performed, IDAT recoding will be performed
    /// regardless of this setting
    ///
    /// Default: `true`
    pub idat_recoding: bool,
    /// Whether to forcibly reduce 16-bit to 8-bit by scaling
    ///
    /// Default: `false`
    pub scale_16: bool,
    /// Which chunks to strip from the PNG file, if any
    ///
    /// Default: `None`
    pub strip: StripChunks,
    /// Which DEFLATE algorithm to use
    ///
    /// Default: `Libdeflater`
    pub deflate: Deflaters,
    /// Whether to use fast evaluation to pick the best filter
    ///
    /// Default: `true`
    pub fast_evaluation: bool,

    /// Maximum amount of time to spend on optimizations.
    /// Further potential optimizations are skipped if the timeout is exceeded.
    pub timeout: Option<Duration>,
}

impl Options {
    pub fn from_preset(level: u8) -> Options {
        let opts = Options::default();
        match level {
            0 => opts.apply_preset_0(),
            1 => opts.apply_preset_1(),
            2 => opts.apply_preset_2(),
            3 => opts.apply_preset_3(),
            4 => opts.apply_preset_4(),
            5 => opts.apply_preset_5(),
            6 => opts.apply_preset_6(),
            _ => {
                warn!("Level 7 and above don't exist yet and are identical to level 6");
                opts.apply_preset_6()
            }
        }
    }

    pub fn max_compression() -> Options {
        Options::from_preset(6)
    }

    // The following methods make assumptions that they are operating
    // on an `Options` struct generated by the `default` method.
    fn apply_preset_0(mut self) -> Self {
        self.filter.clear();
        if let Deflaters::Libdeflater { compression } = &mut self.deflate {
            *compression = 5;
        }
        self
    }

    fn apply_preset_1(mut self) -> Self {
        self.filter.clear();
        if let Deflaters::Libdeflater { compression } = &mut self.deflate {
            *compression = 10;
        }
        self
    }

    fn apply_preset_2(self) -> Self {
        self
    }

    fn apply_preset_3(mut self) -> Self {
        self.fast_evaluation = false;
        self.filter = indexset! {
            RowFilter::None,
            RowFilter::Bigrams,
            RowFilter::BigEnt,
            RowFilter::Brute
        };
        self
    }

    fn apply_preset_4(mut self) -> Self {
        if let Deflaters::Libdeflater { compression } = &mut self.deflate {
            *compression = 12;
        }
        self.apply_preset_3()
    }

    fn apply_preset_5(mut self) -> Self {
        self.fast_evaluation = false;
        self.filter.insert(RowFilter::Up);
        self.filter.insert(RowFilter::MinSum);
        self.filter.insert(RowFilter::BigEnt);
        self.filter.insert(RowFilter::Brute);
        if let Deflaters::Libdeflater { compression } = &mut self.deflate {
            *compression = 12;
        }
        self
    }

    fn apply_preset_6(mut self) -> Self {
        self.filter.insert(RowFilter::Average);
        self.filter.insert(RowFilter::Paeth);
        self.apply_preset_5()
    }
}

impl Default for Options {
    fn default() -> Options {
        // Default settings based on -o 2 from the CLI interface
        Options {
            fix_errors: false,
            force: false,
            filter: indexset! {RowFilter::None, RowFilter::Sub, RowFilter::Entropy, RowFilter::Bigrams},
            interlace: Some(Interlacing::None),
            optimize_alpha: false,
            bit_depth_reduction: true,
            color_type_reduction: true,
            palette_reduction: true,
            grayscale_reduction: true,
            idat_recoding: true,
            scale_16: false,
            strip: StripChunks::None,
            deflate: Deflaters::Libdeflater { compression: 11 },
            fast_evaluation: true,
            timeout: None,
        }
    }
}

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
    ) -> Result<Self, PngError> {
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
        let row_bytes = (bpp * width as usize + 7) / 8;
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
                    interlaced: Interlacing::None,
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
        let deflater = Deflaters::Libdeflater { compression: 1 };
        if let Ok(iccp) = construct_iccp(data, deflater) {
            self.aux_chunks.push(iccp);
        }
    }

    /// Create an optimized png from the raw image data using the options provided
    pub fn create_optimized_png(&self, opts: &Options) -> PngResult<Vec<u8>> {
        let deadline = Arc::new(Deadline::new(opts.timeout));
        let mut png = optimize_raw(self.png.clone(), opts, deadline.clone(), None)
            .ok_or_else(|| PngError::new("Failed to optimize input data"))?;

        // Process aux chunks
        png.aux_chunks = self
            .aux_chunks
            .iter()
            .filter(|c| opts.strip.keep(&c.name))
            .cloned()
            .collect();
        postprocess_chunks(&mut png, opts, deadline, &self.png.ihdr);

        Ok(png.output())
    }
}

/// Perform optimization on the input file using the options provided
pub fn optimize(input: &InFile, output: &OutFile, opts: &Options) -> PngResult<()> {
    // Read in the file and try to decode as PNG.
    info!("Processing: {}", input);

    let deadline = Arc::new(Deadline::new(opts.timeout));

    // grab metadata before even opening input file to preserve atime
    let opt_metadata_preserved;
    let in_data = match *input {
        InFile::Path(ref input_path) => {
            if matches!(
                output,
                OutFile::Path {
                    preserve_attrs: true,
                    ..
                }
            ) {
                opt_metadata_preserved = input_path
                    .metadata()
                    .map_err(|err| {
                        // Fail if metadata cannot be preserved
                        PngError::new(&format!(
                            "Unable to read metadata from input file {:?}: {}",
                            input_path, err
                        ))
                    })
                    .map(Some)?;
                trace!("preserving metadata: {:?}", opt_metadata_preserved);
            } else {
                opt_metadata_preserved = None;
            }
            PngData::read_file(input_path)?
        }
        InFile::StdIn => {
            opt_metadata_preserved = None;
            let mut data = Vec::new();
            stdin()
                .read_to_end(&mut data)
                .map_err(|e| PngError::new(&format!("Error reading stdin: {}", e)))?;
            data
        }
    };

    let mut png = PngData::from_slice(&in_data, opts)?;

    // Run the optimizer on the decoded PNG.
    let mut optimized_output = optimize_png(&mut png, &in_data, opts, deadline)?;

    let in_length = in_data.len();

    if is_fully_optimized(in_data.len(), optimized_output.len(), opts) {
        match (output, input) {
            // if p is None, it also means same as the input path
            (OutFile::Path { path, .. }, InFile::Path(ref input_path))
                if path.as_ref().map_or(true, |p| p == input_path) =>
            {
                info!("{}: Could not optimize further, no change written", input);
                return Ok(());
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
            info!("{}: Running in pretend mode, no output", savings);
        }
        (&OutFile::StdOut, _) | (&OutFile::Path { path: None, .. }, &InFile::StdIn) => {
            let mut buffer = BufWriter::new(stdout());
            buffer
                .write_all(&optimized_output)
                .map_err(|e| PngError::new(&format!("Unable to write to stdout: {}", e)))?;
        }
        (OutFile::Path { path, .. }, _) => {
            let output_path = path
                .as_ref()
                .map(|p| p.as_path())
                .unwrap_or_else(|| input.path().unwrap());
            let out_file = File::create(output_path).map_err(|err| {
                PngError::new(&format!(
                    "Unable to write to file {}: {}",
                    output_path.display(),
                    err
                ))
            })?;
            if let Some(metadata_input) = &opt_metadata_preserved {
                copy_permissions(metadata_input, &out_file)?;
            }

            let mut buffer = BufWriter::new(out_file);
            buffer
                .write_all(&optimized_output)
                // flush BufWriter so IO errors don't get swallowed silently on close() by drop!
                .and_then(|()| buffer.flush())
                .map_err(|e| {
                    PngError::new(&format!(
                        "Unable to write to {}: {}",
                        output_path.display(),
                        e
                    ))
                })?;
            // force drop and thereby closing of file handle before modifying any timestamp
            std::mem::drop(buffer);
            if let Some(metadata_input) = &opt_metadata_preserved {
                copy_times(metadata_input, output_path)?;
            }
            info!("{}: {}", savings, output_path.display());
        }
    }
    Ok(())
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

type TrialResult = (RowFilter, Vec<u8>);

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
    report_format("    ", &raw);
    debug!("    IDAT size = {} bytes", idat_original_size);
    debug!("    File size = {} bytes", file_original_size);

    // Check for APNG by presence of acTL chunk
    let opts = if png.aux_chunks.iter().any(|c| &c.name == b"acTL") {
        warn!("APNG detected, disabling all reductions");
        let mut opts = opts.to_owned();
        opts.interlace = None;
        opts.bit_depth_reduction = false;
        opts.color_type_reduction = false;
        opts.palette_reduction = false;
        opts.grayscale_reduction = false;
        Cow::Owned(opts)
    } else {
        Cow::Borrowed(opts)
    };
    let max_size = if opts.force {
        None
    } else {
        Some(png.estimated_output_size())
    };
    if let Some(new_png) = optimize_raw(raw.clone(), &opts, deadline.clone(), max_size) {
        png.raw = new_png.raw;
        png.idat_data = new_png.idat_data;
    }

    postprocess_chunks(png, &opts, deadline, &raw.ihdr);

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
) -> Option<PngData> {
    // Libdeflate has four algorithms: 1-4 = 'greedy', 5-7 = 'lazy', 8-9 = 'lazy2', 10-12 = 'near-optimal'
    // 5 is the minimumm required for a decent evaluation result
    // 7 is not noticeably slower than 5 and improves evaluation of filters in 'fast' mode (o2 and lower)
    // 8 is a little slower but not noticeably when used only for reductions (o3 and higher)
    // 9 is not appreciably better than 8
    // 10 and higher are quite slow - good for filters but only good for reductions if matching the main zc level
    let eval_compression = match opts.deflate {
        Deflaters::Libdeflater { compression } => {
            if opts.fast_evaluation { 7 } else { 8 }.min(compression)
        }
        _ => 8,
    };
    // None and Bigrams work well together, especially for alpha reductions
    let eval_filters = indexset! {RowFilter::None, RowFilter::Bigrams};
    // This will collect all versions of images and pick one that compresses best
    let eval = Evaluator::new(
        deadline.clone(),
        eval_filters.clone(),
        eval_compression,
        false,
    );
    let mut png = perform_reductions(image.clone(), opts, &deadline, &eval);
    let mut eval_result = eval.get_best_candidate();
    if let Some(ref result) = eval_result {
        png = result.image.clone();
    }
    let reduction_occurred = png.ihdr.color_type != image.ihdr.color_type
        || png.ihdr.bit_depth != image.ihdr.bit_depth
        || png.ihdr.interlaced != image.ihdr.interlaced;

    if reduction_occurred {
        report_format("Reducing image to ", &png);
    }

    if opts.idat_recoding || reduction_occurred {
        let mut filters = opts.filter.clone();
        let fast_eval = opts.fast_evaluation && (filters.len() > 1 || eval_result.is_some());
        let best: Option<TrialResult> = if fast_eval {
            // Perform a fast evaluation of selected filters followed by a single main compression trial

            if eval_result.is_some() {
                // Some filters have already been evaluated, we don't need to try them again
                filters = filters.difference(&eval_filters).cloned().collect();
            }

            if !filters.is_empty() {
                trace!("Evaluating: {} filters", filters.len());
                let eval = Evaluator::new(deadline, filters, eval_compression, opts.optimize_alpha);
                if let Some(ref result) = eval_result {
                    eval.set_best_size(result.idat_data.len());
                }
                eval.try_image(png.clone());
                if let Some(result) = eval.get_best_candidate() {
                    eval_result = Some(result);
                }
            }
            // We should have a result here - fail if not (e.g. deadline passed)
            let result = eval_result?;

            match opts.deflate {
                Deflaters::Libdeflater { compression } if compression <= eval_compression => {
                    // No further compression required
                    Some((result.filter, result.idat_data))
                }
                _ => {
                    debug!("Trying: {}", result.filter);
                    let best_size = AtomicMin::new(max_size);
                    perform_trial(&result.filtered, opts, result.filter, &best_size)
                }
            }
        } else {
            // Perform full compression trials of selected filters and determine the best

            if filters.is_empty() {
                // Pick a filter automatically
                if png.ihdr.bit_depth as u8 >= 8 {
                    // Bigrams is the best all-rounder when there's at least one byte per pixel
                    filters.insert(RowFilter::Bigrams);
                } else {
                    // Otherwise delta filters generally don't work well, so just stick with None
                    filters.insert(RowFilter::None);
                }
            }

            debug!("Trying: {} filters", filters.len());

            let best_size = AtomicMin::new(max_size);
            let results_iter = filters.into_par_iter().with_max_len(1);
            let best = results_iter.filter_map(|filter| {
                if deadline.passed() {
                    return None;
                }
                let filtered = &png.filter_image(filter, opts.optimize_alpha);
                perform_trial(filtered, opts, filter, &best_size)
            });
            best.reduce_with(|i, j| {
                if i.1.len() < j.1.len() || (i.1.len() == j.1.len() && i.0 < j.0) {
                    i
                } else {
                    j
                }
            })
        };

        if let Some((filter, idat_data)) = best {
            let image = PngData {
                raw: png,
                idat_data,
                aux_chunks: Vec::new(),
            };
            if image.estimated_output_size() < max_size.unwrap_or(usize::MAX) {
                debug!("Found better combination:");
                debug!(
                    "    zc = {}  f = {:8}  {} bytes",
                    opts.deflate,
                    filter,
                    image.idat_data.len()
                );
                return Some(image);
            }
        }
    } else if let Some(result) = eval_result {
        // If idat_recoding is off and reductions were attempted but ended up choosing the baseline,
        // we should still check if the evaluator compressed the baseline smaller than the original.
        let image = PngData {
            raw: result.image,
            idat_data: result.idat_data,
            aux_chunks: Vec::new(),
        };
        if image.estimated_output_size() < max_size.unwrap_or(usize::MAX) {
            debug!("Found better combination:");
            debug!(
                "    zc = {}  f = {:8}  {} bytes",
                eval_compression,
                result.filter,
                image.idat_data.len()
            );
            return Some(image);
        }
    }

    None
}

/// Execute a compression trial
fn perform_trial(
    filtered: &[u8],
    opts: &Options,
    filter: RowFilter,
    best_size: &AtomicMin,
) -> Option<TrialResult> {
    match opts.deflate.deflate(filtered, best_size) {
        Ok(new_idat) => {
            let bytes = new_idat.len();
            best_size.set_min(bytes);
            trace!(
                "    zc = {}  f = {:8}  {} bytes",
                opts.deflate,
                filter,
                bytes
            );
            Some((filter, new_idat))
        }
        Err(PngError::DeflatedDataTooLong(bytes)) => {
            trace!(
                "    zc = {}  f = {:8} >{} bytes",
                opts.deflate,
                filter,
                bytes,
            );
            None
        }
        Err(_) => None,
    }
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
fn report_format(prefix: &str, png: &PngImage) {
    debug!(
        "{}{}-bit {}, {}",
        prefix, png.ihdr.bit_depth, png.ihdr.color_type, png.ihdr.interlaced
    );
}

/// Perform cleanup of certain chunks from the `PngData` object, after optimization has been completed
fn postprocess_chunks(
    png: &mut PngData,
    opts: &Options,
    deadline: Arc<Deadline>,
    orig_ihdr: &IhdrData,
) {
    if let Some(iccp_idx) = png.aux_chunks.iter().position(|c| &c.name == b"iCCP") {
        // See if we can replace an iCCP chunk with an sRGB chunk
        let may_replace_iccp = opts.strip != StripChunks::None && opts.strip.keep(b"sRGB");
        if may_replace_iccp && png.aux_chunks.iter().any(|c| &c.name == b"sRGB") {
            // Files aren't supposed to have both chunks, so we chose to honor sRGB
            trace!("Removing iCCP chunk due to conflict with sRGB chunk");
            png.aux_chunks.remove(iccp_idx);
        } else if let Some(icc) = extract_icc(&png.aux_chunks[iccp_idx]) {
            let intent = if may_replace_iccp {
                srgb_rendering_intent(&icc)
            } else {
                None
            };
            // sRGB-like profile can be replaced with an sRGB chunk with the same rendering intent
            if let Some(intent) = intent {
                trace!("Replacing iCCP chunk with equivalent sRGB chunk");
                png.aux_chunks[iccp_idx] = Chunk {
                    name: *b"sRGB",
                    data: vec![intent],
                };
            } else if opts.idat_recoding {
                // Try recompressing the profile
                if let Ok(iccp) = construct_iccp(&icc, opts.deflate) {
                    let cur_len = png.aux_chunks[iccp_idx].data.len();
                    let new_len = iccp.data.len();
                    if new_len < cur_len {
                        debug!(
                            "Recompressed iCCP chunk: {} ({} bytes decrease)",
                            new_len,
                            cur_len - new_len
                        );
                        png.aux_chunks[iccp_idx] = iccp;
                    }
                }
            }
        }
    }

    // If the depth/color type has changed, some chunks may be invalid and should be dropped
    // While these could potentially be converted, they have no known use case today and are
    // generally more trouble than they're worth
    let ihdr = &png.raw.ihdr;
    if orig_ihdr.bit_depth != ihdr.bit_depth || orig_ihdr.color_type != ihdr.color_type {
        png.aux_chunks.retain(|c| {
            let invalid = &c.name == b"bKGD" || &c.name == b"sBIT" || &c.name == b"hIST";
            if invalid {
                warn!(
                    "Removing {} chunk as it no longer matches the image data",
                    std::str::from_utf8(&c.name).unwrap()
                );
            }
            !invalid
        });
    }

    // Find fdAT chunks and attempt to recompress them
    // Note if there are multiple fdATs per frame then decompression will fail and nothing will change
    let mut fdat: Vec<_> = png
        .aux_chunks
        .iter_mut()
        .filter(|c| &c.name == b"fdAT")
        .collect();
    if opts.idat_recoding && !fdat.is_empty() {
        let buffer_size = orig_ihdr.raw_data_size();
        fdat.par_iter_mut()
            .with_max_len(1)
            .enumerate()
            .for_each(|(i, c)| {
                if deadline.passed() || c.data.len() <= 4 {
                    return;
                }
                if let Ok(mut data) = deflate::inflate(&c.data[4..], buffer_size).and_then(|data| {
                    let max_size = AtomicMin::new(Some(c.data.len() - 5));
                    opts.deflate.deflate(&data, &max_size)
                }) {
                    debug!(
                        "Recompressed fdAT #{:<2}: {} ({} bytes decrease)",
                        i,
                        c.data.len(),
                        c.data.len() - 4 - data.len()
                    );
                    c.data.truncate(4);
                    c.data.append(&mut data);
                }
            })
    }
}

/// Check if an image was already optimized prior to oxipng's operations
fn is_fully_optimized(original_size: usize, optimized_size: usize, opts: &Options) -> bool {
    original_size <= optimized_size && !opts.force
}

#[cfg(not(unix))]
fn copy_permissions(metadata_input: &Metadata, out_file: &File) -> PngResult<()> {
    let readonly_input = metadata_input.permissions().readonly();

    out_file
        .metadata()
        .map_err(|err_io| {
            PngError::new(&format!(
                "unable to read filesystem metadata of output file: {}",
                err_io
            ))
        })
        .and_then(|out_meta| {
            out_meta.permissions().set_readonly(readonly_input);
            out_file
                .metadata()
                .map_err(|err_io| {
                    PngError::new(&format!(
                        "unable to re-read filesystem metadata of output file: {}",
                        err_io
                    ))
                })
                .and_then(|out_meta_reread| {
                    if out_meta_reread.permissions().readonly() != readonly_input {
                        Err(PngError::new(&format!(
                            "failed to set readonly, expected: {}, found: {}",
                            readonly_input,
                            out_meta_reread.permissions().readonly()
                        )))
                    } else {
                        Ok(())
                    }
                })
        })
}

#[cfg(unix)]
fn copy_permissions(metadata_input: &Metadata, out_file: &File) -> PngResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = metadata_input.permissions().mode();

    out_file
        .metadata()
        .map_err(|err_io| {
            PngError::new(&format!(
                "unable to read filesystem metadata of output file: {}",
                err_io
            ))
        })
        .and_then(|out_meta| {
            out_meta.permissions().set_mode(permissions);
            out_file
                .metadata()
                .map_err(|err_io| {
                    PngError::new(&format!(
                        "unable to re-read filesystem metadata of output file: {}",
                        err_io
                    ))
                })
                .and_then(|out_meta_reread| {
                    if out_meta_reread.permissions().mode() != permissions {
                        Err(PngError::new(&format!(
                            "failed to set permissions, expected: {:04o}, found: {:04o}",
                            permissions,
                            out_meta_reread.permissions().mode()
                        )))
                    } else {
                        Ok(())
                    }
                })
        })
}

#[cfg(not(feature = "filetime"))]
fn copy_times(_: &Metadata, _: &Path) -> PngResult<()> {
    Ok(())
}

#[cfg(feature = "filetime")]
fn copy_times(input_path_meta: &Metadata, out_path: &Path) -> PngResult<()> {
    let atime = filetime::FileTime::from_last_access_time(input_path_meta);
    let mtime = filetime::FileTime::from_last_modification_time(input_path_meta);
    trace!(
        "attempting to set file times: atime: {:?}, mtime: {:?}",
        atime,
        mtime
    );
    filetime::set_file_times(out_path, atime, mtime).map_err(|err_io| {
        PngError::new(&format!(
            "unable to set file times on {:?}: {}",
            out_path, err_io
        ))
    })
}
