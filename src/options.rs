use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use indexmap::{IndexSet, indexset};
use log::warn;

use crate::{deflate::Deflaters, filters::RowFilter, headers::StripChunks, interlace::Interlacing};

/// Write destination for [`optimize`][crate::optimize].
/// You can use [`optimize_from_memory`](crate::optimize_from_memory) to avoid external I/O.
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
    #[must_use]
    pub fn from_path(path: PathBuf) -> Self {
        OutFile::Path {
            path: Some(path),
            preserve_attrs: false,
        }
    }

    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match *self {
            Self::Path {
                path: Some(ref p), ..
            } => Some(p.as_path()),
            _ => None,
        }
    }
}

/// Where to read images from in [`optimize`][crate::optimize].
/// You can use [`optimize_from_memory`](crate::optimize_from_memory) to avoid external I/O.
#[derive(Clone, Debug)]
pub enum InFile {
    Path(PathBuf),
    StdIn,
}

impl InFile {
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match *self {
            Self::Path(ref p) => Some(p.as_path()),
            Self::StdIn => None,
        }
    }
}

impl fmt::Display for InFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Path(ref p) => write!(f, "{}", p.display()),
            Self::StdIn => f.write_str("stdin"),
        }
    }
}

impl<T: Into<PathBuf>> From<T> for InFile {
    fn from(s: T) -> Self {
        Self::Path(s.into())
    }
}

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
    /// Which `RowFilters` to try on the file
    ///
    /// Default: `None,Sub,Entropy,Bigrams`
    pub filter: IndexSet<RowFilter>,
    /// Whether to change the interlacing type of the file.
    ///
    /// These are the interlacing types avaliable:
    /// - `None` will not change the current interlacing type.
    /// - `Some(x)` will change the file to interlacing mode `x`.
    ///   See [`Interlacing`] for the possible interlacing types.
    ///
    /// Default: `Some(Interlacing::None)`
    pub interlace: Option<Interlacing>,
    /// Whether to allow transparent pixels to be altered to improve compression.
    ///
    /// Default: `false`
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
    /// Which DEFLATE (zlib) algorithm to use
    #[cfg_attr(feature = "zopfli", doc = "(e.g. Zopfli)")]
    ///
    /// Default: `Libdeflater`
    pub deflate: Deflaters,
    /// Whether to use fast evaluation to pick the best filter
    ///
    /// Default: `true`
    pub fast_evaluation: bool,
    /// Maximum amount of time to spend on optimizations.
    /// Further potential optimizations are skipped if the timeout is exceeded.
    ///
    /// Default: `None`
    pub timeout: Option<Duration>,
}

impl Options {
    #[must_use]
    pub fn from_preset(level: u8) -> Self {
        let opts = Self::default();
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

    #[must_use]
    pub fn max_compression() -> Self {
        Self::from_preset(6)
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
    fn default() -> Self {
        // Default settings based on -o 2 from the CLI interface
        Self {
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
