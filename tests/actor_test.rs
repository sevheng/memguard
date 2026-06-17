use memguard::actor::Actor;
use memguard::events::{Event, VecEventLog};
use memguard::policy::Action;
use std::fs;
use std::sync::Arc;

#[test]
fn test_actor_freezes_cgroup() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup = tmp.path().join("bg.scope");
    fs::create_dir(&cgroup).unwrap();
    fs::write(cgroup.join("cgroup.freeze"), "0").unwrap();

    let log = Arc::new(VecEventLog::default());
    let actor = Actor::new(tmp.path(), log.clone());
    actor
        .execute(&Action::Freeze {
            cgroup: cgroup.clone(),
        })
        .unwrap();

    let val = fs::read_to_string(cgroup.join("cgroup.freeze")).unwrap();
    assert_eq!(val.trim(), "1");
    assert!(log.events().iter().any(|e| matches!(e, Event::AppFrozen { .. })));
}

#[test]
fn test_actor_throttles_cpu() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup = tmp.path().join("bg.scope");
    fs::create_dir(&cgroup).unwrap();
    fs::write(cgroup.join("cpu.max"), "max").unwrap();

    let log = Arc::new(VecEventLog::default());
    let actor = Actor::new(tmp.path(), log.clone());
    actor
        .execute(&Action::Throttle {
            cgroup: cgroup.clone(),
        })
        .unwrap();

    let val = fs::read_to_string(cgroup.join("cpu.max")).unwrap();
    assert_eq!(val.trim(), "50000 100000");
    assert!(log.events().iter().any(|e| matches!(e, Event::AppThrottled { .. })));
}
