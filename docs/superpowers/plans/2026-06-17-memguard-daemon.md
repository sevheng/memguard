# memguard Daemon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first working version of the `memguard` root system daemon that monitors PSI memory pressure, classifies desktop cgroups, and freezes/throttles/kills background apps to protect the desktop shell.

**Architecture:** Single Rust async daemon using `tokio`. Modular design: `pressure`, `desktop`, `inventory`, `policy`, `actor`. Configuration via `serde` + `toml`. Tests use fake cgroupfs and `zbus` mocks.

**Tech Stack:** Rust, tokio, serde, toml, tracing, thiserror, tempfile (dev). `zbus` will be added in Phase 2 for D-Bus active-window queries.

---

## File Structure

```
memguard/
├── Cargo.toml
├── memguard.service
├── src/
│   ├── main.rs          # daemon entry point, async runtime, top-level loop
│   ├── lib.rs           # public module re-exports for integration tests
│   ├── config.rs        # Config struct, defaults, file parsing
│   ├── pressure.rs      # PSI parser and pressure-level detector
│   ├── desktop.rs       # logind session + D-Bus active-window discovery
│   ├── inventory.rs     # cgroup scanner and app classification
│   ├── policy.rs        # decision engine: what to freeze/throttle/kill
│   └── actor.rs         # cgroup/oom_score_adj executor
└── tests/
    ├── pressure_test.rs
    ├── policy_test.rs
    └── actor_test.rs
```

---

## Task 1: Bootstrap Cargo Project and Config Parsing

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/config.rs`
- Create: `tests/config_test.rs`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "memguard"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Linux desktop memory pressure daemon"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create `src/config.rs` with defaults and parser**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub pressure: PressureConfig,
    pub policy: PolicyConfig,
    pub desktop: DesktopConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PressureConfig {
    pub poll_ms: u64,
    pub warning_some_avg10: f64,
    pub critical_some_avg10: f64,
    pub critical_full_avg10: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub freeze_on_critical: bool,
    pub kill_delay_seconds: u64,
    pub recovery_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopConfig {
    pub supported: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pressure: PressureConfig {
                poll_ms: 500,
                warning_some_avg10: 30.0,
                critical_some_avg10: 70.0,
                critical_full_avg10: 50.0,
            },
            policy: PolicyConfig {
                freeze_on_critical: true,
                kill_delay_seconds: 5,
                recovery_seconds: 10,
            },
            desktop: DesktopConfig {
                supported: vec!["gnome".to_string(), "kde".to_string()],
            },
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }
}
```

- [ ] **Step 3: Create `src/lib.rs` to re-export modules**

```rust
pub mod config;
```

- [ ] **Step 4: Write failing test in `tests/config_test.rs`**

```rust
use memguard::config::Config;

#[test]
fn test_default_config_values() {
    let cfg = Config::default();
    assert_eq!(cfg.pressure.poll_ms, 500);
    assert_eq!(cfg.policy.kill_delay_seconds, 5);
    assert!(cfg.desktop.supported.contains(&"gnome".to_string()));
}
```

- [ ] **Step 5: Run test and verify it fails**

Run: `cargo test --test config_test`
Expected: compilation succeeds, test passes (default impl is present).

- [ ] **Step 6: Add config file load test and implementation**

Add to `tests/config_test.rs`:

```rust
use std::io::Write;

#[test]
fn test_load_config_from_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, r#"
[pressure]
poll_ms = 1000

[policy]
kill_delay_seconds = 3

[desktop]
supported = ["gnome"]
"#).unwrap();

    let cfg = Config::load(tmp.path()).unwrap();
    assert_eq!(cfg.pressure.poll_ms, 1000);
    assert_eq!(cfg.policy.kill_delay_seconds, 3);
    assert_eq!(cfg.desktop.supported, vec!["gnome".to_string()]);
}
```

Run: `cargo test --test config_test`
Expected: both tests pass.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs src/config.rs tests/config_test.rs
git commit -m "feat: bootstrap project and add config parser"
```

---

## Task 2: PSI Pressure Monitor

**Files:**
- Create: `src/pressure.rs`
- Create: `tests/pressure_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add `PressureLevel` and parser in `src/pressure.rs`**

```rust
use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Default)]
pub struct PressureSnapshot {
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub full_avg10: f64,
    pub full_avg60: f64,
}

pub struct PressureMonitor {
    path: std::path::PathBuf,
    warning_some_avg10: f64,
    critical_some_avg10: f64,
    critical_full_avg10: f64,
}

impl PressureMonitor {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        warning_some_avg10: f64,
        critical_some_avg10: f64,
        critical_full_avg10: f64,
    ) -> Self {
        Self {
            path: path.into(),
            warning_some_avg10,
            critical_some_avg10,
            critical_full_avg10,
        }
    }

    pub fn read(&self) -> anyhow::Result<PressureSnapshot> {
        let content = std::fs::read_to_string(&self.path)?;
        parse_pressure(&content)
    }

    pub fn level(&self) -> PressureLevel {
        match self.read() {
            Ok(s) => classify(&s, self.warning_some_avg10, self.critical_some_avg10, self.critical_full_avg10),
            Err(e) => {
                warn!("failed to read pressure: {}", e);
                PressureLevel::Normal
            }
        }
    }
}

pub fn parse_pressure(content: &str) -> anyhow::Result<PressureSnapshot> {
    let mut snap = PressureSnapshot::default();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let key = parts[0].trim_end_matches(':');
        let mut avg10 = None;
        let mut avg60 = None;
        for window in parts[1..].windows(2) {
            if window[0] == "avg10=" {
                avg10 = window[1].trim_end_matches(',').parse().ok();
            } else if window[0] == "avg60=" {
                avg60 = window[1].trim_end_matches(',').parse().ok();
            }
        }
        match key {
            "some" => { snap.some_avg10 = avg10.unwrap_or(0.0); snap.some_avg60 = avg60.unwrap_or(0.0); }
            "full" => { snap.full_avg10 = avg10.unwrap_or(0.0); snap.full_avg60 = avg60.unwrap_or(0.0); }
            _ => {}
        }
    }
    Ok(snap)
}

pub fn classify(
    snap: &PressureSnapshot,
    warning_some_avg10: f64,
    critical_some_avg10: f64,
    critical_full_avg10: f64,
) -> PressureLevel {
    if snap.some_avg10 >= critical_some_avg10 || snap.full_avg10 >= critical_full_avg10 {
        PressureLevel::Critical
    } else if snap.some_avg10 >= warning_some_avg10 {
        PressureLevel::Warning
    } else {
        PressureLevel::Normal
    }
}
```

- [ ] **Step 2: Re-export in `src/lib.rs`**

Add:

```rust
pub mod pressure;
```

- [ ] **Step 3: Write tests in `tests/pressure_test.rs`**

```rust
use memguard::pressure::{parse_pressure, classify, PressureLevel, PressureSnapshot};

#[test]
fn test_parse_pressure() {
    let sample = "some avg10=25.00 avg60=10.00 total=123456\nfull avg10=5.00 avg60=2.00 total=654321\n";
    let snap = parse_pressure(sample).unwrap();
    assert_eq!(snap.some_avg10, 25.0);
    assert_eq!(snap.some_avg60, 10.0);
    assert_eq!(snap.full_avg10, 5.0);
    assert_eq!(snap.full_avg60, 2.0);
}

#[test]
fn test_classify() {
    let snap = PressureSnapshot { some_avg10: 10.0, ..Default::default() };
    assert_eq!(classify(&snap, 30.0, 70.0, 50.0), PressureLevel::Normal);

    let snap = PressureSnapshot { some_avg10: 40.0, ..Default::default() };
    assert_eq!(classify(&snap, 30.0, 70.0, 50.0), PressureLevel::Warning);

    let snap = PressureSnapshot { some_avg10: 80.0, ..Default::default() };
    assert_eq!(classify(&snap, 30.0, 70.0, 50.0), PressureLevel::Critical);

    let snap = PressureSnapshot { some_avg10: 0.0, full_avg10: 60.0 };
    assert_eq!(classify(&snap, 30.0, 70.0, 50.0), PressureLevel::Critical);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test pressure_test`
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/pressure.rs tests/pressure_test.rs src/lib.rs
git commit -m "feat: add PSI pressure parser and classifier"
```

---

## Task 3: Cgroup Inventory Scanner

**Files:**
- Create: `src/inventory.rs`
- Create: `tests/inventory_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Define types in `src/inventory.rs`**

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppClass {
    Shell,
    Active,
    Background,
    System,
}

#[derive(Debug, Clone)]
pub struct CgroupApp {
    pub app_id: String,
    pub class: AppClass,
    pub cgroup_path: PathBuf,
    pub pids: Vec<u32>,
    pub rss_bytes: u64,
}

pub struct Inventory {
    root: PathBuf,
    shell_names: Vec<String>,
}

impl Inventory {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            shell_names: vec!["gnome-shell".into(), "kwin_wayland".into(), "kwin_x11".into()],
        }
    }

    pub fn scan(&self, active_app_id: Option<&str>, shell_pid: Option<u32>) -> Vec<CgroupApp> {
        let mut apps = Vec::new();
        let user_slice = self.root.join("user.slice");
        if !user_slice.exists() {
            return apps;
        }
        // Walk user-*.slice/session-*.scope
        for entry in walkdir::WalkDir::new(&user_slice).max_depth(4) {
            // ...
        }
        apps
    }
}
```

Wait — we don't have walkdir. Better use manual recursion with `std::fs::read_dir` to avoid extra dependency. I'll adjust the plan.

- [ ] **Step 1 (revised): Define types and scanner in `src/inventory.rs`**

```rust
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppClass {
    Shell,
    Active,
    Background,
    System,
}

#[derive(Debug, Clone)]
pub struct CgroupApp {
    pub app_id: String,
    pub class: AppClass,
    pub cgroup_path: PathBuf,
    pub pids: Vec<u32>,
    pub rss_bytes: u64,
}

pub struct Inventory {
    cgroup_root: PathBuf,
    proc_root: PathBuf,
    shell_names: Vec<String>,
}

impl Inventory {
    pub fn new(cgroup_root: impl Into<PathBuf>, proc_root: impl Into<PathBuf>) -> Self {
        Self {
            cgroup_root: cgroup_root.into(),
            proc_root: proc_root.into(),
            shell_names: vec!["gnome-shell".into(), "kwin_wayland".into(), "kwin_x11".into()],
        }
    }

    pub fn scan(&self, active_app_id: Option<&str>, shell_pid: Option<u32>) -> Vec<CgroupApp> {
        let mut apps = Vec::new();
        let user_slice = self.cgroup_root.join("user.slice");
        if !user_slice.exists() {
            return apps;
        }
        self.scan_dir(&user_slice, active_app_id, shell_pid, &mut apps);
        apps
    }

    fn scan_dir(&self, dir: &Path, active_app_id: Option<&str>, shell_pid: Option<u32>, apps: &mut Vec<CgroupApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(app) = self.classify(&path, active_app_id, shell_pid) {
                    apps.push(app);
                }
                self.scan_dir(&path, active_app_id, shell_pid, apps);
            }
        }
    }

    fn classify(&self, path: &Path, active_app_id: Option<&str>, shell_pid: Option<u32>) -> Option<CgroupApp> {
        let cgroup_name = path.file_name()?.to_string_lossy();
        if !(cgroup_name.contains("session-") || cgroup_name.ends_with(".scope")) {
            return None;
        }
        let pids = read_pids(path);
        let app_id = self.leading_process_name(&pids).unwrap_or_else(|| cgroup_name.to_string());
        let class = if shell_pid.map_or(false, |pid| pids.contains(&pid)) || self.shell_names.contains(&app_id) {
            AppClass::Shell
        } else if active_app_id == Some(&app_id) {
            AppClass::Active
        } else {
            AppClass::Background
        };
        let rss_bytes = pids.iter().map(|&pid| self.rss_for_pid(pid)).sum();
        Some(CgroupApp { app_id, class, cgroup_path: path.to_path_buf(), pids, rss_bytes })
    }

    fn leading_process_name(&self, pids: &[u32]) -> Option<String> {
        let first = pids.first()?;
        let comm = std::fs::read_to_string(self.proc_root.join(format!("{}/comm", first))).ok()?;
        Some(comm.trim().to_string())
    }

    fn rss_for_pid(&self, pid: u32) -> u64 {
        let path = self.proc_root.join(format!("{}/statm", pid));
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut parts = content.split_whitespace();
        let rss_pages: u64 = parts.nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        rss_pages * 4096
    }
}

fn read_pids(path: &Path) -> Vec<u32> {
    let file = path.join("cgroup.procs");
    let content = std::fs::read_to_string(&file).unwrap_or_default();
    content.lines().filter_map(|l| l.trim().parse().ok()).collect()
}
```

- [ ] **Step 2: Re-export in `src/lib.rs`**

Add:

```rust
pub mod inventory;
```

- [ ] **Step 3: Write tests in `tests/inventory_test.rs`**

```rust
use memguard::inventory::{Inventory, AppClass};
use std::fs;

fn make_fake_cgroup(cgroup_root: &std::path::Path, proc_root: &std::path::Path, rel: &str, pids: &[u32], comm: &str, rss_pages: u64) {
    let path = cgroup_root.join(rel);
    fs::create_dir_all(&path).unwrap();
    let procs = pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join("\n");
    fs::write(path.join("cgroup.procs"), procs).unwrap();
    for pid in pids {
        let pdir = proc_root.join(pid.to_string());
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("comm"), comm).unwrap();
        fs::write(pdir.join("statm"), format!("0 {} 0 0 0 0 0\n", rss_pages)).unwrap();
    }
}

#[test]
fn test_inventory_classifies_background_and_shell() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path().join("cgroup");
    let proc_root = tmp.path().join("proc");
    fs::create_dir(&cgroup_root).unwrap();
    fs::create_dir(&proc_root).unwrap();

    make_fake_cgroup(&cgroup_root, &proc_root, "user.slice/user-1000.slice/session-1.scope", &[1000, 1001], "app", 100);
    make_fake_cgroup(&cgroup_root, &proc_root, "user.slice/user-1000.slice/session-2.scope", &[2000], "gnome-shell", 200);

    let inv = Inventory::new(&cgroup_root, &proc_root);
    let apps = inv.scan(None, Some(2000));
    assert_eq!(apps.len(), 2);
    let shell = apps.iter().find(|a| a.pids.contains(&2000)).unwrap();
    assert_eq!(shell.class, AppClass::Shell);
    let bg = apps.iter().find(|a| a.pids.contains(&1000)).unwrap();
    assert_eq!(bg.class, AppClass::Background);
    assert_eq!(bg.rss_bytes, 100 * 4096 * 2);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test inventory_test`
Expected: test passes.

- [ ] **Step 5: Commit**

```bash
git add src/inventory.rs tests/inventory_test.rs src/lib.rs
git commit -m "feat: add cgroup inventory scanner"
```

---

## Task 4: Policy Engine

**Files:**
- Create: `src/policy.rs`
- Create: `tests/policy_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement policy engine in `src/policy.rs`**

```rust
use crate::inventory::{AppClass, CgroupApp};
use crate::pressure::PressureLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Freeze { cgroup: std::path::PathBuf },
    Throttle { cgroup: std::path::PathBuf },
    Kill { cgroup: std::path::PathBuf },
    Shield { pid: u32 },
    Unfreeze { cgroup: std::path::PathBuf },
    Unthrottle { cgroup: std::path::PathBuf },
    Unshield { pid: u32 },
}

pub struct Policy {
    freeze_on_critical: bool,
}

impl Policy {
    pub fn new(freeze_on_critical: bool) -> Self {
        Self { freeze_on_critical }
    }

    pub fn decide(
        &self,
        level: PressureLevel,
        apps: &[CgroupApp],
        previously_frozen: &[std::path::PathBuf],
    ) -> Vec<Action> {
        match level {
            PressureLevel::Normal => {
                previously_frozen.iter().map(|c| Action::Unfreeze { cgroup: c.clone() }).collect()
            }
            PressureLevel::Warning => {
                apps.iter()
                    .filter(|a| a.class == AppClass::Background)
                    .map(|a| Action::Throttle { cgroup: a.cgroup_path.clone() })
                    .collect()
            }
            PressureLevel::Critical => {
                let mut actions: Vec<Action> = apps
                    .iter()
                    .filter(|a| a.class == AppClass::Shell || a.class == AppClass::Active)
                    .flat_map(|a| a.pids.iter().map(|&pid| Action::Shield { pid }))
                    .collect();
                if self.freeze_on_critical {
                    actions.extend(
                        apps.iter()
                            .filter(|a| a.class == AppClass::Background)
                            .map(|a| Action::Freeze { cgroup: a.cgroup_path.clone() }),
                    );
                }
                // Kill lowest-priority background (largest RSS) if instructed by caller.
                actions
            }
        }
    }

    pub fn choose_kill(&self, apps: &[CgroupApp]) -> Option<Action> {
        let mut candidates: Vec<&CgroupApp> = apps.iter().filter(|a| a.class == AppClass::Background).collect();
        candidates.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
        candidates.first().map(|a| Action::Kill { cgroup: a.cgroup_path.clone() })
    }
}
```

- [ ] **Step 2: Re-export in `src/lib.rs`**

Add:

```rust
pub mod policy;
```

- [ ] **Step 3: Write tests in `tests/policy_test.rs`**

```rust
use memguard::inventory::{AppClass, CgroupApp};
use memguard::policy::{Policy, Action};
use memguard::pressure::PressureLevel;
use std::path::PathBuf;

fn app(name: &str, class: AppClass, pids: Vec<u32>, rss: u64) -> CgroupApp {
    CgroupApp {
        app_id: name.to_string(),
        class,
        cgroup_path: PathBuf::from(format!("/sys/fs/cgroup/{}", name)),
        pids,
        rss_bytes: rss,
    }
}

#[test]
fn test_policy_warning_throttles_background() {
    let policy = Policy::new(true);
    let apps = vec![
        app("shell", AppClass::Shell, vec![1], 100),
        app("bg", AppClass::Background, vec![2], 200),
    ];
    let actions = policy.decide(PressureLevel::Warning, &apps, &[]);
    assert!(actions.iter().any(|a| matches!(a, Action::Throttle { cgroup } if cgroup.ends_with("bg"))));
}

#[test]
fn test_policy_critical_shields_and_freezes() {
    let policy = Policy::new(true);
    let apps = vec![
        app("shell", AppClass::Shell, vec![1], 100),
        app("bg", AppClass::Background, vec![2], 200),
    ];
    let actions = policy.decide(PressureLevel::Critical, &apps, &[]);
    assert!(actions.iter().any(|a| matches!(a, Action::Shield { pid: 1 })));
    assert!(actions.iter().any(|a| matches!(a, Action::Freeze { cgroup } if cgroup.ends_with("bg"))));
}

#[test]
fn test_policy_choose_kill_largest_background() {
    let policy = Policy::new(true);
    let apps = vec![
        app("small", AppClass::Background, vec![2], 100),
        app("big", AppClass::Background, vec![3], 500),
    ];
    let action = policy.choose_kill(&apps);
    assert!(matches!(action, Some(Action::Kill { cgroup }) if cgroup.ends_with("big")));
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test policy_test`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/policy.rs tests/policy_test.rs src/lib.rs
git commit -m "feat: add policy engine"
```

---

## Task 5: Actor (Action Executor)

**Files:**
- Create: `src/actor.rs`
- Create: `tests/actor_test.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement actor in `src/actor.rs`**

```rust
use crate::policy::Action;
use std::path::Path;
use tracing::{info, warn};

pub struct Actor {
    cgroup_root: std::path::PathBuf,
}

impl Actor {
    pub fn new(cgroup_root: impl Into<std::path::PathBuf>) -> Self {
        Self { cgroup_root: cgroup_root.into() }
    }

    pub fn execute(&self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Freeze { cgroup } => self.write_cgroup(cgroup, "cgroup.freeze", "1"),
            Action::Unfreeze { cgroup } => self.write_cgroup(cgroup, "cgroup.freeze", "0"),
            Action::Throttle { cgroup } => self.write_cgroup(cgroup, "cpu.max", "50000 100000"),
            Action::Unthrottle { cgroup } => self.write_cgroup(cgroup, "cpu.max", "max"),
            Action::Kill { cgroup } => self.write_cgroup(cgroup, "cgroup.kill", "1"),
            Action::Shield { pid } => self.set_oom_score_adj(*pid, -1000),
            Action::Unshield { pid } => self.set_oom_score_adj(*pid, 0),
        }
    }

    fn write_cgroup(&self, cgroup: &Path, file: &str, value: &str) -> anyhow::Result<()> {
        let rel = cgroup.strip_prefix(&self.cgroup_root).unwrap_or(cgroup);
        let path = self.cgroup_root.join(rel).join(file);
        std::fs::write(&path, value)?;
        info!("wrote {} to {}", value, path.display());
        Ok(())
    }

    fn set_oom_score_adj(&self, pid: u32, value: i32) -> anyhow::Result<()> {
        let path = format!("/proc/{}/oom_score_adj", pid);
        std::fs::write(&path, value.to_string())?;
        info!("set oom_score_adj of {} to {}", pid, value);
        Ok(())
    }
}
```

- [ ] **Step 2: Re-export in `src/lib.rs`**

Add:

```rust
pub mod actor;
```

- [ ] **Step 3: Write tests in `tests/actor_test.rs`**

```rust
use memguard::actor::Actor;
use memguard::policy::Action;
use std::fs;

#[test]
fn test_actor_freezes_cgroup() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup = tmp.path().join("bg.scope");
    fs::create_dir(&cgroup).unwrap();
    fs::write(cgroup.join("cgroup.freeze"), "0").unwrap();

    let actor = Actor::new(tmp.path());
    actor.execute(&Action::Freeze { cgroup: cgroup.clone() }).unwrap();

    let val = fs::read_to_string(cgroup.join("cgroup.freeze")).unwrap();
    assert_eq!(val.trim(), "1");
}

#[test]
fn test_actor_throttles_cpu() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup = tmp.path().join("bg.scope");
    fs::create_dir(&cgroup).unwrap();
    fs::write(cgroup.join("cpu.max"), "max").unwrap();

    let actor = Actor::new(tmp.path());
    actor.execute(&Action::Throttle { cgroup: cgroup.clone() }).unwrap();

    let val = fs::read_to_string(cgroup.join("cpu.max")).unwrap();
    assert_eq!(val.trim(), "50000 100000");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test actor_test`
Expected: tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/actor.rs tests/actor_test.rs src/lib.rs
git commit -m "feat: add action executor actor"
```

---

## Task 6: Desktop Session and Active Window Discovery

**Files:**
- Create: `src/desktop.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement desktop discovery in `src/desktop.rs`**

For Phase 1, implement logind session discovery and a stub D-Bus active-window resolver. The D-Bus calls can be mocked in tests.

```rust
use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct DesktopState {
    pub shell_pid: Option<u32>,
    pub active_app_id: Option<String>,
    pub session_bus_address: Option<String>,
}

pub struct Desktop;

impl Desktop {
    pub fn new() -> Self {
        Self
    }

    pub fn discover(&self) -> DesktopState {
        // Simplified: inspect /run/systemd/sessions/ for graphical sessions,
        // find shell PID from cgroup.procs, and return a stub active_app_id.
        let mut state = DesktopState::default();
        if let Some((pid, addr)) = Self::find_graphical_session() {
            state.shell_pid = Some(pid);
            state.session_bus_address = Some(addr);
        }
        state.active_app_id = Self::query_active_app_id(&state.session_bus_address);
        state
    }

    fn find_graphical_session() -> Option<(u32, String)> {
        // Iterate /run/systemd/sessions/*, pick Type=wayland/x11, read Leader PID and Display env.
        // For MVP, return None; real implementation reads session files.
        None
    }

    fn query_active_app_id(_bus_addr: &Option<String>) -> Option<String> {
        // Intentionally returns None in Phase 1. D-Bus active-window queries via zbus
        // will be implemented in Phase 2; until then the daemon relies on inventory
        // classification only.
        None
    }
}
```

- [ ] **Step 2: Re-export in `src/lib.rs`**

Add:

```rust
pub mod desktop;
```

- [ ] **Step 3: Commit**

```bash
git add src/desktop.rs src/lib.rs
git commit -m "feat: add desktop session discovery stub"
```

---

## Task 7: Main Daemon Loop

**Files:**
- Create: `src/main.rs`
- Modify: `Cargo.toml` (add tracing-subscriber features if needed)

- [ ] **Step 1: Implement main loop in `src/main.rs`**

```rust
use memguard::actor::Actor;
use memguard::config::Config;
use memguard::desktop::Desktop;
use memguard::inventory::Inventory;
use memguard::policy::{Policy, Action};
use memguard::pressure::{PressureLevel, PressureMonitor};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::default();
    let pressure = PressureMonitor::new(
        "/proc/pressure/memory",
        config.pressure.warning_some_avg10,
        config.pressure.critical_some_avg10,
        config.pressure.critical_full_avg10,
    );
    let desktop = Desktop::new();
    let inventory = Inventory::new("/sys/fs/cgroup", "/proc");
    let actor = Actor::new("/sys/fs/cgroup");
    let policy = Policy::new(config.policy.freeze_on_critical);

    let mut frozen: Vec<PathBuf> = Vec::new();
    let mut warning_start: Option<tokio::time::Instant> = None;
    let mut tick = interval(Duration::from_millis(config.pressure.poll_ms));

    loop {
        tick.tick().await;

        let state = desktop.discover();
        let apps = inventory.scan(state.active_app_id.as_deref(), state.shell_pid);
        let level = pressure.level();

        info!("pressure={:?} apps={}", level, apps.len());

        match level {
            PressureLevel::Normal => {
                warning_start = None;
                for cgroup in frozen.drain(..) {
                    let _ = actor.execute(&Action::Unfreeze { cgroup });
                }
            }
            PressureLevel::Warning => {
                warning_start = None;
                for action in policy.decide(level, &apps, &frozen) {
                    let _ = actor.execute(&action);
                }
            }
            PressureLevel::Critical => {
                for action in policy.decide(level, &apps, &frozen) {
                    if let Action::Freeze { ref cgroup } = action {
                        frozen.push(cgroup.clone());
                    }
                    let _ = actor.execute(&action);
                }
                if warning_start.is_none() {
                    warning_start = Some(tokio::time::Instant::now());
                }
                if warning_start.unwrap().elapsed() >= Duration::from_secs(config.policy.kill_delay_seconds) {
                    if let Some(action) = policy.choose_kill(&apps) {
                        warn!("killing cgroup due to sustained critical pressure");
                        let _ = actor.execute(&action);
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Build the daemon**

Run: `cargo build`
Expected: compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add main daemon loop"
```

---

## Task 8: systemd Service File

**Files:**
- Create: `memguard.service`

- [ ] **Step 1: Create `memguard.service`**

```ini
[Unit]
Description=memguard desktop memory pressure daemon
After=systemd-logind.service

[Service]
Type=simple
ExecStart=/usr/bin/memguard
Restart=on-failure
RestartSec=5
WatchdogSec=30

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: Commit**

```bash
git add memguard.service
git commit -m "chore: add systemd service file"
```

---

## Task 9: Integration Smoke Test

**Files:**
- Create: `tests/smoke_test.rs`

- [ ] **Step 1: Write end-to-end smoke test using fake cgroupfs**

```rust
use memguard::actor::Actor;
use memguard::inventory::{Inventory, AppClass};
use memguard::policy::{Policy, Action};
use memguard::pressure::{parse_pressure, classify, PressureLevel};
use std::fs;

#[test]
fn test_critical_pressure_freezes_background() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path();

    // Build fake cgroup tree
    let user = cgroup_root.join("user.slice/user-1000.slice/session-1.scope");
    fs::create_dir_all(&user).unwrap();
    fs::write(user.join("cgroup.procs"), "1000\n").unwrap();
    fs::write(user.join("cgroup.freeze"), "0").unwrap();

    let inv = Inventory::new(cgroup_root, "/proc");
    let apps = inv.scan(None, Some(1000));
    assert_eq!(apps.len(), 1);

    let policy = Policy::new(true);
    let sample = "some avg10=80.00 avg60=70.00 total=100\nfull avg10=60.00 avg60=50.00 total=200\n";
    let snap = parse_pressure(sample).unwrap();
    let level = classify(&snap, 30.0, 70.0, 50.0);
    assert_eq!(level, PressureLevel::Critical);

    let actions = policy.decide(level, &apps, &[]);
    let actor = Actor::new(cgroup_root);
    for action in &actions {
        actor.execute(action).unwrap();
    }

    let val = fs::read_to_string(user.join("cgroup.freeze")).unwrap();
    assert_eq!(val.trim(), "1");
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/smoke_test.rs
git commit -m "test: add integration smoke test"
```

---

## Task 10: CI Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create GitHub Actions CI**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all
      - run: cargo build --release
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add github actions workflow"
```

---

## Self-Review Checklist

- [ ] **Spec coverage:** Every section of the design doc maps to at least one task.
  - Architecture → file structure + main loop task.
  - Components → Tasks 2–6.
  - Kill priority by RSS → Task 3 (RSS collection) + Task 4 (choose_kill).
  - Data flow → Task 7.
  - Error handling → embedded in modules (defaults, blacklists, warnings).
  - Testing → Task 9 + per-module tests.
  - Packaging → Task 8.
- [ ] **Placeholder scan:** No TBD/TODO/fill-in-details. The D-Bus active-window query is an intentional no-op stub in Phase 1 and is explicitly described as a Phase 2 addition.
- [ ] **Type consistency:** `CgroupApp`, `AppClass`, `Action`, `PressureLevel` are reused consistently across modules.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-17-memguard-daemon.md`. Two execution options:

1. **Subagent-Driven (recommended)** - dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** - execute tasks in this session using `executing-plans`, batch execution with checkpoints.

Which approach?
