# CLAUDE.md — Prismaxis LLM Gateway

> **Living document.** Update this file at the end of every epic — crate
> map, phase status, and forbidden patterns drift fast in a workspace this
> size, and a stale version misleads the next session more than no doc at
> all.

BYOK (bring-your-own-key) LLM gateway: one virtual API key, one endpoint,
routed to the customer's own OpenAI/Anthropic/Gemini/custom provider
credentials. See `README.md` for the product framing and `DEMO.md` for a
walkthrough.

## graphify

This project has a knowledge graph at `graphify-out/` with god nodes,
community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when `graphify-out/graph.json` exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than `GRAPH_REPORT.md` or raw grep output.
- If `graphify-out/wiki/index.md` exists, use it for broad navigation instead of raw source browsing.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).

## Crate map

A Cargo workspace, not a monolith — 10 crates exist today, each one job:

| Crate | Responsibility |
|---|---|
| `gateway-config` | `AppConfig` — layered load (`.env` → process env → `#[serde(default)]`) plus startup validation (DB reachability, encryption-key format, bind-addr format). Depends on nothing else in the workspace — a foundation, below `gateway-common`. |
| `gateway-telemetry` | Tracing init (with a redacting writer — JWT/virtual-key/provider-key shapes never reach the log sink even if a handler accidentally logs one), Prometheus metrics registry + HTTP-metrics middleware, graceful-shutdown signal/deadline helpers. Also depends on nothing else in the workspace — the other foundation, alongside `gateway-config`. |
| `gateway-common` | `AppError`, canonical OpenAI-compatible DTOs (`canonical.rs`). Re-exports `AppConfig` from `gateway-config` and `init_tracing` from `gateway-telemetry` for backward compatibility. Depended on by everything. |
| `gateway-db` | `sqlx` repositories (one per table/aggregate) + a `ping()` reachability check for `/readyz` + `build_pool`/`PoolConfig` (max connections/acquire timeout/idle timeout, sourced from `AppConfig` by the caller — see Commands for `make db-reset`/`db-seed`). No business logic, just typed queries and `RepoError`. |
| `gateway-crypto` | AES-256-GCM envelope encryption for provider keys at rest, behind the `KeyEnvelopeCipher` trait. |
| `gateway-auth` | argon2 password hashing, JWT issuance/validation, refresh-token rotation. Depends on `gateway-db` for user/token persistence. |
| `gateway-providers` | `LlmProvider` trait + openai/anthropic/gemini adapters + `ProviderRegistry`. Pure HTTP + translation — no DB dependency at all. Also home to `sse.rs`: one incremental SSE frame decoder shared by every adapter, parameterized by a per-adapter `EventMapper` fn — so each vendor's streaming quirks live next to its own translation code, not in a downstream switch. |
| `gateway-core` | `RoutingEngine` (model → provider-credential resolution) + `FallbackEngine` (retries across candidate models) + `ProviderCaller` (decrypt + dispatch). Composes db + crypto + providers. Depends on `gateway-config` too, unused today — prep for the config-snapshot epic below. Also home to `token_meter` — a passthrough `Stream` adapter that counts output tokens as chunks flow to the client and reports once from `Drop` (so an aborted stream still records what it delivered). Also home to `pipeline::{Stage, Executor, build_chain}` — a stage/executor abstraction that `gateway-api`'s `pipeline_stages` middleware runs before the handler; today's fixed chain is `[AuthenticateStage, RoutingStage]`, ports of the now-deleted `virtual_key_auth`/`routing_layer` middleware (see the forbidden-patterns note below on the query-count tradeoff this introduced). Per-project configurable chains are Phase 3 work, once the config-snapshot epic's project/environment concept exists — not built yet. |
| `gateway-api` | Axum HTTP server: routes (incl. `/healthz`, `/readyz`, `/version`, `/metrics`), Tower middleware stack, `AppState`, OpenAPI docs, graceful shutdown on SIGTERM. Wires everything else together. |
| `gateway-testkit` | Shared test fixtures: a `MockProviderBuilder` wrapping wiremock (re-exports `MockServer` so callers need no direct wiremock dep), factories over the real `gateway-db` repos (org/team/user/virtual-key/provider-credential — the virtual-key factory replicates `virtual_keys.rs`'s exact generation scheme so its plaintext authenticates for real), and `golden_settings()` (insta redaction config for id/timestamp fields). Dev-dependency of `gateway-api` (used by `chat_completions_test.rs`) and `gateway-core` (added, not yet consumed — `fallback.rs`/`routing.rs` still hand-roll their own identical `setup_team`/`store_credential`/`test_cipher`; migrating them is follow-up work, not done in this epic). Depends on `gateway-db` + `gateway-crypto` + `wiremock` + `insta` — sits below `gateway-core` so it stays a valid dev-dependency for it. |

**Planned, not yet created** — do not assume these exist or create stub
crates speculatively; add each when the work that needs it actually lands:
- **Config snapshot / hot reload** (in-memory routing-config cache, `arc-swap`) lands inside `gateway-config` once the projects/environments schema exists — explicitly deferred out of the crate's initial extraction epic, not a new crate.
- **OTLP trace export** — out of scope for `gateway-telemetry`'s initial epic (metrics + health + redaction + shutdown only); lands as a later addition to that same crate, not a new one.
- `gateway-analytics` — today's usage analytics (`gateway-api/src/routes/analytics.rs`) is a thin query layer over `request_logs` living directly in `gateway-api`; it becomes its own crate only if/when it grows real logic.
- **Provider contract-test cassettes** (recorded HTTP fixtures for Azure/Bedrock/Ollama adapters) — a dedicated Phase 2 epic once those adapters exist; deliberately out of scope for `gateway-testkit`'s initial epic. `gateway-testkit` becoming a dev-dependency of `gateway-providers` for this is anticipated but not wired up yet.

## The dependency rule

One-way dependency graph, enforced by what's importable, not just convention:

```
gateway-config   gateway-telemetry
      ↑                 ↑
      └── gateway-common (re-exports AppConfig, init_tracing)
                ↑
                ├── gateway-db
                ├── gateway-crypto
                ├── gateway-auth        (→ gateway-db)
                └── gateway-providers   (pure HTTP/translation — NO gateway-db dependency, checked: its Cargo.toml has no gateway-db/sqlx line)
                          ↑
                    gateway-core        (→ gateway-db + gateway-crypto + gateway-providers + gateway-config)
                          ↑
                    gateway-api         (→ everything, wires into Axum)
```

- `gateway-testkit` isn't in the diagram above because it's dev-only: it sits alongside `gateway-db`/`gateway-crypto` (depends on both, nothing else in-workspace) and is a *dev*-dependency of `gateway-core` and `gateway-api`, never a runtime one.
- `gateway-providers` must never gain a `gateway-db` or `sqlx` dependency. It knows how to talk to a vendor's HTTP API and translate to/from the canonical DTOs — nothing about credentials storage, routing, or Postgres.
- `gateway-core` is the only crate allowed to depend on both `gateway-db`/`gateway-crypto` and `gateway-providers` — that composition *is* its job (`RoutingEngine` + `FallbackEngine` + `ProviderCaller`).
- `gateway-config` and `gateway-telemetry` both depend on nothing else in the workspace — they're the two crates *below* `gateway-common`. `gateway-config` talks to `sqlx` directly for its DB-reachability check (not `gateway-db`) and duplicates a 3-line base64/length check rather than depending on `gateway-crypto` — either of those would cycle back through `gateway-common`. Same reasoning kept `gateway-telemetry` free of any workspace dependency too.
- `gateway-common` depends on `gateway-config` + `gateway-telemetry` and nothing else in the workspace. This is the one exception to "depends on nothing" — it exists solely to re-export `AppConfig`/`init_tracing`, not because `gateway-common`'s own code uses either.
- **A PR that adds a dependency pointing the wrong way (e.g. `gateway-providers` → `gateway-db`, or `gateway-config`/`gateway-telemetry` → anything in the workspace) is rejected in review, full stop** — not a style nit.

## Authorization model

Two tiers today:

- **Org-scoped**: `org_members.role` (`"admin"` or `"member"`, a plain `TEXT` column — no DB enum) and `team_memberships.status` (`pending`/`approved`/`rejected` — no per-team role yet; a team lead tier is a deliberately deferred follow-up, not built). Every check funnels through exactly two functions in `gateway-api/src/authz.rs` — `require_org_admin` and `require_team_access` — called at the top of each handler rather than generic middleware, since path params vary per route.
- **Platform-wide (master admin)**: bypasses `require_org_admin`/`require_team_access` for every org/team, not just ones they belong to. `authz::resolve_master_admin` is the single place that decides it, from two independent grants (either sufficient): the account's email matching `AppConfig.master_admin_email` (default `master@cloud.in`, env `MASTER_ADMIN_EMAIL`), or the `users.is_master_admin` column. The email grant means a fresh database needs no manual SQL; the column grant survives an email change and can add a second master without a redeploy. Resolved in-process, not baked into the JWT: `CurrentUser` (see `middleware/jwt_auth.rs`) is populated from a fresh `UserRepo::find_by_id` on every request, so either grant takes effect on the user's *next* request with no re-login. **The email is compared against the DB row, never the JWT claim** — a token outlives a change to the row it was minted from, so the claim isn't authoritative for a privilege this broad. No grant/revoke endpoint exists yet (`UPDATE users SET is_master_admin = true WHERE email = '...'` for the column path); a proper grant flow and a "team lead" role are follow-up work.
- **The master holds no membership rows**, so anything reading `org_members`/`team_memberships` directly will show them nothing. `/api/me` therefore returns both lists verbatim (empty for a master) *plus* an `is_master_admin` flag, and the frontend branches on that flag — `isOrgAdmin()` in `AuthContext.tsx` returns true for a master, and `AdminPage` sources its org from the global `/api/orgs` list (which is already unrestricted) rather than from `primaryAdminOrgId`. Any *new* screen that infers access by scanning memberships will silently exclude the master; branch on the flag instead.
- **Both must be threaded manually.** `require_org_admin`/`require_team_access` take `is_master_admin: bool` as an explicit parameter (not looked up internally) so it's computed once per request in `jwt_auth`, not once per authz call. Any new route calling either function must pass `current_user.is_master_admin` — a route that hardcodes `false` or omits it silently breaks master access for that one endpoint.

## Provider resolution

**The gateway is not limited to the vendors it ships adapters for.** Two
separate questions, deliberately kept apart:

1. **Which credential serves this request** — `RoutingEngine::resolve`
   (`gateway-core/src/routing.rs`). Purely customer configuration: an
   explicit `routing_rules` match, else the team's sole active credential,
   else `NoRoute`. Every provider is treated identically.
2. **Where to send it** — `ProviderCaller::call` / `call_stream`
   (`gateway-core/src/pipeline/caller.rs`). The credential's own `base_url`
   wins; with none set, the provider must have a protocol adapter. Both call
   paths go through the same private `resolve()` so streaming and
   non-streaming can never disagree about which endpoint a credential means.

`ProviderRegistry` is a set of **protocol translators, not a whitelist**.
`ADAPTER_BACKED_PROVIDERS` (openai/anthropic/gemini) names the only providers
that speak their own wire format and carry a default endpoint from
`AppConfig`; anything else — Cohere, Mistral, Groq, a private deployment —
is first-class via `base_url` + the OpenAI-wire adapter.

Rules that keep it that way:

- **No build-time model→provider table on the request path.** The old
  `DEFAULT_PREFIX_MAP` (`gpt-`/`claude-`/`gemini-`) was deleted: it let three
  vendors route with zero configuration while every other provider silently
  failed the moment a team added a second credential. Reintroducing one
  re-creates that asymmetry. Making routing automatic again is a
  per-credential model-configuration feature, applying to all providers at
  once — not a constant.
- **A credential that can never be called is rejected at add time.**
  `routes/provider_keys.rs::create` 422s a provider with no adapter and no
  `base_url`. Previously it stored fine and failed only on first use, as a
  misleading "no route found for the requested model".
- `CoreError::ProviderNotConfigured` is the runtime counterpart, separate
  from `NoRoute` — routing succeeded, the endpoint is missing. Don't collapse
  the two; they send people to different places.
- The frontend must not branch on a provider list either. The Add Key form
  shows Base URL unconditionally and lets the backend decide.

## Virtual key storage

Each virtual key has **two independent representations**, and conflating them
is the mistake to avoid:

- `key_hash` — sha256, `UNIQUE`. **The only thing authentication reads**
  (`AuthenticateStage` → `VirtualKeyRepo::find_by_hash`). Unchanged by the
  envelope work.
- `encrypted_key`/`key_nonce`/`key_version` — AES-256-GCM via the same
  `KeyEnvelopeCipher` that protects provider keys. Exists solely so
  `GET /api/teams/{team_id}/virtual-keys/{id}/reveal` can hand the plaintext
  back to a team member. Nullable: keys issued before this existed have no
  ciphertext and 404 on reveal.

`VirtualKeyRepo::find_secret_for_team` is a **separate query** from
`find_by_hash`/`list_for_team` on purpose — the auth hot path and every
dashboard load have no business pulling key material into memory. Keep the
ciphertext columns off the shared `VirtualKey` struct.

Security posture this changes, stated plainly: a database dump plus the
master key now yields live gateway keys, where before it yielded only
hashes. The same dump already yielded every provider key it protects, so the
marginal exposure is small — but `reveal` is the one route that returns a
live credential, and `require_team_access` is the only thing gating it.

## Error handling convention

Every crate defines its own `thiserror`-derived error enum (`RepoError`,
`CryptoError`, `AuthError`, `ProviderError`, `CoreError`) — no manual
`impl std::error::Error`, no `anyhow` in library code. Each domain error
converts **upward** into `gateway_common::AppError` via `impl From<X> for
AppError`, chaining through `CoreError` when a lower error needs to cross
two hops (e.g. `RepoError` → `CoreError::Repo` → `AppError` via
`CoreError`'s own `From` impl, which just calls `.into()` on the wrapped
variant).

`AppError` (`crates/gateway-common/src/error.rs`) is the only error type
that knows about HTTP:

```rust
pub enum AppError {
    NotFound, Unauthorized, Forbidden, Conflict(String), Validation(String),
    BadRequest(String), TooManyAttempts, UpstreamError(String),
    UpstreamTimeout, Internal(String),
}
```

- `status_code()` maps each variant to a `StatusCode` (404/401/403/409/422/400/429/502/504/500).
- `impl IntoResponse for AppError` is what route handlers rely on — every handler returns `Result<_, AppError>` and Axum calls this automatically on `Err`.
- `Internal(String)` is special: the detail string is logged via `tracing::error!` but **never** put in the response body — the client always gets a generic `"internal server error"`. Never add a variant that puts raw internal detail (SQL errors, panics, stack info) in the client-facing message.
- `TooManyAttempts` exists on the enum today with no caller — reserved so the future rate-limiting work only needs a new `match` arm upstream, not a new HTTP-mapping shape.

**When adding a new failure case:** add it to the *originating* crate's own
error enum first (not directly to `AppError`), then extend that crate's
`From` impl. Only add directly to `AppError` for something genuinely
transport-level with no domain owner.

## Logging / tracing convention

- `gateway_telemetry::init_tracing()` (re-exported as `gateway_common::init_tracing()` for backward compatibility) sets up a single JSON-ish `tracing_subscriber::fmt()` subscriber honoring `RUST_LOG` (default `"info"`), writing through a `RedactingMakeWriter`. Called once, at the top of `main.rs`.
- **Never log secrets.** No provider API keys, virtual keys, JWTs, or the raw `Authorization` header — this remains the primary defense (documented in `gateway_telemetry::init_tracing`'s own doc comment), not just a habit. On top of that, `gateway_telemetry::redact` scans every rendered log line for three known secret shapes (JWT-looking strings, this app's `vk_live_...` virtual keys, `sk-...`-style provider keys) and replaces any match with a fixed-length `[REDACTED]` marker before it reaches the writer — a last-resort net for a handler that accidentally logs one, not a general-purpose secret scanner. An oddly-shaped custom/BYOK vendor key won't match any of the three shapes and would still leak if logged; the "never log one in the first place" rule is what actually has to hold.
- Request-id propagation is a fixed 4-layer Axum/Tower stack, outermost to innermost, assembled in `gateway_api::app()` (`crates/gateway-api/src/lib.rs`):
  1. `SetRequestIdLayer::x_request_id(MakeRequestUuid)` — generates the id.
  2. `TraceLayer::new_for_http()` — structured per-request tracing, keyed on that id.
  3. `PropagateRequestIdLayer::x_request_id()` — echoes it back on the response.
  4. `CorsLayer`.
- **This ordering must be preserved in every future change.** `TraceLayer` has to sit between the two request-id layers to see the id that was just set and have it survive onto the response. Adding a new outer layer (rate limiting, auth-adjacent middleware) goes *outside* `SetRequestIdLayer`, not between these three. `gateway_telemetry::track_http_metrics` (the HTTP-metrics middleware, added via `axum::middleware::from_fn` inside these four) relies on `axum::extract::MatchedPath` already being set by the router's own dispatch — it must stay applied via `Router::layer` on the merged router (as it is today), not moved to somewhere that runs before routing.
- **Metrics**: `gateway_telemetry::install_metrics_recorder()` installs the process-global Prometheus recorder once at startup (called from `main.rs`; tests share one install per test binary via a `OnceLock`, see `tests/support/mod.rs::test_metrics_handle` — a second real install attempt in the same process errors). `GET /metrics` renders it. Today's coverage is basic HTTP (`http_requests_total`, `http_request_duration_seconds`, both labeled by method/route-template/status) and process uptime — per-stage pipeline metrics land once the `Stage` trait exists.
- **Health endpoints**: `GET /healthz` is liveness only (always 200 once the process is up, no dependency checks). `GET /readyz` checks Postgres reachability via `gateway_db::ping` against the already-open pool and 503s until it succeeds — Redis is deliberately not checked (removed from this codebase entirely, see Phase status); a config-snapshot reachability check gets added here once that exists. `GET /version` returns the git SHA and build time embedded by `gateway-api/build.rs` at compile time. All four are unauthenticated by design.
- **Graceful shutdown**: on SIGTERM/SIGINT, `main.rs` stops accepting new connections and drains in-flight ones via `axum::serve(...).with_graceful_shutdown(...)`, bounded by `AppConfig.shutdown_grace_period_seconds` (default 30) via `gateway_telemetry::with_deadline_watchdog` — a watchdog force-exits if the deadline elapses before draining finishes on its own. See `DEMO.md`'s "Verifying graceful shutdown" section for the manual real-`SIGTERM` procedure.

## Forbidden patterns

- **No `unwrap()`/`expect()` outside `#[test]` code.** `gateway-api/src/main.rs`'s config-load/validate and DB-pool-build failures are no longer `.expect()` — both go through `exit_with_startup_error()` (prints a one-line, field-specific message to stderr, `std::process::exit(1)`), per the startup-validation work in `gateway-config`. Remaining accepted exceptions: the TCP `bind`/`axum::serve` calls in the same `main.rs` (OS-level runtime conditions, not config values), and `gateway-api/src/lib.rs::build_state`'s master-key `.expect()` — now effectively unreachable via `main.rs` since `AppConfig::validate()` checks that format first, but still present as a defensive fallback and used directly by tests (which always pass a valid key). `gateway-providers/src/adapters/gemini.rs:202` has a live `.unwrap()` on `tool_calls.as_ref()` inside `to_native_request` — it's guarded by a match arm that already checked `is_some_and(...)`, but it's still a real violation of this rule on the request-translation hot path and should become a proper `if let`/early-return before it ships again.
- **No blocking calls inside async fns** — no `std::thread::sleep`, no `std::fs::*`, no `reqwest::blocking`, no synchronous `Mutex` held across an `.await`. None found in the current codebase; keep it that way.
- **No synchronous database access on the hot path (`POST /v1/chat/completions`) once the config-snapshot work lands.** Today, `gateway-api/src/middleware/pipeline.rs`'s `pipeline_stages` middleware (which replaced the old `virtual_key_auth` middleware — `routing_layer` was already gone before that, deleted when routing moved per-attempt into `FallbackEngine`) runs `AuthenticateStage` + `RoutingStage` before the handler, hitting Postgres via `VirtualKeyRepo`/`RoutingRuleRepo`/`ProviderCredentialRepo` — plus a fire-and-forget `request_logs` insert in the `chat_completions` handler. That's the accepted current state. **Known, deliberate exception logged here so it isn't mistaken for drift:** `RoutingStage` resolves routing for every candidate model as a pre-check, and `FallbackEngine` (still called from the handler, unchanged) independently re-resolves the same candidates in its own retry loop — a real, intentional query-count regression, accepted for this epoch to prove the Stage/executor pattern without also folding the provider call into a Stage. It goes away once a later epoch turns `ProviderCaller`'s call into a `Stage` too, letting `RoutingStage`'s own resolution be the only one. `gateway-config` exists now (config extraction + startup validation only — see the crate map), but its snapshot/hot-reload cache is still explicitly deferred to a later epic (needs the projects/environments schema first). The moment that lands, routing/virtual-key resolution on this path must read from the in-memory snapshot, not issue a new query per request — don't add *more* per-request queries beyond what's noted here, and treat any other new one as a regression.

## Commands

No CI workflow and no `nextest`/`rustfmt.toml`/`.clippy.toml` config exist in
this repo yet — plain `cargo` subcommands, defaults only.

```bash
# Backend (from repo root; needs Postgres up — see DEMO.md)
cargo build --workspace
cargo test --workspace          # #[sqlx::test] provisions/tears down a throwaway DB per test
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Dev database (Makefile; see scripts/seed.sql for what gets seeded)
make db-reset   # drop, recreate, migrate, seed — start over cleanly
make db-seed    # load scripts/seed.sql into the current DB (not idempotent — INSERTs, no ON CONFLICT)

# Frontend (from frontend/ — see frontend/package.json)
npm run dev       # vite dev server
npm run build     # tsc -b && vite build
npm run lint      # oxlint
npm run preview   # preview a production build
```

`sqlx::query!`/`query_as!` type-check against the checked-in `.sqlx/` cache
via `SQLX_OFFLINE=true` (set in `.cargo/config.toml`) so `cargo build`/`test`
work without a live DB for compilation — but **running** the tests still
needs Postgres. After changing any SQL in a `query!`/`query_as!` macro, run:

```bash
cargo sqlx prepare --workspace -- --tests
```

against a live `DATABASE_URL`, and commit the resulting `.sqlx/*.json`.

## Phase status

No `BLUEPRINT.md` exists in this repo — the phase boundaries below are
inferred from `README.md`'s own "Phase 1 / Phase 2 extension points"
framing, extended to a 0–4 numbering. Update this table whenever a phase's
status changes; it's the first thing a new session should read.

| Phase | Scope | Status | Evidence |
|---|---|---|---|
| **0 — Foundation** | Workspace layout, `AppConfig`, `AppError`, tracing init, migration infra | **Done** | 9-crate workspace, `gateway-common`/`gateway-config`/`gateway-telemetry`, 4 migrations in `migrations/` |
| **1 — Core BYOK gateway** | Signup/login/JWT, encrypted provider-key storage, virtual-key issuance, OpenAI-compatible proxy to openai/anthropic/gemini | **Done** | `gateway-auth`, `gateway-crypto`, `gateway-providers` adapters, `POST /v1/chat/completions` |
| **2 — Orgs/Teams + routing flexibility** | Orgs/teams with join-request approval, per-key `routing_rules` overrides, generic BYOK vendor routing (any OpenAI-wire-compatible host via `base_url`), cross-provider fallback | **Done** | `org_repo.rs`, `team_membership_repo.rs`, `routing_rule_repo.rs`, `gateway-core/src/fallback.rs`, admin/member frontend, `DEMO.md`'s walkthrough |
| **3 — Operational hardening** | Rate limiting/login throttling, budgets, response caching, PII redaction/guardrails, streaming, config-snapshot for the hot path | **Partial** | **Done (Epic 2.1, SSE streaming + token metering):** `POST /v1/chat/completions` with `stream: true` now serves `text/event-stream`. `gateway-providers/src/sse.rs` decodes frames incrementally; openai/anthropic/gemini each implement `chat_completion_stream` and normalize to `ChatCompletionChunk`; output is re-framed as OpenAI-wire `chat.completion.chunk` so unmodified OpenAI SDKs consume an Anthropic or Gemini stream. `gateway_core::token_meter` counts output tokens in-flight and writes `request_logs` from `Drop` (status 499 on client abort). `RoutingStage` no longer rejects `stream: true`. **Done earlier:** `gateway-telemetry` (log redaction, Prometheus `/metrics`, `/healthz`+`/readyz`+`/version`, graceful shutdown). **Still not started:** rate limiting/login throttling, budgets, response caching, PII redaction/guardrails on request bodies (log redaction ≠ this), config-snapshot for the hot path. `Redis` remains fully removed; `AppError::TooManyAttempts` is still an unimplemented seam. OTLP trace export is deferred, lands in `gateway-telemetry` later. |
| **4 — Scale & monetization** | Billing, SSO, richer usage analytics, multi-region | **Partial** | Basic usage analytics exists today (`routes/analytics.rs` — request counts/tokens/errors per team+provider+model, admin-only via `require_org_admin`/`require_team_access`), but no billing, no SSO |

## Frontend

`frontend/` — React 19 + TypeScript + Vite, React Router 7, TanStack Query 5,
Tailwind 4. `oxlint` for linting, `tsc -b` for type-checking as part of
`npm run build`. No test runner configured — a gap, not a convention to
follow.

**Zero UI-component libraries, deliberately** — no Radix/shadcn/Headless UI.
Every primitive is hand-rolled in `src/components/`: `Button.tsx`,
`Input.tsx` (Input/Select/Textarea/Field), `Card.tsx`
(Card/ErrorText/Callout/Badge/Muted/StatCard), `Table.tsx`,
`HoverCard.tsx` (portal + `position: fixed`, opens on hover *and* focus,
closes on Escape — it has to be a portal because its callers sit inside
`overflow-x-auto` tables that would clip it), `icons.tsx` (hand-drawn SVG
paths). Adding a component library is the change to argue for in review, not
to slip in.

**Design system: strictly black + blue, dual-tone.** Defined once in
`src/index.css`:

- Neutrals are semantic `@theme` tokens — `ink` (app background),
  `surface` (cards/sidebar), `raised` (insets), `line` (borders), `fg`,
  `muted` — with the light values on `:root` and the dark values overridden
  under `.dark`. So it's `bg-surface`, not `bg-white dark:bg-zinc-900`: one
  class per element, and repaletting means editing one file. The dark values
  are verbatim Tailwind zinc (`#000` / zinc-950 / zinc-900 / zinc-800 /
  zinc-100 / zinc-400).
- **Accents stay literal `blue-*`** (`bg-blue-600`, `hover:bg-blue-500`,
  `text-blue-400`, `focus-visible:ring-blue-500/50`) — blue is identical in
  both themes, so tokenising it would only hide it.
- Red is the one off-palette family, reserved for destructive actions and
  error states. Don't introduce a third accent hue (the old cyan/emerald/amber
  mix was removed on purpose).
- Dark is the default theme (`class="dark"` on `<html>` in `index.html`,
  plus a 1-line pre-paint script that strips it for a stored `light` choice);
  `ThemeContext` no longer follows the OS preference.

Other conventions worth keeping:

- **Mutations go through `useMutation`**, not hand-rolled
  `useState`+`try/catch` — including the auth forms. Error text comes from
  `errorMessage(mutation.error, fallback)` in `lib/api.ts`, which unwraps
  `AppError`'s `{"error": "…"}` body via `ApiError` and falls back for
  network/parse failures. Client-side validation keeps its own `formError`
  state and is rendered `formError ?? errorMessage(...)`.
- **`lib/types.ts` is a verbatim mirror of the JSON API** — snake_case field
  names, and it tracks the Rust response DTOs in `gateway-api/src/routes/`
  (not the DB rows). `RequestLogEntry.fallback_reason` is the one field the
  backend doesn't send yet; it's optional so the UI shows a deterministic
  placeholder today and the real value the moment the column lands.
- `UsageTable` fetches its own data from a `{ org }` or `{ team }` scope and
  paginates client-side — the analytics endpoints return the whole rollup in
  one response, so there's nothing to page over server-side yet.
