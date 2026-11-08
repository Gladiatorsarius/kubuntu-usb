#!/usr/bin/env bash
# boot-stick.sh - boot the KUBUNTU-USB stick in a VM window while your normal
# Kubuntu keeps running. Test the stick, install apps, or sync without
# rebooting.
#
# One-time needs:  sudo apt install qemu-system-x86 ovmf
#                  Virtualization enabled in BIOS (VT-x / AMD-V, usually on)
#                  Your user in the kvm group: sudo usermod -aG kvm $USER
#                  (log out and back in after that)
set -euo pipefail

STICK_LABEL="KUBUNTU-USB"

PART="$(readlink -f "/dev/disk/by-label/$STICK_LABEL" 2>/dev/null || true)"
[[ -n "$PART" ]] || { echo "Stick with label $STICK_LABEL not found. Plug it in."; exit 1; }

# Whole-disk device behind the partition (e.g. /dev/sdb2 -> /dev/sdb).
DISK="/dev/$(lsblk -no PKNAME "$PART")"
[[ -b "$DISK" ]] || { echo "Cannot determine disk for $PART"; exit 1; }
[[ "$(lsblk -dn -o TRAN "$DISK")" == "usb" ]] || { echo "Refusing: $DISK is not a USB device."; exit 1; }

if findmnt -rn -S "LABEL=$STICK_LABEL" >/dev/null; then
  echo "The stick is mounted. Unmount it first:"
  echo "  udisksctl unmount -b $PART"
  exit 1
fi

[[ -w /dev/kvm ]] || { echo "/dev/kvm not writable - join the kvm group (see header)."; exit 1; }

# UEFI firmware for the VM (Kubuntu on the stick was installed for UEFI).
OVMF_CODE="/usr/share/OVMF/OVMF_CODE.fd"
[[ -f "$OVMF_CODE" ]] || { echo "Missing ovmf package: sudo apt install ovmf"; exit 1; }
VARS="$HOME/.cache/kubuntu-usb-vm-vars.fd"
mkdir -p "$(dirname "$VARS")"
[[ -f "$VARS" ]] || cp /usr/share/OVMF/OVMF_VARS.fd "$VARS"

if [[ ! -r "$DISK" || ! -w "$DISK" ]]; then
  echo "Need raw access to $DISK - restarting with sudo..."
  exec sudo -E env HOME="$HOME" DISPLAY="${DISPLAY:-}" XAUTHORITY="${XAUTHORITY:-}" \
    "$0" "$@"
fi

echo "Booting $DISK in a VM (4 GB RAM, 2 cores). Close the window to stop."
exec qemu-system-x86_64 \
  -enable-kvm -cpu host -m 4096 -smp 2 \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive if=pflash,format=raw,file="$VARS" \
  -drive file="$DISK",format=raw,if=ide,index=0,media=disk \
  -usb -device usb-tablet \
  -display gtk,show-cursor=on
