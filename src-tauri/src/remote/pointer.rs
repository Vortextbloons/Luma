use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

const TICK: Duration = Duration::from_millis(16);
const LEASE: Duration = Duration::from_millis(1500);
const SPEED: f32 = 1100.0;
const DEAD_ZONE: f32 = 0.16;

pub(crate) const HIDE: &str = "(() => { if (document.getElementById('luma-remote-cursor')) return; const style = document.createElement('style'); style.id = 'luma-remote-cursor'; style.textContent = '*{cursor:none !important}'; document.documentElement.appendChild(style); const show = event => { if (event && event.type === 'pointermove' && !event.movementX && !event.movementY) return; style.remove(); window.removeEventListener('pointermove', show, true); if (window.__lumaRemoteCursor === show) delete window.__lumaRemoteCursor; }; window.__lumaRemoteCursor = show; window.addEventListener('pointermove', show, true); })();";
pub(crate) const SHOW: &str = "(() => { const show = window.__lumaRemoteCursor; if (typeof show === 'function') show(); document.getElementById('luma-remote-cursor')?.remove(); })();";

struct Motion {
    owner: String,
    x: f32,
    y: f32,
    rest_x: f32,
    rest_y: f32,
    lease: Instant,
}

#[derive(Default)]
struct State {
    motion: Option<Motion>,
    running: bool,
    released: bool,
    hidden: bool,
}

#[derive(Default)]
pub struct Pointer {
    state: Mutex<State>,
}

fn with_state<T>(app: &tauri::AppHandle, action: impl FnOnce(&mut State) -> T) -> T {
    let pointer = app.state::<Pointer>();
    let mut state = pointer
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    action(&mut state)
}

fn clamp(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn velocity(x: f32, y: f32) -> (f32, f32) {
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= DEAD_ZONE {
        return (0.0, 0.0);
    }
    let scaled = ((magnitude - DEAD_ZONE) / (1.0 - DEAD_ZONE)).min(1.0);
    let speed = SPEED * scaled / magnitude;
    (x * speed, y * speed)
}

fn cursor(app: &tauri::AppHandle, script: &str) {
    for label in ["main", "youtube", "stremio"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.eval(script);
        }
    }
    crate::commands::services::sync_cursor(app);
}

pub(crate) fn cursor_hidden(app: &tauri::AppHandle) -> bool {
    with_state(app, |state| state.hidden)
}

fn watch(app: &tauri::AppHandle) {
    let active = with_state(app, |state| state.hidden || state.motion.is_some());
    if active {
        physical::watch(app);
    } else {
        physical::release();
    }
}

pub fn start(app: &tauri::AppHandle, owner: &str, x: f32, y: f32) {
    let spawn = with_state(app, |state| {
        state.released = false;
        state.motion = Some(Motion {
            owner: owner.into(),
            x: clamp(x),
            y: clamp(y),
            rest_x: 0.0,
            rest_y: 0.0,
            lease: Instant::now() + LEASE,
        });
        if state.running {
            false
        } else {
            state.running = true;
            true
        }
    });
    show_cursor(app);
    if spawn {
        let app = app.clone();
        tauri::async_runtime::spawn(run(app));
    }
}

pub fn update(app: &tauri::AppHandle, owner: &str, x: f32, y: f32) -> bool {
    let (released, spawn) = with_state(app, |state| {
        let now = Instant::now();
        let accepted = state
            .motion
            .as_mut()
            .filter(|motion| motion.owner == owner && motion.lease > now)
            .map(|motion| {
                motion.x = clamp(x);
                motion.y = clamp(y);
                motion.lease = now + LEASE;
            })
            .is_some();
        let spawn = if accepted && !state.running {
            state.running = true;
            true
        } else {
            false
        };
        (state.released, spawn)
    });
    if spawn {
        let app = app.clone();
        tauri::async_runtime::spawn(run(app));
    }
    released
}

pub fn stop(app: &tauri::AppHandle, owner: &str) {
    with_state(app, |state| {
        if state
            .motion
            .as_ref()
            .is_some_and(|motion| motion.owner == owner)
        {
            state.motion = None;
        }
    });
    watch(app);
}

pub fn stop_all(app: &tauri::AppHandle) {
    with_state(app, |state| {
        state.motion = None;
        state.released = false;
    });
    watch(app);
}

pub fn takeover(app: &tauri::AppHandle) {
    with_state(app, |state| {
        state.motion = None;
        state.released = true;
        state.hidden = false;
    });
    cursor(app, SHOW);
    watch(app);
}

pub fn hide_cursor(app: &tauri::AppHandle) {
    with_state(app, |state| state.hidden = true);
    cursor(app, HIDE);
    watch(app);
}

pub fn show_cursor(app: &tauri::AppHandle) {
    with_state(app, |state| state.hidden = false);
    cursor(app, SHOW);
    watch(app);
}

pub(crate) fn hides_cursor(command: &str) -> bool {
    !matches!(command, "stop" | "click" | "fullscreen")
}

pub fn click(app: &tauri::AppHandle) -> Result<(), String> {
    show_cursor(app);
    platform::click()
}

async fn run(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(TICK).await;
        let (dx, dy, done) = with_state(&app, |state| {
            if !state.running {
                return (0, 0, true);
            }
            let now = Instant::now();
            if state
                .motion
                .as_ref()
                .map_or(true, |motion| motion.lease <= now)
            {
                state.motion = None;
                state.released = true;
                state.running = false;
                (0, 0, true)
            } else {
                let motion = state.motion.as_mut().expect("motion is present");
                let (x, y) = velocity(motion.x, motion.y);
                motion.rest_x += x * TICK.as_secs_f32();
                motion.rest_y += y * TICK.as_secs_f32();
                let dx = motion.rest_x.trunc() as i32;
                let dy = motion.rest_y.trunc() as i32;
                motion.rest_x -= dx as f32;
                motion.rest_y -= dy as f32;
                (dx, dy, false)
            }
        });
        if done {
            watch(&app);
            return;
        }
        if dx != 0 || dy != 0 {
            platform::move_cursor(dx, dy);
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::mem::size_of;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_MOVE, MOUSEINPUT,
    };

    fn mouse(
        dx: i32,
        dy: i32,
        flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
    ) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn send(inputs: &[INPUT]) -> Result<(), String> {
        let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err("Windows could not move the pointer.".into());
        }
        Ok(())
    }

    pub fn move_cursor(dx: i32, dy: i32) {
        let _ = send(&[mouse(dx, dy, MOUSEEVENTF_MOVE)]);
    }

    pub fn click() -> Result<(), String> {
        send(&[
            mouse(0, 0, MOUSEEVENTF_LEFTDOWN),
            mouse(0, 0, MOUSEEVENTF_LEFTUP),
        ])
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn move_cursor(_: i32, _: i32) {}
    pub fn click() -> Result<(), String> {
        Err("Pointer control requires Windows.".into())
    }
}

#[cfg(windows)]
mod physical {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    };
    use tauri::AppHandle;
    use windows::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
        },
    };

    static APP: OnceLock<AppHandle> = OnceLock::new();
    static MONITORING: AtomicBool = AtomicBool::new(false);
    static STARTED: AtomicBool = AtomicBool::new(false);

    pub fn watch(app: &AppHandle) {
        let _ = APP.set(app.clone());
        MONITORING.store(true, Ordering::SeqCst);
        if STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        std::thread::spawn(|| unsafe {
            let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), None, 0) else {
                STARTED.store(false, Ordering::SeqCst);
                return;
            };
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let _ = UnhookWindowsHookEx(hook);
        });
    }

    pub fn release() {
        MONITORING.store(false, Ordering::SeqCst);
    }

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && MONITORING.load(Ordering::SeqCst) {
            let data = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            if data.flags & LLMHF_INJECTED == 0 {
                MONITORING.store(false, Ordering::SeqCst);
                if let Some(app) = APP.get().cloned() {
                    tauri::async_runtime::spawn(async move {
                        super::takeover(&app);
                    });
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

#[cfg(not(windows))]
mod physical {
    use tauri::AppHandle;

    pub fn watch(_: &AppHandle) {}
    pub fn release() {}
}

#[cfg(test)]
mod tests {
    use super::{clamp, hides_cursor, velocity, DEAD_ZONE};

    #[test]
    fn joystick_dead_zone_suppresses_drift() {
        assert_eq!(velocity(0.0, 0.0), (0.0, 0.0));
        assert_eq!(velocity(DEAD_ZONE, 0.0), (0.0, 0.0));
        assert_eq!(velocity(0.0, -DEAD_ZONE), (0.0, 0.0));
    }

    #[test]
    fn joystick_speed_scales_with_distance_and_direction() {
        let (x, y) = velocity(1.0, 0.0);
        assert!(x > 1000.0);
        assert_eq!(y, 0.0);
        let (x, y) = velocity(0.0, 1.0);
        assert_eq!(x, 0.0);
        assert!(y > 1000.0);
        let (slow_x, _) = velocity(0.2, 0.0);
        assert!(slow_x < 100.0);
    }

    #[test]
    fn remote_clicks_and_fullscreen_keep_the_pointer_visible() {
        for command in ["stop", "click", "fullscreen"] {
            assert!(!hides_cursor(command), "{command}");
        }
        for command in [
            "up", "down", "left", "right", "activate", "back", "home", "toggle", "previous",
            "next", "volumeup", "volumedown", "mute", "youtube", "netflix",
        ] {
            assert!(hides_cursor(command), "{command}");
        }
    }

    #[test]
    fn pointer_input_is_clamped_and_rejects_non_finite_values() {
        assert_eq!(clamp(4.0), 1.0);
        assert_eq!(clamp(-4.0), -1.0);
        assert_eq!(clamp(f32::NAN), 0.0);
        assert_eq!(clamp(f32::INFINITY), 0.0);
    }
}
