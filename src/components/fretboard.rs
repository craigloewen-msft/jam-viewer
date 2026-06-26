use leptos::prelude::*;

use crate::theory::{
    caged_shapes, fret_marker, pitch_class, CagedShape, Section, FRET_MAX, FRET_MIN, NOTE_NAMES,
    STRINGS,
};

/// The guitar fretboard, drawn as a grid centered on fret 12.
///
/// The grid of cells is built **once** so the DOM nodes stay stable; only each
/// cell's class changes reactively as the chord changes. That lets CSS
/// transitions animate the differences cleanly instead of snapping.
///
/// Two modes:
/// - Default: layer coloring for the current chord — root, chord tone, the
///   current chord's scale, and the overall song scale (always shown).
/// - CAGED: the five movable major shapes (C-A-G-E-D) for the current chord
///   root, color-coded with connecting boundary boxes.
///
/// Everything is driven by `FRET_MIN`/`FRET_MAX`, so the visible window can be
/// retuned by editing only those two constants.
#[component]
pub fn Fretboard(current: Memo<Section>, key_pcs: Vec<u8>, caged: Memo<bool>) -> impl IntoView {
    // Number of visible fret columns, derived from the window so the grid (and
    // the CAGED overlay) stay correct at any range.
    let fret_count = (FRET_MAX - FRET_MIN + 1) as usize;
    let grid_cols = format!("44px repeat({}, minmax(0, 1fr))", fret_count);
    let board_style = format!("grid-template-columns: {}", grid_cols);
    let overlay_style = format!("grid-template-columns: {}", grid_cols);

    // CAGED placements for the current chord root, recomputed on chord change.
    let placements = Memo::new(move |_| caged_shapes(current.get().chord_root % 12));

    let mut cells = Vec::new();

    // Header row: corner + fret numbers with inlay markers (static).
    cells.push(view! { <div class="corner"></div> }.into_any());
    for fret in FRET_MIN..=FRET_MAX {
        let dots = fret_marker(fret);
        cells.push(
            view! {
                <div class="fret-num">
                    <span class="fret-digit">{fret}</span>
                    <span class="inlay">
                        {(0..dots).map(|_| view! { <i></i> }).collect_view()}
                    </span>
                </div>
            }
            .into_any(),
        );
    }

    // One row per string. Each cell keeps a fixed position/note; only its
    // class reacts to the current chord (and the CAGED toggle).
    for (string_idx, gs) in STRINGS.iter().enumerate() {
        cells.push(view! { <div class="string-label">{gs.label}</div> }.into_any());

        for fret in FRET_MIN..=FRET_MAX {
            let pc = pitch_class(gs.open_pc, fret);
            let name = NOTE_NAMES[pc as usize];
            // Whether this note belongs to the overall song scale is fixed.
            let in_song_scale = key_pcs.contains(&pc);

            let kind = move || {
                if caged.get() {
                    // Find which CAGED placement(s) cover this cell. The primary
                    // (lowest-position) shape sets the color; mark shared cells.
                    let found = placements.with(|ps| {
                        let mut best: Option<(CagedShape, bool)> = None;
                        let mut best_min = u8::MAX;
                        let mut count = 0u32;
                        for p in ps {
                            if let Some(c) = p
                                .cells
                                .iter()
                                .find(|c| c.string_index == string_idx && c.fret == fret)
                            {
                                count += 1;
                                if p.min_fret < best_min {
                                    best_min = p.min_fret;
                                    best = Some((p.shape, c.is_root));
                                }
                            }
                        }
                        best.map(|(sh, root)| (sh, root, count > 1))
                    });

                    match found {
                        Some((shape, is_root, shared)) => {
                            let mut c = format!("caged caged-{}", shape.slug());
                            if is_root {
                                c.push_str(" caged-root");
                            }
                            if shared {
                                c.push_str(" caged-shared");
                            }
                            c
                        }
                        None => "caged caged-off".to_string(),
                    }
                } else {
                    let s = current.get();
                    if pc == s.chord_root % 12 {
                        "root".to_string()
                    } else if s.chord_pcs().contains(&pc) {
                        "chord".to_string()
                    } else if s.scale_pcs().contains(&pc) {
                        "scale".to_string()
                    } else if in_song_scale {
                        "song".to_string()
                    } else {
                        "off".to_string()
                    }
                }
            };

            cells.push(
                view! {
                    <div class=move || format!("cell {}", kind())>
                        <span class="note">{name}</span>
                    </div>
                }
                .into_any(),
            );
        }
    }

    // Boundary boxes, one per visible shape placement. Positioned on the same
    // grid lines as the board's fret columns, so they clip to the window.
    let boxes = move || {
        if !caged.get() {
            return Vec::new();
        }
        placements
            .get()
            .into_iter()
            .map(|p| {
                let start = 2 + (p.min_fret - FRET_MIN) as usize;
                let end = 3 + (p.max_fret - FRET_MIN) as usize;
                let style = format!("grid-column: {} / {}; grid-row: 1 / 7", start, end);
                let slug = p.shape.slug();
                let letter = p.shape.letter();
                view! {
                    <div class=format!("caged-box caged-box-{}", slug) style=style>
                        <span class="caged-badge">{letter}</span>
                    </div>
                }
                .into_any()
            })
            .collect::<Vec<_>>()
    };

    view! {
        <section class="board-wrap">
            <div class="board-stack">
                <div class="board" style=board_style>{cells}</div>
                <div
                    class="caged-overlay"
                    class:show=move || caged.get()
                    style=overlay_style
                >
                    {boxes}
                </div>
            </div>

            <div class="legend" class:hidden=move || caged.get()>
                <span class="key root"><i></i>"Root"</span>
                <span class="key chord"><i></i>"Chord tone"</span>
                <span class="key scale"><i></i>"Chord scale"</span>
                <span class="key song-note"><i></i>"Song scale"</span>
            </div>

            <div class="legend caged-legend" class:hidden=move || !caged.get()>
                <span class="key caged-key-c"><i></i>"C shape"</span>
                <span class="key caged-key-a"><i></i>"A shape"</span>
                <span class="key caged-key-g"><i></i>"G shape"</span>
                <span class="key caged-key-e"><i></i>"E shape"</span>
                <span class="key caged-key-d"><i></i>"D shape"</span>
            </div>
        </section>
    }
}
