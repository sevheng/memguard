use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Default)]
pub struct PressureSnapshot {
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub full_avg10: f64,
    pub full_avg60: f64,
}

pub struct PressureMonitor {
    path: std::path::PathBuf,
    warning_some_avg10: f64,
    critical_some_avg10: f64,
    critical_full_avg10: f64,
}

impl PressureMonitor {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        warning_some_avg10: f64,
        critical_some_avg10: f64,
        critical_full_avg10: f64,
    ) -> Self {
        Self {
            path: path.into(),
            warning_some_avg10,
            critical_some_avg10,
            critical_full_avg10,
        }
    }

    pub fn read(&self) -> anyhow::Result<PressureSnapshot> {
        let content = std::fs::read_to_string(&self.path)?;
        parse_pressure(&content)
    }

    pub fn level(&self) -> PressureLevel {
        match self.read() {
            Ok(s) => classify(
                &s,
                self.warning_some_avg10,
                self.critical_some_avg10,
                self.critical_full_avg10,
            ),
            Err(e) => {
                warn!("failed to read pressure: {}", e);
                PressureLevel::Normal
            }
        }
    }
}

pub fn parse_pressure(content: &str) -> anyhow::Result<PressureSnapshot> {
    let mut snap = PressureSnapshot::default();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let key = parts[0].trim_end_matches(':');
        let mut avg10 = None;
        let mut avg60 = None;
        for part in &parts[1..] {
            if let Some(value) = part.strip_prefix("avg10=") {
                avg10 = value.trim_end_matches(',').parse().ok();
            } else if let Some(value) = part.strip_prefix("avg60=") {
                avg60 = value.trim_end_matches(',').parse().ok();
            }
        }
        match key {
            "some" => {
                snap.some_avg10 = avg10.unwrap_or(0.0);
                snap.some_avg60 = avg60.unwrap_or(0.0);
            }
            "full" => {
                snap.full_avg10 = avg10.unwrap_or(0.0);
                snap.full_avg60 = avg60.unwrap_or(0.0);
            }
            _ => {}
        }
    }
    Ok(snap)
}

pub fn classify(
    snap: &PressureSnapshot,
    warning_some_avg10: f64,
    critical_some_avg10: f64,
    critical_full_avg10: f64,
) -> PressureLevel {
    if snap.some_avg10 >= critical_some_avg10 || snap.full_avg10 >= critical_full_avg10 {
        PressureLevel::Critical
    } else if snap.some_avg10 >= warning_some_avg10 {
        PressureLevel::Warning
    } else {
        PressureLevel::Normal
    }
}
