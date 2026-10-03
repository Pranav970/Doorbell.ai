use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::OrgMember;

#[derive(Clone)]
pub struct OrgMemberRepo {
    pool: PgPool,
}

/// An org membership joined with the organization's display name — what the
/// `/api/me` and signup/join-browse screens actually need to render.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrgMembershipView {
    pub org_id: Uuid,
    pub org_name: String,
    pub role: String,
}

impl OrgMemberRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<OrgMember, RepoError> {
        sqlx::query_as!(
            OrgMember,
            r#"INSERT INTO org_members (org_id, user_id, role) VALUES ($1, $2, $3)
               RETURNING id, org_id, user_id, role, created_at"#,
            org_id,
            user_id,
            role
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "already a member of this organization"))
    }

    pub async fn find_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, RepoError> {
        let row = sqlx::query!(
            r#"SELECT role FROM org_members WHERE org_id = $1 AND user_id = $2"#,
            org_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(row.map(|r| r.role))
    }

    pub async fn list_orgs_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OrgMembershipView>, RepoError> {
        sqlx::query_as!(
            OrgMembershipView,
            r#"SELECT om.org_id, o.name as org_name, om.role
               FROM org_members om JOIN organizations o ON o.id = om.org_id
               WHERE om.user_id = $1 ORDER BY om.created_at ASC"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
