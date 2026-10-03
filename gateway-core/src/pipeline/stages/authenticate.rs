use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

use gateway_common::AppError;
use gateway_db::VirtualKeyRepo;

use crate::context::ResolvedVirtualKey;
use crate::pipeline::stage::{FailureMode, Outcome, RequestCtx, ResponseCtx, Stage};

/// The raw bearer token presented on the request, fed into `ctx.extensions`
/// by the caller before the chain runs (gateway-core has no notion of HTTP
/// headers) — `None` if the caller found no `Authorization: Bearer ...`
/// header at all, matching `extract_bearer`'s old missing/malformed case.
#[derive(Debug, Clone)]
pub struct RawBearerToken(pub Option<String>);

fn hash_virtual_key(raw: &str) -> String {
    STANDARD.encode(Sha256::digest(raw.as_bytes()))
}

/// Ports `gateway-api`'s old `virtual_key_auth` middleware verbatim: resolve
/// the presented key, reject anything not found or not active, and record
/// the resulting tenant on `ctx` for later stages.
pub struct AuthenticateStage {
    virtual_key_repo: VirtualKeyRepo,
}

impl AuthenticateStage {
    pub fn new(virtual_key_repo: VirtualKeyRepo) -> Self {
        Self { virtual_key_repo }
    }
}

#[async_trait]
impl Stage for AuthenticateStage {
    fn name(&self) -> &'static str {
        "authenticate"
    }

    fn failure_mode(&self) -> FailureMode {
        FailureMode::FailClosed
    }

    async fn on_request(&self, ctx: &mut RequestCtx) -> Outcome {
        let raw_key = match ctx.extensions.get::<RawBearerToken>() {
            Some(RawBearerToken(Some(key))) => key.clone(),
            _ => return Outcome::Reject(AppError::Unauthorized),
        };

        let key_hash = hash_virtual_key(&raw_key);
        let vk = match self.virtual_key_repo.find_by_hash(&key_hash).await {
            Ok(Some(vk)) => vk,
            Ok(None) => return Outcome::Reject(AppError::Unauthorized),
            Err(e) => return Outcome::Fail(e.into()),
        };

        if vk.status != "active" {
            return Outcome::Reject(AppError::Unauthorized);
        }

        // Fire-and-forget, same as today — a slow write never adds latency.
        let repo = self.virtual_key_repo.clone();
        let vk_id = vk.id;
        tokio::spawn(async move {
            let _ = repo.touch_last_used(vk_id).await;
        });

        ctx.tenant = Some(ResolvedVirtualKey {
            virtual_key_id: vk.id,
            team_id: vk.team_id,
            issued_by: vk.issued_by,
        });
        Outcome::Continue
    }

    async fn on_response(&self, _ctx: &mut ResponseCtx) -> Outcome {
        Outcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with_token(token: Option<&str>) -> RequestCtx {
        let mut ctx = RequestCtx::new("req-1".to_string(), None);
        ctx.extensions
            .insert(RawBearerToken(token.map(|t| t.to_string())));
        ctx
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn continues_with_a_valid_active_virtual_key(pool: sqlx::PgPool) {
        let team = gateway_testkit::setup_team(&pool, "auth-stage-1@example.com").await;
        let vk = gateway_testkit::create_virtual_key(&pool, team.team_id, team.user_id, None).await;
        let stage = AuthenticateStage::new(VirtualKeyRepo::new(pool));

        let mut ctx = ctx_with_token(Some(&vk.plaintext));
        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(outcome, Outcome::Continue));
        let tenant = ctx.tenant.expect("tenant set on success");
        assert_eq!(tenant.virtual_key_id, vk.row.id);
        assert_eq!(tenant.team_id, team.team_id);
        assert_eq!(tenant.issued_by, team.user_id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn rejects_a_missing_bearer_token(pool: sqlx::PgPool) {
        let stage = AuthenticateStage::new(VirtualKeyRepo::new(pool));
        let mut ctx = ctx_with_token(None);

        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(outcome, Outcome::Reject(AppError::Unauthorized)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn rejects_an_unknown_virtual_key(pool: sqlx::PgPool) {
        let stage = AuthenticateStage::new(VirtualKeyRepo::new(pool));
        let mut ctx = ctx_with_token(Some("vk_live_not_a_real_key"));

        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(outcome, Outcome::Reject(AppError::Unauthorized)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn rejects_a_revoked_virtual_key(pool: sqlx::PgPool) {
        let team = gateway_testkit::setup_team(&pool, "auth-stage-2@example.com").await;
        let vk = gateway_testkit::create_virtual_key(&pool, team.team_id, team.user_id, None).await;
        let repo = VirtualKeyRepo::new(pool.clone());
        repo.revoke(vk.row.id, team.team_id).await.unwrap();
        let stage = AuthenticateStage::new(repo);

        let mut ctx = ctx_with_token(Some(&vk.plaintext));
        let outcome = stage.on_request(&mut ctx).await;

        assert!(matches!(outcome, Outcome::Reject(AppError::Unauthorized)));
    }
}
