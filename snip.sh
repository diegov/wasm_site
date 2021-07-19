#!/usr/bin/env bash

set -e

INPUT="$1"
OUTPUT="$2"

wasm-snip --snip-rust-fmt-code \
          --snip-rust-panicking-code \
          -o "$INPUT" \
          "$INPUT"

wasm-strip "$INPUT"
wasm-opt -o "$OUTPUT" -Oz --dce "$INPUT"
