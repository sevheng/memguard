use memguard::actor::Actor;
use memguard::events::{Event, VecEventLog};
use memguard::inventory::Inventory;
use memguard::policy::Policy;
use memguard::pressure::{PressureLevel, PressureMonitor};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;

struct TestCgroup {
    path: PathBuf,
    children: Vec<Child>,
}

impl TestCgroup {
    fn new() -> anyhow::Result<Self> {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            anyhow::bail!("stress test must run as root");
        }
        if !Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
            anyhow::bail!("cgroup v2 not available");
        }

        let tmp = tempfile::TempDir::new()?;
        let path = Path::new("/sys/fs/cgroup").join(format!(
            "memguard-stress-{}",
            tmp.path().file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir(&path)?;
        let _ = fs::write(path.join("cgroup.subtree_control"), "+cpu +memory");

        Ok(Self {
            path,
            children: Vec::new(),
        })
    }

    fn spawn_scope(&mut self, name: &str) -> anyhow::Result<u32> {
        let scope = self.path.join(name);
        fs::create_dir(&scope)?;
        let child = Command::new("sleep").arg("60").spawn()?;
        let pid = child.id();
        fs::write(scope.join("cgroup.procs"), format!("{pid}\n"))?;
        self.children.push(child);
        Ok(pid)
    }
}

impl Drop for TestCgroup {
    fn drop(&mut self) {
        let _ = fs::write(self.path.join("cgroup.kill"), "1");
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_pressure(path: &Path, some_avg10: f64, full_avg10: f64) {
    let content = format!(
        "some avg10={:.2} avg60=0.00 avg300=0.00 total=0\nfull avg10={:.2} avg60=0.00 avg300=0.00 total=0\n",
        some_avg10, full_avg10
    );
    fs::write(path, content).unwrap();
}

#[ignore = "requires root and cgroup v2"]
#[test]
fn stress_test_protects_shell_and_freezes_background() {
    let mut cg = TestCgroup::new().unwrap();
    let shell_pid = cg.spawn_scope("shell.scope").unwrap();
    let _bg_pid = cg.spawn_scope("background.scope").unwrap();

    let tmp = tempfile::TempDir::new().unwrap();
    let pressure_path = tmp.path().join("pressure");
    write_pressure(&pressure_path, 0.0, 0.0);

    let event_log = Arc::new(VecEventLog::default());
    let actor = Actor::new("/sys/fs/cgroup", event_log.clone());
    let inventory = Inventory::new("/sys/fs/cgroup", "/proc");
    let policy = Policy::new(true);
    let pressure = PressureMonitor::new(&pressure_path, 30.0, 70.0, 50.0);

    // Normal: nothing should happen.
    let apps = inventory.scan_at(&cg.path, None, Some(shell_pid));
    let level = pressure.level();
    assert_eq!(level, PressureLevel::Normal);
    for action in policy.decide(level, &apps, &[]) {
        actor.execute(&action).unwrap();
    }
    assert!(!event_log
        .events()
        .iter()
        .any(|e| matches!(e, Event::AppShielded { .. })));

    // Warning: background should be throttled.
    write_pressure(&pressure_path, 50.0, 0.0);
    let apps = inventory.scan_at(&cg.path, None, Some(shell_pid));
    let level = pressure.level();
    assert_eq!(level, PressureLevel::Warning);
    for action in policy.decide(level, &apps, &[]) {
        actor.execute(&action).unwrap();
    }
    assert!(event_log.events().iter().any(|e| matches!(
        e,
        Event::AppThrottled { cgroup } if cgroup.contains("background.scope")
    )));

    // Critical: shell shielded, background frozen.
    write_pressure(&pressure_path, 80.0, 0.0);
    let apps = inventory.scan_at(&cg.path, None, Some(shell_pid));
    let level = pressure.level();
    assert_eq!(level, PressureLevel::Critical);
    let mut frozen = Vec::new();
    for action in policy.decide(level, &apps, &[]) {
        if let memguard::policy::Action::Freeze { ref cgroup } = action {
            frozen.push(cgroup.clone());
        }
        actor.execute(&action).unwrap();
    }
    assert!(event_log.events().iter().any(|e| matches!(
        e,
        Event::AppShielded { pid } if *pid == shell_pid
    )));
    assert!(event_log.events().iter().any(|e| matches!(
        e,
        Event::AppFrozen { cgroup } if cgroup.contains("background.scope")
    )));
    assert_eq!(
        fs::read_to_string(cg.path.join("background.scope/cgroup.freeze"))
            .unwrap()
            .trim(),
        "1"
    );

    // Kill background cgroup.
    if let Some(action) = policy.choose_kill(&apps) {
        actor.execute(&action).unwrap();
    }
    std::thread::sleep(Duration::from_millis(200));
    assert!(event_log.events().iter().any(|e| matches!(
        e,
        Event::AppKilled { cgroup } if cgroup.contains("background.scope")
    )));
    let status = cg.children[1].try_wait().unwrap();
    assert!(
        status.is_some(),
        "background process should have exited after kill"
    );

    // Drop cleans up the shell scope.
    drop(cg);
}
