use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use std::time::Duration;

use web_sys::{HtmlInputElement, Url};

use crate::components::fretboard::Fretboard;
use crate::components::timeline::Timeline;
use crate::ingest::{
    fetch_library, ingest_youtube, load_song, recognize_file, IngestResult, LibraryEntry,
    DEFAULT_SIDECAR_URL,
};
use crate::theory::{demo_song, locate, section_ordinal, Section, NOTE_NAMES};

/// Root component: an ingestion panel on top, and below it either the built-in
/// demo jam (default) or an audio-synced player for an ingested real song.
#[component]
pub fn App() -> impl IntoView {
    // The ingested song, if any. `None` shows the demo.
    let ingested = RwSignal::new(None::<IngestResult>);

    view! {
        <div class="app">
            <IngestPanel ingested=ingested/>
            {move || match ingested.get() {
                Some(result) => view! { <SongPlayer result=result/> }.into_any(),
                None => view! { <DemoPlayer/> }.into_any(),
            }}
        </div>
    }
}

/// The upload / YouTube ingestion controls. On success it writes an
/// [`IngestResult`] into `ingested`, which swaps the view to the song player.
#[component]
fn IngestPanel(ingested: RwSignal<Option<IngestResult>>) -> impl IntoView {
    let server_url = RwSignal::new(DEFAULT_SIDECAR_URL.to_string());
    let youtube_url = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let status = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let show_advanced = RwSignal::new(false);
    // Previously analyzed songs from the sidecar's persistent cache.
    let library = RwSignal::new(Vec::<LibraryEntry>::new());

    let file_ref = NodeRef::<leptos::html::Input>::new();

    // Reload the library listing from the sidecar.
    let refresh_library = move || {
        let backend = server_url.get_untracked();
        spawn_local(async move {
            if let Ok(songs) = fetch_library(&backend).await {
                library.set(songs);
            }
        });
    };
    // Populate the library on first render.
    Effect::new(move |_| {
        refresh_library();
    });

    // Analyze an uploaded file: POST straight to the ChordMini container and
    // play it back from a browser object URL.
    let analyze_file = move |_| {
        error.set(None);
        let Some(input) = file_ref.get() else { return };
        let input: HtmlInputElement = input;
        let Some(files) = input.files() else {
            error.set(Some("Choose an audio file first.".into()));
            return;
        };
        let Some(file) = files.get(0) else {
            error.set(Some("Choose an audio file first.".into()));
            return;
        };
        let object_url = match Url::create_object_url_with_blob(&file) {
            Ok(u) => u,
            Err(_) => {
                error.set(Some("Could not create a playback URL for this file.".into()));
                return;
            }
        };
        let backend = server_url.get();
        loading.set(true);
        status.set("Analyzing audio… (this can take a moment)".into());
        spawn_local(async move {
            match recognize_file(&backend, &file, object_url).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                    refresh_library();
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    // Analyze a YouTube URL via the sidecar.
    let analyze_youtube = move |_| {
        error.set(None);
        let url = youtube_url.get();
        if url.trim().is_empty() {
            error.set(Some("Paste a YouTube URL first.".into()));
            return;
        }
        let sidecar = server_url.get();
        loading.set(true);
        status.set("Downloading & analyzing from YouTube… (this can take a minute)".into());
        spawn_local(async move {
            match ingest_youtube(&sidecar, &url).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                    refresh_library();
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    // Load a previously analyzed song from the library cache (instant — no
    // re-download or re-analysis).
    let load_from_library = move |id: String, title: String| {
        error.set(None);
        let backend = server_url.get();
        loading.set(true);
        status.set(format!("Loading “{title}” from library…"));
        spawn_local(async move {
            match load_song(&backend, &id).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let clear = move |_| {
        ingested.set(None);
        status.set(String::new());
        error.set(None);
    };

    view! {
        <section class="ingest">
            <div class="ingest-row">
                <div class="ingest-group">
                    <label class="ingest-label">"Audio file"</label>
                    <input
                        class="ingest-file"
                        type="file"
                        accept="audio/*"
                        node_ref=file_ref
                        prop:disabled=move || loading.get()
                    />
                    <button
                        class="btn"
                        on:click=analyze_file
                        prop:disabled=move || loading.get()
                    >
                        "Analyze file"
                    </button>
                </div>

                <div class="ingest-sep">"or"</div>

                <div class="ingest-group grow">
                    <label class="ingest-label">"YouTube URL"</label>
                    <input
                        class="ingest-url"
                        type="text"
                        placeholder="https://www.youtube.com/watch?v=…"
                        prop:value=move || youtube_url.get()
                        on:input=move |ev| youtube_url.set(event_target_value(&ev))
                        prop:disabled=move || loading.get()
                    />
                    <button
                        class="btn"
                        on:click=analyze_youtube
                        prop:disabled=move || loading.get()
                    >
                        "Analyze YouTube"
                    </button>
                </div>

                <div class="ingest-group">
                    <button
                        class="btn ghost"
                        on:click=move |_| show_advanced.update(|v| *v = !*v)
                    >
                        "⚙"
                    </button>
                    {move || ingested.get().is_some().then(|| view! {
                        <button class="btn" on:click=clear>"✕ Clear"</button>
                    })}
                </div>
            </div>

            {move || show_advanced.get().then(|| view! {
                <div class="ingest-row advanced">
                    <div class="ingest-group grow">
                        <label class="ingest-label">"Server URL"</label>
                        <input
                            class="ingest-url"
                            type="text"
                            prop:value=move || server_url.get()
                            on:input=move |ev| server_url.set(event_target_value(&ev))
                        />
                    </div>
                </div>
            })}

            <div class="ingest-status">
                {move || loading.get().then(|| view! { <span class="spinner"></span> })}
                {move || match error.get() {
                    Some(e) => view! { <span class="ingest-error">{e}</span> }.into_any(),
                    None => view! { <span class="ingest-msg">{move || status.get()}</span> }.into_any(),
                }}
            </div>

            {move || {
                let songs = library.get();
                (!songs.is_empty()).then(|| {
                    view! {
                        <div class="ingest-library">
                            <span class="library-label">"Library"</span>
                            <div class="library-chips">
                                {songs.into_iter().map(|s| {
                                    let id = s.id.clone();
                                    let title = if s.title.is_empty() { s.id.clone() } else { s.title.clone() };
                                    let label = title.clone();
                                    let icon = if s.source == "youtube" { "▶" } else { "♪" };
                                    let meta = format!("{} chords", s.chords);
                                    view! {
                                        <button
                                            class="library-chip"
                                            title=meta
                                            prop:disabled=move || loading.get()
                                            on:click=move |_| load_from_library(id.clone(), title.clone())
                                        >
                                            <span class="chip-icon">{icon}</span>
                                            <span class="chip-title">{label}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                })
            }}
        </section>
    }
}

/// The built-in looping demo jam driven by a beat clock (the original behavior).
#[component]
fn DemoPlayer() -> impl IntoView {
    let song = StoredValue::new(demo_song());
    let key_name = song.with_value(|s| s.key_name());
    let key_root = song.with_value(|s| s.key_root);
    let key_pcs = song.with_value(|s| s.key_pcs());
    let sections = song.with_value(|s| s.sections.clone());

    let (playing, set_playing) = signal(true);
    let (bpm, set_bpm) = signal(100u32);
    let (total, set_total) = signal(0usize);
    let (caged, set_caged) = signal(false);
    let caged = Memo::new(move |_| caged.get());

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

    let position = Memo::new(move |_| song.with_value(|s| locate(&s.sections, total.get())));
    let section_idx = Memo::new(move |_| position.get().0);
    let beat_in = Memo::new(move |_| position.get().1);
    let ordinal = Memo::new(move |_| song.with_value(|s| section_ordinal(&s.sections, total.get())));
    let current: Memo<Section> =
        Memo::new(move |_| song.with_value(|s| s.sections[section_idx.get()]));

    let toggle_play = move |_| set_playing.update(|p| *p = !*p);
    let restart = move |_| set_total.set(0);
    let toggle_caged = move |_| set_caged.update(|c| *c = !*c);

    view! {
        <Timeline
            sections=sections
            ordinal=ordinal
            beat_in=beat_in
            key_name=key_name
            key_root=key_root
        />

        <main class="stage">
            <Fretboard current=current key_pcs=key_pcs caged=caged/>
        </main>

        <footer class="transport">
            <button class="btn play" on:click=toggle_play>
                {move || if playing.get() { "⏸ Pause" } else { "▶ Play" }}
            </button>
            <button class="btn" on:click=restart>"⏮ Restart"</button>
            <button
                class="btn caged-toggle"
                class:is-on=move || caged.get()
                on:click=toggle_caged
            >
                {move || if caged.get() { "CAGED ✓" } else { "CAGED" }}
            </button>

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
    }
}

/// An audio-synced player for an ingested real song: a native `<audio>` element
/// drives the highlighted chord on the fretboard and the sliding timeline.
#[component]
fn SongPlayer(result: IngestResult) -> impl IntoView {
    let key_root = result.key_root;
    let key_scale = result.key_scale;
    let key_name = format!("{} {}", NOTE_NAMES[key_root as usize % 12], key_scale.label());
    let key_pcs: Vec<u8> = key_scale
        .intervals()
        .iter()
        .map(|i| (key_root + i) % 12)
        .collect();

    let sections: Vec<Section> = result.chords.iter().map(|c| c.to_section()).collect();
    let bounds: Vec<(f64, f64)> = result.chords.iter().map(|c| (c.start, c.end)).collect();
    let n = sections.len().max(1);
    let audio_url = result.audio_url.clone();

    let sections_sv = StoredValue::new(sections.clone());
    let bounds_sv = StoredValue::new(bounds);

    let (caged, set_caged) = signal(false);
    let caged = Memo::new(move |_| caged.get());

    // Playback time of the audio element, polled on an interval for smooth sync.
    let (audio_time, set_audio_time) = signal(0.0f64);
    let audio_ref = NodeRef::<leptos::html::Audio>::new();

    Effect::new(move |_| {
        let handle = set_interval_with_handle(
            move || {
                if let Some(el) = audio_ref.get_untracked() {
                    set_audio_time.set(el.current_time());
                }
            },
            Duration::from_millis(60),
        )
        .ok();
        on_cleanup(move || {
            if let Some(h) = handle {
                h.clear();
            }
        });
    });

    // Index of the chord that contains the current playback time.
    let current_idx = Memo::new(move |_| {
        let t = audio_time.get();
        bounds_sv.with_value(|b| {
            match b.binary_search_by(|&(s, e)| {
                if t < s {
                    Ordering::Greater
                } else if t >= e {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            }) {
                Ok(i) => i,
                Err(i) => i.min(b.len().saturating_sub(1)),
            }
        })
    });

    let ordinal = Memo::new(move |_| current_idx.get());
    let beat_in = Memo::new(move |_| {
        let t = audio_time.get();
        let i = current_idx.get();
        bounds_sv.with_value(|b| {
            let (s, _) = b.get(i).copied().unwrap_or((0.0, 0.0));
            ((t - s).floor().max(0.0) as usize) + 1
        })
    });
    let current: Memo<Section> =
        Memo::new(move |_| sections_sv.with_value(|s| s[current_idx.get() % n]));

    let chord_label = move || sections_sv.with_value(|s| s[current_idx.get() % n].chord_name());
    let toggle_caged = move |_| set_caged.update(|c| *c = !*c);

    view! {
        <Timeline
            sections=sections
            ordinal=ordinal
            beat_in=beat_in
            key_name=key_name
            key_root=key_root
        />

        <main class="stage">
            <Fretboard current=current key_pcs=key_pcs caged=caged/>
        </main>

        <footer class="transport song">
            <audio
                class="audio-player"
                node_ref=audio_ref
                src=audio_url
                controls=true
                preload="auto"
            ></audio>
            <div class="now-chord">
                <span class="now-label">"NOW"</span>
                <span class="now-name">{chord_label}</span>
            </div>
            <button
                class="btn caged-toggle"
                class:is-on=move || caged.get()
                on:click=toggle_caged
            >
                {move || if caged.get() { "CAGED ✓" } else { "CAGED" }}
            </button>
        </footer>
    }
}
