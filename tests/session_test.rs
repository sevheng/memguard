use memguard::desktop::session::find_graphical_session;
use std::fs;

#[tokio::test]
async fn test_find_graphical_session() {
    let tmp = tempfile::TempDir::new().unwrap();
    let session_dir = tmp.path();

    fs::write(
        session_dir.join("1.session"),
        "Type=wayland\nClass=user\nUser=1000\nLeader=1234\nDisplay=:1\n",
    )
    .unwrap();

    // Without a real cgroup/proc, shell_pid falls back to leader.
    let session = find_graphical_session(session_dir).await.unwrap();
    assert_eq!(session.uid, 1000);
    assert_eq!(session.shell_pid, 1234);
    assert_eq!(session.bus_address, "unix:path=/run/user/1000/bus");
}

#[tokio::test]
async fn test_ignores_non_graphical_session() {
    let tmp = tempfile::TempDir::new().unwrap();
    let session_dir = tmp.path();

    fs::write(
        session_dir.join("2.session"),
        "Type=tty\nClass=user\nUser=1000\nLeader=1234\n",
    )
    .unwrap();

    assert!(find_graphical_session(session_dir).await.is_none());
}
