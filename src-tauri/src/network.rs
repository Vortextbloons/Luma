use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    vpn_detected: bool,
    vpn_names: Vec<String>,
}

fn vpn_interface(kind: u32, name: &str) -> bool {
    let name = name.to_lowercase();
    matches!(kind, 23 | 131) || ["vpn", "wireguard", "wintun", "tailscale", "zerotier", "tap-windows", "mullvad", "proton", "nordlynx"]
        .iter().any(|part| name.contains(part))
}

#[cfg(windows)]
#[tauri::command]
pub async fn network_status() -> Result<Status, String> {
    tauri::async_runtime::spawn_blocking(snapshot).await.map_err(|error| error.to_string())?
}

#[cfg(windows)]
fn snapshot() -> Result<Status, String> {
    use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
    let profiles = NetworkInformation::GetConnectionProfiles().map_err(|error| error.to_string())?;
    let mut vpn_names = Vec::new();
    for profile in profiles {
        if profile.GetNetworkConnectivityLevel().map_err(|error| error.to_string())? == NetworkConnectivityLevel::None { continue; }
        let name = profile.ProfileName().map_err(|error| error.to_string())?.to_string();
        let kind = profile.NetworkAdapter().and_then(|adapter| adapter.IanaInterfaceType()).map_err(|error| error.to_string())?;
        if vpn_interface(kind, &name) { vpn_names.push(name); }
    }
    Ok(Status { vpn_detected: !vpn_names.is_empty(), vpn_names })
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn network_status() -> Result<Status, String> { Err("Network status requires Windows.".into()) }

#[cfg(windows)]
pub fn local_addresses() -> Result<Vec<String>, String> {
    use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
    let mut addresses = Vec::new();
    let mut adapters = Vec::new();
    for profile in NetworkInformation::GetConnectionProfiles().map_err(|error| error.to_string())? {
        if profile.GetNetworkConnectivityLevel().ok() == Some(NetworkConnectivityLevel::None) { continue; }
        let Ok(adapter) = profile.NetworkAdapter() else { continue; };
        let Ok(kind) = adapter.IanaInterfaceType() else { continue; };
        let Ok(name) = profile.ProfileName() else { continue; };
        if matches!(kind, 6 | 71) && !vpn_interface(kind, &name.to_string()) {
            if let Ok(id) = adapter.NetworkAdapterId() { adapters.push(id); }
        }
    }
    for host in NetworkInformation::GetHostNames().map_err(|error| error.to_string())? {
        let Ok(info) = host.IPInformation() else { continue; };
        let Ok(adapter) = info.NetworkAdapter() else { continue; };
        let Ok(id) = adapter.NetworkAdapterId() else { continue; };
        if !adapters.contains(&id) { continue; }
        let Ok(name) = host.CanonicalName() else { continue; };
        let Ok(ip) = name.to_string().parse::<std::net::Ipv4Addr>() else { continue; };
        if !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified() { addresses.push(ip.to_string()); }
    }
    addresses.sort();
    addresses.dedup();
    Ok(addresses)
}

#[cfg(not(windows))]
pub fn local_addresses() -> Result<Vec<String>, String> { Ok(Vec::new()) }

#[cfg(windows)]
pub struct Watcher(i64);

#[cfg(windows)]
impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = windows::Networking::Connectivity::NetworkInformation::RemoveNetworkStatusChanged(self.0);
    }
}

#[cfg(windows)]
pub fn watch(app: &tauri::AppHandle) -> windows::core::Result<Watcher> {
    use tauri::Emitter;
    use windows::Networking::Connectivity::{NetworkInformation, NetworkStatusChangedEventHandler};
    let app = app.clone();
    NetworkInformation::NetworkStatusChanged(&NetworkStatusChangedEventHandler::new(move |_| {
        let _ = app.emit("network-changed", ());
        Ok(())
    })).map(Watcher)
}

#[cfg(test)]
mod tests {
    use super::vpn_interface;

    #[test]
    fn identifies_tunnels_and_named_vpn_adapters_without_marking_wifi() {
        assert!(vpn_interface(131, "Tunnel"));
        assert!(vpn_interface(23, "Work connection"));
        assert!(vpn_interface(6, "WireGuard Tunnel"));
        assert!(vpn_interface(6, "ProtonVPN"));
        assert!(!vpn_interface(71, "Home Wi-Fi"));
        assert!(!vpn_interface(6, "Ethernet"));
    }
}
