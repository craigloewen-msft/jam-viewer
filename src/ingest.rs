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

/// Response of `/api/recognize-chords` (and the sidecar's youtube endpoint).
#[derive(Debug, Deserialize)]
struct RecognizeResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    chords: Vec<ChordEntry>,
    #[serde(default)]
    error: Option<String>,
    /// Only present on the sidecar response: URL to stream the audio back.
    #[serde(default)]
    audio_url: Option<String>,
}

/// The fully-parsed result of ingesting a song.
#[derive(Clone, Debug, PartialEq)]
pub struct IngestResult {
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

/// Build an [`IngestResult`] from parsed chords plus a playback URL.
fn build_result(chords: Vec<TimedChord>, audio_url: String) -> Result<IngestResult, String> {
    if chords.is_empty() {
        return Err("No chords were detected in this audio.".to_string());
    }
    let (key_root, key_scale) = guess_key(&chords);
    let duration = chords.last().map(|c| c.end).unwrap_or(0.0);
    Ok(IngestResult {
        chords,
        key_root,
        key_scale,
        audio_url,
        duration,
    })
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
    build_result(parse_entries(parsed.chords), audio_url)
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
    // The sidecar returns a relative path like "/api/audio/<id>"; make absolute.
    let audio_url = if audio_path.starts_with("http") {
        audio_path
    } else {
        format!("{}{}", sidecar, audio_path)
    };
    build_result(parse_entries(parsed.chords), audio_url)
}
