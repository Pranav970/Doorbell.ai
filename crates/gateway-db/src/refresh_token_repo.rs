use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::RepoError;
use crate::models::RefreshToken;

#[derive(Clone)]
pub struct RefreshTokenRepo {
    pool: PgPool,
}

impl RefreshTokenRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        user_id: Uuid,
        token_hash: &str,
        family_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<RefreshToken, RepoError> {
        sqlx::query_as!(
            RefreshToken,
            r#"INSERT INTO refresh_tokens (user_id, token_hash, family_id, expires_at)
               VALUES ($1, $2, $3, $4)
               RETURNING id, user_id, token_hash, family_id, issued_at, expires_at, revoked_at, replaced_by"#,
            user_id,
            token_hash,
            family_id,
            expires_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, RepoError> {
        sqlx::query_as!(
            RefreshToken,
            r#"SELECT id, user_id, token_hash, family_id, issued_at, expires_at, revoked_at, replaced_by
               FROM refresh_tokens WHERE token_hash = $1"#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Revokes a single token (normal logout, or marking the old token dead
    /// during rotation). `replaced_by` is set when this call is part of a
    /// rotation, `None` for a plain logout.
    pub async fn revoke(&self, id: Uuid, replaced_by: Option<Uuid>) -> Result<(), RepoError> {
        sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $2 WHERE id = $1"#,
            id,
            replaced_by
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(())
    }

    /// Revokes every still-active token in a rotation family. Used when a
    /// refresh token is replayed after already being rotated out — a signal
    /// the family may be compromised.
    pub async fn revoke_family(&self, family_id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL"#,
            family_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(())
    }
}
