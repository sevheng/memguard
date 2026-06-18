use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use tokio::sync::mpsc;
use tracing::{error, warn};

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

/// Event-driven PSI watcher.
///
/// Registers a kernel PSI trigger on the memory pressure file and returns the
/// current pressure level whenever the trigger fires. This avoids polling.
pub struct PressureWatcher {
    rx: mpsc::Receiver<PressureLevel>,
}

impl PressureWatcher {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        warning_some_avg10: f64,
        critical_some_avg10: f64,
        critical_full_avg10: f64,
    ) -> anyhow::Result<Self> {
        let path = path.into();
        let (tx, rx) = mpsc::channel(16);

        std::thread::spawn(move || {
            if let Err(e) = Self::watch_loop(
                &path,
                warning_some_avg10,
                critical_some_avg10,
                critical_full_avg10,
                tx,
            ) {
                error!("pressure watcher failed: {}", e);
            }
        });

        Ok(Self { rx })
    }

    pub async fn wait(&mut self) -> Option<PressureLevel> {
        self.rx.recv().await
    }

    fn watch_loop(
        path: &std::path::Path,
        warning_some_avg10: f64,
        critical_some_avg10: f64,
        critical_full_avg10: f64,
        tx: mpsc::Sender<PressureLevel>,
    ) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?;

        // Register a PSI trigger for "some" memory stall over a 1 second window.
        // threshold_us maps warning_some_avg10 percent to microseconds in that window.
        let window_us: u64 = 1_000_000;
        let threshold_us = ((warning_some_avg10 / 100.0) * window_us as f64) as u64;
        let trigger = format!("some {} {}\n", threshold_us.max(1), window_us);
        file.write_all(trigger.as_bytes())?;

        let fd = file.as_raw_fd();
        let mut buf = [0u8; 512];

        loop {
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLPRI,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, -1) };
            if ret < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if pfd.revents & libc::POLLPRI == 0 {
                continue;
            }

            // Re-read current PSI averages after the trigger fired.
            file.seek(SeekFrom::Start(0))?;
            let n = file.read(&mut buf)?;
            let content = std::str::from_utf8(&buf[..n]).unwrap_or("");
            let level = match parse_pressure(content) {
                Ok(s) => classify(
                    &s,
                    warning_some_avg10,
                    critical_some_avg10,
                    critical_full_avg10,
                ),
                Err(e) => {
                    warn!("failed to parse pressure after event: {}", e);
                    PressureLevel::Normal
                }
            };

            if tx.blocking_send(level).is_err() {
                // Receiver dropped; exit watcher thread cleanly.
                break;
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pressure() {
        let content = "some avg10=29.97 avg60=22.82 avg300=11.92 total=92159505\nfull avg10=8.76 avg60=5.17 avg300=3.38 total=43136045\n";
        let snap = parse_pressure(content).unwrap();
        assert!((snap.some_avg10 - 29.97).abs() < 0.01);
        assert!((snap.full_avg10 - 8.76).abs() < 0.01);
    }

    #[test]
    fn test_classify() {
        let normal = PressureSnapshot {
            some_avg10: 10.0,
            full_avg10: 0.0,
            ..Default::default()
        };
        let warning = PressureSnapshot {
            some_avg10: 35.0,
            full_avg10: 0.0,
            ..Default::default()
        };
        let critical = PressureSnapshot {
            some_avg10: 75.0,
            full_avg10: 0.0,
            ..Default::default()
        };
        assert_eq!(classify(&normal, 30.0, 70.0, 50.0), PressureLevel::Normal);
        assert_eq!(classify(&warning, 30.0, 70.0, 50.0), PressureLevel::Warning);
        assert_eq!(classify(&critical, 30.0, 70.0, 50.0), PressureLevel::Critical);
    }
}
