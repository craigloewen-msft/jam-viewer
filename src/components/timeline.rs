use leptos::prelude::*;

use crate::theory::Section;

/// Width of one chord segment and the gap to the next, in pixels. The track is
/// positioned absolutely by ordinal, so these drive the slide geometry.
const SEG_W: f64 = 150.0;
const GAP: f64 = 12.0;
const STEP: f64 = SEG_W + GAP;

/// A horizontal, continuously-sliding timeline of the looping chords.
///
/// Each chord is a persistent element positioned at `ordinal * STEP`. The whole
/// track is translated by `-current_ordinal * STEP` with a CSS transition, so
/// advancing to the next chord slides the strip left smoothly instead of
/// snapping. The header carries the key, current scale and beat counter (this
/// replaces the old top banner).
#[component]
pub fn Timeline(
    sections: Vec<Section>,
    ordinal: Memo<usize>,
    beat_in: Memo<usize>,
    key_name: String,
) -> impl IntoView {
    let len = sections.len().max(1);
    let sections = StoredValue::new(sections);

    // Current section drives the header (scale name + beat total).
    let scale_name = move || sections.with_value(|s| s[ordinal.get() % len].scale_name());
    let beat_total = move || sections.with_value(|s| s[ordinal.get() % len].beats);

    // The window of ordinals near the screen. One behind so the outgoing chord
    // slides off cleanly, several ahead so you can see what's coming.
    let window = move || {
        let o = ordinal.get();
        (o.saturating_sub(1)..o + 9).collect::<Vec<usize>>()
    };

    let track_style =
        move || format!("transform: translateX(-{:.1}px)", ordinal.get() as f64 * STEP);

    view! {
        <section class="timeline">
            <div class="tl-head">
                <div class="tl-meta">
                    <span class="key-tag">"KEY"</span>
                    <span class="tl-key">{key_name}</span>
                    <span class="tl-scale">
                        <span class="scale-dot"></span>
                        {scale_name}
                    </span>
                </div>
                <div class="tl-beat">
                    <span class="tl-beat-num">{move || beat_in.get()}</span>
                    <span class="tl-beat-sep">"/"</span>
                    <span class="tl-beat-total">{beat_total}</span>
                    <span class="tl-beat-label">"BEATS"</span>
                </div>
            </div>

            <div class="tl-viewport">
                <div class="tl-track" style=track_style>
                    <For each=window key=|o| *o let:ord>
                        {
                            let section = sections.with_value(|s| s[ord % len]);
                            let name = section.chord_name();
                            let beats = section.beats;

                            let is_now = move || ord == ordinal.get();

                            let seg_class = move || {
                                let o = ordinal.get();
                                let mut c = String::from("tl-seg");
                                if ord == o {
                                    c.push_str(" now");
                                } else if ord < o {
                                    c.push_str(" past");
                                } else if ord == o + 1 {
                                    c.push_str(" next");
                                    if beat_in.get() >= beats {
                                        c.push_str(" incoming");
                                    }
                                }
                                c
                            };

                            let fill_style = move || {
                                let o = ordinal.get();
                                let pct = if ord < o {
                                    100.0
                                } else if ord == o {
                                    (beat_in.get() as f64 / beats.max(1) as f64 * 100.0)
                                        .min(100.0)
                                } else {
                                    0.0
                                };
                                format!("width:{:.1}%", pct)
                            };

                            let seg_style =
                                format!("left:{:.1}px;width:{:.1}px", ord as f64 * STEP, SEG_W);

                            view! {
                                <div class=seg_class style=seg_style>
                                    <div class="tl-fill" style=fill_style></div>
                                    <div class="tl-body">
                                        <span class="tl-name">{name}</span>
                                        <span class="tl-beats">{format!("{} beats", beats)}</span>
                                    </div>
                                    {move || is_now().then(|| view! { <span class="tl-tag">"NOW"</span> })}
                                </div>
                            }
                        }
                    </For>
                </div>
            </div>
        </section>
    }
}
