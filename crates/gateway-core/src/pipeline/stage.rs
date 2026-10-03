use async_trait::async_trait;
use gateway_common::canonical::ChatCompletionRequest;
use gateway_common::AppError;

use crate::context::ResolvedVirtualKey;

/// Whether an unexpected internal error (`Outcome::Fail`) from this stage
/// should block the request or let it through. Only applies to `Fail` — a
/// deliberate `Outcome::Reject` is always a hard stop regardless of this,
/// since a stage that wants fail-open behavior on its own decision would
/// simply return `Continue` instead of `Reject`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    FailOpen,
    FailClosed,
}

/// A minimal, axum-free HTTP-shaped response — for a stage that answers the
/// request itself without reaching the provider (e.g. a future cache-hit
/// stage). Unused by `AuthenticateStage`/`RoutingStage`; exists to satisfy
/// the `Outcome` contract for stages that need it later.
#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

/// What a stage decided. `Reject` and `Fail` both stop the chain; the
/// distinction is that `Reject` is the stage's own considered judgment
/// (always fatal), while `Fail` is an unexpected error whose severity is
/// governed by `Stage::failure_mode`.
pub enum Outcome {
    Continue,
    ShortCircuit(GatewayResponse),
    Reject(AppError),
    Fail(anyhow::Error),
}

/// One entry per stage outcome, appended by the executor — not by stages
/// themselves, so it can't be forgotten.
#[derive(Debug, Clone)]
pub struct DecisionLogEntry {
    pub stage: &'static str,
    pub phase: &'static str, // "request" | "response"
    pub outcome: String,
}

/// Carries request-scoped state through the chain. `extensions` is also how
/// the caller feeds initial input to the first stage (e.g. the raw bearer
/// token) without gateway-core needing to know about HTTP headers.
pub struct RequestCtx {
    pub request_id: String,
    pub tenant: Option<ResolvedVirtualKey>,
    pub request: Option<ChatCompletionRequest>,
    pub extensions: http::Extensions,
    pub decision_log: Vec<DecisionLogEntry>,
}

impl RequestCtx {
    pub fn new(request_id: String, request: Option<ChatCompletionRequest>) -> Self {
        Self {
            request_id,
            tenant: None,
            request,
            extensions: http::Extensions::new(),
            decision_log: Vec::new(),
        }
    }
}

/// The response-phase counterpart to `RequestCtx`, passed to `on_response`.
/// No stage in this epoch inspects/rewrites the in-flight response, so this
/// deliberately carries only what's needed to prove the executor's reverse
/// pass works — a response-body field lands when a stage that actually
/// needs one does (e.g. redaction).
pub struct ResponseCtx {
    pub request_id: String,
    pub tenant: Option<ResolvedVirtualKey>,
    pub extensions: http::Extensions,
    pub decision_log: Vec<DecisionLogEntry>,
}

#[async_trait]
pub trait Stage: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn stream_compatible(&self) -> bool {
        true
    }
    fn failure_mode(&self) -> FailureMode;
    async fn on_request(&self, ctx: &mut RequestCtx) -> Outcome;
    async fn on_response(&self, _ctx: &mut ResponseCtx) -> Outcome {
        Outcome::Continue
    }
}
