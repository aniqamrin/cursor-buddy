use serde::Serialize;

/// Emitted to the bubble when it activates at the cursor.
#[derive(Clone, Serialize)]
pub struct ActivatePayload {
    pub x: i32,
    pub y: i32,
    pub paused: bool,
    pub permission_level: String,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct TokenPayload {
    pub delta: String,
}

#[derive(Clone, Serialize)]
pub struct DonePayload {
    pub conversation_id: i64,
    pub content: String,
}

#[derive(Clone, Serialize)]
pub struct ErrorPayload {
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct PauseChangedPayload {
    pub paused: bool,
}

#[derive(Clone, Serialize)]
pub struct PermissionChangedPayload {
    pub level: String,
}

pub mod topics {
    pub const ACTIVATE: &str = "cb://activate";
    pub const TOKEN: &str = "cb://token";
    pub const DONE: &str = "cb://done";
    pub const ERROR: &str = "cb://error";
    pub const PAUSE_CHANGED: &str = "cb://pause-changed";
    pub const PERMISSION_CHANGED: &str = "cb://permission-changed";
    pub const GENERATION_STARTED: &str = "cb://generation-started";
}
