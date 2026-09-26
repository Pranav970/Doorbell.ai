-- Extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto; -- gen_random_uuid()

-- USERS
-- Email is plain TEXT with a case-insensitive unique index (rather than
-- CITEXT) to avoid sqlx compile-time query checking needing a decode impl for
-- a non-builtin OID; lookups/inserts normalize with LOWER() explicitly.
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_unique_idx ON users (LOWER(email));

-- REFRESH TOKENS
-- Opaque, hashed, rotated on use. `family_id` groups a rotation chain so that
-- replaying an already-rotated token can revoke the whole chain (reuse
-- detection), and `replaced_by` links each token to the one that superseded
-- it for auditability.
CREATE TABLE refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL,
    family_id       UUID NOT NULL,
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    replaced_by     UUID REFERENCES refresh_tokens(id),
    CONSTRAINT refresh_tokens_token_hash_unique UNIQUE (token_hash)
);
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_family_id ON refresh_tokens(family_id);
CREATE INDEX idx_refresh_tokens_active ON refresh_tokens(token_hash) WHERE revoked_at IS NULL;

-- PROVIDER CREDENTIALS (BYOK)
-- `provider` is a plain TEXT, deliberately not a DB enum/lookup table —
-- validity is enforced app-side against gateway_providers::ProviderRegistry,
-- so adding a new provider never needs a migration.
CREATE TABLE provider_credentials (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider            TEXT NOT NULL,
    encrypted_api_key   BYTEA NOT NULL,
    key_nonce           BYTEA NOT NULL,
    key_version         SMALLINT NOT NULL DEFAULT 1,
    key_last_four       TEXT NOT NULL,
    label               TEXT,
    is_active           BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_provider_credentials_user_id ON provider_credentials(user_id);
CREATE INDEX idx_provider_credentials_user_active ON provider_credentials(user_id) WHERE is_active = true;

-- VIRTUAL KEYS
CREATE TABLE virtual_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash        TEXT NOT NULL,
    key_prefix      TEXT NOT NULL,
    name            TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ,
    CONSTRAINT virtual_keys_key_hash_unique UNIQUE (key_hash)
);
CREATE INDEX idx_virtual_keys_user_id ON virtual_keys(user_id);
CREATE INDEX idx_virtual_keys_prefix ON virtual_keys(key_prefix);

-- ROUTING RULES (per-customer override/alias; seed for future weighted/fallback routing)
CREATE TABLE routing_rules (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    virtual_key_id          UUID NOT NULL REFERENCES virtual_keys(id) ON DELETE CASCADE,
    model_pattern           TEXT NOT NULL,
    provider_credential_id  UUID NOT NULL REFERENCES provider_credentials(id) ON DELETE CASCADE,
    priority                INTEGER NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_routing_rules_vkey_priority ON routing_rules(virtual_key_id, priority);

-- REQUEST LOGS
-- `virtual_key_id` uses ON DELETE SET NULL so the audit trail outlives a
-- revoked/deleted key; every other FK cascades.
CREATE TABLE request_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    virtual_key_id  UUID REFERENCES virtual_keys(id) ON DELETE SET NULL,
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    status_code     INTEGER NOT NULL,
    latency_ms      INTEGER NOT NULL,
    tokens_in       INTEGER,
    tokens_out      INTEGER,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_request_logs_user_id_created_at ON request_logs(user_id, created_at DESC);
CREATE INDEX idx_request_logs_virtual_key_id ON request_logs(virtual_key_id);
