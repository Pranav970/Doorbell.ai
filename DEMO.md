# Running a demo

Chronological steps to get the whole stack (Postgres, backend, frontend) up
from a clean checkout. Two terminals: one for the backend, one for the
frontend.

## 0. Prerequisites

- Rust (stable) + Cargo
- Docker (for Postgres)
- `sqlx-cli` — install once: `cargo install sqlx-cli --no-default-features --features rustls,postgres`
- Node.js 20+ (for the frontend)

## 1. Configure the backend environment

```bash
cp .env.example .env
```

Generate real secrets (the placeholders in `.env.example` will not work —
`docker compose up` now also builds and starts `gateway-api`, and it
validates these at startup and exits if they're placeholders):

```bash
openssl rand -base64 48   # -> JWT_SECRET
openssl rand -base64 32   # -> ENCRYPTION_MASTER_KEY (must decode to exactly 32 bytes)
```

Paste both into `.env` — this must happen *before* step 2, since `docker
compose` reads this file for variable substitution when it starts
`gateway-api`.

## 2. Start Postgres (+ the containerized backend)

```bash
docker compose up -d
```

This builds the `gateway-api` image (see `Dockerfile`) and starts it
alongside Postgres; `docker compose ps` reports `gateway-api` healthy once
`/readyz` returns 200. If you're actively changing backend code, skip the
container rebuild loop and run just Postgres instead:

```bash
docker compose up -d postgres
```

...then use `cargo run -p gateway-api` in step 4 below instead of the
containerized instance (stop the `gateway-api` container first if you
started the full stack, to avoid a port 8080 clash).

## 3. Run database migrations

```bash
export DATABASE_URL=postgres://gateway:gateway@localhost:5432/gateway
sqlx migrate run
```

Optionally, seed one dev org/team/user/virtual-key so you can skip the
signup/org/team/join-request flow below and go straight to step 7:

```bash
make db-seed
```

Dev login is `dev@example.com` / `devpassword123`; the seeded virtual key is
`vk_live_devseed1234567890abcdef` (see `scripts/seed.sql` for what it
creates — no provider credential is seeded, so routing a real model through
it 400s with "no route found" until you add one via the API/UI). To start
completely over — drop, recreate, re-migrate, and re-seed in one shot:

```bash
make db-reset
```

## 4. Start the backend

Already running via `docker compose up -d` in step 2. If you instead
started only `postgres` there (for a faster local dev loop):

```bash
cargo run -p gateway-api
```

Leave this running. Either way it listens on `http://localhost:8080`. API
docs at `http://localhost:8080/swagger-ui`.

## 5. Configure and start the frontend

In a second terminal:

```bash
cd frontend
cp .env.example .env        # VITE_API_BASE_URL=http://localhost:8080 by default
npm install
npm run dev
```

Open the URL Vite prints (typically `http://localhost:5173`).

## 6. Walk through the demo

1. **Sign up as an admin** — on the signup page, choose "Start an
   organization," fill in your name/email/password, then name your org.
   You land on the admin console.
2. **Create a team** — Admin console → Teams tab → create a team.
3. **Sign up as a member** (use a private/incognito window or log out first)
   — choose "Join a team," pick the org/team you just created, submit the
   join request.
4. **Approve the request** — back in the admin window, Requests tab →
   Approve.
5. **Add a BYOK provider key** — as the member, Team dashboard → Keys tab →
   add a provider key (OpenAI/Anthropic/Gemini — any string works against a
   local/mock setup, a real key is only needed to actually call a model).
6. **Issue a virtual key** — Virtual Keys tab → issue one, copy it (shown
   once).
7. **Call the gateway** — Playground tab → pick the issued key, set a model,
   send a message. This calls `POST /v1/chat/completions` through the
   routing proxy using the BYOK credential you added in step 5.
8. **Admin analytics** — back in the admin console, Keys & Usage tab shows
   request counts/tokens per team/provider/model once calls have been made.

## Verifying graceful shutdown (manual)

Automated coverage for the drain mechanism itself lives in
`gateway-telemetry`'s `shutdown::tests::graceful_shutdown_drains_in_flight_requests_before_exiting`
(a fake trigger, not a real signal — see that test's doc comment for why).
This is the real, end-to-end version against the actual binary and a real
`SIGTERM`, useful whenever you touch shutdown/main.rs and want to confirm
nothing regressed:

```bash
# Terminal 1
cargo run -p gateway-api

# Terminal 2 — fire a request slow enough to still be in flight when the
# signal lands (signup's argon2 hashing takes tens of ms), then terminate
# the server almost immediately after
curl -s -w '\nsignup_status=%{http_code}\n' -X POST http://localhost:8080/auth/signup \
  -H 'content-type: application/json' \
  -d '{"email":"drain-check@example.com","password":"a-strong-password"}' &
sleep 0.05 && kill -TERM $(pgrep -f 'target/debug/gateway-api')
```

Expected: the `curl` prints `signup_status=201` (not a connection error),
and Terminal 1's log shows, in order, `shutdown triggered; draining
in-flight requests` then `graceful shutdown complete`, and the process
exits with code 0 (`echo $?` in Terminal 1 after it returns). For a
heavier check, run a short concurrent load (`for i in $(seq 1 20); do curl
... & done`) immediately before the `kill -TERM` and confirm every one of
the 20 responses came back successfully rather than checking just one.

## Troubleshooting

- **Backend won't start** — check `docker compose ps`; Postgres must be
  healthy first.
- **`sqlx migrate run` fails** — make sure `DATABASE_URL` is exported in the
  same shell you run it from.
- **Frontend can't reach the API** — confirm `frontend/.env`'s
  `VITE_API_BASE_URL` matches where `gateway-api` is actually listening
  (`BIND_ADDR` in the root `.env`, default `0.0.0.0:8080`).
