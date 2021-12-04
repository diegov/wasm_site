ARG rust_version
FROM docker.io/rust:${rust_version}-bullseye
MAINTAINER Diego Veralli "diego@diegoveralli.com"

ENV DEBIAN_FRONTEND noninteractive

RUN apt-get -y update
RUN apt-get install -y wabt
RUN apt-get install -y binaryen

RUN mkdir /s

COPY ./rust-toolchain /s/

WORKDIR /s

RUN apt-get install -y libssl-dev
RUN cargo install wasm-pack --no-default-features
RUN cargo install wasm-snip

# This is done automatically when calling wasm-pack, but this way we cache a layer
RUN rustup target add wasm32-unknown-unknown

COPY . /s

RUN ./build.sh

RUN ./minify.sh static/wasm_bg.wasm
RUN ./snip.sh static/wasm_bg.wasm

ENV BUILD_OUTPUT_DIR /s/static
