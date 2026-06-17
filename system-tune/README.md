# memguard-system-tune

One-time static system tuning for low-end Linux desktops running memguard.

## What it does

- Sets the I/O scheduler to `bfq` for block devices that support it.
- Installs and enables `ananicy-cpp` (if `dnf` is available).
- Enables `zram` swap via `systemd-zram-generator`.
- Runs `fstrim`.
- Adds `noatime` to ext4/xfs/btrfs entries in `/etc/fstab`.

## Usage

The tuning runs automatically once on first boot after the package is installed.

To run it manually:

```bash
sudo /usr/bin/memguard-system-tune
```

## Safety

- Backs up `/etc/fstab` before editing.
- Each step is guarded and logs its result.
- Skips steps that are already configured or not supported by the hardware.

## Warning

This script modifies system configuration. Review the changes in the logs and the
fstab backup before rebooting.
