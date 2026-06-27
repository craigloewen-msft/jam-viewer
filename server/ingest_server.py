#!/usr/bin/env python3
"""Ingest sidecar for jam-viewer.

The ChordMini container does not emit CORS headers, so the browser can't call it
directly. This small Flask app is the CORS-open entry point: it proxies audio to
ChordMini for chord recognition and, for YouTube URLs, downloads the audio with
``yt-dlp`` and serves it back for playback.

Analyzed songs are **cached persistently** on disk so the same song can be
re-accessed instantly (no re-download / re-analysis) and browsed as a library.
Each song is keyed by a stable id (``yt_<videoId>`` for YouTube, ``file_<hash>``
for uploads) and stored as ``<id>.<ext>`` (audio) + ``<id>.json`` (metadata).

Endpoints
---------
POST /api/ingest-youtube   body: {"url": "<youtube url>"}
POST /api/ingest-file      multipart: file=<audio>
    -> {"success": true, "id", "title", "source", "chords": [...],
        "duration": <seconds>, "audio_url": "/api/audio/<id>"}
GET  /api/library          -> {"songs": [ {id, title, source, duration, chords}, ... ]}
GET  /api/song/<id>        -> cached analysis for one song (same shape as ingest)
GET  /api/audio/<id>       streams the cached audio (supports range requests)
GET  /                     health check

Environment
-----------
CHORDMINI_URL   ChordMini backend base URL (default http://localhost:5001)
INGEST_PORT     Port to listen on (default 5002)
INGEST_CACHE    Directory for the persistent song cache (default ./_ingest_cache)
"""

import hashlib
import json
import os
import re
import subprocess
import time

import requests
from flask import Flask, abort, jsonify, request, send_file
from flask_cors import CORS

CHORDMINI_URL = os.environ.get("CHORDMINI_URL", "http://localhost:5001").rstrip("/")
PORT = int(os.environ.get("INGEST_PORT", "5002"))
CACHE_DIR = os.environ.get(
    "INGEST_CACHE", os.path.join(os.path.dirname(os.path.abspath(__file__)), "_ingest_cache")
)

os.makedirs(CACHE_DIR, exist_ok=True)

app = Flask(__name__)
CORS(app)  # open CORS so the WASM app can call us from any origin

_YT_HOST_RE = re.compile(
    r"^https?://(www\.)?(youtube\.com|youtu\.be|music\.youtube\.com)/", re.IGNORECASE
)
_YT_ID_RE = re.compile(r"(?:v=|/shorts/|youtu\.be/|/embed/)([A-Za-z0-9_-]{11})")


# --------------------------------------------------------------------------- #
# Persistent cache helpers
# --------------------------------------------------------------------------- #
def _meta_path(song_id: str) -> str:
    return os.path.join(CACHE_DIR, f"{song_id}.json")


def _audio_path(song_id: str):
    """Return the cached audio file path for a song id, if present."""
    for name in os.listdir(CACHE_DIR):
        if name.startswith(song_id + ".") and not name.endswith(".json"):
            return os.path.join(CACHE_DIR, name)
    return None


def _load_meta(song_id: str):
    path = _meta_path(song_id)
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None


def _save_meta(meta: dict) -> None:
    with open(_meta_path(meta["id"]), "w", encoding="utf-8") as fh:
        json.dump(meta, fh)


def _response_from_meta(meta: dict) -> dict:
    return {
        "success": True,
        "id": meta["id"],
        "title": meta.get("title", meta["id"]),
        "source": meta.get("source", "file"),
        "chords": meta.get("chords", []),
        "duration": meta.get("duration", 0.0),
        "audio_url": f"/api/audio/{meta['id']}",
    }


# --------------------------------------------------------------------------- #
# yt-dlp + ChordMini
# --------------------------------------------------------------------------- #
def _youtube_id(url: str):
    m = _YT_ID_RE.search(url)
    return m.group(1) if m else None


def _youtube_title(url: str) -> str:
    try:
        proc = subprocess.run(
            ["yt-dlp", "--no-playlist", "--print", "title", "--skip-download", url],
            capture_output=True,
            text=True,
            timeout=60,
        )
        if proc.returncode == 0 and proc.stdout.strip():
            return proc.stdout.strip().splitlines()[0]
    except Exception:  # noqa: BLE001
        pass
    return ""


def _download_audio(url: str, song_id: str) -> str:
    """Download the bestaudio track as mp3 to ``<song_id>.mp3``; return its path."""
    out_template = os.path.join(CACHE_DIR, f"{song_id}.%(ext)s")
    cmd = [
        "yt-dlp",
        "-f",
        "bestaudio/best",
        "-x",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        "--no-playlist",
        "-o",
        out_template,
        url,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0:
        raise RuntimeError(f"yt-dlp failed: {proc.stderr.strip()[-500:]}")
    path = os.path.join(CACHE_DIR, f"{song_id}.mp3")
    if not os.path.exists(path):
        path = _audio_path(song_id)
    if not path or not os.path.exists(path):
        raise RuntimeError("Downloaded audio file not found")
    return path


def _recognize(path: str) -> dict:
    """Forward an audio file to ChordMini and return its JSON response."""
    with open(path, "rb") as fh:
        files = {"file": (os.path.basename(path), fh, "audio/mpeg")}
        data = {"detector": "chord-cnn-lstm"}
        resp = requests.post(
            f"{CHORDMINI_URL}/api/recognize-chords",
            files=files,
            data=data,
            timeout=1200,
        )
    resp.raise_for_status()
    return resp.json()


def _finish(result: dict, song_id: str, title: str, source: str) -> dict:
    """Persist a recognition result as cache metadata and return the response."""
    if not result.get("success"):
        raise RuntimeError(result.get("error", "Recognition failed"))
    chords = result.get("chords", [])
    duration = result.get("duration") or (chords[-1]["end"] if chords else 0.0)
    meta = {
        "id": song_id,
        "title": title or song_id,
        "source": source,
        "chords": chords,
        "duration": duration,
        "created_at": time.time(),
    }
    _save_meta(meta)
    return _response_from_meta(meta)


# --------------------------------------------------------------------------- #
# Routes
# --------------------------------------------------------------------------- #
@app.get("/")
def health():
    return jsonify({"status": "ok", "service": "jam-viewer-ingest", "chordmini": CHORDMINI_URL})


@app.post("/api/ingest-youtube")
def ingest_youtube():
    payload = request.get_json(silent=True) or {}
    url = (payload.get("url") or "").strip()
    if not url:
        return jsonify({"success": False, "error": "Missing 'url'"}), 400
    if not _YT_HOST_RE.match(url):
        return jsonify({"success": False, "error": "Not a valid YouTube URL"}), 400

    vid = _youtube_id(url)
    song_id = f"yt_{vid}" if vid else "yt_" + hashlib.sha256(url.encode()).hexdigest()[:16]

    # Serve from the persistent cache when we've analyzed this video before.
    cached = _load_meta(song_id)
    if cached and _audio_path(song_id):
        return jsonify(_response_from_meta(cached))

    title = _youtube_title(url)
    try:
        path = _download_audio(url, song_id)
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Download failed: {exc}"}), 502
    try:
        result = _recognize(path)
        return jsonify(_finish(result, song_id, title, "youtube"))
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Chord recognition failed: {exc}"}), 502


@app.post("/api/ingest-file")
def ingest_file():
    """Recognize chords in an uploaded audio file, caching it for later replay."""
    if "file" not in request.files:
        return jsonify({"success": False, "error": "Missing 'file'"}), 400
    upload = request.files["file"]
    raw = upload.read()
    if not raw:
        return jsonify({"success": False, "error": "Empty file"}), 400

    digest = hashlib.sha256(raw).hexdigest()[:16]
    song_id = f"file_{digest}"
    ext = (os.path.splitext(upload.filename or "")[1] or ".mp3").lower()

    cached = _load_meta(song_id)
    if cached and _audio_path(song_id):
        return jsonify(_response_from_meta(cached))

    audio_file = os.path.join(CACHE_DIR, f"{song_id}{ext}")
    with open(audio_file, "wb") as fh:
        fh.write(raw)

    title = os.path.splitext(os.path.basename(upload.filename or ""))[0] or song_id
    try:
        result = _recognize(audio_file)
        return jsonify(_finish(result, song_id, title, "file"))
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Chord recognition failed: {exc}"}), 502


@app.get("/api/library")
def library():
    """List previously analyzed songs, newest first."""
    songs = []
    for name in os.listdir(CACHE_DIR):
        if not name.endswith(".json"):
            continue
        meta = _load_meta(name[:-5])
        if not meta or not _audio_path(meta["id"]):
            continue
        songs.append(
            {
                "id": meta["id"],
                "title": meta.get("title", meta["id"]),
                "source": meta.get("source", "file"),
                "duration": meta.get("duration", 0.0),
                "chords": len(meta.get("chords", [])),
                "created_at": meta.get("created_at", 0.0),
            }
        )
    songs.sort(key=lambda s: s["created_at"], reverse=True)
    return jsonify({"songs": songs})


@app.get("/api/song/<song_id>")
def get_song(song_id: str):
    meta = _load_meta(song_id)
    if not meta or not _audio_path(song_id):
        abort(404)
    return jsonify(_response_from_meta(meta))


@app.get("/api/audio/<song_id>")
def serve_audio(song_id: str):
    path = _audio_path(song_id)
    if not path:
        abort(404)
    return send_file(path, mimetype="audio/mpeg", conditional=True)


if __name__ == "__main__":
    print(f"jam-viewer ingest sidecar on :{PORT}  ->  ChordMini at {CHORDMINI_URL}")
    print(f"song cache: {CACHE_DIR}")
    app.run(host="0.0.0.0", port=PORT, threaded=True)
