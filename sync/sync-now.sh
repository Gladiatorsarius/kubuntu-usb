#!/usr/bin/env bash
# sync-now.sh - two-way sync between this PC and the KUBUNTU-USB stick.
#
# Run it manually anytime:   ./sync-now.sh
# It also runs automatically when the stick is plugged in (udev + systemd).
#
# What it does:
#   1. Finds and mounts the stick (filesystem label KUBUNTU-USB).
#   2. Syncs the school account home, both ways - SKIPPED with a warning
#      if the school user is logged in right now.
#   3. Syncs the personal config folders from config/folders.txt, both ways.
#   4. Reports conflicts. Conflicts are never auto-resolved; both versions
#      are kept (see docs/restore.md).
#
# Safe to interrupt and re-run. Never deletes anything without keeping a
# backup copy under ~/.kubuntu-usb/backups/<timestamp>/.
set -uo pipefail

# ---- adjust these three if your setup differs -----------------------------
PC_USER="${SUDO_USER:-$USER}"     # your personal account on the PC
SCHOOL_USER="school"              # the school account name
STICK_LABEL="KUBUNTU-USB"         # filesystem label of the stick's root
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FOLDERS_FILE="$SCRIPT_DIR/../config/folders.txt"
STATE_DIR="/home/$PC_USER/.kubuntu-usb"
LOG="$STATE_DIR/sync.log"
CONFLICTS_FILE="$STATE_DIR/last-conflicts.txt"
RUN_ID="$(date +%F_%H-%M-%S)"
BACKUP_DIR="$STATE_DIR/backups/$RUN_ID"

say() { echo "[sync] $*"; }
log() { mkdir -p "$STATE_DIR"; echo "$(date -Is) $*" >> "$LOG"; say "$*"; }

# ---- 1. find the stick -----------------------------------------------------
MNT="$(findmnt -rn -o TARGET -S "LABEL=$STICK_LABEL" 2>/dev/null || true)"
if [[ -z "$MNT" ]]; then
  DEV="$(readlink -f "/dev/disk/by-label/$STICK_LABEL" 2>/dev/null || true)"
  if [[ -n "$DEV" ]]; then
    say "Stick not mounted, trying udisks..."
    MNT="$(sudo -u "$PC_USER" udisksctl mount -b "$DEV" 2>/dev/null | sed -n 's/.* at \(.*\)\./\1/p' || true)"
  fi
fi
if [[ -z "$MNT" || ! -d "$MNT" ]]; then
  say "ERROR: stick with label $STICK_LABEL not found or not mountable."
  say "Plug it in, or check the label with: lsblk -o NAME,LABEL"
  log "FAILED: stick not found"
  exit 1
fi
say "Stick mounted at $MNT"
command -v unison >/dev/null 2>&1 || { say "ERROR: unison not installed (sudo apt install unison)"; exit 1; }

run_unison() {  # $1=profile  $2=pc-root  $3=stick-root  [extra -path args...]
  local profile="$1" pcroot="$2" stickroot="$3"; shift 3
  mkdir -p "$BACKUP_DIR"
  sudo -u "$PC_USER" HOME="/home/$PC_USER" unison "$SCRIPT_DIR/$profile" \
    "$pcroot" "$stickroot" \
    -backupdir "$BACKUP_DIR" \
    "$@" 2>&1 | tee -a "$LOG"
}

CONFLICTS=""

# ---- 2. school account -----------------------------------------------------
if loginctl list-sessions --no-legend 2>/dev/null | awk '{print $3}' | grep -qx "$SCHOOL_USER" \
   || who 2>/dev/null | awk '{print $1}' | grep -qx "$SCHOOL_USER"; then
  say "SKIP school account: '$SCHOOL_USER' is logged in. Syncing a live home"
  say "directory can corrupt open app data. Log out of $SCHOOL_USER first."
  log "SKIPPED school profile (user logged in)"
else
  say "Syncing school account home (both ways)..."
  OUT="$(run_unison school-home.prf "/home/$SCHOOL_USER" "$MNT/home/$SCHOOL_USER")"
  echo "$OUT" | tail -5
  C="$(echo "$OUT" | grep -iE 'conflict|skipped' || true)"
  [[ -n "$C" ]] && CONFLICTS+=$'\n'"[school-home]"$'\n'"$C"
fi

# ---- 3. personal config folders --------------------------------------------
if [[ -f "$FOLDERS_FILE" ]]; then
  PATH_ARGS=()
  while IFS= read -r line; do
    line="${line%%#*}"; line="${line//[[:space:]]/}"
    [[ -n "$line" ]] && PATH_ARGS+=(-path "$line")
  done < "$FOLDERS_FILE"
  if [[ ${#PATH_ARGS[@]} -gt 0 ]]; then
    say "Syncing personal config folders (both ways)..."
    OUT="$(run_unison configs.prf "/home/$PC_USER" "$MNT/home/$PC_USER" "${PATH_ARGS[@]}")"
    echo "$OUT" | tail -5
    C="$(echo "$OUT" | grep -iE 'conflict|skipped' || true)"
    [[ -n "$C" ]] && CONFLICTS+=$'\n'"[configs]"$'\n'"$C"
  fi
fi

# ---- 4. report ---------------------------------------------------------------
if [[ -n "$CONFLICTS" ]]; then
  printf '%s\n' "$CONFLICTS" > "$CONFLICTS_FILE"
  say "DONE WITH CONFLICTS - nothing was overwritten:"
  say "$CONFLICTS"
  say "Resolve per docs/restore.md, then sync again."
  log "finished WITH conflicts"
  exit 2
fi
say "Done. Everything in sync. (Backups of replaced files: $BACKUP_DIR)"
log "finished clean"
