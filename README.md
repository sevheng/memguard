# memguard

Linux desktop memory pressure daemon.

memguard monitors system memory pressure and protects the active desktop shell
and foreground application by freezing or killing background cgroups before the
OOM killer is triggered.

## Features

- **PSI-based monitoring** of memory pressure (`/proc/pressure/memory`).
- **Desktop awareness**: discovers the graphical session, shell PID, and active
  window via logind and D-Bus (GNOME / KDE).
- ** cgroup-level mitigations**:
  - Throttle background apps on warning.
  - Freeze background apps and shield the shell/active app on critical.
  - Kill the largest background cgroup if critical pressure persists.
- **Structured JSON event logging** to `/var/log/memguard/events.jsonl`.
- **Companion package** `memguard-system-tune` for static low-end desktop
  tuning (bfq I/O scheduler, ananicy-cpp, zram, fstrim, fstab `noatime`).

## Packages

| Package | Description |
|---------|-------------|
| `memguard` | The daemon, systemd service, D-Bus policy, and default config. |
| `memguard-system-tune` | One-shot tuning script and systemd service. |

## Installation from COPR

```bash
sudo dnf copr enable sevheng/memguard
sudo dnf install memguard memguard-system-tune
sudo systemctl enable --now memguard
```

## Installation from local RPMs

Build the RPMs on a Fedora host:

```bash
make rpm
```

Then install:

```bash
sudo dnf install \
  ~/rpmbuild/RPMS/x86_64/memguard-*.rpm \
  ~/rpmbuild/RPMS/noarch/memguard-system-tune-*.rpm
sudo systemctl enable --now memguard
```

## Build from source

```bash
cargo build --release
sudo cp target/release/memguard /usr/local/bin/
sudo cp memguard.service /etc/systemd/system/
sudo cp dbus/memguard.conf /etc/dbus-1/system.d/
sudo mkdir -p /etc/memguard
sudo cp config.toml /etc/memguard/
sudo systemctl daemon-reload
sudo systemctl enable --now memguard
```

## Usage

```bash
sudo systemctl status memguard
sudo journalctl -u memguard -f
sudo tail -f /var/log/memguard/events.jsonl
```

## Configuration

Edit `/etc/memguard/config.toml`:

```toml
[pressure]
poll_ms = 500

[policy]
freeze_on_critical = true
kill_delay_seconds = 5
recovery_seconds = 10

[events]
log_path = "/var/log/memguard/events.jsonl"
```

Restart after changes:

```bash
sudo systemctl restart memguard
```

## Stress testing

See [`docs/test-report-template.md`](docs/test-report-template.md) for a manual
N150 / low-end hardware stress-test protocol.

A root-only automated integration test is also included:

```bash
sudo cargo test --test stress_test -- --ignored
```

## Documentation

- [`docs/user-guide.md`](docs/user-guide.md) — install, configure, troubleshoot.
- [`docs/architecture.md`](docs/architecture.md) — components and data flow.
- [`docs/test-report-template.md`](docs/test-report-template.md) — manual stress
  test protocol.

## License

MIT. See `LICENSE`.
