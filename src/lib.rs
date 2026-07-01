//! Jam Viewer — a Leptos fullstack app.
//!
//! The UI (fretboard, timeline, transport) runs in the browser via hydration.
//! Song ingestion (YouTube / file upload), chord recognition proxying to a
//! ChordMini container, and the persistent song library are implemented as
//! `#[server]` functions in [`api`], so the whole thing is a single Rust
//! codebase — no separate sidecar process. ChordMini itself stays a separate
//! container, reached over HTTP by the server functions.

pub mod api;
pub mod app;
pub mod components;
pub mod ingest;
pub mod theory;

#[cfg(feature = "ssr")]
pub mod server;

#[cfg(feature = "ssr")]
use leptos::prelude::*;

/// The full HTML document shell rendered by the server. Hydration scripts and
/// the compiled stylesheet are injected here; `<App/>` is the live UI.
#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta
                    name="viewport"
                    content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
                />
                <title>"Jam Viewer — Guitar Scale Trainer"</title>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <link rel="stylesheet" id="leptos" href="/pkg/jam-viewer.css" />
            </head>
            <body>
                <app::App />
            </body>
        </html>
    }
}

/// WASM entry point: hydrate the server-rendered HTML.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
