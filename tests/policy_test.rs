use memguard::inventory::{AppClass, CgroupApp};
use memguard::policy::{Action, Policy};
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
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::Throttle { cgroup } if cgroup.ends_with("bg")
    )));
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
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::Freeze { cgroup } if cgroup.ends_with("bg")
    )));
}

#[test]
fn test_policy_choose_kill_largest_background() {
    let policy = Policy::new(true);
    let apps = vec![
        app("small", AppClass::Background, vec![2], 100),
        app("big", AppClass::Background, vec![3], 500),
    ];
    let action = policy.choose_kill(&apps);
    assert!(matches!(
        action,
        Some(Action::Kill { cgroup }) if cgroup.ends_with("big")
    ));
}
