use crate::policy::Action;
use std::path::Path;
use tracing::info;

pub struct Actor {
    cgroup_root: std::path::PathBuf,
}

impl Actor {
    pub fn new(cgroup_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            cgroup_root: cgroup_root.into(),
        }
    }

    pub fn execute(&self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Freeze { cgroup } => self.write_cgroup(cgroup, "cgroup.freeze", "1"),
            Action::Unfreeze { cgroup } => self.write_cgroup(cgroup, "cgroup.freeze", "0"),
            Action::Throttle { cgroup } => self.write_cgroup(cgroup, "cpu.max", "50000 100000"),
            Action::Unthrottle { cgroup } => self.write_cgroup(cgroup, "cpu.max", "max"),
            Action::Kill { cgroup } => self.write_cgroup(cgroup, "cgroup.kill", "1"),
            Action::Shield { pid } => self.set_oom_score_adj(*pid, -1000),
            Action::Unshield { pid } => self.set_oom_score_adj(*pid, 0),
        }
    }

    fn write_cgroup(&self, cgroup: &Path, file: &str, value: &str) -> anyhow::Result<()> {
        let rel = cgroup.strip_prefix(&self.cgroup_root).unwrap_or(cgroup);
        let path = self.cgroup_root.join(rel).join(file);
        std::fs::write(&path, value)?;
        info!("wrote {} to {}", value, path.display());
        Ok(())
    }

    fn set_oom_score_adj(&self, pid: u32, value: i32) -> anyhow::Result<()> {
        let path = format!("/proc/{}/oom_score_adj", pid);
        std::fs::write(&path, value.to_string())?;
        info!("set oom_score_adj of {} to {}", pid, value);
        Ok(())
    }
}
