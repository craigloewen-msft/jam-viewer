use leptos::prelude::*;
use std::time::Duration;

use crate::components::fretboard::Fretboard;
use crate::components::timeline::Timeline;
use crate::theory::{demo_song, locate, section_ordinal, LabelMode, Section, POSITION_COUNT};

#[component]
pub fn App() -> impl IntoView {
    // The song is static for now; store it so reactive closures can read it.
    let song = StoredValue::new(demo_song());
    let key_name = song.with_value(|s| s.key_name());
    let key_root = song.with_value(|s| s.key_root);
    let key_pcs = song.with_value(|s| s.key_pcs());
    let sections = song.with_value(|s| s.sections.clone());

    // Transport state.
    let (playing, set_playing) = signal(true);
    let (bpm, set_bpm) = signal(100u32);
    let (total, set_total) = signal(0usize);

    // Fretboard view state: how notes are labelled, and which scale-position
    // box is currently highlighted.
    let (label_mode, set_label_mode) = signal(LabelMode::NoteName);
    let (pos_idx, set_pos_idx) = signal(0usize);

    // Drive the beat counter. Re-create the interval whenever play/pause or the
    // tempo changes, clearing the previous one.
    Effect::new(move |prev: Option<Option<IntervalHandle>>| {
        if let Some(Some(handle)) = prev {
            handle.clear();
        }
        if !playing.get() {
            return None;
        }
        let ms = (60_000.0 / bpm.get() as f64) as u64;
        set_interval_with_handle(
            move || set_total.update(|t| *t += 1),
            Duration::from_millis(ms.max(1)),
        )
        .ok()
    });

    // Position within the song.
    let position = Memo::new(move |_| song.with_value(|s| locate(&s.sections, total.get())));
    let section_idx = Memo::new(move |_| position.get().0);
    let beat_in = Memo::new(move |_| position.get().1);
    // Monotonic chord index across loops, for the continuous timeline slide.
    let ordinal = Memo::new(move |_| song.with_value(|s| section_ordinal(&s.sections, total.get())));
    let current: Memo<Section> =
        Memo::new(move |_| song.with_value(|s| s.sections[section_idx.get()]));
    // The chord coming up next, for fretboard targeting rings.
    let next: Memo<Section> = Memo::new(move |_| {
        song.with_value(|s| {
            let len = s.sections.len().max(1);
            s.sections[(section_idx.get() + 1) % len]
        })
    });

    let toggle_play = move |_| set_playing.update(|p| *p = !*p);
    let restart = move |_| set_total.set(0);
    let prev_pos =
        move |_| set_pos_idx.update(|p| *p = (*p + POSITION_COUNT - 1) % POSITION_COUNT);
    let next_pos = move |_| set_pos_idx.update(|p| *p = (*p + 1) % POSITION_COUNT);

    view! {
        <div class="app">
            <Timeline
                sections=sections
                ordinal=ordinal
                beat_in=beat_in
                key_name=key_name
                key_root=key_root
            />

            <main class="stage">
                <Fretboard
                    current=current
                    next=next
                    key_pcs=key_pcs
                    label_mode=label_mode
                    pos_idx=pos_idx
                />
            </main>

            <footer class="transport">
                <button class="btn play" on:click=toggle_play>
                    {move || if playing.get() { "⏸ Pause" } else { "▶ Play" }}
                </button>
                <button class="btn" on:click=restart>"⏮ Restart"</button>

                <div class="seg" role="group" aria-label="Note labels">
                    <button
                        class=move || {
                            let on = label_mode.get() == LabelMode::NoteName;
                            format!("seg-btn{}", if on { " on" } else { "" })
                        }
                        on:click=move |_| set_label_mode.set(LabelMode::NoteName)
                    >"Names"</button>
                    <button
                        class=move || {
                            let on = label_mode.get() == LabelMode::Degree;
                            format!("seg-btn{}", if on { " on" } else { "" })
                        }
                        on:click=move |_| set_label_mode.set(LabelMode::Degree)
                    >"Degrees"</button>
                </div>

                <div class="pos" role="group" aria-label="Scale position">
                    <button class="btn pos-step" on:click=prev_pos aria-label="Previous position">"‹"</button>
                    <span class="pos-label">
                        {move || format!("Pos {}/{}", pos_idx.get() + 1, POSITION_COUNT)}
                    </span>
                    <button class="btn pos-step" on:click=next_pos aria-label="Next position">"›"</button>
                </div>

                <div class="tempo">
                    <label>"Tempo"</label>
                    <input
                        type="range" min="50" max="200" step="1"
                        prop:value=move || bpm.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                set_bpm.set(v);
                            }
                        }
                    />
                    <span class="bpm">{move || format!("{} BPM", bpm.get())}</span>
                </div>
            </footer>
        </div>
    }
}
