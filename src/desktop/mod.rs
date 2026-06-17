use std::path::PathBuf;

pub mod detector;
pub mod gnome;
pub mod kde;
pub mod session;

#[derive(Debug, Clone, Default)]
pub struct DesktopState {
    pub shell_pid: Option<u32>,
    pub active_app_id: Option<String>,
    pub session_bus_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Gnome,
    Kde,
    Unknown,
}

pub struct Desktop {
    session_dir: PathBuf,
}

impl Desktop {
    pub fn new(session_dir: impl Into<PathBuf>) -> Self {
        Self {
            session_dir: session_dir.into(),
        }
    }

    pub async fn discover(&self) -> DesktopState {
        let mut state = DesktopState::default();

        let Some(session) = session::find_graphical_session(&self.session_dir).await else {
            return state;
        };

        state.shell_pid = Some(session.shell_pid);
        state.session_bus_address = Some(session.bus_address.clone());

        let de = match detector::detect(&session.bus_address).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("failed to detect desktop environment: {}", e);
                DesktopEnvironment::Unknown
            }
        };

        let _active_pid = match de {
            DesktopEnvironment::Gnome => gnome::active_pid(&session.bus_address).await.ok(),
            DesktopEnvironment::Kde => kde::active_pid(&session.bus_address).await.ok(),
            DesktopEnvironment::Unknown => None,
        };

        // active_app_id mapping will be added after Inventory::app_id_for_pid is implemented.

        state
    }
}
