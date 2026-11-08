# Restoring files

## How the safety net works

Whenever Unison is about to overwrite or delete a file because the OTHER
side has the newer version, it first saves a backup copy on the side that
is about to lose its version.

Backups live in:

    ~/.kubuntu-usb/backups/

Each sync run that replaces something gets its own timestamped folder, with
the original path below it, for example:

    ~/.kubuntu-usb/backups/2026-11-08_22-15-03/home/school/Documents/essay.odt

Unison keeps file content only: permissions and timestamps of the backup
itself are not preserved, and directories are not backed up as such (only
the files inside them, when those files are replaced).

## Restore the losing version of a file

1. Find the backup:

       ls -lt ~/.kubuntu-usb/backups/

   or search by name:

       find ~/.kubuntu-usb/backups -name 'essay.odt'

2. Copy it back over the current version:

       cp ~/.kubuntu-usb/backups/2026-11-08_22-15-03/home/school/Documents/essay.odt \
          /home/school/Documents/essay.odt

3. Run `sync-now.sh` again so both sides match the restored version.

## Resolve a conflict (same file edited on both sides)

A conflict means NEITHER side was overwritten: both versions are still in
their normal places. They are listed in `~/.kubuntu-usb/last-conflicts.txt`.

1. Open both versions and decide which one wins, or merge them by hand:

       diff /home/school/Documents/essay.odt \
            /media/$USER/KUBUNTU-USB/home/school/Documents/essay.odt

   (adjust the stick mount path; for office documents open both in
   LibreOffice instead)

2. Copy the winner over the loser.

3. Run `sync-now.sh` again. The conflict is gone once both sides match.

## Clean up old backups

Backups accumulate. Once a month or whenever you check that everything is
fine:

    rm -rf ~/.kubuntu-usb/backups/*
