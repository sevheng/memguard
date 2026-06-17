use crate::desktop::DesktopEnvironment;

pub async fn detect(_bus_address: &str) -> anyhow::Result<DesktopEnvironment> {
    Ok(DesktopEnvironment::Unknown)
}
