/// A single chat message in provider-agnostic form.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

/// Abstraction over AI providers so the app is never coupled to one vendor.
///
/// Implementations stream tokens through the `on_delta` callback and return
/// the complete response text. Object safety is intentionally avoided
/// (RPITIT) — call sites are generic.
pub trait ChatProvider {
    fn stream_chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        on_delta: &mut (dyn FnMut(&str) + Send),
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
}
