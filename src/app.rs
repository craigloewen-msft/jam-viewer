use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use std::time::Duration;

use web_sys::{FormData, HtmlInputElement};

use crate::api::{get_library, ingest_file, ingest_youtube, load_song};
use crate::components::fretboard::Fretboard;
use crate::components::timeline::Timeline;
use crate::ingest::{IngestResult, LibraryEntry};
use crate::theory::{
    demo_song, locate, section_ordinal, LabelMode, Section, NOTE_NAMES, POSITION_COUNT,
};

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
    let youtube_url = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let status = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    // Previously analyzed songs from the persistent cache (server-side).
    let library = RwSignal::new(Vec::<LibraryEntry>::new());
    // Whether a library reload is in flight, and the last reload error (if any),
    // so the Library panel can show feedback instead of silently staying empty.
    let library_loading = RwSignal::new(false);
    let library_error = RwSignal::new(None::<String>);
    // Library panel is collapsed by default; `library_query` filters the chips.
    let library_open = RwSignal::new(false);
    let library_query = RwSignal::new(String::new());

    let file_ref = NodeRef::<leptos::html::Input>::new();

    // Reload the library listing via the `get_library` server function.
    let refresh_library = move || {
        library_loading.set(true);
        spawn_local(async move {
            match get_library().await {
                Ok(songs) => {
                    library.set(songs);
                    library_error.set(None);
                }
                Err(e) => library_error.set(Some(e.to_string())),
            }
            library_loading.set(false);
        });
    };
    // Populate the library on first render.
    Effect::new(move |_| {
        refresh_library();
    });

    // Analyze an uploaded file via the `ingest_file` server function. The file
    // is streamed to the server as multipart form data, cached, and analyzed.
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
        let form = FormData::new().unwrap();
        if form.append_with_blob("file", &file).is_err() {
            error.set(Some("Could not attach the selected file.".into()));
            return;
        }
        loading.set(true);
        status.set("Analyzing audio… (this can take a moment)".into());
        spawn_local(async move {
            match ingest_file(form.into()).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                    refresh_library();
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    // Analyze a YouTube URL via the `ingest_youtube` server function.
    let analyze_youtube = move |_| {
        error.set(None);
        let url = youtube_url.get();
        if url.trim().is_empty() {
            error.set(Some("Paste a YouTube URL first.".into()));
            return;
        }
        loading.set(true);
        status.set("Downloading & analyzing from YouTube… (this can take a minute)".into());
        spawn_local(async move {
            match ingest_youtube(url).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                    refresh_library();
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    // Load a previously analyzed song from the cache (instant — no re-analysis).
    let load_from_library = move |id: String, title: String| {
        error.set(None);
        loading.set(true);
        status.set(format!("Loading “{title}” from library…"));
        spawn_local(async move {
            match load_song(id).await {
                Ok(result) => {
                    status.set(format!("Loaded {} chords.", result.chords.len()));
                    ingested.set(Some(result));
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    let clear = move |_| {
        ingested.set(None);
        status.set(String::new());
        error.set(None);
    };

    // Songs filtered by the (case-insensitive) search box.
    let filtered = move || {
        let q = library_query.get().to_lowercase();
        library
            .get()
            .into_iter()
            .filter(|s| {
                q.is_empty()
                    || s.title.to_lowercase().contains(&q)
                    || s.id.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
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
                    {move || ingested.get().is_some().then(|| view! {
                        <button class="btn" on:click=clear>"✕ Clear"</button>
                    })}
                </div>
            </div>

            <div class="ingest-status">
                {move || loading.get().then(|| view! { <span class="spinner"></span> })}
                {move || match error.get() {
                    Some(e) => view! { <span class="ingest-error">{e}</span> }.into_any(),
                    None => view! { <span class="ingest-msg">{move || status.get()}</span> }.into_any(),
                }}
            </div>

            <div class="ingest-library" class:open=move || library_open.get()>
                <div class="library-head">
                    <button
                        class="library-toggle"
                        aria-expanded=move || library_open.get().to_string()
                        on:click=move |_| library_open.update(|o| *o = !*o)
                    >
                        <span class="library-caret">
                            {move || if library_open.get() { "▾" } else { "▸" }}
                        </span>
                        <span class="library-label">"Library"</span>
                        <span class="library-count">
                            {move || format!("({})", library.get().len())}
                        </span>
                    </button>
                    {move || library_open.get().then(|| view! {
                        <button
                            class="btn ghost library-refresh"
                            title="Reload saved songs"
                            prop:disabled=move || library_loading.get()
                            on:click=move |_| refresh_library()
                        >
                            {move || if library_loading.get() { "Refreshing…" } else { "↻ Refresh" }}
                        </button>
                    })}
                </div>

                {move || library_open.get().then(|| view! {
                    <div class="library-body">
                        <input
                            class="library-search"
                            type="search"
                            placeholder="Search saved songs…"
                            prop:value=move || library_query.get()
                            on:input=move |ev| library_query.set(event_target_value(&ev))
                        />
                        {move || {
                            let songs = filtered();
                            if !songs.is_empty() {
                                view! {
                                    <div class="library-chips">
                                        {songs.into_iter().map(|s| {
                                            let id = s.id.clone();
                                            let title = if s.title.is_empty() { s.id.clone() } else { s.title.clone() };
                                            let label = title.clone();
                                            let icon = if s.source == "youtube" { "▶" } else { "♪" };
                                            let mins = (s.duration / 60.0) as u32;
                                            let secs = (s.duration % 60.0) as u32;
                                            let tip = format!("{} chords · {}:{:02}", s.chords, mins, secs);
                                            view! {
                                                <button
                                                    class="library-chip"
                                                    title=tip
                                                    prop:disabled=move || loading.get()
                                                    on:click=move |_| load_from_library(id.clone(), title.clone())
                                                >
                                                    <span class="chip-icon">{icon}</span>
                                                    <span class="chip-title">{label}</span>
                                                </button>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            } else if let Some(err) = library_error.get() {
                                view! {
                                    <span class="library-empty error">
                                        {format!("Couldn't load the library: {err}")}
                                    </span>
                                }.into_any()
                            } else if library_loading.get() {
                                view! { <span class="library-empty">"Loading saved songs…"</span> }.into_any()
                            } else if !library_query.get().is_empty() {
                                view! { <span class="library-empty">"No songs match your search."</span> }.into_any()
                            } else {
                                view! {
                                    <span class="library-empty">
                                        "No saved songs yet — analyze a file or YouTube URL and it will be saved here for instant reloading."
                                    </span>
                                }.into_any()
                            }
                        }}
                    </div>
                })}
            </div>
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

    let position = Memo::new(move |_| song.with_value(|s| locate(&s.sections, total.get())));
    let section_idx = Memo::new(move |_| position.get().0);
    let beat_in = Memo::new(move |_| position.get().1);
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
    let toggle_caged = move |_| set_caged.update(|c| *c = !*c);
    let prev_pos =
        move |_| set_pos_idx.update(|p| *p = (*p + POSITION_COUNT - 1) % POSITION_COUNT);
    let next_pos = move |_| set_pos_idx.update(|p| *p = (*p + 1) % POSITION_COUNT);

    view! {
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
                caged=caged
                label_mode=label_mode
                pos_idx=pos_idx
            />
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

    // Fretboard view state: how notes are labelled, and which scale-position
    // box is currently highlighted.
    let (label_mode, set_label_mode) = signal(LabelMode::NoteName);
    let (pos_idx, set_pos_idx) = signal(0usize);

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
    // The chord coming up next, for fretboard targeting rings.
    let next: Memo<Section> =
        Memo::new(move |_| sections_sv.with_value(|s| s[(current_idx.get() + 1) % n]));

    let chord_label = move || sections_sv.with_value(|s| s[current_idx.get() % n].chord_name());
    let toggle_caged = move |_| set_caged.update(|c| *c = !*c);
    let prev_pos =
        move |_| set_pos_idx.update(|p| *p = (*p + POSITION_COUNT - 1) % POSITION_COUNT);
    let next_pos = move |_| set_pos_idx.update(|p| *p = (*p + 1) % POSITION_COUNT);

    view! {
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
                caged=caged
                label_mode=label_mode
                pos_idx=pos_idx
            />
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
        </footer>
    }
}
