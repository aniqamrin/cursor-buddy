use futures_util::StreamExt;

use super::provider::{ChatMessage, ChatProvider};

pub struct OpenAiProvider {
    pub api_key: String,
    /// Overridable for gateways/local proxies; defaults to api.openai.com.
    pub base_url: String,
    pub http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, http: reqwest::Client) -> Self {
        let base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into());
        Self {
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        }
    }
}

/// Which wire protocol a request should use.
#[derive(Clone, Copy, PartialEq, Debug)]
enum ApiStyle {
    /// OpenAI Responses API (default for api.openai.com).
    Responses,
    /// OpenAI-compatible chat/completions (Gemini, OpenRouter, proxies...).
    ChatCompletions,
}

const GEMINI_COMPAT_BASE: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai";

fn route_for(model: &str, base_url: &str) -> (String, ApiStyle) {
    if model.to_lowercase().starts_with("gemini") {
        return (format!("{GEMINI_COMPAT_BASE}/chat/completions"), ApiStyle::ChatCompletions);
    }
    // A custom base_url implies an OpenAI-compatible gateway.
    if base_url != "https://api.openai.com/v1" {
        return (format!("{base_url}/chat/completions"), ApiStyle::ChatCompletions);
    }
    (format!("{base_url}/responses"), ApiStyle::Responses)
}

impl ChatProvider for OpenAiProvider {
    async fn stream_chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        on_delta: &mut (dyn FnMut(&str) + Send),
    ) -> Result<String, String> {
        let (url, style) = route_for(model, &self.base_url);

        let body = match style {
            ApiStyle::Responses => {
                // Responses API with SSE streaming. `store:false` keeps
                // conversations out of provider-side storage (privacy default).
                let input: Vec<serde_json::Value> = messages
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "role": m.role,
                            "content": [{ "type": input_content_type(&m.role), "text": m.content }]
                        })
                    })
                    .collect();
                serde_json::json!({
                    "model": model,
                    "input": input,
                    "stream": true,
                    "store": false,
                })
            }
            ApiStyle::ChatCompletions => {
                let msgs: Vec<serde_json::Value> = messages
                    .iter()
                    .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
                    .collect();
                serde_json::json!({
                    "model": model,
                    "messages": msgs,
                    "stream": true,
                })
            }
        };

        let mut req = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Accept", "text/event-stream");

        // Google's gateway also accepts its native key header; sending both
        // avoids auth quirks with newer AI Studio key formats.
        if url.contains("generativelanguage.googleapis.com") {
            req = req.header("x-goog-api-key", &self.api_key);
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("provider error {status}: {}", truncate(&text, 400)));
        }

        let mut full = String::new();
        let mut buffer = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("stream error: {e}"))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // SSE frames are separated by a blank line.
            while let Some(idx) = buffer.find("\n\n") {
                let frame = buffer[..idx].to_string();
                buffer.drain(..idx + 2);
                if let Some(event) = parse_sse_frame(&frame, style) {
                    match event {
                        SseEvent::Delta(text) => {
                            full.push_str(&text);
                            on_delta(&text);
                        }
                        SseEvent::Completed => return Ok(full),
                        SseEvent::Failed(msg) => return Err(msg),
                    }
                }
            }
        }

        Ok(full)
    }
}

fn input_content_type(role: &str) -> &'static str {
    // Responses API naming differs per role kind.
    if role == "assistant" {
        "output_text"
    } else {
        "input_text"
    }
}

#[derive(Debug)]
enum SseEvent {
    Delta(String),
    Completed,
    Failed(String),
}

fn parse_sse_frame(frame: &str, style: ApiStyle) -> Option<SseEvent> {
    let data_line = frame
        .lines()
        .find_map(|l| l.strip_prefix("data: "))?
        .trim();

    if data_line == "[DONE]" {
        return Some(SseEvent::Completed);
    }

    let value: serde_json::Value = serde_json::from_str(data_line).ok()?;

    match style {
        ApiStyle::Responses => match value.get("type").and_then(|t| t.as_str()) {
            Some("response.output_text.delta") => Some(SseEvent::Delta(
                value.get("delta").and_then(|d| d.as_str()).unwrap_or("").to_string(),
            )),
            Some("response.completed") => Some(SseEvent::Completed),
            Some("response.failed") | Some("error") => {
                let msg = value.pointer("/response/status_details/error/message")
                    .or_else(|| value.pointer("/response/error/message"))
                    .or_else(|| value.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("the model request failed");
                Some(SseEvent::Failed(msg.to_string()))
            }
            _ => None,
        },
        ApiStyle::ChatCompletions => {
            // Error payloads surface as {"error": {...}}.
            if let Some(msg) = value.pointer("/error/message").and_then(|m| m.as_str()) {
                return Some(SseEvent::Failed(msg.to_string()));
            }
            let choice = value.get("choices").and_then(|c| c.get(0))?;
            if choice.get("finish_reason").and_then(|f| f.as_str()) == Some("stop") {
                return Some(SseEvent::Completed);
            }
            choice
                .pointer("/delta/content")
                .and_then(|d| d.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| SseEvent::Delta(s.to_string()))
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_delta_frames() {
        let frame = "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}";
        match parse_sse_frame(frame, ApiStyle::Responses) {
            Some(SseEvent::Delta(t)) => assert_eq!(t, "Hello"),
            _ => panic!("expected delta"),
        }
    }

    #[test]
    fn parses_completed_and_done() {
        let f1 = "data: {\"type\":\"response.completed\"}";
        assert!(matches!(parse_sse_frame(f1, ApiStyle::Responses), Some(SseEvent::Completed)));
        let f2 = "data: [DONE]";
        assert!(matches!(parse_sse_frame(f2, ApiStyle::Responses), Some(SseEvent::Completed)));
    }

    #[test]
    fn parses_failure() {
        let f = "data: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\",\"status_details\":{\"error\":{\"message\":\"bad key\"}}}}";
        match parse_sse_frame(f, ApiStyle::Responses) {
            Some(SseEvent::Failed(m)) => assert_eq!(m, "bad key"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_chat_completions_stream() {
        let f = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}";
        match parse_sse_frame(f, ApiStyle::ChatCompletions) {
            Some(SseEvent::Delta(t)) => assert_eq!(t, "Hi"),
            _ => panic!("expected delta"),
        }
        let stop = "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}";
        assert!(matches!(parse_sse_frame(stop, ApiStyle::ChatCompletions), Some(SseEvent::Completed)));
        let err = "data: {\"error\":{\"message\":\"API key not valid\"}}";
        match parse_sse_frame(err, ApiStyle::ChatCompletions) {
            Some(SseEvent::Failed(m)) => assert_eq!(m, "API key not valid"),
            _ => panic!("expected failure"),
        }
    }

    #[test]
    fn routes_gemini_models_to_compat_endpoint() {
        let (url, style) = route_for("gemini-2.0-flash", "https://api.openai.com/v1");
        assert!(url.ends_with("/openai/chat/completions"));
        assert_eq!(style, ApiStyle::ChatCompletions);

        let (_, style) = route_for("gpt-4o-mini", "https://api.openai.com/v1");
        assert_eq!(style, ApiStyle::Responses);
    }
}
