use std::sync::Arc;

use gateway_common::canonical::{ChatCompletionRequest, ChatCompletionResponse};
use gateway_crypto::{EncryptedBlob, KeyEnvelopeCipher};
use gateway_db::ProviderCredentialRepo;
use gateway_providers::adapters::openai::OpenAiProvider;
use gateway_providers::{ChatStream, LlmProvider, ProviderRegistry};
use secrecy::SecretString;

use crate::context::ResolvedRoute;
use crate::error::CoreError;

/// The leaf of the request pipeline: given an already-resolved route,
/// decrypts the matching provider credential in-memory (plaintext lives only
/// on this call's stack, for the duration of the outbound HTTP call) and
/// invokes the matching `LlmProvider` adapter.
///
/// A single-shot call to a single resolved route — `FallbackEngine` is the
/// retrying decorator on top of this that walks a caller's fallback model
/// list on a retryable error.
#[derive(Clone)]
pub struct ProviderCaller {
    provider_credentials: ProviderCredentialRepo,
    crypto: Arc<dyn KeyEnvelopeCipher>,
    registry: Arc<ProviderRegistry>,
}

impl ProviderCaller {
    pub fn new(
        provider_credentials: ProviderCredentialRepo,
        crypto: Arc<dyn KeyEnvelopeCipher>,
        registry: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            provider_credentials,
            crypto,
            registry,
        }
    }

    pub async fn call(
        &self,
        route: &ResolvedRoute,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, CoreError> {
        let (adapter, api_key) = self.resolve(route).await?;
        let response = adapter.chat_completion(&api_key, request).await?;
        Ok(response)
    }

    /// The streaming twin of `call`. Resolves the credential and adapter
    /// identically — so a `base_url` BYOK vendor streams over the OpenAI
    /// wire format exactly the way it already completes over it — and hands
    /// back the live chunk stream without consuming any of it.
    ///
    /// An error here means the stream never opened (bad credentials, upstream
    /// 5xx, a provider with no adapter). That is what lets `FallbackEngine`
    /// treat streaming failover as safe: by the time this returns `Ok`,
    /// nothing has been written to the client yet, and once it does, no
    /// further failover is attempted.
    pub async fn call_stream(
        &self,
        route: &ResolvedRoute,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream, CoreError> {
        let (adapter, api_key) = self.resolve(route).await?;
        let stream = adapter.chat_completion_stream(&api_key, request).await?;
        Ok(stream)
    }

    /// Decrypts the route's credential and picks the adapter that can reach
    /// it. Shared by both call paths so streaming and non-streaming can
    /// never drift on which endpoint a credential resolves to.
    ///
    /// Endpoint resolution is driven by the credential the user configured,
    /// not by any fixed list of supported vendors:
    ///
    ///   base_url set  -> call that host over the OpenAI wire format.
    ///                    Works for ANY provider — Cohere, Mistral, Groq,
    ///                    Together, a self-hosted server, something that
    ///                    doesn't exist yet.
    ///   base_url unset-> the provider must have a native-protocol adapter
    ///                    (openai/anthropic/gemini speak their own wire
    ///                    formats and carry a default endpoint from
    ///                    AppConfig), otherwise there is nothing to call.
    ///
    /// The registry is therefore a set of protocol translators, not a
    /// whitelist of permitted providers.
    async fn resolve(
        &self,
        route: &ResolvedRoute,
    ) -> Result<(Arc<dyn LlmProvider>, SecretString), CoreError> {
        let cred = self
            .provider_credentials
            .find_by_id(route.provider_credential_id)
            .await?
            .ok_or(CoreError::NoRoute)?;

        let blob = EncryptedBlob {
            nonce: cred.key_nonce,
            ciphertext: cred.encrypted_api_key,
            key_version: cred.key_version,
        };
        let api_key = self.crypto.decrypt(&blob)?;

        let adapter: Arc<dyn LlmProvider> = match &cred.base_url {
            Some(base_url) => Arc::new(
                OpenAiProvider::new(base_url.clone())
                    // A BYOK host that rejects unknown fields opts out here;
                    // the column defaults true, so this is a no-op for every
                    // credential that hasn't explicitly turned it off.
                    .with_stream_options(cred.supports_stream_options),
            ),
            None => self
                .registry
                .get(&route.provider_name)
                .ok_or_else(|| CoreError::ProviderNotConfigured(route.provider_name.clone()))?,
        };
        Ok((adapter, api_key))
    }
}
