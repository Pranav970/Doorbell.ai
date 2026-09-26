-- Local-dev seed data: one admin user, one org, one team, one virtual key.
-- Run via `make db-seed`, or `make db-reset` (which runs this after
-- migrations on a freshly recreated database).
--
-- Not idempotent by design, same as every other INSERT-based fixture in
-- this codebase: the org/team slugs and the user's email are UNIQUE, so
-- running this twice against the same database fails on those constraints
-- rather than silently duplicating rows. Use `make db-reset` to start over
-- cleanly instead of re-running this against existing data.
--
-- Dev login:        dev@example.com / devpassword123
-- Dev virtual key:   vk_live_devseed1234567890abcdef
-- (the raw key above is never stored anywhere — only its hash is; this
-- comment is the only place it exists in plaintext, and only for local dev)

WITH seed_user AS (
    INSERT INTO users (email, password_hash, display_name)
    VALUES (
        'dev@example.com',
        -- argon2id hash of "devpassword123" (Argon2::default() params, per
        -- gateway-auth::password::hash_password)
        '$argon2id$v=19$m=19456,t=2,p=1$mOTAO0byn3f4Uu3gaFCwxw$TYRCur29N0nBXGpJ4pDuBBXCWo5ps5Up83Y+e029jLs',
        'Dev Seed User'
    )
    RETURNING id
),
seed_org AS (
    INSERT INTO organizations (name, slug, created_by)
    SELECT 'Seed Org', 'seed-org', id FROM seed_user
    RETURNING id, created_by
),
seed_org_member AS (
    INSERT INTO org_members (org_id, user_id, role)
    SELECT id, created_by, 'admin' FROM seed_org
    RETURNING org_id
),
seed_team AS (
    INSERT INTO teams (org_id, name, slug, created_by)
    SELECT seed_org.id, 'Seed Team', 'seed-team', seed_org.created_by
    FROM seed_org
    RETURNING id
)
INSERT INTO virtual_keys (team_id, issued_by, key_hash, key_prefix, name, status)
SELECT
    seed_team.id,
    seed_org.created_by,
    -- base64(sha256("vk_live_devseed1234567890abcdef")) — see
    -- gateway-api::middleware::virtual_key_auth::hash_virtual_key for the
    -- exact scheme this must match.
    'Ril2dBXqPefdc+tbNOggvy9/ERIfPgWxbfmxgslAVwE=',
    'vk_live_devseed1',
    'Seed Virtual Key',
    'active'
FROM seed_team, seed_org;
