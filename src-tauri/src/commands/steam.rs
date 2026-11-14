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
    playtime_minutes: u64,
}

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
    open_in_steam(&app, format!("steam://rungameid/{appid}")).await
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
    if appid == 0 {
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
    let Some(config) = local_config(&root) else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(config).map_err(|error| error.to_string())?;
    let data = parse_vdf(&text).ok_or("Steam's local play history could not be read.")?;
    let apps = data
        .path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
        .and_then(Vdf::map);
    let Some(apps) = apps else {
        return Ok(Vec::new());
    };
    let mut recent: Vec<(u64, u32, u64)> = apps
        .iter()
        .filter_map(|(key, entry)| {
            let appid = key.parse::<u32>().ok()?;
            let last_played = number(entry, "LastPlayed")?;
            let playtime = number(entry, "Playtime").unwrap_or(0);
            (last_played > 0 && playtime > 0).then_some((last_played, appid, playtime))
        })
        .collect();
    recent.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    let names = installed_names(&library_paths(&root));
    let icons = root.join("appcache").join("librarycache");
    Ok(recent
        .into_iter()
        .filter_map(|(_, appid, playtime_minutes)| {
            let name = names.get(&appid)?.clone();
            Some(SteamGame {
                appid,
                name,
                icon: local_icon(&icons, appid).unwrap_or_default(),
                hero: artwork(appid, "library_hero.jpg"),
                playtime_minutes,
            })
        })
        .take(12)
        .collect())
}

fn number(node: &Vdf, key: &str) -> Option<u64> {
    node.get(key)?.text()?.parse().ok()
}

fn artwork(appid: u32, file: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/{file}")
}

fn local_icon(cache: &Path, appid: u32) -> Option<String> {
    for entry in std::fs::read_dir(cache.join(appid.to_string())).ok()?.flatten() {
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
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

fn steam_root() -> Option<PathBuf> {
    let path = crate::commands::apps::detect("Steam")?;
    path.parent().map(Path::to_path_buf)
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
}
