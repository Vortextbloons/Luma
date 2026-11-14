use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const STEP_UNITS: i32 = 12;
const STEP_MINIMUM: i32 = 24;

#[derive(Default)]
pub struct Session(Mutex<Option<Launched>>);

struct Launched {
    name: String,
    pid: u32,
}

pub fn track(app: &AppHandle, name: &str, pid: u32) {
    if let Ok(mut current) = app.state::<Session>().0.lock() {
        *current = Some(Launched {
            name: name.into(),
            pid,
        });
    }
}

pub fn reuse(app: &AppHandle, name: &str) -> bool {
    let state = app.state::<Session>();
    let Ok(current) = state.0.lock() else {
        return false;
    };
    let pid = current
        .as_ref()
        .filter(|launched| launched.name == name)
        .map(|launched| launched.pid);
    drop(current);
    let found = match pid {
        Some(pid) => platform::find(pid, name),
        None => platform::titled(name),
    };
    let Some(window) = found else {
        return false;
    };
    platform::restore(window);
    if pid.is_none() {
        if let Some(pid) = platform::pid(window) {
            track(app, name, pid);
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
}

fn action(luma_foreground: bool, found: bool, running: bool, command: &str) -> Action {
    if luma_foreground {
        return Action::Pass;
    }
    if command == "home" {
        return Action::Home;
    }
    if found {
        return Action::Forward;
    }
    if running {
        Action::Pass
    } else {
        Action::Release
    }
}

pub fn route(app: &AppHandle, command: &str) -> bool {
    let state = app.state::<Session>();
    let Ok(mut current) = state.0.lock() else {
        return false;
    };
    let Some(launched) = current.as_ref() else {
        return false;
    };
    let (name, pid) = (launched.name.clone(), launched.pid);
    let luma_foreground = platform::foreground_is_luma();
    let window = if luma_foreground {
        None
    } else {
        platform::find(pid, &name)
    };
    match action(
        luma_foreground,
        window.is_some(),
        !luma_foreground && platform::running(pid),
        command,
    ) {
        Action::Pass => false,
        Action::Forward => {
            let Some(window) = window else {
                return false;
            };
            match command {
                "up" => platform::step(window, 0, -1),
                "down" => platform::step(window, 0, 1),
                "left" => platform::step(window, -1, 0),
                "right" => platform::step(window, 1, 0),
                "activate" => platform::click(window),
                "back" => platform::escape(window),
                _ => return false,
            }
            true
        }
        Action::Home => {
            if let Some(window) = window {
                platform::minimize(window);
            }
            *current = None;
            drop(current);
            restore(app);
            true
        }
        Action::Release => {
            *current = None;
            drop(current);
            restore(app);
            false
        }
    }
}

fn restore(app: &AppHandle) {
    crate::background::show(app);
}

fn stepped(cursor: (i32, i32), rect: (i32, i32, i32, i32), dx: i32, dy: i32) -> (i32, i32) {
    let step = ((rect.3 - rect.1) / STEP_UNITS).max(STEP_MINIMUM);
    let x = (cursor.0 + dx * step).clamp(rect.0, (rect.2 - 1).max(rect.0));
    let y = (cursor.1 + dy * step).clamp(rect.1, (rect.3 - 1).max(rect.1));
    (x, y)
}

#[cfg(windows)]
mod platform {
    use super::stepped;
    use std::{
        collections::{HashMap, HashSet},
        mem::size_of,
    };
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            System::Threading::AttachThreadInput,
            UI::{
                Input::KeyboardAndMouse::{
                    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
                    KEYEVENTF_KEYUP, MOUSEINPUT, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
                    MOUSE_EVENT_FLAGS, VK_ESCAPE,
                },
                WindowsAndMessaging::{
                    EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowRect,
                    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
                    SetCursorPos, SetForegroundWindow, ShowWindow, SW_MINIMIZE, SW_RESTORE,
                },
            },
        },
    };

    pub type Window = HWND;

    pub fn find(pid: u32, name: &str) -> Option<Window> {
        let owned = tree(pid);
        visible()
            .iter()
            .find(|(_, owner, _)| *owner == pid || owned.contains(owner))
            .map(|(window, _, _)| *window)
            .or_else(|| titled(name))
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
        let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
        unsafe {
            SendInput(&inputs, size_of::<INPUT>() as i32);
        }
    }

    fn focus(window: Window) {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground == window {
                return;
            }
            let from = GetWindowThreadProcessId(foreground, None);
            let to = GetWindowThreadProcessId(window, None);
            let attached = from != 0 && to != 0 && from != to && AttachThreadInput(from, to, true).as_bool();
            let _ = SetForegroundWindow(window);
            if attached {
                let _ = AttachThreadInput(from, to, false);
            }        }
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

    fn key(flag: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_ESCAPE,
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

    pub fn find(_: u32, _: &str) -> Option<Window> {
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
    pub fn titled(_: &str) -> Option<Window> {
        None
    }
    pub fn restore(_: Window) {}
    pub fn step(_: Window, _: i32, _: i32) {}
    pub fn click(_: Window) {}
    pub fn escape(_: Window) {}
}

#[cfg(test)]
mod tests {
    use super::{action, stepped, Action};

    #[test]
    fn remote_input_stays_in_luma_once_it_is_foreground() {
        assert_eq!(action(true, true, true, "left"), Action::Pass);
        assert_eq!(action(true, true, true, "activate"), Action::Pass);
        assert_eq!(action(true, false, true, "left"), Action::Pass);
        assert_eq!(action(false, true, true, "left"), Action::Forward);
    }

    #[test]
    fn home_releases_the_app_only_while_it_owns_the_screen() {
        assert_eq!(action(false, true, true, "home"), Action::Home);
        assert_eq!(action(true, true, true, "home"), Action::Pass);
    }

    #[test]
    fn a_closed_app_releases_the_session() {
        assert_eq!(action(false, false, false, "left"), Action::Release);
        assert_eq!(action(false, false, true, "left"), Action::Pass);
    }

    #[test]
    fn keeps_the_pointer_inside_the_app_window() {
        let rect = (100, 200, 300, 400);
        assert_eq!(stepped((150, 250), rect, 1, 0), (174, 250));
        assert_eq!(stepped((299, 250), rect, 1, 0), (299, 250));
        assert_eq!(stepped((150, 399), rect, 0, 1), (150, 399));
        assert_eq!(stepped((0, 0), rect, -1, -1), (100, 200));
        assert_eq!(stepped((150, 250), (100, 200, 100, 200), 1, 1), (100, 200));
        assert_eq!(stepped((150, 2500), (100, 200, 300, 4299), 0, 1), (150, 2841));
    }
}
