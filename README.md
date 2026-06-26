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
| `src/app.rs`                  | Root component, beat timer, transport controls   |
| `src/theory.rs`               | Notes, chords, scales, fretboard math, demo song |
| `src/components/timeline.rs`  | Sliding chord timeline (also the header)         |
| `src/components/fretboard.rs` | Fretboard visualization                          |

The song is currently a hard-coded looping demo (`Am → F → C → Em → G7`) in the
key of C major. A continuously **sliding timeline** at the top doubles as the
header (key, current scale, beat counter) and shows the repeating chords coming
up; advancing to the next chord slides the strip smoothly rather than snapping.
Each chord on the timeline shows its letter name with its **Roman numeral** in
the key beneath it (e.g. in C major: `Am`→`vi`, `F`→`IV`, `C`→`I`, `Em`→`iii`,
`G7`→`V7`), so you can read each chord's place in the scale. The
fretboard shows four layers — chord root, chord tones, the current chord's scale,
and the overall **song scale** (always shown) — and the notes animate cleanly
between states as the chord changes. A **CAGED** toggle in the transport bar
swaps the layer view for the five movable major chord shapes (C-A-G-E-D) of the
current chord, color-coded with connecting boundary boxes. The fret window
(currently frets 7–17) is driven entirely by the `FRET_MIN`/`FRET_MAX` constants
in `src/theory.rs`, so it — and the CAGED overlay — can be retuned by editing
only those two values. Real music ingestion is a future step.
