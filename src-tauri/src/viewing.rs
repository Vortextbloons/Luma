use serde::{Deserialize, Serialize};
use std::sync::Mutex;
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
}

#[derive(Default)]
pub struct Viewing(Mutex<Layout>);

fn live(app: &tauri::AppHandle) -> Result<Layout, String> {
    let state = app.state::<Viewing>();
    let mut layout = state.0.lock().map_err(|error| error.to_string())?;
    layout.players.retain(|label| {
        app.get_webview_window(label)
            .is_some_and(|window| window.is_visible().unwrap_or(false))
    });
    if !layout
        .selected
        .as_ref()
        .is_some_and(|label| layout.players.contains(label))
    {
        layout.selected = layout.players.first().cloned();
    }
    Ok(layout.clone())
}

#[tauri::command]
pub fn viewing_state(app: tauri::AppHandle) -> Result<Layout, String> {
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
    if let Some(window) = app.get_webview_window(label) {
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
    if layout.mode == Mode::Single {
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
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
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
    } else {
        arrange(&app)?;
        if changing_selection {
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
        return Ok(false);
    }
    if !["main", "youtube", "stremio", "sports"].contains(&window.label()) {
        return Ok(false);
    }
    if window.label() != "main" {
        if !layout.players.iter().any(|label| label == window.label()) {
            if layout.players.len() == 2 {
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
        .map_err(|error| error.to_string())? = layout;
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
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_HWNDPARENT};
        let handle = window.hwnd().map_err(|error| error.to_string())?;
        let owner = if enabled {
            main.hwnd().map_err(|error| error.to_string())?.0 as isize
        } else {
            0
        };
        unsafe {
            SetWindowLongPtrW(
                windows::Win32::Foundation::HWND(handle.0),
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
    if !["youtube", "stremio", "sports"].contains(&label.as_str()) {
        return Err("Unknown player.".into());
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
    selected(app).is_some()
        && app
            .get_webview_window("main")
            .is_some_and(|main| main.is_focused().unwrap_or(false))
}

pub fn return_home(app: &tauri::AppHandle) -> bool {
    if !live(app).is_ok_and(|layout| layout.mode != Mode::Single) {
        return false;
    }
    crate::background::show(app);
    let _ = app.emit_to("main", "remote-navigation", "home");
    true
}

pub fn navigate(app: &tauri::AppHandle, command: &str) -> bool {
    let Some(window) = selected(app) else {
        return false;
    };
    if command == "home" {
        return_home(app);
    } else if app
        .get_webview_window("main")
        .is_some_and(|main| main.is_focused().unwrap_or(false))
    {
        return false;
    } else {
        match window.label() {
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
