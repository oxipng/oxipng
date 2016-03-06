extern crate bit_vec;
extern crate byteorder;
extern crate crc;
extern crate crossbeam;
extern crate libc;
extern crate libz_sys;

use std::collections::{HashMap, HashSet};
use std::fs::{File, copy};
use std::io::{BufWriter, Write, stderr, stdout};
use std::path::{Path, PathBuf};

pub mod deflate {
    pub mod deflate;
    pub mod stream;
}
pub mod png;

#[derive(Clone,Debug)]
pub struct Options {
    pub backup: bool,
    pub out_file: PathBuf,
    pub out_dir: Option<PathBuf>,
    pub stdout: bool,
    pub fix_errors: bool,
    pub pretend: bool,
    pub recursive: bool,
    pub clobber: bool,
    pub create: bool,
    pub force: bool,
    pub preserve_attrs: bool,
    pub verbosity: Option<u8>,
    pub filter: HashSet<u8>,
    pub interlace: Option<u8>,
    pub compression: HashSet<u8>,
    pub memory: HashSet<u8>,
    pub strategies: HashSet<u8>,
    pub window: u8,
    pub bit_depth_reduction: bool,
    pub color_type_reduction: bool,
    pub palette_reduction: bool,
    pub idat_recoding: bool,
    pub strip: png::Headers,
    pub use_heuristics: bool,
}

pub fn optimize(filepath: &Path, opts: &Options) -> Result<(), String> {
    // Decode PNG from file
    if opts.verbosity.is_some() {
        writeln!(&mut stderr(), "Processing: {}", filepath.to_str().unwrap()).ok();
    }
    let in_file = Path::new(filepath);
    let mut png = match png::PngData::new(&in_file) {
        Ok(x) => x,
        Err(x) => return Err(x),
    };

    // Print png info
    let idat_original_size = png.idat_data.len();
    let file_original_size = filepath.metadata().unwrap().len() as usize;
    if opts.verbosity.is_some() {
        writeln!(&mut stderr(),
                 "    {}x{} pixels, PNG format",
                 png.ihdr_data.width,
                 png.ihdr_data.height)
            .ok();
        if let Some(palette) = png.palette.clone() {
            writeln!(&mut stderr(),
                     "    {} bits/pixel, {} colors in palette",
                     png.ihdr_data.bit_depth,
                     palette.len() / 3)
                .ok();
        } else {
            writeln!(&mut stderr(),
                     "    {}x{} bits/pixel, {:?}",
                     png.channels_per_pixel(),
                     png.ihdr_data.bit_depth,
                     png.ihdr_data.color_type)
                .ok();
        }
        writeln!(&mut stderr(),
                 "    IDAT size = {} bytes",
                 idat_original_size)
            .ok();
        writeln!(&mut stderr(),
                 "    File size = {} bytes",
                 file_original_size)
            .ok();
    }

    let mut filter = opts.filter.clone();
    let compression = opts.compression.clone();
    let memory = opts.memory.clone();
    let mut strategies = opts.strategies.clone();

    if opts.use_heuristics {
        // Heuristically determine which set of options to use
        if png.ihdr_data.bit_depth.as_u8() >= 8 &&
           png.ihdr_data.color_type != png::ColorType::Indexed {
            if filter.is_empty() {
                filter.insert(5);
            }
            if strategies.is_empty() {
                strategies.insert(1);
            }
        } else {
            if filter.is_empty() {
                filter.insert(0);
            }
            if strategies.is_empty() {
                strategies.insert(0);
            }
        }
    }

    let mut something_changed = false;

    if opts.bit_depth_reduction {
        if png.reduce_bit_depth() {
            something_changed = true;
            if opts.verbosity == Some(1) {
                report_reduction(&png);
            }
        };
    }

    if opts.color_type_reduction {
        if png.reduce_color_type() {
            something_changed = true;
            if opts.verbosity == Some(1) {
                report_reduction(&png);
            }
        };
    }

    if opts.palette_reduction {
        if png.reduce_palette() {
            something_changed = true;
            if opts.verbosity == Some(1) {
                report_reduction(&png);
            }
        };
    }

    if something_changed && opts.verbosity.is_some() {
        report_reduction(&png);
    }

    if let Some(interlacing) = opts.interlace {
        if png.change_interlacing(interlacing) {
            something_changed = true;
            if opts.verbosity == Some(1) {
                report_reduction(&png);
            }
        }
    }

    if opts.idat_recoding || something_changed {
        // Go through selected permutations and determine the best
        let mut best: Option<(u8, u8, u8, u8, Vec<u8>)> = None;
        let combinations = filter.len() * compression.len() * memory.len() * strategies.len();
        let mut results = Vec::with_capacity(combinations);
        if opts.verbosity.is_some() {
            writeln!(&mut stderr(), "Trying: {} combinations", combinations).ok();
        }
        crossbeam::scope(|scope| {
            for f in &filter {
                let filtered = png.filter_image(*f);
                for zc in &compression {
                    for zm in &memory {
                        for zs in &strategies {
                            let moved_filtered = filtered.clone();
                            results.push(scope.spawn(move || {
                                let new_idat = match deflate::deflate::deflate(&moved_filtered,
                                                                               *zc,
                                                                               *zm,
                                                                               *zs,
                                                                               opts.window) {
                                    Ok(x) => x,
                                    Err(x) => return Err(x),
                                };

                                if opts.verbosity == Some(1) {
                                    writeln!(&mut stderr(), "    zc = {}  zm = {}  zs = {}  f = {}        {} bytes",
                                             *zc,
                                             *zm,
                                             *zs,
                                             *f,
                                             new_idat.len()).ok();
                                }

                                Ok((*f, *zc, *zm, *zs, new_idat.clone()))
                            }));
                        }
                    }
                }
            }
        });

        for result in results {
            if let Ok(ok_result) = result.join() {
                if (best.is_some() &&
                    ok_result.4.len() < best.as_ref().map(|x| x.4.len()).unwrap()) ||
                   (best.is_none() &&
                    (ok_result.4.len() < png.idat_data.len() ||
                     (opts.interlace.is_some() &&
                      opts.interlace != Some(png.ihdr_data.interlaced)) ||
                     opts.force)) {
                    best = Some(ok_result);
                }
            }
        }

        if let Some(better) = best {
            png.idat_data = better.4.clone();
            if opts.verbosity.is_some() {
                writeln!(&mut stderr(), "Found better combination:").ok();
                writeln!(&mut stderr(),
                         "    zc = {}  zm = {}  zs = {}  f = {}        {} bytes",
                         better.1,
                         better.2,
                         better.3,
                         better.0,
                         png.idat_data.len())
                    .ok();
            }
        }
    }

    match opts.strip.clone() {
        // Strip headers
        png::Headers::None => (),
        png::Headers::Some(hdrs) => {
            for hdr in &hdrs {
                png.aux_headers.remove(hdr);
            }
        }
        png::Headers::Safe => {
            const PRESERVED_HEADERS: [&'static str; 9] = ["cHRM", "gAMA", "iCCP", "sBIT", "sRGB",
                                                          "bKGD", "hIST", "pHYs", "sPLT"];
            let mut preserved = HashMap::new();
            for (hdr, contents) in png.aux_headers.iter() {
                if PRESERVED_HEADERS.contains(&hdr.as_ref()) {
                    preserved.insert(hdr.clone(), contents.clone());
                }
            }
            png.aux_headers = preserved;
        }
        png::Headers::All => {
            png.aux_headers = HashMap::new();
        }
    }

    let output_data = png.output();
    if file_original_size <= output_data.len() && !opts.force && opts.interlace.is_none() {
        writeln!(&mut stderr(), "File already optimized").ok();
        return Ok(());
    }

    if opts.pretend {
        writeln!(&mut stderr(), "Running in pretend mode, no output").ok();
    } else {
        if opts.backup {
            match copy(in_file,
                       in_file.with_extension(format!("bak.{}",
                                                      in_file.extension()
                                                             .unwrap()
                                                             .to_str()
                                                             .unwrap()))) {
                Ok(x) => x,
                Err(_) => {
                    return Err(format!("Unable to write to backup file at {}",
                                       opts.out_file.display()))
                }
            };
        }

        if opts.stdout {
            let mut buffer = BufWriter::new(stdout());
            match buffer.write_all(&output_data) {
                Ok(_) => (),
                Err(_) => return Err("Unable to write to stdout".to_owned()),
            }
        } else {
            let out_file = match File::create(&opts.out_file) {
                Ok(x) => x,
                Err(_) => {
                    return Err(format!("Unable to write to file {}", opts.out_file.display()))
                }
            };
            let mut buffer = BufWriter::new(out_file);
            match buffer.write_all(&output_data) {
                Ok(_) => {
                    if opts.verbosity.is_some() {
                        writeln!(&mut stderr(), "Output: {}", opts.out_file.display()).ok();
                    }
                }
                Err(_) => {
                    return Err(format!("Unable to write to file {}", opts.out_file.display()))
                }
            }
        }
    }
    if opts.verbosity.is_some() {
        if idat_original_size >= png.idat_data.len() {
            writeln!(&mut stderr(),
                     "    IDAT size = {} bytes ({} bytes decrease)",
                     png.idat_data.len(),
                     idat_original_size - png.idat_data.len())
                .ok();
        } else {
            writeln!(&mut stderr(),
                     "    IDAT size = {} bytes ({} bytes increase)",
                     png.idat_data.len(),
                     png.idat_data.len() - idat_original_size)
                .ok();
        }
        if file_original_size >= output_data.len() {
            writeln!(&mut stderr(),
                     "    file size = {} bytes ({} bytes = {:.2}% decrease)",
                     output_data.len(),
                     file_original_size - output_data.len(),
                     (file_original_size - output_data.len()) as f64 / file_original_size as f64 *
                     100f64)
                .ok();
        } else {
            writeln!(&mut stderr(),
                     "    file size = {} bytes ({} bytes = {:.2}% increase)",
                     output_data.len(),
                     output_data.len() - file_original_size,
                     (output_data.len() - file_original_size) as f64 / file_original_size as f64 *
                     100f64)
                .ok();
        }
    }
    Ok(())
}

fn report_reduction(png: &png::PngData) {
    if let Some(palette) = png.palette.clone() {
        writeln!(&mut stderr(),
                 "Reducing image to {} bits/pixel, {} colors in palette",
                 png.ihdr_data.bit_depth,
                 palette.len() / 3)
            .ok();
    } else {
        writeln!(&mut stderr(),
                 "Reducing image to {}x{} bits/pixel, {}",
                 png.channels_per_pixel(),
                 png.ihdr_data.bit_depth,
                 png.ihdr_data.color_type)
            .ok();
    }
}
