use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::TeamMembership;

#[derive(Clone)]
pub struct TeamMembershipRepo {
    pool: PgPool,
}

/// A membership joined with the team's display name — what the `/api/me`
/// and member-dashboard screens need to render.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamMembershipView {
    pub team_id: Uuid,
    pub team_name: String,
    pub org_id: Uuid,
    pub status: String,
}

/// A pending join request joined with the team name and requester's email —
/// what the admin "Requests" tab renders, aggregated across every team in
/// an org so the admin doesn't have to open each team individually.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingRequestView {
    pub id: Uuid,
    pub team_id: Uuid,
    pub team_name: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub requested_at: DateTime<Utc>,
}

/// An approved member joined with their user info — what the team detail
/// page renders.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApprovedMemberView {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
}

impl TeamMembershipRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_request(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<TeamMembership, RepoError> {
        sqlx::query_as!(
            TeamMembership,
            r#"INSERT INTO team_memberships (team_id, user_id) VALUES ($1, $2)
               RETURNING id, team_id, user_id, status, requested_at, decided_at, decided_by"#,
            team_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "a membership request for this team already exists"))
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<TeamMembership>, RepoError> {
        sqlx::query_as!(
            TeamMembership,
            r#"SELECT id, team_id, user_id, status, requested_at, decided_at, decided_by
               FROM team_memberships WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Returns the number of rows affected (0 means the request didn't
    /// exist or was no longer pending).
    pub async fn approve(&self, id: Uuid, decided_by: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"UPDATE team_memberships SET status = 'approved', decided_at = now(), decided_by = $2
               WHERE id = $1 AND status = 'pending'"#,
            id,
            decided_by
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    pub async fn reject(&self, id: Uuid, decided_by: Uuid) -> Result<u64, RepoError> {
        let result = sqlx::query!(
            r#"UPDATE team_memberships SET status = 'rejected', decided_at = now(), decided_by = $2
               WHERE id = $1 AND status = 'pending'"#,
            id,
            decided_by
        )
        .execute(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(result.rows_affected())
    }

    pub async fn find_approved(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamMembership>, RepoError> {
        sqlx::query_as!(
            TeamMembership,
            r#"SELECT id, team_id, user_id, status, requested_at, decided_at, decided_by
               FROM team_memberships WHERE team_id = $1 AND user_id = $2 AND status = 'approved'"#,
            team_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Pending requests across every team in an org — backs the admin
    /// "Requests" tab.
    pub async fn list_pending_for_org(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PendingRequestView>, RepoError> {
        sqlx::query_as!(
            PendingRequestView,
            r#"SELECT tm.id, tm.team_id, t.name as team_name, tm.user_id, u.email as user_email, tm.requested_at
               FROM team_memberships tm
               JOIN teams t ON t.id = tm.team_id
               JOIN users u ON u.id = tm.user_id
               WHERE t.org_id = $1 AND tm.status = 'pending'
               ORDER BY tm.requested_at ASC"#,
            org_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn list_approved_for_team(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<ApprovedMemberView>, RepoError> {
        sqlx::query_as!(
            ApprovedMemberView,
            r#"SELECT u.id as user_id, u.email, u.display_name
               FROM team_memberships tm JOIN users u ON u.id = tm.user_id
               WHERE tm.team_id = $1 AND tm.status = 'approved'
               ORDER BY tm.decided_at ASC"#,
            team_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<TeamMembershipView>, RepoError> {
        sqlx::query_as!(
            TeamMembershipView,
            r#"SELECT tm.team_id, t.name as team_name, t.org_id, tm.status
               FROM team_memberships tm JOIN teams t ON t.id = tm.team_id
               WHERE tm.user_id = $1 ORDER BY tm.requested_at ASC"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
