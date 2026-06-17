use memguard::inventory::{AppClass, Inventory};
use std::fs;

fn make_fake_cgroup(
    cgroup_root: &std::path::Path,
    proc_root: &std::path::Path,
    rel: &str,
    pids: &[u32],
    comm: &str,
    rss_pages: u64,
) {
    let path = cgroup_root.join(rel);
    fs::create_dir_all(&path).unwrap();
    let procs = pids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path.join("cgroup.procs"), procs).unwrap();
    for pid in pids {
        let pdir = proc_root.join(pid.to_string());
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("comm"), comm).unwrap();
        fs::write(pdir.join("statm"), format!("0 {} 0 0 0 0 0\n", rss_pages))
            .unwrap();
    }
}

#[test]
fn test_inventory_classifies_background_and_shell() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path().join("cgroup");
    let proc_root = tmp.path().join("proc");
    fs::create_dir(&cgroup_root).unwrap();
    fs::create_dir(&proc_root).unwrap();

    make_fake_cgroup(
        &cgroup_root,
        &proc_root,
        "user.slice/user-1000.slice/session-1.scope",
        &[1000, 1001],
        "app",
        100,
    );
    make_fake_cgroup(
        &cgroup_root,
        &proc_root,
        "user.slice/user-1000.slice/session-2.scope",
        &[2000],
        "gnome-shell",
        200,
    );

    let inv = Inventory::new(&cgroup_root, &proc_root);
    let apps = inv.scan(None, Some(2000));
    assert_eq!(apps.len(), 2);
    let shell = apps.iter().find(|a| a.pids.contains(&2000)).unwrap();
    assert_eq!(shell.class, AppClass::Shell);
    let bg = apps.iter().find(|a| a.pids.contains(&1000)).unwrap();
    assert_eq!(bg.class, AppClass::Background);
    assert_eq!(bg.rss_bytes, 100 * 4096 * 2);
}

#[test]
fn test_app_id_for_pid() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cgroup_root = tmp.path().join("cgroup");
    let proc_root = tmp.path().join("proc");
    fs::create_dir(&cgroup_root).unwrap();
    fs::create_dir(&proc_root).unwrap();

    make_fake_cgroup(
        &cgroup_root,
        &proc_root,
        "user.slice/user-1000.slice/session-1.scope",
        &[1234],
        "firefox",
        100,
    );

    let app_id = Inventory::app_id_for_pid(1234, &proc_root, &cgroup_root).unwrap();
    assert_eq!(app_id, "firefox");
}
