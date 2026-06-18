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
    cgroup_root: PathBuf,
    proc_root: PathBuf,
    shell_names: Vec<String>,
}

impl Inventory {
    pub fn new(cgroup_root: impl Into<PathBuf>, proc_root: impl Into<PathBuf>) -> Self {
        Self {
            cgroup_root: cgroup_root.into(),
            proc_root: proc_root.into(),
            shell_names: vec![
                "gnome-shell".into(),
                "kwin_wayland".into(),
                "kwin_x11".into(),
            ],
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

    pub fn scan_at(
        &self,
        root: &Path,
        active_app_id: Option<&str>,
        shell_pid: Option<u32>,
    ) -> Vec<CgroupApp> {
        let mut apps = Vec::new();
        self.scan_dir(root, active_app_id, shell_pid, &mut apps);
        apps
    }

    fn scan_dir(
        &self,
        dir: &Path,
        active_app_id: Option<&str>,
        shell_pid: Option<u32>,
        apps: &mut Vec<CgroupApp>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
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

    fn classify(
        &self,
        path: &Path,
        active_app_id: Option<&str>,
        shell_pid: Option<u32>,
    ) -> Option<CgroupApp> {
        let cgroup_name = path.file_name()?.to_string_lossy();
        if !(cgroup_name.contains("session-") || cgroup_name.ends_with(".scope")) {
            return None;
        }
        let pids = read_pids(path);
        let app_id = self
            .leading_process_name(&pids)
            .unwrap_or_else(|| cgroup_name.to_string());
        let class = if shell_pid.is_some_and(|pid| pids.contains(&pid))
            || self.shell_names.contains(&app_id)
        {
            AppClass::Shell
        } else if active_app_id == Some(&app_id) {
            AppClass::Active
        } else {
            AppClass::Background
        };
        let rss_bytes = pids.iter().map(|&pid| self.rss_for_pid(pid)).sum();
        Some(CgroupApp {
            app_id,
            class,
            cgroup_path: path.to_path_buf(),
            pids,
            rss_bytes,
        })
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

    pub fn app_id_for_pid(
        pid: u32,
        proc_root: &Path,
        cgroup_root: &Path,
    ) -> anyhow::Result<String> {
        let comm = std::fs::read_to_string(proc_root.join(format!("{}/comm", pid)))?;
        let comm = comm.trim().to_string();

        if let Some(cgroup) = Self::find_cgroup_for_pid(cgroup_root, pid) {
            let pids = read_pids(&cgroup);
            if pids.contains(&pid) {
                return Ok(Self::leading_process_name_for_pids(proc_root, &pids).unwrap_or(comm));
            }
        }

        Ok(comm)
    }

    fn find_cgroup_for_pid(cgroup_root: &Path, pid: u32) -> Option<PathBuf> {
        Self::scan_dir_for_pid(cgroup_root, pid)
    }

    fn scan_dir_for_pid(dir: &Path, pid: u32) -> Option<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return None;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = Self::scan_dir_for_pid(&path, pid) {
                    return Some(found);
                }
            }
            if path.file_name()? == "cgroup.procs" {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if content.lines().any(|l| l.trim() == pid.to_string()) {
                    return Some(path.parent()?.to_path_buf());
                }
            }
        }
        None
    }

    fn leading_process_name_for_pids(proc_root: &Path, pids: &[u32]) -> Option<String> {
        let first = pids.first()?;
        let comm = std::fs::read_to_string(proc_root.join(format!("{}/comm", first))).ok()?;
        Some(comm.trim().to_string())
    }
}

fn read_pids(path: &Path) -> Vec<u32> {
    let file = path.join("cgroup.procs");
    let content = std::fs::read_to_string(&file).unwrap_or_default();
    content
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}
