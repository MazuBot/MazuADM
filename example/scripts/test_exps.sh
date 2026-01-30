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
CHALS="${CHALS:-all}"

build_image() {
  local image="$1"
  local context="$2"
  echo "==> build $image"
  docker build -t "$image" "$context" >/dev/null
}

run_exp() {
  local mode="$1"
  local image="$2"
  local port="$3"
  echo "==> $image"
  if [ "$mode" = "env" ]; then
    docker run --rm "${TTY_ARGS[@]}" --network host \
      -e TARGET_HOST="$HOST" -e TARGET_PORT="$port" -e TARGET_TEAM_ID="$TEAM_ID" \
      "$image"
  else
    docker run --rm "${TTY_ARGS[@]}" --network host \
      "$image" /run "$HOST" "$port" "$TEAM_ID"
  fi
}

test_chal() {
  local chal="$1"
  local mode="$2"
  local port="$3"
  local context="$4"

  build_image "sample-${chal}-exp" "$context/exp"
  build_image "sample-${chal}-exp2" "$context/exp2"
  run_exp "$mode" "sample-${chal}-exp" "$port"
  run_exp "$mode" "sample-${chal}-exp2" "$port"
}

case "$CHALS" in
  all)
    test_chal chal1 env "$CHAL1_PORT" "$SAMPLE_DIR/chals/chal1-web"
    test_chal chal2 args "$CHAL2_PORT" "$SAMPLE_DIR/chals/chal2-bin"
    test_chal chal3 args "$CHAL3_PORT" "$SAMPLE_DIR/chals/chal3-cmdi"
    ;;
  chal1)
    test_chal chal1 env "$CHAL1_PORT" "$SAMPLE_DIR/chals/chal1-web"
    ;;
  chal2)
    test_chal chal2 args "$CHAL2_PORT" "$SAMPLE_DIR/chals/chal2-bin"
    ;;
  chal3)
    test_chal chal3 args "$CHAL3_PORT" "$SAMPLE_DIR/chals/chal3-cmdi"
    ;;
  *)
    echo "[!] CHALS must be one of: all, chal1, chal2, chal3" >&2
    exit 2
    ;;
esac
