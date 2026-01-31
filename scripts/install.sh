#!/bin/sh
set -eu

DEBUG_API=0
FRONTEND_ONLY=0
BACKEND_ONLY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --debug)
      DEBUG_API=1
      shift
      ;;
    --frontend)
      FRONTEND_ONLY=1
      shift
      ;;
    --backend)
      BACKEND_ONLY=1
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

if [ "$FRONTEND_ONLY" -eq 1 ] && [ "$BACKEND_ONLY" -eq 1 ]; then
  echo "Error: --frontend and --backend cannot be used together"
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo require root
  sudo id
  SUDO="sudo"
else
  SUDO=""
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT"
if [ "$FRONTEND_ONLY" -eq 0 ]; then
  cargo build --release -p mazuadm-cli &
  if [ "$DEBUG_API" -eq 1 ]; then
    RUSTFLAGS="--cfg tokio_unstable" cargo build -p mazuadm-api &
  else
    cargo build --release -p mazuadm-api &
  fi
fi
if [ "$BACKEND_ONLY" -eq 0 ]; then
  {
    npm --prefix web ci
    npm --prefix web run build
  } &
fi
wait

BIN_DIR="/usr/local/bin"
CLI_BIN="./target/release/mazuadm-cli"
if [ "$DEBUG_API" -eq 1 ]; then
  API_BIN="./target/debug/mazuadm-api"
else
  API_BIN="./target/release/mazuadm-api"
fi
CONFIG_DIR='/opt/mazuadm'

$SUDO mkdir -p "$CONFIG_DIR"

if [ "$FRONTEND_ONLY" -eq 0 ]; then
  $SUDO systemctl stop mazuadm-api.service 
  $SUDO mkdir -p "$BIN_DIR"
  $SUDO cp "$CLI_BIN" "$BIN_DIR/mazuadm-cli"
  $SUDO chmod 0755 "$BIN_DIR/mazuadm-cli"
  $SUDO cp "$API_BIN" "$CONFIG_DIR/mazuadm-api"
  $SUDO chmod 0755 "$CONFIG_DIR/mazuadm-api"
  $SUDO systemctl start mazuadm-api.service 
fi

if [ "$BACKEND_ONLY" -eq 0 ]; then
  WEB_SRC="./web/build"
  WEB_DST="$CONFIG_DIR/web"
  $SUDO mkdir -p "$WEB_DST/"
  $SUDO cp -R "$WEB_SRC/." "$WEB_DST/"
fi

TEMPLATE_SRC="./example/exp-template"
TEMPLATE_DST="$CONFIG_DIR/exp-template"
CONFIG_SRC="./config.toml"
CONFIG_DST="$CONFIG_DIR/config.toml"

$SUDO mkdir -p "$TEMPLATE_DST"
$SUDO cp -R "$TEMPLATE_SRC/." "$TEMPLATE_DST/"
if [ -f "$CONFIG_SRC" ]; then
  $SUDO cp "$CONFIG_SRC" "$CONFIG_DST"
  $SUDO chmod 0755 "$CONFIG_DST"
fi

if [ "$FRONTEND_ONLY" -eq 0 ]; then
  echo "Installed mazuadm-cli and mazuadm-api to $BIN_DIR."
fi
if [ "$BACKEND_ONLY" -eq 0 ]; then
  echo "Web copied to $CONFIG_DIR/web."
fi
echo "Exploit template copied to $TEMPLATE_DST."
if [ -f "$CONFIG_SRC" ]; then
  echo "Config copied to $CONFIG_DST."
fi
