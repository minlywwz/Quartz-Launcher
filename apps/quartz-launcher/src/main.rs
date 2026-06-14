#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod discord;
mod game_sessions;
mod state;
mod system_memory;

use state::AppState;
use tauri::Manager;

use crate::discord::sync_enabled;

fn main() {
    load_env();

    let instance = single_instance::SingleInstance::new("com.quartz.launcher").unwrap();
    if !instance.is_single() {
        eprintln!("Quartz Launcher is already running.");
        std::process::exit(0);
    }

    let app_state = AppState::new().expect("failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            let state = app.state::<AppState>();
            sync_enabled(state.discord_enabled());
            Ok(())
        })
        .invoke_handler(commands::invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running Quartz Launcher");
}

fn load_env() {
    let candidates = [".env", "../.env", "../../.env"];
    for path in candidates {
        let _ = dotenvy::from_filename(path);
    }
}
