#!/usr/bin/env bash

set -e
set -o pipefail

if [ "$1" == "docker" ]; then
    shift;
    rust_version=$(tr -d '\n' <rust-toolchain)
    DOCKER_BUILDKIT=1 docker build --build-arg rust_version="$rust_version" --file Dockerfile --output docker_target . "$@"
    exit $?
fi

THIS_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$THIS_SCRIPT_DIR"

# I think this is all the size reduction possible before switching to nightly
# to be able to use Xargo + 'panic_immediate_abort' for std.
export RUSTFLAGS="-C debuginfo=0 -C force-unwind-tables=no -C panic=abort -C embed-bitcode=no -Clinker-plugin-lto"

mkdir -p static

if [ -f assets/sites.json ]; then
    cp assets/sites.json static/sites.json
else
    cp sites.demo.json static/sites.json
fi

wasm-pack build --release --no-typescript --target web --out-name wasm --out-dir ./static

cp css/*.css static/

find assets \( -name "favicon*.ico" -or -name "favicon*.png" \) \
     -exec cp "{}" static/ \;

if [ "$1" == "check" ]; then
    # We need standard rustc options for tests
    unset RUSTFLAGS
    
    cargo test
    cargo clean -p "$(cargo read-manifest | jq -r '.name')"
    cargo clippy -- -D warnings
    cargo fmt -- --check
elif [ "$1" == "audit" ]; then
    cargo audit
fi
