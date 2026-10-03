use std::sync::Arc;

use tracing::Instrument;

use super::stage::{DecisionLogEntry, FailureMode, Outcome, RequestCtx, ResponseCtx, Stage};

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Continue => "continue".to_string(),
        Outcome::ShortCircuit(r) => format!("short_circuit({})", r.status),
        Outcome::Reject(e) => format!("reject({})", e.status_code()),
        Outcome::Fail(e) => format!("fail({e})"),
    }
}

/// Runs a fixed stage chain. Request-phase and response-phase are separate
/// calls (not one round trip) so a caller can run `on_request` before
/// dispatching to whatever produces the real response (out of scope here —
/// stays in the `chat_completions` handler this epoch) and `on_response`
/// after, on the way back.
pub struct Executor;

impl Executor {
    /// Forward pass: runs `on_request` on each stage in order, stopping at
    /// the first non-`Continue` outcome (a `Fail` on a fail-open stage is
    /// logged and treated as `Continue` instead of stopping). Returns that
    /// outcome plus how many *preceding* stages get `on_response` on the way
    /// back — a stage whose own outcome stopped the chain already decided
    /// the result and doesn't get its own `on_response` call; only earlier
    /// stages that returned `Continue` do.
    pub async fn run_request(chain: &[Arc<dyn Stage>], ctx: &mut RequestCtx) -> (Outcome, usize) {
        for (i, stage) in chain.iter().enumerate() {
            let span =
                tracing::info_span!("pipeline_stage", stage = stage.name(), phase = "request");
            let outcome = stage.on_request(ctx).instrument(span).await;

            if let Outcome::Fail(err) = &outcome {
                if stage.failure_mode() == FailureMode::FailOpen {
                    tracing::warn!(stage = stage.name(), error = %err, "stage failed open, continuing");
                    ctx.decision_log.push(DecisionLogEntry {
                        stage: stage.name(),
                        phase: "request",
                        outcome: describe(&outcome),
                    });
                    continue;
                }
            }

            ctx.decision_log.push(DecisionLogEntry {
                stage: stage.name(),
                phase: "request",
                outcome: describe(&outcome),
            });

            match outcome {
                Outcome::Continue => continue,
                other => return (other, i),
            }
        }
        (Outcome::Continue, chain.len())
    }

    /// Reverse pass: runs `on_response` for the first `executed` stages —
    /// the ones whose `on_request` actually ran — in reverse chain order.
    pub async fn run_response(
        chain: &[Arc<dyn Stage>],
        executed: usize,
        ctx: &mut ResponseCtx,
    ) -> Outcome {
        for stage in chain[..executed].iter().rev() {
            let span =
                tracing::info_span!("pipeline_stage", stage = stage.name(), phase = "response");
            let outcome = stage.on_response(ctx).instrument(span).await;

            if let Outcome::Fail(err) = &outcome {
                if stage.failure_mode() == FailureMode::FailOpen {
                    tracing::warn!(stage = stage.name(), error = %err, "stage failed open, continuing");
                    ctx.decision_log.push(DecisionLogEntry {
                        stage: stage.name(),
                        phase: "response",
                        outcome: describe(&outcome),
                    });
                    continue;
                }
            }

            ctx.decision_log.push(DecisionLogEntry {
                stage: stage.name(),
                phase: "response",
                outcome: describe(&outcome),
            });

            match outcome {
                Outcome::Continue => continue,
                other => return other,
            }
        }
        Outcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::pipeline::stage::GatewayResponse;

    /// A stage that always continues, recording its name + phase into a
    /// shared log so tests can assert call order deterministically.
    struct RecordingStage {
        name: &'static str,
        log: Arc<Mutex<Vec<String>>>,
        failure_mode: FailureMode,
        on_request_outcome: fn() -> Outcome,
        on_response_outcome: fn() -> Outcome,
    }

    #[async_trait]
    impl Stage for RecordingStage {
        fn name(&self) -> &'static str {
            self.name
        }
        fn failure_mode(&self) -> FailureMode {
            self.failure_mode
        }
        async fn on_request(&self, _ctx: &mut RequestCtx) -> Outcome {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}:request", self.name));
            (self.on_request_outcome)()
        }
        async fn on_response(&self, _ctx: &mut ResponseCtx) -> Outcome {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}:response", self.name));
            (self.on_response_outcome)()
        }
    }

    fn continuing(name: &'static str, log: &Arc<Mutex<Vec<String>>>) -> Arc<dyn Stage> {
        Arc::new(RecordingStage {
            name,
            log: log.clone(),
            failure_mode: FailureMode::FailClosed,
            on_request_outcome: || Outcome::Continue,
            on_response_outcome: || Outcome::Continue,
        })
    }

    fn empty_ctx() -> RequestCtx {
        RequestCtx::new("req-1".to_string(), None)
    }

    fn empty_response_ctx() -> ResponseCtx {
        ResponseCtx {
            request_id: "req-1".to_string(),
            tenant: None,
            extensions: http::Extensions::new(),
            decision_log: Vec::new(),
        }
    }

    #[tokio::test]
    async fn executes_stages_in_chain_order_on_request() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain: Vec<Arc<dyn Stage>> = vec![continuing("a", &log), continuing("b", &log)];
        let mut ctx = empty_ctx();

        let (outcome, executed) = Executor::run_request(&chain, &mut ctx).await;

        assert!(matches!(outcome, Outcome::Continue));
        assert_eq!(executed, 2);
        assert_eq!(*log.lock().unwrap(), vec!["a:request", "b:request"]);
    }

    #[tokio::test]
    async fn runs_on_response_in_reverse_chain_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain: Vec<Arc<dyn Stage>> = vec![
            continuing("a", &log),
            continuing("b", &log),
            continuing("c", &log),
        ];
        let mut req_ctx = empty_ctx();
        let (_, executed) = Executor::run_request(&chain, &mut req_ctx).await;

        let mut resp_ctx = empty_response_ctx();
        let outcome = Executor::run_response(&chain, executed, &mut resp_ctx).await;

        assert!(matches!(outcome, Outcome::Continue));
        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "a:request",
                "b:request",
                "c:request",
                "c:response",
                "b:response",
                "a:response"
            ]
        );
    }

    #[tokio::test]
    async fn short_circuit_stops_remaining_stages_and_skips_their_on_response() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let short_circuiting: Arc<dyn Stage> = Arc::new(RecordingStage {
            name: "b",
            log: log.clone(),
            failure_mode: FailureMode::FailClosed,
            on_request_outcome: || {
                Outcome::ShortCircuit(GatewayResponse {
                    status: 200,
                    body: serde_json::json!({}),
                })
            },
            on_response_outcome: || Outcome::Continue,
        });
        let chain: Vec<Arc<dyn Stage>> = vec![
            continuing("a", &log),
            short_circuiting,
            continuing("c", &log),
        ];
        let mut ctx = empty_ctx();

        let (outcome, executed) = Executor::run_request(&chain, &mut ctx).await;

        assert!(matches!(outcome, Outcome::ShortCircuit(_)));
        assert_eq!(
            executed, 1,
            "only stage a gets on_response — b decided the outcome, c never ran on_request"
        );

        let mut resp_ctx = empty_response_ctx();
        Executor::run_response(&chain, executed, &mut resp_ctx).await;
        // "b" short-circuited so it doesn't get its own on_response; "a" ran
        // on_request so it does, on the way back. "c" never ran either phase.
        assert_eq!(
            *log.lock().unwrap(),
            vec!["a:request", "b:request", "a:response"]
        );
    }

    #[tokio::test]
    async fn reject_stops_the_chain_and_returns_the_stage_error() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let rejecting: Arc<dyn Stage> = Arc::new(RecordingStage {
            name: "a",
            log: log.clone(),
            failure_mode: FailureMode::FailClosed,
            on_request_outcome: || Outcome::Reject(gateway_common::AppError::Unauthorized),
            on_response_outcome: || Outcome::Continue,
        });
        let chain: Vec<Arc<dyn Stage>> = vec![rejecting, continuing("b", &log)];
        let mut ctx = empty_ctx();

        let (outcome, executed) = Executor::run_request(&chain, &mut ctx).await;

        assert!(matches!(
            outcome,
            Outcome::Reject(gateway_common::AppError::Unauthorized)
        ));
        assert_eq!(
            executed, 0,
            "a rejected on its own outcome, so it doesn't get its own on_response either"
        );
        assert_eq!(*log.lock().unwrap(), vec!["a:request"]);
    }

    #[tokio::test]
    async fn fail_closed_stage_error_blocks_the_request() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let failing: Arc<dyn Stage> = Arc::new(RecordingStage {
            name: "a",
            log: log.clone(),
            failure_mode: FailureMode::FailClosed,
            on_request_outcome: || Outcome::Fail(anyhow::anyhow!("db unreachable")),
            on_response_outcome: || Outcome::Continue,
        });
        let chain: Vec<Arc<dyn Stage>> = vec![failing, continuing("b", &log)];
        let mut ctx = empty_ctx();

        let (outcome, executed) = Executor::run_request(&chain, &mut ctx).await;

        assert!(matches!(outcome, Outcome::Fail(_)));
        assert_eq!(
            executed, 0,
            "b never ran, and a's own failure excludes it from on_response too"
        );
        assert_eq!(*log.lock().unwrap(), vec!["a:request"]);
    }

    #[tokio::test]
    async fn fail_open_stage_error_continues_the_chain() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let failing: Arc<dyn Stage> = Arc::new(RecordingStage {
            name: "a",
            log: log.clone(),
            failure_mode: FailureMode::FailOpen,
            on_request_outcome: || Outcome::Fail(anyhow::anyhow!("metrics sink down")),
            on_response_outcome: || Outcome::Continue,
        });
        let chain: Vec<Arc<dyn Stage>> = vec![failing, continuing("b", &log)];
        let mut ctx = empty_ctx();

        let (outcome, executed) = Executor::run_request(&chain, &mut ctx).await;

        assert!(matches!(outcome, Outcome::Continue));
        assert_eq!(executed, 2, "b still ran despite a's fail-open error");
        assert_eq!(*log.lock().unwrap(), vec!["a:request", "b:request"]);
    }

    #[tokio::test]
    async fn decision_log_records_one_entry_per_stage() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain: Vec<Arc<dyn Stage>> = vec![continuing("a", &log), continuing("b", &log)];
        let mut ctx = empty_ctx();

        Executor::run_request(&chain, &mut ctx).await;

        assert_eq!(ctx.decision_log.len(), 2);
        assert_eq!(ctx.decision_log[0].stage, "a");
        assert_eq!(ctx.decision_log[0].outcome, "continue");
        assert_eq!(ctx.decision_log[1].stage, "b");
    }
}
