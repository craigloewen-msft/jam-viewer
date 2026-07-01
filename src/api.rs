//! `#[server]` functions: the browser-facing API. These run on the server (in
//! the `ssr` build) and are called from the WASM client as plain async
//! functions. They replace the former Python sidecar's HTTP endpoints.

use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};

use crate::ingest::{IngestResult, LibraryEntry};

/// List previously analyzed songs from the persistent cache, newest first.
#[server]
pub async fn get_library() -> Result<Vec<LibraryEntry>, ServerFnError> {
    Ok(crate::server::ingest::list_library())
}

/// Load one cached song's full analysis by id.
#[server]
pub async fn load_song(id: String) -> Result<IngestResult, ServerFnError> {
    crate::server::ingest::load_song(&id).map_err(ServerFnError::new)
}

/// Ingest a YouTube URL: download, recognize chords, cache, and return.
#[server]
pub async fn ingest_youtube(url: String) -> Result<IngestResult, ServerFnError> {
    crate::server::ingest::ingest_youtube(&url)
        .await
        .map_err(ServerFnError::new)
}

/// Ingest an uploaded audio file (multipart): cache, recognize, return.
#[server(input = MultipartFormData)]
pub async fn ingest_file(data: MultipartData) -> Result<IngestResult, ServerFnError> {
    let mut data = data.into_inner().unwrap();

    let mut filename = String::from("upload.mp3");
    let mut bytes: Vec<u8> = Vec::new();
    while let Ok(Some(mut field)) = data.next_field().await {
        if let Some(name) = field.file_name() {
            filename = name.to_string();
        }
        while let Ok(Some(chunk)) = field.chunk().await {
            bytes.extend_from_slice(&chunk);
        }
    }

    crate::server::ingest::ingest_file_bytes(bytes, &filename)
        .await
        .map_err(ServerFnError::new)
}
