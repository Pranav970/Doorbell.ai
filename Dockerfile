# syntax=docker/dockerfile:1

# Toolchain version tracks rust-toolchain.toml — keep the two in sync.
ARG RUST_VERSION=1.96.1

FROM rust:${RUST_VERSION}-slim-bookworm AS chef
COPY --from=lukemathwalker/cargo-chef:latest-rust-1-slim-bookworm /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Dependency-only layer: cached as long as recipe.json (Cargo.toml/Cargo.lock) doesn't change.
RUN cargo chef cook --release --recipe-path recipe.json --bin gateway-api --bin healthcheck
COPY . .
# .sqlx/ is checked in and .cargo/config.toml sets SQLX_OFFLINE=true, so this
# doesn't need a live DATABASE_URL.
RUN cargo build --release --bin gateway-api --bin healthcheck

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /app/target/release/gateway-api /usr/local/bin/gateway-api
COPY --from=builder /app/target/release/healthcheck /usr/local/bin/healthcheck

# distroless/cc-debian12:nonroot already runs as uid 65532 (nonroot).
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/healthcheck"]

ENTRYPOINT ["/usr/local/bin/gateway-api"]
