use crate::desktop::DesktopEnvironment;
use zbus::{Address, Connection};

pub async fn detect(bus_address: &str) -> anyhow::Result<DesktopEnvironment> {
    let address: Address = bus_address.parse()?;
    let conn = zbus::ConnectionBuilder::address(address)?.build().await?;

    if name_exists(&conn, "org.gnome.Shell").await {
        return Ok(DesktopEnvironment::Gnome);
    }
    if name_exists(&conn, "org.kde.KWin").await {
        return Ok(DesktopEnvironment::Kde);
    }

    Ok(DesktopEnvironment::Unknown)
}

async fn name_exists(conn: &Connection, name: &str) -> bool {
    match zbus::fdo::DBusProxy::new(conn).await {
        Ok(proxy) => {
            let bus_name = match zbus::names::BusName::try_from(name) {
                Ok(n) => n,
                Err(_) => return false,
            };
            proxy.name_has_owner(bus_name).await.unwrap_or(false)
        }
        Err(_) => false,
    }
}
