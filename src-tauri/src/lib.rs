mod commands;
pub mod core;
mod managers;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::default())
        .setup(|app| {
            commands::spawn_hotkey_listener(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::run_pedal_smoketest,
            commands::parse_midi_structure,
            commands::compile_notes,
            commands::compile_notes_from_sheet,
            commands::compile_pedal,
            commands::compile_pedal_and_play,
            commands::start_playback,
            commands::toggle_pause,
            commands::stop_playback,
            commands::seek_playback,
            commands::set_save_dir,
            commands::get_save_dir,
            commands::set_midi_dir,
            commands::get_midi_dir,
            commands::load_app_config,
            commands::save_app_config,
            commands::get_themes_file,
            commands::set_themes_dir,
            commands::get_active_theme_name,
            commands::set_active_theme_name,
            commands::get_custom_themes,
            commands::save_custom_theme,
            commands::delete_custom_theme,
            commands::save_playback,
            commands::load_save_file,
            commands::list_saves,
            commands::resume_from_save,
            commands::rename_save,
            commands::delete_save,
            commands::translate_sheet_to_notes,
            commands::notes_to_sheet,
            commands::start_binding,
            commands::start_save_binding,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                commands::shutdown_playback_now(app_handle.state::<AppState>().inner());
                core::session_cache::clear_cache();
            }
        });
}
