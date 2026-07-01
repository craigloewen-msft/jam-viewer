//! Native server binary (the `ssr` build). Serves the hydrated Leptos app, the
//! `#[server]` API functions, and a range-capable audio route for cached songs.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{extract::Path, routing::get, Router};
    use jam_viewer::app::App;
    use jam_viewer::{server::ingest, shell};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Stream a cached song's audio with HTTP range support so <audio> can seek.
    async fn serve_audio(
        Path(id): Path<String>,
        req: axum::extract::Request,
    ) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;
        use tower::ServiceExt;
        use tower_http::services::ServeFile;

        match ingest::audio_path(&id) {
            Some(path) => match ServeFile::new(path).oneshot(req).await {
                Ok(res) => res.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            },
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }

    let app = Router::new()
        .route("/api/audio/{id}", get(serve_audio))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    log!("jam-viewer listening on http://{}", &addr);
    log!("ChordMini backend: {}", ingest::chordmini_url());
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // The binary is only meaningful in the server (ssr) build. cargo-leptos
    // compiles the client as a library (hydrate), so this is a no-op stub.
}
