# memguard-system-tune

One-time static system tuning for low-end Linux desktops running memguard.

## Prerequisites

- Root privileges (the script modifies system configuration).
- A system running `systemd`.
- A compatible `/etc/fstab` file (ext4, xfs, or btrfs filesystems).

## What it does

- Sets the I/O scheduler to `bfq` for block devices that support it.
- On `dnf`-based distributions, installs and enables `ananicy-cpp`.
- Enables `zram` swap via `systemd-zram-generator`.
- Runs `fstrim`.
- Adds `noatime` to ext4/xfs/btrfs entries in `/etc/fstab`.

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
