#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SAMPLE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TTY_ARGS=()
if [ -t 0 ]; then
  TTY_ARGS=(-it)
fi

TEAM_ID="${TEAM_ID:-1}"
HOST="${HOST:-127.0.0.1}"
CHAL1_PORT="${CHAL1_PORT:-18000}"
CHAL2_PORT="${CHAL2_PORT:-18001}"
CHAL3_PORT="${CHAL3_PORT:-18002}"

require_image() {
  if ! docker image inspect "$1" >/dev/null 2>&1; then
    echo "[!] missing docker image: $1" >&2
    exit 1
  fi
}

build_image() {
  local image="$1"
  local context="$2"
  echo "==> build $image"
  docker build -t "$image" "$context" >/dev/null
}

run_chal1() {
  local image="$1"
  echo "==> $image"
  docker run --rm "${TTY_ARGS[@]}" --network host \
    -e TARGET_HOST="$HOST" -e TARGET_PORT="$CHAL1_PORT" -e TARGET_TEAM_ID="$TEAM_ID" \
    "$image"
}

run_chal2() {
  local image="$1"
  echo "==> $image"
  docker run --rm "${TTY_ARGS[@]}" --network host \
    "$image" /run "$HOST" "$CHAL2_PORT" "$TEAM_ID"
}

run_chal3() {
  local image="$1"
  echo "==> $image"
  docker run --rm "${TTY_ARGS[@]}" --network host \
    "$image" /run "$HOST" "$CHAL3_PORT" "$TEAM_ID"
}

build_image sample-chal1-exp "$SAMPLE_DIR/chals/chal1-web/exp"
build_image sample-chal1-exp2 "$SAMPLE_DIR/chals/chal1-web/exp2"
build_image sample-chal2-exp "$SAMPLE_DIR/chals/chal2-bin/exp"
build_image sample-chal2-exp2 "$SAMPLE_DIR/chals/chal2-bin/exp2"
build_image sample-chal3-exp "$SAMPLE_DIR/chals/chal3-cmdi/exp"
build_image sample-chal3-exp2 "$SAMPLE_DIR/chals/chal3-cmdi/exp2"

run_chal1 sample-chal1-exp
run_chal1 sample-chal1-exp2
run_chal2 sample-chal2-exp
run_chal2 sample-chal2-exp2
run_chal3 sample-chal3-exp
run_chal3 sample-chal3-exp2
