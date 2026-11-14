use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Manager, State};

struct Service {
    id: &'static str,
    executables: &'static [(&'static str, &'static str)],
    url: Option<&'static str>,
}

const SERVICES: &[Service] = &[
    Service {
        id: "Spotify",
        url: None,
        executables: &[
            ("APPDATA", r"Spotify\Spotify.exe"),
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\Spotify.exe"),
            ("LOCALAPPDATA", r"Programs\Spotify\Spotify.exe"),
            ("ProgramFiles", r"Spotify\Spotify.exe"),
            ("ProgramFiles(x86)", r"Spotify\Spotify.exe"),
        ],
    },
    Service {
        id: "Stremio",
        url: None,
        executables: &[],
    },
    Service {
        id: "Netflix",
        url: Some("https://www.netflix.com/browse"),
        executables: &[
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\Netflix.exe"),
            ("LOCALAPPDATA", r"Programs\Netflix\Netflix.exe"),
        ],
    },
    Service {
        id: "Disney+",
        url: Some("https://www.disneyplus.com/"),
        executables: &[
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\Disney+.exe"),
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\DisneyPlus.exe"),
        ],
    },
    Service {
        id: "Prime Video",
        url: Some("https://www.primevideo.com/"),
        executables: &[
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\AmazonPrimeVideo.exe"),
            ("LOCALAPPDATA", r"Microsoft\WindowsApps\PrimeVideo.exe"),
        ],
    },
    Service {
        id: "Apple TV+",
        url: Some("https://tv.apple.com/"),
        executables: &[],
    },
    Service {
        id: "Hulu",
        url: Some("https://www.hulu.com/"),
        executables: &[],
    },
    Service {
        id: "Max",
        url: Some("https://play.max.com/"),
        executables: &[],
    },
    Service {
        id: "Peacock",
        url: Some("https://www.peacocktv.com/"),
        executables: &[],
    },
    Service {
        id: "Custom",
        url: None,
        executables: &[],
    },
];

fn service(id: &str) -> Option<&'static Service> {
    SERVICES.iter().find(|service| service.id == id)
}

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
    let Ok(resolved) = value.canonicalize() else {
        return Ok(value.to_path_buf());
    };
    if resolved.to_string_lossy().starts_with(r"\\?\UNC\") {
        return Err("Network executables are not allowed.".into());
    }
    Ok(resolved)
}

pub(crate) fn detect(name: &str) -> Option<PathBuf> {
    known_paths(name)
        .into_iter()
        .find(|candidate| validate(&candidate.to_string_lossy()).is_ok())
        .or_else(|| registry_install(name))
}

fn known_paths(name: &str) -> Vec<PathBuf> {
    let Some(service) = service(name) else {
        return Vec::new();
    };
    service
        .executables
        .iter()
        .filter_map(|(variable, suffix)| {
            std::env::var_os(variable).map(|root| PathBuf::from(root).join(suffix))
        })
        .collect()
}

#[cfg(windows)]
fn registry_install(name: &str) -> Option<PathBuf> {
    registry_app(name).or_else(|| uninstall_app(name))
}

#[cfg(not(windows))]
fn registry_install(_: &str) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn registry_app(name: &str) -> Option<PathBuf> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let executable = format!("{}.exe", name.to_lowercase());
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for prefix in ["Software", r"Software\WOW6432Node"] {
            let key = format!(r"{prefix}\Microsoft\Windows\CurrentVersion\App Paths\{executable}");
            let Some(value) = reg_text(hive, Some(&key), None) else {
                continue;
            };
            let value = value.trim_matches('"');
            if validate(value).is_ok() {
                return Some(PathBuf::from(value));
            }
        }
    }
    None
}

#[cfg(windows)]
fn uninstall_app(name: &str) -> Option<PathBuf> {
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::System::Registry::{
            RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        },
    };
    let name = name.to_lowercase();
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let mut root = HKEY::default();
            let opened = unsafe {
                RegOpenKeyExW(
                    hive,
                    PCWSTR(wide(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall").as_ptr()),
                    None,
                    KEY_READ | view,
                    &mut root,
                )
            };
            if opened.is_err() {
                continue;
            }
            for index in 0.. {
                let mut buffer = [0u16; 256];
                let mut length = buffer.len() as u32;
                let status = unsafe {
                    RegEnumKeyExW(
                        root,
                        index,
                        Some(PWSTR(buffer.as_mut_ptr())),
                        &mut length,
                        None,
                        None,
                        None,
                        None,
                    )
                };
                if status.is_err() {
                    break;
                }
                let subkey = String::from_utf16_lossy(&buffer[..length as usize]);
                let mut key = HKEY::default();
                let opened = unsafe {
                    RegOpenKeyExW(
                        root,
                        PCWSTR(wide(&subkey).as_ptr()),
                        None,
                        KEY_READ | view,
                        &mut key,
                    )
                };
                if opened.is_err() {
                    continue;
                }
                let matched = reg_text(key, None, Some("DisplayName"))
                    .is_some_and(|value| value.to_lowercase().contains(&name));
                let icon = if matched {
                    reg_text(key, None, Some("DisplayIcon"))
                        .and_then(|value| icon_path(&value))
                        .filter(|path| validate(&path.to_string_lossy()).is_ok())
                } else {
                    None
                };
                let _ = unsafe { RegCloseKey(key) };
                if let Some(icon) = icon {
                    let _ = unsafe { RegCloseKey(root) };
                    return Some(icon);
                }
            }
            let _ = unsafe { RegCloseKey(root) };
        }
    }
    None
}

#[cfg(windows)]
fn icon_path(value: &str) -> Option<PathBuf> {
    let text = match value.trim().split_once('"') {
        Some((_, rest)) => rest.split_once('"').map_or(rest, |(path, _)| path),
        None => value.trim(),
    };
    let text = match text.rsplit_once(',') {
        Some((path, index)) if index.trim().parse::<u32>().is_ok() => path,
        _ => text,
    };
    let path = PathBuf::from(text.trim());
    path.extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
        .then_some(path)
}

#[cfg(windows)]
fn reg_text(
    key: windows::Win32::System::Registry::HKEY,
    subkey: Option<&str>,
    name: Option<&str>,
) -> Option<String> {
    use windows::{
        core::PCWSTR,
        Win32::System::Registry::{RegGetValueW, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ},
    };
    let subkey = subkey.map(wide);
    let name = name.map(wide);
    let pointer = |value: &Option<Vec<u16>>| match value {
        Some(value) => PCWSTR(value.as_ptr()),
        None => PCWSTR::null(),
    };
    let mut buffer = [0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            key,
            pointer(&subkey),
            pointer(&name),
            RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    result.is_ok().then(|| {
        let length = buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..length])
    })
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
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
        .filter(|(id, _)| service(id).is_some())
        .collect();
    Ok(())
}

#[tauri::command]
pub fn app_paths(apps: State<Apps>) -> Result<BTreeMap<String, String>, String> {
    let paths = apps.0.lock().map_err(|e| e.to_string())?;
    Ok(SERVICES
        .iter()
        .map(|service| (service.id.into(), paths.get(service.id).cloned().unwrap_or_default()))
        .collect())
}

#[tauri::command]
pub fn detect_apps() -> Vec<DetectedApp> {
    SERVICES
        .iter()
        .filter(|service| !service.executables.is_empty())
        .map(|service| DetectedApp {
            name: service.id.into(),
            path: detect(service.id).map(|p| p.to_string_lossy().into_owned()),
        })
        .collect()
}

#[tauri::command]
pub fn save_app_paths(
    app: tauri::AppHandle,
    apps: State<Apps>,
    paths: BTreeMap<String, String>,
) -> Result<(), String> {
    if paths.keys().any(|id| service(id).is_none()) {
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
pub fn launch_app(name: String, apps: State<Apps>, app: tauri::AppHandle) -> Result<(), String> {
    let service = service(&name).ok_or("Application is not approved.")?;
    if name == "Stremio" {
        crate::commands::stremio::show(app.clone(), None)?;
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    let configured = apps
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(&name)
        .cloned()
        .unwrap_or_default();
    if name == "Spotify" {
        if !crate::integrations::spotify::process::running() {
            let detected = if configured.is_empty() {
                detect(&name)
            } else {
                Some(validate(&configured)?)
            };
            let executable = detected.ok_or(
                "Spotify was not found. Install it or set its executable path in Settings.",
            )?;
            crate::integrations::spotify::process::launch_path_minimized(&executable)?;
        }
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    if crate::commands::session::reuse(&app, &name) {
        if let Some(window) = app.get_webview_window("main") {
            window.minimize().map_err(|e| e.to_string())?;
        }
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    let detected = if configured.is_empty() {
        detect(&name)
    } else {
        Some(validate(&configured)?)
    };
    let (executable, arguments) = match detected {
        Some(executable) => (executable, Vec::new()),
        None => {
            let Some(url) = service.url else {
                return Err(format!(
                    "{name} was not found. Install it or set its executable path in Settings."
                ));
            };
            let browser = edge().ok_or_else(|| {
                format!("Microsoft Edge is required to open {name}. Install Edge or set an executable path in Settings.")
            })?;
            (browser, edge_arguments(&app, service.id, url)?)
        }
    };
    let mut command = std::process::Command::new(&executable);
    if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    let child = command
        .args(&arguments)
        .spawn()
        .map_err(|e| format!("Could not start {name}: {e}"))?;
    crate::commands::session::track(&app, &name, child.id());
    crate::activity::record_launch(&app, &name);
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn edge() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
        .iter()
        .filter_map(|variable| {
            std::env::var_os(variable)
                .map(|root| PathBuf::from(root).join(r"Microsoft\Edge\Application\msedge.exe"))
        })
        .find(|candidate| candidate.is_file())
}

fn edge_arguments(app: &tauri::AppHandle, id: &str, url: &str) -> Result<Vec<String>, String> {
    let profile = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("services")
        .join(id.to_lowercase().replace('+', "plus").replace(' ', "-"));
    Ok([
        format!("--app={url}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--start-maximized".into(),
    ]
    .to_vec())
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
