use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const STEP_UNITS: i32 = 12;
const STEP_MINIMUM: i32 = 24;

#[derive(Default)]
pub struct Session(Mutex<Option<Launched>>);

pub(crate) fn place_current(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Session>();
    let current = state.0.lock().map_err(|error| error.to_string())?;
    if let Some(window) = current.as_ref().and_then(locate) {
        if let Some(bounds) = crate::tv::bounds(app) {
            platform::place(window, bounds);
        }
    }
    Ok(())
}

struct Launched {
    name: String,
    pid: u32,
    web: bool,
    keys: bool,
    title: Option<&'static str>,
}

const STEAM_TITLE: &str = "Steam Big Picture Mode";

pub fn track(app: &AppHandle, name: &str, pid: u32) {
    track_with(app, name, pid, false, false, None);
}

pub(crate) fn track_web(app: &AppHandle, name: &str, pid: u32) {
    track_with(app, name, pid, true, false, None);
}

pub(crate) fn track_steam(app: &AppHandle) {
    track_with(app, "Steam", 0, false, true, Some(STEAM_TITLE));
}

fn track_with(
    app: &AppHandle,
    name: &str,
    pid: u32,
    web: bool,
    keys: bool,
    title: Option<&'static str>,
) {
    if !crate::viewing::enabled(app) { crate::viewing::restore_external(app); }
    if let Ok(mut current) = app.state::<Session>().0.lock() {
        *current = Some(Launched {
            name: name.into(),
            pid,
            web,
            keys,
            title,
        });
    }
    let bounds = crate::tv::bounds(app);
    if bounds.is_some() || crate::viewing::enabled(app) {
        let handle = app.clone();
        let name = name.to_owned();
        let attempts = if title.is_some() { 150 } else { 50 };
        tauri::async_runtime::spawn(async move {
            for _ in 0..attempts {
                let current = handle
                    .state::<Session>()
                    .0
                    .lock()
                    .ok()
                    .is_some_and(|session| {
                        session
                            .as_ref()
                            .is_some_and(|session| session.pid == pid && session.name == name)
                    });
                if !current && !crate::viewing::enabled(&handle) {
                    return;
                }
                if let Some(window) = locate_window(pid, &name, title) {
                    if crate::viewing::enabled(&handle) {
                        if let Err(error) = crate::viewing::attach_native(&handle, platform::id(window), &name) {
                            use tauri::Emitter;
                            let _ = handle.emit("viewing-error", error);
                        }
                    } else if let Some(bounds) = bounds { platform::place(window, bounds); }
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            if crate::viewing::enabled(&handle) {
                use tauri::Emitter;
                let _ = handle.emit("viewing-error", format!("{name} did not expose a window for Multi Watch. Open it in windowed mode, then use Add open window."));
            }
        });
    }
}

fn locate_window(pid: u32, name: &str, title: Option<&str>) -> Option<platform::Window> {
    match title {
        Some(title) => platform::exact(title),
        None => platform::find(pid, name),
    }
}

fn locate(launched: &Launched) -> Option<platform::Window> {
    locate_window(launched.pid, &launched.name, launched.title)
}

pub(crate) fn focus_steam(app: &AppHandle) -> bool {
    let Some(window) = platform::exact(STEAM_TITLE) else {
        return false;
    };
    if let Some(bounds) = crate::tv::bounds(app) {
        platform::place(window, bounds);
    }
    platform::restore(window);
    let pid = platform::pid(window).unwrap_or(0);
    track_with(app, "Steam", pid, false, true, Some(STEAM_TITLE));
    true
}

pub(crate) fn close_steam(app: &AppHandle) {
    let state = app.state::<Session>();
    let Ok(mut current) = state.0.lock() else {
        return;
    };
    if !current
        .as_ref()
        .is_some_and(|launched| launched.name == "Steam")
    {
        return;
    }
    let launched = current.take();
    drop(current);
    end_steam(launched.and_then(|launched| locate(&launched)));
}

pub(crate) fn active(app: &AppHandle) -> bool {
    app.state::<Session>()
        .0
        .lock()
        .map(|current| {
            current.as_ref().is_some_and(|launched| {
                locate(launched).is_some()
                    || (!launched.web
                        && launched.title.is_none()
                        && launched.pid != 0
                        && platform::running(launched.pid))
            })
        })
        .unwrap_or(false)
}

pub fn reuse(app: &AppHandle, name: &str) -> bool {
    let state = app.state::<Session>();
    let Ok(current) = state.0.lock() else {
        return false;
    };
    let session = current
        .as_ref()
        .filter(|launched| launched.name == name)
        .map(|launched| (launched.pid, launched.web, launched.keys, launched.title));
    drop(current);
    let found = match session {
        Some((_, _, _, Some(title))) => platform::exact(title),
        Some((pid, _, _, None)) if pid != 0 => platform::find(pid, name),
        _ => platform::titled(name),
    };
    let Some(window) = found else {
        return false;
    };
    if crate::viewing::enabled(app) {
        if let Err(error) = crate::viewing::attach_native(app, platform::id(window), name) {
            use tauri::Emitter;
            let _ = app.emit("viewing-error", error);
        }
    } else if let Some(bounds) = crate::tv::bounds(app) { platform::place(window, bounds); }
    platform::restore(window);
    if session.is_none() || session.is_some_and(|(pid, _, _, _)| pid == 0) {
        if let Some(pid) = platform::pid(window) {
            let (web, keys, title) = session
                .map(|(_, web, keys, title)| (web, keys, title))
                .unwrap_or((false, false, None));
            track_with(app, name, pid, web, keys, title);
        }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Pass,
    Forward,
    Home,
    Release,
    Wait,
}

fn action(luma_foreground: bool, found: bool, running: bool, command: &str, web: bool) -> Action {
    if command == "home" {
        return Action::Home;
    }
    if luma_foreground {
        return Action::Pass;
    }
    if found {
        return Action::Forward;
    }
    if !web && running {
        Action::Wait
    } else {
        Action::Release
    }
}

#[tauri::command]
pub fn close_launched_app(app: AppHandle) {
    if crate::viewing::enabled(&app) || crate::viewing::external_selected(&app) { return; }
    let state = app.state::<Session>();
    let Ok(mut current) = state.0.lock() else {
        return;
    };
    let Some(launched) = current.take() else {
        return;
    };
    if launched.name == "Steam" {
        drop(current);
        home_steam(&app, locate(&launched));
        return;
    }
    if let Some(window) = platform::find(launched.pid, &launched.name) {
        platform::close(window);
    }
    if launched.web {
        crate::commands::services::forget(&app, &launched.name);
    }
    restore(&app);
}

pub(crate) fn fullscreen(app: &AppHandle) -> Option<Result<(), String>> {
    let state = app.state::<Session>();
    let (name, pid, web, keys, title) = {
        let current = match state.0.lock() {
            Ok(current) => current,
            Err(_) => return Some(Err("The remote could not read the app session.".into())),
        };
        let launched = current.as_ref()?;
        (
            launched.name.clone(),
            launched.pid,
            launched.web,
            launched.keys,
            launched.title,
        )
    };
    if platform::foreground_is_luma() {
        return None;
    }
    let window = if title.is_some() {
        locate_window(pid, &name, title)
    } else {
        platform::owned(pid)
    };
    let Some(window) = window else {
        return Some(if title.is_none() && pid != 0 && platform::running(pid) {
            Err(format!("{name} is still starting. Try again in a moment."))
        } else {
            Err(format!("{name} is not showing a window."))
        });
    };
    if !platform::foreground(window) {
        return Some(Err(format!("Bring {name} to the front on the PC first.")));
    }
    if keys {
        return Some(Ok(()));
    }
    if !web && !platform::browser(window) {
        return Some(Err(format!(
            "Full screen is not available for {name} from the remote. Use its own fullscreen control."
        )));
    }
    platform::fullscreen(window);
    Some(Ok(()))
}

fn browser_executable(path: &str) -> bool {
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "msedge.exe"
            | "chrome.exe"
            | "firefox.exe"
            | "brave.exe"
            | "opera.exe"
            | "opera_gx.exe"
            | "vivaldi.exe"
    )
}

fn steam_window() -> Option<platform::Window> {
    platform::exact(STEAM_TITLE).or_else(|| platform::exact("Steam"))
}

fn end_steam(window: Option<platform::Window>) {
    if window.is_none() && steam_window().is_none() {
        return;
    }
    crate::commands::steam::close();
    if let Some(window) = window {
        platform::minimize(window);
    }
    tauri::async_runtime::spawn(async move {
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if let Some(window) = steam_window() {
                platform::minimize(window);
            }
        }
    });
}

fn home_steam(app: &AppHandle, window: Option<platform::Window>) {
    end_steam(window);
    if platform::foreground_is_luma() {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        crate::background::show(&handle);
    });
}

pub fn route(app: &AppHandle, command: &str) -> bool {
    let state = app.state::<Session>();
    let Ok(mut current) = state.0.lock() else {
        return false;
    };
    let Some(launched) = current.as_ref() else {
        return false;
    };
    let (name, pid, web, keys, title) = (
        launched.name.clone(),
        launched.pid,
        launched.web,
        launched.keys,
        launched.title,
    );
    let luma_foreground = platform::foreground_is_luma();
    let window = if luma_foreground && command != "home" {
        None
    } else {
        locate_window(pid, &name, title)
    };
    match action(
        luma_foreground,
        window.is_some(),
        !luma_foreground && title.is_none() && pid != 0 && platform::running(pid),
        command,
        web,
    ) {
        Action::Pass => false,
        Action::Wait => true,
        Action::Forward => {
            if web {
                if let Some(window) = window {
                    platform::restore(window);
                }
                drop(current);
                crate::commands::services::forward(app, &name, command);
                return true;
            }
            let Some(window) = window else {
                return false;
            };
            if keys {
                platform::restore(window);
                match command {
                    "up" | "down" | "left" | "right" | "activate" => {
                        platform::press(window, command)
                    }
                    "back" => platform::escape(window),
                    _ => return false,
                }
            } else {
                match command {
                    "up" => platform::step(window, 0, -1),
                    "down" => platform::step(window, 0, 1),
                    "left" => platform::step(window, -1, 0),
                    "right" => platform::step(window, 1, 0),
                    "activate" => platform::click(window),
                    "back" => platform::escape(window),
                    _ => return false,
                }
            }
            true
        }
        Action::Home => {
            if name == "Steam" {
                home_steam(app, window);
                *current = None;
                drop(current);
                return true;
            }
            if let Some(window) = window {
                platform::close(window);
            }
            if web {
                crate::commands::services::forget(app, &name);
            }
            *current = None;
            drop(current);
            restore(app);
            true
        }
        Action::Release => {
            if web {
                crate::commands::services::forget(app, &name);
            }
            *current = None;
            drop(current);
            restore(app);
            false
        }
    }
}

pub(crate) fn release(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Session>();
    let launched = state.0.lock().map_err(|error| error.to_string())?.take();
    if let Some(launched) = launched
        .as_ref()
        .filter(|launched| launched.name == "Steam")
    {
        end_steam(locate(launched));
        return Ok(());
    }
    if let Some(launched) = launched.as_ref().filter(|launched| launched.web) {
        crate::commands::services::forget(app, &launched.name);
    }
    if !platform::foreground_is_luma() {
        if let Some(launched) = launched {
            if let Some(window) = locate(&launched) {
                platform::minimize(window);
            }
        }
    }
    Ok(())
}

fn restore(app: &AppHandle) {
    crate::background::show(app);
}

#[cfg(windows)]
pub(crate) fn native_executable(app: &AppHandle) -> Option<String> {
    let state = app.state::<Session>();
    let current = state.0.lock().ok()?;
    let launched = current
        .as_ref()
        .filter(|launched| !launched.web && !launched.keys)?;
    let window = locate(launched)?;
    if !platform::foreground(window) {
        return None;
    }
    platform::executable(window)
}

pub(crate) fn web_current(app: &AppHandle, name: &str) -> bool {
    if crate::viewing::enabled(app) || crate::viewing::external_selected(app) {
        return !crate::viewing::browsing(app) && crate::viewing::external_source(app).as_deref() == Some(name);
    }
    app.state::<Session>().0.lock().ok().is_some_and(|current| {
        current
            .as_ref()
            .is_some_and(|launched| launched.web && launched.name == name)
    }) && !platform::foreground_is_luma()
}

pub(crate) fn finish(app: &AppHandle, name: &str) {
    let state = app.state::<Session>();
    let Ok(mut current) = state.0.lock() else {
        return;
    };
    if !current
        .as_ref()
        .is_some_and(|launched| launched.web && launched.name == name)
        || platform::foreground_is_luma()
    {
        return;
    }
    let launched = current.take();
    drop(current);
    if let Some(launched) = launched {
        crate::commands::services::forget(app, &launched.name);
        if let Some(window) = platform::find(launched.pid, &launched.name) {
            platform::minimize(window);
        }
    }
    restore(app);
}

pub(crate) fn press(app: &AppHandle, name: &str, command: &str) {
    if !web_current(app, name) {
        return;
    }
    let Some(window) = platform::titled(name) else {
        return;
    };
    platform::restore(window);
    platform::press(window, command);
}

pub(crate) fn window_alive(id: isize) -> bool {
    platform::alive(id)
}

pub(crate) fn window_browser(id: isize) -> bool {
    platform::from_id(id).is_some_and(platform::browser)
}

pub(crate) fn window_pid(id: isize) -> Option<u32> {
    platform::pid_of(id)
}

pub(crate) fn find_window(name: &str) -> Option<isize> {
    platform::titled(name).map(platform::id)
}

pub(crate) fn find_window_for(pid: u32, name: &str) -> Option<isize> {
    platform::find(pid, name).map(platform::id)
}

pub(crate) fn restore_window(id: isize) {
    if let Some(window) = platform::from_id(id) {
        platform::restore(window);
    }
}

pub(crate) fn detach_window(app: &AppHandle, id: isize) {
    if let Ok(mut current) = app.state::<Session>().0.lock() {
        if current.as_ref().and_then(locate).is_some_and(|window| platform::id(window) == id) {
            *current = None;
        }
    }
}

pub(crate) fn focus_window(id: isize) {
    if window_alive(id) {
        if let Some(window) = platform::from_id(id) { platform::focus(window); }
    }
}

pub(crate) fn press_window(id: isize, command: &str) -> Result<(), String> {
    if !window_alive(id) { return Err("That window has closed.".into()); }
    let window = platform::from_id(id).ok_or("That window has closed.")?;
    if command == "back" && !window_browser(id) { platform::escape(window); }
    else { platform::press(window, command); }
    Ok(())
}

fn stepped(cursor: (i32, i32), rect: (i32, i32, i32, i32), dx: i32, dy: i32) -> (i32, i32) {
    let step = ((rect.3 - rect.1) / STEP_UNITS).max(STEP_MINIMUM);
    let x = (cursor.0 + dx * step).clamp(rect.0, (rect.2 - 1).max(rect.0));
    let y = (cursor.1 + dy * step).clamp(rect.1, (rect.3 - 1).max(rect.1));
    (x, y)
}

#[cfg(windows)]
mod platform {
    use super::{browser_executable, stepped};
    use std::{
        collections::{HashMap, HashSet},
        mem::size_of,
    };
    use windows::{
        core::{BOOL, PWSTR},
        Win32::{
            Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            System::Threading::{
                AttachThreadInput, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
            UI::{
                Input::KeyboardAndMouse::{
                    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
                    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN,
                    MOUSEEVENTF_LEFTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_DOWN,
                    VK_ESCAPE, VK_F11, VK_LEFT, VK_MENU, VK_RETURN, VK_RIGHT, VK_UP,
                },
                WindowsAndMessaging::{
                    EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowRect,
                    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
                    IsWindowVisible, PostMessageW, SetCursorPos, SetForegroundWindow, ShowWindow,
                    SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
                },
            },
        },
    };

    pub type Window = HWND;

    pub fn place(window: Window, bounds: (i32, i32, u32, u32)) {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};
        unsafe {
            let _ = ShowWindow(window, SW_RESTORE);
            let _ = SetWindowPos(
                window,
                None,
                bounds.0,
                bounds.1,
                bounds.2 as i32,
                bounds.3 as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }

    pub fn id(window: Window) -> isize {
        window.0 as isize
    }

    pub fn from_id(id: isize) -> Option<Window> {
        (id != 0).then_some(HWND(id as *mut _))
    }

    pub fn alive(id: isize) -> bool {
        from_id(id).is_some_and(|window| unsafe { IsWindow(Some(window)).as_bool() })
    }

    pub fn pid_of(id: isize) -> Option<u32> {
        from_id(id).and_then(pid)
    }

    pub fn find(pid: u32, name: &str) -> Option<Window> {
        if pid == 0 {
            return titled(name);
        }
        let owned = tree(pid);
        visible()
            .iter()
            .find(|(_, owner, _)| *owner == pid || owned.contains(owner))
            .map(|(window, _, _)| *window)
            .or_else(|| titled(name))
    }

    pub fn owned(pid: u32) -> Option<Window> {
        if pid == 0 {
            return None;
        }
        let tree = tree(pid);
        let foreground = unsafe { GetForegroundWindow() };
        let windows: Vec<_> = visible()
            .into_iter()
            .filter(|(_, owner, _)| *owner == pid || tree.contains(owner))
            .collect();
        windows
            .iter()
            .find(|(window, _, _)| *window == foreground)
            .or_else(|| windows.first())
            .map(|(window, _, _)| *window)
    }

    pub fn titled(name: &str) -> Option<Window> {
        if name.is_empty() || name == "Custom" {
            return None;
        }
        let name = name.to_lowercase();
        visible()
            .into_iter()
            .find(|(_, _, title)| title.to_lowercase().contains(&name))
            .map(|(window, _, _)| window)
    }

    pub fn exact(title: &str) -> Option<Window> {
        if title.is_empty() {
            return None;
        }
        let title = title.to_lowercase();
        visible()
            .into_iter()
            .find(|(_, _, value)| value.to_lowercase() == title)
            .map(|(window, _, _)| window)
    }

    pub fn restore(window: Window) {
        unsafe {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        focus(window);
    }

    pub fn running(pid: u32) -> bool {
        snapshot().contains_key(&pid)
    }

    pub fn foreground_is_luma() -> bool {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(foreground, Some(&mut pid));
            pid == std::process::id()
        }
    }

    pub fn pid(window: Window) -> Option<u32> {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut pid));
        }
        (pid != 0).then_some(pid)
    }

    pub fn minimize(window: Window) {
        unsafe {
            let _ = ShowWindow(window, SW_MINIMIZE);
        }
    }

    pub fn close(window: Window) {
        unsafe {
            let _ = ShowWindow(window, SW_MINIMIZE);
            let _ = PostMessageW(
                Some(window),
                WM_CLOSE,
                Default::default(),
                Default::default(),
            );
        }
    }

    pub fn step(window: Window, dx: i32, dy: i32) {
        focus(window);
        let mut cursor = POINT::default();
        let mut rect = RECT::default();
        unsafe {
            if GetCursorPos(&mut cursor).is_err() || GetWindowRect(window, &mut rect).is_err() {
                return;
            }
            let (x, y) = stepped(
                (cursor.x, cursor.y),
                (rect.left, rect.top, rect.right, rect.bottom),
                dx,
                dy,
            );
            let _ = SetCursorPos(x, y);
        }
    }

    pub fn click(window: Window) {
        step(window, 0, 0);
        let inputs = [mouse(MOUSEEVENTF_LEFTDOWN), mouse(MOUSEEVENTF_LEFTUP)];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    pub fn escape(window: Window) {
        focus(window);
        let inputs = [
            key(VK_ESCAPE, KEYBD_EVENT_FLAGS(0)),
            key(VK_ESCAPE, KEYEVENTF_KEYUP),
        ];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    pub fn press(window: Window, command: &str) {
        if command == "back" {
            back(window);
            return;
        }
        let (key_code, extended) = match command {
            "up" => (VK_UP, true),
            "down" => (VK_DOWN, true),
            "left" => (VK_LEFT, true),
            "right" => (VK_RIGHT, true),
            "activate" => (VK_RETURN, false),
            _ => return,
        };
        focus(window);
        let flags = if extended {
            KEYEVENTF_EXTENDEDKEY
        } else {
            KEYBD_EVENT_FLAGS(0)
        };
        let inputs = [key(key_code, flags), key(key_code, flags | KEYEVENTF_KEYUP)];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    fn back(window: Window) {
        focus(window);
        let inputs = [
            key(VK_MENU, KEYBD_EVENT_FLAGS(0)),
            key(VK_LEFT, KEYEVENTF_EXTENDEDKEY),
            key(VK_LEFT, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP),
            key(VK_MENU, KEYEVENTF_KEYUP),
        ];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    pub fn fullscreen(window: Window) {
        focus(window);
        let inputs = [
            key(VK_F11, KEYBD_EVENT_FLAGS(0)),
            key(VK_F11, KEYEVENTF_KEYUP),
        ];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    pub fn foreground(window: Window) -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.0.is_null() {
            return false;
        }
        foreground == window
            || matches!((pid(foreground), pid(window)), (Some(a), Some(b)) if a == b)
    }

    pub fn browser(window: Window) -> bool {
        executable(window).is_some_and(|path| browser_executable(&path))
    }

    pub(super) fn executable(window: Window) -> Option<String> {
        let pid = pid(window)?;
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };
        let mut buffer = [0u16; 512];
        let mut length = buffer.len() as u32;
        let result = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
        };
        let _ = unsafe { CloseHandle(handle) };
        result
            .ok()
            .map(|_| String::from_utf16_lossy(&buffer[..length as usize]))
    }

    pub fn focus(window: Window) {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground == window {
                return;
            }
            let from = GetWindowThreadProcessId(foreground, None);
            let to = GetWindowThreadProcessId(window, None);
            let attached =
                from != 0 && to != 0 && from != to && AttachThreadInput(from, to, true).as_bool();
            let _ = SetForegroundWindow(window);
            if attached {
                let _ = AttachThreadInput(from, to, false);
            }
        }
    }

    fn mouse(flag: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn key(vk: VIRTUAL_KEY, flag: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn visible() -> Vec<(HWND, u32, String)> {
        unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> BOOL {
            let list = unsafe { &mut *(data.0 as *mut Vec<(HWND, u32, String)>) };
            unsafe {
                if !IsWindowVisible(window).as_bool() {
                    return BOOL(1);
                }
                let mut pid = 0u32;
                GetWindowThreadProcessId(window, Some(&mut pid));
                let length = GetWindowTextLengthW(window);
                if length > 0 {
                    let mut buffer = vec![0u16; length as usize + 1];
                    let written = GetWindowTextW(window, &mut buffer);
                    list.push((
                        window,
                        pid,
                        String::from_utf16_lossy(&buffer[..written as usize]),
                    ));
                }
            }
            BOOL(1)
        }
        let mut list = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut list as *mut _ as isize));
        }
        list
    }

    fn snapshot() -> HashMap<u32, u32> {
        let mut parents = HashMap::new();
        unsafe {
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return parents;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        parents
    }

    fn tree(pid: u32) -> HashSet<u32> {
        let parents = snapshot();
        parents
            .keys()
            .copied()
            .filter(|candidate| {
                let mut current = *candidate;
                for _ in 0..32 {
                    match parents.get(&current) {
                        Some(&parent) if parent != 0 && parent != current => {
                            if parent == pid {
                                return true;
                            }
                            current = parent;
                        }
                        _ => return false,
                    }
                }
                false
            })
            .collect()
    }
}

#[cfg(not(windows))]
mod platform {
    pub type Window = ();
    pub fn focus(_: Window) {}
    pub fn place(_: Window, _: (i32, i32, u32, u32)) {}

    pub fn id(_: Window) -> isize {
        0
    }
    pub fn from_id(_: isize) -> Option<Window> {
        None
    }
    pub fn alive(_: isize) -> bool {
        false
    }
    pub fn pid_of(_: isize) -> Option<u32> {
        None
    }
    pub fn find(_: u32, _: &str) -> Option<Window> {
        None
    }
    pub fn owned(_: u32) -> Option<Window> {
        None
    }
    pub fn running(_: u32) -> bool {
        false
    }
    pub fn foreground_is_luma() -> bool {
        false
    }
    pub fn pid(_: Window) -> Option<u32> {
        None
    }
    pub fn minimize(_: Window) {}
    pub fn close(_: Window) {}
    pub fn titled(_: &str) -> Option<Window> {
        None
    }
    pub fn exact(_: &str) -> Option<Window> {
        None
    }
    pub fn restore(_: Window) {}
    pub fn step(_: Window, _: i32, _: i32) {}
    pub fn click(_: Window) {}
    pub fn escape(_: Window) {}
    pub fn press(_: Window, _: &str) {}
    pub fn fullscreen(_: Window) {}
    pub fn foreground(_: Window) -> bool {
        false
    }
    pub fn browser(_: Window) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{action, browser_executable, stepped, Action};

    #[test]
    fn remote_input_stays_in_luma_once_it_is_foreground() {
        assert_eq!(action(true, true, true, "left", false), Action::Pass);
        assert_eq!(action(true, true, true, "activate", false), Action::Pass);
        assert_eq!(action(true, false, true, "left", false), Action::Pass);
        assert_eq!(action(false, true, true, "left", false), Action::Forward);
        assert_eq!(action(false, true, true, "left", true), Action::Forward);
    }

    #[test]
    fn home_releases_the_app_even_after_luma_has_regained_focus() {
        assert_eq!(action(false, true, true, "home", false), Action::Home);
        assert_eq!(action(false, true, true, "home", true), Action::Home);
        assert_eq!(action(true, true, true, "home", false), Action::Home);
        assert_eq!(action(true, true, true, "home", true), Action::Home);
    }

    #[test]
    fn a_closed_app_releases_the_session() {
        assert_eq!(action(false, false, false, "left", false), Action::Release);
        assert_eq!(action(false, false, true, "left", false), Action::Wait);
        assert_eq!(action(false, false, true, "left", true), Action::Release);
    }

    #[test]
    fn only_browsers_receive_the_fullscreen_key() {
        assert!(browser_executable(
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"
        ));
        assert!(browser_executable(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe"
        ));
        assert!(browser_executable("/usr/bin/firefox.exe"));
        assert!(!browser_executable(
            r"C:\Program Files\VideoLAN\VLC\vlc.exe"
        ));
        assert!(!browser_executable(""));
    }

    #[test]
    fn keeps_the_pointer_inside_the_app_window() {
        let rect = (100, 200, 300, 400);
        assert_eq!(stepped((150, 250), rect, 1, 0), (174, 250));
        assert_eq!(stepped((299, 250), rect, 1, 0), (299, 250));
        assert_eq!(stepped((150, 399), rect, 0, 1), (150, 399));
        assert_eq!(stepped((0, 0), rect, -1, -1), (100, 200));
        assert_eq!(stepped((150, 250), (100, 200, 100, 200), 1, 1), (100, 200));
        assert_eq!(
            stepped((150, 2500), (100, 200, 300, 4299), 0, 1),
            (150, 2841)
        );
    }
}
