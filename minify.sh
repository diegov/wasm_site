#!/usr/bin/env bash

set -e

tmpdir=$(mktemp -d)

watfile="$tmpdir"/file.wat

wasm2wat "$1" > "$watfile"

sed -i s%'/usr/local/cargo/registry/src/'%'                              '%gi "$watfile"
sed -i s%'github.com'%'          '%gi "$watfile"

wat2wasm "$watfile" -o "$1"
