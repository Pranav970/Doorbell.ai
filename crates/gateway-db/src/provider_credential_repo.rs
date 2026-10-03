use sqlx::PgPool;
use uuid::Uuid;

use crate::error::RepoError;
use crate::models::ProviderCredential;

#[derive(Clone)]
pub struct ProviderCredentialRepo {
    pool: PgPool,
}

impl ProviderCredentialRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        team_id: Uuid,
        added_by: Uuid,
        provider: &str,
        encrypted_api_key: &[u8],
        key_nonce: &[u8],
        key_version: i16,
        key_last_four: &str,
        label: Option<&str>,
        base_url: Option<&str>,
        supports_stream_options: bool,
    ) -> Result<ProviderCredential, RepoError> {
        sqlx::query_as!(
            ProviderCredential,
            r#"INSERT INTO provider_credentials
                 (team_id, added_by, provider, encrypted_api_key, key_nonce, key_version, key_last_four, label, base_url, supports_stream_options)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id, team_id, added_by, provider, encrypted_api_key, key_nonce, key_version,
                         key_last_four, label, is_active, created_at, base_url, supports_stream_options"#,
            team_id,
            added_by,
            provider,
            encrypted_api_key,
            key_nonce,
            key_version,
            key_last_four,
            label,
            base_url,
            supports_stream_options
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn list_for_team(&self, team_id: Uuid) -> Result<Vec<ProviderCredential>, RepoError> {
        sqlx::query_as!(
            ProviderCredential,
            r#"SELECT id, team_id, added_by, provider, encrypted_api_key, key_nonce, key_version,
                      key_last_four, label, is_active, created_at, base_url, supports_stream_options
               FROM provider_credentials WHERE team_id = $1 ORDER BY created_at DESC"#,
            team_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_id_for_team(
        &self,
        id: Uuid,
        team_id: Uuid,
    ) -> Result<Option<ProviderCredential>, RepoError> {
        sqlx::query_as!(
            ProviderCredential,
            r#"SELECT id, team_id, added_by, provider, encrypted_api_key, key_nonce, key_version,
                      key_last_four, label, is_active, created_at, base_url, supports_stream_options
               FROM provider_credentials WHERE id = $1 AND team_id = $2"#,
            id,
            team_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<ProviderCredential>, RepoError> {
        sqlx::query_as!(
            ProviderCredential,
            r#"SELECT id, team_id, added_by, provider, encrypted_api_key, key_nonce, key_version,
                      key_last_four, label, is_active, created_at, base_url, supports_stream_options
               FROM provider_credentials WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Soft delete — sets `is_active = false`. Returns the number of rows
    /// affected (0 means the id didn't belong to this team / doesn't exist).
    pub async fn deactivate(&self, id: Uuid, team_id: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"UPDATE provider_credentials SET is_active = false WHERE id = $1 AND team_id = $2"#,
            id,
            team_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    /// Hard delete — only ever removes an already-deactivated row (guarded by
    /// `is_active = false`) so a credential still in use can't be purged out
    /// from under a live routing path without being deactivated first.
    /// Returns the number of rows affected.
    pub async fn delete(&self, id: Uuid, team_id: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"DELETE FROM provider_credentials WHERE id = $1 AND team_id = $2 AND is_active = false"#,
            id,
            team_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    pub async fn count_active_for_team(&self, team_id: Uuid) -> Result<i64, RepoError> {
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count!" FROM provider_credentials WHERE team_id = $1 AND is_active = true"#,
            team_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(row.count)
    }
}
