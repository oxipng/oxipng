# syntax=docker/dockerfile:1
# check=error=true
FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.10.0@sha256:40fe21f44bb8cddf1eb1226db190d0a984118f62ee02a91b77d99448d28264a6 AS xx

FROM --platform=$BUILDPLATFORM rust:1.99.0-alpine@sha256:0cce0a5e0e8ba67b455257a3a02a1d99005f382748789d6464460028810f1627 AS base

RUN apk update && \
    apk add \
        gcc \
        g++ \
        clang

COPY --from=xx / /

ARG TARGETPLATFORM
RUN xx-info env

RUN xx-apk add \
    gcc \
    musl-dev \
    libdeflate

WORKDIR /src

COPY . .

RUN --mount=type=cache,target=/root/.cargo/git/db \
    --mount=type=cache,target=/root/.cargo/registry/cache \
    --mount=type=cache,target=/root/.cargo/registry/index \
    xx-cargo build --release && \
    xx-verify /src/target/$(xx-cargo --print-target-triple)/release/oxipng && \
    cp /src/target/$(xx-cargo --print-target-triple)/release/oxipng /src/target/oxipng

FROM scratch AS tool

LABEL org.opencontainers.image.title="Oxipng"
LABEL org.opencontainers.image.description="Multithreaded PNG optimizer written in Rust"
LABEL org.opencontainers.image.authors="Joshua Holmer <jholmer.in@gmail.com>"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/oxipng/oxipng"

COPY --from=base /src/target/oxipng /usr/local/bin/oxipng

WORKDIR /work
ENTRYPOINT [ "oxipng" ]
CMD [ "--help" ]
