use std::time::Duration;
use tauri::{menu::{Menu, MenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, Manager};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn open_setup_link(service: String) -> Result<(), String> {
    let url = match service.as_str() {
        "spotify" => "https://developer.spotify.com/dashboard",
        "plex" => "https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/",
        "jellyfin" => "https://jellyfin.org/docs/general/quick-start/",
        "trakt" => "https://developer.trakt.tv/docs/authentication-oauth",
        _ => return Err("Choose a supported setup guide.".into()),
    };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        tauri::async_runtime::spawn(async move {
            let _ = present(window, None).await;
        });
    }
}

pub(crate) const MOTION_INIT: &str = r#"(() => {
  if (window.top !== window || window.__lumaEnter) return;
  const EASE = 'cubic-bezier(.22,.8,.24,1)';
  const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const current = () => document.getElementById('luma-enter');

  const fade = () => {
    const element = current();
    if (!element) return;
    if (reduced()) { element.remove(); return; }
    const animation = element.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 150, easing: EASE });
    animation.onfinish = () => element.remove();
    animation.oncancel = () => element.remove();
  };

  const cover = () => {
    current()?.remove();
    if (reduced()) return;
    const element = document.createElement('div');
    element.id = 'luma-enter';
    element.setAttribute('aria-hidden', 'true');
    element.style.cssText = 'position:fixed;inset:0;z-index:2147483647;pointer-events:none;background:#0b0d12';
    document.documentElement.append(element);
    setTimeout(() => { if (element.isConnected) element.remove(); }, 20000);
  };

  const reveal = () => {
    if (!current()) return;
    if (document.readyState === 'complete') { fade(); return; }
    window.addEventListener('load', fade, { once: true });
    setTimeout(fade, 6000);
  };

  window.__lumaEnter = { cover, reveal };

  let fresh = true;
  try {
    fresh = !sessionStorage.getItem('luma-entered');
    sessionStorage.setItem('luma-entered', '1');
  } catch {}
  if (fresh) cover();
})();"#;

pub(crate) const COVER: &str = "window.__lumaEnter && window.__lumaEnter.cover();";
pub(crate) const REVEAL: &str = "window.__lumaEnter && window.__lumaEnter.reveal();";

pub(crate) async fn present(window: tauri::WebviewWindow, fullscreen: Option<bool>) -> Result<(), String> {
    crate::tv::place(&window)?;
    let audio_error = crate::tv::apply_audio(window.app_handle()).await.err();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let _ = window.eval(MOTION_INIT);
    let _ = window.eval(COVER);
    let surface = window.clone();
    let dispatcher = window.clone();
    dispatcher.run_on_main_thread(move || {
        let result = (|| -> tauri::Result<()> {
            if let Some(fullscreen) = fullscreen {
                window.set_fullscreen(fullscreen)?;
            }
            window.show()?;
            if window.is_minimized()? {
                window.unminimize()?;
            }
            for label in ["main", "youtube", "stremio"] {
                if label != window.label() {
                    if let Some(other) = window.app_handle().get_webview_window(label) {
                        if other.is_visible()? {
                            other.hide()?;
                        }
                    }
                }
            }
            window.set_focus()?;
            window.as_ref().show()?;
            window.as_ref().set_focus()?;
            Ok(())
        })();
        let _ = sender.send(result.map_err(|error| error.to_string()));
    }).map_err(|error| error.to_string())?;
    let presented = receiver.await.map_err(|error| error.to_string())?;
    tokio::time::sleep(Duration::from_millis(70)).await;
    let _ = surface.eval(REVEAL);
    presented?;
    crate::tv::report_audio(surface.app_handle(), audio_error);
    Ok(())
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Luma TV", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Luma and stop server", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::new().tooltip("Luma TV · Remote server").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) { show(tray.app_handle()); }
        });
    if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
    tray.build(app)?;
    if !std::env::args().any(|arg| arg == "--background") { show(app.handle()); }
    Ok(())
}

#[tauri::command]
pub fn startup_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_startup(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enabled { launcher.enable() } else { launcher.disable() }.map_err(|error| error.to_string())
}
