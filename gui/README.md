# kubuntu-usb-sync (gui/)

Desktop app that turns a USB stick into a bootable Kubuntu drive that
carries your files, apps and KDE settings, and keeps chosen folders in
sync between the PC and the stick.

Built with [egui](https://github.com/emilk/egui) (pure Rust), styled
after KDE Breeze dark.

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
