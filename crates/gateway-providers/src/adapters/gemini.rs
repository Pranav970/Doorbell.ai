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

pub struct GeminiProvider {
    http: reqwest::Client,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn chat_completion(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        // Gemini authenticates via a `key` query param, not a header — unlike
        // OpenAI/Anthropic. The trait passes `api_key` as a plain value so
        // each adapter can decide how to inject it.
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            request.model
        );
        let native_request = to_native_request(request);

        let response = self
            .http
            .post(&url)
            .query(&[("key", api_key.expose_secret())])
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

        Ok(from_native_response(
            uuid::Uuid::new_v4().to_string(),
            request.model.clone(),
            native_response,
        ))
    }

    async fn chat_completion_stream(
        &self,
        api_key: &SecretString,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream, ProviderError> {
        // `alt=sse` is what makes Gemini frame its stream as SSE at all —
        // without it `streamGenerateContent` returns a JSON array, which no
        // SSE decoder can read incrementally.
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent",
            self.base_url.trim_end_matches('/'),
            request.model
        );
        let native_request = to_native_request(request);

        let response = self
            .http
            .post(&url)
            .query(&[("key", api_key.expose_secret().as_str()), ("alt", "sse")])
            .json(&native_request)
            .send()
            .await
            .map_err(map_transport_error)?;

        if !response.status().is_success() {
            return Err(sse::error_from_response(response).await);
        }

        Ok(sse::decode(response.bytes_stream(), map_event))
    }
}

/// Every Gemini SSE frame is a whole `GenerateContentResponse`, not a delta
/// envelope — so `usageMetadata` may repeat on each frame with cumulative
/// counts. The meter takes the last reported value, which is what makes that
/// safe.
fn map_event(_event: &str, data: &str) -> Option<ChatCompletionChunk> {
    let value: Value = serde_json::from_str(data).ok()?;

    let candidate = value.get("candidates").and_then(|c| c.get(0));
    let delta = candidate
        .and_then(|c| c.pointer("/content/parts"))
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .unwrap_or_default();
    // Passed through verbatim ("STOP", not "stop") to match what the
    // non-streaming adapter above already returns for this provider.
    let finish_reason = candidate
        .and_then(|c| c.get("finishReason"))
        .and_then(|f| f.as_str())
        .map(str::to_string);

    let usage = value.pointer("/usageMetadata").and_then(|u| {
        let completion = u
            .get("candidatesTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        let prompt = u
            .get("promptTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        (completion > 0 || prompt > 0).then_some(Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: u
                .get("totalTokenCount")
                .and_then(|t| t.as_u64())
                .unwrap_or((prompt + completion) as u64) as u32,
        })
    });

    if delta.is_empty() && finish_reason.is_none() && usage.is_none() {
        return None;
    }

    Some(ChatCompletionChunk {
        id: String::new(),
        model: value
            .get("modelVersion")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_string(),
        delta,
        finish_reason,
        usage,
    })
}

#[derive(Debug, Serialize)]
struct NativeRequest {
    contents: Vec<NativeContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<NativeContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<NativeGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<NativeToolWrapper>>,
}

#[derive(Debug, Serialize)]
struct NativeToolWrapper {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<NativeFunctionDecl>,
}

#[derive(Debug, Serialize)]
struct NativeFunctionDecl {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NativeContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<NativePart>,
}

/// A part is a single-key union in Gemini's wire format (`text`, or
/// `functionCall`, or `functionResponse`) — modeled here as one struct with
/// the other two skipped on serialize/left `None` on deserialize, rather
/// than a tagged enum, since none of the three keys share a discriminant.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct NativePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<NativeFunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<NativeFunctionResponse>,
}

impl NativePart {
    fn text(s: impl Into<String>) -> Self {
        Self {
            text: Some(s.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NativeFunctionCall {
    name: String,
    args: Value,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct NativeFunctionResponse {
    name: String,
    response: Value,
}

#[derive(Debug, Serialize)]
struct NativeGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "topP", skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct NativeResponse {
    #[serde(default)]
    candidates: Vec<NativeCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<NativeUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct NativeCandidate {
    content: NativeContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NativeUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

fn to_native_tools(tools: &[Tool]) -> Vec<NativeToolWrapper> {
    vec![NativeToolWrapper {
        function_declarations: tools
            .iter()
            .map(|t| NativeFunctionDecl {
                name: t.function.name.clone(),
                description: t.function.description.clone(),
                parameters: t.function.parameters.clone(),
            })
            .collect(),
    }]
}

/// Remaps canonical roles onto Gemini's vocabulary: "assistant" -> "model",
/// "tool" -> "function" (carrying a `functionResponse` part instead of
/// text), everything else (i.e. "user") stays "user". "system" is pulled out
/// into the dedicated `systemInstruction` field rather than `contents`.
fn to_native_request(req: &ChatCompletionRequest) -> NativeRequest {
    let mut contents = Vec::new();
    let mut system_parts = Vec::new();
    for m in &req.messages {
        match m.role.as_str() {
            "system" => system_parts.push(m.content.clone()),
            "assistant" if m.tool_calls.as_ref().is_some_and(|c| !c.is_empty()) => {
                let parts = m
                    .tool_calls
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|call| {
                        let args = serde_json::from_str(&call.function.arguments)
                            .unwrap_or(Value::Object(Default::default()));
                        NativePart {
                            function_call: Some(NativeFunctionCall {
                                name: call.function.name.clone(),
                                args,
                            }),
                            ..Default::default()
                        }
                    })
                    .collect();
                contents.push(NativeContent {
                    role: Some("model".to_string()),
                    parts,
                });
            }
            "assistant" => contents.push(NativeContent {
                role: Some("model".to_string()),
                parts: vec![NativePart::text(m.content.clone())],
            }),
            // Gemini has no "tool" role — a tool result is a `function`
            // content carrying a `functionResponse` part instead.
            "tool" => {
                let name = m
                    .name
                    .clone()
                    .unwrap_or_else(|| m.tool_call_id.clone().unwrap_or_default());
                contents.push(NativeContent {
                    role: Some("function".to_string()),
                    parts: vec![NativePart {
                        function_response: Some(NativeFunctionResponse {
                            name,
                            response: serde_json::json!({ "content": m.content }),
                        }),
                        ..Default::default()
                    }],
                });
            }
            _ => contents.push(NativeContent {
                role: Some("user".to_string()),
                parts: vec![NativePart::text(m.content.clone())],
            }),
        }
    }

    let system_instruction = (!system_parts.is_empty()).then(|| NativeContent {
        role: None,
        parts: vec![NativePart::text(system_parts.join("\n"))],
    });

    let generation_config =
        if req.temperature.is_some() || req.top_p.is_some() || req.max_tokens.is_some() {
            Some(NativeGenerationConfig {
                temperature: req.temperature,
                top_p: req.top_p,
                max_output_tokens: req.max_tokens,
            })
        } else {
            None
        };

    NativeRequest {
        contents,
        system_instruction,
        generation_config,
        tools: req
            .tools
            .as_ref()
            .map(|t| to_native_tools(t))
            .filter(|t| !t.is_empty()),
    }
}

fn from_native_response(id: String, model: String, resp: NativeResponse) -> ChatCompletionResponse {
    let candidate = resp.candidates.into_iter().next();
    let (text, tool_calls, finish_reason) = match candidate {
        Some(c) => {
            let mut text = String::new();
            let mut tool_calls = Vec::new();
            for (i, part) in c.content.parts.into_iter().enumerate() {
                if let Some(t) = part.text {
                    text.push_str(&t);
                } else if let Some(call) = part.function_call {
                    tool_calls.push(ToolCall {
                        id: format!("call_{i}"),
                        kind: "function".to_string(),
                        function: ToolCallFunction {
                            name: call.name,
                            arguments: call.args.to_string(),
                        },
                    });
                }
            }
            (text, tool_calls, c.finish_reason)
        }
        None => (String::new(), Vec::new(), None),
    };

    let usage = resp.usage_metadata.map(|u| Usage {
        prompt_tokens: u.prompt_token_count,
        completion_tokens: u.candidates_token_count,
        total_tokens: u.total_token_count,
    });

    ChatCompletionResponse {
        id,
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
            finish_reason,
        }],
        usage,
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
            model: "gemini-1.5-pro".to_string(),
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
    fn remaps_assistant_to_model_and_extracts_system_instruction() {
        let req = base_request(vec![
            msg("system", "Be terse."),
            msg("user", "Hi"),
            msg("assistant", "Hello."),
        ]);

        let native = to_native_request(&req);
        assert_eq!(
            native.system_instruction,
            Some(NativeContent {
                role: None,
                parts: vec![NativePart::text("Be terse.")]
            })
        );
        assert_eq!(
            native.contents,
            vec![
                NativeContent {
                    role: Some("user".to_string()),
                    parts: vec![NativePart::text("Hi")]
                },
                NativeContent {
                    role: Some("model".to_string()),
                    parts: vec![NativePart::text("Hello.")]
                },
            ]
        );
    }

    #[test]
    fn no_system_or_generation_params_omits_optional_fields() {
        let req = base_request(vec![msg("user", "Hi")]);
        let native = to_native_request(&req);
        assert!(native.system_instruction.is_none());
        assert!(native.generation_config.is_none());
    }

    #[test]
    fn translates_native_response_back_to_canonical() {
        let native = NativeResponse {
            candidates: vec![NativeCandidate {
                content: NativeContent {
                    role: Some("model".to_string()),
                    parts: vec![NativePart::text("Hello there.")],
                },
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: Some(NativeUsageMetadata {
                prompt_token_count: 8,
                candidates_token_count: 4,
                total_token_count: 12,
            }),
        };
        let canonical =
            from_native_response("req-1".to_string(), "gemini-1.5-pro".to_string(), native);
        assert_eq!(canonical.choices[0].message.role, "assistant");
        assert_eq!(canonical.choices[0].message.content, "Hello there.");
        assert_eq!(canonical.choices[0].finish_reason, Some("STOP".to_string()));
        let usage = canonical.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 8);
        assert_eq!(usage.completion_tokens, 4);
        assert_eq!(usage.total_tokens, 12);
    }

    #[test]
    fn tool_definitions_become_function_declarations() {
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
        let decls = &native.tools.unwrap()[0].function_declarations;
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "get_weather");
        assert_eq!(decls[0].parameters["properties"]["city"]["type"], "string");
    }

    #[test]
    fn assistant_tool_call_becomes_function_call_part_and_tool_result_becomes_function_content() {
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
        tool_result.name = Some("get_weather".to_string());

        let req = base_request(vec![msg("user", "Weather in NYC?"), assistant, tool_result]);
        let native = to_native_request(&req);

        assert_eq!(native.contents[1].role, Some("model".to_string()));
        let call = native.contents[1].parts[0].function_call.as_ref().unwrap();
        assert_eq!(call.name, "get_weather");
        assert_eq!(call.args["city"], "NYC");

        assert_eq!(native.contents[2].role, Some("function".to_string()));
        let response = native.contents[2].parts[0]
            .function_response
            .as_ref()
            .unwrap();
        assert_eq!(response.name, "get_weather");
        assert_eq!(response.response["content"], "72F and sunny");
    }

    #[test]
    fn function_call_response_part_becomes_canonical_tool_call() {
        let native = NativeResponse {
            candidates: vec![NativeCandidate {
                content: NativeContent {
                    role: Some("model".to_string()),
                    parts: vec![NativePart {
                        function_call: Some(NativeFunctionCall {
                            name: "get_weather".to_string(),
                            args: serde_json::json!({"city": "NYC"}),
                        }),
                        ..Default::default()
                    }],
                },
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
        };
        let canonical =
            from_native_response("req-2".to_string(), "gemini-1.5-pro".to_string(), native);
        let tool_calls = canonical.choices[0].message.tool_calls.as_ref().unwrap();
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
    fn concatenates_text_parts_and_reads_usage_metadata() {
        let chunk = map_event(
            "",
            r#"{"candidates":[{"content":{"parts":[{"text":"Hello "},{"text":"there"}],"role":"model"}}],"usageMetadata":{"promptTokenCount":8,"candidatesTokenCount":4,"totalTokenCount":12}}"#,
        )
        .expect("gemini frame maps");
        assert_eq!(chunk.delta, "Hello there");
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 8);
        assert_eq!(usage.completion_tokens, 4);
    }

    #[test]
    fn keeps_the_vendors_own_finish_reason_casing() {
        let chunk = map_event(
            "",
            r#"{"candidates":[{"content":{"parts":[{"text":"x"}]},"finishReason":"STOP"}]}"#,
        )
        .expect("finish frame maps");
        assert_eq!(chunk.finish_reason.as_deref(), Some("STOP"));
    }

    #[test]
    fn a_frame_with_neither_text_nor_counts_is_dropped() {
        assert!(map_event("", r#"{"candidates":[]}"#).is_none());
    }
}
