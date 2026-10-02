/// Common `insta::Settings` for gateway snapshots: redacts fields that are
/// nondeterministic across runs (ids, timestamps) so snapshots of
/// DB-backed or HTTP responses stay reproducible. Intended for the
/// streaming epic's SSE frame snapshots and the OpenAPI-output golden-file
/// tests.
///
/// `insta::assert_snapshot!`/`assert_json_snapshot!` are macros because they
/// need the call site's context to name the snapshot file — wrapping them
/// in a function would break that, so callers bind this at their own call
/// site instead:
///
/// ```ignore
/// let _guard = gateway_testkit::golden_settings().bind_to_scope();
/// insta::assert_json_snapshot!(response_body);
/// ```
pub fn golden_settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.add_redaction(".**.id", "[uuid]");
    settings.add_redaction(".**.created_at", "[timestamp]");
    settings.add_redaction(".**.updated_at", "[timestamp]");
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_id_and_timestamp_fields() {
        let _guard = golden_settings().bind_to_scope();
        let value = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "proxy-key",
            "created_at": "2026-08-10T12:00:00Z",
        });
        insta::assert_json_snapshot!(value, @r###"
        {
          "created_at": "[timestamp]",
          "id": "[uuid]",
          "name": "proxy-key"
        }
        "###);
    }
}
