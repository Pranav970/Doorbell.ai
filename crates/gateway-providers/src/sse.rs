//! Incremental Server-Sent Events decoding, shared by every adapter that
//! streams.
//!
//! One decoder, parameterized by a per-provider [`EventMapper`] — the
//! vendor-specific part is a single pure function per adapter, so
//! normalization lives next to the translation code it belongs with rather
//! than in a switch statement somewhere downstream.
//!
//! **Latency contract:** a chunk is yielded the instant the frame
//! terminating it arrives. Nothing waits for the next frame, nothing
//! accumulates the response body, and the only state carried between polls
//! is the trailing bytes of a frame that was split across TCP reads. A
//! caller awaiting this stream sees a token as soon as the socket does.

use std::collections::VecDeque;

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use gateway_common::canonical::ChatCompletionChunk;

use crate::error::{map_transport_error, ProviderError};
use crate::trait_def::ChatStream;

/// Translates one decoded SSE frame into the canonical chunk shape.
///
/// `event` is the frame's `event:` name (empty when the provider doesn't
/// send one — OpenAI and Gemini don't, Anthropic does). `data` is the joined
/// `data:` payload.
///
/// Returning `None` drops the frame: keepalive comments, `[DONE]`, and the
/// bookkeeping events that carry neither text nor token counts.
pub type EventMapper = fn(event: &str, data: &str) -> Option<ChatCompletionChunk>;

struct Decoder<S> {
    inner: S,
    /// Bytes of a frame that hasn't terminated yet. Bounded in practice by
    /// one frame — cleared every time a `\n\n` is found.
    buf: Vec<u8>,
    /// Chunks decoded from the most recent read but not yet yielded. A
    /// single TCP read can contain several complete frames.
    pending: VecDeque<ChatCompletionChunk>,
    finished: bool,
}

/// Wraps a raw response byte stream into a stream of canonical chunks.
pub fn decode<S>(bytes: S, map: EventMapper) -> ChatStream
where
    S: Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
{
    let state = Decoder {
        inner: Box::pin(bytes),
        buf: Vec::new(),
        pending: VecDeque::new(),
        finished: false,
    };

    Box::pin(futures_util::stream::unfold(
        state,
        move |mut st| async move {
            loop {
                // Anything already decoded goes out before we touch the socket
                // again — this is what makes a multi-frame read emit N chunks
                // rather than collapsing them.
                if let Some(chunk) = st.pending.pop_front() {
                    return Some((Ok(chunk), st));
                }
                if st.finished {
                    return None;
                }

                match st.inner.next().await {
                    Some(Ok(bytes)) => {
                        st.buf.extend_from_slice(&bytes);
                        drain_frames(&mut st.buf, map, &mut st.pending);
                    }
                    Some(Err(e)) => {
                        // A transport failure mid-stream. Surfaced as a stream
                        // item rather than swallowed, so the caller can log a
                        // truncated response instead of a clean one.
                        st.finished = true;
                        return Some((Err(map_transport_error(e)), st));
                    }
                    None => {
                        // Upstream closed. A well-formed stream ends on a frame
                        // boundary and leaves nothing here, but a provider that
                        // omits the final blank line would otherwise lose its
                        // last frame — including a usage-carrying one.
                        st.finished = true;
                        if !st.buf.is_empty() {
                            let tail = std::mem::take(&mut st.buf);
                            if let Some(chunk) = decode_frame(&tail, map) {
                                st.pending.push_back(chunk);
                            }
                        }
                    }
                }
            }
        },
    ))
}

/// Pulls every complete frame out of `buf`, leaving any partial trailing
/// frame behind for the next read.
fn drain_frames(buf: &mut Vec<u8>, map: EventMapper, out: &mut VecDeque<ChatCompletionChunk>) {
    // SSE only ever uses CR as part of a line terminator, and a literal CR
    // inside a JSON payload is escaped as the two characters `\r` — so
    // dropping every CR byte normalizes CRLF framing without touching data.
    buf.retain(|b| *b != b'\r');

    while let Some(end) = find_frame_end(buf) {
        let frame: Vec<u8> = buf.drain(..end + 2).collect();
        if let Some(chunk) = decode_frame(&frame[..end], map) {
            out.push_back(chunk);
        }
    }
}

fn find_frame_end(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

fn decode_frame(frame: &[u8], map: EventMapper) -> Option<ChatCompletionChunk> {
    // Frames are split on a boundary, never mid-codepoint, so this is lossy
    // only for genuinely invalid upstream UTF-8.
    let text = String::from_utf8_lossy(frame);

    let mut event = String::new();
    let mut data = String::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("data:") {
            // Multiple `data:` lines in one frame concatenate with newlines,
            // per the SSE spec.
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.strip_prefix(' ').unwrap_or(value));
        } else if let Some(value) = line.strip_prefix("event:") {
            event = value.trim().to_string();
        }
        // Everything else (`id:`, `retry:`, `:` comments) is irrelevant here.
    }

    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return None;
    }
    map(&event, data)
}

/// Convenience for the common "this frame is just text" case.
pub(crate) fn text_chunk(id: &str, model: &str, delta: impl Into<String>) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: id.to_string(),
        model: model.to_string(),
        delta: delta.into(),
        finish_reason: None,
        usage: None,
    }
}

/// Turns a non-2xx streaming response into the same error shape the
/// non-streaming path produces. Streaming errors arrive as an ordinary JSON
/// body, before any SSE framing starts.
pub(crate) async fn error_from_response(response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    crate::error::map_upstream_error(status, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    fn passthrough(_event: &str, data: &str) -> Option<ChatCompletionChunk> {
        Some(text_chunk("id", "m", data))
    }

    /// Takes byte slices, not strings, so a test can split a multi-byte
    /// codepoint across two reads the way a real socket would.
    async fn collect(parts: Vec<&'static [u8]>) -> Vec<String> {
        let byte_stream = stream::iter(parts.into_iter().map(|p| Ok(Bytes::from_static(p))));
        decode(byte_stream, passthrough)
            .map(|c| c.unwrap().delta)
            .collect::<Vec<_>>()
            .await
    }

    #[tokio::test]
    async fn splits_frames_and_skips_done_and_comments() {
        let out = collect(vec![
            b": keepalive\n\ndata: one\n\ndata: two\n\ndata: [DONE]\n\n",
        ])
        .await;
        assert_eq!(out, vec!["one", "two"]);
    }

    #[tokio::test]
    async fn reassembles_a_frame_split_across_reads() {
        // The decoder must not emit until the terminating blank line lands,
        // and must not lose the first half.
        let out = collect(vec![b"data: hel", b"lo\n", b"\ndata: world\n\n"]).await;
        assert_eq!(out, vec!["hello", "world"]);
    }

    #[tokio::test]
    async fn multibyte_utf8_split_across_reads_survives() {
        // "日" is 3 bytes; cutting it in half would corrupt it if the buffer
        // were decoded as a string per-read instead of per-frame.
        let out = collect(vec![b"data: \xe6\x97", b"\xa5\n\n"]).await;
        assert_eq!(out, vec!["日"]);
    }

    #[tokio::test]
    async fn handles_crlf_framing_and_a_missing_final_blank_line() {
        let out = collect(vec![b"data: a\r\n\r\ndata: b"]).await;
        assert_eq!(out, vec!["a", "b"]);
    }
}
