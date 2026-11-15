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
        id: "Steam",
        url: None,
        executables: &[
            ("ProgramFiles(x86)", r"Steam\steam.exe"),
            ("ProgramFiles", r"Steam\steam.exe"),
        ],
    },
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
            (
                "LOCALAPPDATA",
                r"Microsoft\WindowsApps\AmazonPrimeVideo.exe",
            ),
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

pub(crate) struct CatalogEntry {
    pub command: String,
    pub name: &'static str,
    pub icon: &'static [u8],
}

pub(crate) fn slug(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn icon(id: &str) -> Option<&'static [u8]> {
    Some(match id {
        "YouTube" => include_bytes!("../../resources/icons/youtube.png"),
        "Steam" => include_bytes!("../../resources/icons/steam.png"),
        "Spotify" => include_bytes!("../../resources/icons/spotify.png"),
        "Stremio" => include_bytes!("../../resources/icons/stremio.png"),
        "Netflix" => include_bytes!("../../resources/icons/netflix.png"),
        "Disney+" => include_bytes!("../../resources/icons/disneyplus.png"),
        "Prime Video" => include_bytes!("../../resources/icons/primevideo.png"),
        "Apple TV+" => include_bytes!("../../resources/icons/appletv.png"),
        "Hulu" => include_bytes!("../../resources/icons/hulu.png"),
        "Max" => include_bytes!("../../resources/icons/max.png"),
        "Peacock" => include_bytes!("../../resources/icons/peacock.png"),
        "Custom" => include_bytes!("../../resources/icons/custom.png"),
        _ => return None,
    })
}

pub(crate) fn catalog() -> Vec<CatalogEntry> {
    std::iter::once("YouTube")
        .chain(SERVICES.iter().map(|service| service.id))
        .filter_map(|id| {
            icon(id).map(|icon| CatalogEntry {
                command: slug(id),
                name: id,
                icon,
            })
        })
        .chain(super::sports::shortcuts())
        .collect()
}

pub(crate) fn app_command(command: &str) -> Option<&'static str> {
    catalog()
        .into_iter()
        .find(|entry| entry.command == command)
        .map(|entry| entry.name)
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
        return Err("Choose an existing .exe file on this PC.".into());
    }
    let Ok(resolved) = value.canonicalize() else {
        return Ok(value.to_path_buf());
    };
    if resolved.to_string_lossy().starts_with(r"\\?\UNC\") {
        return Err("Apps on a network drive are not supported.".into());
    }
    Ok(resolved)
}

pub(crate) fn detect(name: &str) -> Option<PathBuf> {
    if name == "Steam" {
        return steam_path();
    }
    known_paths(name)
        .into_iter()
        .find(|candidate| validate(&candidate.to_string_lossy()).is_ok())
        .or_else(|| registry_install(name))
}

fn steam_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let registry = reg_text(
            HKEY_CURRENT_USER,
            Some(r"Software\Valve\Steam"),
            Some("SteamExe"),
        )
        .or_else(|| {
            reg_text(
                HKEY_LOCAL_MACHINE,
                Some(r"SOFTWARE\WOW6432Node\Valve\Steam"),
                Some("InstallPath"),
            )
            .map(|path| format!(r"{}\steam.exe", path.trim_matches('"')))
        });
        registry
            .and_then(|path| validate(path.trim_matches('"')).ok())
            .or_else(|| {
                known_paths("Steam")
                    .into_iter()
                    .find(|candidate| validate(&candidate.to_string_lossy()).is_ok())
            })
    }
    #[cfg(not(windows))]
    {
        None
    }
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
        .map(|service| {
            let path = if service.id == "Stremio" {
                String::new()
            } else {
                paths.get(service.id).cloned().unwrap_or_default()
            };
            (service.id.into(), path)
        })
        .collect())
}

#[tauri::command]
pub async fn detect_apps() -> Result<Vec<DetectedApp>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        SERVICES
            .iter()
            .filter(|service| !service.executables.is_empty())
            .map(|service| DetectedApp {
                name: service.id.into(),
                path: detect(service.id).map(|p| p.to_string_lossy().into_owned()),
            })
            .collect()
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_app_paths(
    app: tauri::AppHandle,
    apps: State<Apps>,
    paths: BTreeMap<String, String>,
) -> Result<(), String> {
    if paths.keys().any(|id| service(id).is_none()) {
        return Err("One of those apps is not recognized by Luma.".into());
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
pub async fn launch_app(
    name: String,
    apps: State<'_, Apps>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(entry) = super::sports::shortcuts().into_iter().find(|entry| entry.name == name) {
        let league = super::sports::shortcut_league(&entry.command).ok_or("Unknown sports app")?;
        crate::remote::server::open_sports(&app, league).await?;
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    let service = service(&name).ok_or("This app is not set up in Luma yet.")?;
    if name == "Spotify" {
        crate::remote::server::open_music(&app)?;
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    if name == "Stremio" {
        crate::commands::stremio::show(app.clone(), None).await?;
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    if name == "Steam" {
        crate::commands::steam::launch(&app)?;
        crate::activity::record_launch(&app, &name);
        if let Some(window) = app.get_webview_window("main") {
            window.minimize().map_err(|e| e.to_string())?;
            crate::background::publish_visibility(&window, false);
        }
        return Ok(());
    }
    let configured = apps
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .get(&name)
        .cloned()
        .unwrap_or_default();
    if let Some(url) = service.url {
        if crate::commands::services::reuse(&app, service.id, url) {
            if let Some(window) = app.get_webview_window("main") {
                window.minimize().map_err(|e| e.to_string())?;
                crate::background::publish_visibility(&window, false);
            }
            crate::activity::record_launch(&app, &name);
            return Ok(());
        }
    }
    if crate::commands::session::reuse(&app, &name) {
        if let Some(window) = app.get_webview_window("main") {
            window.minimize().map_err(|e| e.to_string())?;
            crate::background::publish_visibility(&window, false);
        }
        crate::activity::record_launch(&app, &name);
        return Ok(());
    }
    let detected = if configured.is_empty() {
        detect(&name)
    } else {
        Some(validate(&configured)?)
    };
    if detected.is_none() {
        if let Some(url) = service.url {
            return crate::commands::services::launch(&app, service.id, url).await;
        }
        return Err(format!(
            "{name} was not found. Install it or choose its program under App locations in Settings."
        ));
    }
    let executable = detected.expect("a detected executable is present");
    let mut command = std::process::Command::new(&executable);
    if let Some(parent) = executable.parent() {
        command.current_dir(parent);
    }
    let child = command
        .spawn()
        .map_err(|e| format!("Could not start {name}: {e}"))?;
    crate::commands::session::track(&app, &name, child.id());
    crate::activity::record_launch(&app, &name);
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|e| e.to_string())?;
        crate::background::publish_visibility(&window, false);
    }
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
