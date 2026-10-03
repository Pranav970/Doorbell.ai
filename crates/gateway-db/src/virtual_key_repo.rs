use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::VirtualKey;

/// The envelope-encrypted copy of a virtual key's plaintext. Mirrors
/// `gateway_crypto::EncryptedBlob` without `gateway-db` depending on
/// `gateway-crypto` — this crate stores bytes, it does not do cryptography.
#[derive(Debug, Clone)]
pub struct VirtualKeySecret {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_version: i16,
}

#[derive(Clone)]
pub struct VirtualKeyRepo {
    pool: PgPool,
}

impl VirtualKeyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// `key_hash` remains the authentication credential; `secret` is an
    /// independent envelope-encrypted copy, stored only so an authorized team
    /// member can retrieve the key later (see `find_secret_for_team`). Pass
    /// `None` to store a key that can never be revealed.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        team_id: Uuid,
        issued_by: Uuid,
        key_hash: &str,
        key_prefix: &str,
        name: Option<&str>,
        secret: Option<VirtualKeySecret>,
    ) -> Result<VirtualKey, RepoError> {
        let (ciphertext, nonce, version) = match secret {
            Some(s) => (Some(s.ciphertext), Some(s.nonce), Some(s.key_version)),
            None => (None, None, None),
        };
        sqlx::query_as!(
            VirtualKey,
            r#"INSERT INTO virtual_keys (team_id, issued_by, key_hash, key_prefix, name, encrypted_key, key_nonce, key_version)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, team_id, issued_by, key_hash, key_prefix, name, status, created_at, last_used_at"#,
            team_id,
            issued_by,
            key_hash,
            key_prefix,
            name,
            ciphertext.as_deref(),
            nonce.as_deref(),
            version
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "virtual key collision, please retry"))
    }

    /// The stored ciphertext for one key, scoped to its team so a caller can
    /// never reach another team's material by id alone. Deliberately a
    /// separate query from `find_by_hash`/`list_for_team`: those run on the
    /// authentication hot path and on every dashboard load, and neither has
    /// any business pulling key material into memory.
    ///
    /// `Ok(None)` covers both "no such key for this team" and "issued before
    /// envelope storage existed" — the caller can't reveal either, so they
    /// are the same outcome.
    pub async fn find_secret_for_team(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<VirtualKeySecret>, RepoError> {
        let row = sqlx::query!(
            r#"SELECT encrypted_key, key_nonce, key_version
               FROM virtual_keys WHERE id = $1 AND team_id = $2"#,
            id,
            team_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)?;

        Ok(row.and_then(|r| {
            Some(VirtualKeySecret {
                ciphertext: r.encrypted_key?,
                nonce: r.key_nonce?,
                key_version: r.key_version?,
            })
        }))
    }

    pub async fn list_for_team(&self, team_id: Uuid) -> Result<Vec<VirtualKey>, RepoError> {
        sqlx::query_as!(
            VirtualKey,
            r#"SELECT id, team_id, issued_by, key_hash, key_prefix, name, status, created_at, last_used_at
               FROM virtual_keys WHERE team_id = $1 ORDER BY created_at DESC"#,
            team_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_hash(&self, key_hash: &str) -> Result<Option<VirtualKey>, RepoError> {
        sqlx::query_as!(
            VirtualKey,
            r#"SELECT id, team_id, issued_by, key_hash, key_prefix, name, status, created_at, last_used_at
               FROM virtual_keys WHERE key_hash = $1"#,
            key_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_id_for_team(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<VirtualKey>, RepoError> {
        sqlx::query_as!(
            VirtualKey,
            r#"SELECT id, team_id, issued_by, key_hash, key_prefix, name, status, created_at, last_used_at
               FROM virtual_keys WHERE id = $1 AND team_id = $2"#,
            id,
            team_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Returns the number of rows affected (0 means the id didn't belong to
    /// this team / doesn't exist).
    pub async fn revoke(&self, id: Uuid, team_id: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"UPDATE virtual_keys SET status = 'revoked' WHERE id = $1 AND team_id = $2"#,
            id,
            team_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    /// Hard delete — only ever removes an already-revoked row (guarded by
    /// `status != 'active'`) so a live key can't be purged without being
    /// revoked first. Returns the number of rows affected.
    pub async fn delete(&self, id: Uuid, team_id: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"DELETE FROM virtual_keys WHERE id = $1 AND team_id = $2 AND status != 'active'"#,
            id,
            team_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    pub async fn touch_last_used(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"UPDATE virtual_keys SET last_used_at = now() WHERE id = $1"#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(())
    }
}
