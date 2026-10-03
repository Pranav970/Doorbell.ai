pub mod error;

pub use error::ConfigError;

use std::net::SocketAddr;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Deserialize;

const ENCRYPTION_KEY_LEN: usize = 32;
const DB_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // NOTE: all fields are `pub` so tests can construct an `AppConfig`
    // directly rather than only via `AppConfig::load()` from the process
    // environment.
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    pub database_url: String,

    pub jwt_secret: String,
    #[serde(default = "default_jwt_access_ttl")]
    pub jwt_access_ttl_seconds: i64,
    #[serde(default = "default_refresh_ttl")]
    pub refresh_token_ttl_seconds: i64,

    /// 32 raw bytes, base64-encoded. See `gateway_crypto::EnvKeyCipher`.
    pub encryption_master_key: String,

    #[serde(default = "default_openai_base_url")]
    pub openai_base_url: String,
    #[serde(default = "default_anthropic_base_url")]
    pub anthropic_base_url: String,
    #[serde(default = "default_anthropic_api_version")]
    pub anthropic_api_version: String,
    #[serde(default = "default_gemini_base_url")]
    pub gemini_base_url: String,

    #[serde(default)]
    pub cors_allowed_origins: String,

    /// The one account that governs every org/team platform-wide. Matched
    /// case-insensitively against `users.email`; the matching account is
    /// treated as master even with no `org_members`/`team_memberships` rows
    /// anywhere. Config rather than a hardcoded constant so a different
    /// deployment can designate a different address without a rebuild.
    ///
    /// The `users.is_master_admin` column is the other, independent way to
    /// grant this (either one is sufficient) — that one survives an email
    /// change, this one works on a fresh database with no manual SQL.
    #[serde(default = "default_master_admin_email")]
    pub master_admin_email: String,

    /// Ceiling on how long graceful shutdown waits for in-flight requests
    /// to finish after SIGTERM before forcing an exit. See
    /// `gateway_telemetry::shutdown`.
    #[serde(default = "default_shutdown_grace_period_seconds")]
    pub shutdown_grace_period_seconds: u64,

    /// The three defaults below match `sqlx::PgPoolOptions::default()`
    /// exactly (10 / 30s / 10min) — made explicit and configurable here
    /// rather than left implicit, not changed. See `gateway_db::PoolConfig`.
    #[serde(default = "default_db_max_connections")]
    pub db_max_connections: u32,
    #[serde(default = "default_db_acquire_timeout_seconds")]
    pub db_acquire_timeout_seconds: u64,
    #[serde(default = "default_db_idle_timeout_seconds")]
    pub db_idle_timeout_seconds: u64,
}

fn default_bind_addr() -> String {
    "0.0.0.0:8080".to_string()
}
fn default_master_admin_email() -> String {
    "master@cloud.in".to_string()
}
fn default_jwt_access_ttl() -> i64 {
    900
}
fn default_refresh_ttl() -> i64 {
    2_592_000
}
fn default_openai_base_url() -> String {
    "https://api.openai.com".to_string()
}
fn default_anthropic_base_url() -> String {
    "https://api.anthropic.com".to_string()
}
fn default_anthropic_api_version() -> String {
    "2023-06-01".to_string()
}
fn default_gemini_base_url() -> String {
    "https://generativelanguage.googleapis.com".to_string()
}
fn default_shutdown_grace_period_seconds() -> u64 {
    30
}
fn default_db_max_connections() -> u32 {
    10
}
fn default_db_acquire_timeout_seconds() -> u64 {
    30
}
fn default_db_idle_timeout_seconds() -> u64 {
    600
}

impl AppConfig {
    /// Loads `.env` (if present) then reads settings from the process
    /// environment. Env vars always win over `.env` file contents (a
    /// `dotenvy` guarantee: it never overwrites a var that's already set),
    /// and any field missing from both falls back to its `#[serde(default)]`.
    /// No file layer beyond `.env` exists today — see `.env.example` for the
    /// full set of recognized variables.
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()
            .map_err(|e| ConfigError::Load(e.to_string()))?;
        cfg.try_deserialize()
            .map_err(|e| ConfigError::Load(e.to_string()))
    }

    /// `load()` followed by `validate()` — the entry point `main.rs` should
    /// call. Kept separate from `load()` so tests can exercise layering and
    /// validation independently.
    pub async fn load_and_validate() -> Result<Self, ConfigError> {
        let cfg = Self::load()?;
        cfg.validate().await?;
        Ok(cfg)
    }

    /// Semantic checks `serde`'s deserialize can't express on its own: a
    /// value that's a well-formed `String` but not a usable address, key,
    /// or reachable database. Cheap synchronous checks run first so an
    /// obviously-wrong local value fails fast, before the network call.
    pub async fn validate(&self) -> Result<(), ConfigError> {
        validate_bind_addr(&self.bind_addr)?;
        validate_encryption_master_key(&self.encryption_master_key)?;
        validate_database_reachable(&self.database_url).await?;
        Ok(())
    }

    pub fn cors_origins(&self) -> Vec<String> {
        self.cors_allowed_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

fn validate_bind_addr(addr: &str) -> Result<(), ConfigError> {
    addr.parse::<SocketAddr>()
        .map(|_| ())
        .map_err(|e| ConfigError::Invalid {
            field: "bind_addr",
            reason: format!("not a valid host:port address: {e}"),
        })
}

/// Duplicates `gateway_crypto::EnvKeyCipher`'s base64/length check rather
/// than depending on `gateway-crypto` — `gateway-common` depends on this
/// crate (to keep re-exporting `AppConfig`), and `gateway-crypto` depends on
/// `gateway-common`, so `gateway-config -> gateway-crypto` would cycle.
fn validate_encryption_master_key(key: &str) -> Result<(), ConfigError> {
    let bytes = BASE64
        .decode(key.trim())
        .map_err(|e| ConfigError::Invalid {
            field: "encryption_master_key",
            reason: format!("not valid base64: {e}"),
        })?;
    if bytes.len() != ENCRYPTION_KEY_LEN {
        return Err(ConfigError::Invalid {
            field: "encryption_master_key",
            reason: format!(
                "must decode to exactly {ENCRYPTION_KEY_LEN} raw bytes, got {}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

/// A real, short-timeout connection attempt — not just URL-format
/// validation — so an unreachable Postgres host fails at startup with a
/// clear message instead of surfacing on the first request. Same reasoning
/// as the encryption-key check: this crate talks to `sqlx` directly rather
/// than depending on `gateway-db`, to keep the dependency direction pointing
/// one way.
async fn validate_database_reachable(database_url: &str) -> Result<(), ConfigError> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(DB_CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map(|_pool| ())
        .map_err(|e| ConfigError::Invalid {
            field: "database_url",
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn load_from_toml(toml: &str) -> Result<AppConfig, ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .map_err(|e| ConfigError::Load(e.to_string()))?;
        cfg.try_deserialize()
            .map_err(|e| ConfigError::Load(e.to_string()))
    }

    #[test]
    fn missing_required_field_is_a_clear_error() {
        let err = load_from_toml("database_url = \"postgres://x\"").unwrap_err();
        assert!(
            err.to_string().contains("jwt_secret"),
            "error should name the missing field, got: {err}"
        );
    }

    #[test]
    fn defaults_fill_in_optional_fields() {
        let cfg = load_from_toml(
            r#"
            database_url = "postgres://gateway:gateway@localhost/gateway"
            jwt_secret = "test-secret"
            encryption_master_key = "dGVzdC1rZXktMzItYnl0ZXMtbG9uZy1wbGVhc2U="
            "#,
        )
        .unwrap();
        assert_eq!(cfg.bind_addr, "0.0.0.0:8080");
        assert_eq!(cfg.jwt_access_ttl_seconds, 900);
        assert_eq!(cfg.openai_base_url, "https://api.openai.com");
    }

    #[test]
    fn cors_origins_splits_and_trims() {
        let cfg = load_from_toml(
            r#"
            database_url = "postgres://gateway:gateway@localhost/gateway"
            jwt_secret = "test-secret"
            encryption_master_key = "dGVzdC1rZXktMzItYnl0ZXMtbG9uZy1wbGVhc2U="
            cors_allowed_origins = "http://localhost:3000, http://example.com"
            "#,
        )
        .unwrap();
        assert_eq!(
            cfg.cors_origins(),
            vec!["http://localhost:3000", "http://example.com"]
        );
    }

    // Process env is global, shared mutable state — this mutex keeps the one
    // test below that touches it from racing other tests in the same run.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    #[test]
    fn env_var_overrides_dotenv_file_value() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("gateway-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dotenv_path = dir.join(".env");
        std::fs::write(&dotenv_path, "BIND_ADDR=0.0.0.0:1111\n").unwrap();

        std::env::set_var("BIND_ADDR", "0.0.0.0:2222");
        dotenvy::from_path(&dotenv_path).ok();

        assert_eq!(
            std::env::var("BIND_ADDR").unwrap(),
            "0.0.0.0:2222",
            "a real env var must win over the same key in a .env file"
        );

        std::env::remove_var("BIND_ADDR");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_bind_addr_that_is_not_a_socket_address() {
        let err = validate_bind_addr("not-an-address").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "bind_addr",
                ..
            }
        ));
    }

    #[test]
    fn rejects_encryption_master_key_that_is_not_valid_base64() {
        let err = validate_encryption_master_key("not-valid-base64!!!").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "encryption_master_key",
                ..
            }
        ));
    }

    #[test]
    fn rejects_encryption_master_key_of_wrong_decoded_length() {
        let short_key = BASE64.encode([1u8; 16]);
        let err = validate_encryption_master_key(&short_key).unwrap_err();
        match err {
            ConfigError::Invalid {
                field: "encryption_master_key",
                reason,
            } => {
                assert!(
                    reason.contains("16"),
                    "reason should mention the actual length: {reason}"
                );
                assert!(
                    reason.contains("32"),
                    "reason should mention the expected length: {reason}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn accepts_a_valid_encryption_master_key() {
        let key = BASE64.encode([7u8; 32]);
        assert!(validate_encryption_master_key(&key).is_ok());
    }

    #[tokio::test]
    async fn rejects_unreachable_database_url() {
        // Nothing listens on port 1 (a privileged, unbound port) — connection
        // refused comes back near-instantly, no timeout wait needed.
        let err = validate_database_reachable("postgres://gateway:gateway@127.0.0.1:1/gateway")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "database_url",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn accepts_a_reachable_database_url() {
        // Same convention as the rest of the workspace (#[sqlx::test] et al.):
        // this needs a live Postgres at TEST_DATABASE_URL, no #[ignore] gate.
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://gateway:gateway@localhost:5432/gateway".to_string());
        validate_database_reachable(&url).await.unwrap();
    }
}
