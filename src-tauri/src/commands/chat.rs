use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::ai::agent;
use crate::events::topics;
use crate::state::AppState;

/// Entry point for a user message. Validates pause/generation state,
/// then runs the turn on the async runtime so the UI stays responsive.
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    text: String,
) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("Message is empty.".into());
    }

    // Atomically claim the generation slot so concurrent sends are rejected.
    {
        let mut rt = state.runtime.lock().unwrap();
        if rt.paused {
            return Err("Cursor Buddy is paused. Resume from the tray menu to chat.".into());
        }
        if rt.generating {
            return Err("Still answering the previous message.".into());
        }
        rt.generating = true;
    }
    let _ = app.emit(topics::GENERATION_STARTED, ());

    let state_inner = Arc::clone(state.inner());
    tauri::async_runtime::spawn(async move {
        let result = agent::run_turn(app.clone(), text).await;
        if let Err(err) = result {
            // Errors are also emitted as events; this log line aids debugging.
            eprintln!("chat turn failed: {err}");
        }
        state_inner.runtime.lock().unwrap().generating = false;
    });

    Ok(())
}
