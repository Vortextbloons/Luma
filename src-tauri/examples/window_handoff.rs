#[path = "../src/background.rs"]
#[allow(dead_code)]
mod background;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    let profile = std::env::temp_dir().join(format!("luma-handoff-{}", std::process::id()));
    let test_profile = profile.clone();
    let focus_events = Arc::new(AtomicUsize::new(0));
    let events = focus_events.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .on_window_event(move |window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Focused(true)) {
                events.fetch_add(1, Ordering::Relaxed);
            }
        })
        .setup(move |app| {
            let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::External("about:blank".parse()?))
                .data_directory(test_profile.join("main")).title("Luma handoff regression").fullscreen(true).build()?;
            let embedded = WebviewWindowBuilder::new(app, "youtube", WebviewUrl::External("about:blank".parse()?))
                .data_directory(test_profile.join("youtube")).title("Luma embedded regression").visible(false).focused(false).build()?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    std::thread::sleep(Duration::from_millis(500));
                    verify_hover(&main);
                    let mut focused = 0;
                    for cycle in 0..20 {
                        tauri::async_runtime::block_on(background::present(embedded.clone(), Some(true))).unwrap();
                        focused += verify(&embedded, &main, cycle, "embedded");
                        tauri::async_runtime::block_on(background::present(main.clone(), None)).unwrap();
                        focused += verify(&main, &embedded, cycle, "main");
                    }
                    assert!(focused > 0, "no foreground input checks ran");
                    println!("PASS: 40 fullscreen handoffs, {focused} foreground input checks, {} main focus events; no remote server", focus_events.load(Ordering::Relaxed));
                }));
                handle.exit(if result.is_ok() { 0 } else { 1 });
            });
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(25));
                eprintln!("FAIL: native event loop or handoff timed out");
                std::process::exit(2);
            });
            Ok(())
        })
        .build(context).unwrap();
    app.run(|_, _| {});
    let _ = std::fs::remove_dir_all(profile);
}

fn verify_hover(window: &tauri::WebviewWindow) {
    let css =
        serde_json::to_string(&include_str!("../../src/app.css").replace(":hover", ".test-hover"))
            .unwrap();
    let script = format!(
        r#"(async () => {{
        const style = document.createElement('style');
        style.textContent = {css};
        document.head.append(style);
        const failures = [];
        for (const name of ['header-btn', 'nav-btn', 'app-card', 'media-card', 'activity-main', 'video-card', 'music-card', 'settings-category', 'swatch-choice', 'panel-secondary']) {{
            const button = document.createElement('button');
            button.className = name;
            button.textContent = 'Hover target';
            button.style.cssText = 'position:fixed;left:100px;top:100px;width:160px;height:100px;margin:0';
            document.body.append(button);
            const rect = button.getBoundingClientRect();
            const x = rect.x + rect.width / 2;
            const y = rect.bottom - .25;
            button.classList.add('test-hover');
            await Promise.all(button.getAnimations().map(animation => animation.finished));
            if (!button.contains(document.elementFromPoint(x, y))) failures.push(name);
            button.remove();
        }}
        style.remove();
        window.__hoverFailures = failures;
    }})()"#
    );
    window.eval(script).unwrap();
    std::thread::sleep(Duration::from_secs(2));
    let (tx, rx) = std::sync::mpsc::channel();
    window
        .eval_with_callback("JSON.stringify(window.__hoverFailures)", move |value| {
            let _ = tx.send(value);
        })
        .unwrap();
    let result = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        result, "\"[]\"",
        "hover moves the hit target away from a stationary pointer: {result}"
    );
    println!("PASS: stationary pointer stays inside all 10 hovered controls");
}

fn verify(
    target: &tauri::WebviewWindow,
    hidden: &tauri::WebviewWindow,
    cycle: usize,
    label: &str,
) -> usize {
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        target.is_visible().unwrap(),
        "{cycle} {label}: target hidden"
    );
    assert!(
        !hidden.is_visible().unwrap(),
        "{cycle} {label}: old window visible"
    );
    assert!(
        !hidden.is_focused().unwrap(),
        "{cycle} {label}: hidden window retained input focus"
    );
    let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    let mut pid = 0;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            foreground,
            Some(&mut pid),
        );
    }
    let (tx, rx) = std::sync::mpsc::channel();
    target.eval_with_callback("JSON.stringify({focused: document.hasFocus(), ready: document.readyState, visibility: document.visibilityState, element: document.activeElement?.tagName})", move |value| { let _ = tx.send(value); }).unwrap();
    let response = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("webview event loop stopped responding");
    if pid == std::process::id() {
        assert_eq!(
            foreground.0,
            target.hwnd().unwrap().0,
            "{cycle} {label}: hidden window stole the foreground"
        );
        assert!(
            response.contains("true"),
            "{cycle} {label}: foreground webview lost input focus: {response}"
        );
        1
    } else {
        0
    }
}
