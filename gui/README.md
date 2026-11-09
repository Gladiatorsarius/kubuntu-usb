# kubuntu-usb-sync (gui/)

Desktop app that turns a USB stick into a bootable Kubuntu drive that
carries your files, apps and KDE settings, and keeps chosen folders in
sync between the PC and the stick.

Built with [egui](https://github.com/emilk/egui) (pure Rust), styled
after KDE Breeze dark.

## Installing the .deb

1. Download `kubuntu-usb-sync_0.1.1_amd64.deb` (if it arrived as
   `.deb.txt`, rename it back to `.deb` first).

2. Install it with apt - not `dpkg -i` - so the dependencies are
   resolved automatically:

   ```sh
   sudo apt install ./kubuntu-usb-sync_0.1.1_amd64.deb
   ```

   This pulls in the four runtime tools from the Kubuntu repos:
   `unison`, `qemu-system-x86`, `ovmf`, `curl`.

3. One-time setup for flashing and VM boot (raw stick access):

   ```sh
   sudo usermod -aG disk,kvm $USER
   ```

   Then log out and back in. Without this, flashing fails with a
   permission error.

4. For auto-sync on plug-in, the stick's root filesystem needs the
   label `KUBUNTU-USB`:

   ```sh
   lsblk                              # find the stick's root partition
   sudo e2label /dev/sdXN KUBUNTU-USB # use the real device
   ```

Nothing runs at install time and no service needs enabling: the package
is just the binary (`/usr/bin/kubuntu-usb-sync`), the app-menu entry
("Kubuntu USB Sync", under Utilities/System), a systemd *user* unit,
and the udev rule that triggers it when the stick is plugged in. The
watcher only runs while you are logged in, and auto-sync is off by
default - flip it on in the app's Settings screen.

Launch it from the app menu or by running `kubuntu-usb-sync`. First
launch creates `~/.config/kubuntu-usb-sync/config.toml`.

Remove it with `sudo apt remove kubuntu-usb-sync`; your config in
`~/.config/kubuntu-usb-sync/` and the ISO cache in
`~/.cache/kubuntu-usb-sync/` stay behind.

## Run

```sh
cargo run
```

Runtime tools the app shells out to (declared as .deb dependencies):

- `unison` - folder sync
- `qemu-system-x86_64` - booting the stick / installer in a VM
- `lsblk` - USB drive detection
- `curl` - Kubuntu ISO download

## Configuration

`~/.config/kubuntu-usb-sync/config.toml` (created on first run).
ISO cache lives in `~/.cache/kubuntu-usb-sync/`.

The Apps screen lists apt (`apt-mark showmanual`) and flatpak apps only -
snap packages are not covered.

The ISO is SHA256-verified against Ubuntu's published SHA256SUMS, but
that file comes over the same HTTPS channel as the image and is not
GPG-verified - this guards against corrupted downloads, not a
compromised origin.

The stick's filesystem label (`stick_label` in the config) is coupled to
the udev rule: `packaging/etc/udev/rules.d/99-kubuntu-usb-sync.rules`
matches `ID_FS_LABEL=="KUBUNTU-USB"`. Change the label in both places or
auto-sync on plug-in stops firing.

## Debug flags (used for UI screenshots)

- `--screen folders|apps|flash|vm|sync|settings` - start on a screen
- `--add-dialog` - open the Add-folder dialog on start
- `--erase-dialog` - open the erase confirmation on start
- `--vm-running` - start the VM screen in "running" state
- `--demo-drives` - pretend a SanDisk Ultra stick is plugged in

`./shot.sh <name> [flags...]` captures a screenshot under Xvfb.
