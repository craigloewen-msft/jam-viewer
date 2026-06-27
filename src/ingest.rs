//! Song ingestion: send audio (an uploaded file or a YouTube URL) to the
//! `ingest_server.py` sidecar, which runs chord recognition via the self-hosted
//! ChordMini container and (for YouTube) downloads the audio with `yt-dlp`.
//! The parsed, timed chords drive real-audio-synced highlighting in the app.
//!
//! Both transports go through the sidecar because it is the CORS-open entry
//! point: the ChordMini container itself does not emit CORS headers, so a
//! browser cannot call it directly. For file uploads the browser still plays
//! the audio locally via an object URL; only the bytes needed for analysis are
//! proxied. For YouTube the sidecar also serves the downloaded audio back.

use serde::Deserialize;
use web_sys::{File, FormData};

use crate::theory::{guess_key, parse_chord_label, ScaleType, TimedChord};

/// Default URL of the ingest sidecar (handles both file and YouTube paths).
pub const DEFAULT_SIDECAR_URL: &str = "http://localhost:5002";

/// One chord span as returned by ChordMini (Harte notation, e.g. `"E:min"`).
#[derive(Debug, Deserialize)]
struct ChordEntry {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    chord: String,
}

/// Response of the sidecar's ingest / song endpoints.
#[derive(Debug, Deserialize)]
struct RecognizeResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    chords: Vec<ChordEntry>,
    #[serde(default)]
    error: Option<String>,
    /// Stable song id in the persistent cache (e.g. `yt_<videoId>`).
    #[serde(default)]
    id: Option<String>,
    /// Human-readable title (YouTube title or uploaded filename).
    #[serde(default)]
    title: Option<String>,
    /// URL to stream the cached audio back (relative `/api/audio/<id>`).
    #[serde(default)]
    audio_url: Option<String>,
}

/// One entry in the persistent song library (`GET /api/library`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LibraryEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub duration: f64,
    /// Number of detected chord spans.
    #[serde(default)]
    pub chords: usize,
}

#[derive(Debug, Deserialize)]
struct LibraryResponse {
    #[serde(default)]
    songs: Vec<LibraryEntry>,
}

/// The fully-parsed result of ingesting a song.
#[derive(Clone, Debug, PartialEq)]
pub struct IngestResult {
    /// Stable song id in the persistent cache, if known.
    pub id: Option<String>,
    /// Human-readable title, if known.
    pub title: Option<String>,
    /// Timed chords, sorted by start time, with no-chord gaps dropped.
    pub chords: Vec<TimedChord>,
    /// Guessed tonic pitch class.
    pub key_root: u8,
    /// Guessed key scale (major / natural minor).
    pub key_scale: ScaleType,
    /// URL the `<audio>` element should play.
    pub audio_url: String,
    /// Total duration in seconds (end of last chord).
    pub duration: f64,
}

/// Convert raw chord entries into sorted [`TimedChord`]s, skipping "no chord"
/// markers and zero-length spans.
fn parse_entries(mut entries: Vec<ChordEntry>) -> Vec<TimedChord> {
    entries.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::new();
    for e in entries {
        if e.end <= e.start {
            continue;
        }
        if let Some((root, quality)) = parse_chord_label(&e.chord) {
            out.push(TimedChord {
                chord_root: root,
                chord_quality: quality,
                start: e.start,
                end: e.end,
            });
        }
    }
    out
}

/// Build an [`IngestResult`], carrying through the cache id and title.
fn build_result_with(
    chords: Vec<TimedChord>,
    audio_url: String,
    id: Option<String>,
    title: Option<String>,
) -> Result<IngestResult, String> {
    if chords.is_empty() {
        return Err("No chords were detected in this audio.".to_string());
    }
    let (key_root, key_scale) = guess_key(&chords);
    let duration = chords.last().map(|c| c.end).unwrap_or(0.0);
    Ok(IngestResult {
        id,
        title,
        chords,
        key_root,
        key_scale,
        audio_url,
        duration,
    })
}

/// Resolve an audio URL returned by the sidecar (often relative) to an absolute
/// URL the `<audio>` element can load.
fn absolute_audio_url(sidecar: &str, audio_path: &str) -> String {
    if audio_path.starts_with("http") {
        audio_path.to_string()
    } else {
        format!("{}{}", sidecar.trim_end_matches('/'), audio_path)
    }
}

/// Recognize chords in an uploaded file by POSTing it to the ingest sidecar
/// (which proxies to ChordMini). `audio_url` is the browser object URL used for
/// playback — the audio itself never leaves the browser for playback purposes.
pub async fn recognize_file(
    sidecar_url: &str,
    file: &File,
    audio_url: String,
) -> Result<IngestResult, String> {
    let form = FormData::new().map_err(|_| "Failed to create form data".to_string())?;
    form.append_with_blob("file", file)
        .map_err(|_| "Failed to attach file".to_string())?;

    let url = format!("{}/api/ingest-file", sidecar_url.trim_end_matches('/'));
    let resp = gloo_net::http::Request::post(&url)
        .body(form)
        .map_err(|e| format!("Request build error: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Could not reach ingest server at {url}: {e}"))?;

    let parsed: RecognizeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid server response: {e}"))?;

    if !parsed.success {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Server reported failure".to_string()));
    }
    build_result_with(parse_entries(parsed.chords), audio_url, parsed.id, parsed.title)
}

/// Ingest a YouTube URL via the sidecar: it downloads the audio, runs chord
/// recognition, and returns both the chords and a URL to play the audio.
pub async fn ingest_youtube(sidecar_url: &str, youtube_url: &str) -> Result<IngestResult, String> {
    let sidecar = sidecar_url.trim_end_matches('/');
    let url = format!("{}/api/ingest-youtube", sidecar);
    let resp = gloo_net::http::Request::post(&url)
        .json(&serde_json::json!({ "url": youtube_url }))
        .map_err(|e| format!("Request build error: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Could not reach ingest sidecar at {url}: {e}"))?;

    let parsed: RecognizeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid sidecar response: {e}"))?;

    if !parsed.success {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Sidecar reported failure".to_string()));
    }
    let audio_path = parsed
        .audio_url
        .clone()
        .ok_or_else(|| "Sidecar did not return an audio URL".to_string())?;
    let audio_url = absolute_audio_url(sidecar, &audio_path);
    build_result_with(parse_entries(parsed.chords), audio_url, parsed.id, parsed.title)
}

/// Fetch the list of previously analyzed songs from the sidecar's persistent
/// library (`GET /api/library`), newest first.
pub async fn fetch_library(sidecar_url: &str) -> Result<Vec<LibraryEntry>, String> {
    let sidecar = sidecar_url.trim_end_matches('/');
    let url = format!("{}/api/library", sidecar);
    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not reach ingest sidecar at {url}: {e}"))?;
    let parsed: LibraryResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid library response: {e}"))?;
    Ok(parsed.songs)
}

/// Load a cached song's full analysis by id (`GET /api/song/<id>`).
pub async fn load_song(sidecar_url: &str, id: &str) -> Result<IngestResult, String> {
    let sidecar = sidecar_url.trim_end_matches('/');
    let url = format!("{}/api/song/{}", sidecar, id);
    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not reach ingest sidecar at {url}: {e}"))?;
    let parsed: RecognizeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid sidecar response: {e}"))?;
    if !parsed.success {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Song not found in library".to_string()));
    }
    let audio_path = parsed
        .audio_url
        .clone()
        .ok_or_else(|| "Sidecar did not return an audio URL".to_string())?;
    let audio_url = absolute_audio_url(sidecar, &audio_path);
    build_result_with(parse_entries(parsed.chords), audio_url, parsed.id, parsed.title)
}
