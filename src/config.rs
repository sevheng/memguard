use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub pressure: PressureConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub desktop: DesktopConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PressureConfig {
    pub poll_ms: u64,
    pub warning_some_avg10: f64,
    pub critical_some_avg10: f64,
    pub critical_full_avg10: f64,
}

impl Default for PressureConfig {
    fn default() -> Self {
        Self {
            poll_ms: 500,
            warning_some_avg10: 30.0,
            critical_some_avg10: 70.0,
            critical_full_avg10: 50.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicyConfig {
    pub freeze_on_critical: bool,
    pub kill_delay_seconds: u64,
    pub recovery_seconds: u64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            freeze_on_critical: true,
            kill_delay_seconds: 5,
            recovery_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub supported: Vec<String>,
    pub session_dir: String,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            supported: vec!["gnome".to_string(), "kde".to_string()],
            session_dir: "/run/systemd/sessions".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pressure: PressureConfig {
                poll_ms: 500,
                warning_some_avg10: 30.0,
                critical_some_avg10: 70.0,
                critical_full_avg10: 50.0,
            },
            policy: PolicyConfig {
                freeze_on_critical: true,
                kill_delay_seconds: 5,
                recovery_seconds: 10,
            },
            desktop: DesktopConfig {
                supported: vec!["gnome".to_string(), "kde".to_string()],
                session_dir: "/run/systemd/sessions".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn load_or_default(path: &std::path::Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match Self::load(path) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("failed to load config from {}: {e}", path.display());
                Self::default()
            }
        }
    }
}
