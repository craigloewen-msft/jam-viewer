#!/usr/bin/env python3
"""YouTube ingest sidecar for jam-viewer.

The jam-viewer WASM app can POST uploaded files straight to the ChordMini
container (CORS is open there). YouTube URLs, however, need a server-side step:
this small Flask app downloads the audio with ``yt-dlp``, forwards it to
ChordMini for chord recognition, and serves the downloaded audio back so the
browser can play it in sync with the highlighted chords.

Endpoints
---------
POST /api/ingest-youtube   body: {"url": "<youtube url>"}
    -> {"success": true, "chords": [...], "audio_url": "/api/audio/<id>",
        "duration": <seconds>}
GET  /api/audio/<id>       streams the downloaded audio (supports range requests)
GET  /                     health check

Environment
-----------
CHORDMINI_URL   ChordMini backend base URL (default http://localhost:5001)
INGEST_PORT     Port to listen on (default 5002)
INGEST_CACHE    Directory for downloaded audio (default ./_ingest_cache)
"""

import os
import re
import subprocess
import tempfile
import uuid

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

# id -> absolute path of the downloaded audio file
_AUDIO_INDEX: dict[str, str] = {}

_YT_RE = re.compile(r"^https?://(www\.)?(youtube\.com|youtu\.be)/", re.IGNORECASE)


@app.get("/")
def health():
    return jsonify({"status": "ok", "service": "jam-viewer-ingest", "chordmini": CHORDMINI_URL})


def _download_audio(url: str) -> str:
    """Download the bestaudio track as mp3 and return the file path."""
    audio_id = uuid.uuid4().hex
    out_template = os.path.join(CACHE_DIR, f"{audio_id}.%(ext)s")
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
    path = os.path.join(CACHE_DIR, f"{audio_id}.mp3")
    if not os.path.exists(path):
        # Fall back to whatever extension yt-dlp produced.
        for name in os.listdir(CACHE_DIR):
            if name.startswith(audio_id):
                path = os.path.join(CACHE_DIR, name)
                break
    if not os.path.exists(path):
        raise RuntimeError("Downloaded audio file not found")
    _AUDIO_INDEX[audio_id] = path
    return audio_id, path


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


@app.post("/api/ingest-file")
def ingest_file():
    """Recognize chords in an uploaded audio file by proxying it to ChordMini.

    The browser keeps the original file for playback (via an object URL), so we
    only return the timed chords here — no audio is stored or served back.
    """
    if "file" not in request.files:
        return jsonify({"success": False, "error": "Missing 'file'"}), 400
    upload = request.files["file"]
    suffix = os.path.splitext(upload.filename or "")[1] or ".mp3"
    tmp = tempfile.NamedTemporaryFile(delete=False, dir=CACHE_DIR, suffix=suffix)
    try:
        upload.save(tmp.name)
        tmp.close()
        result = _recognize(tmp.name)
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Chord recognition failed: {exc}"}), 502
    finally:
        try:
            os.unlink(tmp.name)
        except OSError:
            pass

    if not result.get("success"):
        return jsonify(
            {"success": False, "error": result.get("error", "Recognition failed")}
        ), 502
    chords = result.get("chords", [])
    duration = result.get("duration") or (chords[-1]["end"] if chords else 0.0)
    return jsonify({"success": True, "chords": chords, "duration": duration})


@app.post("/api/ingest-youtube")
def ingest_youtube():
    payload = request.get_json(silent=True) or {}
    url = (payload.get("url") or "").strip()
    if not url:
        return jsonify({"success": False, "error": "Missing 'url'"}), 400
    if not _YT_RE.match(url):
        return jsonify({"success": False, "error": "Not a valid YouTube URL"}), 400

    try:
        audio_id, path = _download_audio(url)
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Download failed: {exc}"}), 502

    try:
        result = _recognize(path)
    except Exception as exc:  # noqa: BLE001
        return jsonify({"success": False, "error": f"Chord recognition failed: {exc}"}), 502

    if not result.get("success"):
        return jsonify(
            {"success": False, "error": result.get("error", "Recognition failed")}
        ), 502

    chords = result.get("chords", [])
    duration = result.get("duration") or (chords[-1]["end"] if chords else 0.0)
    return jsonify(
        {
            "success": True,
            "chords": chords,
            "duration": duration,
            "audio_url": f"/api/audio/{audio_id}",
        }
    )


@app.get("/api/audio/<audio_id>")
def serve_audio(audio_id: str):
    path = _AUDIO_INDEX.get(audio_id)
    if not path or not os.path.exists(path):
        # Tolerate restarts: rebuild the index from disk on demand.
        for name in os.listdir(CACHE_DIR):
            if name.startswith(audio_id):
                path = os.path.join(CACHE_DIR, name)
                _AUDIO_INDEX[audio_id] = path
                break
    if not path or not os.path.exists(path):
        abort(404)
    return send_file(path, mimetype="audio/mpeg", conditional=True)


if __name__ == "__main__":
    print(f"jam-viewer ingest sidecar on :{PORT}  ->  ChordMini at {CHORDMINI_URL}")
    app.run(host="0.0.0.0", port=PORT, threaded=True)
