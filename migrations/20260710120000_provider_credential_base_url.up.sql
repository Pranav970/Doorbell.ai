-- Lets a BYOK credential point at a different host than the provider's
-- built-in adapter default (e.g. Groq/Together/a self-hosted server, all of
-- which speak the OpenAI-compatible wire format). NULL means "use the
-- provider's registered adapter/base URL as-is" — unchanged behavior for
-- existing openai/anthropic/gemini credentials.
ALTER TABLE provider_credentials ADD COLUMN base_url TEXT;
