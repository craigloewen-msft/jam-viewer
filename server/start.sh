#!/usr/bin/env bash
# Start the jam-viewer ingest sidecar (and remind about the ChordMini backend).
#
# Usage: ./server/start.sh
#   CHORDMINI_URL  ChordMini backend base URL (default http://localhost:5001)
#   INGEST_PORT    Port to listen on (default 5002)
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v yt-dlp >/dev/null 2>&1; then
  echo "warning: yt-dlp not found on PATH — YouTube ingestion will fail." >&2
fi
if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "warning: ffmpeg not found on PATH — yt-dlp audio extraction will fail." >&2
fi

if [ ! -d .venv ]; then
  echo "Creating virtualenv and installing dependencies..."
  python -m venv .venv
  ./.venv/bin/pip install --quiet --upgrade pip
  ./.venv/bin/pip install --quiet -r requirements.txt
fi

export CHORDMINI_URL="${CHORDMINI_URL:-http://localhost:5001}"
echo "Using ChordMini backend at $CHORDMINI_URL"
exec ./.venv/bin/python ingest_server.py
