pub mod adapters;
pub mod error;
pub mod registry;
pub mod sse;
pub mod trait_def;

pub use error::ProviderError;
pub use registry::{ProviderEndpointsConfig, ProviderRegistry};
pub use trait_def::{ChatStream, LlmProvider};

/// Providers that speak their own wire format and therefore ship a protocol
/// adapter plus a default endpoint from `AppConfig`. **Not a list of
/// supported providers** — any vendor reachable over the OpenAI wire format
/// works with no entry here, via its credential's `base_url`. What this list
/// actually answers is the narrower question `has_adapter` asks: "can a
/// credential for this provider omit `base_url`?"
pub const ADAPTER_BACKED_PROVIDERS: &[&str] = &["openai", "anthropic", "gemini"];

/// Whether a credential naming this provider can be called with no
/// `base_url`. Everything else needs one — enforced when the credential is
/// added (`routes/provider_keys.rs`) rather than discovered at request time.
pub fn has_adapter(provider: &str) -> bool {
    ADAPTER_BACKED_PROVIDERS.contains(&provider)
}

/// Additional foundational-model providers offered as BYOK presets even
/// though they have no adapter yet — storing a credential for one is valid,
/// but routing a request to it needs a `routing_rules` override (no built-in
/// prefix mapping) until an adapter lands.
pub const PRESET_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "gemini",
    "mistral",
    "cohere",
    "meta-llama",
    "azure-openai",
    "deepseek",
    "xai-grok",
    "perplexity",
];

/// A credential's `provider` accepts any of the presets above, or a
/// user-named custom slug (letters, numbers, `-`/`_`, up to 40 chars) so
/// someone can BYOK a self-hosted or otherwise unlisted model.
pub fn is_valid_provider(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 40
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
