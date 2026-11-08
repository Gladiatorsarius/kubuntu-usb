# Test checklist

Run these once after setup, before you trust the system with real work.
Each test names what SHOULD happen. If anything differs, check
docs/caveats.md and the log at ~/.kubuntu-usb/sync.log.

Setup for the tests: pick a scratch folder that is synced, e.g.
`~/Documents/sync-test` in the school account, and the matching folder on
the stick.

## 1. Different files edited on each side

- On the PC, create `pc-file.txt`. On the stick, create `stick-file.txt`.
- Run sync (plug the stick in, or `sync-now.sh`).
- EXPECT: both files exist on both sides. No conflicts reported.

## 2. Same file edited on both sides (conflict)

- Create `shared.txt` on the PC, sync so it reaches the stick.
- Edit it on the PC AND (differently) on the stick without syncing in
  between.
- Run sync.
- EXPECT: a conflict is reported for `shared.txt`, both copies are left
  untouched, and it appears in `~/.kubuntu-usb/last-conflicts.txt`.
  Resolve per docs/restore.md, sync again, expect a clean run.

## 3. Delete on one side, edit on the other

- Delete `shared.txt` on the PC. Edit it on the stick.
- Run sync.
- EXPECT: Unison treats delete-vs-edit as a conflict. The edited stick
  version survives. (If both sides had simply deleted, the file would be
  gone on both.)

## 4. Stick missing or mounted in the wrong place

- Run `sync-now.sh` WITHOUT the stick plugged in.
- EXPECT: a clear "stick not found" message, exit code 1, nothing synced,
  no half-made changes.

## 5. Interrupted transfer

- Start a sync with a large new file (e.g. copy a 500 MB file into the
  synced folder). Pull the stick out mid-copy. Plug it back in, sync again.
- EXPECT: the rerun completes the file or restarts it cleanly. At most a
  leftover `.unison.*.unison.tmp` file, which you can delete. No corrupt
  final file (compare sizes or open it).

## 6. Restore the losing version

- After any sync that replaced a file, find it under
  `~/.kubuntu-usb/backups/`, restore it per docs/restore.md, sync again.
- EXPECT: the restored content ends up on both sides.

## 7. School account logged in

- Log into the school account on the PC, plug the stick in (or run
  `sync-now.sh`).
- EXPECT: the school-home sync is SKIPPED with a warning telling you to
  log out of the school account first. Configs sync still runs.

## 8. VM boot of the stick

- With the stick plugged in and NOT mounted, run `vm/boot-stick.sh`.
- EXPECT: a VM window opens and boots into the stick's Kubuntu.
