use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::Organization;

#[derive(Clone)]
pub struct OrgRepo {
    pool: PgPool,
}

impl OrgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        slug: &str,
        created_by: Uuid,
    ) -> Result<Organization, RepoError> {
        sqlx::query_as!(
            Organization,
            r#"INSERT INTO organizations (name, slug, created_by) VALUES ($1, $2, $3)
               RETURNING id, name, slug, created_by, created_at"#,
            name,
            slug,
            created_by
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "organization slug already taken"))
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Organization>, RepoError> {
        sqlx::query_as!(
            Organization,
            r#"SELECT id, name, slug, created_by, created_at FROM organizations WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// No visibility restriction in Phase 1 — every authenticated user can
    /// browse all orgs/teams to submit a join request.
    pub async fn list_all(&self) -> Result<Vec<Organization>, RepoError> {
        sqlx::query_as!(
            Organization,
            r#"SELECT id, name, slug, created_by, created_at FROM organizations ORDER BY created_at ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
