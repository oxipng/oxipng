## Version 10.0.0

- [Breaking] CLI: Change short zopfli flag from `-Z` to `-z`.
- [Breaking] CLI: Change `--pretend`/`-P` to `--dry-run`/`-D`.
- [Breaking] CLI: Change `--interlace` options from `1`/`0` to `on`/`off`.
- [Breaking] API: Change `Options.interlace: Option<Interlacing>` to `Options.interlace: Option<bool>`.
- [Breaking] API: Change `Options.filter: IndexSet<RowFilter>` to `Options.filters: IndexSet<FilterStrategy>`.
- [Breaking] API: Change `Options.deflate: Deflaters` to `Options.deflater: Deflater`.
- [Breaking] API: Change `Deflaters::Zopfli { .. }` to `Deflater::Zopfli(ZopfliOptions { .. })`.
- [Breaking] API: Change `optimize()` to return `(usize, usize)`, with original and optimized sizes.
- [Feature] Allow zopfli iterations higher than 255.
- [Feature] Add `--ziwi` option for zopfli iterations without improvement.
- [Feature] Add `--max-raw-size` option to skip images that are too large.
- [Feature] Add `--brute-level` and `--brute-lines` options for advanced control of Brute filter.
- [Improvement] Reduce memory usage for `fast` mode.
- [Improvement] Slightly improve compression with Brute filter at levels 5 and 6.
- [Misc] Change `--preserve` option to no longer preserve last access time.
- [Misc] Make help output colored.

## Version 9.1.5

- [Feature] Add `--sequential` option to process files sequentially rather than in parallel.
- [Performance] Update to latest Zopfli with greatly improved performance.
- [Improvement] Reduce memory usage.
- [Bugfix] Correct handling of grayscale conversion when ICC profile is present.

## Version 9.1.4

- [Improvement] Improve optimization of APNG files (reductions still not supported yet).
- [Improvement] Improve reductions for small images and ensure consistent results for repeat runs.
- [Build] Add feature `system-libdeflate` to use the system-installed version of libdeflate.
- [Misc] Strip C2PA metadata by default.

## Version 9.1.3

- [Feature] Add `--zi` option to control the number of Zopfli iterations.
- [Improvement] Allow setting compression level to 0.
- [Performance] Improve filtering performance for some images.
- [Build] Move man page generation to an xtask.

## Version 9.1.2

- [Bugfix] Fix `--nx` still applying deinterlacing by default.
- [Bugfix] Fix wildcard matching being case-sensitive on Windows.
- [Bugfix] Fix optimized APNGs not being compatible with some programs.
- [Build] Fix feature `sanity-checks` not working without `parallel`.
- [Misc] Resolve ambiguity between optional dependencies and crate features.

## Version 9.1.1

- [Build] Change man page generation path to resolve issue with cargo publish.

## Version 9.1.0

- [Improvement] Add `--keep display` equivalent to `--strip safe`.
- [Improvement] Add modified zeng palette sorting method, improving optimization of indexed images.
- [Improvement] If only one filter is specified, guarantee to only use this one.
- [Improvement] Evaluate low-depth indexed even if low-depth grayscale was already achieved.
- [Bugfix] Fix battiato palette sorting method not being used if the input was not already indexed.
- [Bugfix] Fix rare crash caused by a truncated palette.
- [Build] Reduce size of binaries.
- [Build] Add man page generation.
- [Build] Publish deb archives for Linux.
- [Misc] Bump minimum Rust version to 1.74.0.

## Version 9.0.0

- [Breaking] Remove `--backup` option. Use `--out` or `--dir` to preserve existing files.
- [Breaking] Remove `--check` option. Use `--nx --nz` to perform a non-optimizing run.
- [Breaking] API: Replace `pretend` option with `OutFile::None`.
- [Breaking] API: Move `preserve_attrs` into `OutFile::Path`.
- [Breaking] Default to removing interlacing. Use `-i keep` to retain existing interlacing.
- [Feature] Add Raw API for creating optimised PNGs from raw image data.
- [Feature] Add basic support for APNG files.
- [Feature] Add `--scale16` option to forcibly reduce 16-bit images to 8-bit.
- [Improvement] Process multiple files in parallel.
- [Improvement] Improve reductions, particularly for indexed or very small images.
- [Improvement] Improve compression with latest version of libdeflate.
- [Improvement] Recompress iCCP chunks.
- [Improvement] Change recursive mode to only process .png/.apng files.
- [Improvement] Add support for glob patterns in quotes on Windows.
- [Improvement] Quieter default output logging, with multiple levels of verbosity.
- [Bugfix] Fix deadlock when using oxipng within an existing Rayon thread pool.
- [Bugfix] Fix early abort in recursive mode when a read error occurred.
- [Bugfix] Fix losing aux chunks when there's more than one of the same type.
- [Bugfix] Fix sometimes writing output even when it was larger.
- [Misc] Revamp CI workflow to upload artifacts and generate binaries for additional architectures.
- [Misc] Bump minimum Rust version to 1.66.0.

## Version 8.0.0

- [Breaking] Revamp alpha optimization
- [Bugfix] Fix grayscale depth reduction with tRNS pixel
- [Bugfix] Fix fast mode with zopfli
- [Improvement] Tweaks to interlacing and format display
- [Improvement] Ability to reduce alpha channel to tRNS pixel
- [Improvement] Performance improvements to colorspace reduction
- [Misc] Disable image validation in release mode, as it is deemed no longer necessary

## Version 7.0.0

- [Breaking] Switch to libdeflater as the default
- [Breaking] Rebalance presets
- [Breaking] Update safe headers to remove
- [Breaking] Bump minimum Rust version to 1.61.0
- [Feature] Introduce new filter strategies
- [Feature] Allow setting libdeflate compression level
- [Feature] Add optional dockerization
- [Improvement] Improve alpha reductions
- [Improvement] Fast deinterlace for 8-bit
- [Improvement] Refactor heuristics for fast filter evaluation
- [Improvement] Improve file size reduction further
- [Bugfix] Avoid printing output bytes on decompress error
- [Bugfix] Fix compression level 0
- [Bugfix] Fix directional alpha reductions
- [Bugfix] Fix edge cases in palette reduction

### Version 6.0.1

- [Bugfix] Fix an issue where Zopfli mode could generate corrupt images

### Version 6.0.0

- [Breaking] Bump minimum Rust version to 1.57.0
- [Feature] Add `--check`/`-c` CLI option
- [Security] Update stderrlog to 0.5.2 (Fixes RUSTSEC-2022-0006)
- [Security] Remove chrono as a transitive dependency (Fixes RUSTSEC-2020-0159)
- [Misc] Bump `clap` to 3.2
- [Misc] Bump `zopfli` to 0.7
- [Misc] Bump `libdeflater` to 0.10
- [Misc] Remove `byteorder` dependency in favor of stdlib functions
- [Misc] Bump `image` to 0.24
- [Misc] Bump `crc` to 3.0
- [Misc] Bump `miniz_oxide` to 0.6
- [Misc] Update to Rust edition 2021
- [Misc] Various internal improvements

### Version 5.0.1

- [Bugfix] Fix an issue where -o5 and -o6 were not testing all options
- [Misc] Bump miniz_oxide to 0.5

### Version 5.0.0

- [Breaking] Bump minimum Rust version to 1.46.0
- [Bugfix] Avoid many panics that may occur on broken files
- [Bugfix] Fix race condition that may cause tests to fail
- [Performance] Optimizations to unfiltering and copying
- [Feature] Add `--ng` option to skip grayscale reduction
- [Feature] Also preserve timestamps when using `--preserve`
- [Misc] Bump `crc` to 2.x

### Version 4.0.3

- Bump itertools to 0.10.x
- Temporarily disable i686 releases, which were failing due to an odd linker issue,
  so that at least amd64 builds will publish successfully
  - This only relates to the releases published on Github. You can still manually compile oxipng for any platform.

### Version 4.0.2

- Includes another update to libdeflater that improves support for targets
  without a C stdlib, like wasm32.

### Version 4.0.1

- Includes an update to libdeflater that improves support for targets
  without a C stdlib, like wasm32.

### Version 4.0.0

- [Breaking] Bump minimum Rust version to 1.45.0
- [Feature] Make `libdeflater` and `zopfli` optional for API users
- [Bugfix] Fix cloudflare-zlib on aarch64 CPUs
- [Bugfix] Don't exit on finding a non-PNG file when crawling recursively
- [Bugfix] Make `rayon` truly optional
- Various internal improvements

### Version 3.0.1

- [Bugfix] Re-add `--force` flag to CLI
  - This was accidentally removed somehow
- Many non-breaking dependency version bumps

### Version 3.0.0

- [Breaking] Bump minimum Rust version to 1.41.0
- [Breaking] Use IndexMap/IndexSet to provide more consistent performance ([#202](https://github.com/oxipng/oxipng/pull/202))
  - This changes some public-facing types.
    `IndexMap` and `IndexSet` are reexported
    at the crate root to aid migration.
- [Breaking] Remove fields from the `Options` struct which were never used ([#211](https://github.com/oxipng/oxipng/pull/211/files#diff-b4aea3e418ccdb71239b96952d9cddb6L217), [#212](https://github.com/oxipng/oxipng/pull/212/files#diff-b4aea3e418ccdb71239b96952d9cddb6L134))
- [Breaking] Refactor zlib-specific options in the `Options` struct ([#210](https://github.com/oxipng/oxipng/pull/210/files))
- [Feature] Add libdeflater as an option ([#203](https://github.com/oxipng/oxipng/pull/203))
- [Feature] Use standard `log` library ([#218](https://github.com/oxipng/oxipng/pull/218))
- [Feature] Add `-o max` setting which will always reference the highest compression preset ([#224](https://github.com/oxipng/oxipng/pull/224))
- [Deprecated] `-o 4` was found to be equivalent to `-o 3` and is deprecated.
  It will likely be removed in a future release.
  For now it remains equivalent to `-o 3`. ([#224](https://github.com/oxipng/oxipng/pull/224))
- [Bugfix] Ensure output is deterministic ([#199](https://github.com/oxipng/oxipng/pull/199))
- Update `image` crate to 0.23
- Update `itertools` crate to 0.9
- Various performance and internal improvements

### Version 2.3.0

- Allow disabling all alpha optimizations ([#181](https://github.com/oxipng/oxipng/pull/181))
- Fix interlacing issues on tiny images ([#182](https://github.com/oxipng/oxipng/pull/182))
- Reduce memory usage in filtering ([#191](https://github.com/oxipng/oxipng/pull/191))
- Implement palette sorting to improve compression ([#193](https://github.com/oxipng/oxipng/pull/193))
- Disable alpha optimizations by default ([#187](https://github.com/oxipng/oxipng/pull/187))
- Add support for WASM ([#194](https://github.com/oxipng/oxipng/pull/194))

### Version 2.2.2

- Fix grayscale bit-depth reduction ([#171](https://github.com/oxipng/oxipng/pull/171))
- Fix typos and incorrect log message ([#172](https://github.com/oxipng/oxipng/pull/172))
- Make metadata order deterministic ([#174](https://github.com/oxipng/oxipng/pull/174))
- Fix 32-bit builds ([#176](https://github.com/oxipng/oxipng/pull/176))
- Enable LTO in release builds ([#177](https://github.com/oxipng/oxipng/pull/177))
- Use deterministic compression strategy ([#179](https://github.com/oxipng/oxipng/pull/179))
- Fix decoding interlaced images with height or width <= 2 ([#175](https://github.com/oxipng/oxipng/pull/175))
- Preallocate memory in reduced_alpha_to_up ([#180](https://github.com/oxipng/oxipng/pull/180))
- Update `bit-vec` crate to 0.6

### Version 2.2.1

- Fix compression of very large files ([#167](https://github.com/oxipng/oxipng/pull/167)) ([#168](https://github.com/oxipng/oxipng/pull/168))

### Version 2.2.0

- Various internal improvements ([#154](https://github.com/oxipng/oxipng/pull/154)) ([#158](https://github.com/oxipng/oxipng/pull/158)) ([#160](https://github.com/oxipng/oxipng/pull/160)) ([#161](https://github.com/oxipng/oxipng/pull/161)) ([#162](https://github.com/oxipng/oxipng/pull/162)) ([#163](https://github.com/oxipng/oxipng/pull/163))
- Update `image` crate to 0.21.0
- Update `itertools` crate to 0.8.0
- Update `zopfli` crate to 0.4.0
- Use Rust edition 2018
- Bump minimum required Rust version to 1.31.0

### Version 2.1.8

- Fix non-standard sBIT headers in other code locations ([#153](https://github.com/oxipng/oxipng/issues/153))

### Version 2.1.7

- 80x faster palette reduction ([#150](https://github.com/oxipng/oxipng/pull/150))
- Optimize RGB to palette conversion ([#148](https://github.com/oxipng/oxipng/pull/148))
- Various microoptimizations ([#146](https://github.com/oxipng/oxipng/pull/146))
- Introduce third-party safe wrapper around cloudflare-zlib ([#149](https://github.com/oxipng/oxipng/pull/149))

### Version 2.1.6

- Identify and drop useless sRGB profiles ([#143](https://github.com/oxipng/oxipng/pull/143))
- Alpha heuristic improvements ([#144](https://github.com/oxipng/oxipng/pull/144))
- Bump `miniz_oxide` and `cloudflare-zlib-sys` to 0.2.0

### Version 2.1.5

- Fix issue where some images will incorrectly reduce bit depth ([#140](https://github.com/oxipng/oxipng/issues/140))

### Version 2.1.4

- Bump `image` crate to 0.20.0

### Version 2.1.3

- Fix i686 builds
- Performance improvements

### Version 2.1.2

- Fix issue with PNG to Indexed reduction on images with transparency pixel ([#129](https://github.com/oxipng/oxipng/issues/129))

### Version 2.1.1

- More fixes for alpha optimization on interlaced images ([#133](https://github.com/oxipng/oxipng/issues/133))

### Version 2.1.0

- [SEMVER_MINOR] Bump minimum Rust version to 1.27.0
- [SEMVER_MINOR] Reenable faster Cloudflare zlib compression on platforms that support it
- Fix memory leak with Cloudflare zlib ([#126](https://github.com/oxipng/oxipng/issues/126))
- Minor fixes and cleanup

### Version 2.0.2

- Fix an issue in alpha optimization on interlaced images ([#113](https://github.com/oxipng/oxipng/issues/113))

### Version 2.0.1

- Revert making Cloudflare zlib the default, as it introduced a major memory leak. It will be put back behind a feature flag, and reenabled when the issue is fixed.
- Revert minimum Rust version to 1.24.0

### Version 2.0.0

- [SEMVER_MAJOR] Make PngError a proper Error enum
- [SEMVER_MINOR] Bump minimum Rust version to 1.27.0
- [SEMVER_MINOR] Make Rayon an optional dependency (enabled by default)
- [SEMVER_MINOR] Option to limit wall clock time spent in optimization trials
- [SEMVER_MINOR] `--keep` option (works similar to `--strip`, but takes a comma-separated list of headers to keep, and removes all other non-critical headers)
- Use faster Cloudflare zlib compression on platforms that support it
- Allow specifying more than 2 filter types via the CLI
- Avoid double glob processing on unix
- Fix reading from stdin
- Cleanup help text
- Various performance improvements

### Version 1.0.4

- Bump `image` to 0.19.0
- Bump `bit-vec` to 0.5.0
- Bump `regex` to 1.0.0

### Version 1.0.3

- Bump dependencies

### Version 1.0.2

- Fix a change that breaks Itertools::flatten with recent Rust nightlies

### Version 1.0.1

- Bump rayon to 1.0 ([#99](https://github.com/oxipng/oxipng/pull/99) @cuviper)
- Bump minor versions of other dependencies for binary distribution

### Version 1.0.0

- Remove the C dependency on miniz, and replace it with a Rust version ([#57](https://github.com/oxipng/oxipng/issues/57))
  - This improves decompression speed by 15%. Compression speed is not affected.
  - [SEMVER_MAJOR] This also obsoletes the `-zm` command line option and the `memory` key on the `Options` struct.
  - Presets will be updated automatically. This means that presets 3 and higher will run significantly more quickly.
- [SEMVER_MAJOR] Adjust the presets, now that `-zm` is no longer an option.
  - `-o3` now tests all filter types. This will result in 50% more trials than before, but may give up to 10% more compression gain.
  - `-o4` and higher now test all alpha optimization types. This adds 5 trials specific to the alpha channel. Only transparent images are affected.
- Implement unix-specific permissions copying for `-p` option
- Performance optimizations
- Refactor of internal code

### Version 0.19.0

- [SEMVER_MAJOR] Default to overwriting the input file if `out_file` is not set.
  This does not affect the CLI, but with the library, it was easy to forget to set the `out_file`,
  and there was no warning that no output file would be written.
- Bump dependencies, reduces binary size by a considerable amount
- Hide all modules from documentation, and only export the specific structures that should be public.
  Previously there were too many implementation details made public. The modules are still public for the purposes of our integration tests,
  but we strongly advise against using undocumented modules. These may become private in the future.
- Internal refactoring and code cleanup
- Fix an error message that was displaying the wrong file path
- Fix an issue where the output file would not be written if the input was already optimized,
  even if the output path was different from the input path

### Version 0.18.3

- Return exit code of 1 if an error occurred while processing a file using the CLI app ([#93](https://github.com/oxipng/oxipng/issues/93))

### Version 0.18.2

- Bump `image` to 0.18
- Fix unfiltering of scan lines in interlaced images ([#92](https://github.com/oxipng/oxipng/issues/92))

### Version 0.18.1

- Bump `rayon` to 0.9
- Fix failure to optimize on certain grayscale images ([#89](https://github.com/oxipng/oxipng/issues/89))

### Version 0.18.0

- Bump `itertools` to 0.7
- Bump `image` to 0.17
- [SEMVER_MAJOR] Bump minimum rustc version to 1.20.0
- Fix parsing of glob paths on Windows ([#90](https://github.com/oxipng/oxipng/issues/90))

### Version 0.17.2

- Bump `image` to 0.16
- Quickly pass over files that do not have a PNG header ([#85](https://github.com/oxipng/oxipng/issues/85) @emielbeinema)
- Return an error instead of crashing on APNG files ([#83](https://github.com/oxipng/oxipng/issues/83) @emielbeinema)

### Version 0.17.1

- Remove VC++ download requirement for Windows users

### Version 0.17.0

- [SEMVER_MAJOR] Bump minimum required rustc version to 1.19.0
- [SEMVER_MINOR] Oxipng will now, by default, attempt to change all transparent pixels to `rgba(0, 0, 0, 0)` to improve compression.
  It does fast trials with filters 0 and 5 to see if this is an improvement over
  the existing alpha channel.
- [SEMVER_MINOR] Add a `-a` option to the command line (`alphas` in the struct) which enables 6 different
  trials for optimizing the alpha channel, using the previously mentioned fast heuristic.
  This option will make optimization of images with transparency somewhat slower,
  but may improve compression.
- Fixed a bug in reducing palettes for images with bit depth of two ([#80](https://github.com/oxipng/oxipng/issues/80))
- Fixed another bug in reducing palettes for images with bit depth less than eight ([#82](https://github.com/oxipng/oxipng/issues/82))
- Code cleanup
- Bump `image` to 0.15

### Version 0.16.3

- Fix command-line help text ([#70](https://github.com/oxipng/oxipng/issues/70))

### Version 0.16.2

- Publicly export `error` module

### Version 0.16.1

- Fix rayon's breaking changes that they made in a point release

### Version 0.16.0

- [SEMVER_MAJOR] Bump minimum rustc version to 1.17.0
- Bump `image` to 0.14
- Bump `rayon` to 0.8

### Version 0.15.2

- Bump `image` to 0.13 ([#65](https://github.com/oxipng/oxipng/pull/65))
- Bump `rayon` to 0.7
- Bump `itertools` to 0.6

### Version 0.15.1

- Ignore color reductions that would increase file size ([#61](https://github.com/oxipng/oxipng/issues/61))

### Version 0.15.0

- [SEMVER_MINOR] Check images for correctness before writing result ([#60](https://github.com/oxipng/oxipng/issues/60))
- Fix invalid output when reducing image to a different color type but file size does not improve ([#60](https://github.com/oxipng/oxipng/issues/60))
- Don't write new file if moving from interlaced to non-interlaced if new file would be larger

### Version 0.14.4

- Fix bug when reducing RGBA to Indexed if image has 256 colors plus a background color

### Version 0.14.3

- Fix multiple bugs when reducing transparency palettes

### Version 0.14.2

- Fix a bug when reducing palette in images with bit depth less than 8
- Fix a bug when reducing palette in images with transparency

### Version 0.14.1

- Remove zlib dependency and switch entirely to miniz, since zlib 1.2.11 was not working with oxipng. This costs some performance, but is better than having a broken application.

### Version 0.14.0

- Performance optimizations
- [SEMVER_MAJOR] Bump minimum rustc version to 1.13.0
- Add categories on crates.io

### Version 0.13.1

- Bump regex dependency to 0.2
- Bump byteorder dependency to 1.0
- Bump rayon dependency to 0.6

### Version 0.13.0

- Fix bug in certain PNG headers when reducing color type ([#52](https://github.com/oxipng/oxipng/issues/52))
- [SEMVER_MAJOR] Reduction functions now take `&mut PngData` and return a `bool` indicating whether the image was reduced
- [SMEVER_MAJOR] Bump minimum required rustc version to 1.12.0

### Version 0.12.0

- Performance optimizations
- Fix processing filenames that contain commas (@aliceatlas [#50](https://github.com/oxipng/oxipng/pull/50))
- [SEMVER_MINOR] Add zopfli option (-Z), disabled by default. Gives about 10% better compression, but is currently 50-100x slower.

### Version 0.11.0

- [SEMVER_MAJOR] Bump minimum rustc version to 1.9.0, required by dependencies
- [SEMVER_MINOR] Allow calling optimization presets via crate using `Options::from_preset`
- [SEMVER_MAJOR] Return proper `PngError` type which implements `std::error::Error` from `Result`s
- [SEMVER_MAJOR] Rename module `deflate::deflate` to `deflate`
- Performance optimizations

### Version 0.10.0

- [SEMVER_MINOR] Make clap and regex dependencies optional
  - Enabled by default, needed for executable build; can be disabled for use in crates
- Remove reduction from palette to grayscale, which was not working and provided minimal benefit

### Version 0.9.0

- [SEMVER_MAJOR] Significant refactoring of modules
- Use `itertools` to cleanup areas of code
- Use multiple threads for filtering trials

### Version 0.8.2

- Fix issue where images smaller than 4px width would crash on interlacing ([#42](https://github.com/oxipng/oxipng/issues/42))

### Version 0.8.1

- Minor optimizations
- Fix issue where interlaced images with certain widths would fail to optimize

### Version 0.8.0

- [SEMVER_MINOR] Add support for optimizing PNGs already loaded into memory via library function

### Version 0.7.0

- Minor compression improvement on interlaced images
- Performance optimizations
- [SEMVER_MINOR] Move default Options into a Default impl
- [SEMVER_MINOR] Add option for setting number of threads ([#39](https://github.com/oxipng/oxipng/issues/39))

### Version 0.6.0

- Fix issue where output directory would not be created if it did not exist
- Use miniz for compression strategies where it outperforms zlib
- [SEMVER_MINOR] Partially implement -p / --preserve, as far as stable Rust will allow for now
- [SEMVER_MINOR] Implement --fix to ignore CRC errors and recalculate correct CRC in output

### Version 0.5.0

- [SEMVER_MINOR] Palette entries can now reduced, on by default ([#11](https://github.com/oxipng/oxipng/issues/11))
- Don't report that we are in pretend mode if verbosity is set to none
- Add cargo bench suite ([#7](https://github.com/oxipng/oxipng/issues/7))

### Version 0.4.0

- Performance optimizations
- [SEMVER_MAJOR] `-s` automatically infers `--strip safe` ([#31](https://github.com/oxipng/oxipng/issues/31))
- Update byteorder and clap crates
- Fix issue where interlaced images incorrectly applied filters on the first line of a pass

### Version 0.3.0

- Properly decode interlaced images
- [SEMVER_MINOR] Allow converting between progressive and interlaced images ([#3](https://github.com/oxipng/oxipng/issues/3))
- Fix a bug that would cause oxipng to crash on very small images

### Version 0.2.2

- Limit number of threads to 1.5x number of cores
- Significantly improve memory usage, especially with high optimization levels. ([#32](https://github.com/oxipng/oxipng/issues/32))
- Refactor output code ([#19](https://github.com/oxipng/oxipng/issues/19))

### Version 0.2.1

- Add rustdoc for public methods and structs
- Improve filter mode 5 heuristic ([#16](https://github.com/oxipng/oxipng/issues/16))
- Add tests for edge-case images with subtitles ([#29](https://github.com/oxipng/oxipng/issues/29))

### Version 0.2.0

- Fix program version that is displayed when running `oxipng -V`
- Ensure `--quiet` mode is actually quiet (@SethDusek [#20](https://github.com/oxipng/oxipng/pull/20))
- Write status/debug information to stderr instead of stdout
- Use heuristics to determine best combination for `-o1` ([#21](https://github.com/oxipng/oxipng/issues/21))
- [SEMVER_MAJOR] Allow 'safe', 'all', or comma-separated list as options for `--strip`
- [SEMVER_MINOR] Add `-s` alias for `--strip`

### Version 0.1.1

- Fix `oxipng *` writing all input files to one output file ([#15](https://github.com/oxipng/oxipng/issues/15))

### Version 0.1.0

- Initial beta release
- Reduce color type and bit depth
- Recompress with zlib
- Multithreading
- Strip headers option
- Backup file before writing option
- Write to stdout option
