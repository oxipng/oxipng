# Oxipng

[![Build Status](https://github.com/oxipng/oxipng/workflows/oxipng/badge.svg)](https://github.com/oxipng/oxipng/actions?query=branch%3Amaster)
[![Version](https://img.shields.io/crates/v/oxipng.svg)](https://crates.io/crates/oxipng)
[![License](https://img.shields.io/crates/l/oxipng.svg)](https://github.com/oxipng/oxipng/blob/master/LICENSE)
[![Docs](https://docs.rs/oxipng/badge.svg)](https://docs.rs/oxipng)

## Overview

Oxipng is a multithreaded lossless PNG/APNG compression optimizer. It can be used via a command-line
interface or as a library in other Rust programs. It is fast and highly effective.

## Installing

Oxipng for Windows can be downloaded via the
[Releases](https://github.com/oxipng/oxipng/releases) section on its GitHub page. Recently,
however, Oxipng has also been made available through package managers. Check the list below for
up-to-date options.

For MacOS or Linux, it is recommended to install from your distro's package repository, provided
Oxipng is available there in a not too outdated version for your use case.

Oxipng is known to be packaged for the environments listed below.

[![Packaging status](https://repology.org/badge/vertical-allrepos/oxipng.svg?exclude_unsupported=1&columns=3&exclude_sources=modules,site)](https://repology.org/project/oxipng/versions)

Alternatively, Oxipng can be installed from Cargo, via the following command:

```
cargo install oxipng
```

Oxipng can also be built from source using the latest stable or nightly Rust.
This is primarily useful for developing on Oxipng.

```
git clone https://github.com/oxipng/oxipng.git
cd oxipng
cargo build --release
cp target/release/oxipng /usr/local/bin
```

The current minimum supported Rust version is **1.85.1**.

Oxipng follows Semantic Versioning.

## Usage

Oxipng is a command-line utility. An example usage, suitable for web, may be the following:

```
oxipng -o 4 --strip safe --alpha *.png
```

The most commonly used options are as follows:

- Optimization: `-o 0` through `-o 6` (or `-o max`), lower is faster, higher is better compression.
  The default (`-o 2`) is quite fast and provides good compression. Higher levels can be notably
  better* but generally have increasingly diminishing returns.
- Strip: Used to remove metadata info from processed images. Used via `--strip [safe,all]`.
  Can save a few kilobytes if you don't need the metadata. "Safe" removes only metadata that
  will never affect rendering of the image. "All" removes all metadata that is not critical
  to the image. You can also pass a comma-separated list of specific metadata chunks to remove.
  `-s` can be used as a shorthand for `--strip safe`.
- Alpha: `--alpha` can improve compression of images with transparency, by altering the color
  values of fully transparent pixels. This is generally recommended, but take care as this is
  technically a lossy transformation and may be unsuitable for some specific applications.

More advanced options can be found by running `oxipng --help`, or viewed [here](MANUAL.txt).

Some options have both short (`-a`) and long (`--alpha`) forms. Which form you use is just a
matter of preference. Multiple short options can be combined together, e.g.:
`-savvo6` is equivalent to to `--strip safe --alpha --verbose --verbose --opt 6`.
All options are case-sensitive.

\* Note that oxipng is not a brute-force optimizer. This means that while higher optimization levels
are almost always better or equal to lower levels, this is not guaranteed and it is possible in
rare circumstances that a lower level may give a marginally smaller output. Similarly, using Zopfli
compression (`-z`) is not guaranteed to always be better than without.

## APNG support

Oxipng currently only supports limited optimization of animated PNGs (APNGs). It can perform
alpha-optimization, refiltering and recompression of all frames, but all transformations will be
disabled. For best results, it is recommended to use another tool such as
[apngopt](https://sourceforge.net/projects/apng/files/APNG_Optimizer/) prior to running Oxipng.

## Git integration via [pre-commit]

Create a `.pre-commit-config.yaml` file like this, or add the lines after the `repos` map
preamble to an already existing one:

```yaml
repos:
  - repo: https://github.com/oxipng/oxipng
    rev: v10.0.0
    hooks:
      - id: oxipng
        args: ["-o", "4", "--strip", "safe", "--alpha"]
```
[pre-commit]: https://pre-commit.com/

## Docker

A Docker image is availlable at [`ghcr.io/oxipng/oxipng`](https://github.com/oxipng/oxipng/pkgs/container/oxipng) for `linux/amd64` and `linux/arm64`.

You can use it the following way:

```bash
docker run --rm -v $(pwd):/work ghcr.io/oxipng/oxipng -o 4 /work/file.png
```

Some older images are also available at [`ghcr.io/shssoichiro/oxipng`](https://github.com/users/shssoichiro/packages/container/package/oxipng).

## Library Usage

Although originally intended to be used as an executable, Oxipng can also be used as a library in
other Rust projects. To do so, simply add Oxipng as a dependency in your Cargo.toml. You should then
have access to all of the library functions [documented here](https://docs.rs/oxipng). The simplest
method of usage involves creating an [Options
struct](https://docs.rs/oxipng/latest/oxipng/struct.Options.html) and passing it, along with an
input filename, into the [optimize function](https://docs.rs/oxipng/latest/oxipng/fn.optimize.html).

It is recommended to disable the "binary" feature when including Oxipng as a library. Currently, there is
no simple way to just disable one feature in Cargo, it has to be done by disabling default features
and specifying the desired ones, for example:
`oxipng = { version = "10.0", features = ["parallel", "zopfli", "filetime"], default-features = false }`

## Software using Oxipng

- [ImageOptim](https://imageoptim.com): Mac app and web service for optimizing images
- [Squoosh](https://squoosh.app): Web app for optimizing images
- [FileOptimizer](https://nikkhokkho.sourceforge.io/?page=FileOptimizer): Windows app for optimizing files
- [Curtail](https://github.com/Huluti/Curtail): Linux app for optimizing images
- [pyoxipng](https://pypi.org/project/pyoxipng/): Python wrapper for Oxipng
- [jSquash](https://github.com/jamsinclair/jSquash): Collection of WebAssembly image codecs
- [Trunk](https://trunk.io): Developer experience toolkit for managing code

## History

Oxipng began in 2015 as a rewrite of the OptiPNG project. The core goal was to implement
multithreading, which would have been very difficult to do within the existing C codebase of OptiPNG.
This also served as an opportunity to choose a more modern, safer language (Rust).

However, Oxipng has evolved considerably since then. While some of the options remain similar to
OptiPNG, the architecture and capabilities are now quite different. It is not a drop-in
replacement - if you are migrating from OptiPNG, please check the [help](MANUAL.txt) before using.

## Contributing

Any contributions are welcome and will be accepted via pull request on GitHub. Bug reports can be
filed via GitHub issues. Please include as many details as possible. If you have the capability
to submit a fix with the bug report, it is preferred that you do so via pull request,
however you do not need to be a Rust developer to contribute.
Other contributions (such as improving documentation or translations) are also welcome via GitHub.

## License

Oxipng is open-source software, distributed under the MIT license.

## Benchmarks

An independent benchmark conducted by @demetris is linked here with permission:\
[oxipng and friends: A comparison of PNG optimization tools](https://op111.net/posts/2025/09/png-compression-oxipng-optipng-fileoptimizer-cwebp/)
