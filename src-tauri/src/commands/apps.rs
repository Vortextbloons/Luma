use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Manager, State};

const IDS: [&str; 3] = ["Spotify", "Stremio", "Custom"];

#[derive(Default)]
pub struct Apps(pub Mutex<BTreeMap<String, String>>);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedApp {
    name: String,
    path: Option<String>,
}

fn validate(path: &str) -> Result<PathBuf, String> {
    let value = Path::new(path);
    if path.starts_with(r"\\")
        || !value.is_absolute()
        || !value.is_file()
        || !value
            .extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("exe"))
    {
        return Err("Choose an existing local executable using its absolute .exe path.".into());
    }
    let resolved = value.canonicalize().map_err(|e| e.to_string())?;
    if resolved.to_string_lossy().starts_with(r"\\?\UNC\") {
        return Err("Network executables are not allowed.".into());
    }
    Ok(resolved)
}

fn detect(name: &str) -> Option<PathBuf> {
    let relative: &[(&str, &str)] = match name {
        "Spotify" => &[
            ("APPDATA", r"Spotify\Spotify.exe"),
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\Spotify.exe"),
            ("ProgramFiles", r"Spotify\Spotify.exe"),
        ],
        "Stremio" => &[
            ("LOCALAPPDATA", r"Programs\LNV\Stremio-5\Stremio.exe"),
            ("LOCALAPPDATA", r"Programs\Stremio\Stremio.exe"),
            ("LOCALAPPDATA", r"Programs\LNV\Stremio-4\stremio.exe"),
            ("ProgramFiles", r"Stremio\Stremio.exe"),
            ("ProgramFiles(x86)", r"Stremio\Stremio.exe"),
        ],
        _ => return None,
    };
    for (variable, suffix) in relative {
        if let Some(root) = std::env::var_os(variable) {
            let candidate = PathBuf::from(root).join(suffix);
            if validate(&candidate.to_string_lossy()).is_ok() {
                return Some(candidate);
            }
        }
    }
    #[cfg(windows)]
    if let Some(path) = registry_app(name) {
        return Some(path);
    }
    None
}

#[cfg(windows)]
fn registry_app(name: &str) -> Option<PathBuf> {
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
        },
    };
    let key: Vec<u16> = format!(
        r"Software\Microsoft\Windows\CurrentVersion\App Paths\{}.exe",
        name.to_lowercase()
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let mut buffer = [0u16; 2048];
        let mut size = (buffer.len() * 2) as u32;
        let result = unsafe {
            RegGetValueW(
                hive,
                PCWSTR(key.as_ptr()),
                PCWSTR::null(),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if result.is_ok() {
            let text = String::from_utf16_lossy(
                &buffer[..buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len())],
            );
            if validate(text.trim_matches('"')).is_ok() {
                return Some(PathBuf::from(text.trim_matches('"')));
            }
        }
    }
    None
}

pub fn load(app: &tauri::AppHandle) -> Result<(), String> {
    let file = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("apps.json");
    if !file.exists() {
        return Ok(());
    }
    let paths: BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    *app.state::<Apps>().0.lock().map_err(|e| e.to_string())? = paths
        .into_iter()
        .filter(|(id, _)| IDS.contains(&id.as_str()))
        .collect();
    Ok(())
}

#[tauri::command]
pub fn app_paths(apps: State<Apps>) -> Result<BTreeMap<String, String>, String> {
    let paths = apps.0.lock().map_err(|e| e.to_string())?;
    Ok(IDS
        .iter()
        .map(|id| ((*id).into(), paths.get(*id).cloned().unwrap_or_default()))
        .collect())
}

#[tauri::command]
pub fn detect_apps() -> Vec<DetectedApp> {
    ["Spotify", "Stremio"]
        .into_iter()
        .map(|name| DetectedApp {
            name: name.into(),
            path: detect(name).map(|p| p.to_string_lossy().into_owned()),
        })
        .collect()
}

#[tauri::command]
pub fn save_app_paths(
    app: tauri::AppHandle,
    apps: State<Apps>,
    paths: BTreeMap<String, String>,
) -> Result<(), String> {
    if paths.keys().any(|id| !IDS.contains(&id.as_str())) {
        return Err("Unknown application.".into());
    }
    for path in paths.values().filter(|p| !p.is_empty()) {
        validate(path)?;
    }
    let directory = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("apps.json"),
        serde_json::to_vec(&paths).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    *apps.0.lock().map_err(|e| e.to_string())? = paths;
    Ok(())
}

#[tauri::command]
pub fn launch_app(name: String, apps: State<Apps>) -> Result<(), String> {
    if !IDS.contains(&name.as_str()) {
        return Err("Application is not approved.".into());
    }
    let configured = apps
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(&name)
        .cloned()
        .unwrap_or_default();
    let executable = if configured.is_empty() {
        detect(&name).ok_or_else(|| {
            format!("{name} was not found. Install it or set its executable path in Settings.")
        })?
    } else {
        validate(&configured)?
    };
    let mut command = std::process::Command::new(&executable);
    if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    command
        .spawn()
        .map_err(|e| format!("Could not start {name}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_shell_relative_and_network_paths() {
        for value in [
            "cmd /c calc",
            "Spotify.exe",
            r"\\server\share\app.exe",
            "C:\\missing\\app.exe",
        ] {
            assert!(validate(value).is_err());
        }
    }
}
