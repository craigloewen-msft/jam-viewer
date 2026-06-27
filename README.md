# jam-viewer

A guitar practice app that visualizes the chord and scale you should play, on a
fretboard centered at fret 12. Built with [Leptos](https://leptos.dev/) (WASM,
client-side rendering).

## Run it

```bash
# one-time setup
cargo install trunk                      # build tool (or: pacman -S trunk)
rustup target add wasm32-unknown-unknown # WASM target (or: pacman -S rust-wasm)

# dev server with live reload at http://127.0.0.1:8080
trunk serve --open

# production build into ./dist
trunk build --release
```

## Why Trunk and not just cargo?

`cargo build` produces a raw `.wasm` file but not the JavaScript glue, bundled
CSS, or HTML needed to run it in a browser. Trunk wraps Cargo + `wasm-bindgen`
and the asset pipeline to do all of that. (`cargo leptos` is the alternative,
but it's for full-stack/SSR apps — this one is client-only.)

## Project layout

| Path                          | Purpose                                          |
| ----------------------------- | ------------------------------------------------ |
| `index.html`                  | Trunk entry point                                |
| `style.css`                   | App styling                                      |
| `src/main.rs`                 | Mounts the app                                   |
| `src/app.rs`                  | Root component, ingest panel, demo + song player |
| `src/theory.rs`               | Notes, chords, scales, fretboard math, demo song |
| `src/ingest.rs`               | Song ingestion (file / YouTube) + chord parsing  |
| `src/components/timeline.rs`  | Sliding chord timeline (also the header)         |
| `src/components/fretboard.rs` | Fretboard visualization                          |
| `server/ingest_server.py`     | Ingest sidecar (yt-dlp + ChordMini proxy)        |

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

## Ingesting real songs

Chord recognition runs on a self-hosted [ChordMini](https://github.com/ptnghia-j/ChordMiniApp)
backend (madmom for beats + Chord-CNN-LSTM for chords). A small Flask **sidecar**
(`server/ingest_server.py`) is the browser-facing entry point: it proxies audio
to ChordMini for analysis and, for YouTube URLs, downloads the audio with
`yt-dlp` and serves it back for playback. (The browser talks only to the sidecar
because the ChordMini container does not emit CORS headers.)

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

### 2. Run the ingest sidecar

```bash
cd server
python -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt          # flask, flask-cors, requests, yt-dlp
# (yt-dlp also needs ffmpeg on PATH)
CHORDMINI_URL=http://localhost:5001 python ingest_server.py   # listens on :5002
```

Or start it via `server/start.sh`.

### 3. Use it

Run `trunk serve`, then in the app's **ingest panel**: choose an audio file and
click **Analyze file**, or paste a **YouTube URL** and click **Analyze YouTube**.
The app detects the key, fills the timeline with the song's chords, and
highlights the current chord on the fretboard in time with the `<audio>` player.
The **⚙** button reveals a configurable server URL (defaults to
`http://localhost:5002`). **✕ Clear** returns to the demo jam.

### Persistent library

Every analyzed song is **cached on disk** by the sidecar (under
`server/_ingest_cache/`) keyed by a stable id — `yt_<videoId>` for YouTube,
`file_<sha256>` for uploads — as `<id>.mp3` (audio) plus `<id>.json` (chords +
metadata). Re-analyzing the same song returns instantly instead of
re-downloading and re-running recognition, and the cache survives sidecar
restarts (it is rebuilt from disk).

Previously analyzed songs appear as clickable **Library** chips in the ingest
panel; clicking one reloads that song immediately. The sidecar exposes
`GET /api/library` (summaries) and `GET /api/song/<id>` (full cached analysis)
for this.

