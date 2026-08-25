use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::state::AppState;

/// Register `accel` as the global activation shortcut, replacing any
/// previously registered one. Used at startup and when settings change.
pub fn register(app: &AppHandle, accel: &str) -> Result<(), String> {
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|_| format!("'{accel}' is not a valid shortcut"))?;

    // Clear only shortcuts registered by this app, then apply the new one.
    app.global_shortcut()
        .unregister_all()
        .map_err(|err| format!("unregister failed: {err}"))?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|err| format!("register failed: {err}"))?;

    Ok(())
}

/// True when the pressed shortcut equals the currently active accelerator.
/// Compared structurally (parsed vs pressed) so display-format differences
/// can never filter out presses.
pub fn matches_current(app: &AppHandle, shortcut: &Shortcut) -> bool {
    let state = app.state::<Arc<AppState>>();
    let current = state.current_hotkey.lock().unwrap().clone();
    match current.parse::<Shortcut>() {
        Ok(active) => active == *shortcut,
        Err(_) => false,
    }
}
