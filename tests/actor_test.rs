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
    actor
        .execute(&Action::Freeze {
            cgroup: cgroup.clone(),
        })
        .unwrap();

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
    actor
        .execute(&Action::Throttle {
            cgroup: cgroup.clone(),
        })
        .unwrap();

    let val = fs::read_to_string(cgroup.join("cpu.max")).unwrap();
    assert_eq!(val.trim(), "50000 100000");
}
