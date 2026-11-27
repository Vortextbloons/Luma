use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Runtime,
};
use tauri_plugin_autostart::ManagerExt;

static KEEP_LAUNCHER_READY: AtomicBool = AtomicBool::new(false);
const LAUNCHER_RUNTIME: &str = "launcher-runtime";
static RETAIN_VIEWS: AtomicBool = AtomicBool::new(false);
const TRAY_ID: &str = "luma-tray";
const MAX_TRAY_PLUGIN_NAME: usize = 40;

fn unloading() -> &'static Mutex<HashSet<String>> {
    static UNLOADING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    UNLOADING.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(crate) fn is_unloading(label: &str) -> bool {
    unloading()
        .lock()
        .ok()
        .is_some_and(|labels| labels.contains(label))
}

pub(crate) fn clear_unloading(label: &str) {
    if let Ok(mut labels) = unloading().lock() {
        labels.remove(label);
    }
}

pub(crate) fn unload(app: &tauri::AppHandle, label: &str) {
    let handle = app.clone();
    let name = label.to_string();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(&name) else {
            return;
        };
        if name == "main" && KEEP_LAUNCHER_READY.load(Ordering::Relaxed) {
            if let Err(error) = keep_launcher_runtime(&handle) {
                eprintln!("Could not keep the launcher ready: {error}");
            }
        }
        if let Ok(mut labels) = unloading().lock() {
            labels.insert(name.clone());
        }
        let _ = window.destroy();
    });
}

pub(crate) fn retains_views() -> bool {
    RETAIN_VIEWS.load(Ordering::Relaxed)
}

pub(crate) fn publish_visibility(window: &tauri::WebviewWindow, visible: bool) {
    let _ = window.emit("window-visibility", visible);
}

fn pause(window: &tauri::WebviewWindow) {
    let _ =
        window.eval("document.querySelectorAll('video,audio').forEach(media => media.pause());");
}

pub(crate) fn dismiss(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        pause(&window);
        let _ = window.hide();
        publish_visibility(&window, false);
    }
    crate::viewing::release(app, label);
    if !retains_views() {
        unload(app, label);
    }
}

fn keep_launcher_runtime(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(LAUNCHER_RUNTIME).is_none() {
        tauri::WebviewWindowBuilder::new(
            app,
            LAUNCHER_RUNTIME,
            tauri::WebviewUrl::External("about:blank".parse().expect("valid blank URL")),
        )
        .additional_browser_args(crate::BROWSER_ARGS)
        .inner_size(1.0, 1.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .build()?;
    }
    Ok(())
}

fn drop_launcher_runtime(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(LAUNCHER_RUNTIME) {
        let _ = window.destroy();
    }
}

#[tauri::command]
pub fn set_keep_launcher_ready(app: tauri::AppHandle, enabled: bool) {
    let was_enabled = KEEP_LAUNCHER_READY.swap(enabled, Ordering::Relaxed);
    if was_enabled && !enabled {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if !KEEP_LAUNCHER_READY.load(Ordering::Relaxed) {
                drop_launcher_runtime(&handle);
            }
        });
    }
}

#[tauri::command]
pub fn set_fast_app_switch(app: tauri::AppHandle, enabled: bool) {
    RETAIN_VIEWS.store(enabled, Ordering::Relaxed);
    if enabled {
        return;
    }
    for label in ["youtube", "stremio"] {
        if app
            .get_webview_window(label)
            .is_some_and(|window| !window.is_visible().unwrap_or(true))
        {
            unload(&app, label);
        }
    }
}

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

fn readiness() -> &'static tokio::sync::watch::Sender<bool> {
    static READY: OnceLock<tokio::sync::watch::Sender<bool>> = OnceLock::new();
    READY.get_or_init(|| tokio::sync::watch::channel(false).0)
}

#[tauri::command]
pub fn launcher_ready() {
    readiness().send_replace(true);
}

pub(crate) async fn main_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    let mut ready = readiness().subscribe();
    let handle = app.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let result = match handle.get_webview_window("main") {
            Some(window) => Ok(window),
            None => {
                readiness().send_replace(false);
                tauri::WebviewWindowBuilder::from_config(&handle, &handle.config().app.windows[0])
                    .and_then(|builder| builder.additional_browser_args(crate::BROWSER_ARGS).build())
                    .map_err(|error| error.to_string())
            }
        };
        if result.is_ok() {
            drop_launcher_runtime(&handle);
        }
        let _ = sender.send(result);
    })
    .map_err(|error| error.to_string())?;
    let window = receiver.await.map_err(|error| error.to_string())??;
    tokio::time::timeout(Duration::from_secs(20), ready.wait_for(|ready| *ready))
        .await
        .map_err(|_| "Luma took too long to start.".to_string())?
        .map_err(|error| error.to_string())?;
    Ok(window)
}

pub fn show(app: &tauri::AppHandle) {
    show_with(app, None, None);
}

pub fn show_at_sign_in(app: &tauri::AppHandle) {
    show_with(app, None, Some(true));
}

fn show_with(app: &tauri::AppHandle, plugin_id: Option<String>, fullscreen: Option<bool>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match main_window(&app).await {
            Ok(window) => {
                if present(window, fullscreen).await.is_ok() {
                    crate::extension_spike::launcher_presented(&app);
                    if let Some(plugin_id) = plugin_id {
                        if let Err(error) = crate::extension_spike::open(&app, &plugin_id) {
                            crate::customization::plugin::notice(&app, error);
                        }
                    }
                }
            }
            Err(error) => eprintln!("Could not open Luma: {error}"),
        }
    });
}

pub fn request_exit(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let window = match main_window(&app).await {
            Ok(window) => window,
            Err(error) => {
                eprintln!("Could not open exit confirmation: {error}");
                return;
            }
        };
        let _ = present(window, None).await;
        let _ = app.emit("exit-requested", ());
    });
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    crate::commands::session::close_steam(&app);
    app.exit(0);
}

#[tauri::command]
pub fn hide_to_tray(app: tauri::AppHandle) {
    crate::viewing::restore_external(&app);
    crate::commands::session::close_steam(&app);
    crate::extension_spike::launcher_hidden(&app);
    unload(&app, "main");
    unload(&app, "youtube");
    unload(&app, "stremio");
    unload(&app, "sports");
    tauri::async_runtime::spawn(async move {
        let _ = crate::tv::restore_audio().await;
    });
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

pub(crate) async fn present(
    window: tauri::WebviewWindow,
    fullscreen: Option<bool>,
) -> Result<(), String> {
    crate::extension_spike::hide_surfaces(window.app_handle());
    if crate::viewing::present(&window)? { return Ok(()); }
    crate::tv::place(&window)?;
    let audio_error = crate::tv::apply_audio(window.app_handle()).await.err();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let _ = window.eval(MOTION_INIT);
    let _ = window.eval(COVER);
    let surface = window.clone();
    let dispatcher = window.clone();
    dispatcher
        .run_on_main_thread(move || {
            let result = (|| -> tauri::Result<()> {
                crate::extension_spike::dismiss(window.app_handle());
                if let Some(fullscreen) = fullscreen {
                    window.set_fullscreen(fullscreen)?;
                }
                window.show()?;
                if window.is_minimized()? {
                    window.unminimize()?;
                }
                for label in ["main", "youtube", "stremio", "sports"] {
                    if label != window.label() {
                        if let Some(other) = window.app_handle().get_webview_window(label) {
                            if other.is_visible()? {
                                if label != "main" {
                                    pause(&other);
                                }
                                other.hide()?;
                                publish_visibility(&other, false);
                                if label == "sports" || (label != "main" && !retains_views()) {
                                    unload(window.app_handle(), label);
                                }
                            }
                        }
                    }
                }
                window.set_focus()?;
                window.as_ref().show()?;
                window.as_ref().set_focus()?;
                publish_visibility(&window, true);
                Ok(())
            })();
            let _ = sender.send(result.map_err(|error| error.to_string()));
        })
        .map_err(|error| error.to_string())?;
    let presented = receiver.await.map_err(|error| error.to_string())?;
    tokio::time::sleep(Duration::from_millis(70)).await;
    let _ = surface.eval(REVEAL);
    presented?;
    crate::tv::report_audio(surface.app_handle(), audio_error);
    Ok(())
}

fn tray_plugin_name(name: &str, id: &str) -> String {
    let source = if name.trim().is_empty() { id } else { name };
    let mut characters = source.chars().filter(|character| !character.is_control());
    let mut label: String = characters.by_ref().take(MAX_TRAY_PLUGIN_NAME).collect();
    if characters.next().is_some() {
        label.pop();
        label.push('…');
    }
    label
}

fn tray_plugin_actions<R: Runtime>(app: &tauri::AppHandle<R>) -> Option<Vec<(String, String)>> {
    let snapshot = app
        .try_state::<crate::customization::Store>()?
        .snapshot()
        .ok()
        .flatten()?;
    let mut actions = Vec::new();
    for plugin in snapshot.plugins.plugins {
        if !plugin.enabled || !plugin.executable {
            continue;
        }
        let name = tray_plugin_name(&plugin.name, &plugin.id);
        match plugin.runtime.as_str() {
            "isolated-ui" => {
                actions.push((format!("plugin:open:{}", plugin.id), format!("Open {name}")))
            }
            "background" => actions.push((
                format!("plugin:disable:{}", plugin.id),
                format!("Disable {name}"),
            )),
            _ => {}
        }
    }
    Some(actions)
}

fn tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    let open = MenuItem::with_id(app, "open", "Open Luma TV", true, None::<&str>)?;
    menu.append(&open)?;
    if let Some(actions) = tray_plugin_actions(app) {
        if !actions.is_empty() {
            let plugins = Submenu::with_id(app, "plugins", "Plugins", true)?;
            for (id, label) in actions {
                let item = MenuItem::with_id(app, id, label, true, None::<&str>)?;
                plugins.append(&item)?;
            }
            menu.append(&plugins)?;
        }
    }
    let quit = MenuItem::with_id(app, "quit", "Exit Luma…", true, None::<&str>)?;
    menu.append(&quit)?;
    Ok(menu)
}

pub(crate) fn refresh_plugin_menu(app: &tauri::AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = tray_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn disable_plugin(app: &tauri::AppHandle, plugin_id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let revision = app
            .try_state::<crate::customization::Store>()
            .and_then(|store| store.read().ok().flatten())
            .map(|document| document.revision);
        let Some(revision) = revision else {
            eprintln!("Could not disable plugin: plugin storage is unavailable.");
            return;
        };
        if let Err(error) =
            crate::customization::customization_plugin_disable(app.clone(), plugin_id, revision)
                .await
        {
            crate::customization::plugin::notice(&app, error.message);
        }
        refresh_plugin_menu(&app);
    });
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let menu = tray_menu(app.handle())?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Luma TV · Remote server")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            if id == "open" {
                show(app);
            } else if id == "quit" {
                request_exit(app);
            } else if let Some(plugin_id) = id.strip_prefix("plugin:open:") {
                show_with(app, Some(plugin_id.to_string()), None);
            } else if let Some(plugin_id) = id.strip_prefix("plugin:disable:") {
                disable_plugin(app, plugin_id.to_string());
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    #[cfg(windows)]
    migrate_startup(app.handle());
    if std::env::args().any(|arg| arg == "--sign-in") {
        show_at_sign_in(app.handle());
    } else if !std::env::args().any(|arg| arg == "--background") {
        show(app.handle());
    }
    Ok(())
}

#[cfg(windows)]
fn migrate_startup(app: &tauri::AppHandle) {
    use windows::Win32::System::Registry::HKEY_CURRENT_USER;

    let legacy = crate::commands::apps::reg_text(
        HKEY_CURRENT_USER,
        Some(r"Software\Microsoft\Windows\CurrentVersion\Run"),
        Some(&app.package_info().name),
    )
    .is_some_and(|entry| entry.trim_end().ends_with(" --background"));
    if legacy && matches!(app.autolaunch().is_enabled(), Ok(true)) {
        if let Err(error) = app.autolaunch().enable() {
            eprintln!("Could not update Windows startup: {error}");
        }
    }
}

#[tauri::command]
pub async fn sleep_pc() -> Result<(), String> {
    #[cfg(windows)]
    {
        #[link(name = "PowrProf")]
        extern "system" {
            fn SetSuspendState(hibernate: u8, force: u8, disable_wake: u8) -> u8;
        }

        tauri::async_runtime::spawn_blocking(|| unsafe {
            use windows::Win32::{
                Foundation::{CloseHandle, GetLastError, ERROR_NOT_ALL_ASSIGNED, HANDLE},
                Security::{
                    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES,
                    SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES,
                    TOKEN_PRIVILEGES, TOKEN_QUERY,
                },
                System::Threading::{GetCurrentProcess, OpenProcessToken},
            };

            static SLEEP: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _guard = SLEEP.try_lock().map_err(|_| "Sleep is already in progress.".to_string())?;
            let mut token = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token)
                .map_err(|error| error.to_string())?;
            let result = (|| {
                let mut privilege = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES {
                        Attributes: SE_PRIVILEGE_ENABLED,
                        ..Default::default()
                    }],
                };
                LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut privilege.Privileges[0].Luid)?;
                let mut previous = TOKEN_PRIVILEGES::default();
                let mut length = 0;
                AdjustTokenPrivileges(
                    token, false, Some(&privilege), std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                    Some(&mut previous), Some(&mut length),
                )?;
                if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                    return Err(windows::core::Error::from_win32());
                }
                let slept = if SetSuspendState(0, 0, 0) == 0 {
                    Err(windows::core::Error::from_win32())
                } else {
                    Ok(())
                };
                AdjustTokenPrivileges(token, false, Some(&previous), 0, None, None)?;
                slept
            })();
            let _ = CloseHandle(token);
            result.map_err(|error| format!("Windows could not sleep: {error}"))
        })
        .await
        .map_err(|error| error.to_string())?
    }
    #[cfg(not(windows))]
    Err("Sleep is available on Windows.".to_string())
}

#[tauri::command]
pub fn startup_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_startup(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|error| error.to_string())
}
