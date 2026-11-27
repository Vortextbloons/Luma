use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};
mod windows;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Single,
    Split,
    Displays,
    Floating,
}

#[derive(Clone, Default, Serialize)]
pub struct Layout {
    mode: Mode,
    selected: Option<String>,
    players: Vec<String>,
    names: HashMap<String, String>,
    external: Vec<String>,
    #[serde(skip)]
    windows: HashMap<String, windows::Window>,
}

#[derive(Default)]
pub struct Viewing(Mutex<Layout>);

impl Layout {
    fn can_add(&self, name: &str) -> bool {
        self.mode == Mode::Single
            || self.players.len() < 2
            || self.players.iter().any(|label| {
                label == name || self.names.get(label).is_some_and(|value| value == name)
            })
    }
}

pub fn enabled(app: &tauri::AppHandle) -> bool {
    app.state::<Viewing>()
        .0
        .lock()
        .is_ok_and(|layout| layout.mode != Mode::Single)
}

pub fn active_selection(app: &tauri::AppHandle) -> Option<String> {
    let layout = live(app).ok()?;
    let external = layout
        .selected
        .as_ref()
        .is_some_and(|label| layout.windows.contains_key(label));
    (layout.mode != Mode::Single || external)
        .then_some(layout.selected)
        .flatten()
}

pub fn external_selected(app: &tauri::AppHandle) -> bool {
    live(app).is_ok_and(|layout| {
        layout
            .selected
            .as_ref()
            .is_some_and(|label| layout.windows.contains_key(label))
    })
}

pub fn external_source(app: &tauri::AppHandle) -> Option<String> {
    let layout = live(app).ok()?;
    let window = layout.windows.get(layout.selected.as_ref()?)?;
    crate::commands::services::window_source(app, window.id)
}

pub fn refresh_external_focus(app: &tauri::AppHandle) {
    let Ok(layout) = live(app) else {
        return;
    };
    if let Some((label, _)) = layout
        .windows
        .iter()
        .find(|(label, window)| layout.selected.as_ref() != Some(label) && window.focused())
    {
        focus(app, label);
    }
}

pub fn external_name(app: &tauri::AppHandle) -> Option<String> {
    let layout = live(app).ok()?;
    let key = layout.selected.as_ref()?;
    layout.windows.get(key)?;
    layout.names.get(key).cloned()
}

pub fn check_capacity(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let layout = live(app)?;
    if !layout.can_add(name) {
        return Err(
            "Two players are already open. Remove one in Multi Watch before adding another.".into(),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn viewing_windows() -> Result<Vec<windows::Choice>, String> {
    windows::list()
}

#[tauri::command]
pub fn viewing_sources() -> Vec<windows::Choice> {
    crate::commands::apps::video_sources()
        .into_iter()
        .map(|name| windows::Choice {
            id: name.into(),
            name: name.into(),
        })
        .collect()
}

#[tauri::command]
pub async fn viewing_open_source(app: tauri::AppHandle, id: String) -> Result<(), String> {
    if !crate::commands::apps::video_sources().contains(&id.as_str()) {
        return Err("Unknown video source.".into());
    }
    if id == "YouTube" {
        return crate::commands::youtube::open_youtube(app, true, None).await;
    }
    crate::commands::apps::launch_app(id, app.state::<crate::commands::apps::Apps>(), app.clone())
        .await
}

#[tauri::command]
pub fn viewing_add_window(app: tauri::AppHandle, id: String) -> Result<Layout, String> {
    let choice = windows::list()?
        .into_iter()
        .find(|choice| choice.id == id)
        .ok_or("That window has closed. Refresh the list.")?;
    let handle = choice
        .id
        .rsplit(':')
        .next()
        .and_then(|id| id.parse().ok())
        .ok_or("Invalid window.")?;
    if windows::capture(handle)?.key() != choice.id {
        return Err("That window has changed. Refresh the list.".into());
    }
    attach_native(&app, handle, &choice.name)?;
    live(&app)
}

pub fn attach_native(app: &tauri::AppHandle, id: isize, name: &str) -> Result<(), String> {
    let mut layout = live(app)?;
    if layout.mode == Mode::Single {
        return Err("Choose a Multi Watch layout before adding a window.".into());
    }
    let window = windows::capture(id)?;
    let key = window.key();
    if !layout.players.contains(&key) {
        check_capacity(app, &key)?;
        layout.players.push(key.clone());
        layout.windows.insert(key.clone(), window.clone());
    }
    layout.names.insert(key.clone(), name.into());
    layout.selected = Some(key.clone());
    *app.state::<Viewing>()
        .0
        .lock()
        .map_err(|error| error.to_string())? = layout;
    if let Err(error) = arrange(app).and_then(|_| window.focus()) {
        let _ = viewing_remove(app.clone(), key);
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
        return Err(error);
    }
    crate::commands::session::detach_window(app, id);
    publish(app);
    Ok(())
}

pub fn restore_external(app: &tauri::AppHandle) {
    let Ok(mut layout) = live(app) else {
        return;
    };
    for window in layout.windows.values() {
        let _ = window.restore();
    }
    layout
        .players
        .retain(|label| !layout.windows.contains_key(label));
    layout.windows.clear();
    if let Ok(mut current) = app.state::<Viewing>().0.lock() {
        *current = layout;
    }
    publish(app);
}

fn live(app: &tauri::AppHandle) -> Result<Layout, String> {
    let state = app.state::<Viewing>();
    let mut layout = state.0.lock().map_err(|error| error.to_string())?;
    let previous_selection = layout.selected.clone();
    let external: Vec<String> = layout
        .windows
        .iter()
        .filter(|(_, window)| window.alive())
        .map(|(key, _)| key.clone())
        .collect();
    layout.players.retain(|label| {
        external.contains(label)
            || app
                .get_webview_window(label)
                .is_some_and(|window| window.is_visible().unwrap_or(false))
    });
    let players = layout.players.clone();
    layout.windows.retain(|label, _| players.contains(label));
    layout.names.retain(|label, _| players.contains(label));
    layout.external = external;
    if !layout
        .selected
        .as_ref()
        .is_some_and(|label| layout.players.contains(label))
    {
        layout.selected = layout.players.first().cloned();
    }
    let changed = previous_selection != layout.selected;
    let result = layout.clone();
    drop(layout);
    if changed {
        let _ = audio(app);
    }
    Ok(result)
}

#[tauri::command]
pub fn viewing_state(app: tauri::AppHandle) -> Result<Layout, String> {
    refresh_external_focus(&app);
    live(&app)
}

pub fn selected(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    let layout = live(app).ok()?;
    if layout.mode == Mode::Single {
        return None;
    }
    app.get_webview_window(layout.selected.as_deref()?)
}

fn publish(app: &tauri::AppHandle) {
    if let Ok(layout) = live(app) {
        let _ = app.emit("viewing-changed", layout);
    }
}

pub fn focus(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Viewing>();
    if let Ok(mut layout) = state.0.lock() {
        if layout.players.iter().any(|player| player == label) {
            layout.selected = Some(label.into());
        }
    }
    let _ = audio(app);
    if label == "main" {
        if let Ok(layout) = live(app) {
            if layout.mode == Mode::Floating {
                for window in layout.windows.values() {
                    window.raise();
                }
            }
        }
    }
    publish(app);
}

pub fn release(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if let Some(main) = app.get_webview_window("main") {
            let _ = floating(&window, &main, false);
        }
        let _ = window.set_decorations(true);
        let _ = mute(&window, false);
    }
    let _ = arrange(app);
    publish(app);
}

fn audio(app: &tauri::AppHandle) -> Result<(), String> {
    let layout = live(app)?;
    for label in &layout.players {
        if let Some(window) = app.get_webview_window(label) {
            mute(&window, layout.selected.as_ref() != Some(label))?;
        }
    }
    Ok(())
}

fn mute(window: &WebviewWindow, muted: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let app = window.app_handle().clone();
        window
            .with_webview(move |webview| {
                use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
                use windows_core::Interface;
                let result = unsafe {
                    webview
                        .controller()
                        .CoreWebView2()
                        .and_then(|view| view.cast::<ICoreWebView2_8>())
                        .and_then(|view| view.SetIsMuted(muted))
                };
                if let Err(error) = result {
                    let _ = app.emit(
                        "tv-setup-error",
                        format!("Player audio could not switch: {error}"),
                    );
                }
            })
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(windows))]
    window
        .eval(format!(
            "document.querySelectorAll('video,audio').forEach(media => media.muted = {muted});"
        ))
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn next(app: &tauri::AppHandle) -> Result<(), String> {
    let layout = live(app)?;
    if layout.mode == Mode::Single || layout.players.is_empty() {
        return Err("Open Multi Watch players first.".into());
    }
    let index = layout
        .selected
        .as_ref()
        .and_then(|label| layout.players.iter().position(|player| player == label))
        .unwrap_or(0);
    let label = &layout.players[(index + 1) % layout.players.len()];
    if let Some(window) = layout.windows.get(label) {
        window.focus()?;
        focus(app, label);
    } else if let Some(window) = app.get_webview_window(label) {
        window.set_focus().map_err(|error| error.to_string())?;
        focus(app, label);
    }
    Ok(())
}

fn geometry(
    mode: Mode,
    index: usize,
    count: usize,
    bounds: (i32, i32, u32, u32),
) -> (i32, i32, u32, u32) {
    let (x, y, width, height) = bounds;
    if mode == Mode::Floating {
        let available_height =
            height.saturating_sub(48 + count.saturating_sub(1) as u32 * 16) / count.max(1) as u32;
        let w = (width * 2 / 5)
            .min(available_height * 16 / 9)
            .max(1)
            .min(width);
        let h = (w * 9 / 16).min(height);
        (
            x + width.saturating_sub(w + 24) as i32,
            y + 24 + index as i32 * (h as i32 + 16),
            w,
            h,
        )
    } else if mode == Mode::Split && count > 1 {
        let left = width / 2;
        (
            x + if index == 0 { 0 } else { left as i32 },
            y,
            if index == 0 { left } else { width - left },
            height,
        )
    } else {
        bounds
    }
}

pub fn arrange(app: &tauri::AppHandle) -> Result<(), String> {
    let layout = live(app)?;
    if layout.mode == Mode::Single
        && !layout
            .selected
            .as_ref()
            .is_some_and(|label| layout.windows.contains_key(label))
    {
        return Ok(());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Open Luma to arrange players.")?;
    let monitors = app
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let current = main.current_monitor().map_err(|error| error.to_string())?;
    let primary = current
        .as_ref()
        .or_else(|| monitors.first())
        .ok_or("No display is available.")?;
    let mut displays = vec![primary.clone()];
    displays.extend(
        monitors
            .iter()
            .filter(|monitor| monitor.position() != primary.position())
            .cloned(),
    );
    if layout.mode == Mode::Displays && displays.len() < 2 {
        return Err("Connect two displays or choose Side by side.".into());
    }
    for (index, label) in layout.players.iter().enumerate() {
        if layout.mode == Mode::Single && Some(label) != layout.selected.as_ref() {
            continue;
        }
        let display = if layout.mode == Mode::Displays {
            &displays[index % displays.len()]
        } else {
            primary
        };
        let bounds = geometry(
            layout.mode,
            index,
            layout.players.len(),
            (
                display.position().x,
                display.position().y,
                display.size().width,
                display.size().height,
            ),
        );
        if let Some(window) = layout.windows.get(label) {
            window.place(bounds, matches!(layout.mode, Mode::Floating | Mode::Single))?;
            if layout.mode == Mode::Single {
                window.maximize()?;
            }
            continue;
        }
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        window
            .set_fullscreen(false)
            .map_err(|error| error.to_string())?;
        window
            .set_min_size(Some(PhysicalSize::new(320, 180)))
            .map_err(|error| error.to_string())?;
        window
            .set_decorations(layout.mode == Mode::Floating)
            .map_err(|error| error.to_string())?;
        floating(&window, &main, layout.mode == Mode::Floating)?;
        window
            .set_position(PhysicalPosition::new(bounds.0, bounds.1))
            .map_err(|error| error.to_string())?;
        window
            .set_size(PhysicalSize::new(bounds.2, bounds.3))
            .map_err(|error| error.to_string())?;
        if layout.mode == Mode::Displays {
            window
                .set_fullscreen(true)
                .map_err(|error| error.to_string())?;
        }
    }
    audio(app)
}

#[tauri::command]
pub fn viewing_configure(
    app: tauri::AppHandle,
    mode: Mode,
    selected: Option<String>,
) -> Result<Layout, String> {
    if mode == Mode::Displays
        && app
            .available_monitors()
            .map_err(|error| error.to_string())?
            .len()
            < 2
    {
        return Err("Connect two displays or choose Side by side.".into());
    }
    let mut layout = live(&app)?;
    let previous = layout.clone();
    let changing_selection = selected.is_some();
    if let Some(label) = selected {
        if !layout.players.contains(&label) {
            return Err("That player has closed.".into());
        }
        layout.selected = Some(label);
    }
    if layout.players.is_empty() {
        layout.players = ["youtube", "stremio", "sports"]
            .into_iter()
            .filter(|label| {
                app.get_webview_window(label)
                    .is_some_and(|window| window.is_visible().unwrap_or(false))
            })
            .map(str::to_string)
            .take(2)
            .collect();
        layout.selected = layout.players.first().cloned();
    }
    layout.mode = mode;
    *app.state::<Viewing>()
        .0
        .lock()
        .map_err(|error| error.to_string())? = layout.clone();
    if mode == Mode::Single {
        for label in &layout.players {
            if let Some(window) = layout.windows.get(label) {
                if Some(label) != layout.selected.as_ref() {
                    window.restore()?;
                }
                continue;
            }
            if let Some(window) = app.get_webview_window(label) {
                if let Some(main) = app.get_webview_window("main") {
                    floating(&window, &main, false)?;
                }
                window
                    .set_decorations(true)
                    .map_err(|error| error.to_string())?;
                mute(&window, false)?;
                if Some(label) != layout.selected.as_ref() {
                    crate::background::dismiss(&app, label);
                } else {
                    crate::tv::place(&window)?;
                    window
                        .set_fullscreen(true)
                        .map_err(|error| error.to_string())?;
                    window.set_focus().map_err(|error| error.to_string())?;
                }
            }
        }
        if external_selected(&app) {
            arrange(&app)?;
        }
    } else {
        if let Err(error) = arrange(&app) {
            *app.state::<Viewing>()
                .0
                .lock()
                .map_err(|cause| cause.to_string())? = previous.clone();
            if previous.mode == Mode::Single {
                for window in previous.windows.values() {
                    let _ = window.restore();
                }
                for label in &previous.players {
                    if let Some(window) = app.get_webview_window(label) {
                        let _ = window.set_fullscreen(true);
                    }
                }
            } else {
                let _ = arrange(&app);
            }
            publish(&app);
            return Err(error);
        }
        if changing_selection {
            if let Some(label) = layout
                .selected
                .as_ref()
                .and_then(|label| layout.windows.get(label))
            {
                label.focus()?;
            }
            if let Some(window) = self::selected(&app) {
                window.set_focus().map_err(|error| error.to_string())?;
            }
        }
    }
    publish(&app);
    live(&app)
}

pub fn present(window: &WebviewWindow) -> Result<bool, String> {
    let app = window.app_handle();
    let mut layout = live(app)?;
    if layout.mode == Mode::Single {
        if window.label() != "main" {
            restore_external(app);
        }
        return Ok(false);
    }
    if !["main", "youtube", "stremio", "sports"].contains(&window.label()) {
        return Ok(false);
    }
    if window.label() != "main" {
        if !layout.players.iter().any(|label| label == window.label()) {
            if !layout.can_add(window.label()) {
                crate::background::dismiss(app, window.label());
                return Err("Two players are already open. Remove one in Multi Watch before opening another service.".into());
            }
            layout.players.push(window.label().into());
        }
        layout.selected = Some(window.label().into());
    }
    window.show().map_err(|error| error.to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    *app.state::<Viewing>()
        .0
        .lock()
        .map_err(|error| error.to_string())? = layout.clone();
    if window.label() != "main" {
        arrange(app)?;
    }

    window.set_focus().map_err(|error| error.to_string())?;
    crate::background::publish_visibility(window, true);
    publish(app);
    Ok(true)
}

fn floating(window: &WebviewWindow, main: &WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_always_on_top(false)
        .map_err(|error| error.to_string())?;
    #[cfg(windows)]
    {
        use ::windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_HWNDPARENT};
        let handle = window.hwnd().map_err(|error| error.to_string())?;
        let owner = if enabled {
            main.hwnd().map_err(|error| error.to_string())?.0 as isize
        } else {
            0
        };
        unsafe {
            SetWindowLongPtrW(
                ::windows::Win32::Foundation::HWND(handle.0),
                GWLP_HWNDPARENT,
                owner,
            );
        }
    }
    #[cfg(not(windows))]
    {
        let _ = main;
        window
            .set_always_on_top(enabled)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn viewing_remove(app: tauri::AppHandle, label: String) -> Result<Layout, String> {
    let mut layout = live(&app)?;
    if !layout.players.contains(&label) {
        return Err("That player has closed.".into());
    }
    if let Some(window) = layout.windows.remove(&label) {
        window.restore()?;
        layout.players.retain(|player| player != &label);
        *app.state::<Viewing>()
            .0
            .lock()
            .map_err(|error| error.to_string())? = layout;
        arrange(&app)?;
        publish(&app);
        return live(&app);
    }
    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_always_on_top(false)
            .map_err(|error| error.to_string())?;
        window
            .set_decorations(true)
            .map_err(|error| error.to_string())?;
        mute(&window, false)?;
    }
    crate::background::dismiss(&app, &label);
    arrange(&app)?;
    publish(&app);
    live(&app)
}

pub fn browsing(app: &tauri::AppHandle) -> bool {
    active_selection(app).is_some()
        && app
            .get_webview_window("main")
            .is_some_and(|main| main.is_focused().unwrap_or(false))
}

pub fn return_home(app: &tauri::AppHandle) -> bool {
    if !enabled(app) && !external_selected(app) {
        return false;
    }
    crate::background::show(app);
    let _ = app.emit_to("main", "remote-navigation", "home");
    true
}

pub fn navigate(app: &tauri::AppHandle, command: &str) -> bool {
    let Some(label) = active_selection(app) else {
        return false;
    };
    if command == "home" {
        return_home(app);
    } else if app
        .get_webview_window("main")
        .is_some_and(|main| main.is_focused().unwrap_or(false))
    {
        return false;
    } else if let Some(window) = live(app)
        .ok()
        .and_then(|layout| layout.windows.get(&label).cloned())
    {
        if !window.alive() {
            return false;
        }
        if let Some(name) = crate::commands::services::window_source(app, window.id) {
            crate::commands::services::forward(app, &name, command);
        } else if let Err(error) = crate::commands::session::press_window(window.id, command) {
            let _ = app.emit("viewing-error", error);
        }
    } else {
        match label.as_str() {
            "sports" => crate::commands::sports::forward(app, command),
            "stremio" => crate::commands::stremio::forward(app, command),
            _ => crate::commands::youtube::forward(app, command),
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_sources_share_capacity_and_can_reselect_existing_players() {
        let mut layout = Layout {
            mode: Mode::Split,
            players: vec!["youtube".into(), "window:100:200".into()],
            ..Default::default()
        };
        layout
            .names
            .insert("window:100:200".into(), "Netflix".into());
        assert!(layout.can_add("youtube"));
        assert!(layout.can_add("window:100:200"));
        assert!(layout.can_add("Netflix"));
        assert!(!layout.can_add("stremio"));
        assert!(!layout.can_add("window:101:300"));
        layout.players.pop();
        assert!(layout.can_add("stremio"));
    }

    #[test]
    fn floating_players_fit_a_wide_display() {
        for index in 0..2 {
            let (x, y, width, height) = geometry(Mode::Floating, index, 2, (-3840, 0, 3840, 1080));
            assert!(x >= -3840 && y >= 0);
            assert!(x + width as i32 <= 0 && y + height as i32 <= 1080);
        }
    }

    #[test]
    fn split_covers_odd_width_and_negative_monitor_origin() {
        assert_eq!(
            geometry(Mode::Split, 0, 2, (-1921, -200, 1921, 1080)),
            (-1921, -200, 960, 1080)
        );
        assert_eq!(
            geometry(Mode::Split, 1, 2, (-1921, -200, 1921, 1080)),
            (-961, -200, 961, 1080)
        );
    }
}
