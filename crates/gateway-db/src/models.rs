use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// A platform-wide superuser: bypasses `require_org_admin`/
    /// `require_team_access` (see `gateway-api/src/authz.rs`) for every org
    /// and team, not just ones they're a member of. Not exposed via any
    /// signup/grant endpoint yet — set directly in the database.
    pub is_master_admin: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub family_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProviderCredential {
    pub id: Uuid,
    pub team_id: Uuid,
    pub added_by: Uuid,
    pub provider: String,
    pub encrypted_api_key: Vec<u8>,
    pub key_nonce: Vec<u8>,
    pub key_version: i16,
    pub key_last_four: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    /// Overrides the provider's registered adapter endpoint when set — lets a
    /// credential target an OpenAI-wire-compatible host (Groq, Together, a
    /// self-hosted server) that has no adapter of its own.
    pub base_url: Option<String>,
    /// Whether this endpoint tolerates OpenAI's `stream_options` parameter.
    /// True (the default) asks for `include_usage` so streamed responses
    /// carry real token counts; false omits it for a strict vendor that
    /// would 400 on the unrecognised field, falling back to estimation.
    pub supports_stream_options: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VirtualKey {
    pub id: Uuid,
    pub team_id: Uuid,
    pub issued_by: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrgMember {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Team {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamMembership {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoutingRule {
    pub id: Uuid,
    pub virtual_key_id: Uuid,
    pub model_pattern: String,
    pub provider_credential_id: Uuid,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewRequestLog {
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub virtual_key_id: Option<Uuid>,
    pub provider: String,
    pub model: String,
    pub status_code: i32,
    pub latency_ms: i32,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub error_message: Option<String>,
    /// How many models were tried before the one that served (or the last
    /// one attempted, on total failure) — 0 means the primary model worked.
    pub fallback_count: i32,
}
