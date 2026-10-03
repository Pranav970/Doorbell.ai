use std::sync::Arc;

use gateway_db::VirtualKeyRepo;

use super::stage::Stage;
use super::stages::{AuthenticateStage, RoutingStage};
use crate::routing::RoutingEngine;

/// Always the same two stages, in the same order. Per-project configurable
/// chains are Phase 3 work (need the config-snapshot epic's
/// project/environment concept first) — this is deliberately not
/// configurable yet.
pub fn build_chain(
    virtual_key_repo: VirtualKeyRepo,
    routing: RoutingEngine,
) -> Vec<Arc<dyn Stage>> {
    vec![
        Arc::new(AuthenticateStage::new(virtual_key_repo)),
        Arc::new(RoutingStage::new(routing)),
    ]
}

#[cfg(test)]
mod tests {
    use gateway_db::{ProviderCredentialRepo, RoutingRuleRepo};
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn build_chain_returns_authenticate_then_routing(pool: PgPool) {
        let chain = build_chain(
            VirtualKeyRepo::new(pool.clone()),
            RoutingEngine::new(
                RoutingRuleRepo::new(pool.clone()),
                ProviderCredentialRepo::new(pool),
            ),
        );

        let names: Vec<&str> = chain.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["authenticate", "routing"]);
    }
}
