use memguard::actor::Actor;
use memguard::inventory::Inventory;
use memguard::policy::Policy;
use memguard::pressure::{classify, parse_pressure, PressureLevel};
use std::fs;

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
    let actor = Actor::new(&cgroup_root);
    for action in &actions {
        actor.execute(action).unwrap();
    }

    let val = fs::read_to_string(user.join("cgroup.freeze")).unwrap();
    assert_eq!(val.trim(), "1");
}
