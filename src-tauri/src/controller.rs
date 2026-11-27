use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub(crate) struct ControllerHome(Mutex<(bool, Option<u8>)>);

impl Default for ControllerHome {
    fn default() -> Self {
        Self(Mutex::new((true, Some(8))))
    }
}

#[tauri::command]
pub fn set_controller_home(
    state: State<'_, ControllerHome>,
    enabled: bool,
    home_button: Option<u8>,
) {
    if let Ok(mut guard) = state.0.lock() {
        *guard = (enabled, home_button);
    }
}

#[cfg(windows)]
fn button_pressed(gamepad: &windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD, button: u8) -> bool {
    use windows::Win32::UI::Input::XboxController::*;
    let mask = gamepad.wButtons.0;
    match button {
        0 => (mask & XINPUT_GAMEPAD_A.0) != 0,
        1 => (mask & XINPUT_GAMEPAD_B.0) != 0,
        2 => (mask & XINPUT_GAMEPAD_X.0) != 0,
        3 => (mask & XINPUT_GAMEPAD_Y.0) != 0,
        4 => (mask & XINPUT_GAMEPAD_LEFT_SHOULDER.0) != 0,
        5 => (mask & XINPUT_GAMEPAD_RIGHT_SHOULDER.0) != 0,
        6 => gamepad.bLeftTrigger > 128,
        7 => gamepad.bRightTrigger > 128,
        8 => (mask & XINPUT_GAMEPAD_BACK.0) != 0,
        9 => (mask & XINPUT_GAMEPAD_START.0) != 0,
        10 => (mask & XINPUT_GAMEPAD_LEFT_THUMB.0) != 0,
        11 => (mask & XINPUT_GAMEPAD_RIGHT_THUMB.0) != 0,
        12 => (mask & XINPUT_GAMEPAD_DPAD_UP.0) != 0,
        13 => (mask & XINPUT_GAMEPAD_DPAD_DOWN.0) != 0,
        14 => (mask & XINPUT_GAMEPAD_DPAD_LEFT.0) != 0,
        15 => (mask & XINPUT_GAMEPAD_DPAD_RIGHT.0) != 0,
        _ => false,
    }
}

#[cfg(windows)]
pub(crate) fn start(app: AppHandle) {
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};

    std::thread::spawn(move || {
        let mut was_down = [false; 4];
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let (enabled, home_button) = app
                .state::<ControllerHome>()
                .0
                .lock()
                .map(|guard| *guard)
                .unwrap_or((false, None));
            let Some(button) = (if enabled { home_button } else { None }) else {
                was_down = [false; 4];
                continue;
            };

            let main_focused = app
                .get_webview_window("main")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);

            for user in 0..4u32 {
                let mut state = XINPUT_STATE::default();
                let ok = unsafe { XInputGetState(user, &mut state) } == 0;
                let down = ok && button_pressed(&state.Gamepad, button);
                let idx = user as usize;
                if down && !was_down[idx] && !main_focused {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        crate::remote::server::navigate(
                            &handle,
                            crate::remote::server::NavigationCommand::Home,
                        );
                    });
                }
                was_down[idx] = down;
            }
        }
    });
}

#[cfg(not(windows))]
pub(crate) fn start(_app: AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn maps_xinput_buttons_to_standard_indices() {
        use windows::Win32::UI::Input::XboxController::{
            XINPUT_GAMEPAD, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_UP,
        };
        let mut pad = XINPUT_GAMEPAD::default();
        pad.wButtons = XINPUT_GAMEPAD_BACK;
        assert!(button_pressed(&pad, 8));
        assert!(!button_pressed(&pad, 0));
        pad.wButtons = XINPUT_GAMEPAD_A | XINPUT_GAMEPAD_DPAD_UP;
        assert!(button_pressed(&pad, 0));
        assert!(button_pressed(&pad, 12));
    }
}
