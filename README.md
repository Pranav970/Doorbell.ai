# LLM Gateway (Phase 1)

A BYOK (bring-your-own-key) LLM gateway/router. Customers get one stable
virtual API key and one endpoint; the gateway routes each request to
whichever provider/model they've configured, using the customer's own
provider credentials (OpenAI, Anthropic, Gemini).

Phase 1 covers auth, orgs/teams with join-request approval, provider key
management, virtual key issuance, an OpenAI-compatible routing/proxy engine,
per-key routing-rule overrides (including generic BYOK vendors beyond
OpenAI/Anthropic/Gemini), and cross-provider fallback. Rate limiting,
budgets, PII redaction, response caching, streaming, and billing are still
out of scope — see [Phase 2 extension points](#phase-2-extension-points) for
where each one hooks into the architecture that's already in place.

## Architecture

A Cargo workspace, not a monolith binary — each crate has one job:

```
crates/
├── gateway-common/    # shared errors, AppConfig, canonical OpenAI-compatible DTOs, tracing init
├── gateway-db/        # sqlx repositories + migrations
├── gateway-crypto/    # AES-256-GCM envelope encryption for provider keys at rest
├── gateway-auth/      # argon2 password hashing, JWT issuance/validation, refresh token rotation
├── gateway-providers/ # LlmProvider trait + OpenAI/Anthropic/Gemini adapters + registry
├── gateway-core/      # routing engine (model -> provider resolution) + request pipeline orchestration
└── gateway-api/       # Axum HTTP server: routes, Tower middleware stack, OpenAPI docs
```

Dependency direction is one-way: `gateway-common` underlies everything;
`gateway-providers` depends only on `gateway-common` (pure HTTP + translation
logic, no DB knowledge) so it can never depend on `gateway-core`;
`gateway-core` composes `gateway-db` + `gateway-crypto` + `gateway-providers`;
`gateway-api` wires all of it into Axum.

### Provider extensibility

`provider_credentials.provider` is a plain `TEXT` column, not a DB enum or
lookup table. Validity is enforced in application code against
`gateway_providers::ProviderRegistry`. Adding a new provider means: one new
adapter module implementing `LlmProvider`, one line in
`ProviderRegistry::new`, and a config entry for its base URL — no migration,
no changes to routing/pipeline code.

### Request pipeline (`POST /v1/chat/completions`)

Implemented as a Tower/Axum middleware stack, outermost to innermost:

1. `SetRequestIdLayer` / `TraceLayer` — request-id + structured tracing (never logs the `Authorization` header or request/response bodies).
2. `pipeline_stages` — runs `gateway-core`'s fixed `[AuthenticateStage, RoutingStage]` chain (a `Stage`/`Executor` abstraction, not raw middleware functions): resolves the virtual key and checks it's active, then resolves the target provider credential across the primary model and any `fallback_models` (a `routing_rules` match, else the team's sole active credential — no model-prefix guessing) — rejecting only if none of them route. Rejects `stream: true` (streaming is a Phase 2 trait stub only). Inserts `ResolvedVirtualKey` into request extensions.
3. `chat_completions` handler — calls `FallbackEngine` (which re-resolves routing per candidate model itself, independent of step 2's pre-check — see `CLAUDE.md`'s forbidden-patterns note on this), decrypts the credential in-memory for the duration of the call, invokes the matching `LlmProvider` adapter, writes a `request_logs` row (fire-and-forget), returns the OpenAI-compatible response.

## Local setup

Prerequisites: Rust (stable), Docker, `sqlx-cli`.

```bash
# 1. Install sqlx-cli if you don't have it
cargo install sqlx-cli --no-default-features --features rustls,postgres

# 2. Start Postgres
docker compose up -d

# 3. Configure environment
cp .env.example .env
# Generate real secrets rather than using the placeholders:
#   openssl rand -base64 48   -> JWT_SECRET
#   openssl rand -base64 32   -> ENCRYPTION_MASTER_KEY (must decode to exactly 32 bytes)

# 4. Run migrations
export DATABASE_URL=postgres://gateway:gateway@localhost:5432/gateway
sqlx migrate run

# 5. Run the server
cargo run -p gateway-api
```

The server listens on `BIND_ADDR` (default `0.0.0.0:8080`). API docs are
served at `/swagger-ui`, the raw OpenAPI spec at `/api-docs/openapi.json`.

### Running tests

Tests use `#[sqlx::test]`, which provisions an ephemeral database per test
against `DATABASE_URL` (the same Postgres started by `docker compose up`) and
tears it down automatically — no separate test database setup needed.

```bash
docker compose up -d
cargo test --workspace
```

Provider HTTP calls are never made against real vendors in tests — the
end-to-end proxy test (`crates/gateway-api/tests/chat_completions_test.rs`)
points the OpenAI adapter's configurable `base_url` at a `wiremock` server, so
the real `reqwest` + translation code runs against a fake network hop.

## API walkthrough

```bash
BASE=http://localhost:8080

# Sign up and log in
curl -s -X POST $BASE/auth/signup -H 'content-type: application/json' \
  -d '{"email":"you@example.com","password":"a-strong-password"}'

TOKENS=$(curl -s -X POST $BASE/auth/login -H 'content-type: application/json' \
  -d '{"email":"you@example.com","password":"a-strong-password"}')
ACCESS_TOKEN=$(echo $TOKENS | jq -r .access_token)

# Add your own OpenAI key (BYOK) — encrypted at rest, never returned in full again
curl -s -X POST $BASE/api/provider-keys -H "authorization: Bearer $ACCESS_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"provider":"openai","api_key":"sk-...","label":"prod"}'

# Issue a virtual key — shown once, only its hash is stored thereafter
VK=$(curl -s -X POST $BASE/api/virtual-keys -H "authorization: Bearer $ACCESS_TOKEN" \
  -H 'content-type: application/json' -d '{"name":"my-app"}')
VIRTUAL_KEY=$(echo $VK | jq -r .key)

# Call the gateway exactly like the OpenAI SDK — swap base_url and API key, nothing else
curl -s -X POST $BASE/v1/chat/completions -H "authorization: Bearer $VIRTUAL_KEY" \
  -H 'content-type: application/json' \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hello"}]}'
```

Full route list, request/response shapes: `/swagger-ui`.

## Environment variables

See `.env.example` for the full list with defaults. The two that must be
generated (not left as placeholders) are `JWT_SECRET` and
`ENCRYPTION_MASTER_KEY` — the latter must decode to exactly 32 raw bytes.

## Phase 2 extension points

Nothing below is implemented in Phase 1, but the architecture has a concrete
seam for each:

| Feature | Hook point |
|---|---|
| Rate limiting / login throttling | New `Stage` in `gateway-core`'s pipeline chain, right after `AuthenticateStage` (keyed on `ResolvedVirtualKey`) or wrapping `/auth/login`, reading counters from Redis (add `deadpool-redis` + a connection pool in `AppState` when this is actually built — dropped from Phase 1 since nothing read from it). Short-circuits with 429 before any DB/provider work. |
| Budgets / cost controls | Same position as rate limiting — reads spend-to-date for the resolved key, 402/429s if over budget. |
| Response/semantic caching | New `Stage` inserted after `RoutingStage` (once model + provider are resolved), keyed on a hash of the normalized request + provider. A cache hit skips the provider call entirely. |
| PII redaction / guardrails | `Stage` operating on the now-typed request body, right before or after `RoutingStage`; can also run on the response via `on_response` before it's returned. |
| Multi-provider fallback | Already built — `gateway-core::FallbackEngine`, called from the `chat_completions` handler: on `ProviderError::Upstream`/`Timeout`/context-length, walks the caller's `fallback_models` list, independently routing each attempt. |
| Streaming | `LlmProvider::chat_completion_stream` is already on the trait (default-implemented as `NotImplemented`); implementing it per-adapter is additive, not a trait break. `RoutingStage` currently rejects `stream: true` with 400. |
| Teams/RBAC, SSO, usage analytics, billing | New crates/tables layered on top of `users`/`virtual_keys`; `request_logs` is already the seed data set for analytics and billing. |

## Data model

`users`, `refresh_tokens` (rotation with theft-detection via `family_id`),
`provider_credentials` (AES-256-GCM encrypted, `key_version` for future
KMS/Vault migration), `virtual_keys` (sha256 hash for auth + an envelope-encrypted copy so an
authorized team member can retrieve one via `/reveal`), `routing_rules`
(per-virtual-key model-pattern overrides, CRUD via `/api/teams/{team_id}/virtual-keys/{virtual_key_id}/routing-rules`),
`request_logs` (audit trail, outlives a revoked virtual key).

## Routing custom/BYOK vendors (Mistral, NVIDIA NIM, Groq, etc.)

Only OpenAI, Anthropic and Gemini have a registered adapter — they speak
their own wire formats and carry a default endpoint from config. That is the
*only* thing the list decides; it is not a restriction on which providers are
usable, and no model-name prefix routes by convention any more. Any other vendor that speaks
the same OpenAI-compatible wire format — Mistral, NVIDIA NIM, Groq, Together,
a self-hosted server — is reached generically via a credential's `base_url`
(`pipeline.rs`: a `base_url` on the credential always uses the generic OpenAI
adapter against that host, no vendor-specific code needed).

If that's the team's *only* active provider credential, `RoutingEngine`
routes every model there automatically — no `routing_rules` row needed, since
there's nothing to disambiguate. An explicit `routing_rules` row is required
once a team has more than one credential, **for every provider equally**:
there is no built-in `gpt-`/`claude-`/`gemini-` shortcut that would let some
vendors skip the step.

A credential naming a provider with no adapter and no `base_url` is rejected
when you add it — it would have no endpoint to call.

`base_url` must be just the bare host — the adapter always appends
`/v1/chat/completions` itself (`gateway-providers/src/adapters/openai.rs`),
so a value that already includes `/v1` or `/chat/completions` produces a
broken doubled-up path:

```bash
# 1. Add the credential with base_url pointed at the vendor's endpoint
CRED=$(curl -s -X POST $BASE/api/teams/$TEAM_ID/provider-keys -H "authorization: Bearer $ACCESS_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"provider":"nvidia","api_key":"nvapi-...","base_url":"https://integrate.api.nvidia.com"}')
CRED_ID=$(echo $CRED | jq -r .id)

# 2. Point a model-name pattern at it (trailing '*' does a prefix match)
curl -s -X POST $BASE/api/teams/$TEAM_ID/virtual-keys/$VK_ID/routing-rules \
  -H "authorization: Bearer $ACCESS_TOKEN" -H 'content-type: application/json' \
  -d '{"model_pattern":"nvidia/*","provider_credential_id":"'$CRED_ID'","priority":0}'
```

Calling `/v1/chat/completions` with a matching `model` now routes to that
credential — no code changes, no adapter, nothing hardcoded to edit.
