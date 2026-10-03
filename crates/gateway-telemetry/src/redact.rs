/// Always the same length regardless of the matched secret's length — the
/// point is to not even leak *how long* a redacted value was.
const REDACTED_MARKER: &str = "[REDACTED]";

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

/// Three dot-separated base64url segments, each long enough that a
/// version-like string ("1.2.3") or a decimal can't match by accident.
fn looks_like_jwt(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            p.len() >= 8
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

/// Matches this codebase's actual virtual-key format (`vk_live_{suffix}`,
/// see `gateway-api/src/routes/virtual_keys.rs::generate_virtual_key`).
fn looks_like_virtual_key(token: &str) -> bool {
    const PREFIX: &str = "vk_live_";
    token.len() > PREFIX.len() + 4 && token.starts_with(PREFIX)
}

/// The `sk-...` shape OpenAI and Anthropic both use for their own API keys —
/// not sourced from any prefix table in this codebase (there isn't one;
/// `gateway-crypto` is provider-agnostic), just the common real-world shape.
fn looks_like_provider_key(token: &str) -> bool {
    const PREFIX: &str = "sk-";
    token.len() > PREFIX.len() + 6 && token.starts_with(PREFIX)
}

fn looks_like_secret(token: &str) -> bool {
    looks_like_jwt(token) || looks_like_virtual_key(token) || looks_like_provider_key(token)
}

/// Scans `line` for known secret shapes (JWTs, this app's `vk_live_...`
/// virtual keys, `sk-...`-style provider keys) and replaces each match with
/// a fixed-length marker. A best-effort net for a handler that accidentally
/// logs a full secret — not a substitute for never logging one in the first
/// place, and not a general-purpose secret scanner: an oddly-shaped
/// custom/BYOK vendor key won't match any of these three shapes.
pub fn redact(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token_start: Option<usize> = None;

    for (idx, c) in line.char_indices() {
        if is_token_char(c) {
            token_start.get_or_insert(idx);
            continue;
        }
        if let Some(start) = token_start.take() {
            push_token_or_redacted(&mut out, &line[start..idx]);
        }
        out.push(c);
    }
    if let Some(start) = token_start {
        push_token_or_redacted(&mut out, &line[start..]);
    }
    out
}

fn push_token_or_redacted(out: &mut String, token: &str) {
    if looks_like_secret(token) {
        out.push_str(REDACTED_MARKER);
    } else {
        out.push_str(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_a_jwt_shaped_string() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let line = format!("authorization=Bearer {jwt}");
        let redacted = redact(&line);
        assert!(!redacted.contains(jwt));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_a_virtual_key_shaped_string() {
        let key = "vk_live_abcdEFGH1234567890";
        let line = format!("key={key}");
        let redacted = redact(&line);
        assert!(!redacted.contains(key));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_a_provider_key_shaped_string() {
        let key = "sk-abcdefghijklmnopqrstuvwxyz123456";
        let line = format!("provider_api_key={key}");
        let redacted = redact(&line);
        assert!(!redacted.contains(key));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn leaves_ordinary_log_output_untouched() {
        let line = "request_id=3fa85f64-5717-4562-b3fc-2c963f66afa6 method=POST status=200 path=/auth/login";
        assert_eq!(redact(line), line);
    }

    #[test]
    fn redacted_marker_has_fixed_length_regardless_of_secret_length() {
        let short = redact("sk-shortkey123");
        let long = redact("sk-averylongprovidertokenthatgoesonandonandonforever1234567890");
        let short_marker = &short[short.find('[').unwrap()..];
        let long_marker = &long[long.find('[').unwrap()..];
        assert_eq!(short_marker, "[REDACTED]");
        assert_eq!(long_marker, "[REDACTED]");
        assert_eq!(short_marker.len(), long_marker.len());
    }
}
