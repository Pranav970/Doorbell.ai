use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::Team;

#[derive(Clone)]
pub struct TeamRepo {
    pool: PgPool,
}

impl TeamRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        org_id: Uuid,
        name: &str,
        slug: &str,
        created_by: Uuid,
    ) -> Result<Team, RepoError> {
        sqlx::query_as!(
            Team,
            r#"INSERT INTO teams (org_id, name, slug, created_by) VALUES ($1, $2, $3, $4)
               RETURNING id, org_id, name, slug, created_by, created_at"#,
            org_id,
            name,
            slug,
            created_by
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "team slug already taken in this organization"))
    }

    pub async fn list_for_org(&self, org_id: Uuid) -> Result<Vec<Team>, RepoError> {
        sqlx::query_as!(
            Team,
            r#"SELECT id, org_id, name, slug, created_by, created_at
               FROM teams WHERE org_id = $1 ORDER BY created_at ASC"#,
            org_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Team>, RepoError> {
        sqlx::query_as!(
            Team,
            r#"SELECT id, org_id, name, slug, created_by, created_at FROM teams WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
