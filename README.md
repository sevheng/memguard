# memguard

Linux desktop memory pressure daemon.

memguard monitors system memory pressure and protects the active desktop shell
and foreground application by freezing or killing background cgroups before the
OOM killer is triggered.

## Installation

Enable the COPR repository and install the package:

```bash
sudo dnf copr enable @username/memguard
sudo dnf install memguard
```

## Starting the service

```bash
sudo systemctl enable --now memguard
```

## Checking status and logs

```bash
sudo systemctl status memguard
sudo journalctl -u memguard
```

## Configuration

Edit `/etc/memguard/config.toml` and restart the service:

```bash
sudo systemctl restart memguard
```

## License

MIT. See `LICENSE`.
