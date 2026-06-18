# memguard-system-tune

One-time static system tuning for low-end Linux desktops running memguard.

## Prerequisites

- Root privileges (the script modifies system configuration).
- A system running `systemd`.
- A compatible `/etc/fstab` file (ext4, xfs, or btrfs filesystems).

## What it does

- Enables the bundled `ananicy-cpp` service for process niceness/IO tuning.
- Enables `zram` swap via `systemd-zram-generator`.
- Runs `fstrim`.
- Adds `noatime` to ext4/xfs/btrfs entries in `/etc/fstab`.

## The combo

`memguard-system-tune` enables three services that work together:

1. `ananicy-cpp` — keeps interactive apps responsive by renicing background work.
2. `zram` — provides compressed in-RAM swap when `systemd-zram-generator` is available
   and the machine has at least 2 GiB RAM.
3. `memguard` — protects the foreground desktop from memory pressure.

`ananicy-cpp` is configured to leave `memguard` and the desktop shell (`gnome-shell`,
`kwin_wayland`, `kwin_x11`) at neutral priority so they are not throttled.

## Usage

The tuning runs automatically once on first boot after the package is installed.

To run it manually:

```bash
sudo /usr/bin/memguard-system-tune
```

To view the logs:

```bash
journalctl -u memguard-system-tune
```

## Safety

- Backs up `/etc/fstab` before editing to `/etc/fstab.memguard-backup-<timestamp>`.
- If needed, restore the original fstab with a command such as:
  ```bash
  sudo cp /etc/fstab.memguard-backup-<timestamp> /etc/fstab
  ```
- Each step is guarded and logs its result.
- Skips steps that are already configured or not supported by the hardware.

## Warning

This script modifies system configuration. Review the changes in the logs and the
fstab backup before rebooting.
