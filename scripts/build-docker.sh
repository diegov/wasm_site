#!/usr/bin/env bash

set -e
set -o pipefail

if which podman >/dev/null; then
    command=podman
else
    command=docker
fi 

THIS_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

cd "$THIS_SCRIPT_DIR"/..

mkdir -p docker_target
rust_version=$(tr -d '\n' <rust-toolchain)
image=$("$command" build --build-arg rust_version="$rust_version" \
               . --file Dockerfile | tail -n 1)

"$command" run -v "$PWD"/docker_target:/output \
           "$image" \
           ./scripts/copy_container_output.sh /output

