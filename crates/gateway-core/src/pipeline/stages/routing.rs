use async_trait::async_trait;

use crate::error::CoreError;
use crate::pipeline::stage::{FailureMode, Outcome, RequestCtx, ResponseCtx, Stage};
use crate::routing::RoutingEngine;

/// Restores the "is this request routable at all" check that used to be
/// `gateway-api`'s `routing_layer` middleware (deleted when routing moved
/// per-attempt into `FallbackEngine` — see `fallback.rs`). Resolves every
/// candidate model (primary, then `fallback_models` in order) exactly the
/// way `FallbackEngine`'s loop does, and only rejects if every one of them
/// fails — the same condition that ends in a rejection today. Any
/// candidate that resolves lets the request through; `FallbackEngine`
/// re-resolves each attempt independently as it always has, so this is a
/// deliberate duplicate query for now — it goes away once the provider call
/// itself becomes a stage in a later epoch.
pub struct RoutingStage {
    routing: RoutingEngine,
}

impl RoutingStage {
    pub fn new(routing: RoutingEngine) -> Self {
        Self { routing }
    }
}

#[async_trait]
impl Stage for RoutingStage {
    fn name(&self) -> &'static str {
        "routing"
    }

    fn failure_mode(&self) -> FailureMode {
        FailureMode::FailClosed
    }

    async fn on_request(&self, ctx: &mut RequestCtx) -> Outcome {
        let Some(request) = ctx.request.as_ref() else {
            // Body didn't parse — defer to the handler's own extractor,
            // which will report the same rejection it does today.
            return Outcome::Continue;
        };

        // `stream: true` used to be rejected right here. It isn't any more —
        // the handler serves it over SSE via `FallbackEngine::call_stream`.
        // Routability is checked identically for both, so a streaming
        // request with no usable credential still fails fast below rather
        // than after response headers are already on the wire.

        let Some(tenant) = ctx.tenant.as_ref() else {
            return Outcome::Fail(anyhow::anyhow!(
                "RoutingStage ran before AuthenticateStage resolved a tenant"
            ));
        };

        let mut candidates = vec![request.model.clone()];
        if let Some(fallbacks) = &request.fallback_models {
            candidates.extend(fallbacks.iter().cloned());
        }
        let team_id = tenant.team_id;
        let virtual_key_id = tenant.virtual_key_id;

        // Same semantics as FallbackEngine's loop: any error (including a
        // real repo failure, not just "no match") just moves on to the next
        // candidate, and the last error is what surfaces if none resolve.
        let mut last_err = CoreError::NoRoute;
        for model in &candidates {
            match self.routing.resolve(team_id, virtual_key_id, model).await {
                Ok(_) => return Outcome::Continue,
                Err(e) => last_err = e,
            }
        }
        Outcome::Reject(last_err.into())
    }

    async fn on_response(&self, _ctx: &mut ResponseCtx) -> Outcome {
        Outcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use gateway_common::canonical::{ChatCompletionRequest, ChatMessage};
    use gateway_common::AppError;
    use gateway_db::{ProviderCredentialRepo, RoutingRuleRepo};

    use super::*;
    use crate::context::ResolvedVirtualKey;

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

    fn ctx_for(tenant: ResolvedVirtualKey, request: ChatCompletionRequest) -> RequestCtx {
        let mut ctx = RequestCtx::new("req-1".to_string(), Some(request));
        ctx.tenant = Some(tenant);
        ctx
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_streaming_request_is_routed_like_any_other(pool: sqlx::PgPool) {
        // Guards the removal of the old `stream: true` rejection. A stream
        // with a usable credential must now reach the handler; the identical
        // check for a non-streaming request is what the test below covers.
        let team = gateway_testkit::setup_team(&pool, "routing-stage-stream@example.com").await;
        gateway_testkit::create_provider_credential(
            &pool,
            &gateway_testkit::test_cipher(),
            team.team_id,
            team.user_id,
            "openai",
            "sk-test-stream",
            None,
        )
        .await;

        let stage = RoutingStage::new(RoutingEngine::new(
            RoutingRuleRepo::new(pool.clone()),
            ProviderCredentialRepo::new(pool),
        ));
        let tenant = ResolvedVirtualKey {
            virtual_key_id: uuid::Uuid::new_v4(),
            team_id: team.team_id,
            issued_by: team.user_id,
        };

        let mut request = chat_request("gpt-4o", None);
        request.stream = Some(true);
        let mut ctx = ctx_for(tenant, request);

        assert!(matches!(
            stage.on_request(&mut ctx).await,
            Outcome::Continue
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn an_unroutable_streaming_request_is_still_rejected_before_any_bytes(
        pool: sqlx::PgPool,
    ) {
        // The routability pre-check still applies to streams, and matters
        // more here: once SSE headers are flushed there is no way to send a
        // clean HTTP error, so this has to fail now or not at all.
        let team = gateway_testkit::setup_team(&pool, "routing-stage-stream-2@example.com").await;
        let stage = RoutingStage::new(RoutingEngine::new(
            RoutingRuleRepo::new(pool.clone()),
            ProviderCredentialRepo::new(pool),
        ));
        let tenant = ResolvedVirtualKey {
            virtual_key_id: uuid::Uuid::new_v4(),
            team_id: team.team_id,
            issued_by: team.user_id,
        };

        let mut request = chat_request("gpt-4o", None);
        request.stream = Some(true);
        let mut ctx = ctx_for(tenant, request);

        // Rejected for having no route — never for being a stream.
        assert!(matches!(
            stage.on_request(&mut ctx).await,
            Outcome::Reject(AppError::BadRequest(msg))
                if msg == "no route found for the requested model"
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn rejects_when_no_candidate_model_resolves(pool: sqlx::PgPool) {
        let team = gateway_testkit::setup_team(&pool, "routing-stage-2@example.com").await;
        // No provider credentials at all for this team.
        let stage = RoutingStage::new(RoutingEngine::new(
            RoutingRuleRepo::new(pool.clone()),
            ProviderCredentialRepo::new(pool),
        ));
        let tenant = ResolvedVirtualKey {
            virtual_key_id: uuid::Uuid::new_v4(),
            team_id: team.team_id,
            issued_by: team.user_id,
        };
        let mut ctx = ctx_for(tenant, chat_request("gpt-4o", None));

        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(
            outcome,
            Outcome::Reject(AppError::BadRequest(msg)) if msg == "no route found for the requested model"
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn continues_when_only_a_fallback_candidate_resolves(pool: sqlx::PgPool) {
        let team = gateway_testkit::setup_team(&pool, "routing-stage-3@example.com").await;
        let creds = ProviderCredentialRepo::new(pool.clone());
        let rules = RoutingRuleRepo::new(pool.clone());
        let cipher = gateway_testkit::test_cipher();

        // Two credentials (so the sole-active-credential shortcut doesn't
        // apply) and no default-prefix match for either model — the
        // primary is genuinely unroutable without an explicit rule.
        gateway_testkit::create_provider_credential(
            &pool,
            &cipher,
            team.team_id,
            team.user_id,
            "openai",
            "sk-fake-openai",
            None,
        )
        .await;
        let mistral_cred = gateway_testkit::create_provider_credential(
            &pool,
            &cipher,
            team.team_id,
            team.user_id,
            "mistral",
            "sk-fake-mistral",
            None,
        )
        .await;
        let vk = gateway_testkit::create_virtual_key(&pool, team.team_id, team.user_id, None).await;
        sqlx::query!(
            "INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
             VALUES ($1, 'mistral-small-latest', $2, 0)",
            vk.row.id,
            mistral_cred.id,
        )
        .execute(&pool)
        .await
        .unwrap();

        let stage = RoutingStage::new(RoutingEngine::new(rules, creds));
        let tenant = ResolvedVirtualKey {
            virtual_key_id: vk.row.id,
            team_id: team.team_id,
            issued_by: team.user_id,
        };
        let mut ctx = ctx_for(
            tenant,
            chat_request(
                "some-unrouted-model",
                Some(vec!["mistral-small-latest".to_string()]),
            ),
        );

        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(outcome, Outcome::Continue));
    }
}
