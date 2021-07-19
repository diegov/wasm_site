ARG rust_version
FROM rust:${rust_version}-bullseye as buildenv
MAINTAINER Diego Veralli "diego@diegoveralli.com"

ENV DEBIAN_FRONTEND noninteractive

RUN apt-get -y update
RUN apt-get install -y wabt
RUN apt-get install -y binaryen

RUN mkdir /s

COPY ./rust-toolchain /s/

WORKDIR /s

RUN cargo install wasm-pack
RUN cargo install wasm-snip

COPY . /s

RUN ./build.sh

RUN ./minify.sh static/wasm_bg.wasm
RUN ./snip.sh static/wasm_bg.wasm tmp.wasm
RUN mv tmp.wasm static/wasm_bg.wasm

FROM scratch AS export-stage

COPY --from=buildenv /s/static/ .
