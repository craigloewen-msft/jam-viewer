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

## ChordMini backend (optional, for real songs)

Build and run the image from a clone of
[ChordMiniApp](https://github.com/ptnghia-j/ChordMiniApp) using `wslc`:

```bash
cd python_backend
wslc build -t chordmini-backend .
wslc run -d -p 5001:8080 --name chordmini chordmini-backend
```

Then start jam-viewer with the backend URL:

```bash
CHORDMINI_URL=http://localhost:5001 cargo leptos watch
```
