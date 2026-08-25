use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::context::ContextSnapshot;
use crate::safety::PermissionLevel;
use crate::storage::Db;

pub const DEFAULT_HOTKEY: &str = "Control+Shift+Space";
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_MODEL_GEMINI: &str = "gemini-3.6-flash";
const SETTINGS_KEY: &str = "settings";
const API_KEY_SETTING: &str = "api_key";

/// Persisted user settings. `api_key` is deliberately NOT part of this
/// struct — it lives in its own row and is never serialized to the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
    pub model: String,
    pub autostart: bool,
    /// Gates active-app/window-title context in prompts.
    pub activity_context_enabled: bool,
    /// Gates screen capture context (vision arrives in Phase 2).
    pub screen_context_enabled: bool,
    pub permission_level: PermissionLevel,
    /// Optional long-term memory (Phase 6); off by default.
    pub memory_enabled: bool,
    pub first_run_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.to_string(),
            model: DEFAULT_MODEL.to_string(),
            autostart: false,
            activity_context_enabled: true,
            screen_context_enabled: true,
            permission_level: PermissionLevel::Assist,
            memory_enabled: false,
            first_run_completed: false,
        }
    }
}

impl Settings {
    pub fn load(db: &Db) -> Settings {
        db.get_setting(SETTINGS_KEY)
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, db: &Db) -> Result<(), String> {
        let json =
            serde_json::to_string(self).map_err(|e| format!("serialize settings: {e}"))?;
        db.set_setting(SETTINGS_KEY, &json)
    }
}

/// Mutable runtime state (not persisted).
#[derive(Default)]
pub struct Runtime {
    pub paused: bool,
    pub conversation_id: Option<i64>,
    pub last_context: Option<ContextSnapshot>,
    pub generating: bool,
    /// When the bubble was last summoned (drives the OCR grace-wait).
    pub activated_at: Option<std::time::Instant>,
    /// When pinned, the bubble ignores blur-dismissal and the hotkey
    /// summons it back to its last position instead of the cursor.
    pub pinned: bool,
    /// Last known physical position of the bubble (captured on pin).
    pub last_pos: Option<(i32, i32)>,
}

pub struct AppState {
    pub db: Db,
    pub settings: Mutex<Settings>,
    pub runtime: Mutex<Runtime>,
    /// The accelerator string currently registered with the OS.
    pub current_hotkey: Mutex<String>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(db: Db) -> Result<Self, String> {
        Ok(Self {
            db,
            settings: Mutex::new(Settings::default()),
            runtime: Mutex::default(),
            current_hotkey: Mutex::new(DEFAULT_HOTKEY.to_string()),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .map_err(|e| format!("http client: {e}"))?,
        })
    }
}

/// API key resolution: stored setting first, then environment.
pub enum ApiKeySource {
    Stored(String),
    Env(String),
    Missing,
}

/// Detect provider from the key's shape — no network calls needed.
/// - Google AI Studio keys start with "AQ." (current) or "AIza" (legacy).
/// - Everything else is treated as OpenAI-style.
pub fn detect_provider(key: &str) -> &'static str {
    let k = key.trim();
    if k.starts_with("AQ.") || k.starts_with("AIza") {
        "gemini"
    } else {
        "openai"
    }
}

pub fn resolve_api_key(db: &Db) -> ApiKeySource {
    if let Some(k) = db.get_setting(API_KEY_SETTING).filter(|k| !k.trim().is_empty()) {
        return ApiKeySource::Stored(k);
    }
    match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.trim().is_empty() => ApiKeySource::Env(k),
        _ => ApiKeySource::Missing,
    }
}

pub fn store_api_key(db: &Db, key: &str) -> Result<(), String> {
    db.set_setting(API_KEY_SETTING, key)
}

pub fn clear_stored_api_key(db: &Db) -> Result<(), String> {
    db.delete_setting(API_KEY_SETTING)
}

pub fn mask_key(key: &str) -> String {
    let k = key.trim();
    if k.len() > 12 {
        format!("{}…{}", &k[..4], &k[k.len() - 4..])
    } else {
        "••••".to_string()
    }
}
