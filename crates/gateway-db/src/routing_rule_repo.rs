use sqlx::PgPool;
use uuid::Uuid;

use crate::error::RepoError;
use crate::models::RoutingRule;

#[derive(Clone)]
pub struct RoutingRuleRepo {
    pool: PgPool,
}

impl RoutingRuleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Ordered by priority (ascending, lower = evaluated first), ties broken
    /// by insertion order. Both the routing engine and the CRUD API below
    /// read this.
    pub async fn list_for_virtual_key(
        &self,
        virtual_key_id: Uuid,
    ) -> Result<Vec<RoutingRule>, RepoError> {
        sqlx::query_as!(
            RoutingRule,
            r#"SELECT id, virtual_key_id, model_pattern, provider_credential_id, priority, created_at
               FROM routing_rules WHERE virtual_key_id = $1 ORDER BY priority ASC, created_at ASC"#,
            virtual_key_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Lets a team point an arbitrary `model_pattern` at any of its own BYOK
    /// credentials (custom/base_url providers included) without needing a
    /// hardcoded prefix entry for that vendor anywhere in the routing engine.
    pub async fn create(
        &self,
        virtual_key_id: Uuid,
        model_pattern: &str,
        provider_credential_id: Uuid,
        priority: i32,
    ) -> Result<RoutingRule, RepoError> {
        sqlx::query_as!(
            RoutingRule,
            r#"INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
               VALUES ($1, $2, $3, $4)
               RETURNING id, virtual_key_id, model_pattern, provider_credential_id, priority, created_at"#,
            virtual_key_id,
            model_pattern,
            provider_credential_id,
            priority
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Scoped by `virtual_key_id` so one team can never delete another
    /// team's rule. Returns the number of rows affected.
    pub async fn delete(&self, id: Uuid, virtual_key_id: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"DELETE FROM routing_rules WHERE id = $1 AND virtual_key_id = $2"#,
            id,
            virtual_key_id
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }
}
