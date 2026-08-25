mod ai;
mod commands;
mod context;
mod events;
mod hotkey;
mod placement;
mod safety;
mod state;
mod storage;
mod tray;
mod vision;
mod windows;

use std::sync::Arc;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second launch attempt: focus the existing main window.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state == ShortcutState::Pressed {
                        #[cfg(debug_assertions)]
                        eprintln!("[hotkey] pressed {shortcut}");
                        if hotkey::matches_current(app, shortcut) {
                            ai::agent::activate_bubble(app);
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::chat::chat_send,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::set_api_key,
            commands::settings::remove_api_key,
            commands::settings::api_key_status,
            commands::misc::toggle_pause,
            commands::misc::set_permission_level,
            commands::misc::hide_bubble,
            commands::misc::set_bubble_pinned,
            commands::misc::show_main,
            commands::misc::quit_app,
            commands::misc::clear_history,
            commands::misc::list_conversations,
            commands::misc::get_messages,
            commands::misc::activate_at_cursor,
        ])
        .on_menu_event(|app, event| {
            tray::handle_menu(app, event.id().as_ref());
        })
        .setup(|app| {
            let handle = app.handle().clone();

            let data_dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("cursor-buddy.db");
            let db = storage::Db::open(&db_path)?;
            let app_state = Arc::new(state::AppState::new(db)?);

            *app_state.settings.lock().unwrap() = state::Settings::load(&app_state.db);
            let configured_hotkey =
                app_state.settings.lock().unwrap().hotkey.clone();
            *app_state.current_hotkey.lock().unwrap() = configured_hotkey.clone();
            app.manage(app_state.clone());

            hotkey::register(&handle, &configured_hotkey)?;
            tray::create(&handle)?;

            let first_run = !app_state.settings.lock().unwrap().first_run_completed;
            if first_run {
                if let Some(w) = handle.get_webview_window("main") {
                    w.show()?;
                    w.set_focus()?;
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cursor Buddy");
}
