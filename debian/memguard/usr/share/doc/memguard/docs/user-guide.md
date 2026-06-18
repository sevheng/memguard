# memguard User Guide

## What memguard does

memguard monitors Linux memory pressure and protects your desktop session. When
RAM runs low, it freezes or kills background applications so the desktop shell
and the active window survive.

## Installation

```bash
sudo dnf copr enable @username/memguard
sudo dnf install memguard
```

## Start and stop

```bash
sudo systemctl enable --now memguard
sudo systemctl stop memguard
```

## Configuration

Edit `/etc/memguard/config.toml`:

```toml
[pressure]
poll_ms = 500

[policy]
freeze_on_critical = true
kill_delay_seconds = 5

[events]
log_path = "/var/log/memguard/events.jsonl"
```

Restart after changes:

```bash
sudo systemctl restart memguard
```

## Logs

```bash
sudo journalctl -u memguard -f
sudo tail -f /var/log/memguard/events.jsonl
```

## Troubleshooting

- **Service fails to start**: ensure the system is using cgroup v2 and the
  daemon is running as root.
- **No events file**: memguard falls back to journal logging if it cannot
  create `/var/log/memguard/events.jsonl`.
