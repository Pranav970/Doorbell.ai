# Prismaxis (LLM Gateway) — Handover Doc

Generated 2026-08-06 from branch `phase-1-frontend`, originally at `2efc9c3`;
updated same-day after the fixes below (uncommitted at doc-update time — see
git status/diff before merging). A BYOK (bring-your-own-key) LLM gateway:
customers get one virtual API key, the gateway routes each call to their own
OpenAI/Anthropic/Gemini/custom provider credentials.

## Tech stack

**Backend** — Rust, Cargo workspace (7 crates, ~9.5k LOC in `crates/`)
- `axum` 0.7 (HTTP) + `tower`/`tower-http` (request-id, tracing, CORS, limits)
- `sqlx` 0.8 (Postgres, compile-time checked queries, `sqlx::test` for integration tests)
- `tokio` (async runtime)
- `jsonwebtoken` (JWT) + `argon2` (password hashing)
- `aes-gcm` + `zeroize` + `secrecy` (AES-256-GCM envelope encryption for provider keys at rest)
- `reqwest` (rustls-tls) for outbound provider calls
- `utoipa` + `utoipa-swagger-ui` (OpenAPI, served at `/swagger-ui`)
- `wiremock` (dev-dep) — fakes provider HTTP in tests

**Frontend** — `frontend/`, React 19 + TypeScript + Vite
- `react-router-dom` 7, `@tanstack/react-query` 5, Tailwind CSS 4 (`@tailwindcss/postcss`)
- `oxlint` for linting, `tsc -b` for type-checking (build = typecheck + `vite build`)
- No test runner configured for the frontend

**Infra**
- Postgres via `docker-compose.yml` (Redis removed — see Problems #4)
- `sqlx-cli` for migrations (plain SQL, up/down pairs in `migrations/`)

## Codebase structure

```
.
├── Cargo.toml                  # workspace root
├── docker-compose.yml          # Postgres + Redis
├── migrations/                 # 4 sqlx migrations (up/down)
├── .env.example                # backend config template
├── crates/
│   ├── gateway-common/         # AppConfig, AppError, canonical OpenAI-compatible DTOs, tracing init
│   ├── gateway-db/             # sqlx repos: org/team/user/provider_credential/virtual_key/routing_rule/request_log/refresh_token
│   ├── gateway-crypto/         # AES-256-GCM envelope cipher for provider keys at rest
│   ├── gateway-auth/           # argon2 hashing, JWT issuance/validation, refresh-token rotation
│   ├── gateway-providers/      # LlmProvider trait + openai/anthropic/gemini adapters + registry
│   ├── gateway-core/           # RoutingEngine (model -> credential) + FallbackEngine + request pipeline
│   └── gateway-api/            # Axum server: routes/, middleware/, authz.rs, openapi.rs, state.rs, main.rs
│       ├── src/routes/         # auth, me, orgs, teams, team_membership, analytics, provider_keys, virtual_keys, routing_rules, gateway
│       └── tests/              # one integration test file per route module + chat_completions_test.rs (wiremock)
└── frontend/
    └── src/
        ├── pages/              # LoginPage, SignupPage, AdminPage, MemberPage
        ├── components/          # Shell, ThemeToggle, UsageTable, icons
        ├── context/             # AuthContext, ThemeContext
        └── lib/                 # api.ts (fetch wrapper), types.ts
```

Dependency direction (one-way): `gateway-common` underlies everything →
`gateway-providers` (pure HTTP/translation, no DB) → `gateway-core` (composes
db + crypto + providers) → `gateway-api` (wires into Axum).

## Data model

`orgs`, `org_members`, `teams`, `team_memberships` (join-request/approval
flow), `users`, `refresh_tokens` (rotation w/ theft detection via
`family_id`), `provider_credentials` (AES-256-GCM encrypted, per-team,
optional `base_url` for BYOK/custom vendors), `virtual_keys` (hashed, shown
once), `routing_rules` (per-virtual-key model-pattern → credential
overrides, with priority), `request_logs` (audit trail + fallback count,
outlives revoked keys).

## Request flow — `POST /v1/chat/completions`

Tower/Axum middleware stack: `SetRequestIdLayer`/`TraceLayer` →
`virtual_key_auth` (resolves + validates virtual key) → `routing_layer`
(resolves provider credential via `routing_rules` or model-prefix
fallback) → `chat_completions` handler (decrypts credential in-memory,
calls `FallbackEngine`, which tries the primary model then
`fallback_models` in order via the matching `LlmProvider` adapter, writes a
`request_logs` row, returns the OpenAI-compatible response with a
`fallback` summary of what was attempted).

## Local setup (see `DEMO.md` for the full walkthrough)

```bash
docker compose up -d
cp .env.example .env   # generate JWT_SECRET + ENCRYPTION_MASTER_KEY via openssl
export DATABASE_URL=postgres://gateway:gateway@localhost:5432/gateway
sqlx migrate run
cargo run -p gateway-api        # http://localhost:8080, docs at /swagger-ui

cd frontend && cp .env.example .env && npm install && npm run dev   # http://localhost:5173
```

Verified this session: `cargo build --workspace` succeeds clean (no
warnings). Frontend `tsc -b --noEmit` passes clean. `cargo test --workspace`
run against a live Postgres (Docker started, migrations applied) — **all 62
tests pass**, including a real bug that surfaced and got fixed (see Problems
#1 below).

## Problems — resolved this session

1. **Failing test: `fallback::tests::a_404_on_the_primary_triggers_fallback`.**
   Real bug, not a doc issue — `cargo test --workspace` was actually run
   (Docker started, Postgres + migrations up) and this test failed. Root
   cause: the test gives a team two active provider credentials (gemini +
   mistral) and asks the router to fall back to a `mistral-*` model with no
   `routing_rules` row disambiguating it — which `RoutingEngine::resolve`
   correctly treats as ambiguous (see
   `routing::tests::multiple_non_default_credentials_with_no_match_is_ambiguous`
   for the same rule tested directly). Fixed by inserting the missing
   `routing_rules` row in the test, matching the pattern
   `routing_rule_beats_default_prefix_map` already uses. All 62 tests pass
   now.
2. **README/DEMO.md were stale relative to the actual code.** They described
   orgs, teams/RBAC, routing-rule overrides, and multi-provider fallback as
   Phase-2/"out of scope" with only a "hook point" in place, even though all
   four are fully implemented and exercised by `DEMO.md`'s own walkthrough.
   Rewrote the relevant intro paragraph and Phase 2 table row.
3. **`sqlx-postgres` 0.7.4 future-incompat warning** ("contains code that
   will be rejected by a future version of Rust") — fixed for real, not
   just documented: bumped `sqlx` 0.7 → 0.8.6 across the workspace,
   regenerated `.sqlx/` against a live Postgres (`cargo sqlx prepare
   --workspace -- --tests`), full test suite still green. Warning is gone
   from `cargo build`.
4. **Redis was dead weight.** `deadpool-redis` was a workspace dependency
   and `AppConfig.redis_url` was parsed from env, but no code ever created
   a pool or issued a command — `docker-compose.yml` started a Redis
   container nothing talked to, and `.env.example`'s comment claiming a
   "connection pool is created and health-checked at startup" was false.
   Removed entirely (dependency, config field, docker service, env var,
   README/DEMO mentions) rather than wiring up an unused pool — nothing
   reads it yet. `README.md`'s Phase 2 table now says to add
   `deadpool-redis` back when rate limiting/budgets is actually built.

## Problems — still open (intentional Phase 2 scope, not bugs)

5. **Streaming is unimplemented.** `LlmProvider::chat_completion_stream`
   default-implements to `ProviderError::NotImplemented("streaming")`
   (`gateway-providers/src/trait_def.rs`), and `routing_layer` rejects
   `stream: true` with 400. No adapter overrides it. Deliberately deferred
   per `README.md`'s Phase 2 table — implementing it per-adapter is
   additive, not a trait break, whenever it's prioritized.
6. **No rate limiting, budgets, PII redaction/guardrails, caching, billing,
   or SSO.** None have any code yet; all are documented hook points in
   `README.md`'s Phase 2 table. Rate limiting/budgets will need Redis
   re-added (see #4) as part of that work, not before.
7. **Frontend has no test suite.** `oxlint` (lint) and `tsc -b` (typecheck)
   run in `npm run build`, but there's no unit/component/e2e test runner
   configured — regressions in `AdminPage`/`MemberPage` flows are only
   caught by hand-walking `DEMO.md`. Not fixed here: adding a test
   framework to a zero-test frontend is a scope decision for whoever picks
   this up, not a bug fix.

## Useful references

- `README.md` — architecture rationale, provider-extensibility model,
  custom/BYOK vendor routing recipe (treat the Phase-2 table with the
  caveat in Problem #1 above).
- `DEMO.md` — step-by-step demo script covering the org/team/BYOK/virtual-key/
  fallback flow end to end.
- `/swagger-ui` (server running) — live route list and request/response shapes.
- `graphify-out/GRAPH_REPORT.md` — dependency graph / god-node report, built
  from commit `72a97ff` (one commit behind current `HEAD`; run
  `graphify update .` to refresh).
