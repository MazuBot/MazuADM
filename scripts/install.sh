#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cargo build --release -p mazuadm-cli -p mazuadm-api

BIN_DIR="/usr/local/bin"
CLI_BIN="$ROOT/target/release/mazuadm-cli"
API_BIN="$ROOT/target/release/mazuadm-api"

if [ "$(id -u)" -ne 0 ]; then
  sudo mkdir -p "$BIN_DIR"
  sudo cp "$CLI_BIN" "$BIN_DIR/mazuadm-cli"
  sudo cp "$API_BIN" "$BIN_DIR/mazuadm-api"
  sudo chmod 0755 "$BIN_DIR/mazuadm-cli" "$BIN_DIR/mazuadm-api"
else
  mkdir -p "$BIN_DIR"
  cp "$CLI_BIN" "$BIN_DIR/mazuadm-cli"
  cp "$API_BIN" "$BIN_DIR/mazuadm-api"
  chmod 0755 "$BIN_DIR/mazuadm-cli" "$BIN_DIR/mazuadm-api"
fi

TEMPLATE_SRC="$ROOT/sample/exp-template"
TEMPLATE_DST="/etc/mazuadm/exp-template"
CONFIG_SRC="$PWD/config.toml"
CONFIG_DST="/etc/mazuadm/config.toml"

if [ "$(id -u)" -ne 0 ]; then
  sudo mkdir -p "$TEMPLATE_DST"
  sudo cp -R "$TEMPLATE_SRC/." "$TEMPLATE_DST/"
  if [ -f "$CONFIG_SRC" ]; then
    sudo mkdir -p "/etc/mazuadm"
    sudo cp "$CONFIG_SRC" "$CONFIG_DST"
  fi
else
  mkdir -p "$TEMPLATE_DST"
  cp -R "$TEMPLATE_SRC/." "$TEMPLATE_DST/"
  if [ -f "$CONFIG_SRC" ]; then
    mkdir -p "/etc/mazuadm"
    cp "$CONFIG_SRC" "$CONFIG_DST"
  fi
fi

echo "Installed mazuadm-cli and mazuadm-api to $BIN_DIR."
echo "Exploit template copied to $TEMPLATE_DST."
if [ -f "$CONFIG_SRC" ]; then
  echo "Config copied to $CONFIG_DST."
fi
