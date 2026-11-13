#[tauri::command]
fn launch_app(name: &str, path: &str) -> Result<(), String> {
    if !["Spotify", "Stremio", "Custom"].contains(&name) { return Err("Application is not approved.".into()); }
    let executable = std::path::Path::new(path);
    if !executable.is_absolute() || !executable.is_file() || executable.extension().and_then(|v| v.to_str()).map(|v| !v.eq_ignore_ascii_case("exe")).unwrap_or(true) {
        return Err(format!("Configure an absolute path to an installed {} .exe in Settings.", name));
    }
    std::process::Command::new(executable).spawn().map_err(|e| format!("Could not start {name}: {e}"))?;
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![launch_app])
        .run(tauri::generate_context!())
        .expect("error while running Luma");
}
