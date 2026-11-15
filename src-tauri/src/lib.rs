mod activity;
mod background;
mod tv;
mod frame;
mod network;
mod commands;
mod credentials;
mod integrations;
mod remote;
mod sound;
mod customization;

use commands::{apps, media, services, session, steam, stremio, youtube};
use integrations::spotify::{self, auth::Auth};
use remote::server;
use tauri::Manager;

pub(crate) const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

#[tauri::command]
fn updates_enabled(app: tauri::AppHandle) -> bool {
    app.config().plugins.0.get("updater").is_some_and(|config| {
        config.get("pubkey").and_then(|value| value.as_str()).is_some_and(|key| !key.is_empty())
            && config.get("endpoints").and_then(|value| value.as_array()).is_some_and(|urls| !urls.is_empty())
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") { background::show(app); }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().args(["--background"]).build())
        .plugin(tauri_plugin_opener::init())
        .manage(tv::Tv::default())
        .manage(apps::Apps::default())
        .manage(services::Services::default())
        .manage(session::Session::default())
        .manage(server::Remote::default())
        .manage(remote::pointer::Pointer::default())
        .manage(frame::Frame::default())
        .manage(youtube::Bridge::default())
        .manage(stremio::Stremio::default())
        .manage(spotify::player::Player::default())
        .manage(spotify::windows_media::Media::default())
        .setup(|app| {
            app.manage(customization::Store::open(app.handle()));
            if let Ok(Some(document)) = app.state::<customization::Store>().read() {
                customization::publish(app.handle(), &document);
            }
            let customization_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = customization::server::start(customization_handle).await {
                    eprintln!("Customization server: {error}");
                }
            });
            #[cfg(windows)]
            app.manage(network::watch(app.handle())?);
            tv::load(app.handle()).map_err(std::io::Error::other)?;
            background::setup(app)?;
            app.manage(Auth::load(app.handle()).map_err(std::io::Error::other)?);
            app.manage(activity::Activity::initialize(app.handle()).map_err(std::io::Error::other)?);
            apps::load(app.handle()).map_err(std::io::Error::other)?;
            stremio::load(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = frame::initialize(handle.clone()).await { eprintln!("Display Box: {error}"); }
                tauri::async_runtime::spawn(spotify::initialize(handle.clone()));
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    background::request_exit(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            customization::customization_read,
            customization::customization_initialize,
            customization::customization_commit,
            updates_enabled,
            tv::tv_setup,
            tv::tv_setup_warning,
            tv::save_tv_setup,
            tv::restore_tv_audio,
            network::network_status,
            sound::play_startup_sound,
            background::launcher_ready,
            background::startup_enabled,
            background::open_setup_link,
            background::quit_app,
            background::hide_to_tray,
            background::set_startup,
            apps::launch_app,
            steam::steam_recent_games,
            steam::steam_open_game,
            steam::steam_open_store,
            steam::steam_recommendations,
            session::close_launched_app,
            apps::app_paths,
            apps::save_app_paths,
            apps::detect_apps,
            spotify::spotify_status,
            spotify::spotify_playback,
            spotify::spotify_control,
            spotify::spotify_play,
            spotify::spotify_transfer,
            spotify::spotify_home,
            spotify::spotify_browse,
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
            frame::forget_remotes,
            frame::frame_pairing,
            frame::phone_pairing,
            frame::phone_remote_qr,
            frame::open_phone_remote,
            server::update_app_state,
            media::report_timeline,
            server::update_appearance,
            server::update_settings,
            youtube::open_youtube,
            youtube::youtube_feed,
            youtube::youtube_open_video,
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
        if let tauri::RunEvent::ExitRequested { api, code: None, .. } = &event {
            api.prevent_exit();
        }
        if matches!(event, tauri::RunEvent::Exit) {
            commands::session::close_steam(app);
            media::stop(app);
            stremio::shutdown(app);
            let _ = tv::restore_audio_sync();
        }
    });
}
