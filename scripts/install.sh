#!/bin/sh
set -eu

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
cargo build --release -p mazuadm-cli -p mazuadm-api &
npm --prefix web run build &
wait

BIN_DIR="/usr/local/bin"
CLI_BIN="./target/release/mazuadm-cli"
API_BIN="./target/release/mazuadm-api"

$SUDO systemctl stop mazuadm-api.service 
$SUDO mkdir -p "$BIN_DIR"
$SUDO cp "$CLI_BIN" "$BIN_DIR/mazuadm-cli"
$SUDO cp "$API_BIN" "$BIN_DIR/mazuadm-api"
$SUDO chmod 0755 "$BIN_DIR/mazuadm-cli"
$SUDO chmod 0750 "$BIN_DIR/mazuadm-api"
$SUDO systemctl start mazuadm-api.service 

CONFIG_DIR='/etc/mazuadm'
$SUDO mkdir -p "$CONFIG_DIR"

WEB_SRC="./web/build"
WEB_DST="$CONFIG_DIR/web"
TEMPLATE_SRC="./sample/exp-template"
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
