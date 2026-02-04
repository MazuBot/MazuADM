#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SAMPLE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

FLAG_SECRET="${FLAG_SECRET:-sample-secret}"
CHAL1_PORT="${CHAL1_PORT:-18000}"
CHAL2_PORT="${CHAL2_PORT:-18001}"
CHAL3_PORT="${CHAL3_PORT:-18002}"

: "${MAZUADM_API_URL:=http://localhost:3000}"
export MAZUADM_API_URL

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[!] missing required command: $cmd" >&2
    exit 1
  fi
}

require_cmd mazuadm-cli
require_cmd docker
require_cmd rg

FLAG_REGEX='FLAG\{[^}]+\}'

ensure_challenge() {
  local name="$1"
  local port="$2"
  local priority="$3"
  local regex="$4"

  if mazuadm-cli challenge add --name "$name" --port "$port" --priority "$priority" --flag-regex "$regex"; then
    echo "==> created challenge $name"
  else
    echo "==> challenge $name exists; updating"
    mazuadm-cli challenge update "$name" --name "$name" --port "$port" --priority "$priority" --flag-regex "$regex"
  fi
}

ensure_team() {
  local id="$1"
  local name="$2"
  local ip="$3"
  local priority="$4"

  if mazuadm-cli team add --id "$id" --name "$name" --ip "$ip" --priority "$priority"; then
    echo "==> created team $id"
  else
    echo "==> team $id exists; updating"
    mazuadm-cli team update "$id" --name "$name" --ip "$ip" --priority "$priority"
  fi
}

check_config() {
  local dir="$1"
  local cfg="$dir/config.toml"
  if [ ! -f "$cfg" ]; then
    echo "[!] missing config.toml in $dir" >&2
    exit 1
  fi
}

pack_exploit() {
  local name="$1"
  local challenge="$2"
  local dir="$3"
  local cfg="$dir/config.toml"

  if mazuadm-cli exploit pack "$name" --challenge "$challenge" --config "$cfg" --dir "$dir"; then
    echo "==> packed exploit $name"
  else
    echo "==> exploit $name exists; updating"
    mazuadm-cli exploit update "$name" --challenge "$challenge" --config "$cfg"
  fi
}

add_run() {
  local exploit="$1"
  local challenge="$2"
  local team="$3"
  local seq="$4"

  if mazuadm-cli run add --exploit "$exploit" --challenge "$challenge" --team "$team" --priority 100 --sequence "$seq"; then
    echo "==> added run $exploit/$team"
  else
    echo "==> run exists for $exploit/$team; skipping"
  fi
}

add_runs_for_challenge() {
  local challenge="$1"
  local exp1="$2"
  local exp2="$3"
  local seq=0

  for exploit in "$exp1" "$exp2"; do
    for team in team1 team2; do
      add_run "$exploit" "$challenge" "$team" "$seq"
      seq=$((seq + 1))
    done
  done
}

echo "==> starting sample challenge services"
FLAG_SECRET="$FLAG_SECRET" CHAL1_PORT="$CHAL1_PORT" CHAL2_PORT="$CHAL2_PORT" CHAL3_PORT="$CHAL3_PORT" \
  "$SCRIPT_DIR/start_chals.sh"

echo "==> ensuring challenges"
ensure_challenge "chal1-web" "$CHAL1_PORT" 99 "$FLAG_REGEX"
ensure_challenge "chal2-pwn" "$CHAL2_PORT" 99 "$FLAG_REGEX"
ensure_challenge "chal3-cmdi" "$CHAL3_PORT" 99 "$FLAG_REGEX"

echo "==> ensuring teams"
ensure_team "team1" "Team 1" "127.0.0.1" 0
ensure_team "team2" "Team 2" "localhost" 0

EXPLOITS=(
  "sample-chal1-exp|chal1-web|$SAMPLE_DIR/chals/chal1-web/exp"
  "sample-chal1-exp2|chal1-web|$SAMPLE_DIR/chals/chal1-web/exp2"
  "sample-chal2-exp|chal2-pwn|$SAMPLE_DIR/chals/chal2-bin/exp"
  "sample-chal2-exp2|chal2-pwn|$SAMPLE_DIR/chals/chal2-bin/exp2"
  "sample-chal3-exp|chal3-cmdi|$SAMPLE_DIR/chals/chal3-cmdi/exp"
  "sample-chal3-exp2|chal3-cmdi|$SAMPLE_DIR/chals/chal3-cmdi/exp2"
)

echo "==> validating exploit configs"
for entry in "${EXPLOITS[@]}"; do
  IFS='|' read -r name challenge dir <<< "$entry"
  check_config "$dir"
  if ! rg -q '^insert_into_rounds\s*=\s*true' "$dir/config.toml"; then
    echo "[!] insert_into_rounds must be true in $dir/config.toml" >&2
    exit 1
  fi
  if ! rg -q "^challenge\s*=\s*\"$challenge\"" "$dir/config.toml"; then
    echo "[!] challenge name mismatch in $dir/config.toml (expected $challenge)" >&2
    exit 1
  fi
  if ! rg -q "^name\s*=\s*\"$name\"" "$dir/config.toml"; then
    echo "[!] name mismatch in $dir/config.toml (expected $name)" >&2
    exit 1
  fi
  if ! rg -q "^image\s*=\s*\"$name\"" "$dir/config.toml"; then
    echo "[!] image mismatch in $dir/config.toml (expected $name)" >&2
    exit 1
  fi
done

echo "==> building and importing exploits"
for entry in "${EXPLOITS[@]}"; do
  IFS='|' read -r name challenge dir <<< "$entry"
  pack_exploit "$name" "$challenge" "$dir"
done

echo "==> creating exploit runs"
add_runs_for_challenge "chal1-web" "sample-chal1-exp" "sample-chal1-exp2"
add_runs_for_challenge "chal2-pwn" "sample-chal2-exp" "sample-chal2-exp2"
add_runs_for_challenge "chal3-cmdi" "sample-chal3-exp" "sample-chal3-exp2"

echo "==> creating and running a new round"
round_out="$(mazuadm-cli round new)"
echo "$round_out"
round_id="$(echo "$round_out" | awk '/Created round/ {print $3; exit}')"
if [ -z "$round_id" ]; then
  echo "[!] failed to parse round id" >&2
  exit 1
fi
mazuadm-cli round run "$round_id"
