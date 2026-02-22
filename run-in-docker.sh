#!/bin/bash
set -e

# --- Usage ---
show_usage() {
    echo "Usage: ./run-in-docker.sh [command]"
    echo ""
    echo "Commands:"
    echo "  lint    Run cargo fmt and clippy checks"
    echo "  test    Run workspace unit tests"
    echo "  all     Run both lint and test (default)"
    echo "  help    Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  GITHUB_ACTIONS=true    Enable CI-optimized output and caching"
    echo "  NO_BUILD=true          Skip the build phase (useful if image is already built)"
}

# --- Configuration ---
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

COMMAND=${1:-all}

if [[ "$COMMAND" == "help" || "$COMMAND" == "--help" || "$COMMAND" == "-h" ]]; then
    show_usage
    exit 0
fi

# Detect CI Environment
if [ "$GITHUB_ACTIONS" = "true" ]; then
    echo "👷 Running in CI mode..."
    export CARGO_HOME_REGISTRY=${CARGO_HOME_REGISTRY:-/tmp/cargo_registry}
    export CARGO_HOME_GIT=${CARGO_HOME_GIT:-/tmp/cargo_git}
    export SCCACHE_CACHE_DIR=${SCCACHE_CACHE_DIR:-/tmp/sccache_cache}
    mkdir -p "$CARGO_HOME_REGISTRY" "$CARGO_HOME_GIT" "$SCCACHE_CACHE_DIR"
    COMPOSE_FLAGS="--progress=plain"
else
    echo "🏠 Running in local mode..."
    COMPOSE_FLAGS=""
fi

if [ "$NO_BUILD" != "true" ] && [ "$SKIP_BUILD" != "true" ]; then
    echo "🐳 Building Docker image via Compose..."
    docker compose build $COMPOSE_FLAGS dev
fi

case "$COMMAND" in
    lint)
        echo "🦀 Running Lint & Format in Docker..."
        docker compose run --rm dev bash -c "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
        ;;
    test)
        echo "🧪 Running Tests in Docker..."
        docker compose run --rm dev cargo test --workspace
        ;;
    all)
        echo "🦀 Running Lint & Format in Docker..."
        docker compose run --rm dev bash -c "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
        echo "🧪 Running Tests in Docker..."
        docker compose run --rm dev cargo test --workspace
        ;;
    *)
        echo "❌ Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac

echo "✨ Checks completed successfully in the Docker environment!"
