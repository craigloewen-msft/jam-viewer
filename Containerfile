# syntax=docker/dockerfile:1

# --------------------------------------------------------------------------- #
# Stage 1a: toolchain base — wasm target + prebuilt cargo-leptos & cargo-chef.
# Installing the tools from release binaries (instead of `cargo install`, which
# compiles them from source) shaves several minutes off a clean build.
# --------------------------------------------------------------------------- #
FROM rust:1-bookworm AS chef

# cargo-leptos drives the SSR + WASM build; the wasm target compiles the client.
RUN rustup target add wasm32-unknown-unknown

# Pinned tool versions (bump deliberately; keeps builds reproducible).
ARG CARGO_LEPTOS_VERSION=v0.3.7
ARG CARGO_CHEF_VERSION=v0.1.77
RUN set -eux; \
    curl -fsSL "https://github.com/leptos-rs/cargo-leptos/releases/download/${CARGO_LEPTOS_VERSION}/cargo-leptos-x86_64-unknown-linux-gnu.tar.gz" \
      | tar -xz -C /usr/local/bin --strip-components=1 cargo-leptos-x86_64-unknown-linux-gnu/cargo-leptos; \
    apt-get update && apt-get install -y --no-install-recommends xz-utils && rm -rf /var/lib/apt/lists/*; \
    curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/${CARGO_CHEF_VERSION}/cargo-chef-x86_64-unknown-linux-gnu.tar.xz" \
      | tar -xJ -C /usr/local/bin --strip-components=1 cargo-chef-x86_64-unknown-linux-gnu/cargo-chef; \
    cargo-leptos --version; cargo chef --version

WORKDIR /app

# --------------------------------------------------------------------------- #
# Stage 1b: capture the dependency "recipe" from the manifests only.
# --------------------------------------------------------------------------- #
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --------------------------------------------------------------------------- #
# Stage 1c: build the fullstack Leptos app (server binary + hydrated WASM/site).
# Dependencies are cooked first in their own cached layer, so editing app source
# no longer recompiles every crate. cargo-leptos runs two builds — a native SSR
# binary (target/) and a wasm client (target/front) — so we cook both, matching
# their target dirs/features exactly for cache reuse.
# --------------------------------------------------------------------------- #
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json \
        --no-default-features --features ssr --bin jam-viewer \
 && cargo chef cook --release --recipe-path recipe.json \
        --target wasm32-unknown-unknown --target-dir target/front \
        --no-default-features --features hydrate

COPY . .

# Produces target/release/jam-viewer (server) and target/site (static assets).
RUN cargo leptos build --release

# --------------------------------------------------------------------------- #
# Stage 2: slim runtime with the tools ingestion needs (yt-dlp + ffmpeg).
# --------------------------------------------------------------------------- #
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg curl \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux \
        -o /usr/local/bin/yt-dlp \
    && chmod +x /usr/local/bin/yt-dlp \
    && apt-get purge -y --auto-remove curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/jam-viewer /app/jam-viewer
COPY --from=builder /app/target/site /app/site

# Leptos reads its runtime configuration from these env vars. Bind to 0.0.0.0 so
# the server is reachable from outside the container. The song cache is written
# to /data/cache; bind-mount a host directory there so the converted files land
# in a folder you can access and copy to another machine.
ENV LEPTOS_OUTPUT_NAME=jam-viewer \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:5002 \
    INGEST_CACHE=/data/cache \
    CHORDMINI_URL=http://localhost:5001

RUN mkdir -p /data/cache

EXPOSE 5002

CMD ["/app/jam-viewer"]
