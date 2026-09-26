-- ORGANIZATIONS
CREATE TABLE organizations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL,
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX organizations_slug_unique_idx ON organizations (LOWER(slug));

-- ORG MEMBERS
-- `role` is a plain TEXT ('admin' | 'member') rather than a DB enum, matching
-- the `provider_credentials.provider` precedent — validity enforced app-side.
CREATE TABLE org_members (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT org_members_org_user_unique UNIQUE (org_id, user_id)
);
CREATE INDEX idx_org_members_user_id ON org_members(user_id);
CREATE INDEX idx_org_members_org_id ON org_members(org_id);

-- TEAMS
CREATE TABLE teams (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL,
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT teams_org_slug_unique UNIQUE (org_id, slug)
);
CREATE INDEX idx_teams_org_id ON teams(org_id);

-- TEAM MEMBERSHIPS
-- `status` ('pending' | 'approved' | 'rejected') drives the join-request /
-- admin-approval flow; a user can only have one membership row per team.
CREATE TABLE team_memberships (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id         UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending',
    requested_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at      TIMESTAMPTZ,
    decided_by      UUID REFERENCES users(id),
    CONSTRAINT team_memberships_team_user_unique UNIQUE (team_id, user_id)
);
CREATE INDEX idx_team_memberships_user_id ON team_memberships(user_id);
CREATE INDEX idx_team_memberships_team_status ON team_memberships(team_id, status);

-- USERS: capture a display name at signup.
ALTER TABLE users ADD COLUMN display_name TEXT;

-- PROVIDER CREDENTIALS: move ownership from a user to a team (shared BYOK).
-- `added_by` keeps the audit trail of who entered the key.
-- Requires an empty table (fresh/reset dev DB) since team_id has no backfill
-- source from a bare user_id — acceptable pre-launch.
ALTER TABLE provider_credentials RENAME COLUMN user_id TO added_by;
ALTER TABLE provider_credentials ADD COLUMN team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_provider_credentials_user_id;
DROP INDEX IF EXISTS idx_provider_credentials_user_active;
CREATE INDEX idx_provider_credentials_team_id ON provider_credentials(team_id);
CREATE INDEX idx_provider_credentials_team_active ON provider_credentials(team_id) WHERE is_active = true;

-- VIRTUAL KEYS: move ownership from a user to a team.
-- `issued_by` keeps the audit trail of who created the key.
ALTER TABLE virtual_keys RENAME COLUMN user_id TO issued_by;
ALTER TABLE virtual_keys ADD COLUMN team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE;
DROP INDEX IF EXISTS idx_virtual_keys_user_id;
CREATE INDEX idx_virtual_keys_team_id ON virtual_keys(team_id);

-- REQUEST LOGS: tag each request with the team whose credentials served it.
ALTER TABLE request_logs ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE SET NULL;
CREATE INDEX idx_request_logs_team_id ON request_logs(team_id);
