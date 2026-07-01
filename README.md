# jam-viewer

A guitar practice app that visualizes the chord and scale you should play, on a
fretboard centered at fret 12. Built with [Leptos](https://leptos.dev/) as a
**single fullstack Rust app** (server-side rendering + WASM hydration) — the UI,
the song library, and the YouTube/file ingestion API all live in one codebase and
run from one process. Chord recognition runs in a separate
[ChordMini](https://github.com/ptnghia-j/ChordMiniApp) container.

## Run it

```bash
# one-time setup
cargo install cargo-leptos                # fullstack build tool
rustup target add wasm32-unknown-unknown  # WASM target

# dev server with live reload at http://127.0.0.1:5002
cargo leptos watch

# production build (server binary + hashed client assets under ./target)
cargo leptos build --release
./target/release/jam-viewer               # serves the release build
```

`cargo leptos` compiles the crate twice from the same source — once natively for
the axum server (the `ssr` feature) and once to WASM for the browser (the
`hydrate` feature) — then bundles the JS glue, CSS, and hashed assets. There is
no separate frontend build step and no sidecar process.

## Project layout

| Path                          | Purpose                                            |
| ----------------------------- | -------------------------------------------------- |
| `style.css`                   | App styling (served by cargo-leptos)               |
| `src/lib.rs`                  | Crate root: modules, HTML shell, hydrate entry     |
| `src/main.rs`                 | axum server: Leptos routes + `/api/audio/{id}`     |
| `src/app.rs`                  | Root component, ingest panel, demo + song player   |
| `src/api.rs`                  | `#[server]` functions (library / ingest API)       |
| `src/server/ingest.rs`        | Server-side ingest: yt-dlp, ChordMini proxy, cache |
| `src/ingest.rs`               | Shared ingest types + chord parsing                |
| `src/theory.rs`               | Notes, chords, scales, fretboard math, demo song   |
| `src/components/timeline.rs`  | Sliding chord timeline (also the header)           |
| `src/components/fretboard.rs` | Fretboard visualization                            |

The app has two modes:

- **Demo jam** (default): a hard-coded looping progression (`Am → F → C → Em →
  G7`) in C major, driven by a beat clock.
- **Ingested song**: drop in an audio file or paste a YouTube URL and the app
  detects the real chords and **highlights them in sync with the audio** as it
  plays.

A continuously **sliding timeline** at the top doubles as the header (key,
current scale, beat counter) and shows the repeating chords coming up; advancing
to the next chord slides the strip smoothly rather than snapping. Each chord on
the timeline shows its letter name with its **Roman numeral** in the key beneath
it (e.g. in C major: `Am`→`vi`, `F`→`IV`, `C`→`I`, `Em`→`iii`, `G7`→`V7`), so
you can read each chord's place in the scale. The fretboard shows four layers —
chord root, chord tones, the current chord's scale, and the overall **song
scale** (always shown) — and the notes animate cleanly between states as the
chord changes. A **CAGED** toggle in the transport bar swaps the layer view for
the five movable major chord shapes (C-A-G-E-D) of the current chord, color-coded
with connecting boundary boxes. The fret window (currently frets 7–17) is driven
entirely by the `FRET_MIN`/`FRET_MAX` constants in `src/theory.rs`, so it — and
the CAGED overlay — can be retuned by editing only those two values.

For soloing, the fretboard adds three lead-playing aids, driven from the transport bar:

- **Names / Degrees toggle** — relabel every note between its pitch name (`C`,
  `D#`) and its scale degree relative to the current scale root (`1`, `b3`, `5`).
- **Scale position boxes** — a `Pos N/5` stepper highlights one CAGED-style
  position at a time and dims notes outside it, so you can anchor a solo in one
  shape and cycle through positions.
- **Next-chord targeting** — an amber dot marks where the *upcoming* chord's
  tones live, so you can aim your resolutions ahead of the change.

## Ingesting real songs

Chord recognition runs on a self-hosted [ChordMini](https://github.com/ptnghia-j/ChordMiniApp)
backend (madmom for beats + Chord-CNN-LSTM for chords) in its own container. The
jam-viewer server does everything else itself: for YouTube URLs it downloads the
audio with `yt-dlp` (needs `ffmpeg` on `PATH`), forwards audio to ChordMini for
analysis, caches the result, and serves the audio back for playback — all over
same-origin `#[server]` functions and the `/api/audio/{id}` route, so no CORS or
extra process is involved.

### 1. Run the ChordMini backend container

Any Docker-compatible runtime works. Build the image from the ChordMini repo's
`python_backend/` and run it on port 5001:

```bash
# from a clone of https://github.com/ptnghia-j/ChordMiniApp
#   git submodule update --init python_backend/models/Chord-CNN-LSTM \
#                                python_backend/models/Beat-Transformer
cd python_backend
docker build -t chordmini-backend .
docker run -d -p 5001:8080 --name chordmini chordmini-backend
# health check: curl http://localhost:5001/api/model-info
```

The jam-viewer server reads the ChordMini base URL from the `CHORDMINI_URL`
environment variable (default `http://localhost:5001`).

### 2. Run jam-viewer and use it

```bash
CHORDMINI_URL=http://localhost:5001 cargo leptos watch   # serves on :5002
```

In the app's **ingest panel**: choose an audio file and click **Analyze file**,
or paste a **YouTube URL** and click **Analyze YouTube**. The app detects the
key, fills the timeline with the song's chords, and highlights the current chord
on the fretboard in time with the `<audio>` player. **✕ Clear** returns to the
demo jam.

### Persistent library

Every analyzed song is **cached on disk** (under `server/_ingest_cache/`, override
with the `INGEST_CACHE` env var) keyed by a stable id — `yt_<videoId>` for
YouTube, `file_<sha256>` for uploads — as `<id>.<ext>` (audio) plus `<id>.json`
(chords + metadata). Re-analyzing the same song returns instantly instead of
re-downloading and re-running recognition, and the cache survives restarts (it is
rebuilt from disk).

Previously analyzed songs appear in a **Library** panel in the ingest panel. The
panel is **collapsed by default**; expand it to reveal a **search box** (filter
saved songs by title) and clickable chips — clicking a chip reloads that song
immediately. The library is backed by the `get_library` and `load_song`
`#[server]` functions.
