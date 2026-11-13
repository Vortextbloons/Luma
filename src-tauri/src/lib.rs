mod commands;
mod remote;

use commands::{apps, media};
use remote::server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(apps::Apps::default())
        .manage(media::Media::default())
        .manage(server::Remote::default())
        .setup(|app| {
            apps::load(app.handle()).map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server::initialize(handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apps::launch_app,
            apps::app_paths,
            apps::save_app_paths,
            apps::detect_apps,
            media::media_state,
            media::media_control,
            server::remote_status,
            server::configure_remote,
            server::update_app_state
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
