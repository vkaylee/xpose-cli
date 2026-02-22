#!/bin/bash
set -e

# --- Configuration ---
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

# Detect CI Environment
if [ "$GITHUB_ACTIONS" = "true" ]; then
    echo "👷 Running in CI mode..."
    # In CI, we use a temporary directory for cargo cache if the host paths don't exist
    export CARGO_HOME_REGISTRY=${CARGO_HOME_REGISTRY:-/tmp/cargo_registry}
    export CARGO_HOME_GIT=${CARGO_HOME_GIT:-/tmp/cargo_git}
    export SCCACHE_CACHE_DIR=${SCCACHE_CACHE_DIR:-/tmp/sccache_cache}
    mkdir -p "$CARGO_HOME_REGISTRY" "$CARGO_HOME_GIT" "$SCCACHE_CACHE_DIR"
    COMPOSE_FLAGS="--progress=plain"
else
    echo "🏠 Running in local mode..."
    COMPOSE_FLAGS=""
fi

echo "🐳 Building Docker image via Compose..."
docker compose build $COMPOSE_FLAGS dev

echo "🦀 Running Lint & Format in Docker..."
docker compose run --rm dev bash -c "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"

echo "🧪 Running Tests in Docker..."
docker compose run --rm dev cargo test --workspace

echo "✨ All local checks passed successfully in the Docker environment!"
