use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;
use gateway_common::canonical::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
};
use secrecy::SecretString;

use crate::error::ProviderError;

pub type ChatStream =
    Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ProviderError>> + Send>>;

/// One adapter implements this per vendor. Adding a new provider is a new
/// module implementing this trait, plus one line in `ProviderRegistry::new`
/// and a config entry for its base URL — routing/pipeline code never
/// changes.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Registry key, e.g. "openai" | "anthropic" | "gemini".
    fn name(&self) -> &'static str;

    /// Translates the canonical OpenAI-compatible request into this
    /// provider's native wire format, calls it, and translates the response
    /// back to canonical shape.
    async fn chat_completion(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError>;

    /// Phase 2 stub: default-implemented as `NotImplemented` so adding real
    /// SSE streaming later is additive per-adapter, not a trait break for
    /// callers. The routing layer rejects `stream: true` before this would
    /// ever be reached in Phase 1.
    async fn chat_completion_stream(
        &self,
        _api_key: &SecretString,
        _request: &ChatCompletionRequest,
    ) -> Result<ChatStream, ProviderError> {
        Err(ProviderError::NotImplemented("streaming"))
    }
}
