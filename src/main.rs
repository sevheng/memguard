use memguard::actor::Actor;
use memguard::config::Config;
use memguard::desktop::Desktop;
use memguard::inventory::Inventory;
use memguard::policy::{Action, Policy};
use memguard::pressure::{PressureLevel, PressureMonitor};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::default();
    let pressure = PressureMonitor::new(
        "/proc/pressure/memory",
        config.pressure.warning_some_avg10,
        config.pressure.critical_some_avg10,
        config.pressure.critical_full_avg10,
    );
    let desktop = Desktop::new(&config.desktop.session_dir);
    let inventory = Inventory::new("/sys/fs/cgroup", "/proc");
    let actor = Actor::new("/sys/fs/cgroup");
    let policy = Policy::new(config.policy.freeze_on_critical);

    let mut frozen: Vec<PathBuf> = Vec::new();
    let mut warning_start: Option<tokio::time::Instant> = None;
    let mut tick = interval(Duration::from_millis(config.pressure.poll_ms));

    loop {
        tick.tick().await;

        let state = desktop.discover().await;
        let apps = inventory.scan(state.active_app_id.as_deref(), state.shell_pid);
        let level = pressure.level();

        info!("pressure={:?} apps={}", level, apps.len());

        match level {
            PressureLevel::Normal => {
                warning_start = None;
                for cgroup in frozen.drain(..) {
                    let _ = actor.execute(&Action::Unfreeze { cgroup });
                }
            }
            PressureLevel::Warning => {
                warning_start = None;
                for action in policy.decide(level, &apps, &frozen) {
                    let _ = actor.execute(&action);
                }
            }
            PressureLevel::Critical => {
                for action in policy.decide(level, &apps, &frozen) {
                    if let Action::Freeze { ref cgroup } = action {
                        frozen.push(cgroup.clone());
                    }
                    let _ = actor.execute(&action);
                }
                if warning_start.is_none() {
                    warning_start = Some(tokio::time::Instant::now());
                }
                if warning_start.unwrap().elapsed()
                    >= Duration::from_secs(config.policy.kill_delay_seconds)
                {
                    if let Some(action) = policy.choose_kill(&apps) {
                        warn!("killing cgroup due to sustained critical pressure");
                        let _ = actor.execute(&action);
                    }
                }
            }
        }
    }
}
