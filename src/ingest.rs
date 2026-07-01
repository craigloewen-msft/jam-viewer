//! Shared ingestion types and chord parsing, used by **both** the WASM client
//! and the native server. All network/disk work (yt-dlp, ChordMini, the
//! persistent cache) lives in server-only code under [`crate::server`] and is
//! exposed to the UI through the `#[server]` functions in [`crate::api`].

use serde::{Deserialize, Serialize};

use crate::theory::{guess_key, parse_chord_label, ScaleType, TimedChord};

/// One chord span as produced by ChordMini and stored in the cache (Harte
/// notation, e.g. `"E:min"`; `"N"` marks "no chord").
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChordSpan {
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub end: f64,
    #[serde(default)]
    pub chord: String,
}

/// One entry in the persistent song library (`get_library`).
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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

/// The fully-parsed result of ingesting a song, returned by the server
/// functions and consumed directly by the player UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IngestResult {
    /// Stable song id in the persistent cache.
    pub id: Option<String>,
    /// Human-readable title, if known.
    pub title: Option<String>,
    /// Timed chords, sorted by start time, with no-chord gaps dropped.
    pub chords: Vec<TimedChord>,
    /// Guessed tonic pitch class.
    pub key_root: u8,
    /// Guessed key scale (major / natural minor).
    pub key_scale: ScaleType,
    /// URL the `<audio>` element should play (relative `/api/audio/<id>`).
    pub audio_url: String,
    /// Total duration in seconds (end of last chord).
    pub duration: f64,
}

/// Convert raw chord spans into sorted [`TimedChord`]s, skipping "no chord"
/// markers and zero-length spans.
pub fn parse_entries(mut entries: Vec<ChordSpan>) -> Vec<TimedChord> {
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

/// Build an [`IngestResult`] from parsed chords, carrying through the cache id
/// and title and guessing the song key.
pub fn build_result(
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
