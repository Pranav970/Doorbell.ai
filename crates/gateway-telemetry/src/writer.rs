use std::io::{self, Write};

use tracing_subscriber::fmt::MakeWriter;

use crate::redact::redact;

/// Wraps any `Write` sink, redacting known secret shapes from every buffer
/// written to it before forwarding — the last line of defense if a handler
/// accidentally formats a secret into a log field.
pub struct RedactingWriter<W> {
    inner: W,
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        self.inner.write_all(redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A `MakeWriter` that produces a `RedactingWriter` around whatever the
/// wrapped `MakeWriter` produces — plug into
/// `tracing_subscriber::fmt().with_writer(RedactingMakeWriter::new(std::io::stdout))`.
pub struct RedactingMakeWriter<M> {
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.inner.make_writer(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A `MakeWriter` over a shared in-memory buffer — the standard pattern
    /// for asserting on `tracing` output without touching stdout or the
    /// global subscriber (`tracing::subscriber::with_default` scopes the
    /// subscriber to this test only).
    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for TestWriter {
        type Writer = TestWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn synthetic_key_never_reaches_the_log_writer_even_if_a_handler_logs_the_full_request() {
        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let make_writer = RedactingMakeWriter::new(TestWriter(buf.clone()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(make_writer)
            .with_ansi(false)
            .finish();

        let synthetic_secret = "vk_live_totallyRealLookingSecretValue987";
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                request_body = %format!("POST /v1/chat/completions Authorization: Bearer {synthetic_secret}"),
                "accidental full-request log"
            );
        });

        let output = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(
            !output.contains(synthetic_secret),
            "raw secret leaked into log output: {output}"
        );
        assert!(
            output.contains("[REDACTED]"),
            "expected a redaction marker in: {output}"
        );
    }
}
