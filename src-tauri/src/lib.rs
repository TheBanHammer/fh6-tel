pub mod api;
#[cfg(feature = "desktop")]
pub mod commands;
pub mod db;
pub mod event;
pub mod parser;
#[cfg(feature = "server")]
pub mod server;
pub mod session;
pub mod settings;
pub mod udp;
#[cfg(feature = "desktop")]
pub mod update;

use session::SessionManager;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub session_manager: Mutex<SessionManager>,
    pub settings: Mutex<settings::Settings>,
}

/// Shared owner so the UDP writer and the request handlers share one DB mutex.
pub type Shared = std::sync::Arc<AppState>;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::Arc;
    use tauri::Emitter;

    let loaded_settings = settings::load();
    let port = loaded_settings.port;
    let auto_record = loaded_settings.auto_record;

    let state: Shared = Arc::new(AppState {
        db: Mutex::new(db::open().expect("failed to open database")),
        session_manager: Mutex::new(SessionManager::new(auto_record)),
        settings: Mutex::new(loaded_settings),
    });

    // The initial receiver is dropped; the forwarder task below calls tx.subscribe()
    // before the ingest loop starts sending, so no ticks are missed.
    let (tx, _rx) = tokio::sync::broadcast::channel::<event::ServerEvent>(256);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::get_session_packets,
            commands::get_session_laps,
            commands::delete_session,
            commands::clear_all_sessions,
            commands::rename_session,
            commands::set_session_bookmark,
            commands::get_settings,
            commands::save_settings,
            update::check_for_update,
            update::install_update,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Forward broadcast events to the webview via the original event names.
            let mut rx = tx.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(ev) = rx.recv().await {
                    match ev {
                        event::ServerEvent::Tick(pkt) => {
                            let _ = handle.emit("telemetry_tick", &pkt);
                        }
                        event::ServerEvent::BindFailed(msg) => {
                            let _ = handle.emit("udp_bind_failed", msg);
                        }
                        event::ServerEvent::SessionError(msg) => {
                            let _ = handle.emit("session_error", msg);
                        }
                    }
                }
            });

            // Ingest loop.
            let udp_state = state.clone();
            let udp_tx = tx.clone();
            tauri::async_runtime::spawn(async move {
                udp::run(udp_state, port, udp_tx).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
