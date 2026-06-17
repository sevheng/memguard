use zbus::Address;

pub async fn active_pid(bus_address: &str) -> anyhow::Result<u32> {
    let address: Address = bus_address.parse()?;
    let conn = zbus::ConnectionBuilder::address(address)?.build().await?;

    let kwin_proxy = zbus::Proxy::new(
        &conn,
        "org.kde.KWin",
        "/KWin",
        "org.kde.KWin",
    )
    .await?;

    let uuid: String = kwin_proxy.get_property("activeWindow").await?;

    let window_path = format!("/Windows/{}", uuid);
    let window_proxy = zbus::Proxy::new(
        &conn,
        "org.kde.KWin",
        window_path.as_str(),
        "org.kde.KWin.Window",
    )
    .await?;

    let pid: u32 = window_proxy.get_property("pid").await?;
    Ok(pid)
}
