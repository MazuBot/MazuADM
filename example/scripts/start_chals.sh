#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SAMPLE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

FLAG_SECRET="${FLAG_SECRET:-sample-secret}"
CHAL1_PORT="${CHAL1_PORT:-18000}"
CHAL2_PORT="${CHAL2_PORT:-18001}"
CHAL3_PORT="${CHAL3_PORT:-18002}"

build_image() {
  local image="$1"
  local context="$2"
  echo "==> build $image"
  docker build -t "$image" "$context" >/dev/null
}

container_exists() {
  local name="$1"
  docker ps -a --format '{{.Names}}' | rg -x "$name" >/dev/null 2>&1
}

container_running() {
  local name="$1"
  docker ps --format '{{.Names}}' | rg -x "$name" >/dev/null 2>&1
}

start_container() {
  local name="$1"
  shift

  if container_running "$name"; then
    echo "==> $name already running; skipping"
    return 0
  fi

  if container_exists "$name"; then
    echo "[!] $name exists (stopped); skipping to avoid touching it" >&2
    return 0
  fi

  echo "==> start $name"
  docker run -d --rm --name "$name" "$@" >/dev/null
}

build_image chal1-srv "$SAMPLE_DIR/chals/chal1-web/srv"
build_image chal2-srv "$SAMPLE_DIR/chals/chal2-bin/srv"
build_image chal3-srv "$SAMPLE_DIR/chals/chal3-cmdi/srv"

start_container chal1srv -p "${CHAL1_PORT}:8000" -e "FLAG_SECRET=${FLAG_SECRET}" chal1-srv
start_container chal2srv -p "${CHAL2_PORT}:5000" chal2-srv
start_container chal3srv -p "${CHAL3_PORT}:8000" chal3-srv
