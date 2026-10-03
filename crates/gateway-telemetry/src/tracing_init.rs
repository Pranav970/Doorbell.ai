use crate::writer::RedactingMakeWriter;

/// Initializes a JSON-structured tracing subscriber honoring `RUST_LOG`
/// (defaults to "info"), writing through a `RedactingMakeWriter` so any
/// known secret shape (JWT, virtual key, provider key — see `redact`) that
/// reaches a log field is replaced with a fixed marker before it's written
/// anywhere. This is a last-resort net: nothing in this codebase should log
/// provider keys, virtual keys, JWTs, or the raw `Authorization` header in
/// the first place.
pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(RedactingMakeWriter::new(std::io::stdout))
        .init();
}
