# kubuntu-usb

A full Kubuntu workstation on a 32 GB USB stick: boot it anywhere, and it
syncs itself with your home PC when you plug it back in.

- Boot the stick on any PC that allows USB boot (F12 boot menu)
- Your apps: same apt/snap/flatpak set as your PC
- Your school account: whole home directory synced BOTH WAYS with the PC
- Your personal settings (KDE theme, panels, shortcuts, KDE Connect
  pairing): synced both ways
- Automatic: plug the stick into your PC at home and the sync runs by itself
- Safe: the same file edited on both sides is flagged as a conflict, never
  silently overwritten; replaced files are backed up first

Works with Kubuntu 24.04 LTS or newer.

## What you need

- A 32 GB USB stick, USB 3 strongly recommended (the TARGET)
- A second stick, 4+ GB, for the installer (it gets erased)
- A Kubuntu ISO from https://kubuntu.org/getkubuntu/
- Your PC running Kubuntu

## Step 1: Install Kubuntu on the stick

Follow [setup/01-install-kubuntu.md](setup/01-install-kubuntu.md).

Short version: flash the installer stick, boot it, install onto the TARGET
stick, and make sure BOTH the target disk AND the bootloader dropdown point
at the stick, not your SSD. Your internal drive is never touched.

## Step 2: First boot on the stick

Follow [setup/02-first-boot.md](setup/02-first-boot.md).

It covers: installing your apps (export on PC, install on stick), copying
your KDE settings, copying the KDE Connect identity so your phone stays
paired, and creating the `school` account on the stick.

## Step 3: Set up sync

On your PC (normal SSD boot):

```bash
# 1. Install Unison (the two-way sync tool)
sudo apt install unison

# 2. Copy this repo's sync folder into place
sudo mkdir -p /usr/local/bin/kubuntu-usb
sudo cp -r sync config /usr/local/bin/kubuntu-usb/
sudo cp sync/99-usb-sync.rules /etc/udev/rules.d/
sudo cp sync/usb-sync.service /etc/systemd/system/
sudo udevadm control --reload-rules
sudo systemctl daemon-reload

# 3. Label the stick's root filesystem so the scripts can find it
#    (find the partition with: lsblk -o NAME,LABEL,FSTYPE,SIZE)
sudo e2label /dev/sdX2 KUBUNTU-USB

# 4. Open /usr/local/bin/kubuntu-usb/sync/sync-now.sh and check the three
#    variables at the top (personal username, school username, stick label)
```

Choose what syncs by editing `config/folders.txt` (personal settings).
The school account's whole home is always included.

## Step 4: Daily routine

- Work on the stick anywhere (school laptop: power on, press F12, pick the
  stick).
- At home, plug the stick into your running PC. The sync starts by itself
  and carries changes in both directions.
- Watch for a conflict report. Conflicts mean you edited the same file on
  both sides; both versions are kept. Resolve per
  [docs/restore.md](docs/restore.md).
- One rule: the school account must be LOGGED OUT when the sync runs. The
  script skips the school sync with a warning if it isn't.
- Manual sync anytime: `sudo /usr/local/bin/kubuntu-usb/sync/sync-now.sh`

## Step 5: Boot the stick on another machine

1. Power on, tap F12 (on some laptops: F9, F10, ESC or F8) for the boot
   menu.
2. Pick the USB stick (often listed as "UEFI: <brand>").
3. If it won't boot: see Secure Boot notes in
   [docs/caveats.md](docs/caveats.md). A school laptop with a locked BIOS
   can block USB boot entirely - test this early.

## Bonus: boot the stick in a VM

Test or update the stick without rebooting your PC:

```bash
sudo apt install qemu-system-x86 ovmf
./vm/boot-stick.sh
```

See [vm/boot-stick.sh](vm/boot-stick.sh) for the one-time setup in its
header comment.

## Repo map

```
setup/    install walkthroughs + app export/import scripts
sync/     Unison profiles, the sync script, auto-run udev rule + service
vm/       boot the stick in a VM window
config/   folders.txt (what syncs), apps-extra.txt (non-apt apps)
docs/     caveats.md, restore.md, test-checklist.md
```

## After setup

Run the acceptance tests in [docs/test-checklist.md](docs/test-checklist.md)
once, so you know exactly how conflicts, deletes and interruptions behave
before real schoolwork depends on it.

## Troubleshooting

| Problem | Fix |
| --- | --- |
| Stick doesn't appear in F12 menu | Try another port; BIOS may block USB boot (see caveats) |
| "Security Policy Violation" on boot | Secure Boot issue, see docs/caveats.md |
| Sync says "stick not found" | Check label: `lsblk -o NAME,LABEL`; re-run the e2label step |
| Sync skips the school account | Log out of the school account and plug the stick in again |
| Conflict reported | Not an error: both versions kept, see docs/restore.md |
| VM script refuses to boot the stick | Unmount it first: `udisksctl unmount -b /dev/sdX2` |
| Phone sees two KDE Connect devices | You paired twice; remove one in the phone's KDE Connect app, or copy the identity folder (setup/02) |
| Stick feels slow | Use a blue USB 3 port; avoid USB 2 hubs |
