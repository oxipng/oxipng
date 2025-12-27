#[cfg(not(feature = "parallel"))]
mod rayon;

#[cfg(feature = "zopfli")]
use std::num::NonZeroU64;
use std::{
    ffi::{OsStr, OsString},
    fs::DirBuilder,
    io::{IsTerminal, Write, stdout},
    path::PathBuf,
    process::ExitCode,
    sync::atomic::{AtomicUsize, Ordering::AcqRel},
    time::Duration,
};

use clap::ArgMatches;
mod cli;
use indexmap::IndexSet;
use log::{Level, LevelFilter, error, warn};
#[cfg(feature = "zopfli")]
use oxipng::ZopfliOptions;
use oxipng::{
    Deflater, FilterStrategy, InFile, OptimizationResult, Options, OutFile, PngError, StripChunks,
};
use rayon::prelude::*;

use crate::cli::DISPLAY_CHUNKS;

fn main() -> ExitCode {
    let matches = cli::build_command()
        // Set the value parser for filters which isn't appropriate to do in the build_command function
        .mut_arg("filters", |arg| {
            arg.value_parser(|x: &str| {
                parse_numeric_range_opts(x, 0, 9).map_err(|_| "Invalid option for filters")
            })
        })
        .after_help("Run `oxipng --help` to see full details of all options")
        .after_long_help("")
        .get_matches_from(std::env::args());

    let (mut out_file, out_dir, opts) = match parse_opts_into_struct(&matches) {
        Ok(x) => x,
        Err(x) => {
            error!("{x}");
            return ExitCode::FAILURE;
        }
    };

    // Determine input and output
    let file_args = matches.get_many::<PathBuf>("files").unwrap().cloned();
    #[cfg(windows)]
    let inputs: Vec<_> = file_args.flat_map(apply_glob_pattern).collect();
    #[cfg(not(windows))]
    let inputs: Vec<_> = file_args.collect();
    let using_stdin = inputs.len() == 1 && inputs[0].to_str() == Some("-");
    if using_stdin && out_dir.is_some() {
        error!("Cannot use --dir when reading from stdin.");
        return ExitCode::FAILURE;
    }
    if using_stdin && matches!(out_file, OutFile::Path { path: None, .. }) {
        out_file = OutFile::StdOut;
    }
    let using_stdout = matches!(out_file, OutFile::StdOut);
    let json = matches.get_flag("json");
    if using_stdout && json {
        error!("Cannot use --json when writing to stdout.");
        return ExitCode::FAILURE;
    }

    let files = if using_stdin {
        vec![(InFile::StdIn, out_file)]
    } else {
        collect_files(
            inputs,
            &out_dir,
            &out_file,
            matches.get_flag("recursive"),
            true,
        )
    };

    let is_verbose = matches.get_count("verbose") > 0;
    let print_summary = !matches.get_flag("quiet") && !using_stdout;
    let print_progress = print_summary && !is_verbose && stdout().is_terminal();
    let total_files = files.len();
    let num_processed = AtomicUsize::new(0);
    if print_progress {
        print!("Files processed: 0/{}...", total_files);
        stdout().flush().ok();
    }
    let process = |(input, output): &(InFile, OutFile)| {
        let result = process_file(input, output, &opts);
        if print_progress && matches!(result, OptimizationResult::Ok(_)) {
            let value = num_processed.fetch_add(1, AcqRel) + 1;
            print!("\rFiles processed: {}/{}...", value, total_files);
            stdout().flush().ok();
        }
        result
    };
    let results: Vec<OptimizationResult> = if matches.get_flag("parallel-files") {
        files.par_iter().map(process).collect()
    } else {
        files.iter().map(process).collect()
    };

    // Collect stats
    let mut num_succeeded = 0;
    let mut num_not_optimized = 0;
    let mut num_failed = 0;
    let mut total_in: i64 = 0;
    let mut total_out: i64 = 0;
    for result in &results {
        match result {
            Ok((insize, outsize)) => {
                num_succeeded += 1;
                total_in += *insize as i64;
                total_out += *outsize as i64;
                if !opts.force && insize == outsize {
                    num_not_optimized += 1;
                }
            }
            Err(PngError::C2PAMetadataPreventsChanges | PngError::InflatedDataTooLong(_)) => {}
            Err(_) => num_failed += 1,
        }
    }

    // Print results
    if json {
        json_output(&files, &results);
    } else if print_summary {
        let in_bytes = format_bytes(total_in, true);
        let out_bytes = format_bytes(total_out, true);
        let saved = total_in - total_out;
        let saved_bytes = format_bytes(saved, false);
        let percent = if total_in > 0 {
            saved as f64 / total_in as f64 * 100_f64
        } else {
            0_f64
        };
        if is_verbose {
            println!("--------------------");
        }
        println!("\rFiles processed: {num_succeeded}/{total_files}   ");
        println!("Input size: {}", in_bytes);
        println!("Output size: {}", out_bytes);
        println!("Total saved: {} ({:.2}%)", saved_bytes, percent);
        if num_not_optimized == 1 {
            println!("({num_not_optimized} file could not be optimized further)");
        } else if num_not_optimized > 0 {
            println!("({num_not_optimized} files could not be optimized further)");
        }
        if matches.get_flag("dry-run") {
            println!("Dry run, no changes saved");
        }
    }

    // For optimizing single files, this will return the correct exit code always.
    // For recursive optimization, the correct choice is a bit subjective.
    // We're choosing to return a 0 exit code if ANY file in the set
    // runs correctly.
    // The reason for this is that recursion may pick up files that are not
    // PNG files, and return an error for them.
    // We don't really want to return an error code for those files.
    if num_succeeded > 0 {
        ExitCode::SUCCESS
    } else if num_failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::from(3)
    }
}

fn collect_files(
    files: Vec<PathBuf>,
    out_dir: &Option<PathBuf>,
    out_file: &OutFile,
    recursive: bool,
    top_level: bool, //explicitly specify files
) -> Vec<(InFile, OutFile)> {
    let mut in_out_pairs = Vec::new();
    for input in files {
        if input.is_dir() {
            if recursive {
                match input.read_dir() {
                    Ok(dir) => {
                        let files = dir.filter_map(|x| x.ok().map(|x| x.path())).collect();
                        in_out_pairs
                            .extend(collect_files(files, out_dir, out_file, recursive, false));
                    }
                    Err(e) => {
                        warn!("{}: {}", input.display(), e);
                    }
                }
            } else {
                warn!("{} is a directory, skipping", input.display());
            }
            continue;
        }

        // Skip non png files if not given on top level
        if !top_level && {
            let extension = input.extension().map(OsStr::to_ascii_lowercase);
            extension != Some(OsString::from("png")) && extension != Some(OsString::from("apng"))
        } {
            continue;
        }

        let out_file =
            if let (Some(out_dir), &OutFile::Path { preserve_attrs, .. }) = (out_dir, out_file) {
                let path = Some(out_dir.join(input.file_name().unwrap()));
                OutFile::Path {
                    path,
                    preserve_attrs,
                }
            } else {
                (*out_file).clone()
            };
        let in_file = InFile::Path(input);
        in_out_pairs.push((in_file, out_file));
    }
    in_out_pairs
}

#[cfg(windows)]
fn apply_glob_pattern(path: PathBuf) -> Vec<PathBuf> {
    let matches = path
        .to_str()
        // Use MatchOptions::default() to disable case-sensitivity
        .and_then(|pattern| glob::glob_with(pattern, glob::MatchOptions::default()).ok())
        .map(|paths| paths.flatten().collect::<Vec<_>>());

    match matches {
        Some(paths) if !paths.is_empty() => paths,
        _ => vec![path],
    }
}

fn parse_opts_into_struct(
    matches: &ArgMatches,
) -> Result<(OutFile, Option<PathBuf>, Options), String> {
    let log_level = match matches.get_count("verbose") {
        _ if matches.get_flag("quiet") => LevelFilter::Off,
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    env_logger::builder()
        .filter_module(module_path!(), log_level)
        .format(|buf, record| {
            match record.level() {
                Level::Error | Level::Warn => {
                    let style = buf.default_level_style(record.level());
                    // Prepend carriage return to clear progress line
                    writeln!(buf, "\r{style}{}{style:#}", record.args())
                }
                // Leave info, debug and trace unstyled
                _ => writeln!(buf, "{}", record.args()),
            }
        })
        .init();

    let mut opts = match matches.get_one::<String>("optimization") {
        None => Options::default(),
        Some(x) if x == "max" => Options::max_compression(),
        Some(level) => Options::from_preset(level.parse::<u8>().unwrap()),
    };

    // Get custom brute settings and rebuild the filter set to apply them
    let mut brute_lines = matches.get_one::<usize>("brute-lines").copied();
    let mut brute_level = matches.get_one::<i64>("brute-level").map(|x| *x as u8);
    let mut new_filters = IndexSet::new();
    for mut f in opts.filters.drain(..) {
        if let FilterStrategy::Brute { num_lines, level } = &mut f {
            *num_lines = brute_lines.unwrap_or(*num_lines);
            *level = brute_level.unwrap_or(*level);
            // If custom settings were not given, we still need to retain the default values
            // from the preset so we can re-apply them if the filters are overridden below
            brute_lines = Some(*num_lines);
            brute_level = Some(*level);
        }
        new_filters.insert(f);
    }
    opts.filters = new_filters;

    if let Some(x) = matches.get_one::<IndexSet<u8>>("filters") {
        opts.filters = x
            .iter()
            .map(|&f| match f {
                0..=4 => FilterStrategy::Basic(f.try_into().unwrap()),
                5 => FilterStrategy::MinSum,
                6 => FilterStrategy::Entropy,
                7 => FilterStrategy::Bigrams,
                8 => FilterStrategy::BigEnt,
                9 => FilterStrategy::Brute {
                    num_lines: brute_lines.unwrap_or(3),
                    level: brute_level.unwrap_or(1),
                },
                _ => unreachable!(),
            })
            .collect();
    }

    if let Some(&num) = matches.get_one::<u64>("timeout") {
        opts.timeout = Some(Duration::from_secs(num));
    }

    let out_dir = if let Some(path) = matches.get_one::<PathBuf>("output_dir") {
        if !path.exists() {
            match DirBuilder::new().recursive(true).create(path) {
                Ok(()) => (),
                Err(x) => return Err(format!("Could not create output directory {x}")),
            }
        } else if !path.is_dir() {
            return Err(format!(
                "{} is an existing file (not a directory), cannot create directory",
                path.display()
            ));
        }
        Some(path.to_owned())
    } else {
        None
    };

    let out_file = if matches.get_flag("dry-run") {
        OutFile::None
    } else if matches.get_flag("stdout") {
        OutFile::StdOut
    } else {
        OutFile::Path {
            path: matches.get_one::<PathBuf>("output_file").cloned(),
            preserve_attrs: matches.get_flag("preserve"),
        }
    };

    opts.optimize_alpha = matches.get_flag("alpha");

    opts.scale_16 = matches.get_flag("scale16");

    // The default value for fast depends on the preset - make sure we don't change when not provided
    if matches.get_flag("fast") {
        opts.fast_evaluation = matches.get_flag("fast");
    }

    opts.force = matches.get_flag("force");

    opts.fix_errors = matches.get_flag("fix");

    opts.max_decompressed_size = matches.get_one::<u64>("max-size").map(|&x| x as usize);

    opts.bit_depth_reduction = !matches.get_flag("no-bit-reduction");

    opts.color_type_reduction = !matches.get_flag("no-color-reduction");

    opts.palette_reduction = !matches.get_flag("no-palette-reduction");

    opts.grayscale_reduction = !matches.get_flag("no-grayscale-reduction");

    if matches.get_flag("no-reductions") {
        opts.bit_depth_reduction = false;
        opts.color_type_reduction = false;
        opts.palette_reduction = false;
        opts.grayscale_reduction = false;
        opts.interlace = None;
    }

    opts.idat_recoding = !matches.get_flag("no-recoding");

    if let Some(x) = matches.get_one::<String>("interlace") {
        opts.interlace = match x.as_str() {
            "off" | "0" => Some(false),
            "on" | "1" => Some(true),
            _ => None, // keep
        };
    }

    if let Some(keep) = matches.get_one::<String>("keep") {
        let mut keep_display = false;
        let mut names = keep
            .split(',')
            .filter_map(|name| {
                if name == "display" {
                    keep_display = true;
                    return None;
                }
                Some(parse_chunk_name(name))
            })
            .collect::<Result<IndexSet<_>, _>>()?;
        if keep_display {
            names.extend(DISPLAY_CHUNKS.iter().copied());
        }
        opts.strip = StripChunks::Keep(names);
    }

    if let Some(strip) = matches.get_one::<String>("strip") {
        if strip == "safe" {
            opts.strip = StripChunks::Safe;
        } else if strip == "all" {
            opts.strip = StripChunks::All;
        } else {
            const FORBIDDEN_CHUNKS: [[u8; 4]; 5] =
                [*b"IHDR", *b"IDAT", *b"tRNS", *b"PLTE", *b"IEND"];
            let names = strip
                .split(',')
                .map(|x| {
                    if x == "safe" || x == "all" {
                        return Err(
                            "'safe' or 'all' presets for --strip should be used by themselves"
                                .to_owned(),
                        );
                    }
                    let name = parse_chunk_name(x)?;
                    if FORBIDDEN_CHUNKS.contains(&name) {
                        return Err(format!("{x} chunk is not allowed to be stripped"));
                    }
                    Ok(name)
                })
                .collect::<Result<_, _>>()?;
            opts.strip = StripChunks::Strip(names);
        }
    }

    if matches.get_flag("strip-safe") {
        opts.strip = StripChunks::Safe;
    }

    #[cfg(feature = "zopfli")]
    if matches.get_flag("zopfli") {
        let iteration_count = *matches.get_one::<NonZeroU64>("iterations").unwrap();
        let iterations_without_improvement = *matches
            .get_one::<NonZeroU64>("iterations-without-improvement")
            .unwrap_or(&NonZeroU64::MAX);
        opts.deflater = Deflater::Zopfli(ZopfliOptions {
            iteration_count,
            iterations_without_improvement,
            ..Default::default()
        });
    }
    if let (Deflater::Libdeflater { compression }, Some(x)) =
        (&mut opts.deflater, matches.get_one::<i64>("compression"))
    {
        *compression = *x as u8;
    }

    #[cfg(feature = "parallel")]
    if let Some(&threads) = matches.get_one::<usize>("threads") {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .map_err(|err| err.to_string())?;
    }

    Ok((out_file, out_dir, opts))
}

fn parse_chunk_name(name: &str) -> Result<[u8; 4], String> {
    name.trim()
        .as_bytes()
        .try_into()
        .map_err(|_| format!("Invalid chunk name {name}"))
}

fn parse_numeric_range_opts(
    input: &str,
    min_value: u8,
    max_value: u8,
) -> Result<IndexSet<u8>, String> {
    const ERROR_MESSAGE: &str = "Not a valid input";
    let mut items = IndexSet::new();

    // one value
    if let Ok(one_value) = input.parse::<u8>() {
        if (min_value <= one_value) && (one_value <= max_value) {
            items.insert(one_value);
            return Ok(items);
        }
    }

    // a range ("A-B")
    let range_values = input.split('-').collect::<Vec<&str>>();
    if range_values.len() == 2 {
        let first_opt = range_values[0].parse::<u8>();
        let second_opt = range_values[1].parse::<u8>();
        if let (Ok(first), Ok(second)) = (first_opt, second_opt) {
            if min_value <= first && first < second && second <= max_value {
                for i in first..=second {
                    items.insert(i);
                }
                return Ok(items);
            }
        }
        return Err(ERROR_MESSAGE.to_owned());
    }

    // a list ("A,B[,…]")
    let list_items = input.split(',').collect::<Vec<&str>>();
    if list_items.len() > 1 {
        for value in list_items {
            if let Ok(value_int) = value.parse::<u8>() {
                if (min_value <= value_int)
                    && (value_int <= max_value)
                    && !items.contains(&value_int)
                {
                    items.insert(value_int);
                    continue;
                }
            }
            return Err(ERROR_MESSAGE.to_owned());
        }
        return Ok(items);
    }

    Err(ERROR_MESSAGE.to_owned())
}

fn process_file(input: &InFile, output: &OutFile, opts: &Options) -> OptimizationResult {
    if let (Some(max_size), InFile::Path(path)) = (opts.max_decompressed_size, input) {
        if path.metadata().is_ok_and(|m| m.len() > max_size as u64) {
            warn!("{input}: Skipped: File exceeds the maximum size ({max_size} bytes)");
            return Err(PngError::InflatedDataTooLong(max_size));
        }
    }

    let result = oxipng::optimize(input, output, opts);
    match &result {
        Ok(_) => {}
        Err(e @ PngError::C2PAMetadataPreventsChanges | e @ PngError::InflatedDataTooLong(_)) => {
            warn!("{input}: Skipped: {e}");
        }
        Err(e) => {
            error!("{input}: {e}");
        }
    }
    result
}

/// Write optimization results as json.
/// ```
/// {
///   "results": [
///     {
///       "input": string,
///       "status": "success",
///       "output": string|null,
///       "insize": number,
///       "outsize": number
///     },
///     {
///       "input": string,
///       "status": "error",
///       "error": string
///     }
///   ]
/// }
/// ```
fn json_output(files: &[(InFile, OutFile)], results: &[OptimizationResult]) {
    print!(r#"{{"results":["#);
    let mut first = true;
    results
        .iter()
        .zip(files)
        .for_each(|(result, (input, output))| {
            if !first {
                print!(",");
            }
            print!(r#"{{"input":"{}","#, json_escape(&input.to_string()));
            match result {
                Ok((insize, outsize)) => {
                    let outpath = match output {
                        OutFile::None => "null".to_owned(),
                        OutFile::Path { path: None, .. } => {
                            format!(r#""{}""#, json_escape(&input.to_string()))
                        }
                        OutFile::Path { path: Some(p), .. } => {
                            format!(r#""{}""#, json_escape(&p.display().to_string()))
                        }
                        OutFile::StdOut => unreachable!(),
                    };
                    print!(
                        r#""status":"success","output":{},"insize":{},"outsize":{}}}"#,
                        outpath, insize, outsize
                    );
                }
                Err(e) => {
                    print!(
                        r#""status":"error","error":"{}"}}"#,
                        json_escape(&e.to_string())
                    );
                }
            }
            first = false;
        });
    print!("]}}");
}

fn json_escape(string: &str) -> String {
    string
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
        .replace("\x08", "\\b")
        .replace("\x0c", "\\f")
}

/// Format byte counts as IEC units to 3 significant figures.
fn format_bytes(count: i64, include_raw: bool) -> String {
    const K: i64 = 1 << 10;
    const M: i64 = 1 << 20;
    const G: i64 = 1 << 30;
    fn format_3sf(value: f64) -> String {
        match value.abs() {
            ..9.995 => format!("{:.2}", value),
            9.995..99.95 => format!("{:.1}", value),
            _ => format!("{:.0}", value),
        }
    }
    let formatted = match count.abs() {
        ..K => format!("{} bytes", count),
        K..M => format!("{} KiB", format_3sf(count as f64 / K as f64)),
        M..G => format!("{} MiB", format_3sf(count as f64 / M as f64)),
        _ => format!("{} GiB", format_3sf(count as f64 / G as f64)),
    };
    if include_raw && count.abs() >= K {
        format!("{} ({} bytes)", formatted, count)
    } else {
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1023, false), "1023 bytes");
        assert_eq!(format_bytes(800_000, false), "781 KiB");
        assert_eq!(format_bytes(12_500_000, false), "11.9 MiB");
        assert_eq!(format_bytes(2_000_000_000, false), "1.86 GiB");
        assert_eq!(format_bytes(-1024, false), "-1.00 KiB");
    }
}
