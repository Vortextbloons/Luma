mod background;
mod frame;
mod commands;
mod remote;

use commands::{apps, media, youtube};
use remote::server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") { background::show(app); }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().args(["--background"]).build())
        .manage(apps::Apps::default())
        .manage(media::Media::default())
        .manage(server::Remote::default())
        .manage(frame::Frame::default())
        .manage(youtube::Bridge::default())
        .setup(|app| {
            background::setup(app)?;
            apps::load(app.handle()).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server::initialize(handle.clone()).await;
                if let Err(error) = frame::initialize(handle).await { eprintln!("Display Box: {error}"); }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            background::startup_enabled,
            background::set_startup,
            apps::launch_app,
            apps::app_paths,
            apps::save_app_paths,
            apps::detect_apps,
            media::media_state,
            media::media_control,
            frame::frame_status,
            frame::configure_frame,
            frame::frame_pairing,
            server::remote_status,
            server::configure_remote,
            server::update_app_state,
            youtube::open_youtube
        ])
        .build(tauri::generate_context!())
        .expect("error while running Luma");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            server::stop(app);
            media::stop(app);
        }
    });
}
