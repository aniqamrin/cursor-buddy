use serde::Serialize;

use crate::windows::{active_window::ActiveAppInfo, cursor};

/// Screen text older than this never enters prompts (privacy TTL).
pub const SCREEN_TEXT_TTL: std::time::Duration = std::time::Duration::from_secs(300);

/// Everything Cursor Buddy knows about the moment of activation.
/// Captured *before* the popup takes focus so the foreground app
/// is still the user's target application.
#[derive(Clone, Debug, Serialize)]
pub struct ContextSnapshot {
    pub cursor: cursor::CursorPosition,
    pub app: Option<ActiveAppInfo>,
    pub captured_at: String,
    /// OCR text of the visible region, filled in asynchronously after the
    /// bubble is shown. None = disabled or not yet available.
    #[serde(skip)]
    pub screen_text: Option<String>,
    /// When the OCR text landed (drives the privacy TTL).
    #[serde(skip)]
    pub ocr_at: Option<std::time::Instant>,
}

impl ContextSnapshot {
    pub fn capture(include_app_context: bool) -> Result<Self, String> {
        let cur = cursor::get_cursor_position()?;
        let app = if include_app_context {
            crate::windows::active_window::foreground_app_info()
        } else {
            None
        };
        Ok(Self {
            cursor: cur,
            app,
            captured_at: chrono::Local::now().to_rfc3339(),
            screen_text: None,
            ocr_at: None,
        })
    }

    /// Screen text that is still inside the privacy TTL.
    pub fn fresh_screen_text(&self) -> Option<&str> {
        let fresh = self
            .ocr_at
            .map(|t| t.elapsed() < SCREEN_TEXT_TTL)
            .unwrap_or(false);
        if fresh {
            self.screen_text.as_deref().map(str::trim).filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    /// The fragment injected into the system prompt for this turn.
    pub fn prompt_fragment(&self) -> String {
        let mut fragment = match &self.app {
            Some(app) => format!(
                "- Active app: {} (process: {})\n- Window title: {}\n- Window bounds: x={} y={} w={} h={} (physical px)\n- Cursor position: ({}, {}) physical px",
                app.app_name,
                app.process_name,
                if app.window_title.is_empty() { "<untitled>" } else { &app.window_title },
                app.bounds.x,
                app.bounds.y,
                app.bounds.width,
                app.bounds.height,
                self.cursor.x,
                self.cursor.y,
            ),
            None => "- Application context: unavailable (app-context capture is disabled)".to_string(),
        };

        if let Some(text) = self.fresh_screen_text() {
            fragment.push_str(&format!(
                "\n- Visible text on screen (OCR):\n{}",
                crate::vision::excerpt_for_prompt(text)
            ));
        }

        fragment
    }
}
