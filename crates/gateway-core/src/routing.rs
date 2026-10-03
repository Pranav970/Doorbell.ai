use uuid::Uuid;

use gateway_db::{ProviderCredentialRepo, RoutingRuleRepo};

use crate::context::ResolvedRoute;
use crate::error::CoreError;

#[derive(Clone)]
pub struct RoutingEngine {
    routing_rules: RoutingRuleRepo,
    provider_credentials: ProviderCredentialRepo,
}

impl RoutingEngine {
    pub fn new(
        routing_rules: RoutingRuleRepo,
        provider_credentials: ProviderCredentialRepo,
    ) -> Self {
        Self {
            routing_rules,
            provider_credentials,
        }
    }

    /// Resolution comes entirely from what the customer configured. Every
    /// provider is treated identically — there is no vendor whose models
    /// route by convention while others need setup.
    ///
    /// 1. An explicit `routing_rules` match (ordered by priority, lowest
    ///    first). Works for any provider, named or self-hosted.
    /// 2. If the team has exactly one active provider credential, route
    ///    there regardless of provider or model name — nothing to
    ///    disambiguate.
    ///
    /// Anything else is `NoRoute`: two or more credentials and no rule
    /// saying which model goes where is a genuine ambiguity that the gateway
    /// must not guess at.
    ///
    /// **The `gpt-`/`claude-`/`gemini-` prefix map that used to sit between
    /// these two steps is gone.** It hardcoded three vendors into the hot
    /// path, so those three routed with no configuration while every other
    /// provider — Cohere, Mistral, a private deployment — silently failed
    /// the moment a team added a second credential. Reintroducing any
    /// build-time model→provider table here re-creates that asymmetry;
    /// per-credential model configuration is the way to make this automatic
    /// again for everyone at once.
    pub async fn resolve(
        &self,
        team_id: Uuid,
        virtual_key_id: Uuid,
        model: &str,
    ) -> Result<ResolvedRoute, CoreError> {
        let rules = self
            .routing_rules
            .list_for_virtual_key(virtual_key_id)
            .await?;
        for rule in rules {
            if pattern_matches(&rule.model_pattern, model) {
                let cred = self
                    .provider_credentials
                    .find_by_id(rule.provider_credential_id)
                    .await?
                    .filter(|c| c.is_active)
                    .ok_or(CoreError::NoRoute)?;
                return Ok(ResolvedRoute {
                    provider_credential_id: cred.id,
                    provider_name: cred.provider,
                });
            }
        }

        let active_creds: Vec<_> = self
            .provider_credentials
            .list_for_team(team_id)
            .await?
            .into_iter()
            .filter(|c| c.is_active)
            .collect();

        // No rule matched. The model name is deliberately not consulted from
        // here on — inspecting it to guess a vendor is exactly the
        // favouritism this layer no longer does. Only the shape of the team's
        // own configuration can decide.
        let [only] = active_creds.as_slice() else {
            return Err(CoreError::NoRoute);
        };

        Ok(ResolvedRoute {
            provider_credential_id: only.id,
            provider_name: only.provider.clone(),
        })
    }
}

/// `pattern` may end in `*` for a prefix match (e.g. "gpt-4*"); otherwise
/// it's compared for exact equality against an alias model name.
fn pattern_matches(pattern: &str, model: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => model.starts_with(prefix),
        None => pattern == model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gateway_db::{OrgRepo, TeamRepo, UserRepo};

    /// Creates a throwaway user + org + team, returning (user_id, team_id) —
    /// the minimal setup every routing test needs now that credentials and
    /// virtual keys are team-scoped rather than user-scoped.
    async fn setup_team(pool: &sqlx::PgPool, email: &str) -> (uuid::Uuid, uuid::Uuid) {
        let users = UserRepo::new(pool.clone());
        let orgs = OrgRepo::new(pool.clone());
        let teams = TeamRepo::new(pool.clone());

        let user = users.create(email, "hash", None).await.unwrap();
        let org = orgs
            .create("Test Org", &format!("org-{email}"), user.id)
            .await
            .unwrap();
        let team = teams
            .create(org.id, "Test Team", &format!("team-{email}"), user.id)
            .await
            .unwrap();
        (user.id, team.id)
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn an_explicit_rule_routes_regardless_of_model_naming(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "router@example.com").await;
        let creds_repo = ProviderCredentialRepo::new(pool.clone());
        let rules_repo = RoutingRuleRepo::new(pool.clone());
        let vkeys = gateway_db::VirtualKeyRepo::new(pool.clone());

        let _openai_cred = creds_repo
            .create(
                team_id,
                user_id,
                "openai",
                b"ct",
                b"nonce123456",
                1,
                "aaaa",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let anthropic_cred = creds_repo
            .create(
                team_id,
                user_id,
                "anthropic",
                b"ct",
                b"nonce123456",
                1,
                "bbbb",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let vk = vkeys
            .create(team_id, user_id, "vkhash1", "vk_live_A", None, None)
            .await
            .unwrap();

        // Explicit rule: route "gpt-4-custom" to the ANTHROPIC credential.
        // The model name looks like an OpenAI one; nothing in the gateway
        // cares, because only the rule decides.
        sqlx::query!(
            "INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
             VALUES ($1, 'gpt-4-custom', $2, 0)",
            vk.id,
            anthropic_cred.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let engine = RoutingEngine::new(rules_repo, creds_repo);

        let resolved = engine
            .resolve(team_id, vk.id, "gpt-4-custom")
            .await
            .unwrap();
        assert_eq!(resolved.provider_credential_id, anthropic_cred.id);
        assert_eq!(resolved.provider_name, "anthropic");

        // ...and a model with no rule does NOT fall back to a vendor guess,
        // even for the most conventionally-named model there is. Two
        // credentials, no rule, so the team has to say which one serves it.
        assert!(matches!(
            engine.resolve(team_id, vk.id, "gpt-4o").await,
            Err(CoreError::NoRoute)
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn lower_priority_number_wins_when_multiple_rules_match(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "priority@example.com").await;
        let creds_repo = ProviderCredentialRepo::new(pool.clone());
        let rules_repo = RoutingRuleRepo::new(pool.clone());
        let vkeys = gateway_db::VirtualKeyRepo::new(pool.clone());

        let low_priority_cred = creds_repo
            .create(
                team_id,
                user_id,
                "openai",
                b"ct",
                b"nonce123456",
                1,
                "cccc",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let high_priority_cred = creds_repo
            .create(
                team_id,
                user_id,
                "anthropic",
                b"ct",
                b"nonce123456",
                1,
                "dddd",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let vk = vkeys
            .create(team_id, user_id, "vkhash2", "vk_live_B", None, None)
            .await
            .unwrap();

        sqlx::query!(
            "INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
             VALUES ($1, 'my-alias', $2, 10), ($1, 'my-alias', $3, 1)",
            vk.id,
            low_priority_cred.id,
            high_priority_cred.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let engine = RoutingEngine::new(rules_repo, creds_repo);
        let resolved = engine.resolve(team_id, vk.id, "my-alias").await.unwrap();
        assert_eq!(resolved.provider_credential_id, high_priority_cred.id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn unresolvable_model_is_an_error(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "noroute@example.com").await;
        let creds_repo = ProviderCredentialRepo::new(pool.clone());
        let rules_repo = RoutingRuleRepo::new(pool.clone());
        let vkeys = gateway_db::VirtualKeyRepo::new(pool.clone());

        let vk = vkeys
            .create(team_id, user_id, "vkhash3", "vk_live_C", None, None)
            .await
            .unwrap();

        let engine = RoutingEngine::new(rules_repo, creds_repo);
        let err = engine
            .resolve(team_id, vk.id, "some-unknown-model")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NoRoute));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn sole_active_credential_handles_any_model_from_any_vendor(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "sole-cred@example.com").await;
        let creds_repo = ProviderCredentialRepo::new(pool.clone());
        let rules_repo = RoutingRuleRepo::new(pool.clone());
        let vkeys = gateway_db::VirtualKeyRepo::new(pool.clone());

        // A vendor with no adapter of its own — routable with no rule
        // precisely because it is the team's only credential, the same way
        // an OpenAI-only team is.
        let mistral_cred = creds_repo
            .create(
                team_id,
                user_id,
                "mistral",
                b"ct",
                b"nonce123456",
                1,
                "eeee",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let vk = vkeys
            .create(team_id, user_id, "vkhash4", "vk_live_D", None, None)
            .await
            .unwrap();

        let engine = RoutingEngine::new(rules_repo, creds_repo);
        let resolved = engine
            .resolve(team_id, vk.id, "mistral-large-latest")
            .await
            .unwrap();
        assert_eq!(resolved.provider_credential_id, mistral_cred.id);
        assert_eq!(resolved.provider_name, "mistral");

        // No routing_rules row was created — this works purely from "there's
        // only one candidate." A model name that doesn't even hint at the
        // vendor still resolves the same way.
        let resolved = engine
            .resolve(team_id, vk.id, "my-custom-alias")
            .await
            .unwrap();
        assert_eq!(resolved.provider_credential_id, mistral_cred.id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn multiple_non_default_credentials_with_no_match_is_ambiguous(pool: sqlx::PgPool) {
        let (user_id, team_id) = setup_team(&pool, "ambiguous@example.com").await;
        let creds_repo = ProviderCredentialRepo::new(pool.clone());
        let rules_repo = RoutingRuleRepo::new(pool.clone());
        let vkeys = gateway_db::VirtualKeyRepo::new(pool.clone());

        creds_repo
            .create(
                team_id,
                user_id,
                "mistral",
                b"ct",
                b"nonce123456",
                1,
                "ffff",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        creds_repo
            .create(
                team_id,
                user_id,
                "cohere",
                b"ct",
                b"nonce123456",
                1,
                "gggg",
                None,
                None,
                true,
            )
            .await
            .unwrap();
        let vk = vkeys
            .create(team_id, user_id, "vkhash5", "vk_live_E", None, None)
            .await
            .unwrap();

        // With two candidates and nothing to disambiguate them (no rule, no
        // prefix match), resolution must still fail rather than guess.
        let engine = RoutingEngine::new(rules_repo, creds_repo);
        let err = engine
            .resolve(team_id, vk.id, "some-unlisted-model")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::NoRoute));
    }
}
