# memguard Technical Design

## 1. Purpose

`memguard` is a root system daemon that makes Linux desktop memory management as reliable as macOS/Windows: it protects the desktop shell, freezes background applications, and only kills low-priority cgroups when memory pressure becomes critical — all without kernel changes or application cooperation.

This document specifies the initial implementation (Phases 1–2 from `project-purpose.md`). The `memguard-system-tune` companion script is out of scope for this design and will be specified separately.

## 2. Goals and Non-Goals

### Goals

- Desktop shell survives 100% of memory pressure events.
- Active window process is never killed.
- Background applications are frozen before being killed.
- System slows down before crashing.
- Zero configuration: install, enable systemd service, run.
- Minimal footprint: <15 MB RAM, <1% CPU.

### Non-Goals

- GUI, tray icon, notifications.
- Kernel patches or modules.
- Per-application memory optimization.
- Browser extension integration.
- Desktop environments other than GNOME and KDE Plasma in Phase 1/2.

## 3. Architecture

`memguard` is a single Rust system daemon running as root. It targets **cgroup v2** systems only (PSI and `cgroup.kill` require it). It reads system state, classifies desktop cgroups, and applies cgroup-level actions.

```
┌─────────────────────────────────────────────────────────────┐
│                         memguard                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │ pressure │  │ desktop  │  │inventory │  │   policy   │  │
│  │  input   │  │  input   │  │  input   │  │  decision  │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬──────┘  │
│       │             │             │              │          │
│       └─────────────┴─────────────┘              │          │
│                     │                            │          │
│                     ▼                            ▼          │
│              State snapshot              Action decision     │
│                     │                            │          │
│                     └─────────────┬──────────────┘          │
│                                   ▼                          │
│                            ┌──────────┐                     │
│                            │  actor   │                     │
│                            │  output  │                     │
│                            └────┬─────┘                     │
│                                 ▼                          │
│                      cgroup freeze/throttle/kill            │
│                      oom_score_adj updates                  │
└─────────────────────────────────────────────────────────────┘
```

## 4. Components

### 4.1 `pressure`

- Polls `/proc/pressure/memory` every 500 ms.
- Parses `some avg10`, `some avg60`, `full avg10`, `full avg60`.
- Emits `PressureLevel` events: `Normal`, `Warning`, `Critical`.
- Thresholds are configurable but default to:
  - `Warning`: `some avg10 >= 30`
  - `Critical`: `some avg10 >= 70` or `full avg10 >= 50`

### 4.2 `desktop`

- Uses systemd-logind to discover the active graphical session and its user.
- Resolves the desktop shell PID by inspecting the session cgroup and matching known names (`gnome-shell`, `kwin_wayland`, `kwin_x11`).
- Connects to the user's D-Bus session bus by resolving the session bus address from logind or the session environment.
  - GNOME: `org.gnome.Shell` `GetActiveWindow` / window-created signals.
  - KDE Plasma: `org.kde.KWin` / `org.kde.plasmashell` active window APIs.
- Emits `DesktopEvent` containing shell PID and active application PID/cgroup.

### 4.3 `inventory`

- Scans `/sys/fs/cgroup/user.slice/user-*.slice/session-*.scope`.
- Maps each cgroup to an `AppId` derived from the leading process name.
- Classifies each cgroup:
  - `Shell`: the desktop compositor / shell.
  - `Active`: the currently focused application.
  - `Background`: other user applications.
  - `System`: system services outside the user slice.
- Emits a snapshot every 2 seconds and on demand.

### 4.4 `policy`

- Maintains a sorted list of cgroups by protectiveness:
  1. Shell
  2. Active
  3. Background
  4. System (never touched)
- On `Warning`:
  - Pre-computes candidate cgroups.
  - Applies CPU throttle (`cpu.max` clamp to 50% of one CPU) to Background cgroups.
- On `Critical`:
  - Freezes all Background cgroups.
  - Sets `oom_score_adj = -1000` on Shell and Active processes.
  - If pressure persists for more than 5 seconds, kills the lowest-priority Background cgroup. Priority is determined first by classification (Background), then by resident memory size (largest first).
- On recovery to `Normal` for 10 seconds:
  - Unfreezes Background cgroups.
  - Restores `oom_score_adj` values.
  - Removes CPU throttles.
- All actions are debounced to prevent flapping.

### 4.5 `actor`

- Executes filesystem writes:
  - `cgroup.freeze` (0/1)
  - `cpu.max` (throttle / restore)
  - `cgroup.kill` (cgroup v2)
  - `/proc/<pid>/oom_score_adj`
- Logs every action with timestamp, target, reason, and result.
- Refuses to operate on:
  - Its own cgroup.
  - System cgroups outside `user.slice`.
  - Any cgroup containing PID 1 or the daemon itself.

## 5. Data Flow

1. **Startup**
   - Parse `/etc/memguard/config.toml` (optional).
   - `desktop` discovers session and shell PID.
   - `inventory` takes a baseline snapshot.
   - `pressure` begins polling.

2. **Normal operation**
   - Pressure stays `Normal`.
   - Inventory and desktop events update classification continuously.

3. **Warning pressure**
   - Policy pre-computes kill candidates.
   - Actor throttles CPU on Background cgroups.

4. **Critical pressure**
   - Actor freezes all Background cgroups.
   - Actor sets `oom_score_adj = -1000` on Shell and Active processes.
   - If pressure persists >5 s, actor kills the lowest-priority Background cgroup.

5. **Recovery**
   - When pressure returns to `Normal` for 10 s, actor thaws cgroups and restores `oom_score_adj`.

## 6. Error Handling

| Scenario | Behavior |
|----------|----------|
| D-Bus session unavailable | Log warning; fall back to a minimal `/proc` heuristic for active window classification. |
| Shell PID not found | Skip `oom_score_adj` shielding; do not kill cgroups that could contain the shell. |
| cgroup freeze fails | Mark cgroup as unfreezable; escalate directly to kill on next cycle. |
| Kill fails with `EPERM` | Log and blacklist cgroup for 60 s. |
| Daemon self-protection | Daemon sets its own `oom_score_adj = -1000` and never operates on its own cgroup. |
| Event loop hang | systemd watchdog restarts the service. |

## 7. Configuration

`memguard` ships with defaults and reads an optional `/etc/memguard/config.toml`:

```toml
[pressure]
poll_ms = 500
warning_some_avg10 = 30.0
critical_some_avg10 = 70.0
critical_full_avg10 = 50.0

[policy]
freeze_on_critical = true
kill_delay_seconds = 5
recovery_seconds = 10

[desktop]
supported = ["gnome", "kde"]
```

## 8. Packaging and Deployment

- Rust crate `memguard`.
- systemd unit `memguard.service` running as root.
- D-Bus policy file granting root access to the user session bus.
- COPR/RPM packaging in Phase 3.
- License: MIT.

## 9. Testing

- **Unit tests:** pressure parser, config parser, policy scoring, cgroup path utilities (`cargo test`).
- **Integration tests:** fake cgroupfs in `/tmp/memguard-test-cgroup` to verify freeze/throttle/kill without touching real cgroups.
- **Mock D-Bus tests:** `zbus` mock objects for GNOME/KDE active-window signals.
- **Manual stress test:** 50 Chrome tabs + VS Code + Docker on 8 GB RAM.
- **CI:** GitHub Actions runs unit + integration tests on stable Rust.

## 10. Decisions

- **Privilege model:** Single root daemon (Approach A).
- **Desktop support:** GNOME and KDE Plasma only.
- **Active window detection:** D-Bus desktop APIs.
- **License:** MIT.
- **Companion script:** `memguard-system-tune` deferred to a separate spec.

## 11. Open Questions

None remaining; all architectural decisions have been made.
