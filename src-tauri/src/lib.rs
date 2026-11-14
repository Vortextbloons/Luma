mod activity;
mod background;
mod frame;
mod commands;
mod integrations;
mod remote;

use commands::{apps, media, session, stremio, youtube};
use integrations::spotify::{self, auth::Auth};
use remote::server;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") { background::show(app); }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().args(["--background"]).build())
        .plugin(tauri_plugin_opener::init())
        .manage(apps::Apps::default())
        .manage(session::Session::default())
        .manage(media::Media::default())
        .manage(server::Remote::default())
        .manage(frame::Frame::default())
        .manage(youtube::Bridge::default())
        .manage(stremio::Stremio::default())
        .manage(spotify::player::Player::default())
        .manage(spotify::windows_media::Media::default())
        .setup(|app| {
            background::setup(app)?;
            app.manage(Auth::load(app.handle()).map_err(std::io::Error::other)?);
            app.manage(activity::Activity::initialize(app.handle()).map_err(std::io::Error::other)?);
            apps::load(app.handle()).map_err(std::io::Error::other)?;
            stremio::load(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server::initialize(handle.clone()).await;
                if let Err(error) = frame::initialize(handle.clone()).await { eprintln!("Display Box: {error}"); }
                tauri::async_runtime::spawn(spotify::initialize(handle.clone()));
                if !std::env::args().any(|arg| arg == "--background") {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    youtube::prepare(handle).await;
                }
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
            spotify::spotify_status,
            spotify::spotify_playback,
            spotify::spotify_control,
            spotify::spotify_play,
            spotify::spotify_transfer,
            spotify::spotify_home,
            spotify::spotify_search,
            spotify::spotify_queue,
            spotify::spotify_devices,
            spotify::spotify_client_id,
            spotify::spotify_set_client_id,
            spotify::spotify_connect,
            spotify::spotify_disconnect,
            spotify::spotify_start,
            frame::frame_status,
            frame::configure_frame,
            frame::frame_pairing,
            server::remote_status,
            server::configure_remote,
            server::update_app_state,
            youtube::open_youtube,
            activity::manager::activity_snapshot,
            activity::manager::refresh_provider,
            activity::manager::open_activity,
            activity::manager::clear_activity,
            activity::manager::set_activity_tracking,
            activity::manager::set_provider_enabled,
            activity::manager::set_provider_hidden,
            activity::manager::save_provider_config,
            activity::manager::disconnect_provider,
            stremio::stremio_status,
            stremio::stremio_link_start,
            stremio::stremio_link_poll,
            stremio::stremio_link_cancel,
            stremio::stremio_open_link,
            stremio::stremio_sync,
            stremio::stremio_disconnect,
            stremio::stremio_open
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
