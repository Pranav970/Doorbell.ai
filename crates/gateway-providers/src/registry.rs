use std::collections::HashMap;
use std::sync::Arc;

use crate::adapters::anthropic::AnthropicProvider;
use crate::adapters::gemini::GeminiProvider;
use crate::adapters::openai::OpenAiProvider;
use crate::trait_def::LlmProvider;

#[derive(Debug, Clone)]
pub struct ProviderEndpointsConfig {
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub anthropic_api_version: String,
    pub gemini_base_url: String,
}

/// Constructor-time map from provider name to adapter. Routing/pipeline code
/// only ever calls `get(name)` — adding a 4th provider means a new adapter
/// module, one line here, and a config entry, with no other code changes.
pub struct ProviderRegistry {
    providers: HashMap<&'static str, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new(cfg: &ProviderEndpointsConfig) -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn LlmProvider>> = HashMap::new();
        providers.insert(
            "openai",
            Arc::new(OpenAiProvider::new(cfg.openai_base_url.clone())),
        );
        providers.insert(
            "anthropic",
            Arc::new(AnthropicProvider::new(
                cfg.anthropic_base_url.clone(),
                cfg.anthropic_api_version.clone(),
            )),
        );
        providers.insert(
            "gemini",
            Arc::new(GeminiProvider::new(cfg.gemini_base_url.clone())),
        );
        Self { providers }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn is_valid(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}
