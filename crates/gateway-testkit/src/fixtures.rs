use base64::{engine::general_purpose::STANDARD, engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use gateway_crypto::{EnvKeyCipher, KeyEnvelopeCipher};
use gateway_db::models::{Organization, ProviderCredential, Team, User, VirtualKey};
use gateway_db::{OrgRepo, ProviderCredentialRepo, TeamRepo, UserRepo, VirtualKeyRepo};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// A fixed, valid 32-byte AES key so tests don't depend on env vars — shared
/// across every crate's test suite (was duplicated identically as
/// gateway-api's `test_master_key_b64` and gateway-core's `test_cipher`).
pub fn test_master_key_b64() -> String {
    STANDARD.encode([9u8; 32])
}

pub fn test_cipher() -> EnvKeyCipher {
    EnvKeyCipher::from_base64(&test_master_key_b64()).unwrap()
}

pub async fn create_user(pool: &PgPool, email: &str) -> User {
    UserRepo::new(pool.clone())
        .create(email, "test-hash", None)
        .await
        .unwrap()
}

pub async fn create_org(pool: &PgPool, name: &str, created_by: Uuid) -> Organization {
    OrgRepo::new(pool.clone())
        .create(name, &format!("org-{}", Uuid::new_v4()), created_by)
        .await
        .unwrap()
}

pub async fn create_team(pool: &PgPool, org_id: Uuid, name: &str, created_by: Uuid) -> Team {
    TeamRepo::new(pool.clone())
        .create(
            org_id,
            name,
            &format!("team-{}", Uuid::new_v4()),
            created_by,
        )
        .await
        .unwrap()
}

pub struct TeamFixture {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub team_id: Uuid,
}

/// Creates a throwaway user + org + team, returning their ids — the minimal
/// setup nearly every routing/fallback test needs. Generalizes the
/// byte-identical `setup_team` helper duplicated in
/// `gateway-core/src/fallback.rs` and `gateway-core/src/routing.rs`.
pub async fn setup_team(pool: &PgPool, email: &str) -> TeamFixture {
    let user = create_user(pool, email).await;
    let org = create_org(pool, "Test Org", user.id).await;
    let team = create_team(pool, org.id, "Test Team", user.id).await;
    TeamFixture {
        user_id: user.id,
        org_id: org.id,
        team_id: team.id,
    }
}

pub struct VirtualKeyFixture {
    pub row: VirtualKey,
    /// The raw `vk_live_...` value — use directly as a `Bearer` auth header.
    pub plaintext: String,
}

/// Persists a virtual key using the exact same generation scheme as
/// `gateway-api`'s `virtual_keys.rs::generate_virtual_key` (same
/// `vk_live_` + random-bytes shape, same sha256/base64 hash, same 16-char
/// prefix) so `.plaintext` authenticates for real through `virtual_key_auth`.
pub async fn create_virtual_key(
    pool: &PgPool,
    team_id: Uuid,
    issued_by: Uuid,
    name: Option<&str>,
) -> VirtualKeyFixture {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    let suffix = URL_SAFE_NO_PAD.encode(bytes);
    let plaintext = format!("vk_live_{suffix}");
    let key_hash = STANDARD.encode(Sha256::digest(plaintext.as_bytes()));
    let key_prefix: String = plaintext.chars().take(16).collect();

    let row = VirtualKeyRepo::new(pool.clone())
        .create(team_id, issued_by, &key_hash, &key_prefix, name, None)
        .await
        .unwrap();

    VirtualKeyFixture { row, plaintext }
}

fn last_four_chars(s: &str) -> String {
    let len = s.chars().count();
    if len <= 4 {
        s.to_string()
    } else {
        s.chars().skip(len - 4).collect()
    }
}

/// Encrypts `plaintext_api_key` with `cipher` and persists it — the
/// plaintext is caller-supplied (not generated) so tests can assert on it
/// (e.g. the auth header wiremock receives).
#[allow(clippy::too_many_arguments)]
pub async fn create_provider_credential(
    pool: &PgPool,
    cipher: &dyn KeyEnvelopeCipher,
    team_id: Uuid,
    added_by: Uuid,
    provider: &str,
    plaintext_api_key: &str,
    base_url: Option<&str>,
) -> ProviderCredential {
    create_provider_credential_with_stream_options(
        pool,
        cipher,
        team_id,
        added_by,
        provider,
        plaintext_api_key,
        base_url,
        true,
    )
    .await
}

/// As `create_provider_credential`, but lets a test pin
/// `supports_stream_options` — the flag that decides whether the OpenAI-wire
/// adapter sends `stream_options: {"include_usage": true}` on a stream.
#[allow(clippy::too_many_arguments)]
pub async fn create_provider_credential_with_stream_options(
    pool: &PgPool,
    cipher: &dyn KeyEnvelopeCipher,
    team_id: Uuid,
    added_by: Uuid,
    provider: &str,
    plaintext_api_key: &str,
    base_url: Option<&str>,
    supports_stream_options: bool,
) -> ProviderCredential {
    let blob = cipher.encrypt(plaintext_api_key).unwrap();
    ProviderCredentialRepo::new(pool.clone())
        .create(
            team_id,
            added_by,
            provider,
            &blob.ciphertext,
            &blob.nonce,
            blob.key_version,
            &last_four_chars(plaintext_api_key),
            None,
            base_url,
            supports_stream_options,
        )
        .await
        .unwrap()
}
