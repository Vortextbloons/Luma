use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Choice {
    pub id: String,
    pub name: String,
}

#[cfg(windows)]
mod platform {
    use super::Choice;
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{HWND, LPARAM},
            UI::WindowsAndMessaging::*,
        },
    };

    #[derive(Clone)]
    pub struct Window {
        pub id: isize,
        pub pid: u32,
        placement: WINDOWPLACEMENT,
        style: isize,
    }

    fn handle(id: isize) -> HWND {
        HWND(id as *mut _)
    }

    fn layout_style(style: isize, floating: bool) -> isize {
        let style = style & !((WS_MAXIMIZE | WS_MINIMIZE).0 as isize);
        if floating {
            style
        } else {
            style & !((WS_CAPTION | WS_THICKFRAME).0 as isize)
        }
    }

    fn identity(id: isize) -> Option<u32> {
        let mut pid = 0;
        unsafe {
            if !IsWindow(Some(handle(id))).as_bool() {
                return None;
            }
            GetWindowThreadProcessId(handle(id), Some(&mut pid));
        }
        (pid != 0 && pid != std::process::id()).then_some(pid)
    }

    fn title(id: isize) -> String {
        let mut text = [0u16; 1024];
        let count = unsafe { GetWindowTextW(handle(id), &mut text) };
        String::from_utf16_lossy(&text[..count.max(0) as usize])
    }

    pub fn list() -> Result<Vec<Choice>, String> {
        unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
            let result = &mut *(data.0 as *mut Vec<Choice>);
            let id = hwnd.0 as isize;
            if let Some(pid) = identity(id) {
                let name = title(id);
                if IsWindowVisible(hwnd).as_bool()
                    && !name.is_empty()
                    && GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize == 0
                {
                    result.push(Choice {
                        id: format!("window:{pid}:{id}"),
                        name,
                    });
                }
            }
            BOOL(1)
        }
        let mut result: Vec<Choice> = Vec::new();
        unsafe {
            EnumWindows(Some(collect), LPARAM(&mut result as *mut _ as isize))
                .map_err(|error| error.to_string())?;
        }
        result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(result)
    }

    pub fn capture(id: isize) -> Result<Window, String> {
        let pid = identity(id).ok_or("That window has closed.")?;
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        unsafe {
            GetWindowPlacement(handle(id), &mut placement).map_err(|error| error.to_string())?;
            Ok(Window {
                id,
                pid,
                placement,
                style: GetWindowLongPtrW(handle(id), GWL_STYLE),
            })
        }
    }

    impl Window {
        pub fn key(&self) -> String {
            format!("window:{}:{}", self.pid, self.id)
        }
        pub fn alive(&self) -> bool {
            identity(self.id) == Some(self.pid)
        }
        pub fn focused(&self) -> bool {
            self.alive() && unsafe { GetForegroundWindow() == handle(self.id) }
        }
        pub fn focus(&self) -> Result<(), String> {
            self.check()?;
            unsafe {
                if IsIconic(handle(self.id)).as_bool() {
                    let _ = ShowWindow(handle(self.id), SW_RESTORE);
                }
            }
            crate::commands::session::focus_window(self.id);
            Ok(())
        }
        pub fn maximize(&self) -> Result<(), String> {
            self.check()?;
            unsafe {
                let _ = ShowWindow(handle(self.id), SW_MAXIMIZE);
            }
            self.focus()
        }
        fn check(&self) -> Result<(), String> {
            if self.alive() {
                Ok(())
            } else {
                Err("That window has closed.".into())
            }
        }
        pub fn place(&self, bounds: (i32, i32, u32, u32), floating: bool) -> Result<(), String> {
            self.check()?;
            unsafe {
                let hwnd = handle(self.id);
                let _ = ShowWindow(hwnd, SW_RESTORE);
                SetWindowLongPtrW(hwnd, GWL_STYLE, layout_style(self.style, floating));
                SetWindowPos(
                    hwnd,
                    None,
                    bounds.0,
                    bounds.1,
                    bounds.2 as i32,
                    bounds.3 as i32,
                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
                )
                .map_err(|error| error.to_string())?;
                let mut rect = windows::Win32::Foundation::RECT::default();
                GetWindowRect(hwnd, &mut rect).map_err(|error| error.to_string())?;
                if (rect.right - rect.left - bounds.2 as i32).abs() > 8
                    || (rect.bottom - rect.top - bounds.3 as i32).abs() > 8
                {
                    return Err("This app could not fit the viewing layout. Exit its fullscreen mode or choose another layout.".into());
                }
            }
            Ok(())
        }
        pub fn restore(&self) -> Result<(), String> {
            if !self.alive() {
                return Ok(());
            }
            unsafe {
                SetWindowLongPtrW(handle(self.id), GWL_STYLE, self.style);
                SetWindowPlacement(handle(self.id), &self.placement)
                    .map_err(|error| error.to_string())?;
                SetWindowPos(
                    handle(self.id),
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
                )
                .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        pub fn raise(&self) {
            if self.alive() {
                unsafe {
                    let _ = SetWindowPos(
                        handle(self.id),
                        Some(HWND_TOP),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maximized_sources_can_be_arranged_without_losing_their_original_style() {
            let original = (WS_OVERLAPPEDWINDOW | WS_MAXIMIZE | WS_VISIBLE).0 as isize;
            let split = layout_style(original, false);
            assert_eq!(
                split & ((WS_MAXIMIZE | WS_MINIMIZE | WS_CAPTION | WS_THICKFRAME).0 as isize),
                0
            );
            assert_ne!(split & WS_VISIBLE.0 as isize, 0);
            let floating = layout_style(original, true);
            assert_eq!(floating & WS_MAXIMIZE.0 as isize, 0);
            assert_eq!(floating & WS_CAPTION.0 as isize, WS_CAPTION.0 as isize);
            assert_ne!(original & WS_MAXIMIZE.0 as isize, 0);
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Choice;
    #[derive(Clone)]
    pub struct Window {
        pub id: isize,
    }
    pub fn list() -> Result<Vec<Choice>, String> {
        Ok(Vec::new())
    }
    pub fn capture(_: isize) -> Result<Window, String> {
        Err("Adding app windows requires Windows.".into())
    }
    impl Window {
        pub fn key(&self) -> String {
            String::new()
        }
        pub fn alive(&self) -> bool {
            false
        }
        pub fn focused(&self) -> bool {
            false
        }
        pub fn focus(&self) -> Result<(), String> {
            Err("Window is unavailable.".into())
        }
        pub fn maximize(&self) -> Result<(), String> {
            Err("Window is unavailable.".into())
        }
        pub fn place(&self, _: (i32, i32, u32, u32), _: bool) -> Result<(), String> {
            Err("Window is unavailable.".into())
        }
        pub fn restore(&self) -> Result<(), String> {
            Ok(())
        }
        pub fn raise(&self) {}
    }
}

pub use platform::*;
