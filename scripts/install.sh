#!/bin/sh
set -eu

DEBUG_API=0
while [ $# -gt 0 ]; do
  case "$1" in
    --debug)
      DEBUG_API=1
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

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
cargo build --release -p mazuadm-cli &
if [ "$DEBUG_API" -eq 1 ]; then
  RUSTFLAGS="--cfg tokio_unstable" cargo build -p mazuadm-api &
else
  cargo build --release -p mazuadm-api &
fi
npm --prefix web run build &
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

$SUDO systemctl stop mazuadm-api.service 
$SUDO mkdir -p "$BIN_DIR"
$SUDO cp "$CLI_BIN" "$BIN_DIR/mazuadm-cli"
$SUDO chmod 0755 "$BIN_DIR/mazuadm-cli"
$SUDO cp "$API_BIN" "$CONFIG_DIR/mazuadm-api"
$SUDO chmod 0755 "$CONFIG_DIR/mazuadm-api"
$SUDO systemctl start mazuadm-api.service 

WEB_SRC="./web/build"
WEB_DST="$CONFIG_DIR/web"
TEMPLATE_SRC="./example/exp-template"
TEMPLATE_DST="$CONFIG_DIR/exp-template"
CONFIG_SRC="./config.toml"
CONFIG_DST="$CONFIG_DIR/config.toml"

$SUDO mkdir -p "$WEB_DST/"
$SUDO cp -R "$WEB_SRC/." "$WEB_DST/"
$SUDO mkdir -p "$TEMPLATE_DST"
$SUDO cp -R "$TEMPLATE_SRC/." "$TEMPLATE_DST/"
if [ -f "$CONFIG_SRC" ]; then
  $SUDO cp "$CONFIG_SRC" "$CONFIG_DST"
  $SUDO chmod 0755 "$CONFIG_DST"
fi

echo "Installed mazuadm-cli and mazuadm-api to $BIN_DIR."
echo "Web copied to $WEB_DST."
echo "Exploit template copied to $TEMPLATE_DST."
if [ -f "$CONFIG_SRC" ]; then
  echo "Config copied to $CONFIG_DST."
fi

$SUDO systemctl reload nginx.service 
