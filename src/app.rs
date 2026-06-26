use leptos::prelude::*;
use std::time::Duration;

use crate::components::fretboard::Fretboard;
use crate::components::timeline::Timeline;
use crate::theory::{demo_song, locate, section_ordinal, Section};

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

    let toggle_play = move |_| set_playing.update(|p| *p = !*p);
    let restart = move |_| set_total.set(0);

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
                <Fretboard current=current key_pcs=key_pcs/>
            </main>

            <footer class="transport">
                <button class="btn play" on:click=toggle_play>
                    {move || if playing.get() { "⏸ Pause" } else { "▶ Play" }}
                </button>
                <button class="btn" on:click=restart>"⏮ Restart"</button>

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
