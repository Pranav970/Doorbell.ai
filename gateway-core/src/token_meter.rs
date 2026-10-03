//! Counts output tokens on a live stream without getting in its way.
//!
//! [`meter`] wraps a [`ChatStream`] and returns a stream with the identical
//! item sequence. Each chunk is inspected and forwarded in the same poll —
//! there is no queue, no lookahead, and no work between receiving a chunk
//! and yielding it beyond a `usize` add. The caller sees byte-for-byte what
//! the provider sent, at the moment the provider sent it.
//!
//! The count is reported once, from `Drop`, which is what makes requirement
//! "capture partial counts on abort" fall out for free: a client that
//! disconnects mid-generation causes the response body — and therefore this
//! stream — to be dropped, and the accumulated count is reported exactly as
//! it would be on a clean finish, with `completed: false` to distinguish
//! the two.

use futures_util::StreamExt;
use gateway_providers::ChatStream;

/// What the stream ended up costing, handed to `meter`'s callback exactly
/// once per stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamUsage {
    /// Input tokens, when the provider reported them on the stream.
    pub prompt_tokens: Option<u32>,
    /// Output tokens: the provider's own figure when it sent one, otherwise
    /// an estimate (see `estimated`).
    pub completion_tokens: u32,
    /// True when `completion_tokens` came from the heuristic rather than
    /// from the provider. Worth surfacing before anyone bills on it.
    pub estimated: bool,
    /// False when the stream ended early — client disconnect, cancelled
    /// request, or an upstream transport error mid-generation. The counts
    /// are still whatever accumulated up to that point.
    pub completed: bool,
}

/// Divisor for the fallback estimator, applied to characters of streamed
/// text.
///
/// ponytail: the standard ~4-chars-per-token rule of thumb for English BPE.
/// Ceiling is real inaccuracy on CJK (undercounts ~2x) and on code
/// (overcounts). It only ever runs for providers that send no usage at all —
/// OpenAI/Groq (`stream_options`), Anthropic (`message_delta`) and Gemini
/// (`usageMetadata`) all report real counts, so this is a floor for unknown
/// BYOK vendors, not the common path. Upgrade is a real BPE tokenizer
/// (`tiktoken-rs`) behind this same function, once someone bills on it.
const CHARS_PER_TOKEN: usize = 4;

struct Meter<F: FnOnce(StreamUsage)> {
    chars: usize,
    reported_in: Option<u32>,
    reported_out: Option<u32>,
    completed: bool,
    on_finish: Option<F>,
}

impl<F: FnOnce(StreamUsage)> Meter<F> {
    /// Marks a clean end-of-stream. Read back by `Drop`, which is the only
    /// place the count is reported from.
    fn mark_completed(&mut self) {
        self.completed = true;
    }

    fn observe(&mut self, chunk: &gateway_common::canonical::ChatCompletionChunk) {
        self.chars += chunk.delta.chars().count();
        if let Some(usage) = &chunk.usage {
            // Providers report the two halves on different frames (Anthropic)
            // or repeat cumulative totals on every frame (Gemini), so each
            // field is taken independently and a zero is treated as "not
            // reported here" rather than as a real count.
            if usage.prompt_tokens > 0 {
                self.reported_in = Some(usage.prompt_tokens);
            }
            if usage.completion_tokens > 0 {
                self.reported_out = Some(usage.completion_tokens);
            }
        }
    }
}

impl<F: FnOnce(StreamUsage)> Drop for Meter<F> {
    fn drop(&mut self) {
        let Some(on_finish) = self.on_finish.take() else {
            return;
        };
        let (completion_tokens, estimated) = match self.reported_out {
            Some(reported) => (reported, false),
            None => (self.chars.div_ceil(CHARS_PER_TOKEN) as u32, true),
        };
        on_finish(StreamUsage {
            prompt_tokens: self.reported_in,
            completion_tokens,
            estimated,
            completed: self.completed,
        });
    }
}

/// Wraps `stream` so every chunk passing through is counted, then calls
/// `on_finish` once when the stream ends or is dropped.
///
/// `on_finish` runs inside `Drop`, so it must not block or await — the
/// gateway's caller hands it a `tokio::spawn`-ing closure, the same
/// fire-and-forget shape the non-streaming path already uses for request
/// logging.
pub fn meter<F>(stream: ChatStream, on_finish: F) -> ChatStream
where
    F: FnOnce(StreamUsage) + Send + 'static,
{
    let state = (
        stream,
        Meter {
            chars: 0,
            reported_in: None,
            reported_out: None,
            completed: false,
            on_finish: Some(on_finish),
        },
    );

    Box::pin(futures_util::stream::unfold(
        state,
        |(mut stream, mut meter)| async move {
            match stream.next().await {
                Some(Ok(chunk)) => {
                    meter.observe(&chunk);
                    Some((Ok(chunk), (stream, meter)))
                }
                Some(Err(e)) => {
                    // Mid-stream failure: counts so far stand, but this is
                    // not a clean completion.
                    Some((Err(e), (stream, meter)))
                }
                None => {
                    meter.mark_completed();
                    // Returning None drops the state, firing Meter::drop.
                    None
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use futures_util::stream;
    use gateway_common::canonical::{ChatCompletionChunk, Usage};
    use gateway_providers::ProviderError;

    use super::*;

    fn text(delta: &str) -> Result<ChatCompletionChunk, ProviderError> {
        Ok(ChatCompletionChunk {
            delta: delta.to_string(),
            ..Default::default()
        })
    }

    fn usage(prompt: u32, completion: u32) -> Result<ChatCompletionChunk, ProviderError> {
        Ok(ChatCompletionChunk {
            usage: Some(Usage {
                prompt_tokens: prompt,
                completion_tokens: completion,
                total_tokens: prompt + completion,
            }),
            ..Default::default()
        })
    }

    /// Returns (chunks seen by the consumer, reported usage).
    async fn run(
        items: Vec<Result<ChatCompletionChunk, ProviderError>>,
        take: Option<usize>,
    ) -> (usize, StreamUsage) {
        let sink = Arc::new(Mutex::new(None));
        let captured = sink.clone();

        let source: ChatStream = Box::pin(stream::iter(items));
        let metered = meter(source, move |u| *captured.lock().unwrap() = Some(u));

        let seen = match take {
            // Dropping the stream early is exactly what axum does when the
            // client goes away mid-response.
            Some(n) => metered.take(n).count().await,
            None => metered.count().await,
        };

        let reported = sink.lock().unwrap().take().expect("on_finish must fire");
        (seen, reported)
    }

    #[tokio::test]
    async fn prefers_the_providers_own_counts_over_the_estimate() {
        let (seen, u) = run(vec![text("Hello "), text("world"), usage(9, 7)], None).await;
        assert_eq!(seen, 3, "every chunk still reaches the consumer");
        assert_eq!(u.completion_tokens, 7);
        assert_eq!(u.prompt_tokens, Some(9));
        assert!(!u.estimated);
        assert!(u.completed);
    }

    #[tokio::test]
    async fn merges_counts_reported_on_separate_frames() {
        // Anthropic's shape: input tokens up front, output tokens at the end.
        let (_, u) = run(vec![usage(11, 0), text("hi"), usage(0, 23)], None).await;
        assert_eq!(u.prompt_tokens, Some(11));
        assert_eq!(u.completion_tokens, 23);
        assert!(!u.estimated);
    }

    #[tokio::test]
    async fn estimates_when_the_provider_reports_nothing() {
        // 20 characters -> 5 tokens at the documented ratio.
        let (_, u) = run(vec![text("12345678901234567890")], None).await;
        assert_eq!(u.completion_tokens, 5);
        assert!(u.estimated);
    }

    #[tokio::test]
    async fn a_client_that_disconnects_midway_still_reports_partial_counts() {
        let items = vec![text("aaaa"), text("bbbb"), text("cccc"), usage(1, 99)];
        let (seen, u) = run(items, Some(2)).await;
        assert_eq!(seen, 2);
        // Only the 8 characters actually delivered are counted, and the
        // provider's final usage frame was never reached.
        assert_eq!(u.completion_tokens, 2);
        assert!(u.estimated);
        assert!(!u.completed, "an aborted stream must not look complete");
    }
}
