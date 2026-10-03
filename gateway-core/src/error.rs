#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("no route found for the requested model")]
    NoRoute,
    /// Routing succeeded — the request reached a credential — but that
    /// credential names a provider with no built-in adapter and carries no
    /// `base_url`, so there is no endpoint to call. Distinct from `NoRoute`
    /// on purpose: reporting this as "no route for the model" sends people
    /// hunting through routing config for what is a missing endpoint.
    #[error("provider '{0}' has no endpoint configured: it has no built-in adapter, so its credential needs a base_url")]
    ProviderNotConfigured(String),
    #[error(transparent)]
    Repo(#[from] gateway_db::RepoError),
    #[error(transparent)]
    Crypto(#[from] gateway_crypto::CryptoError),
    #[error(transparent)]
    Provider(#[from] gateway_providers::ProviderError),
}

impl From<CoreError> for gateway_common::AppError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NoRoute => gateway_common::AppError::BadRequest(
                "no route found for the requested model".to_string(),
            ),
            CoreError::ProviderNotConfigured(ref provider) => {
                gateway_common::AppError::BadRequest(format!(
                    "provider '{provider}' has no endpoint configured: add a base_url to its credential"
                ))
            }
            CoreError::Repo(re) => re.into(),
            CoreError::Crypto(ce) => ce.into(),
            CoreError::Provider(pe) => pe.into(),
        }
    }
}
