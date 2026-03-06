# bedrock-rs — justfile
# Run with: just <recipe>   (requires `just` from nixpkgs)

# Default: show available recipes
default:
    @just --list

# ── Development ─────────────────────────────────────────────────────────────

# Start the full dev server (SSR + WASM hot-reload)
dev:
    cargo leptos watch

# Watch Tailwind CSS separately (if not using cargo-leptos style pipeline)
css:
    npm run css:watch

# ── Build ────────────────────────────────────────────────────────────────────

# Release build (server binary + optimised WASM)
build:
    cargo leptos build --release

# Build server only (no WASM)
build-server:
    cargo build -p app --features ssr --release

# ── Quality ──────────────────────────────────────────────────────────────────

# Format all Rust code
fmt:
    cargo fmt --all

# Clippy across the workspace (server features)
lint:
    cargo clippy --workspace --features ssr -- -D warnings

# Run workspace tests
test:
    cargo test --workspace --features ssr

# ── Setup ────────────────────────────────────────────────────────────────────

# Install Node dependencies (Tailwind etc.)
npm-install:
    npm install

# Add the WASM target (needed outside nix dev shell)
wasm-target:
    rustup target add wasm32-unknown-unknown

# ── Local infrastructure ─────────────────────────────────────────────────────

# Start local DynamoDB + app
up:
    docker compose up -d

# Stop everything
down:
    docker compose down

# Create DynamoDB tables in local DynamoDB
create-tables:
    docker compose run --rm dynamodb-setup

# Tail app logs
logs:
    docker compose logs -f app

# ── Maintenance ──────────────────────────────────────────────────────────────

# Remove build artefacts
clean:
    cargo clean
    rm -rf target/site node_modules
