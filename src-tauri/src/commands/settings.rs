use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt as _;

use crate::safety::PermissionLevel;
use crate::state::{
    clear_stored_api_key, mask_key, resolve_api_key, store_api_key, ApiKeySource, AppState,
    Settings,
};

#[derive(Clone, Serialize)]
pub struct ApiKeyStatus {
    pub configured: bool,
    pub source: String, // "stored" | "env" | "missing"
    pub masked: Option<String>,
}

fn status_from(state: &AppState) -> ApiKeyStatus {
    match resolve_api_key(&state.db) {
        ApiKeySource::Stored(k) => ApiKeyStatus {
            configured: true,
            source: "stored".into(),
            masked: Some(mask_key(&k)),
        },
        ApiKeySource::Env(k) => ApiKeyStatus {
            configured: true,
            source: "env".into(),
            masked: Some(mask_key(&k)),
        },
        ApiKeySource::Missing => ApiKeyStatus {
            configured: false,
            source: "missing".into(),
            masked: None,
        },
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Persist settings and apply side effects (hotkey re-registration,
/// autostart). Returns the effective settings on success.
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<Settings, String> {
    // Validate permission level string round-trips.
    if PermissionLevel::parse(settings.permission_level.as_str()).is_none() {
        return Err("Unknown permission level.".into());
    }

    // Re-register the global shortcut only when it changed.
    let current = state.current_hotkey.lock().unwrap().clone();
    if settings.hotkey != current {
        crate::hotkey::register(&app, &settings.hotkey)
            .map_err(|e| format!("Could not set shortcut '{0}': {e}", settings.hotkey))?;
        *state.current_hotkey.lock().unwrap() = settings.hotkey.clone();
    }

    // Autostart toggle.
    let autolaunch = app.autolaunch();
    if settings.autostart {
        autolaunch.enable().map_err(|e| format!("autostart: {e}"))?;
    } else {
        autolaunch.disable().map_err(|e| format!("autostart: {e}"))?;
    }

    settings.save(&state.db)?;

    let mut guard = state.settings.lock().unwrap();
    *guard = settings.clone();
    Ok(guard.clone())
}

#[tauri::command]
pub async fn set_api_key(
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<ApiKeyStatus, String> {
    let trimmed = key.trim().to_string();
    if trimmed.len() < 20 {
        return Err("That does not look like a valid API key.".into());
    }
    store_api_key(&state.db, &trimmed)?;

    // Auto-select a matching default model when the current one belongs to
    // a different provider, so pasting a key is all the setup there is.
    let provider = crate::state::detect_provider(&trimmed);
    let desired_model = match provider {
        "gemini" => crate::state::DEFAULT_MODEL_GEMINI,
        _ => crate::state::DEFAULT_MODEL,
    };

    {
        let mut guard = state.settings.lock().unwrap();
        let family_ok = (provider == "gemini" && guard.model.starts_with("gemini"))
            || (provider == "openai" && guard.model.starts_with("gpt"));
        if !family_ok {
            guard.model = desired_model.to_string();
            let _ = guard.save(&state.db);
        }
    }

    Ok(status_from(state.inner()))
}

#[tauri::command]
pub async fn remove_api_key(state: State<'_, Arc<AppState>>) -> Result<ApiKeyStatus, String> {
    clear_stored_api_key(&state.db)?;
    Ok(status_from(state.inner()))
}

#[tauri::command]
pub fn api_key_status(state: State<'_, Arc<AppState>>) -> ApiKeyStatus {
    status_from(state.inner())
}
