# Step 2: First boot - make the stick feel like your PC

Run through this booted INTO the stick, with a network connection.

## 1. Install the same apps

On your PC (normal SSD boot), run `setup/export-packages.sh`. Copy the
resulting `kubuntu-usb-packages` folder onto the stick, boot the stick, and
run:

    ./setup/install-packages.sh ~/kubuntu-usb-packages

## 2. Copy your settings

Almost everything KDE stores lives in `~/.config` and `~/.local/share`.
On the PC, copy what you care about onto the stick's home. Easiest: plug
the stick into the running PC, mount it, then:

    rsync -a ~/.config/ /media/$USER/KUBUNTU-USB/home/<stick-user>/.config/
    rsync -a ~/.local/share/ /media/$USER/KUBUNTU-USB/home/<stick-user>/.local/share/

Or copy only selected folders (dolphinrc, kwinrc, plasma configs...).
Skip `~/.cache` entirely.

Notes:
- Screen layout settings may look wrong on other hardware; just redo the
  display settings once per machine.
- Do this copy while NEITHER side is running the apps being copied (a
  freshly booted stick at the desktop is fine; a running browser is not).

## 3. KDE Connect: keep your phone paired

Your phone pairs with a device IDENTITY, which is just files:

    ~/.config/kdeconnect/

Copy that whole folder from the PC to the stick's home (same path). The
phone then treats the stick as the same device - no re-pairing.

If you'd rather not copy it: on the stick, open KDE Connect and pair again.
That takes a minute, but your phone will show TWO devices with similar
names, and only one is reachable at a time.

Never run both machines with the same identity at once - the phone gets
confused about which one to talk to.

## 4. Create the school account on the stick

    sudo adduser school
    sudo usermod -aG sudo school   # only if the school account needs sudo

Log into it once so its home directory is fully created. This is the
account the two-way sync keeps in step with the school account on your PC.

## 5. Set up sync

Continue with the main README, step 3.
