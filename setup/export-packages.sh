#!/usr/bin/env bash
# export-packages.sh - run on your PC.
# Saves the list of apps you installed via apt (plus snap/flatpak) so the
# stick can install the same set.
set -euo pipefail

OUT_DIR="${1:-$HOME/kubuntu-usb-packages}"
mkdir -p "$OUT_DIR"

# apt: only packages YOU installed, not every preinstalled dependency.
apt-mark showmanual | sort > "$OUT_DIR/apt-packages.txt"

# snaps and flatpaks, if any.
if command -v snap >/dev/null 2>&1; then
  snap list | awk 'NR>1 {print $1}' | sort > "$OUT_DIR/snap-packages.txt"
else
  : > "$OUT_DIR/snap-packages.txt"
fi
if command -v flatpak >/dev/null 2>&1; then
  flatpak list --app --columns=application | sort > "$OUT_DIR/flatpak-packages.txt"
else
  : > "$OUT_DIR/flatpak-packages.txt"
fi

echo "Wrote:"
echo "  $OUT_DIR/apt-packages.txt      ($(wc -l < "$OUT_DIR/apt-packages.txt") packages)"
echo "  $OUT_DIR/snap-packages.txt     ($(wc -l < "$OUT_DIR/snap-packages.txt") packages)"
echo "  $OUT_DIR/flatpak-packages.txt  ($(wc -l < "$OUT_DIR/flatpak-packages.txt") packages)"
echo
echo "Copy the whole $OUT_DIR folder onto the stick (or sync it), then run"
echo "install-packages.sh there."
