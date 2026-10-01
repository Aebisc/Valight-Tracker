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
