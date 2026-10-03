use uuid::Uuid;

/// Identity resolved from a validated virtual key, threaded through request
/// extensions by the gateway-api AuthLayer. Later layers (RateLimitLayer,
/// BudgetLayer in Phase 2) read this to key their own state without needing
/// to know how it was produced.
#[derive(Debug, Clone)]
pub struct ResolvedVirtualKey {
    pub virtual_key_id: Uuid,
    pub team_id: Uuid,
    pub issued_by: Uuid,
}

/// Output of routing resolution: which provider credential (and therefore
/// which vendor adapter) should serve this request.
#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub provider_credential_id: Uuid,
    pub provider_name: String,
}
