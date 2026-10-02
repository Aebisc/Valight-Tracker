#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use valorant_tracker::{commands, riot, state};

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {}", info);
        if let Ok(appdata) = riot::lockfile::local_app_data() {
            let log_dir = appdata.join("VaLight-Tracker");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_file = log_dir.join("crash.log");
            let _ = std::fs::write(log_file, format!("PANIC: {}\n", info));
        }

    }));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let local_client = riot::client::build_local_client();
    let remote_client = riot::client::build_remote_client();
    let app_state = state::AppState::new(local_client, remote_client);

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![commands::get_match])
        .run(tauri::generate_context!())
        .expect("error while running VaLight Tracker");
}
