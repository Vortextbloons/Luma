mod activity;
mod background;
mod commands;
mod credentials;
mod customization;
mod extension_spike;
mod frame;
mod integrations;
mod network;
mod remote;
mod sound;
mod tv;
mod viewing;

use commands::{apps, media, services, session, sports, steam, stremio, youtube};
use integrations::spotify::{self, auth::Auth};
use remote::server;
use tauri::Manager;

pub(crate) const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --disk-cache-size=67108864";

#[tauri::command]
fn updates_enabled(app: tauri::AppHandle) -> bool {
    app.config().plugins.0.get("updater").is_some_and(|config| {
        config
            .get("pubkey")
            .and_then(|value| value.as_str())
            .is_some_and(|key| !key.is_empty())
            && config
                .get("endpoints")
                .and_then(|value| value.as_array())
                .is_some_and(|urls| !urls.is_empty())
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") {
                background::show(app);
            }
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol(extension_spike::SCHEME, extension_spike::protocol)
        .manage(tv::Tv::default())
        .manage(viewing::Viewing::default())
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
        .manage(customization::mcp::McpAccess::default())
        .setup(|app| {
            app.manage(customization::Store::open(app.handle()));
            if let Ok(Some(document)) = app.state::<customization::Store>().read() {
                customization::publish(app.handle(), &document);
            }
            let customization_handle = app.handle().clone();
            let mcp_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = customization::mcp::start(mcp_handle).await {
                    eprintln!("Customization MCP server: {error}");
                }
            });
            tauri::async_runtime::spawn(async move {
                if let Err(error) = customization::server::start(customization_handle).await {
                    eprintln!("Customization server: {error}");
                }
            });
            #[cfg(windows)]
            app.manage(network::watch(app.handle())?);
            tv::load(app.handle()).map_err(std::io::Error::other)?;
            tv::shortcut(app.handle());
            background::setup(app)?;
            app.manage(Auth::load(app.handle()).map_err(std::io::Error::other)?);
            app.manage(
                activity::Activity::initialize(app.handle()).map_err(std::io::Error::other)?,
            );
            apps::load(app.handle()).map_err(std::io::Error::other)?;
            stremio::load(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = frame::initialize(handle.clone()).await {
                    eprintln!("Display Box: {error}");
                }
                tauri::async_runtime::spawn(spotify::initialize(handle.clone()));
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                viewing::focus(window.app_handle(), window.label());
            }
            if matches!(event, tauri::WindowEvent::Focused(false)) {
                viewing::refresh_external_focus(window.app_handle());
            }
            if matches!(event, tauri::WindowEvent::Destroyed) && ["youtube", "stremio", "sports"].contains(&window.label()) {
                viewing::release(window.app_handle(), window.label());
            }
            if window.label() == "main" {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    if let Some(store) = window.app_handle().try_state::<customization::Store>() {
                        store.clear_preview();
                    }
                    extension_spike::dismiss(window.app_handle());
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    background::request_exit(window.app_handle());
                }
            }
        })
        .invoke_handler(extension_spike::guard(tauri::generate_handler![
            customization::customization_read,
            customization::customization_initialize,
            customization::customization_commit,
            customization::customization_preview_read,
            customization::customization_preview,
            customization::customization_preview_keep,
            customization::customization_preview_cancel,
            customization::customization_restore,
            customization::customization_restore_layout,
            customization::customization_history,
            customization::customization_restore_version,
            customization::customization_reset_appearance,
            customization::customization_plugins_read,
            customization::customization_plugin_import,
            customization::customization_plugin_open_surface,
            customization::customization_plugin_export,
            customization::customization_plugin_preview,
            customization::customization_plugin_disable,
            customization::customization_plugin_remove,
            customization::customization_plugin_settings_save,
            customization::mcp::customization_mcp_status,
            customization::mcp::customization_mcp_enable,
            customization::mcp::customization_mcp_disable,
            customization::mcp::customization_mcp_rotate,
            updates_enabled,
            tv::tv_setup,
            tv::switch_display,
            viewing::viewing_state,
            viewing::viewing_windows,
            viewing::viewing_add_window,
            viewing::viewing_sources,
            viewing::viewing_open_source,
            viewing::viewing_configure,
            viewing::viewing_remove,
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
            background::set_fast_app_switch,
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
            frame::forget_device,
            frame::forget_remotes,
            frame::frame_pairing,
            frame::phone_pairing,
            frame::phone_remote_qr,
            frame::open_phone_remote,
            server::update_app_state,
            server::update_appearance,
            sports::sports_listing,
            sports::sports_streameast_listing,
            sports::sports_open,
            sports::sports_scoreboard,
            sports::sports_teams,
            sports::sports_artwork,
            sports::sports_update_score,
            youtube::open_youtube,
            youtube::youtube_feed,
            youtube::youtube_search,
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
        ]))
        .build(tauri::generate_context!())
        .expect("error while running Luma");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } = &event
        {
            api.prevent_exit();
        }
        if matches!(event, tauri::RunEvent::Exit) {
            commands::session::close_steam(app);
            viewing::restore_external(app);
            tv::stop_shortcut(app);
            media::stop(app);
            stremio::shutdown(app);
            let _ = tv::restore_audio_sync();
        }
    });
}
