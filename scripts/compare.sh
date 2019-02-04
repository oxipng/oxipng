#!/bin/bash
cargo build --release
cp README.template.md README.md

CORES=$(sysctl -n hw.ncpu 2>/dev/null || grep -c ^processor /proc/cpuinfo)
CPU=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep '^model name' /proc/cpuinfo | sed 's/model name.\+: //g' | head -n 1)
OXIPNG_VERSION=$(./target/release/oxipng -V)
OPTIPNG_VERSION=$(optipng -v | head -n 1)
RUST_VERSION=$(rustc -V)
echo "Tested $OXIPNG_VERSION (compiled on $RUST_VERSION) against $OPTIPNG_VERSION on $CPU with $CORES logical cores" >> README.md
echo -e '\n\n```\n' >> README.md

hyperfine --warmup 5 './target/release/oxipng -P ./tests/files/rgb_16_should_be_grayscale_8.png' 'optipng -simulate ./tests/files/rgb_16_should_be_grayscale_8.png' | ./node_modules/.bin/strip-ansi >> README.md
echo -e '\n\n' >> README.md
hyperfine --warmup 5 './target/release/oxipng -o4 -P ./tests/files/rgb_16_should_be_grayscale_8.png' 'optipng -o 4 -simulate ./tests/files/rgb_16_should_be_grayscale_8.png' | ./node_modules/.bin/strip-ansi >> README.md

echo -e '\n```\n' >> README.md
