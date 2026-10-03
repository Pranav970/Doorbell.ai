#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("upstream error (status {status}): {body_summary}")]
    Upstream { status: u16, body_summary: String },
    #[error("request to provider timed out")]
    Timeout,
    #[error("failed to serialize/deserialize request or response: {0}")]
    Serialization(String),
    #[error("{0} is not implemented")]
    NotImplemented(&'static str),
    #[error("provider rejected credentials")]
    Unauthorized,
    #[error("request exceeds the model's context length")]
    ContextLengthExceeded,
}

impl ProviderError {
    /// Whether a `FallbackEngine` should treat this as a signal to try the
    /// next model rather than give up: rate limits, upstream 5xxs, timeouts,
    /// context-length overflows, and a 404 (the model doesn't exist/isn't
    /// reachable on this provider — a name typo, a deprecated model, or a
    /// model not enabled for this key) are all things a *different* model or
    /// provider can plausibly succeed at. Bad credentials, malformed
    /// requests, and unsupported features would fail identically anywhere,
    /// so those propagate immediately instead.
    pub fn is_fallback_trigger(&self) -> bool {
        match self {
            ProviderError::Timeout | ProviderError::ContextLengthExceeded => true,
            ProviderError::Upstream { status, .. } => {
                *status == 404 || *status == 429 || *status >= 500
            }
            ProviderError::Unauthorized
            | ProviderError::Serialization(_)
            | ProviderError::NotImplemented(_) => false,
        }
    }
}

impl From<ProviderError> for gateway_common::AppError {
    fn from(e: ProviderError) -> Self {
        match e {
            ProviderError::Unauthorized => {
                gateway_common::AppError::UpstreamError("provider rejected credentials".to_string())
            }
            ProviderError::Timeout => gateway_common::AppError::UpstreamTimeout,
            ProviderError::Upstream {
                status,
                body_summary,
            } => {
                gateway_common::AppError::UpstreamError(format!("status {status}: {body_summary}"))
            }
            ProviderError::Serialization(msg) => gateway_common::AppError::Internal(msg),
            ProviderError::NotImplemented(what) => {
                gateway_common::AppError::BadRequest(format!("{what} is not supported yet"))
            }
            ProviderError::ContextLengthExceeded => gateway_common::AppError::BadRequest(
                "request exceeds the model's context length".to_string(),
            ),
        }
    }
}

pub(crate) fn map_upstream_error(status: reqwest::StatusCode, body: String) -> ProviderError {
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return ProviderError::Unauthorized;
    }
    if is_context_length_error(&body) {
        return ProviderError::ContextLengthExceeded;
    }
    ProviderError::Upstream {
        status: status.as_u16(),
        body_summary: body.chars().take(200).collect(),
    }
}

/// OpenAI, Anthropic and Gemini each report a context-length overflow as a
/// plain 400 with a vendor-specific message rather than a shared error code,
/// so this greps the body for their wording instead of parsing three
/// separate error schemas.
/// ponytail: string-match heuristic — ceiling is a false negative on an
/// unseen vendor phrasing; add the phrase here if one shows up.
fn is_context_length_error(body: &str) -> bool {
    let lower = body.to_lowercase();
    [
        "context_length_exceeded",
        "context length",
        "maximum number of tokens",
        "too many tokens",
        "input length exceeds",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn map_transport_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Serialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_context_length_phrasing_from_any_vendor() {
        assert!(is_context_length_error(
            "{\"error\":{\"code\":\"context_length_exceeded\",\"message\":\"too long\"}}"
        ));
        assert!(is_context_length_error(
            "this model's maximum context length is 8192 tokens"
        ));
        assert!(is_context_length_error(
            "input length exceeds the maximum allowed"
        ));
        assert!(!is_context_length_error("invalid api key"));
    }

    #[test]
    fn fallback_trigger_covers_404_429_5xx_timeout_and_context_length_only() {
        assert!(ProviderError::Timeout.is_fallback_trigger());
        assert!(ProviderError::ContextLengthExceeded.is_fallback_trigger());
        assert!(ProviderError::Upstream {
            status: 404,
            body_summary: String::new()
        }
        .is_fallback_trigger());
        assert!(ProviderError::Upstream {
            status: 429,
            body_summary: String::new()
        }
        .is_fallback_trigger());
        assert!(ProviderError::Upstream {
            status: 503,
            body_summary: String::new()
        }
        .is_fallback_trigger());
        assert!(!ProviderError::Upstream {
            status: 400,
            body_summary: String::new()
        }
        .is_fallback_trigger());
        assert!(!ProviderError::Unauthorized.is_fallback_trigger());
        assert!(!ProviderError::NotImplemented("streaming").is_fallback_trigger());
    }
}
