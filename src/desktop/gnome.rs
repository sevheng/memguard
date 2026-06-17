use zbus::Address;

pub async fn active_pid(bus_address: &str) -> anyhow::Result<u32> {
    let address: Address = bus_address.parse()?;
    let conn = zbus::ConnectionBuilder::address(address)?.build().await?;

    let proxy = zbus::Proxy::new(
        &conn,
        "org.gnome.Shell",
        "/org/gnome/Shell",
        "org.gnome.Shell",
    )
    .await?;

    let (result,): (String,) = proxy
        .call("Eval", &("global.display.focus_window.get_pid()",))
        .await?;

    parse_eval_result(&result)
}

pub fn parse_eval_result(result: &str) -> anyhow::Result<u32> {
    let parts: Vec<&str> = result.split(',').collect();
    if parts.len() != 2 {
        anyhow::bail!("unexpected Eval result: {}", result);
    }
    let ok = parts[0].trim() == "true";
    if !ok {
        anyhow::bail!("Eval failed: {}", result);
    }
    parts[1].trim().parse().map_err(Into::into)
}
