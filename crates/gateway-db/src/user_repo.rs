use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{map_unique_violation, RepoError};
use crate::models::User;

#[derive(Clone)]
pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        email: &str,
        password_hash: &str,
        display_name: Option<&str>,
    ) -> Result<User, RepoError> {
        sqlx::query_as!(
            User,
            r#"INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3)
               RETURNING id, email, password_hash, display_name, created_at, updated_at, is_master_admin"#,
            email,
            password_hash,
            display_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_unique_violation(e, "email already registered"))
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        sqlx::query_as!(
            User,
            r#"SELECT id, email, password_hash, display_name, created_at, updated_at, is_master_admin
               FROM users WHERE LOWER(email) = LOWER($1)"#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, RepoError> {
        sqlx::query_as!(
            User,
            r#"SELECT id, email, password_hash, display_name, created_at, updated_at, is_master_admin
               FROM users WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
