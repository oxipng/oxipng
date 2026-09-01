# syntax=docker/dockerfile:1
# check=error=true
FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.9.0@sha256:c64defb9ed5a91eacb37f96ccc3d4cd72521c4bd18d5442905b95e2226b0e707 AS xx

FROM --platform=$BUILDPLATFORM rust:1.88.0-alpine@sha256:9dfaae478ecd298b6b5a039e1f2cc4fc040fc818a2de9aa78fa714dea036574d AS base

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
