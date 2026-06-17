// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod js_engine;
mod js_utils;

use commands::*;
use js_engine::JsEngine;

fn main() {
    let js_engine = JsEngine::new().expect("Failed to create JS engine");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(js_engine)
        .invoke_handler(tauri::generate_handler![
            preview_rename,
            execute_rename,
            get_folder_files,
            count_folder_files_recursive,
            get_folder_dirs,
            save_history,
            load_history,
            load_saved_scripts,
            save_script,
            delete_script,
            rename_script,
            open_script_file,
            get_scripts_dir_display,
            load_script_logs,
            get_logs_dir_display,
            save_script_manifest,
            get_script_manifest,
            open_script_manifest
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
