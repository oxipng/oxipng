#!/bin/sh -eu
cargo build
cargo xtask mangen

./target/debug/oxipng -V > MANUAL.txt
#Redirect all streams to prevent detection of the terminal width and force an internal default of 100
./target/debug/oxipng --help >> MANUAL.txt 2>/dev/null </dev/null

cp target/xtask/mangen/manpages/oxipng.1 oxipng.1
