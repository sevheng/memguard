use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemorySnapshot {
    pub fn mem_available_pct(&self) -> f64 {
        if self.mem_total_kb == 0 {
            return 100.0;
        }
        100.0 * self.mem_available_kb as f64 / self.mem_total_kb as f64
    }

    pub fn swap_free_pct(&self) -> Option<f64> {
        if self.swap_total_kb == 0 {
            return None;
        }
        Some(100.0 * self.swap_free_kb as f64 / self.swap_total_kb as f64)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    pub mem_available_critical_pct: f64,
    pub swap_free_critical_pct: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            mem_available_critical_pct: 5.0,
            swap_free_critical_pct: 10.0,
        }
    }
}

pub struct MemoryMonitor {
    path: std::path::PathBuf,
    config: MemoryConfig,
}

impl MemoryMonitor {
    pub fn new(path: impl Into<std::path::PathBuf>, config: MemoryConfig) -> Self {
        Self {
            path: path.into(),
            config,
        }
    }

    pub fn read(&self) -> anyhow::Result<MemorySnapshot> {
        let content = std::fs::read_to_string(&self.path)?;
        parse_meminfo(&content)
    }

    pub fn critical(&self) -> bool {
        match self.read() {
            Ok(s) => {
                let mem_low = s.mem_available_pct() <= self.config.mem_available_critical_pct;
                let swap_low = s
                    .swap_free_pct()
                    .map(|p| p <= self.config.swap_free_critical_pct)
                    .unwrap_or(false);
                mem_low || swap_low
            }
            Err(e) => {
                warn!("failed to read memory info: {}", e);
                false
            }
        }
    }
}

pub fn parse_meminfo(content: &str) -> anyhow::Result<MemorySnapshot> {
    let mut snap = MemorySnapshot::default();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next().and_then(|s| s.parse::<u64>().ok()) else {
            continue;
        };
        match key.trim_end_matches(':') {
            "MemTotal" => snap.mem_total_kb = value,
            "MemAvailable" => snap.mem_available_kb = value,
            "SwapTotal" => snap.swap_total_kb = value,
            "SwapFree" => snap.swap_free_kb = value,
            _ => {}
        }
    }
    Ok(snap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_meminfo() {
        let content = "MemTotal:       16000000 kB\nMemAvailable:     800000 kB\nSwapTotal:       4000000 kB\nSwapFree:         200000 kB\n";
        let snap = parse_meminfo(content).unwrap();
        assert_eq!(snap.mem_total_kb, 16_000_000);
        assert_eq!(snap.mem_available_kb, 800_000);
        assert_eq!(snap.swap_total_kb, 4_000_000);
        assert_eq!(snap.swap_free_kb, 200_000);
        assert!((snap.mem_available_pct() - 5.0).abs() < 0.01);
        assert!((snap.swap_free_pct().unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_critical_defaults() {
        let config = MemoryConfig::default();
        let monitor = MemoryMonitor::new("/proc/meminfo", config);
        let low_mem = MemorySnapshot {
            mem_total_kb: 16_000_000,
            mem_available_kb: 400_000, // 2.5%
            swap_total_kb: 4_000_000,
            swap_free_kb: 2_000_000,
        };
        assert!(is_critical(&monitor.config, &low_mem));

        let low_swap = MemorySnapshot {
            mem_total_kb: 16_000_000,
            mem_available_kb: 8_000_000,
            swap_total_kb: 4_000_000,
            swap_free_kb: 100_000, // 2.5%
        };
        assert!(is_critical(&monitor.config, &low_swap));

        let ok = MemorySnapshot {
            mem_total_kb: 16_000_000,
            mem_available_kb: 8_000_000,
            swap_total_kb: 4_000_000,
            swap_free_kb: 2_000_000,
        };
        assert!(!is_critical(&monitor.config, &ok));
    }

    fn is_critical(config: &MemoryConfig, snap: &MemorySnapshot) -> bool {
        snap.mem_available_pct() <= config.mem_available_critical_pct
            || snap.swap_free_pct().map_or(false, |p| {
                p <= config.swap_free_critical_pct
            })
    }
}
