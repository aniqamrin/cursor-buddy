use tauri::{AppHandle, Emitter, Manager};

use super::provider::{ChatMessage, ChatProvider};
use crate::events::{topics, DonePayload, ErrorPayload};
use crate::state::{resolve_api_key, ApiKeySource, AppState};
use crate::windows;

/// Build the system prompt for a turn: persona + rules + live context.
fn build_system_prompt(context_fragment: &str) -> String {
    format!(
        "You are Cursor Buddy, an AI companion that lives beside the user's cursor on Windows. \
You help with whatever is on screen: apps, code, documents, UI, and language.

Context of the user's current moment:
{context_fragment}

Rules:
- Be concise and direct. The bubble is small; short paragraphs win.
- If the context above is relevant, USE IT. Never ask which app the user is in if you already know.
- For how-to questions about the visible app, give concrete numbered steps naming real UI elements.
- When the user's message contains Chinese text, respond with this card format:
  Characters / Pinyin / Literal meaning / Natural English / Tone / Formality / Example sentence.
  Then explain how a native speaker would actually use it (playful? rude? formal?) and one natural reply.
- For teaching questions (e.g. complexity, bugs), guide briefly before revealing full answers.
- Never invent screen contents you were not given."
    )
}

/// Detect CJK characters to add translation-focused behavior hints.
#[allow(dead_code)]
fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

pub struct TurnResult {
    pub conversation_id: i64,
    pub content: String,
}

/// Run one conversational turn end-to-end:
/// persist user message -> build prompt from context -> stream reply ->
/// persist assistant message. Emits `cb://token`, `cb://done`, `cb://error`.
pub async fn run_turn(
    app: AppHandle,
    user_text: String,
) -> Result<TurnResult, String> {
    let state = app.state::<std::sync::Arc<AppState>>();

    let (model, activity_ctx_enabled) = {
        let settings = state.settings.lock().unwrap();
        (settings.model.clone(), settings.activity_context_enabled)
    };

    // Resolve provider credentials without ever exposing the key to the UI layer.
    let api_key = match resolve_api_key(&state.db) {
        ApiKeySource::Stored(k) => k,
        ApiKeySource::Env(k) => k,
        ApiKeySource::Missing => {
            return Err(
                "No API key configured. Open Settings → AI and add your OpenAI API key."
                    .into(),
            )
        }
    };

    let context = {
        let screen_enabled = state.settings.lock().unwrap().screen_context_enabled;

        // Grace-wait: if the bubble was just summoned, OCR is likely still
        // running in the background. Give it a moment so the very first
        // question already sees the screen.
        let fresh_activation = state
            .runtime
            .lock()
            .unwrap()
            .activated_at
            .map(|t| t.elapsed() < std::time::Duration::from_secs(15))
            .unwrap_or(false);

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
        loop {
            let ctx = state.runtime.lock().unwrap().last_context.clone();
            let has_text = ctx
                .as_ref()
                .and_then(|c| c.fresh_screen_text())
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false);
            if has_text || !screen_enabled || !fresh_activation || std::time::Instant::now() >= deadline
            {
                break ctx;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    };
    let context_fragment = match &context {
        Some(ctx) if activity_ctx_enabled => ctx.prompt_fragment(),
        Some(_) => "- Application context: disabled by the user".to_string(),
        None => "- Context: captured at startup; none available".to_string(),
    };

    // Persist user message under the active conversation.
    let conversation_id = {
        let mut rt = state.runtime.lock().unwrap();
        let conv = *rt.conversation_id.get_or_insert_with(|| {
            let title: String = user_text.chars().take(40).collect();
            state.db.create_conversation(&title).unwrap_or(0)
        });
        conv
    };
    state
        .db
        .insert_message(conversation_id, "user", &user_text)?;

    // Assemble message list: system + recent history + current turn.
    let history = state.db.recent_messages(conversation_id, 16)?;
    let mut messages: Vec<ChatMessage> = Vec::with_capacity(history.len() + 2);
    messages.push(ChatMessage {
        role: "system".into(),
        content: build_system_prompt(&context_fragment),
    });
    // History already includes the just-inserted user message as its last row.
    for (role, content) in history {
        messages.push(ChatMessage { role, content });
    }

    let provider = super::openai::OpenAiProvider::new(api_key, state.http.clone());
    let app_for_deltas = app.clone();
    let result = provider
        .stream_chat(&model, &messages, &mut |delta| {
            use tauri::Emitter as _;
            let _ = app_for_deltas.emit(topics::TOKEN, crate::events::TokenPayload {
                delta: delta.to_string(),
            });
        })
        .await;

    match result {
        Ok(content) => {
            state
                .db
                .insert_message(conversation_id, "assistant", &content)?;
            let _ = app.emit(
                topics::DONE,
                DonePayload {
                    conversation_id,
                    content: content.clone(),
                },
            );
            Ok(TurnResult {
                conversation_id,
                content,
            })
        }
        Err(e) => {
            let _ = app.emit(
                topics::ERROR,
                ErrorPayload {
                    message: e.clone(),
                },
            );
            Err(e)
        }
    }
}

/// Capture-and-activate flow shared by hotkey, tray click, and commands.
/// Order matters: snapshot context BEFORE the popup steals focus.
pub fn activate_bubble(app: &AppHandle) {
    let state = app.state::<std::sync::Arc<AppState>>();

    // Always re-snapshot on each activation (fresh context wins).
    let include_app = state.settings.lock().unwrap().activity_context_enabled;
    let snapshot = crate::context::ContextSnapshot::capture(include_app).ok();

    let (cursor, app_info) = match &snapshot {
        Some(s) => ((s.cursor.x, s.cursor.y), s.app.clone()),
        None => ((0, 0), None),
    };
    {
        let mut rt = state.runtime.lock().unwrap();
        rt.last_context = snapshot;
        rt.activated_at = Some(std::time::Instant::now());
    }

    // Placement: physical pixels on the cursor's monitor.
    let monitor = windows::monitor::monitor_for_point(cursor.0, cursor.1)
        .or_else(windows::monitor::primary_monitor);
    let (pos, payload_app_name, payload_title) = match &monitor {
        Some(m) => {
            let logical = (384, 340);
            let physical = m.scale_logical_size(logical.0, logical.1);
            let pos = crate::placement::place_bubble(
                cursor,
                physical,
                m.work_area,
                crate::placement::EDGE_MARGIN,
                crate::placement::CURSOR_GAP * m.scale().round() as i32,
            );
            (
                pos,
                app_info.as_ref().map(|a| a.app_name.clone()),
                app_info.as_ref().map(|a| a.window_title.clone()),
            )
        }
        None => ((100, 100), None, None),
    };

    let (paused, pinned, last_pos) = {
        let rt = state.runtime.lock().unwrap();
        (rt.paused, rt.pinned, rt.last_pos)
    };
    let permission_level = state.settings.lock().unwrap().permission_level.as_str().to_string();

    let window = app.get_webview_window("bubble");

    #[cfg(debug_assertions)]
    eprintln!(
        "[activate] bubble_window={} pos=({},{}) paused={} pinned={}",
        window.is_some(),
        pos.0,
        pos.1,
        paused,
        pinned
    );

    if let Some(window) = window {
        use tauri::PhysicalPosition;

        // Pinned bubble: never yank it back to the cursor. Re-focus when
        // visible; re-show at its last position when hidden.
        if pinned {
            let (x, y) = last_pos.unwrap_or(pos);
            let _ = window.set_position(PhysicalPosition::new(x, y));
            let visible = window.is_visible().unwrap_or(false);
            if !visible {
                let _ = app.emit_to(
                    "bubble",
                    topics::ACTIVATE,
                    crate::events::ActivatePayload {
                        x: cursor.0,
                        y: cursor.1,
                        paused,
                        permission_level,
                        app_name: payload_app_name,
                        window_title: payload_title,
                    },
                );
                let _ = window.show();
            }
            let _ = window.set_focus();
            return;
        }

        let _ = window.set_position(PhysicalPosition::new(pos.0, pos.1));
        let _ = app.emit_to(
            "bubble",
            topics::ACTIVATE,
            crate::events::ActivatePayload {
                x: cursor.0,
                y: cursor.1,
                paused,
                permission_level,
                app_name: payload_app_name,
                window_title: payload_title,
            },
        );
        let _ = window.show();
        let _ = window.set_focus();
    }

    // Phase 2: screen context. The bubble is already visible; OCR runs in
    // the background and attaches its text to the stored snapshot so the
    // next turn (usually seconds later, while the user types) picks it up.
    if !paused && state.settings.lock().unwrap().screen_context_enabled {
        // Region: active window when known, else a cursor-centered crop.
        let region = {
            let rt = state.runtime.lock().unwrap();
            rt.last_context
                .as_ref()
                .and_then(|s| s.app.as_ref().map(|a| a.bounds))
                .unwrap_or(crate::windows::Rect {
                    x: cursor.0 - 400,
                    y: cursor.1 - 300,
                    width: 800,
                    height: 600,
                })
        };

        let state_for_ocr: std::sync::Arc<AppState> = std::sync::Arc::clone(state.inner());
        #[cfg(debug_assertions)]
        eprintln!("[vision] spawning capture, region={region:?}");
        tauri::async_runtime::spawn_blocking(move || {
            // WinRT needs an apartment on this (pool) thread before any
            // Ocr/Imaging call; MTA is fine for blocking .get() usage.
            unsafe {
                use ::windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
                let _ = RoInitialize(RO_INIT_MULTITHREADED);
            }
            let started = std::time::Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::vision::capture_screen_text(region)
            }));
            match outcome {
                Ok(Ok(text)) if !text.trim().is_empty() => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[vision] {} chars in {:?}",
                        text.chars().count(),
                        started.elapsed()
                    );
                    let mut rt = state_for_ocr.runtime.lock().unwrap();
                    if let Some(ctx) = rt.last_context.as_mut() {
                        ctx.screen_text = Some(text);
                        ctx.ocr_at = Some(std::time::Instant::now());
                    }
                }
                Ok(Ok(_)) => eprintln!("[vision] capture ok but no text found"),
                Ok(Err(e)) => eprintln!("[vision] failed: {e}"),
                Err(_) => eprintln!("[vision] capture PANICKED"),
            }
        });
    }
}
