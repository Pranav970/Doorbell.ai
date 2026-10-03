/// `Load` covers layered-loading failures (bad TOML/env syntax, a required
/// field missing entirely) — the underlying `config`/serde error message
/// already names the offending field. `Invalid` covers the semantic checks
/// in `AppConfig::validate` that serde's deserialize can't express (a
/// value that parses fine as a `String` but isn't a usable address, key, or
/// reachable database).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to load configuration: {0}")]
    Load(String),
    #[error("invalid configuration for `{field}`: {reason}")]
    Invalid { field: &'static str, reason: String },
}
