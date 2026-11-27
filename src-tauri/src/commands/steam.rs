use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};
use tauri::{AppHandle, Manager};

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .user_agent("Luma/0.1")
            .build()
            .unwrap_or_default()
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamGame {
    appid: u32,
    name: String,
    icon: String,
    hero: String,
    header: String,
    playtime_minutes: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SteamShortcutInfo {
    appid: u32,
    name: String,
    exe: String,
    start_dir: String,
}

const NON_STEAM_APPID_MASK: u32 = 0x8000_0000;

pub(crate) fn launch(app: &AppHandle) -> Result<(), String> {
    if crate::commands::session::focus_steam(app) {
        return Ok(());
    }
    if crate::commands::apps::detect("Steam").is_none() {
        return Err("Steam was not found. Install Steam, then try again.".into());
    }
    open("steam://open/bigpicture")?;
    crate::commands::session::track_steam(app);
    Ok(())
}

pub(crate) fn close() {
    let _ = open("steam://close/bigpicture");
}

#[tauri::command]
pub async fn steam_recent_games() -> Result<Vec<SteamGame>, String> {
    tauri::async_runtime::spawn_blocking(recent_games)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn steam_open_game(app: AppHandle, appid: u32) -> Result<(), String> {
    if appid == 0 {
        return Err("That game does not have a valid Steam ID.".into());
    }
    if appid >= NON_STEAM_APPID_MASK {
        return tauri::async_runtime::spawn_blocking(move || open_shortcut_game(&app, appid))
            .await
            .map_err(|error| error.to_string())?;
    }
    open_in_steam(&app, format!("steam://rungameid/{appid}")).await
}

#[tauri::command]
pub async fn steam_shortcuts() -> Result<Vec<SteamShortcutInfo>, String> {
    tauri::async_runtime::spawn_blocking(list_shortcuts)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn steam_add_shortcut(
    name: String,
    exe: String,
    start_dir: Option<String>,
) -> Result<SteamShortcutInfo, String> {
    tauri::async_runtime::spawn_blocking(move || add_shortcut(name, exe, start_dir))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn steam_remove_shortcut(appid: u32) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || remove_shortcut(appid))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn steam_open_store(app: AppHandle, appid: u32) -> Result<(), String> {
    if appid == 0 {
        return Err("That game does not have a valid Steam ID.".into());
    }
    open_in_steam(&app, format!("steam://store/{appid}")).await
}

#[tauri::command]
pub async fn steam_recommendations(appid: u32) -> Result<Vec<SteamGame>, String> {
    if appid == 0 || appid >= NON_STEAM_APPID_MASK {
        return Ok(Vec::new());
    }
    let page = client()
        .get(format!(
            "https://store.steampowered.com/recommended/morelike/app/{appid}/"
        ))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    let ids = recommendation_ids(&page, appid);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let input = serde_json::json!({
        "ids": ids.iter().map(|id| serde_json::json!({ "appid": id })).collect::<Vec<_>>(),
        "context": { "language": "english", "country_code": "US" },
        "data_request": { "include_assets": true }
    });
    let items = client()
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
        .query(&[("input_json", input.to_string())])
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json::<StoreItems>()
        .await
        .map_err(|error| error.to_string())?;
    let items: BTreeMap<u32, (String, String)> = items
        .response
        .store_items
        .into_iter()
        .filter_map(|item| {
            let name = item.name?;
            let icon = item
                .assets
                .and_then(|assets| assets.community_icon)
                .map(|hash| {
                    format!(
                        "https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{}/{hash}.jpg",
                        item.id
                    )
                })
                .unwrap_or_default();
            Some((item.id, (name, icon)))
        })
        .collect();
    Ok(ids
        .into_iter()
        .filter_map(|id| {
            let (name, icon) = items.get(&id)?.clone();
            Some(SteamGame {
                appid: id,
                name,
                icon,
                hero: artwork(id, "library_hero.jpg"),
                header: artwork(id, "header.jpg"),
                playtime_minutes: 0,
            })
        })
        .collect())
}

async fn open_in_steam(app: &AppHandle, target: String) -> Result<(), String> {
    if !crate::commands::session::focus_steam(app) {
        open("steam://open/bigpicture")?;
        crate::commands::session::track_steam(app);
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }
    open(&target)?;
    crate::activity::record_launch(app, "Steam");
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|error| error.to_string())?;
        crate::background::publish_visibility(&window, false);
    }
    Ok(())
}

#[derive(Deserialize)]
struct StoreItems {
    response: StoreResponse,
}

#[derive(Deserialize)]
struct StoreResponse {
    #[serde(default)]
    store_items: Vec<StoreItem>,
}

#[derive(Deserialize)]
struct StoreItem {
    id: u32,
    name: Option<String>,
    #[serde(default)]
    assets: Option<StoreAssets>,
}

#[derive(Deserialize)]
struct StoreAssets {
    community_icon: Option<String>,
}

fn recommendation_ids(page: &str, source: u32) -> Vec<u32> {
    const PREFIX: &str = "data-ds-appid=\"";
    let mut ids = Vec::new();
    let mut rest = page;
    while let Some(index) = rest.find(PREFIX) {
        rest = &rest[index + PREFIX.len()..];
        let digits: String = rest
            .chars()
            .take_while(|value| value.is_ascii_digit())
            .collect();
        let Ok(id) = digits.parse::<u32>() else {
            continue;
        };
        if id != source && !ids.contains(&id) {
            ids.push(id);
        }
        if ids.len() == 10 {
            break;
        }
    }
    ids
}

fn recent_games() -> Result<Vec<SteamGame>, String> {
    let Some(root) = steam_root() else {
        return Ok(Vec::new());
    };
    let mut entries: Vec<(u64, SteamGame)> = Vec::new();
    if let Some(config) = local_config(&root) {
        if let Ok(text) = std::fs::read_to_string(&config) {
            if let Some(apps) = parse_vdf(&text)
                .as_ref()
                .and_then(|data| data.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]))
                .and_then(Vdf::map)
            {
                let names = installed_names(&library_paths(&root));
                let icons = root.join("appcache").join("librarycache");
                for (key, entry) in apps {
                    let Some(appid) = key.parse::<u32>().ok() else {
                        continue;
                    };
                    let Some(last_played) = number(entry, "LastPlayed") else {
                        continue;
                    };
                    let playtime_minutes = number(entry, "Playtime").unwrap_or(0);
                    if last_played == 0 || playtime_minutes == 0 {
                        continue;
                    }
                    let Some(name) = names.get(&appid).cloned() else {
                        continue;
                    };
                    entries.push((
                        last_played,
                        SteamGame {
                            appid,
                            name,
                            icon: local_icon(&icons, appid).unwrap_or_default(),
                            hero: local_artwork(&icons, appid, "library_hero.jpg")
                                .or_else(|| local_artwork(&icons, appid, "library_header.jpg"))
                                .unwrap_or_else(|| artwork(appid, "library_hero.jpg")),
                            header: local_artwork(&icons, appid, "library_header.jpg")
                                .unwrap_or_else(|| artwork(appid, "header.jpg")),
                            playtime_minutes,
                        },
                    ));
                }
            }
        }
    }
    if let Some(config_dir) = userdata_config_dir(&root) {
        let shortcuts_path = config_dir.join("shortcuts.vdf");
        let grid_dir = config_dir.join("grid");
        if let Ok(shortcuts) = read_shortcuts_file(&shortcuts_path) {
            for shortcut in shortcuts {
                if shortcut_int(&shortcut, "IsHidden").unwrap_or(0) != 0 {
                    continue;
                }
                let info = shortcut_info(&shortcut);
                if info.name.is_empty() || info.exe.is_empty() {
                    continue;
                }
                let last_played = shortcut_int(&shortcut, "LastPlayTime").unwrap_or(0) as u64;
                let icon_path = shortcut_str(&shortcut, "icon")
                    .map(unquote_path)
                    .filter(|p| !p.is_empty());
                let icon = grid_image(&grid_dir, &[format!("{}_icon", info.appid)])
                    .or_else(|| icon_path.as_deref().and_then(|p| image_data_uri(Path::new(p))))
                    .unwrap_or_default();
                let header = grid_image(
                    &grid_dir,
                    &[format!("{}", info.appid), format!("{}p", info.appid)],
                )
                .unwrap_or_default();
                let hero = grid_image(
                    &grid_dir,
                    &[format!("{}_hero", info.appid), format!("{}", info.appid)],
                )
                .unwrap_or_else(|| header.clone());
                entries.push((
                    last_played,
                    SteamGame {
                        appid: info.appid,
                        name: info.name,
                        icon,
                        hero,
                        header,
                        playtime_minutes: 0,
                    },
                ));
            }
        }
    }
    entries.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    Ok(entries.into_iter().map(|(_, game)| game).take(12).collect())
}

fn number(node: &Vdf, key: &str) -> Option<u64> {
    node.get(key)?.text()?.parse().ok()
}

fn artwork(appid: u32, file: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/{file}")
}

fn local_artwork(cache: &Path, appid: u32, file: &str) -> Option<String> {
    let directory = cache.join(appid.to_string());
    if let Some(image) = data_uri(&directory.join(file)) {
        return Some(image);
    }
    let mut candidates: Vec<_> = std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path().join(file))
        .filter(|path| path.is_file())
        .collect();
    candidates.sort_by_key(|path| {
        std::cmp::Reverse(
            std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok(),
        )
    });
    candidates.iter().find_map(|path| data_uri(path))
}

fn local_icon(cache: &Path, appid: u32) -> Option<String> {
    for entry in std::fs::read_dir(cache.join(appid.to_string()))
        .ok()?
        .flatten()
    {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let small = entry.metadata().is_ok_and(|meta| meta.len() < 8192);
        if small && name.ends_with(".jpg") && !name.contains('_') {
            if let Some(icon) = data_uri(&path) {
                return Some(icon);
            }
        }
    }
    None
}

fn data_uri(path: &Path) -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[(first >> 2) as usize] as char);
        encoded.push(ALPHABET[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(third & 0b111111) as usize] as char
        } else {
            '='
        });
    }
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Some(format!("data:{mime};base64,{encoded}"))
}

fn image_data_uri(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        data_uri(path)
    } else {
        None
    }
}

fn grid_image(grid_dir: &Path, stems: &[String]) -> Option<String> {
    for stem in stems {
        for ext in ["png", "jpg", "jpeg", "webp"] {
            let candidate = grid_dir.join(format!("{stem}.{ext}"));
            if candidate.is_file() {
                if let Some(uri) = data_uri(&candidate) {
                    return Some(uri);
                }
            }
        }
    }
    None
}

fn steam_root() -> Option<PathBuf> {
    let path = crate::commands::apps::detect("Steam")?;
    path.parent().map(Path::to_path_buf)
}

fn userdata_config_dir(root: &Path) -> Option<PathBuf> {
    if let Some(config) = local_config(root) {
        return config.parent().map(Path::to_path_buf);
    }
    for entry in std::fs::read_dir(root.join("userdata")).ok()?.flatten() {
        let dir = entry.path().join("config");
        if dir.is_dir() {
            return Some(dir);
        }
    }
    None
}

fn unquote_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn strip_verbatim_prefix(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

fn now_unix_u32() -> u32 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or(0)
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn shortcut_appid(exe_quoted: &str, name: &str) -> u32 {
    let input = format!("{exe_quoted}{name}");
    crc32_ieee(input.as_bytes()) | NON_STEAM_APPID_MASK
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BinVdf {
    Map(Vec<(String, BinVdf)>),
    Str(String),
    Int(u32),
    Float(u32),
    Uint64(u64),
}

fn read_cstr(bytes: &[u8], cursor: &mut usize) -> Option<String> {
    let rest = bytes.get(*cursor..)?;
    let end = rest.iter().position(|&b| b == 0)?;
    let text = String::from_utf8_lossy(&rest[..end]).into_owned();
    *cursor += end + 1;
    Some(text)
}

fn parse_bin_map(bytes: &[u8], cursor: &mut usize) -> Option<Vec<(String, BinVdf)>> {
    let mut entries = Vec::new();
    while *cursor < bytes.len() {
        let kind = bytes[*cursor];
        *cursor += 1;
        if kind == 0x08 {
            return Some(entries);
        }
        let key = read_cstr(bytes, cursor)?;
        let value = match kind {
            0x00 => BinVdf::Map(parse_bin_map(bytes, cursor)?),
            0x01 => BinVdf::Str(read_cstr(bytes, cursor)?),
            0x02 => {
                let raw = bytes.get(*cursor..*cursor + 4)?;
                *cursor += 4;
                BinVdf::Int(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
            }
            0x03 => {
                let raw = bytes.get(*cursor..*cursor + 4)?;
                *cursor += 4;
                BinVdf::Float(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
            }
            0x07 => {
                let raw = bytes.get(*cursor..*cursor + 8)?;
                *cursor += 8;
                BinVdf::Uint64(u64::from_le_bytes([
                    raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                ]))
            }
            _ => return None,
        };
        entries.push((key, value));
    }
    Some(entries)
}

fn write_bin_map(entries: &[(String, BinVdf)], out: &mut Vec<u8>) {
    for (key, value) in entries {
        match value {
            BinVdf::Map(children) => {
                out.push(0x00);
                out.extend_from_slice(key.as_bytes());
                out.push(0);
                write_bin_map(children, out);
            }
            BinVdf::Str(text) => {
                out.push(0x01);
                out.extend_from_slice(key.as_bytes());
                out.push(0);
                out.extend_from_slice(text.as_bytes());
                out.push(0);
            }
            BinVdf::Int(number) => {
                out.push(0x02);
                out.extend_from_slice(key.as_bytes());
                out.push(0);
                out.extend_from_slice(&number.to_le_bytes());
            }
            BinVdf::Float(bits) => {
                out.push(0x03);
                out.extend_from_slice(key.as_bytes());
                out.push(0);
                out.extend_from_slice(&bits.to_le_bytes());
            }
            BinVdf::Uint64(number) => {
                out.push(0x07);
                out.extend_from_slice(key.as_bytes());
                out.push(0);
                out.extend_from_slice(&number.to_le_bytes());
            }
        }
    }
    out.push(0x08);
}

fn parse_shortcuts_bytes(bytes: &[u8]) -> Result<Vec<Vec<(String, BinVdf)>>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut cursor = 0;
    let root = parse_bin_map(bytes, &mut cursor)
        .ok_or("Steam shortcuts.vdf could not be parsed.")?;
    let shortcuts = root
        .into_iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("shortcuts"))
        .and_then(|(_, v)| match v {
            BinVdf::Map(items) => Some(items),
            _ => None,
        })
        .unwrap_or_default();
    Ok(shortcuts
        .into_iter()
        .filter_map(|(_, v)| match v {
            BinVdf::Map(fields) => Some(fields),
            _ => None,
        })
        .collect())
}

fn serialize_shortcuts_bytes(shortcuts: &[Vec<(String, BinVdf)>]) -> Vec<u8> {
    let indexed: Vec<(String, BinVdf)> = shortcuts
        .iter()
        .enumerate()
        .map(|(index, entry)| (index.to_string(), BinVdf::Map(entry.clone())))
        .collect();
    let root = vec![("shortcuts".to_string(), BinVdf::Map(indexed))];
    let mut out = Vec::new();
    write_bin_map(&root, &mut out);
    out
}

fn read_shortcuts_file(path: &Path) -> Result<Vec<Vec<(String, BinVdf)>>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    parse_shortcuts_bytes(&bytes)
}

fn write_shortcuts_file(path: &Path, shortcuts: &[Vec<(String, BinVdf)>]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serialize_shortcuts_bytes(shortcuts);
    let temp = path.with_extension("vdf.tmp");
    std::fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, path).map_err(|error| error.to_string())
}

fn shortcut_str<'a>(entry: &'a [(String, BinVdf)], key: &str) -> Option<&'a str> {
    entry.iter().find_map(|(k, v)| match v {
        BinVdf::Str(text) if k.eq_ignore_ascii_case(key) => Some(text.as_str()),
        _ => None,
    })
}

fn shortcut_int(entry: &[(String, BinVdf)], key: &str) -> Option<u32> {
    entry.iter().find_map(|(k, v)| match v {
        BinVdf::Int(number) if k.eq_ignore_ascii_case(key) => Some(*number),
        _ => None,
    })
}

fn set_shortcut_field(entry: &mut Vec<(String, BinVdf)>, key: &str, value: BinVdf) {
    if let Some((_, existing)) = entry.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
        *existing = value;
    } else {
        entry.push((key.to_string(), value));
    }
}

fn shortcut_info(entry: &[(String, BinVdf)]) -> SteamShortcutInfo {
    let raw_exe = shortcut_str(entry, "Exe").unwrap_or("");
    let exe = unquote_path(raw_exe);
    let raw_dir = shortcut_str(entry, "StartDir").unwrap_or("");
    let start_dir = unquote_path(raw_dir);
    let name = shortcut_str(entry, "AppName").unwrap_or("").to_string();
    let appid = shortcut_int(entry, "appid")
        .filter(|id| *id >= NON_STEAM_APPID_MASK)
        .unwrap_or_else(|| shortcut_appid(&format!("\"{exe}\""), &name));
    SteamShortcutInfo {
        appid,
        name,
        exe,
        start_dir,
    }
}

fn list_shortcuts() -> Result<Vec<SteamShortcutInfo>, String> {
    let Some(root) = steam_root() else {
        return Ok(Vec::new());
    };
    let Some(config_dir) = userdata_config_dir(&root) else {
        return Ok(Vec::new());
    };
    let shortcuts = read_shortcuts_file(&config_dir.join("shortcuts.vdf"))?;
    Ok(shortcuts
        .iter()
        .filter(|entry| shortcut_int(entry, "IsHidden").unwrap_or(0) == 0)
        .map(|entry| shortcut_info(entry))
        .filter(|info| !info.name.is_empty() && !info.exe.is_empty())
        .collect())
}

fn add_shortcut(
    name: String,
    exe: String,
    start_dir: Option<String>,
) -> Result<SteamShortcutInfo, String> {
    let root = steam_root().ok_or("Steam was not found. Install Steam, then try again.")?;
    let config_dir = userdata_config_dir(&root)
        .ok_or("Steam user configuration was not found. Sign into Steam once, then try again.")?;
    let exe_clean = unquote_path(&exe);
    let resolved_exe = crate::commands::apps::validate(&exe_clean)?;
    let exe_str = strip_verbatim_prefix(&resolved_exe);
    let resolved_dir = match start_dir.map(|d| unquote_path(&d)).filter(|d| !d.is_empty()) {
        Some(dir) => {
            let path = PathBuf::from(&dir);
            if !path.is_dir() {
                return Err("Choose an existing folder for the working directory.".into());
            }
            strip_verbatim_prefix(&path.canonicalize().unwrap_or(path))
        }
        None => resolved_exe
            .parent()
            .map(strip_verbatim_prefix)
            .ok_or("Could not determine the game's folder.")?,
    };
    let display_name = {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            resolved_exe
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Custom Game")
                .to_string()
        } else {
            trimmed.to_string()
        }
    };
    let exe_quoted = format!("\"{exe_str}\"");
    let dir_quoted = format!("\"{resolved_dir}\"");
    let appid = shortcut_appid(&exe_quoted, &display_name);
    let path = config_dir.join("shortcuts.vdf");
    let mut shortcuts = read_shortcuts_file(&path)?;
    let now = now_unix_u32();
    let existing = shortcuts.iter_mut().find(|entry| {
        let info = shortcut_info(entry);
        info.appid == appid || info.exe.eq_ignore_ascii_case(&exe_str)
    });
    if let Some(entry) = existing {
        set_shortcut_field(entry, "appid", BinVdf::Int(appid));
        set_shortcut_field(entry, "AppName", BinVdf::Str(display_name.clone()));
        set_shortcut_field(entry, "Exe", BinVdf::Str(exe_quoted));
        set_shortcut_field(entry, "StartDir", BinVdf::Str(dir_quoted));
        set_shortcut_field(entry, "IsHidden", BinVdf::Int(0));
        set_shortcut_field(entry, "LastPlayTime", BinVdf::Int(now));
    } else {
        shortcuts.push(vec![
            ("appid".into(), BinVdf::Int(appid)),
            ("AppName".into(), BinVdf::Str(display_name.clone())),
            ("Exe".into(), BinVdf::Str(exe_quoted)),
            ("StartDir".into(), BinVdf::Str(dir_quoted)),
            ("icon".into(), BinVdf::Str(String::new())),
            ("ShortcutPath".into(), BinVdf::Str(String::new())),
            ("LaunchOptions".into(), BinVdf::Str(String::new())),
            ("IsHidden".into(), BinVdf::Int(0)),
            ("AllowDesktopConfig".into(), BinVdf::Int(1)),
            ("AllowOverlay".into(), BinVdf::Int(1)),
            ("OpenVR".into(), BinVdf::Int(0)),
            ("Devkit".into(), BinVdf::Int(0)),
            ("DevkitGameID".into(), BinVdf::Str(String::new())),
            ("DevkitOverrideAppID".into(), BinVdf::Int(0)),
            ("LastPlayTime".into(), BinVdf::Int(now)),
            ("FlatpakAppID".into(), BinVdf::Str(String::new())),
            ("tags".into(), BinVdf::Map(Vec::new())),
        ]);
    }
    write_shortcuts_file(&path, &shortcuts)?;
    Ok(SteamShortcutInfo {
        appid,
        name: display_name,
        exe: exe_str,
        start_dir: resolved_dir,
    })
}

fn remove_shortcut(appid: u32) -> Result<(), String> {
    let Some(root) = steam_root() else {
        return Ok(());
    };
    let Some(config_dir) = userdata_config_dir(&root) else {
        return Ok(());
    };
    let path = config_dir.join("shortcuts.vdf");
    let mut shortcuts = read_shortcuts_file(&path)?;
    let before = shortcuts.len();
    shortcuts.retain(|entry| shortcut_info(entry).appid != appid);
    if shortcuts.len() != before {
        write_shortcuts_file(&path, &shortcuts)?;
    }
    Ok(())
}

fn open_shortcut_game(app: &AppHandle, appid: u32) -> Result<(), String> {
    let root = steam_root().ok_or("Steam was not found. Install Steam, then try again.")?;
    let config_dir = userdata_config_dir(&root)
        .ok_or("Steam user configuration was not found.")?;
    let path = config_dir.join("shortcuts.vdf");
    let mut shortcuts = read_shortcuts_file(&path)?;
    let Some(entry) = shortcuts
        .iter_mut()
        .find(|entry| shortcut_info(entry).appid == appid)
    else {
        return Err("That custom Steam game could not be found.".into());
    };
    let info = shortcut_info(entry);
    let executable = crate::commands::apps::validate(&info.exe)?;
    let work_dir = if !info.start_dir.is_empty() && Path::new(&info.start_dir).is_dir() {
        PathBuf::from(&info.start_dir)
    } else if let Some(parent) = executable.parent() {
        parent.to_path_buf()
    } else {
        return Err("Could not determine the game's folder.".into());
    };
    let mut command = std::process::Command::new(&executable);
    command.current_dir(&work_dir);
    let child = command
        .spawn()
        .map_err(|error| format!("Could not start {}: {error}", info.name))?;
    set_shortcut_field(entry, "LastPlayTime", BinVdf::Int(now_unix_u32()));
    let _ = write_shortcuts_file(&path, &shortcuts);
    crate::commands::session::track(app, &info.name, child.id());
    crate::activity::record_launch(app, "Steam");
    if let Some(window) = app
        .get_webview_window("main")
        .filter(|_| !crate::viewing::enabled(app))
    {
        window.minimize().map_err(|error| error.to_string())?;
        crate::background::publish_visibility(&window, false);
    }
    Ok(())
}

fn local_config(root: &Path) -> Option<PathBuf> {
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(root.join("userdata")).ok()?.flatten() {
        let config = entry.path().join("config").join("localconfig.vdf");
        let Ok(modified) = std::fs::metadata(&config).and_then(|meta| meta.modified()) else {
            continue;
        };
        if newest.as_ref().map_or(true, |(time, _)| modified > *time) {
            newest = Some((modified, config));
        }
    }
    newest.map(|(_, config)| config)
}

fn library_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![root.to_path_buf()];
    if let Ok(text) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        if let Some(folders) = parse_vdf(&text)
            .as_ref()
            .and_then(|data| data.get("libraryfolders"))
            .and_then(Vdf::map)
        {
            for folder in folders.values() {
                if let Some(path) = folder.get("path").and_then(Vdf::text) {
                    paths.push(PathBuf::from(path));
                }
            }
        }
    }
    paths
}

fn installed_names(paths: &[PathBuf]) -> BTreeMap<u32, String> {
    let mut names = BTreeMap::new();
    for path in paths {
        let Ok(entries) = std::fs::read_dir(path.join("steamapps")) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(file) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(id) = file
                .strip_prefix("appmanifest_")
                .and_then(|name| name.strip_suffix(".acf"))
                .and_then(|id| id.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            if let Some(name) = parse_vdf(&text)
                .as_ref()
                .and_then(|data| data.path(&["AppState", "name"]))
                .and_then(Vdf::text)
            {
                names.insert(id, name.to_owned());
            }
        }
    }
    names
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Vdf {
    Text(String),
    Map(BTreeMap<String, Vdf>),
}

impl Vdf {
    fn map(&self) -> Option<&BTreeMap<String, Vdf>> {
        match self {
            Self::Map(map) => Some(map),
            Self::Text(_) => None,
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Map(_) => None,
        }
    }

    fn get(&self, key: &str) -> Option<&Vdf> {
        self.map()?.get(key)
    }

    fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |node, key| node.get(key))
    }
}

struct Parser<'a> {
    input: std::str::Chars<'a>,
    peeked: Option<char>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.chars(),
            peeked: None,
        }
    }

    fn peek(&mut self) -> Option<char> {
        if self.peeked.is_none() {
            self.peeked = self.input.next();
        }
        self.peeked
    }

    fn next(&mut self) -> Option<char> {
        self.peeked.take().or_else(|| self.input.next())
    }

    fn skip_space(&mut self) {
        loop {
            match self.peek() {
                Some(value) if value.is_whitespace() => {
                    self.next();
                }
                Some('/') => {
                    self.next();
                    if self.peek() == Some('/') {
                        while self.next().is_some_and(|value| value != '\n') {}
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        loop {
            match self.next()? {
                '"' => return Some(out),
                '\\' => out.push(match self.next()? {
                    'n' => '\n',
                    't' => '\t',
                    value => value,
                }),
                value => out.push(value),
            }
        }
    }

    fn condition(&mut self) {
        self.skip_space();
        if self.peek() == Some('[') {
            while self.next().is_some_and(|value| value != ']') {}
        }
    }

    fn object(&mut self) -> Option<BTreeMap<String, Vdf>> {
        self.skip_space();
        if self.next()? != '{' {
            return None;
        }
        let mut entries = BTreeMap::new();
        loop {
            self.skip_space();
            match self.peek()? {
                '}' => {
                    self.next();
                    return Some(entries);
                }
                '"' => {
                    let key = self.string()?;
                    self.skip_space();
                    let value = if self.peek()? == '{' {
                        Vdf::Map(self.object()?)
                    } else {
                        Vdf::Text(self.string()?)
                    };
                    self.condition();
                    entries.insert(key, value);
                }
                _ => return None,
            }
        }
    }
}

fn parse_vdf(input: &str) -> Option<Vdf> {
    let mut parser = Parser::new(input);
    parser.skip_space();
    let key = parser.string()?;
    parser.skip_space();
    if parser.peek()? != '{' {
        return None;
    }
    Some(Vdf::Map(BTreeMap::from([(
        key,
        Vdf::Map(parser.object()?),
    )])))
}

#[cfg(windows)]
fn open(target: &str) -> Result<(), String> {
    use windows::{
        core::PCWSTR,
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let operation = wide("open");
    let target = wide(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not open Steam.".into());
    }
    Ok(())
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn open(_: &str) -> Result<(), String> {
    Err("Steam requires Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_app_play_history() {
        let data = parse_vdf(
            r#"
"UserLocalConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"392160"
					{
						"LastPlayed"		"1794648824"
						"Playtime"		"3310"
						"Playtime2wks"		"45" [$WIN32]
					}
				}
			}
		}
	}
}
"#,
        )
        .expect("localconfig parses");
        let app = data
            .path(&[
                "UserLocalConfigStore",
                "Software",
                "Valve",
                "Steam",
                "apps",
                "392160",
            ])
            .expect("app entry is present");
        assert_eq!(number(app, "LastPlayed"), Some(1_794_648_824));
        assert_eq!(number(app, "Playtime"), Some(3310));
        assert_eq!(number(app, "Playtime2wks"), Some(45));
    }

    #[test]
    fn unescapes_library_paths() {
        let data = parse_vdf(
            r#""libraryfolders"
{
	"0"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#,
        )
        .expect("libraryfolders parses");
        let path = data
            .path(&["libraryfolders", "0", "path"])
            .and_then(Vdf::text);
        assert_eq!(path, Some(r"D:\SteamLibrary"));
    }

    #[test]
    fn round_trips_binary_shortcuts_vdf() {
        let exe = r#""C:\Games\The Jackbox Party Pack 12\The Jackbox Party Pack 12.exe""#;
        let dir = r#""C:\Games\The Jackbox Party Pack 12""#;
        let name = "The Jackbox Party Pack 12";
        let appid = shortcut_appid(exe, name);
        assert!(appid >= NON_STEAM_APPID_MASK);
        let entry = vec![
            ("appid".into(), BinVdf::Int(appid)),
            ("AppName".into(), BinVdf::Str(name.into())),
            ("Exe".into(), BinVdf::Str(exe.into())),
            ("StartDir".into(), BinVdf::Str(dir.into())),
            ("IsHidden".into(), BinVdf::Int(0)),
            ("LastPlayTime".into(), BinVdf::Int(1_795_000_000)),
            ("tags".into(), BinVdf::Map(vec![("0".into(), BinVdf::Str("Party".into()))])),
        ];
        let bytes = serialize_shortcuts_bytes(&[entry]);
        let parsed = parse_shortcuts_bytes(&bytes).expect("shortcuts parse");
        assert_eq!(parsed.len(), 1);
        let info = shortcut_info(&parsed[0]);
        assert_eq!(info.appid, appid);
        assert_eq!(info.name, name);
        assert_eq!(info.exe, r"C:\Games\The Jackbox Party Pack 12\The Jackbox Party Pack 12.exe");
        assert_eq!(info.start_dir, r"C:\Games\The Jackbox Party Pack 12");
        assert_eq!(shortcut_int(&parsed[0], "LastPlayTime"), Some(1_795_000_000));
    }
}
