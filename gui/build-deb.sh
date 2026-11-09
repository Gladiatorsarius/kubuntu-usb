#!/usr/bin/env bash
# Build kubuntu-usb-sync_<version>_amd64.deb from the packaging/ skeleton.
set -euo pipefail
cd "$(dirname "$0")"

# The crate version is the single source of truth; the staged
# control file is patched to match.
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
STAGE="target/deb-stage"
OUT="target/dist/kubuntu-usb-sync_${VERSION}_amd64.deb"

cargo build --release

rm -rf "$STAGE"
mkdir -p "$STAGE" "$(dirname "$OUT")"
cp -a packaging/. "$STAGE/"
install -Dm755 target/release/kubuntu-usb-sync "$STAGE/usr/bin/kubuntu-usb-sync"
sed -i "s/^Version: .*/Version: $VERSION/" "$STAGE/DEBIAN/control"

# Permissions dpkg-deb will bake in.
find "$STAGE" -type d -exec chmod 755 {} +
chmod 644 "$STAGE/DEBIAN/control" \
  "$STAGE/usr/share/applications/kubuntu-usb-sync.desktop" \
  "$STAGE/etc/udev/rules.d/99-kubuntu-usb-sync.rules" \
  "$STAGE/usr/lib/systemd/user/kubuntu-usb-sync.service"

dpkg-deb --root-owner-group --build "$STAGE" "$OUT"
echo "built $OUT"
