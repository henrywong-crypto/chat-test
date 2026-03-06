# ── Stage 1: Build ──────────────────────────────────────────────────────────────
# cargo-leptos builds both the SSR server binary and the WASM/asset bundle in
# a single command, so this stage needs the full Rust + WASM + Node toolchain.

FROM rust:1.94-bookworm AS builder

# ── System deps ────────────────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev curl binaryen \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# ── Rust WASM target ────────────────────────────────────────────────────────────
RUN rustup target add wasm32-unknown-unknown

# ── cargo-leptos + wasm-bindgen-cli ────────────────────────────────────────────
# Pin versions to match the workspace deps for reproducible builds.
RUN cargo install cargo-leptos --locked
RUN cargo install wasm-bindgen-cli --version 0.2.114 --locked

# ── Dependency pre-fetch (cache layer) ─────────────────────────────────────────
WORKDIR /app

# Copy only the Cargo manifests first. Populate each crate with a stub src so
# `cargo fetch` can resolve the dependency graph without the real source code.
COPY Cargo.toml Cargo.lock ./
COPY crates/app/Cargo.toml     crates/app/Cargo.toml
COPY crates/shared/Cargo.toml  crates/shared/Cargo.toml
COPY crates/auth/Cargo.toml    crates/auth/Cargo.toml
COPY crates/bedrock/Cargo.toml crates/bedrock/Cargo.toml
COPY crates/db/Cargo.toml      crates/db/Cargo.toml

RUN for crate in app shared auth bedrock db; do \
        mkdir -p crates/$crate/src; \
        echo "" > crates/$crate/src/lib.rs; \
    done \
    && mkdir -p crates/app/src \
    && echo "fn main() {}" > crates/app/src/main.rs

RUN cargo fetch

# ── Full source ─────────────────────────────────────────────────────────────────
COPY . .

# ── cargo leptos release build ──────────────────────────────────────────────────
# Produces:
#   target/release/app   ← SSR binary
#   target/site/         ← WASM, CSS, and static assets
RUN cargo leptos build --release

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/app ./server
COPY --from=builder /app/target/site                  ./site

# cargo-leptos / Leptos look for compiled frontend assets here.
ENV LEPTOS_SITE_ROOT=/app/site

# Bind on all interfaces so the container is reachable from the host / ALB.
ENV SITE_ADDR=0.0.0.0:3000

EXPOSE 3000

# Health-check via the unauthenticated models endpoint (fast, no DB required).
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -sf http://localhost:3000/api/models || exit 1

CMD ["./server"]
