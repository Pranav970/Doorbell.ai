use sqlx::PgPool;
use uuid::Uuid;

use crate::error::RepoError;
use crate::models::NewRequestLog;

#[derive(Clone)]
pub struct RequestLogRepo {
    pool: PgPool,
}

/// One row per distinct (provider credential, model) pair used within the
/// summarized scope — the "which keys are in use" admin analytics view.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KeyUsageSummary {
    pub team_id: Uuid,
    pub provider: String,
    pub model: String,
    pub request_count: i64,
    pub error_count: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One row per request — backs the team "Logs" view.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RequestLogEntry {
    pub id: Uuid,
    pub virtual_key_id: Option<Uuid>,
    pub provider: String,
    pub model: String,
    pub status_code: i32,
    pub latency_ms: i32,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub fallback_count: i32,
}

impl RequestLogRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, log: NewRequestLog) -> Result<Uuid, RepoError> {
        let row = sqlx::query!(
            r#"INSERT INTO request_logs
                 (user_id, team_id, virtual_key_id, provider, model, status_code, latency_ms, tokens_in, tokens_out, error_message, fallback_count)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING id"#,
            log.user_id,
            log.team_id,
            log.virtual_key_id,
            log.provider,
            log.model,
            log.status_code,
            log.latency_ms,
            log.tokens_in,
            log.tokens_out,
            log.error_message,
            log.fallback_count
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepoError::from)?;
        Ok(row.id)
    }

    /// Usage rolled up by (team, provider, model) across every team in an
    /// org — backs the admin "Keys & Usage" analytics view.
    pub async fn summary_for_org(&self, org_id: Uuid) -> Result<Vec<KeyUsageSummary>, RepoError> {
        sqlx::query_as!(
            KeyUsageSummary,
            r#"SELECT rl.team_id as "team_id!", rl.provider, rl.model,
                      COUNT(*) as "request_count!",
                      COUNT(*) FILTER (WHERE rl.status_code >= 400) as "error_count!",
                      COALESCE(SUM(rl.tokens_in), 0) as "tokens_in!",
                      COALESCE(SUM(rl.tokens_out), 0) as "tokens_out!",
                      MAX(rl.created_at) as last_used_at
               FROM request_logs rl
               JOIN teams t ON t.id = rl.team_id
               WHERE t.org_id = $1
               GROUP BY rl.team_id, rl.provider, rl.model
               ORDER BY rl.team_id, rl.provider, rl.model"#,
            org_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Same rollup, scoped to a single team.
    pub async fn summary_for_team(&self, team_id: Uuid) -> Result<Vec<KeyUsageSummary>, RepoError> {
        sqlx::query_as!(
            KeyUsageSummary,
            r#"SELECT rl.team_id as "team_id!", rl.provider, rl.model,
                      COUNT(*) as "request_count!",
                      COUNT(*) FILTER (WHERE rl.status_code >= 400) as "error_count!",
                      COALESCE(SUM(rl.tokens_in), 0) as "tokens_in!",
                      COALESCE(SUM(rl.tokens_out), 0) as "tokens_out!",
                      MAX(rl.created_at) as last_used_at
               FROM request_logs rl
               WHERE rl.team_id = $1
               GROUP BY rl.team_id, rl.provider, rl.model
               ORDER BY rl.provider, rl.model"#,
            team_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    /// Most recent requests for a team, newest first — backs the "Logs" tab.
    pub async fn list_recent_for_team(
        &self,
        team_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RequestLogEntry>, RepoError> {
        sqlx::query_as!(
            RequestLogEntry,
            r#"SELECT id, virtual_key_id, provider, model, status_code, latency_ms,
                      tokens_in, tokens_out, error_message, created_at, fallback_count
               FROM request_logs
               WHERE team_id = $1
               ORDER BY created_at DESC
               LIMIT $2"#,
            team_id,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
