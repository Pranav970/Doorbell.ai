DROP INDEX IF EXISTS idx_request_logs_team_id;
ALTER TABLE request_logs DROP COLUMN IF EXISTS team_id;

DROP INDEX IF EXISTS idx_virtual_keys_team_id;
ALTER TABLE virtual_keys DROP COLUMN team_id;
ALTER TABLE virtual_keys RENAME COLUMN issued_by TO user_id;
CREATE INDEX idx_virtual_keys_user_id ON virtual_keys(user_id);

DROP INDEX IF EXISTS idx_provider_credentials_team_active;
DROP INDEX IF EXISTS idx_provider_credentials_team_id;
ALTER TABLE provider_credentials DROP COLUMN team_id;
ALTER TABLE provider_credentials RENAME COLUMN added_by TO user_id;
CREATE INDEX idx_provider_credentials_user_id ON provider_credentials(user_id);
CREATE INDEX idx_provider_credentials_user_active ON provider_credentials(user_id) WHERE is_active = true;

ALTER TABLE users DROP COLUMN IF EXISTS display_name;

DROP TABLE IF EXISTS team_memberships;
DROP TABLE IF EXISTS teams;
DROP TABLE IF EXISTS org_members;
DROP TABLE IF EXISTS organizations;
