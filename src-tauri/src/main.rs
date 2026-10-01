#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod state;
mod commands;
mod riot {
    pub mod lockfile;
    pub mod config;
    pub mod client;
    pub mod endpoints;
}
mod service {
    pub mod player;
    pub mod stats;
    pub mod party;
    pub mod side;
    pub mod match_service;
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {}", info);
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            let log_dir = std::path::PathBuf::from(appdata).join("VaLight-Tracker");
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
