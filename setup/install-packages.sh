#!/usr/bin/env bash
# install-packages.sh - run on the STICK.
# Installs the same apps your PC has, from the lists export-packages.sh made.
# Usage: ./install-packages.sh /path/to/kubuntu-usb-packages
set -euo pipefail

PKG_DIR="${1:?Usage: $0 /path/to/kubuntu-usb-packages}"

if [[ -s "$PKG_DIR/apt-packages.txt" ]]; then
  echo "==> Updating apt and installing $(wc -l < "$PKG_DIR/apt-packages.txt") apt packages..."
  sudo apt-get update
  # --no-install-recommends keeps the 32 GB stick lean; remove it if you want
  # the exact same extras as the PC.
  xargs -a "$PKG_DIR/apt-packages.txt" sudo apt-get install -y --no-install-recommends
fi

if [[ -s "$PKG_DIR/snap-packages.txt" ]] && command -v snap >/dev/null 2>&1; then
  echo "==> Installing snaps..."
  while read -r pkg; do
    [[ -n "$pkg" ]] && sudo snap install "$pkg" || true
  done < "$PKG_DIR/snap-packages.txt"
fi

if [[ -s "$PKG_DIR/flatpak-packages.txt" ]]; then
  if ! command -v flatpak >/dev/null 2>&1; then
    echo "==> Installing flatpak first..."
    sudo apt-get install -y flatpak
    flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
  fi
  echo "==> Installing flatpaks..."
  while read -r app; do
    [[ -n "$app" ]] && flatpak install -y flathub "$app" || true
  done < "$PKG_DIR/flatpak-packages.txt"
fi

echo "==> Done. Per-package failures (removed/renamed apps) were skipped; scroll"
echo "    up if you want to see them."
