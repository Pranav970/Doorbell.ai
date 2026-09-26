# Graph Report - prismaxis  (2026-08-26)

## Corpus Check
- 214 files · ~73,769 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1547 nodes · 3563 edges · 68 communities (66 shown, 2 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 153 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `61360afa`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Uuid
- anthropic.rs
- User
- fallback.rs
- json_request_with_auth
- AuthError
- gemini.rs
- api.ts
- virtual_keys.rs
- chat_completions
- create
- create
- CurrentUser
- AppError
- orgs.rs
- fixtures.rs
- create
- ProviderError
- compilerOptions
- auth.rs
- MockProviderBuilder
- compilerOptions
- gateway-config/src/lib.rs
- dependencies
- AdminPage.tsx
- App.tsx
- me.rs
- check
- TeamRepo
- ops.rs
- sse.rs
- golden_settings
- repo_test.rs
- MemberPage.tsx
- jwt_auth
- plugins
- Landing.tsx
- gateway-db/src/lib.rs
- gateway-api
- pull_request_template.md
- LLM Gateway (Phase 1)
- Running a demo
- vite-env.d.ts
- tsconfig.json
- redact.rs
- Prismaxis (LLM Gateway) — Handover Doc
- React + TypeScript + Vite
- CLAUDE.md — Prismaxis LLM Gateway
- executor.rs
- token_meter.rs
- RepoError
- ProviderCredential
- VirtualKeyRepo
- RefreshTokenRepo
- OrgMemberRepo
- RoutingRuleRepo
- pipeline_stages
- ChatCompletionRequest
- OpenAiProvider
- Shell.tsx
- AppState
- openai.rs

## God Nodes (most connected - your core abstractions)
1. `AppState` - 91 edges
2. `AppError` - 61 edges
3. `RepoError` - 59 edges
4. `CurrentUser` - 35 edges
5. `json_request_with_auth()` - 35 edges
6. `ChatCompletionRequest` - 28 edges
7. `test_app()` - 27 edges
8. `MockProviderBuilder` - 22 edges
9. `require_team_access()` - 21 edges
10. `signup_login_and_team()` - 20 edges

## Surprising Connections (you probably didn't know these)
- `authorize_decision()` --calls--> `require_org_admin()`  [INFERRED]
  crates/gateway-api/src/routes/team_membership.rs → crates/gateway-api/src/authz.rs
- `list_pending_for_org()` --calls--> `require_org_admin()`  [INFERRED]
  crates/gateway-api/src/routes/team_membership.rs → crates/gateway-api/src/authz.rs
- `create()` --calls--> `require_org_admin()`  [INFERRED]
  crates/gateway-api/src/routes/teams.rs → crates/gateway-api/src/authz.rs
- `create()` --calls--> `require_team_access()`  [INFERRED]
  crates/gateway-api/src/routes/provider_keys.rs → crates/gateway-api/src/authz.rs
- `deactivate()` --calls--> `require_team_access()`  [INFERRED]
  crates/gateway-api/src/routes/provider_keys.rs → crates/gateway-api/src/authz.rs

## Import Cycles
- None detected.

## Communities (68 total, 2 thin omitted)

### Community 0 - "Uuid"
Cohesion: 0.20
Nodes (14): ApprovedMemberView, PendingRequestView, DateTime, Option, PgPool, Result, Self, String (+6 more)

### Community 1 - "anthropic.rs"
Cohesion: 0.15
Nodes (31): assistant_tool_call_becomes_tool_use_block_and_tool_result_becomes_user_message(), base_request(), extracts_system_messages_into_top_level_field(), from_native_response(), input_and_output_tokens_arrive_on_separate_frames(), joins_multiple_system_messages_and_preserves_explicit_max_tokens(), map_event(), msg() (+23 more)

### Community 2 - "User"
Cohesion: 0.12
Nodes (28): resolve_master_admin(), NewRequestLog, Organization, OrgMember, RefreshToken, RoutingRule, DateTime, Option (+20 more)

### Community 3 - "fallback.rs"
Cohesion: 0.06
Nodes (60): ResolvedRoute, String, Uuid, CoreError, gateway_common::AppError, From, Self, String (+52 more)

### Community 4 - "json_request_with_auth"
Cohesion: 0.06
Nodes (76): decode_jwt_payload(), PgPool, Value, signup_login_refresh_logout_happy_path(), custom_provider_with_base_url_bypasses_the_registry(), malformed_json_body_with_an_invalid_key_still_returns_401_not_400(), missing_authorization_header_is_rejected_before_reaching_the_provider(), oversized_body_is_rejected_with_413_not_buffered_unbounded() (+68 more)

### Community 5 - "AuthError"
Cohesion: 0.10
Nodes (35): Self, AuthError, gateway_common::AppError, From, Self, String, AccessClaims, decode_access_token() (+27 more)

### Community 6 - "gemini.rs"
Cohesion: 0.11
Nodes (38): assistant_tool_call_becomes_function_call_part_and_tool_result_becomes_function_content(), base_request(), concatenates_text_parts_and_reads_usage_metadata(), from_native_response(), function_call_response_part_becomes_canonical_tool_call(), GeminiProvider, keeps_the_vendors_own_finish_reason_casing(), map_event() (+30 more)

### Community 7 - "api.ts"
Cohesion: 0.11
Nodes (31): AuthContextValue, apiFetch(), chatCompletion(), clearTokens(), getAccessToken(), getRefreshToken(), parseBody(), raiseIfError() (+23 more)

### Community 8 - "virtual_keys.rs"
Cohesion: 0.18
Nodes (28): create(), generate_virtual_key(), IssueVirtualKeyRequest, list(), purge(), reveal(), revoke(), router() (+20 more)

### Community 9 - "chat_completions"
Cohesion: 0.10
Nodes (30): KeyUsageResponse, RequestLogResponse, DateTime, From, Option, Self, String, Utc (+22 more)

### Community 10 - "create"
Cohesion: 0.18
Nodes (24): AddRoutingRuleRequest, create(), delete(), list(), require_virtual_key_in_team(), router(), RoutingRuleListResponse, RoutingRuleResponse (+16 more)

### Community 11 - "create"
Cohesion: 0.16
Nodes (24): create(), CreateTeamRequest, detail(), list_for_org(), router(), DateTime, Extension, From (+16 more)

### Community 12 - "CurrentUser"
Cohesion: 0.20
Nodes (23): CurrentUser, approve(), authorize_decision(), create(), JoinRequestResponse, list_pending_for_org(), PendingRequestListResponse, PendingRequestResponse (+15 more)

### Community 13 - "AppError"
Cohesion: 0.17
Nodes (21): require_org_admin(), require_team_access(), Result, Uuid, KeyUsageListResponse, org_analytics(), RequestLogListResponse, Extension (+13 more)

### Community 14 - "orgs.rs"
Cohesion: 0.13
Nodes (22): create(), CreateOrgRequest, list(), OrgListResponse, OrgResponse, OrgWithTeamsResponse, router(), DateTime (+14 more)

### Community 15 - "fixtures.rs"
Cohesion: 0.10
Nodes (36): ciphertext_never_contains_the_plaintext_bytes(), CryptoError, EncryptedBlob, EnvKeyCipher, gateway_common::AppError, KeyEnvelopeCipher, nonces_are_unique_across_many_encryptions(), round_trip_encrypt_decrypt() (+28 more)

### Community 16 - "create"
Cohesion: 0.21
Nodes (23): AddProviderKeyRequest, create(), deactivate(), is_valid_base_url(), last_four_chars(), list(), ProviderKeyListResponse, ProviderKeyResponse (+15 more)

### Community 17 - "ProviderError"
Cohesion: 0.11
Nodes (17): AnthropicProvider, ChatStream, Client, Result, SecretString, gateway_common::AppError, is_context_length_error(), map_transport_error() (+9 more)

### Community 18 - "compilerOptions"
Cohesion: 0.08
Nodes (23): compilerOptions, allowArbitraryExtensions, allowImportingTsExtensions, erasableSyntaxOnly, jsx, lib, module, moduleDetection (+15 more)

### Community 19 - "auth.rs"
Cohesion: 0.24
Nodes (18): login(), LoginRequest, logout(), LogoutRequest, refresh(), RefreshRequest, From, Json (+10 more)

### Community 20 - "MockProviderBuilder"
Cohesion: 0.07
Nodes (49): BuildError, a_client_that_disconnects_midstream_still_records_partial_tokens(), a_permissive_vendor_receives_stream_options_by_default(), a_strict_vendor_gets_no_stream_options_and_still_succeeds(), anthropic_stream_is_normalized_and_counted_from_message_delta(), assembled_text(), chunks_reach_the_client_before_the_upstream_stream_finishes(), collect_sse() (+41 more)

### Community 21 - "compilerOptions"
Cohesion: 0.10
Nodes (19): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, noEmit, noFallthroughCasesInSwitch (+11 more)

### Community 22 - "gateway-config/src/lib.rs"
Cohesion: 0.07
Nodes (38): CorsLayer, app(), build_cors_layer(), build_state(), PgPool, PrometheusHandle, Router, ApiDoc (+30 more)

### Community 23 - "dependencies"
Cohesion: 0.04
Nodes (45): autoprefixer, framer-motion, dependencies, autoprefixer, framer-motion, postcss, react, react-dom (+37 more)

### Community 24 - "AdminPage.tsx"
Cohesion: 0.15
Nodes (22): Button(), ButtonSize, ButtonVariant, SIZES, VARIANTS, Badge(), Card(), ErrorText() (+14 more)

### Community 25 - "App.tsx"
Cohesion: 0.12
Nodes (24): App(), dashboardFor(), IndexRedirect(), queryClient, RequireAnon(), RequireAuth(), Select(), Shell() (+16 more)

### Community 26 - "me.rs"
Cohesion: 0.23
Nodes (13): me(), MeResponse, OrgMembershipResponse, router(), Extension, Json, Option, Result (+5 more)

### Community 27 - "check"
Cohesion: 0.67
Nodes (3): check(), main(), Result

### Community 28 - "TeamRepo"
Cohesion: 0.28
Nodes (8): Option, PgPool, Result, Self, Team, Uuid, Vec, TeamRepo

### Community 29 - "ops.rs"
Cohesion: 0.19
Nodes (14): HealthResponse, healthz(), metrics(), ReadyResponse, readyz(), router(), DateTime, IntoResponse (+6 more)

### Community 30 - "sse.rs"
Cohesion: 0.18
Nodes (22): ChatCompletionChunk, Self, collect(), decode(), decode_frame(), Decoder, drain_frames(), find_frame_end() (+14 more)

### Community 31 - "golden_settings"
Cohesion: 0.67
Nodes (3): golden_settings(), redacts_id_and_timestamp_fields(), Settings

### Community 32 - "repo_test.rs"
Cohesion: 0.42
Nodes (10): provider_credential_create_list_deactivate(), refresh_token_rotation_and_family_revocation(), request_log_insert(), request_log_list_recent_for_team_scopes_and_orders(), routing_rules_ordered_by_priority(), PgPool, Uuid, setup_team() (+2 more)

### Community 33 - "MemberPage.tsx"
Cohesion: 0.10
Nodes (26): Callout(), HoverCard(), Table(), Td(), Tr(), num(), UsageDetail(), UsageScope (+18 more)

### Community 34 - "jwt_auth"
Cohesion: 0.24
Nodes (9): extract_bearer(), jwt_auth(), Next, Request, Response, Result, State, String (+1 more)

### Community 35 - "plugins"
Cohesion: 0.22
Nodes (8): plugins, rules, react/only-export-components, react/rules-of-hooks, $schema, oxc, typescript, warn

### Community 36 - "Landing.tsx"
Cohesion: 0.14
Nodes (17): GradientDivider(), WaveDivider(), Features, GetStartedModal(), Hero(), HowItWorks(), STEPS, LandingHeader() (+9 more)

### Community 37 - "gateway-db/src/lib.rs"
Cohesion: 0.31
Nodes (7): build_pool(), ping(), PoolConfig, Duration, Error, PgPool, Result

### Community 38 - "gateway-api"
Cohesion: 0.71
Nodes (7): gateway-api, gateway-auth, gateway-common, gateway-core, gateway-crypto, gateway-db, gateway-providers

### Community 39 - "pull_request_template.md"
Cohesion: 0.50
Nodes (3): Checklist, Crates touched, What / why

### Community 40 - "LLM Gateway (Phase 1)"
Cohesion: 0.17
Nodes (11): API walkthrough, Architecture, Data model, Environment variables, LLM Gateway (Phase 1), Local setup, Phase 2 extension points, Provider extensibility (+3 more)

### Community 41 - "Running a demo"
Cohesion: 0.18
Nodes (10): 0. Prerequisites, 1. Configure the backend environment, 2. Start Postgres (+ the containerized backend), 3. Run database migrations, 4. Start the backend, 5. Configure and start the frontend, 6. Walk through the demo, Running a demo (+2 more)

### Community 44 - "redact.rs"
Cohesion: 0.06
Nodes (37): exit_with_startup_error(), main(), is_token_char(), looks_like_jwt(), looks_like_provider_key(), looks_like_secret(), looks_like_virtual_key(), push_token_or_redacted() (+29 more)

### Community 50 - "Prismaxis (LLM Gateway) — Handover Doc"
Cohesion: 0.20
Nodes (9): Codebase structure, Data model, Local setup (see `DEMO.md` for the full walkthrough), Prismaxis (LLM Gateway) — Handover Doc, Problems — resolved this session, Problems — still open (intentional Phase 2 scope, not bugs), Request flow — `POST /v1/chat/completions`, Tech stack (+1 more)

### Community 51 - "React + TypeScript + Vite"
Cohesion: 0.50
Nodes (3): Expanding the Oxlint configuration, React Compiler, React + TypeScript + Vite

### Community 52 - "CLAUDE.md — Prismaxis LLM Gateway"
Cohesion: 0.14
Nodes (13): Authorization model, CLAUDE.md — Prismaxis LLM Gateway, Commands, Crate map, Error handling convention, Forbidden patterns, Frontend, graphify (+5 more)

### Community 54 - "executor.rs"
Cohesion: 0.06
Nodes (58): ResolvedVirtualKey, continuing(), decision_log_records_one_entry_per_stage(), describe(), empty_ctx(), empty_response_ctx(), executes_stages_in_chain_order_on_request(), Executor (+50 more)

### Community 55 - "token_meter.rs"
Cohesion: 0.16
Nodes (16): a_client_that_disconnects_midway_still_reports_partial_counts(), estimates_when_the_provider_reports_nothing(), merges_counts_reported_on_separate_frames(), Meter, Meter<F>, prefers_the_providers_own_counts_over_the_estimate(), ChatStream, F (+8 more)

### Community 56 - "RepoError"
Cohesion: 0.31
Nodes (7): gateway_common::AppError, map_unique_violation(), RepoError, Error, From, Self, String

### Community 57 - "ProviderCredential"
Cohesion: 0.21
Nodes (10): Self, ProviderCredential, Vec, ProviderCredentialRepo, Option, PgPool, Result, Self (+2 more)

### Community 58 - "VirtualKeyRepo"
Cohesion: 0.26
Nodes (9): Option, PgPool, Result, Self, Uuid, Vec, VirtualKey, VirtualKeyRepo (+1 more)

### Community 59 - "RefreshTokenRepo"
Cohesion: 0.24
Nodes (8): RefreshTokenRepo, DateTime, Option, PgPool, Result, Self, Utc, Uuid

### Community 60 - "OrgMemberRepo"
Cohesion: 0.25
Nodes (9): OrgMemberRepo, OrgMembershipView, Option, PgPool, Result, Self, String, Uuid (+1 more)

### Community 61 - "RoutingRuleRepo"
Cohesion: 0.30
Nodes (7): RoutingRuleRepo, PgPool, Result, RoutingRule, Self, Uuid, Vec

### Community 62 - "pipeline_stages"
Cohesion: 0.31
Nodes (10): extract_bearer(), pipeline_stages(), request_id_of(), Next, Option, Request, Response, Result (+2 more)

### Community 63 - "ChatCompletionRequest"
Cohesion: 0.29
Nodes (19): ChatCompletionChoice, ChatCompletionChunkWire, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChunkChoice, ChunkDelta, default_tool_type() (+11 more)

### Community 64 - "OpenAiProvider"
Cohesion: 0.23
Nodes (7): OpenAiProvider, ChatStream, Client, Result, SecretString, Self, String

### Community 65 - "Shell.tsx"
Cohesion: 0.22
Nodes (11): Icon(), IconName, PATHS, NavItem, ThemeToggle(), initialTheme(), Theme, ThemeContext (+3 more)

### Community 66 - "AppState"
Cohesion: 0.24
Nodes (10): router(), router(), build_router(), Router, router(), router(), AppState, Arc (+2 more)

### Community 67 - "openai.rs"
Cohesion: 0.28
Nodes (4): map_event(), maps_text_frames_and_ignores_the_role_only_opener(), reads_the_trailing_usage_only_frame(), Option

## Knowledge Gaps
- **142 isolated node(s):** `ApiDoc`, `ReadyResponse`, `$schema`, `typescript`, `oxc` (+137 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState` connect `AppState` to `Uuid`, `User`, `fallback.rs`, `AuthError`, `virtual_keys.rs`, `chat_completions`, `create`, `create`, `CurrentUser`, `AppError`, `orgs.rs`, `fixtures.rs`, `create`, `auth.rs`, `gateway-config/src/lib.rs`, `me.rs`, `TeamRepo`, `ops.rs`, `jwt_auth`, `executor.rs`, `ProviderCredential`, `VirtualKeyRepo`, `OrgMemberRepo`, `RoutingRuleRepo`, `pipeline_stages`?**
  _High betweenness centrality (0.281) - this node is a cross-community bridge._
- **Why does `AppConfig` connect `gateway-config/src/lib.rs` to `AppState`, `json_request_with_auth`?**
  _High betweenness centrality (0.133) - this node is a cross-community bridge._
- **Why does `test_config()` connect `json_request_with_auth` to `MockProviderBuilder`, `gateway-config/src/lib.rs`?**
  _High betweenness centrality (0.075) - this node is a cross-community bridge._
- **Are the 28 inferred relationships involving `json_request_with_auth()` (e.g. with `signup_login_refresh_logout_happy_path()` and `custom_provider_with_base_url_bypasses_the_registry()`) actually correct?**
  _`json_request_with_auth()` has 28 INFERRED edges - model-reasoned connections that need verification._
- **What connects `ApiDoc`, `ReadyResponse`, `$schema` to the rest of the system?**
  _142 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `anthropic.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.14616755793226383 - nodes in this community are weakly interconnected._
- **Should `User` be split into smaller, more focused modules?**
  _Cohesion score 0.1241565452091768 - nodes in this community are weakly interconnected._