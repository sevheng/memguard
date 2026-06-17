use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct DesktopState {
    pub shell_pid: Option<u32>,
    pub active_app_id: Option<String>,
    pub session_bus_address: Option<String>,
}

pub struct Desktop;

impl Desktop {
    pub fn new() -> Self {
        Self
    }

    pub fn discover(&self) -> DesktopState {
        let mut state = DesktopState::default();
        if let Some((pid, addr)) = Self::find_graphical_session() {
            state.shell_pid = Some(pid);
            state.session_bus_address = Some(addr);
        }
        state.active_app_id = Self::query_active_app_id(&state.session_bus_address);
        state
    }

    fn find_graphical_session() -> Option<(u32, String)> {
        // Phase 1 stub: real implementation will parse /run/systemd/sessions/*.
        warn!("graphical session discovery not yet implemented");
        None
    }

    fn query_active_app_id(_bus_addr: &Option<String>) -> Option<String> {
        // Intentionally returns None in Phase 1. D-Bus active-window queries via zbus
        // will be implemented in Phase 2; until then the daemon relies on inventory
        // classification only.
        None
    }
}
