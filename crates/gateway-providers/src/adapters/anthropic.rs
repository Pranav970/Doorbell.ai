use async_trait::async_trait;
use gateway_common::canonical::{
    ChatCompletionChoice, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, Tool, ToolCall, ToolCallFunction, Usage,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{map_transport_error, map_upstream_error, ProviderError};
use crate::sse;
use crate::trait_def::{ChatStream, LlmProvider};

/// Anthropic requires `max_tokens` (canonical's is optional) and takes
/// `system` as a top-level field rather than a message with role "system".
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct AnthropicProvider {
    http: reqwest::Client,
    base_url: String,
    api_version: String,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_version: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            api_version,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn chat_completion(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let native_request = to_native_request(request);

        let response = self
            .http
            .post(&url)
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", &self.api_version)
            .json(&native_request)
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(map_upstream_error(status, body));
        }

        let native_response: NativeResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        Ok(from_native_response(request.model.clone(), native_response))
    }

    async fn chat_completion_stream(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let mut body = serde_json::to_value(to_native_request(request))
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        if let Some(obj) = body.as_object_mut() {
            obj.insert("stream".to_string(), Value::Bool(true));
        }

        let response = self
            .http
            .post(&url)
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", &self.api_version)
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

/// Anthropic names every frame with an `event:` line and splits token counts
/// across two of them: input tokens arrive up front on `message_start`,
/// output tokens only on the closing `message_delta`. Both are forwarded as
/// partial `Usage` values — the meter merges them, so neither frame has to
/// know about the other.
fn map_event(event: &str, data: &str) -> Option<ChatCompletionChunk> {
    let value: Value = serde_json::from_str(data).ok()?;
    // The `type` field duplicates the event name; prefer the name but fall
    // back so a provider that omits `event:` still decodes.
    let kind = if event.is_empty() {
        value
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
    } else {
        event
    };

    match kind {
        "content_block_delta" => {
            // `text_delta` carries prose; `input_json_delta` carries tool
            // arguments, which are output tokens too but aren't assistant
            // text — only the former is forwarded as visible content.
            let text = value.pointer("/delta/text").and_then(|t| t.as_str())?;
            Some(sse::text_chunk("", "", text))
        }
        "message_start" => {
            let input = value
                .pointer("/message/usage/input_tokens")
                .and_then(|t| t.as_u64())? as u32;
            Some(ChatCompletionChunk {
                id: value
                    .pointer("/message/id")
                    .and_then(|i| i.as_str())
                    .unwrap_or_default()
                    .to_string(),
                model: value
                    .pointer("/message/model")
                    .and_then(|m| m.as_str())
                    .unwrap_or_default()
                    .to_string(),
                delta: String::new(),
                finish_reason: None,
                usage: Some(Usage {
                    prompt_tokens: input,
                    completion_tokens: 0,
                    total_tokens: input,
                }),
            })
        }
        "message_delta" => {
            let output = value
                .pointer("/usage/output_tokens")
                .and_then(|t| t.as_u64())
                .map(|n| n as u32);
            let finish_reason = value
                .pointer("/delta/stop_reason")
                .and_then(|s| s.as_str())
                .map(str::to_string);
            if output.is_none() && finish_reason.is_none() {
                return None;
            }
            Some(ChatCompletionChunk {
                id: String::new(),
                model: String::new(),
                delta: String::new(),
                finish_reason,
                usage: output.map(|out| Usage {
                    prompt_tokens: 0,
                    completion_tokens: out,
                    total_tokens: out,
                }),
            })
        }
        // ping / content_block_start / content_block_stop / message_stop
        // carry neither text nor counts.
        _ => None,
    }
}

#[derive(Debug, Serialize)]
struct NativeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<NativeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<NativeTool>>,
}

#[derive(Debug, Serialize)]
struct NativeTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: Value,
}

#[derive(Debug, Serialize, PartialEq)]
struct NativeMessage {
    role: String,
    content: NativeContent,
}

/// Anthropic's `content` is either a plain string or an array of typed
/// blocks (text / tool_use / tool_result) — this mirrors that union so a
/// plain text turn still serializes as a bare string like the wire format
/// expects, rather than a single-element array.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum NativeContent {
    Text(String),
    Blocks(Vec<NativeBlock>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum NativeBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Deserialize)]
struct NativeResponse {
    id: String,
    content: Vec<NativeContentBlock>,
    stop_reason: Option<String>,
    usage: NativeUsage,
}

#[derive(Debug, Default, Deserialize)]
struct NativeContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct NativeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

fn to_native_tools(tools: &[Tool]) -> Vec<NativeTool> {
    tools
        .iter()
        .map(|t| NativeTool {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            input_schema: t.function.parameters.clone(),
        })
        .collect()
}

/// A canonical assistant message's `tool_calls` become `tool_use` blocks —
/// `arguments` is a JSON-encoded string in canonical form but Anthropic wants
/// a real object, so this parses it (falling back to an empty object on
/// malformed JSON rather than failing the whole request).
fn tool_calls_to_blocks(content: &str, tool_calls: &[ToolCall]) -> Vec<NativeBlock> {
    let mut blocks = Vec::new();
    if !content.is_empty() {
        blocks.push(NativeBlock::Text {
            text: content.to_string(),
        });
    }
    for call in tool_calls {
        let input = serde_json::from_str(&call.function.arguments)
            .unwrap_or(Value::Object(Default::default()));
        blocks.push(NativeBlock::ToolUse {
            id: call.id.clone(),
            name: call.function.name.clone(),
            input,
        });
    }
    blocks
}

/// Extracts any `role: "system"` messages out of `messages` into Anthropic's
/// top-level `system` field (joined with newlines if there were several),
/// translates tool calls/results (see `tool_calls_to_blocks`), and passes the
/// remainder through as the `messages` array.
fn to_native_request(req: &ChatCompletionRequest) -> NativeRequest {
    let mut system_parts = Vec::new();
    let mut messages = Vec::new();
    for m in &req.messages {
        match m.role.as_str() {
            "system" => system_parts.push(m.content.clone()),
            // Anthropic has no "tool" role — a tool result is a `user`
            // message carrying a `tool_result` block instead.
            "tool" => messages.push(NativeMessage {
                role: "user".to_string(),
                content: NativeContent::Blocks(vec![NativeBlock::ToolResult {
                    tool_use_id: m.tool_call_id.clone().unwrap_or_default(),
                    content: m.content.clone(),
                }]),
            }),
            _ => {
                let content = match &m.tool_calls {
                    Some(calls) if !calls.is_empty() => {
                        NativeContent::Blocks(tool_calls_to_blocks(&m.content, calls))
                    }
                    _ => NativeContent::Text(m.content.clone()),
                };
                messages.push(NativeMessage {
                    role: m.role.clone(),
                    content,
                });
            }
        }
    }
    NativeRequest {
        model: req.model.clone(),
        max_tokens: req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        system: (!system_parts.is_empty()).then(|| system_parts.join("\n")),
        messages,
        temperature: req.temperature,
        top_p: req.top_p,
        tools: req
            .tools
            .as_ref()
            .map(|t| to_native_tools(t))
            .filter(|t| !t.is_empty()),
    }
}

fn from_native_response(model: String, resp: NativeResponse) -> ChatCompletionResponse {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    for block in resp.content {
        match block.block_type.as_str() {
            "text" => text.push_str(&block.text),
            "tool_use" => tool_calls.push(ToolCall {
                id: block.id.unwrap_or_default(),
                kind: "function".to_string(),
                function: ToolCallFunction {
                    name: block.name.unwrap_or_default(),
                    arguments: block
                        .input
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "{}".to_string()),
                },
            }),
            _ => {}
        }
    }

    ChatCompletionResponse {
        id: resp.id,
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
                tool_calls: (!tool_calls.is_empty()).then_some(tool_calls),
                tool_call_id: None,
                name: None,
            },
            finish_reason: resp.stop_reason,
        }],
        usage: Some(Usage {
            prompt_tokens: resp.usage.input_tokens,
            completion_tokens: resp.usage.output_tokens,
            total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
        }),
        fallback: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gateway_common::canonical::ChatMessage as CanonicalMessage;

    fn msg(role: &str, content: &str) -> CanonicalMessage {
        CanonicalMessage {
            role: role.to_string(),
            content: content.to_string(),
            ..Default::default()
        }
    }

    fn base_request(messages: Vec<CanonicalMessage>) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: None,
            tools: None,
            fallback_models: None,
        }
    }

    #[test]
    fn extracts_system_messages_into_top_level_field() {
        let req = base_request(vec![
            msg("system", "You are terse."),
            msg("user", "Hi"),
            msg("assistant", "Hello."),
        ]);

        let native = to_native_request(&req);
        assert_eq!(native.system, Some("You are terse.".to_string()));
        assert_eq!(
            native.messages,
            vec![
                NativeMessage {
                    role: "user".to_string(),
                    content: NativeContent::Text("Hi".to_string())
                },
                NativeMessage {
                    role: "assistant".to_string(),
                    content: NativeContent::Text("Hello.".to_string())
                },
            ]
        );
        // Anthropic requires max_tokens; canonical had none, so default fills in.
        assert_eq!(native.max_tokens, DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn joins_multiple_system_messages_and_preserves_explicit_max_tokens() {
        let mut req = base_request(vec![
            msg("system", "Rule one."),
            msg("system", "Rule two."),
            msg("user", "Go"),
        ]);
        req.max_tokens = Some(256);
        let native = to_native_request(&req);
        assert_eq!(native.system, Some("Rule one.\nRule two.".to_string()));
        assert_eq!(native.max_tokens, 256);
    }

    #[test]
    fn no_system_messages_yields_none() {
        let req = base_request(vec![msg("user", "Hi")]);
        assert_eq!(to_native_request(&req).system, None);
    }

    #[test]
    fn translates_native_response_back_to_canonical() {
        let native = NativeResponse {
            id: "msg_123".to_string(),
            content: vec![NativeContentBlock {
                block_type: "text".to_string(),
                text: "Hello there.".to_string(),
                ..Default::default()
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: NativeUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        };
        let canonical = from_native_response("claude-3-opus-20240229".to_string(), native);
        assert_eq!(canonical.id, "msg_123");
        assert_eq!(canonical.choices.len(), 1);
        assert_eq!(canonical.choices[0].message.role, "assistant");
        assert_eq!(canonical.choices[0].message.content, "Hello there.");
        assert_eq!(
            canonical.choices[0].finish_reason,
            Some("end_turn".to_string())
        );
        let usage = canonical.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn translates_tool_definitions_into_input_schema() {
        let mut req = base_request(vec![msg("user", "What's the weather?")]);
        req.tools = Some(vec![Tool {
            kind: "function".to_string(),
            function: gateway_common::canonical::ToolFunctionDef {
                name: "get_weather".to_string(),
                description: Some("Look up current weather".to_string()),
                parameters: serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}}),
            },
        }]);

        let native = to_native_request(&req);
        let tools = native.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_weather");
        assert_eq!(
            tools[0].input_schema["properties"]["city"]["type"],
            "string"
        );
    }

    #[test]
    fn assistant_tool_call_becomes_tool_use_block_and_tool_result_becomes_user_message() {
        let mut assistant = msg("assistant", "");
        assistant.tool_calls = Some(vec![ToolCall {
            id: "call_1".to_string(),
            kind: "function".to_string(),
            function: ToolCallFunction {
                name: "get_weather".to_string(),
                arguments: "{\"city\":\"NYC\"}".to_string(),
            },
        }]);
        let mut tool_result = msg("tool", "72F and sunny");
        tool_result.tool_call_id = Some("call_1".to_string());

        let req = base_request(vec![
            msg("user", "What's the weather in NYC?"),
            assistant,
            tool_result,
        ]);
        let native = to_native_request(&req);

        match &native.messages[1].content {
            NativeContent::Blocks(blocks) => {
                assert_eq!(
                    blocks,
                    &[NativeBlock::ToolUse {
                        id: "call_1".to_string(),
                        name: "get_weather".to_string(),
                        input: serde_json::json!({"city": "NYC"}),
                    }]
                );
            }
            other => panic!("expected tool_use blocks, got {other:?}"),
        }
        assert_eq!(native.messages[2].role, "user");
        match &native.messages[2].content {
            NativeContent::Blocks(blocks) => {
                assert_eq!(
                    blocks,
                    &[NativeBlock::ToolResult {
                        tool_use_id: "call_1".to_string(),
                        content: "72F and sunny".to_string()
                    }]
                );
            }
            other => panic!("expected tool_result block, got {other:?}"),
        }
    }

    #[test]
    fn tool_use_response_block_becomes_canonical_tool_call() {
        let native = NativeResponse {
            id: "msg_456".to_string(),
            content: vec![NativeContentBlock {
                block_type: "tool_use".to_string(),
                id: Some("call_2".to_string()),
                name: Some("get_weather".to_string()),
                input: Some(serde_json::json!({"city": "NYC"})),
                ..Default::default()
            }],
            stop_reason: Some("tool_use".to_string()),
            usage: NativeUsage {
                input_tokens: 20,
                output_tokens: 8,
            },
        };
        let canonical = from_native_response("claude-3-opus-20240229".to_string(), native);
        let tool_calls = canonical.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "get_weather");
        let args: serde_json::Value =
            serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
        assert_eq!(args["city"], "NYC");
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[test]
    fn text_deltas_map_to_content() {
        let chunk = map_event(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}"#,
        )
        .expect("text delta maps");
        assert_eq!(chunk.delta, "Hel");
    }

    #[test]
    fn input_and_output_tokens_arrive_on_separate_frames() {
        let start = map_event(
            "message_start",
            r#"{"message":{"id":"msg_1","model":"claude-sonnet-4","usage":{"input_tokens":11,"output_tokens":0}}}"#,
        )
        .expect("message_start maps");
        assert_eq!(start.usage.as_ref().unwrap().prompt_tokens, 11);
        assert_eq!(start.usage.unwrap().completion_tokens, 0);

        let end = map_event(
            "message_delta",
            r#"{"delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":23}}"#,
        )
        .expect("message_delta maps");
        assert_eq!(end.usage.unwrap().completion_tokens, 23);
        assert_eq!(end.finish_reason.as_deref(), Some("end_turn"));
    }

    #[test]
    fn bookkeeping_events_are_dropped() {
        assert!(map_event("ping", r#"{"type":"ping"}"#).is_none());
        assert!(map_event("message_stop", r#"{"type":"message_stop"}"#).is_none());
    }
}
