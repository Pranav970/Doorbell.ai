use async_trait::async_trait;
use gateway_common::canonical::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Usage,
};
use secrecy::{ExposeSecret, SecretString};

use crate::error::{map_transport_error, map_upstream_error, ProviderError};
use crate::sse;
use crate::trait_def::{ChatStream, LlmProvider};

/// Canonical request/response shape is already ~OpenAI's native shape, so
/// this adapter is close to a passthrough.
pub struct OpenAiProvider {
    http: reqwest::Client,
    base_url: String,
    /// Whether to ask for `stream_options: {"include_usage": true}` on a
    /// streaming call. See `with_stream_options`.
    stream_options: bool,
}

impl OpenAiProvider {
    /// Defaults to sending `stream_options` — correct for OpenAI itself and
    /// for the registry-backed adapter built from `AppConfig`.
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            stream_options: true,
        }
    }

    /// Opts out of `stream_options` for a BYOK host that rejects unknown
    /// request fields instead of ignoring them. Such a vendor 400s the whole
    /// streaming request, so this trades exact provider-reported token counts
    /// for the request working at all — the token meter falls back to its
    /// character-ratio estimate.
    pub fn with_stream_options(mut self, supported: bool) -> Self {
        self.stream_options = supported;
        self
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn chat_completion(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(api_key.expose_secret())
            .json(request)
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(map_upstream_error(status, body));
        }

        response
            .json::<ChatCompletionResponse>()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))
    }

    async fn chat_completion_stream(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream, ProviderError> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );

        let mut body = serde_json::to_value(request)
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        if let Some(obj) = body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(true));
            // Without this, OpenAI omits usage entirely on a stream and the
            // meter would have to estimate every request. Gated because a
            // strict OpenAI-compatible vendor 400s on the unrecognised field
            // rather than ignoring it — see `with_stream_options`, driven by
            // the credential's `supports_stream_options` column.
            if self.stream_options {
                obj.insert(
                    "stream_options".to_string(),
                    serde_json::json!({ "include_usage": true }),
                );
            }
        }

        let response = self
            .http
            .post(&url)
            .bearer_auth(api_key.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(map_transport_error)?;

        if !response.status().is_success() {
            return Err(sse::error_from_response(response).await);
        }

        Ok(sse::decode(response.bytes_stream(), map_event))
    }
}

/// OpenAI (and every OpenAI-wire vendor, Groq included) sends each frame as
/// a `chat.completion.chunk`. The final usage frame carries an empty
/// `choices` array plus a populated `usage`.
fn map_event(_event: &str, data: &str) -> Option<ChatCompletionChunk> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;

    let choice = value.get("choices").and_then(|c| c.get(0));
    let delta = choice
        .and_then(|c| c.pointer("/delta/content"))
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    let finish_reason = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|f| f.as_str())
        .map(str::to_string);
    let usage = value
        .get("usage")
        .filter(|u| !u.is_null())
        .and_then(|u| serde_json::from_value::<Usage>(u.clone()).ok());

    // Role-only openers and other empty frames carry nothing the caller or
    // the token meter needs.
    if delta.is_empty() && finish_reason.is_none() && usage.is_none() {
        return None;
    }

    Some(ChatCompletionChunk {
        id: value
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or_default()
            .to_string(),
        model: value
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_string(),
        delta,
        finish_reason,
        usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_text_frames_and_ignores_the_role_only_opener() {
        assert!(map_event(
            "",
            r#"{"choices":[{"delta":{"role":"assistant"},"finish_reason":null}]}"#
        )
        .is_none());

        let chunk = map_event(
            "",
            r#"{"id":"c1","model":"gpt-4o","choices":[{"delta":{"content":"Hi"}}]}"#,
        )
        .expect("text frame maps");
        assert_eq!(chunk.delta, "Hi");
        assert_eq!(chunk.model, "gpt-4o");
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn reads_the_trailing_usage_only_frame() {
        let chunk = map_event(
            "",
            r#"{"id":"c1","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":7,"total_tokens":16}}"#,
        )
        .expect("usage frame maps");
        assert_eq!(chunk.delta, "");
        assert_eq!(chunk.usage.unwrap().completion_tokens, 7);
    }
}
