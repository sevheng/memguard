use memguard::actor::Actor;
use memguard::events::NullEventLog;
use memguard::inventory::{AppClass, Inventory};
use memguard::policy::{Action, Policy};
use memguard::pressure::{classify, parse_pressure, PressureLevel};
use std::fs;
use std::sync::Arc;

#[test]
fn test_critical_pressure_freezes_background() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path().join("cgroup");
    let proc_root = tmp.path().join("proc");
    fs::create_dir(&cgroup_root).unwrap();
    fs::create_dir(&proc_root).unwrap();

    // Build fake cgroup tree
    let user = cgroup_root.join("user.slice/user-1000.slice/session-1.scope");
    fs::create_dir_all(&user).unwrap();
    fs::write(user.join("cgroup.procs"), "1000\n").unwrap();
    fs::write(user.join("cgroup.freeze"), "0").unwrap();

    // Fake /proc for PID 1000
    let pdir = proc_root.join("1000");
    fs::create_dir_all(&pdir).unwrap();
    fs::write(pdir.join("comm"), "app\n").unwrap();
    fs::write(pdir.join("statm"), "0 100 0 0 0 0 0\n").unwrap();

    let inv = Inventory::new(&cgroup_root, &proc_root);
    let apps = inv.scan(None, None);
    assert_eq!(apps.len(), 1);

    let policy = Policy::new(true);
    let sample = "some avg10=80.00 avg60=70.00 total=100\nfull avg10=60.00 avg60=50.00 total=200\n";
    let snap = parse_pressure(sample).unwrap();
    let level = classify(&snap, 30.0, 70.0, 50.0);
    assert_eq!(level, PressureLevel::Critical);

    let actions = policy.decide(level, &apps, &[]);
    let actor = Actor::new(&cgroup_root, Arc::new(NullEventLog));
    for action in &actions {
        actor.execute(action).unwrap();
    }

    let val = fs::read_to_string(user.join("cgroup.freeze")).unwrap();
    assert_eq!(val.trim(), "1");
}

#[test]
fn test_critical_pressure_protects_active_window() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path().join("cgroup");
    let proc_root = tmp.path().join("proc");
    fs::create_dir(&cgroup_root).unwrap();
    fs::create_dir(&proc_root).unwrap();

    // Background app
    let bg = cgroup_root.join("user.slice/user-1000.slice/bg.scope");
    fs::create_dir_all(&bg).unwrap();
    fs::write(bg.join("cgroup.procs"), "1000\n").unwrap();
    fs::write(bg.join("cgroup.freeze"), "0").unwrap();
    let pdir = proc_root.join("1000");
    fs::create_dir_all(&pdir).unwrap();
    fs::write(pdir.join("comm"), "bgapp\n").unwrap();
    fs::write(pdir.join("statm"), "0 100 0 0 0 0 0\n").unwrap();

    // Active app
    let active = cgroup_root.join("user.slice/user-1000.slice/active.scope");
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("cgroup.procs"), "2000\n").unwrap();
    let adir = proc_root.join("2000");
    fs::create_dir_all(&adir).unwrap();
    fs::write(adir.join("comm"), "activeapp\n").unwrap();
    fs::write(adir.join("statm"), "0 50 0 0 0 0 0\n").unwrap();

    let inv = Inventory::new(&cgroup_root, &proc_root);
    let apps = inv.scan(Some("activeapp"), None);

    let active_app = apps.iter().find(|a| a.app_id == "activeapp").unwrap();
    assert_eq!(active_app.class, AppClass::Active);

    let policy = Policy::new(true);
    let actions = policy.decide(PressureLevel::Critical, &apps, &[]);
    assert!(actions.iter().any(|a| matches!(a, Action::Shield { pid: 2000 })));
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::Freeze { cgroup } if cgroup.ends_with("bg.scope")
    )));
}
