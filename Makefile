DATABASE_URL ?= postgres://gateway:gateway@localhost:5432/gateway

.PHONY: db-reset db-seed

# Drops, recreates, and re-migrates the dev database, then seeds it.
# Requires sqlx-cli (see DEMO.md) and Postgres up (docker compose up -d).
db-reset:
	DATABASE_URL=$(DATABASE_URL) sqlx database reset -y
	$(MAKE) db-seed

# Loads scripts/seed.sql into the dev database via the Postgres container
# itself, so this doesn't need a `psql` install on the host.
db-seed:
	docker compose exec -T postgres psql -U gateway -d gateway -f - < scripts/seed.sql
