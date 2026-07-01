use leptos::prelude::*;

use crate::theory::{
    fret_marker, pitch_class, scale_degree_name, scale_position_span, LabelMode, Section, FRET_MAX,
    FRET_MIN, NOTE_NAMES, STRINGS,
};

/// The guitar fretboard, drawn as a grid centered on fret 12.
///
/// The grid of cells is built **once** so the DOM nodes stay stable; only each
/// cell's class changes reactively as the chord changes. That lets CSS
/// transitions animate the differences cleanly instead of snapping.
///
/// Layers, strongest first: chord root, chord tone, the current chord's scale,
/// and the overall song scale (always shown). On top of those, notes outside the
/// active scale-position box are dimmed, and notes belonging to the *next*
/// chord get a target ring so the player can aim resolutions ahead.
#[component]
pub fn Fretboard(
    current: Memo<Section>,
    next: Memo<Section>,
    key_pcs: Vec<u8>,
    label_mode: ReadSignal<LabelMode>,
    pos_idx: ReadSignal<usize>,
) -> impl IntoView {
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
    // `kind` class reacts to the current chord.
    for gs in STRINGS.iter() {
        cells.push(view! { <div class="string-label">{gs.label}</div> }.into_any());

        for fret in FRET_MIN..=FRET_MAX {
            let pc = pitch_class(gs.open_pc, fret);
            let name = NOTE_NAMES[pc as usize];
            // Whether this note belongs to the overall song scale is fixed.
            let in_song_scale = key_pcs.contains(&pc);

            let kind = move || {
                let s = current.get();
                if pc == s.chord_root % 12 {
                    "root"
                } else if s.chord_pcs().contains(&pc) {
                    "chord"
                } else if s.scale_pcs().contains(&pc) {
                    "scale"
                } else if in_song_scale {
                    "song"
                } else {
                    "off"
                }
            };

            // Dimmed when this fret sits outside the active scale-position box.
            let dimmed = move || {
                let s = current.get();
                let (start, end) = scale_position_span(s.scale_root, s.scale_type, pos_idx.get());
                fret < start || fret > end
            };

            // Ringed when this note is a tone of the upcoming chord.
            let targeted = move || next.get().chord_pcs().contains(&pc);

            let cell_class = move || {
                let mut c = format!("cell {}", kind());
                if dimmed() {
                    c.push_str(" dim");
                }
                if targeted() {
                    c.push_str(" target");
                }
                c
            };

            // The note label switches between absolute name and scale degree.
            let label = move || match label_mode.get() {
                LabelMode::NoteName => name.to_string(),
                LabelMode::Degree => scale_degree_name(pc, current.get().scale_root).to_string(),
            };

            cells.push(
                view! {
                    <div class=cell_class>
                        <span class="note">{label}</span>
                    </div>
                }
                .into_any(),
            );
        }
    }

    view! {
        <section class="board-wrap">
            <div class="board">{cells}</div>

            <div class="legend">
                <span class="key root"><i></i>"Root"</span>
                <span class="key chord"><i></i>"Chord tone"</span>
                <span class="key scale"><i></i>"Chord scale"</span>
                <span class="key song-note"><i></i>"Song scale"</span>
                <span class="key target-note"><i></i>"Next chord"</span>
            </div>
        </section>
    }
}
