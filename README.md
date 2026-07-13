# jam-viewer

A guitar practice app that visualizes the chord and scale to play on a fretboard.
Built with [Leptos](https://leptos.dev/) as a single fullstack Rust app (SSR +
WASM). Real-song chord recognition runs in a separate
[ChordMini](https://github.com/ptnghia-j/ChordMiniApp) container.

## Prerequisites

```bash
cargo install cargo-leptos                # fullstack build tool
rustup target add wasm32-unknown-unknown  # WASM target
```

`yt-dlp` and `ffmpeg` must be on `PATH` for YouTube ingestion.

## Run

```bash
cargo leptos watch    # dev server with live reload at http://127.0.0.1:5002
```

That's enough for the built-in **demo jam**. For analyzing real songs, also run
the ChordMini backend (below) and point jam-viewer at it via `CHORDMINI_URL`
(default `http://localhost:5001`).

## Run in a container

The [`Containerfile`](./Containerfile) builds the whole app (SSR server + WASM
client) and bundles `yt-dlp` and `ffmpeg`, so no host toolchain is needed. Build
and run it with `wslc.exe` (on WSL the container CLI is invoked as `wslc.exe`):

```bash
wslc.exe build -f Containerfile -t jam-viewer .
wslc.exe run -d -p 5002:5002 --name jam-viewer \
  -v ./cache:/data/cache \
  jam-viewer
```

The app is then served at http://localhost:5002. The `-v ./cache:/data/cache`
bind mount writes the analyzed-song library (`INGEST_CACHE`) into a `cache/`
folder next to this README, so the converted files (audio + chord JSON) land on
the host where you can access them directly and copy them to another machine.

To analyze real songs, run the [ChordMini backend](#chordmini-backend-optional-for-real-songs)
too and point the container at it. Since ChordMini runs in its own container,
use the host address instead of `localhost`:

```bash
wslc.exe run -d -p 5002:5002 --name jam-viewer \
  -v ./cache:/data/cache \
  -e CHORDMINI_URL=http://host.docker.internal:5001 \
  jam-viewer
```

## ChordMini backend (optional, for real songs)

Real-song chord recognition is provided by [ChordMini](https://github.com/ptnghia-j/ChordMiniApp),
a separate upstream project. jam-viewer only talks to it over HTTP, so its source
is intentionally **not** vendored here — you build its container from a clone.
The backend lives in that repo's `python_backend/` directory and serves gunicorn
(`app:app`) on port 8080; the run command maps it to host port 5001.

Clone (pinned to the commit this was last verified against) and build/run with
`wslc.exe`:

```bash
git clone https://github.com/ptnghia-j/ChordMiniApp.git
cd ChordMiniApp
git checkout 7ee42fd            # pinned: verified against this upstream commit
cd python_backend
wslc.exe build -t chordmini-backend .
wslc.exe run -d -p 5001:8080 --name chordmini chordmini-backend
```

If you've already built the image before, just (re)start the container:

```bash
wslc.exe start chordmini
```

jam-viewer talks to it at `POST /api/recognize-chords`; check it's up with
`curl http://localhost:5001/` (returns `{"status":"healthy"}`).

Then start jam-viewer with the backend URL:

```bash
CHORDMINI_URL=http://localhost:5001 cargo leptos watch
```
