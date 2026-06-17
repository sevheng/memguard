use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum Event {
    PressureChanged { level: String },
    AppShielded { pid: u32 },
    AppUnshielded { pid: u32 },
    AppFrozen { cgroup: String },
    AppThawed { cgroup: String },
    AppThrottled { cgroup: String },
    AppUnthrottled { cgroup: String },
    AppKilled { cgroup: String },
}

impl Event {
    fn timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub trait EventLog: Send + Sync {
    fn log(&self, event: Event);
}

pub struct NullEventLog;

impl EventLog for NullEventLog {
    fn log(&self, _event: Event) {}
}

#[derive(Default)]
pub struct VecEventLog {
    events: Mutex<Vec<Event>>,
}

impl EventLog for VecEventLog {
    fn log(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

impl VecEventLog {
    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

pub struct FileEventLog {
    file: Mutex<std::fs::File>,
}

impl FileEventLog {
    pub fn new(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl EventLog for FileEventLog {
    fn log(&self, event: Event) {
        #[derive(Serialize)]
        struct Record {
            ts: u64,
            #[serde(flatten)]
            event: Event,
        }

        let record = Record {
            ts: Event::timestamp(),
            event,
        };

        if let Ok(line) = serde_json::to_string(&record) {
            let mut file = self.file.lock().unwrap();
            let _ = writeln!(file, "{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_event_log_records_events() {
        let log = VecEventLog::default();
        log.log(Event::AppShielded { pid: 42 });
        assert_eq!(log.events().len(), 1);
        match &log.events()[0] {
            Event::AppShielded { pid } => assert_eq!(*pid, 42),
            _ => panic!("wrong event"),
        }
    }

    #[test]
    fn test_file_event_log_writes_json_lines() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let log = FileEventLog::new(tmp.path()).unwrap();
        log.log(Event::PressureChanged {
            level: "Critical".to_string(),
        });

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("PressureChanged"));
        assert!(content.contains("Critical"));
        assert!(content.contains("\"ts\":"));
    }
}
