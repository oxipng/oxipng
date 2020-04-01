# Oxipng

[![Build Status](https://travis-ci.org/shssoichiro/oxipng.svg?branch=master)](https://travis-ci.org/shssoichiro/oxipng)
[![Version](https://img.shields.io/crates/v/oxipng.svg)](https://crates.io/crates/oxipng)
[![License](https://img.shields.io/crates/l/oxipng.svg)](https://github.com/shssoichiro/oxipng/blob/master/LICENSE)

## Overview

Oxipng is a multithreaded lossless PNG compression optimizer. It can be used via a command-line
interface or as a library in other Rust programs.

## Installing

Oxipng can be downloaded from the [Releases](https://github.com/shssoichiro/oxipng/releases) link on the GitHub page.

Oxipng can also be installed from Cargo, via the following command:
```
cargo install oxipng
```

Alternatively, oxipng can be built from source using the latest stable or nightly Rust:
```
git clone https://github.com/shssoichiro/oxipng.git
cd oxipng
cargo build --release
cp target/release/oxipng /usr/local/bin
```

The current minimum supported Rust version is **1.41.0**. Oxipng may compile on earlier versions of Rust,
but there is no guarantee.

Oxipng follows Semantic Versioning.

## Usage

Oxipng is a command-line utility. Basic usage looks similar to the following:

```
oxipng -o 4 -i 1 --strip safe *.png
```

The most commonly used options are as follows:
* Optimization: `-o 1` through `-o 6`, lower is faster, higher is better compression.
The default (`-o 2`) is sufficiently fast on a modern CPU and provides 30-50% compression
gains over an unoptimized PNG. `-o 4` is 6 times slower than `-o 2` but can provide 5-10%
extra compression over `-o 2`. Using any setting higher than `-o 4` is unlikely
to give any extra compression gains and is not recommended.
* Interlacing: `-i 1` will enable [Adam7](https://en.wikipedia.org/wiki/Adam7_algorithm)
PNG interlacing on any images that are processed. `-i 0` will remove interlacing from all
processed images. Not specifying either will keep the same interlacing state as the
input image. Note: Interlacing can add 25-50% to the size of an optimized image. Only use
it if you believe the benefits outweigh the costs for your use case.
* Strip: Used to remove metadata info from processed images. Used via `--strip [safe,all]`.
Can save a few kilobytes if you don't need the metadata. "Safe" removes only metadata that
will never affect rendering of the image. "All" removes all metadata that is not critical
to the image. You can also pass a comma-separated list of specific metadata chunks to remove.
`-s` can be used as a shorthand for `--strip safe`.

More advanced options can be found by running `oxipng -h`.

## Library Usage

Although originally intended to be used as an executable, oxipng can also be used as a library in
other Rust projects. To do so, simply add oxipng as a dependency in your Cargo.toml,
then `extern crate oxipng` in your project. You should then have access to all of the library
functions [documented here](https://docs.rs/oxipng). The simplest
method of usage involves creating an
[Options struct](https://docs.rs/oxipng/0.13.0/oxipng/struct.Options.html) and
passing it, along with an input filename, into the
[optimize function](https://docs.rs/oxipng/0.13.0/oxipng/fn.optimize.html).

## History

Oxipng began as a complete rewrite of the OptiPNG project,
which was assumed to be dead as no commit had been made to it since March 2014.
(OptiPNG has since released a new version, after Oxipng was first released.)
The name has been changed to avoid confusion and potential legal issues.

The core goal of rewriting OptiPNG was to implement multithreading,
which would be very difficult to do within the existing C codebase of OptiPNG.
This also served as an opportunity to choose a more modern, safer language (Rust).

## Contributing

Any contributions are welcome and will be accepted via pull request on GitHub. Bug reports can be
filed via GitHub issues. Please include as many details as possible. If you have the capability
to submit a fix with the bug report, it is preferred that you do so via pull request,
however you do not need to be a Rust developer to contribute.
Other contributions (such as improving documentation or translations) are also welcome via GitHub.

## License

Oxipng is open-source software, distributed under the MIT license.

## Benchmarks

Tested oxipng 2.3.0 (compiled on rustc 1.38.0 (625451e37 2019-09-23)) against OptiPNG version 0.7.7 on Intel(R) Core(TM) i7-6700HQ CPU @ 2.60GHz with 8 logical cores


```

Benchmark #1: ./target/release/oxipng -P ./tests/files/rgb_16_should_be_grayscale_8.png
  Time (mean ± σ):      92.4 ms ±   2.7 ms    [User: 249.4 ms, System: 14.5 ms]
  Range (min … max):    88.4 ms …  98.5 ms    31 runs
 
Benchmark #2: optipng -simulate ./tests/files/rgb_16_should_be_grayscale_8.png
  Time (mean ± σ):     273.5 ms ±   1.7 ms    [User: 272.4 ms, System: 1.2 ms]
  Range (min … max):   270.3 ms … 275.1 ms    10 runs
 
Summary
  './target/release/oxipng -P ./tests/files/rgb_16_should_be_grayscale_8.png' ran
    2.96 ± 0.09 times faster than 'optipng -simulate ./tests/files/rgb_16_should_be_grayscale_8.png'



Benchmark #1: ./target/release/oxipng -o4 -P ./tests/files/rgb_16_should_be_grayscale_8.png
  Time (mean ± σ):     120.7 ms ±   4.1 ms    [User: 486.3 ms, System: 14.8 ms]
  Range (min … max):   113.7 ms … 130.0 ms    23 runs
 
Benchmark #2: optipng -o 4 -simulate ./tests/files/rgb_16_should_be_grayscale_8.png
  Time (mean ± σ):     916.8 ms ±   7.4 ms    [User: 914.8 ms, System: 1.7 ms]
  Range (min … max):   906.8 ms … 929.9 ms    10 runs
 
Summary
  './target/release/oxipng -o4 -P ./tests/files/rgb_16_should_be_grayscale_8.png' ran
    7.59 ± 0.26 times faster than 'optipng -o 4 -simulate ./tests/files/rgb_16_should_be_grayscale_8.png'

```

