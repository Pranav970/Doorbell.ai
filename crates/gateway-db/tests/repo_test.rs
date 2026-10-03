use gateway_db::models::NewRequestLog;
use gateway_db::{
    OrgRepo, ProviderCredentialRepo, RefreshTokenRepo, RequestLogRepo, RoutingRuleRepo, TeamRepo,
    UserRepo, VirtualKeyRepo,
};
use uuid::Uuid;

/// Creates a throwaway user + org + team, returning (user_id, team_id) —
/// the minimal setup most repo tests need now that credentials/virtual keys
/// are team-scoped rather than user-scoped.
async fn setup_team(pool: &sqlx::PgPool, email: &str) -> (Uuid, Uuid) {
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
async fn user_repo_create_and_lookup(pool: sqlx::PgPool) {
    let repo = UserRepo::new(pool);
    let user = repo
        .create("Alice@Example.com", "hash123", Some("Alice"))
        .await
        .unwrap();
    assert_eq!(user.email, "Alice@Example.com");
    assert_eq!(user.display_name, Some("Alice".to_string()));

    // case-insensitive lookup
    let found = repo
        .find_by_email("alice@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, user.id);

    let by_id = repo.find_by_id(user.id).await.unwrap().unwrap();
    assert_eq!(by_id.email, user.email);

    // duplicate email (any case) is a conflict
    let err = repo
        .create("ALICE@EXAMPLE.COM", "other", None)
        .await
        .unwrap_err();
    assert!(matches!(err, gateway_db::RepoError::Conflict(_)));
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_token_rotation_and_family_revocation(pool: sqlx::PgPool) {
    let users = UserRepo::new(pool.clone());
    let tokens = RefreshTokenRepo::new(pool);

    let user = users.create("bob@example.com", "hash", None).await.unwrap();
    let family_id = Uuid::new_v4();
    let expires = chrono::Utc::now() + chrono::Duration::days(30);

    let t1 = tokens
        .insert(user.id, "hash-of-token-1", family_id, expires)
        .await
        .unwrap();
    let t2 = tokens
        .insert(user.id, "hash-of-token-2", family_id, expires)
        .await
        .unwrap();

    tokens.revoke(t1.id, Some(t2.id)).await.unwrap();
    let reloaded = tokens
        .find_by_hash("hash-of-token-1")
        .await
        .unwrap()
        .unwrap();
    assert!(reloaded.revoked_at.is_some());
    assert_eq!(reloaded.replaced_by, Some(t2.id));

    tokens.revoke_family(family_id).await.unwrap();
    let t2_reloaded = tokens
        .find_by_hash("hash-of-token-2")
        .await
        .unwrap()
        .unwrap();
    assert!(t2_reloaded.revoked_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn provider_credential_create_list_deactivate(pool: sqlx::PgPool) {
    let (user_id, team_id) = setup_team(&pool, "carol@example.com").await;
    let creds = ProviderCredentialRepo::new(pool);

    let cred = creds
        .create(
            team_id,
            user_id,
            "openai",
            b"ciphertext",
            b"nonce123456",
            1,
            "abcd",
            Some("prod"),
            None,
            true,
        )
        .await
        .unwrap();
    assert_eq!(cred.key_last_four, "abcd");
    assert!(cred.is_active);

    assert_eq!(creds.count_active_for_team(team_id).await.unwrap(), 1);

    let list = creds.list_for_team(team_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let affected = creds.deactivate(cred.id, team_id).await.unwrap();
    assert_eq!(affected, 1);
    assert_eq!(creds.count_active_for_team(team_id).await.unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn virtual_key_create_list_revoke(pool: sqlx::PgPool) {
    let (user_id, team_id) = setup_team(&pool, "dave@example.com").await;
    let vkeys = VirtualKeyRepo::new(pool);

    let vk = vkeys
        .create(
            team_id,
            user_id,
            "hash-of-vk",
            "vk_live_AbCd",
            Some("test-key"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(vk.status, "active");

    let found = vkeys.find_by_hash("hash-of-vk").await.unwrap().unwrap();
    assert_eq!(found.id, vk.id);

    let list = vkeys.list_for_team(team_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let affected = vkeys.revoke(vk.id, team_id).await.unwrap();
    assert_eq!(affected, 1);
    let reloaded = vkeys
        .find_by_id_for_team(vk.id, team_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.status, "revoked");
}

#[sqlx::test(migrations = "../../migrations")]
async fn routing_rules_ordered_by_priority(pool: sqlx::PgPool) {
    let (user_id, team_id) = setup_team(&pool, "erin@example.com").await;
    let creds = ProviderCredentialRepo::new(pool.clone());
    let vkeys = VirtualKeyRepo::new(pool.clone());
    let rules = RoutingRuleRepo::new(pool.clone());

    let cred = creds
        .create(
            team_id,
            user_id,
            "anthropic",
            b"ct",
            b"nonce123456",
            1,
            "wxyz",
            None,
            None,
            true,
        )
        .await
        .unwrap();
    let vk = vkeys
        .create(team_id, user_id, "hash-of-vk2", "vk_live_Ef", None, None)
        .await
        .unwrap();

    sqlx::query!(
        r#"INSERT INTO routing_rules (virtual_key_id, model_pattern, provider_credential_id, priority)
           VALUES ($1, 'my-alias', $2, 5), ($1, 'my-alias-2', $2, 1)"#,
        vk.id,
        cred.id
    )
    .execute(&pool)
    .await
    .unwrap();

    let list = rules.list_for_virtual_key(vk.id).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].model_pattern, "my-alias-2"); // priority 1 first
    assert_eq!(list[1].model_pattern, "my-alias");
}

#[sqlx::test(migrations = "../../migrations")]
async fn request_log_insert(pool: sqlx::PgPool) {
    let (user_id, team_id) = setup_team(&pool, "frank@example.com").await;
    let logs = RequestLogRepo::new(pool);

    let id = logs
        .insert(NewRequestLog {
            user_id,
            team_id: Some(team_id),
            virtual_key_id: None,
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            status_code: 200,
            latency_ms: 42,
            tokens_in: Some(10),
            tokens_out: Some(20),
            error_message: None,
            fallback_count: 0,
        })
        .await
        .unwrap();
    assert!(!id.is_nil());
}

#[sqlx::test(migrations = "../../migrations")]
async fn request_log_list_recent_for_team_scopes_and_orders(pool: sqlx::PgPool) {
    let (user_id, team_id) = setup_team(&pool, "grace@example.com").await;
    let (_, other_team_id) = setup_team(&pool, "henry@example.com").await;
    let logs = RequestLogRepo::new(pool);

    let base = NewRequestLog {
        user_id,
        team_id: Some(team_id),
        virtual_key_id: None,
        provider: "openai".to_string(),
        model: "gpt-4o".to_string(),
        status_code: 200,
        latency_ms: 10,
        tokens_in: Some(1),
        tokens_out: Some(1),
        error_message: None,
        fallback_count: 0,
    };
    logs.insert(NewRequestLog {
        model: "first".to_string(),
        ..base.clone()
    })
    .await
    .unwrap();
    logs.insert(NewRequestLog {
        model: "second".to_string(),
        ..base.clone()
    })
    .await
    .unwrap();
    logs.insert(NewRequestLog {
        team_id: Some(other_team_id),
        model: "other-team".to_string(),
        ..base
    })
    .await
    .unwrap();

    let recent = logs.list_recent_for_team(team_id, 100).await.unwrap();
    let models: Vec<&str> = recent.iter().map(|r| r.model.as_str()).collect();
    assert_eq!(models, vec!["second", "first"]);

    let limited = logs.list_recent_for_team(team_id, 1).await.unwrap();
    assert_eq!(limited.len(), 1);
}
