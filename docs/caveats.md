# Caveats and things to know

Read this once before you rely on the setup. None of these are blockers, but
each one can bite you if you do not know about it.

## Two-way sync conflicts

Unison syncs both ways automatically only when a file changed on ONE side.
If you edit the SAME file on the PC and on the stick since the last sync,
Unison flags a conflict and touches neither copy. Both versions stay where
they are. `sync-now.sh` prints every conflict at the end and writes them to
`~/.kubuntu-usb/last-conflicts.txt`. Resolve it by copying the version you
want over the other (see restore.md), then sync again.

Nothing is ever silently overwritten by a conflict. Files that changed on
only one side are also backed up before being replaced (see restore.md).

## The school account must be logged out during sync

Syncing a home directory while its owner is logged in can corrupt files
that open apps hold in memory (browser profiles, KDE session files).
`sync-now.sh` checks for active sessions and refuses to run the school
profile while the school user is logged in. Log into your personal account
(or stay at the login screen) when you plug the stick in.

## What is deliberately NOT synced

Live application state and volatile files are excluded on purpose:

- `~/.cache` and thumbnails
- Trash folders
- browser session locks and crash-recovery files
- Unison's own archive files
- sockets, pipes and device nodes

Copying these between machines causes broken logins and weird app states.

## Unison backups cover files, not directories or metadata

The backup copy Unison makes before replacing a file keeps the file's
content. It does not preserve permissions or timestamps, and deleted or
replaced directories are not backed up. Details in restore.md.

## Interrupted transfers

If you yank the stick or the PC loses power mid-sync, Unison may leave a
temporary file (name starts with `.unison`) next to the real one. Just run
`sync-now.sh` again; it finishes or rolls back the half-written file.
Delete leftover `.unison.*.unison.tmp` files if one sticks around.

## Secure Boot and the school laptop

Kubuntu's bootloader is signed and usually passes Secure Boot. If the
school laptop refuses to boot the stick (black screen, "Security Policy
Violation", or the stick does not appear in the F12 menu), the BIOS is
locked down. Some school laptops block USB boot entirely. Test this early:
there is nothing this repo can do about a locked BIOS.

## 32 GB is enough, but watch it

Kubuntu takes about 10 GB installed. Your apps and the school account fit
comfortably, but a big `~/Videos` or Steam library will not. Check with
`df -h /` on the stick once in a while.

## The stick is slower than your SSD

A USB 3 stick feels fine for schoolwork. Boot and app starts are slower
than your PC's SSD. Plug into a blue (USB 3) port; USB 2 ports make it
painful.
