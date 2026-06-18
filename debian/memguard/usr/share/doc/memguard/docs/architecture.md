# memguard Architecture

## Components

| Component | Responsibility |
|-----------|----------------|
| `pressure` | Reads PSI memory pressure and classifies `Normal`/`Warning`/`Critical`. |
| `desktop` | Discovers the graphical session, desktop environment, and active window PID. |
| `inventory` | Scans cgroups under `user.slice` and classifies apps as `Shell`, `Active`, or `Background`. |
| `policy` | Decides which mitigations to apply for a given pressure level. |
| `actor` | Executes cgroup freeze/throttle/kill and `oom_score_adj` shielding. |
| `events` | Emits structured JSON events for every pressure change and mitigation. |

## Data flow

1. `main.rs` wakes on a timer.
2. `Desktop::discover()` finds the shell PID and active app.
3. `Inventory::scan()` builds a list of `CgroupApp` entries.
4. `PressureMonitor::level()` reads `/proc/pressure/memory`.
5. `Policy::decide()` returns actions based on level and app classes.
6. `Actor::execute()` applies cgroup/oom adjustments and emits events.

## Mitigation rules

- **Normal**: thaw previously frozen cgroups.
- **Warning**: throttle background cgroups.
- **Critical**: shield shell/active PIDs, freeze background cgroups, and kill
  the largest background cgroup if critical pressure persists for
  `kill_delay_seconds`.

## cgroup model

memguard expects a cgroup v2 hierarchy mounted at `/sys/fs/cgroup`. It acts on
`app.slice`/`session-*.scope` cgroups created by systemd user sessions.
