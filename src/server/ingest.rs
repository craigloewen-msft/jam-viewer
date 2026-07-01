//! Server-side song ingestion: the logic that used to live in the Python
//! `ingest_server.py` sidecar, reimplemented in Rust.
//!
//! Responsibilities:
//! * download YouTube audio with `yt-dlp` (+ ffmpeg),
//! * proxy audio to a ChordMini container for chord recognition,
//! * persist every analyzed song to disk (`<id>.json` + `<id>.<ext>`),
//! * serve the library listing and cached-song lookups.
//!
//! ChordMini stays a separate container; its base URL is read from the
//! `CHORDMINI_URL` env var (default `http://localhost:5001`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ingest::{build_result, parse_entries, ChordSpan, IngestResult, LibraryEntry};

/// ChordMini backend base URL (a separate container).
pub fn chordmini_url() -> String {
    std::env::var("CHORDMINI_URL")
        .unwrap_or_else(|_| "http://localhost:5001".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Directory holding the persistent song cache. Defaults to `server/_ingest_cache`
/// so existing cached songs keep working; override with `INGEST_CACHE`.
pub fn cache_dir() -> PathBuf {
    match std::env::var("INGEST_CACHE") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("server").join("_ingest_cache"),
    }
}

fn ensure_cache_dir() -> PathBuf {
    let dir = cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn meta_path(id: &str) -> PathBuf {
    cache_dir().join(format!("{id}.json"))
}

/// Path to the cached audio file for `id` (any non-`.json` file named `<id>.*`).
pub fn audio_path(id: &str) -> Option<PathBuf> {
    let dir = cache_dir();
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&format!("{id}.")) && !name.ends_with(".json") {
            return Some(dir.join(name.as_ref()));
        }
    }
    None
}

/// Cached analysis metadata, matching the on-disk JSON format.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct CacheMeta {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    chords: Vec<ChordSpan>,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    created_at: f64,
}

fn load_meta(id: &str) -> Option<CacheMeta> {
    let text = std::fs::read_to_string(meta_path(id)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_meta(meta: &CacheMeta) -> Result<(), String> {
    ensure_cache_dir();
    let text = serde_json::to_string(meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(&meta.id), text).map_err(|e| e.to_string())
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Turn cached metadata into the parsed [`IngestResult`] the UI plays.
fn result_from_meta(meta: &CacheMeta) -> Result<IngestResult, String> {
    let chords = parse_entries(meta.chords.clone());
    build_result(
        chords,
        format!("/api/audio/{}", meta.id),
        Some(meta.id.clone()),
        Some(if meta.title.is_empty() {
            meta.id.clone()
        } else {
            meta.title.clone()
        }),
    )
}

// --------------------------------------------------------------------------- //
// Public API (called by the `#[server]` functions)
// --------------------------------------------------------------------------- //

/// List previously analyzed songs, newest first.
pub fn list_library() -> Vec<LibraryEntry> {
    let dir = cache_dir();
    let mut songs: Vec<(f64, LibraryEntry)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let Some(meta) = load_meta(id) else { continue };
        if audio_path(id).is_none() {
            continue;
        }
        songs.push((
            meta.created_at,
            LibraryEntry {
                id: meta.id.clone(),
                title: if meta.title.is_empty() {
                    meta.id.clone()
                } else {
                    meta.title.clone()
                },
                source: meta.source.clone(),
                duration: meta.duration,
                chords: meta.chords.len(),
            },
        ));
    }
    songs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    songs.into_iter().map(|(_, s)| s).collect()
}

/// Load one cached song's full analysis by id.
pub fn load_song(id: &str) -> Result<IngestResult, String> {
    let meta = load_meta(id).ok_or_else(|| "Song not found in library".to_string())?;
    if audio_path(id).is_none() {
        return Err("Cached audio for this song is missing".to_string());
    }
    result_from_meta(&meta)
}

/// Recognize chords for an already-cached audio file, persist, and return.
async fn analyze_and_cache(
    audio: &Path,
    id: &str,
    title: &str,
    source: &str,
) -> Result<IngestResult, String> {
    let chords = recognize(audio).await?;
    let duration = chords.last().map(|c| c.end).unwrap_or(0.0);
    let meta = CacheMeta {
        id: id.to_string(),
        title: title.to_string(),
        source: source.to_string(),
        chords,
        duration,
        created_at: now_secs(),
    };
    save_meta(&meta)?;
    result_from_meta(&meta)
}

/// Ingest a YouTube URL: download audio, recognize, cache, return.
pub async fn ingest_youtube(url: &str) -> Result<IngestResult, String> {
    let url = url.trim();
    if !is_youtube_url(url) {
        return Err("Not a valid YouTube URL".to_string());
    }
    let id = youtube_song_id(url);

    if load_meta(&id).is_some() && audio_path(&id).is_some() {
        return load_song(&id);
    }

    let title = youtube_title(url).await.unwrap_or_default();
    let audio = download_audio(url, &id).await?;
    analyze_and_cache(&audio, &id, &title, "youtube").await
}

/// Ingest raw uploaded audio bytes: cache the file, recognize, cache meta.
pub async fn ingest_file_bytes(
    bytes: Vec<u8>,
    filename: &str,
) -> Result<IngestResult, String> {
    if bytes.is_empty() {
        return Err("Empty file".to_string());
    }
    let dir = ensure_cache_dir();

    use sha2::{Digest, Sha256};
    let digest = hex::encode(Sha256::digest(&bytes));
    let id = format!("file_{}", &digest[..16]);

    if load_meta(&id).is_some() && audio_path(&id).is_some() {
        return load_song(&id);
    }

    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "mp3".to_string());
    let audio = dir.join(format!("{id}.{ext}"));
    std::fs::write(&audio, &bytes).map_err(|e| format!("Could not save upload: {e}"))?;

    let title = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&id)
        .to_string();
    analyze_and_cache(&audio, &id, &title, "file").await
}

// --------------------------------------------------------------------------- //
// yt-dlp + ChordMini
// --------------------------------------------------------------------------- //

fn is_youtube_url(url: &str) -> bool {
    let u = url.to_lowercase();
    u.starts_with("http")
        && (u.contains("youtube.com/") || u.contains("youtu.be/") || u.contains("music.youtube.com/"))
}

/// Extract an 11-char YouTube video id, falling back to a URL hash.
fn youtube_song_id(url: &str) -> String {
    for marker in ["v=", "/shorts/", "youtu.be/", "/embed/"] {
        if let Some(pos) = url.find(marker) {
            let rest = &url[pos + marker.len()..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .take(11)
                .collect();
            if id.len() == 11 {
                return format!("yt_{id}");
            }
        }
    }
    use sha2::{Digest, Sha256};
    let digest = hex::encode(Sha256::digest(url.as_bytes()));
    format!("yt_{}", &digest[..16])
}

async fn youtube_title(url: &str) -> Option<String> {
    let out = tokio::process::Command::new("yt-dlp")
        .args(["--no-playlist", "--print", "title", "--skip-download", url])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let title = String::from_utf8_lossy(&out.stdout);
    title.lines().next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Download the best audio track as `<id>.mp3` and return its path.
async fn download_audio(url: &str, id: &str) -> Result<PathBuf, String> {
    let dir = ensure_cache_dir();
    let out_template = dir.join(format!("{id}.%(ext)s"));
    let status = tokio::process::Command::new("yt-dlp")
        .args([
            "-f", "bestaudio/best",
            "-x", "--audio-format", "mp3",
            "--audio-quality", "0",
            "--no-playlist",
            "-o",
        ])
        .arg(&out_template)
        .arg(url)
        .output()
        .await
        .map_err(|e| format!("yt-dlp failed to start (is it installed?): {e}"))?;
    if !status.status.success() {
        let err = String::from_utf8_lossy(&status.stderr);
        let tail: String = err.chars().rev().take(500).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(format!("yt-dlp download failed: {tail}"));
    }
    let mp3 = dir.join(format!("{id}.mp3"));
    if mp3.exists() {
        return Ok(mp3);
    }
    audio_path(id).ok_or_else(|| "Downloaded audio file not found".to_string())
}

/// ChordMini's `/api/recognize-chords` response.
#[derive(Debug, Deserialize)]
struct RecognizeResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    chords: Vec<ChordSpan>,
    #[serde(default)]
    error: Option<String>,
}

/// Forward an audio file to ChordMini and return its detected chord spans.
async fn recognize(path: &Path) -> Result<Vec<ChordSpan>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read audio: {e}"))?;
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("audio.mp3")
        .to_string();

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("audio/mpeg")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("detector", "chord-cnn-lstm");

    let url = format!("{}/api/recognize-chords", chordmini_url());
    let resp = reqwest::Client::new()
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Could not reach ChordMini at {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("ChordMini returned HTTP {}", resp.status()));
    }
    let parsed: RecognizeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid ChordMini response: {e}"))?;
    if !parsed.success {
        return Err(parsed.error.unwrap_or_else(|| "Chord recognition failed".to_string()));
    }
    Ok(parsed.chords)
}
