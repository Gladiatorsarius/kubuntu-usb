# Step 1: Install Kubuntu on the USB stick

You end this step with a normal Kubuntu system living entirely on the 32 GB
stick. Your PC's SSD is not touched.

## What you need

- The 32 GB stick (the TARGET). USB 3 strongly recommended.
- A second stick, 4 GB or more (the INSTALLER). It gets erased.
- A Kubuntu ISO (24.04 LTS or newer) from https://kubuntu.org/getkubuntu/
- About an hour.

## 1. Flash the installer stick

On your PC:

1. Plug in the INSTALLER stick.
2. Open the ISO with a flashing tool: KDE's ISO Image Writer
   (`sudo apt install isoimagewriter`), Ventoy, or balenaEtcher.
3. Write the ISO to the installer stick.

## 2. Boot the installer

1. Leave the installer stick in, plug the TARGET stick in too, reboot.
2. On your PC, open the boot menu (usually F12, F10 or ESC during power-on)
   and pick the installer stick.
3. Choose "Try or Install Kubuntu".

## 3. Install onto the TARGET stick

In the installer:

1. Language/keyboard as usual.
2. At "Installation type", pick the TARGET stick as the disk. THIS IS THE
   DANGEROUS STEP: the list also shows your internal SSD. Identify the stick
   by its size (about 29-32 GB) and brand. If unsure, cancel, unplug the
   stick, note the disk names, plug it back in, and see which name appeared
   (usually `/dev/sdb` or `/dev/sdc`, never `nvme0n1`).
3. Choose "Erase disk" (on the STICK) for a simple layout, or "Manual
   partitioning" if you want a separate home partition.
4. IMPORTANT: the bootloader dropdown must point at the STICK
   (e.g. `/dev/sdb`), not the SSD. The installer sometimes defaults to the
   first disk. On Kubuntu 24.04's Calamares installer this is the
   "Install boot loader on:" dropdown at the bottom of the partitioning page.
5. Create your user account. Use the SAME username as your school account
   on the PC if you want paths to match exactly (recommended).
6. Let it finish, shut down, remove the INSTALLER stick.

## 4. First boot check

1. Boot the PC from the TARGET stick (boot menu, pick the stick).
2. Confirm you reach the Kubuntu desktop and Wi-Fi works.
3. Run `sudo apt update && sudo apt full-upgrade` once.

Your SSD was never written to: unplug the stick and the PC boots exactly as
before.

Next: 02-first-boot.md
