# syntax=docker/dockerfile:1

# --------------------------------------------------------------------------- #
# Stage 1: build the fullstack Leptos app (server binary + hydrated WASM/site).
# --------------------------------------------------------------------------- #
FROM rust:1-bookworm AS builder

# cargo-leptos drives the SSR + WASM build; the wasm target compiles the client.
RUN rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos --locked

WORKDIR /app
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
# the server is reachable from outside the container; persist the song cache in
# a mountable volume.
ENV LEPTOS_OUTPUT_NAME=jam-viewer \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:5002 \
    INGEST_CACHE=/data/cache \
    CHORDMINI_URL=http://localhost:5001

RUN mkdir -p /data/cache
VOLUME ["/data/cache"]

EXPOSE 5002

CMD ["/app/jam-viewer"]
