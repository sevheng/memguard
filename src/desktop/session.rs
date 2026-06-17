use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Session {
    pub uid: u32,
    pub shell_pid: u32,
    pub bus_address: String,
}

pub async fn find_graphical_session(session_dir: &Path) -> Option<Session> {
    let Ok(entries) = std::fs::read_dir(session_dir) else {
        warn!("cannot read session dir {}", session_dir.display());
        return None;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("session") {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let fields = parse_session_file(&content);
        let is_graphical = fields.get("Type").map(|s| s.as_str()) == Some("wayland")
            || fields.get("Type").map(|s| s.as_str()) == Some("x11");
        let is_user = fields.get("Class").map(|s| s.as_str()) == Some("user");
        if !is_graphical || !is_user {
            continue;
        }

        let uid: u32 = fields.get("User").and_then(|s| s.parse().ok())?;
        let leader: u32 = fields.get("Leader").and_then(|s| s.parse().ok())?;
        let shell_pid = find_shell_pid(uid, leader).await.unwrap_or(leader);
        let bus_address = format!("unix:path=/run/user/{}/bus", uid);

        return Some(Session {
            uid,
            shell_pid,
            bus_address,
        });
    }
    None
}

fn parse_session_file(content: &str) -> std::collections::HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next()?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

async fn find_shell_pid(uid: u32, _leader: u32) -> Option<u32> {
    let shell_names: &[&str] = &["gnome-shell", "kwin_wayland", "kwin_x11"];
    let user_slice = format!("/sys/fs/cgroup/user.slice/user-{}.slice", uid);
    let path = std::path::PathBuf::from(&user_slice);
    if !path.exists() {
        return None;
    }
    find_pid_by_comm(&path, shell_names)
}

fn find_pid_by_comm(dir: &std::path::Path, names: &[&str]) -> Option<u32> {
    let Ok(entries) = std::fs::read_dir(dir) else { return None };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(pid) = find_pid_by_comm(&path, names) {
                return Some(pid);
            }
        }
        if path.file_name()? == "cgroup.procs" {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            for pid_str in content.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    let comm = std::fs::read_to_string(format!("/proc/{}/comm", pid))
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if names.contains(&comm.as_str()) {
                        return Some(pid);
                    }
                }
            }
        }
    }
    None
}
