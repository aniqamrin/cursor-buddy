use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::events::{topics, PauseChangedPayload, PermissionChangedPayload};
use crate::safety::PermissionLevel;
use crate::state::AppState;
use crate::storage::db::{ConversationSummary, MessageRow};

pub fn toggle_pause_internal(app: &AppHandle) -> bool {
    let state = app.state::<Arc<AppState>>();
    let mut rt = state.runtime.lock().unwrap();
    rt.paused = !rt.paused;
    let paused = rt.paused;
    drop(rt);
    let _ = app.emit(topics::PAUSE_CHANGED, PauseChangedPayload { paused });
    paused
}

pub fn set_permission_internal(app: &AppHandle, level: PermissionLevel) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    {
        let mut settings = state.settings.lock().unwrap();
        settings.permission_level = level;
        settings.save(&state.db)?;
    }
    crate::tray::sync_tray_state(app);
    let _ = app.emit(
        topics::PERMISSION_CHANGED,
        PermissionChangedPayload {
            level: level.as_str().to_string(),
        },
    );
    Ok(())
}

#[tauri::command]
pub fn toggle_pause(app: AppHandle) -> bool {
    toggle_pause_internal(&app)
}

#[tauri::command]
pub fn set_permission_level(
    app: AppHandle,
    level: String,
) -> Result<(), String> {
    let parsed = PermissionLevel::parse(&level).ok_or("Unknown permission level.")?;
    set_permission_internal(&app, parsed)
}

#[tauri::command]
pub fn hide_bubble(app: AppHandle) {
    if let Some(w) = app.get_webview_window("bubble") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn show_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn clear_history(state: State<'_, Arc<AppState>>) -> Result<u64, String> {
    let cleared = state.db.clear_history()?;
    state.runtime.lock().unwrap().conversation_id = None;
    Ok(cleared)
}

#[tauri::command]
pub fn list_conversations(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ConversationSummary>, String> {
    state.db.list_conversations()
}

#[tauri::command]
pub fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: i64,
) -> Result<Vec<MessageRow>, String> {
    state.db.messages_for(conversation_id)
}

/// Pin/unpin the bubble. Pinning captures its current position so later
/// activations restore that spot instead of jumping to the cursor.
#[tauri::command]
pub fn set_bubble_pinned(app: AppHandle, pinned: bool) {
    let state = app.state::<Arc<AppState>>();
    let mut rt = state.runtime.lock().unwrap();
    rt.pinned = pinned;
    if pinned {
        if let Some(w) = app.get_webview_window("bubble") {
            if let Ok(pos) = w.outer_position() {
                rt.last_pos = Some((pos.x, pos.y));
            }
        }
    }
}

/// Manual activation (parity with the hotkey path).
#[tauri::command]
pub fn activate_at_cursor(app: AppHandle) {
    crate::ai::agent::activate_bubble(&app);
}
