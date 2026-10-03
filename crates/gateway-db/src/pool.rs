use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Pool tuning, plumbed in from `gateway_config::AppConfig` by the caller
/// (`gateway-db` deliberately doesn't depend on `gateway-config` itself —
/// see `CLAUDE.md`'s dependency rule — so this is a small local struct
/// rather than taking the whole `AppConfig`).
#[derive(Debug, Clone, Copy)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
}

pub async fn build_pool(
    database_url: &str,
    pool_config: &PoolConfig,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(pool_config.max_connections)
        .acquire_timeout(pool_config.acquire_timeout)
        .idle_timeout(pool_config.idle_timeout)
        .connect(database_url)
        .await
}

/// A cheap reachability check against an already-open pool — for `/readyz`,
/// not startup validation (that's `gateway-config`'s job, which opens its
/// own short-lived connection). Reuses whatever connection the pool already
/// has rather than opening a new one on every poll.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_fails_when_postgres_is_unreachable() {
        // `connect_lazy` defers the actual connection attempt to first use,
        // so this doesn't fail at construction the way `connect()` would —
        // exactly what's needed to test `ping()`'s failure path without an
        // already-broken pool. The individual connection attempt to port 1
        // is refused near-instantly, but sqlx's pool *retries* acquiring a
        // connection until `acquire_timeout` elapses (30s by default) rather
        // than surfacing the first failure — an explicit short timeout here
        // is what actually makes this test fast, not the refused connection
        // itself.
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(2))
            .connect_lazy("postgres://gateway:gateway@127.0.0.1:1/gateway")
            .unwrap();
        assert!(ping(&pool).await.is_err());
    }
}
