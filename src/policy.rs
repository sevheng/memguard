use crate::inventory::{AppClass, CgroupApp};
use crate::pressure::PressureLevel;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Freeze { cgroup: PathBuf },
    Throttle { cgroup: PathBuf },
    Kill { cgroup: PathBuf },
    Shield { pid: u32 },
    Unfreeze { cgroup: PathBuf },
    Unthrottle { cgroup: PathBuf },
    Unshield { pid: u32 },
}

pub struct Policy {
    freeze_on_critical: bool,
}

impl Policy {
    pub fn new(freeze_on_critical: bool) -> Self {
        Self { freeze_on_critical }
    }

    pub fn decide(
        &self,
        level: PressureLevel,
        apps: &[CgroupApp],
        previously_frozen: &[PathBuf],
    ) -> Vec<Action> {
        match level {
            PressureLevel::Normal => previously_frozen
                .iter()
                .map(|c| Action::Unfreeze { cgroup: c.clone() })
                .collect(),
            PressureLevel::Warning => apps
                .iter()
                .filter(|a| a.class == AppClass::Background)
                .map(|a| Action::Throttle {
                    cgroup: a.cgroup_path.clone(),
                })
                .collect(),
            PressureLevel::Critical => {
                let mut actions: Vec<Action> = apps
                    .iter()
                    .filter(|a| a.class == AppClass::Shell || a.class == AppClass::Active)
                    .flat_map(|a| a.pids.iter().map(|&pid| Action::Shield { pid }))
                    .collect();
                if self.freeze_on_critical {
                    actions.extend(apps.iter().filter(|a| a.class == AppClass::Background).map(
                        |a| Action::Freeze {
                            cgroup: a.cgroup_path.clone(),
                        },
                    ));
                }
                actions
            }
        }
    }

    pub fn choose_kill(&self, apps: &[CgroupApp]) -> Option<Action> {
        let mut candidates: Vec<&CgroupApp> = apps
            .iter()
            .filter(|a| a.class == AppClass::Background)
            .collect();
        candidates.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
        candidates.first().map(|a| Action::Kill {
            cgroup: a.cgroup_path.clone(),
        })
    }
}
