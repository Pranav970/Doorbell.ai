use uuid::Uuid;

use gateway_common::canonical::{
    ChatCompletionRequest, ChatCompletionResponse, FallbackAttemptSummary,
};
use gateway_providers::ChatStream;

use crate::context::ResolvedRoute;
use crate::error::CoreError;
use crate::pipeline::ProviderCaller;
use crate::routing::RoutingEngine;

/// A chat completion that succeeded, on `model_used` — which may be the
/// caller's primary `model` or one of its `fallback_models`.
#[derive(Debug)]
pub struct FallbackSuccess {
    pub response: ChatCompletionResponse,
    pub route: ResolvedRoute,
    pub model_used: String,
    /// How many prior candidates were actually attempted (and failed)
    /// before this one succeeded. 0 means the primary model worked first try.
    pub fallback_count: i32,
    /// What was tried, in order, before `model_used` — surfaced to the
    /// caller via `ChatCompletionResponse.fallback` so "what happened" is
    /// visible right on the response, not just in `request_logs`.
    pub failed_attempts: Vec<FallbackAttemptSummary>,
}

/// The streaming counterpart of `FallbackSuccess`. Carries the live chunk
/// stream of the one candidate that actually opened, plus the same
/// provenance fields so request logging is identical on both paths.
///
/// There is deliberately no `response` here: by the time this exists the
/// gateway is committed to this model, and everything else about the call
/// is only knowable once the stream drains.
pub struct StreamSuccess {
    pub stream: ChatStream,
    pub route: ResolvedRoute,
    pub model_used: String,
    pub fallback_count: i32,
    pub failed_attempts: Vec<FallbackAttemptSummary>,
}

/// Every candidate failed (or had no resolvable route). Carries enough of
/// the last attempt's identity for request logging even though the error
/// itself (`source`) only describes the final failure.
#[derive(Debug)]
pub struct FallbackFailure {
    pub attempts: i32,
    pub last_provider: Option<String>,
    pub last_model: Option<String>,
    pub failed_attempts: Vec<FallbackAttemptSummary>,
    pub source: CoreError,
}

impl From<FallbackFailure> for gateway_common::AppError {
    fn from(e: FallbackFailure) -> Self {
        let base: gateway_common::AppError = e.source.into();
        if e.failed_attempts.is_empty() {
            return base;
        }
        let trail = e
            .failed_attempts
            .iter()
            .map(|a| format!("{} (via {}) failed: {}", a.model, a.provider, a.error))
            .collect::<Vec<_>>()
            .join("; then ");
        match base {
            gateway_common::AppError::UpstreamError(msg) => {
                gateway_common::AppError::UpstreamError(format!("{trail}; then {msg}"))
            }
            gateway_common::AppError::BadRequest(msg) => {
                gateway_common::AppError::BadRequest(format!("{trail}; then {msg}"))
            }
            gateway_common::AppError::UpstreamTimeout => gateway_common::AppError::UpstreamError(
                format!("{trail}; then the final candidate timed out"),
            ),
            other => other,
        }
    }
}

/// Wraps `RoutingEngine` + `ProviderCaller` with the retry-on-failure
/// behavior the gateway's chat-completions handler needs: try the primary
/// model, and on a retryable error (see `ProviderError::is_fallback_trigger`)
/// walk `request.fallback_models` in order until one succeeds or the list
/// runs out. Each candidate is routed independently, so a fallback can land
/// on an entirely different provider than the primary.
#[derive(Clone)]
pub struct FallbackEngine {
    routing: RoutingEngine,
    caller: ProviderCaller,
}

impl FallbackEngine {
    pub fn new(routing: RoutingEngine, caller: ProviderCaller) -> Self {
        Self { routing, caller }
    }

    pub async fn call(
        &self,
        team_id: Uuid,
        virtual_key_id: Uuid,
        request: &ChatCompletionRequest,
    ) -> Result<FallbackSuccess, FallbackFailure> {
        let mut candidates = vec![request.model.clone()];
        if let Some(fallbacks) = &request.fallback_models {
            candidates.extend(fallbacks.iter().cloned());
        }
        let last_index = candidates.len().saturating_sub(1);

        let mut attempts = 0i32;
        let mut last_err = None;
        let mut last_provider = None;
        let mut last_model = None;
        let mut failed_attempts = Vec::new();

        for (i, model) in candidates.iter().enumerate() {
            // A candidate with no route for this team (e.g. a fallback model
            // naming a provider the team never added a key for) is a config
            // gap, not a provider failure — skip it rather than aborting the
            // whole chain.
            let route = match self.routing.resolve(team_id, virtual_key_id, model).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(e);
                    last_model = Some(model.clone());
                    continue;
                }
            };

            attempts += 1;
            last_provider = Some(route.provider_name.clone());
            last_model = Some(model.clone());

            // fallback_models is a gateway-only extension, not part of any
            // vendor's wire format — strip it before it reaches an adapter.
            let mut attempt = request.clone();
            attempt.model = model.clone();
            attempt.fallback_models = None;

            match self.caller.call(&route, &attempt).await {
                Ok(response) => {
                    return Ok(FallbackSuccess {
                        response,
                        route,
                        model_used: model.clone(),
                        fallback_count: attempts - 1,
                        failed_attempts,
                    });
                }
                Err(CoreError::Provider(pe)) if pe.is_fallback_trigger() && i < last_index => {
                    failed_attempts.push(FallbackAttemptSummary {
                        model: model.clone(),
                        provider: route.provider_name.clone(),
                        error: pe.to_string(),
                    });
                    last_err = Some(CoreError::Provider(pe));
                }
                Err(e) => {
                    return Err(FallbackFailure {
                        attempts,
                        last_provider,
                        last_model,
                        failed_attempts,
                        source: e,
                    });
                }
            }
        }

        Err(FallbackFailure {
            attempts,
            last_provider,
            last_model,
            failed_attempts,
            source: last_err.unwrap_or(CoreError::NoRoute),
        })
    }

    /// Streaming failover, with the same candidate walk as `call` and one
    /// extra guarantee that matters: **failover only ever happens before a
    /// single byte reaches the client.**
    ///
    /// `ProviderCaller::call_stream` returns `Err` only while opening the
    /// upstream stream — connection refused, bad key, 429, 5xx — all of
    /// which happen before any chunk exists. The moment it returns `Ok`,
    /// this loop returns too, and the caller streams that one provider
    /// straight through. A failure *during* generation surfaces as an error
    /// item inside the stream, not as a retry, because half a response has
    /// already been written and re-running the prompt on another model would
    /// duplicate it.
    ///
    /// That is also why token metering attaches downstream of this: only the
    /// surviving stream is ever counted, so a request that failed over twice
    /// bills for the model that actually generated, not for the attempts.
    pub async fn call_stream(
        &self,
        team_id: Uuid,
        virtual_key_id: Uuid,
        request: &ChatCompletionRequest,
    ) -> Result<StreamSuccess, FallbackFailure> {
        let mut candidates = vec![request.model.clone()];
        if let Some(fallbacks) = &request.fallback_models {
            candidates.extend(fallbacks.iter().cloned());
        }
        let last_index = candidates.len().saturating_sub(1);

        let mut attempts = 0i32;
        let mut last_err = None;
        let mut last_provider = None;
        let mut last_model = None;
        let mut failed_attempts = Vec::new();

        for (i, model) in candidates.iter().enumerate() {
            let route = match self.routing.resolve(team_id, virtual_key_id, model).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(e);
                    last_model = Some(model.clone());
                    continue;
                }
            };

            attempts += 1;
            last_provider = Some(route.provider_name.clone());
            last_model = Some(model.clone());

            let mut attempt = request.clone();
            attempt.model = model.clone();
            attempt.fallback_models = None;

            match self.caller.call_stream(&route, &attempt).await {
                Ok(stream) => {
                    return Ok(StreamSuccess {
                        stream,
                        route,
                        model_used: model.clone(),
                        fallback_count: attempts - 1,
                        failed_attempts,
                    });
                }
                Err(CoreError::Provider(pe)) if pe.is_fallback_trigger() && i < last_index => {
                    failed_attempts.push(FallbackAttemptSummary {
                        model: model.clone(),
                        provider: route.provider_name.clone(),
                        error: pe.to_string(),
                    });
                    last_err = Some(CoreError::Provider(pe));
                }
                Err(e) => {
                    return Err(FallbackFailure {
                        attempts,
                        last_provider,
                        last_model,
                        failed_attempts,
                        source: e,
                    });
                }
            }
        }

        Err(FallbackFailure {
            attempts,
            last_provider,
            last_model,
            failed_attempts,
            source: last_err.unwrap_or(CoreError::NoRoute),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use gateway_common::canonical::ChatMessage;
    use gateway_crypto::{EnvKeyCipher, KeyEnvelopeCipher};
    use gateway_db::{
        OrgRepo, ProviderCredentialRepo, RoutingRuleRepo, TeamRepo, UserRepo, VirtualKeyRepo,
    };
    use gateway_providers::{ProviderEndpointsConfig, ProviderRegistry};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_cipher() -> EnvKeyCipher {
        EnvKeyCipher::from_base64(&STANDARD.encode([9u8; 32])).unwrap()
    }

    /// Encrypts a fake provider API key with the test cipher and stores it —
    /// `ProviderCaller` decrypts on every call, so the fixture needs a real
    /// ciphertext/nonce pair rather than arbitrary bytes.
    async fn store_credential(
        creds: &ProviderCredentialRepo,
        cipher: &EnvKeyCipher,
        team_id: Uuid,
        user_id: Uuid,
        provider: &str,
    ) -> gateway_db::models::ProviderCredential {
        let blob = cipher.encrypt("fake-api-key").unwrap();
        creds
            .create(
                team_id,
                user_id,
                provider,
                &blob.ciphertext,
                &blob.nonce,
                blob.key_version,
                "aaaa",
                None,
                None,
                true,
            )
            .await
            .unwrap()
    }

    /// Routing is configuration-driven now: with more than one credential in
    /// play, every model a test expects to resolve needs a rule saying so.
    async fn add_rule(pool: &sqlx::PgPool, vk_id: Uuid, pattern: &str, cred_id: Uuid) {
        sqlx::query!(
            "INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
             VALUES ($1, $2, $3, 0)",
            vk_id,
            pattern,
            cred_id
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn setup_team(pool: &sqlx::PgPool, email: &str) -> (Uuid, Uuid) {
        let users = UserRepo::new(pool.clone());
        let orgs = OrgRepo::new(pool.clone());
        let teams = TeamRepo::new(pool.clone());
        let user = users.create(email, "hash", None).await.unwrap();
        let org = orgs
            .create("Test Org", &format!("org-{email}"), user.id)
            .await
            .unwrap();
        let team = teams
            .create(org.id, "Test Team", &format!("team-{email}"), user.id)
            .await
            .unwrap();
        (user.id, team.id)
    }

    fn chat_request(model: &str, fallback_models: Option<Vec<String>>) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
                ..Default::default()
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stream: None,
            tools: None,
            fallback_models,
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn falls_back_to_next_model_on_a_5xx(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "fallback1@example.com").await;
        let creds = ProviderCredentialRepo::new(pool.clone());
        let vkeys = VirtualKeyRepo::new(pool.clone());

        let openai_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(503).set_body_string("upstream overloaded"))
            .mount(&openai_mock)
            .await;

        let anthropic_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_ok",
                "content": [{"type": "text", "text": "recovered"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&anthropic_mock)
            .await;

        let cipher = test_cipher();
        let openai = store_credential(&creds, &cipher, team_id, user_id, "openai").await;
        let anthropic = store_credential(&creds, &cipher, team_id, user_id, "anthropic").await;
        let vk = vkeys
            .create(team_id, user_id, "vkhash1", "vk_live_A", None, None)
            .await
            .unwrap();
        add_rule(&pool, vk.id, "gpt-4o", openai.id).await;
        add_rule(&pool, vk.id, "claude-3-opus-20240229", anthropic.id).await;

        let registry = Arc::new(ProviderRegistry::new(&ProviderEndpointsConfig {
            openai_base_url: openai_mock.uri(),
            anthropic_base_url: anthropic_mock.uri(),
            anthropic_api_version: "2023-06-01".to_string(),
            gemini_base_url: "http://localhost:0".to_string(),
        }));
        let engine = FallbackEngine::new(
            RoutingEngine::new(RoutingRuleRepo::new(pool.clone()), creds.clone()),
            ProviderCaller::new(creds, Arc::new(cipher), registry),
        );

        let req = chat_request("gpt-4o", Some(vec!["claude-3-opus-20240229".to_string()]));
        let result = engine.call(team_id, vk.id, &req).await.unwrap();
        assert_eq!(result.model_used, "claude-3-opus-20240229");
        assert_eq!(result.fallback_count, 1);
        assert_eq!(result.response.choices[0].message.content, "recovered");

        // "What happened" is captured for the response body, not just logs.
        assert_eq!(result.failed_attempts.len(), 1);
        assert_eq!(result.failed_attempts[0].model, "gpt-4o");
        assert_eq!(result.failed_attempts[0].provider, "openai");
        assert!(
            result.failed_attempts[0].error.contains("503")
                || result.failed_attempts[0].error.contains("upstream")
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_404_on_the_primary_triggers_fallback(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "fallback404@example.com").await;
        let creds = ProviderCredentialRepo::new(pool.clone());
        let vkeys = VirtualKeyRepo::new(pool.clone());

        // Simulates an unreachable/unknown model on the primary provider —
        // Gemini (and most vendors) return 404 for a model name that isn't
        // valid/enabled for the key, which is exactly the case a caller
        // would want a fallback for.
        let gemini_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404).set_body_string("model not found"))
            .mount(&gemini_mock)
            .await;

        let mistral_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-mistral-ok",
                "object": "chat.completion",
                "created": 1_700_000_000,
                "model": "mistral-small-latest",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "hi from mistral"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&mistral_mock)
            .await;

        let cipher = test_cipher();
        let gemini = store_credential(&creds, &cipher, team_id, user_id, "gemini").await;
        // "mistral" has no adapter — reached generically via base_url, same
        // as any other BYOK/custom vendor.
        let mistral_blob = cipher.encrypt("fake-mistral-key").unwrap();
        let mistral_base_url = mistral_mock.uri();
        let mistral_cred = creds
            .create(
                team_id,
                user_id,
                "mistral",
                &mistral_blob.ciphertext,
                &mistral_blob.nonce,
                mistral_blob.key_version,
                "hhhh",
                None,
                Some(mistral_base_url.as_str()),
                true,
            )
            .await
            .unwrap();
        let vk = vkeys
            .create(team_id, user_id, "vkhash6", "vk_live_F", None, None)
            .await
            .unwrap();

        add_rule(&pool, vk.id, "gemini-1.5-flash", gemini.id).await;

        // Two credentials are active (gemini + mistral) and "mistral-*" has
        // no entry in the default prefix map, so — same as
        // `routing_rule_beats_default_prefix_map` — this is only
        // resolvable with an explicit routing_rules row, not the
        // sole-active-credential shortcut.
        sqlx::query!(
            "INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
             VALUES ($1, 'mistral-*', $2, 0)",
            vk.id,
            mistral_cred.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let registry = Arc::new(ProviderRegistry::new(&ProviderEndpointsConfig {
            openai_base_url: "http://localhost:0".to_string(),
            anthropic_base_url: "http://localhost:0".to_string(),
            anthropic_api_version: "2023-06-01".to_string(),
            gemini_base_url: gemini_mock.uri(),
        }));
        let engine = FallbackEngine::new(
            RoutingEngine::new(RoutingRuleRepo::new(pool.clone()), creds.clone()),
            ProviderCaller::new(creds, Arc::new(cipher), registry),
        );

        let req = chat_request(
            "gemini-1.5-flash",
            Some(vec!["mistral-small-latest".to_string()]),
        );
        let result = engine.call(team_id, vk.id, &req).await.unwrap();
        assert_eq!(result.model_used, "mistral-small-latest");
        assert_eq!(result.route.provider_credential_id, mistral_cred.id);
        assert_eq!(
            result.response.choices[0].message.content,
            "hi from mistral"
        );
        assert_eq!(result.failed_attempts[0].model, "gemini-1.5-flash");
        assert!(result.failed_attempts[0].error.contains("404"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn does_not_fall_back_on_a_non_retryable_error(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "fallback2@example.com").await;
        let creds = ProviderCredentialRepo::new(pool.clone());
        let vkeys = VirtualKeyRepo::new(pool.clone());

        let openai_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
            .expect(1)
            .mount(&openai_mock)
            .await;

        let cipher = test_cipher();
        let openai = store_credential(&creds, &cipher, team_id, user_id, "openai").await;
        let anthropic = store_credential(&creds, &cipher, team_id, user_id, "anthropic").await;
        let vk = vkeys
            .create(team_id, user_id, "vkhash2", "vk_live_B", None, None)
            .await
            .unwrap();
        add_rule(&pool, vk.id, "gpt-4o", openai.id).await;
        add_rule(&pool, vk.id, "claude-3-opus-20240229", anthropic.id).await;

        let registry = Arc::new(ProviderRegistry::new(&ProviderEndpointsConfig {
            openai_base_url: openai_mock.uri(),
            anthropic_base_url: "http://localhost:0".to_string(),
            anthropic_api_version: "2023-06-01".to_string(),
            gemini_base_url: "http://localhost:0".to_string(),
        }));
        let engine = FallbackEngine::new(
            RoutingEngine::new(RoutingRuleRepo::new(pool.clone()), creds.clone()),
            ProviderCaller::new(creds, Arc::new(cipher), registry),
        );

        let req = chat_request("gpt-4o", Some(vec!["claude-3-opus-20240229".to_string()]));
        let err = engine.call(team_id, vk.id, &req).await.unwrap_err();
        assert_eq!(err.attempts, 1);
        assert!(matches!(
            err.source,
            CoreError::Provider(gateway_providers::ProviderError::Unauthorized)
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn skips_a_fallback_model_with_no_configured_route(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "fallback3@example.com").await;
        let creds = ProviderCredentialRepo::new(pool.clone());
        let vkeys = VirtualKeyRepo::new(pool.clone());

        let openai_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
            .mount(&openai_mock)
            .await;

        // Two credentials (openai, anthropic) exist but no gemini one — with
        // more than one candidate, `RoutingEngine` has nothing to
        // disambiguate "gemini-1.5-pro" with (no rule, no prefix match, and
        // it's not the team's sole credential), so that fallback candidate
        // has no route and is skipped.
        let cipher = test_cipher();
        let openai = store_credential(&creds, &cipher, team_id, user_id, "openai").await;
        store_credential(&creds, &cipher, team_id, user_id, "anthropic").await;
        let vk = vkeys
            .create(team_id, user_id, "vkhash3", "vk_live_C", None, None)
            .await
            .unwrap();
        // Only the primary is routable; "gemini-1.5-pro" has no rule and no
        // credential, so it is skipped.
        add_rule(&pool, vk.id, "gpt-4o", openai.id).await;

        let registry = Arc::new(ProviderRegistry::new(&ProviderEndpointsConfig {
            openai_base_url: openai_mock.uri(),
            anthropic_base_url: "http://localhost:0".to_string(),
            anthropic_api_version: "2023-06-01".to_string(),
            gemini_base_url: "http://localhost:0".to_string(),
        }));
        let engine = FallbackEngine::new(
            RoutingEngine::new(RoutingRuleRepo::new(pool.clone()), creds.clone()),
            ProviderCaller::new(creds, Arc::new(cipher), registry),
        );

        let req = chat_request("gpt-4o", Some(vec!["gemini-1.5-pro".to_string()]));
        let err = engine.call(team_id, vk.id, &req).await.unwrap_err();
        // The only real attempt was the openai one (429); gemini never had a
        // credential to route to.
        assert_eq!(err.attempts, 1);
        assert_eq!(err.last_provider, Some("openai".to_string()));
    }
}
