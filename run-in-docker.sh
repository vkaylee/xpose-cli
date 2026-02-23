#!/bin/bash
set -e

# --- Usage ---
show_usage() {
    echo "Usage: ./run-in-docker.sh [command]"
    echo ""
    echo "Commands:"
    echo "  lint    Run cargo fmt and clippy checks"
    echo "  test    Run workspace unit tests"
    echo "  coverage Run test coverage measurement (Tarpaulin)"
    echo "  run     Run a custom command inside the container"
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
    export TARGET_DIR=${TARGET_DIR:-/tmp/target_cache}
    mkdir -p "$CARGO_HOME_REGISTRY" "$CARGO_HOME_GIT" "$SCCACHE_CACHE_DIR" "$TARGET_DIR"
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
        echo "🧐 Checking Version Sync..."
        PKG_VERSION=$(grep '"version":' packages/cli/package.json | head -n 1 | cut -d '"' -f 4)
        CARGO_VERSION=$(grep '^version =' packages/cli/Cargo.toml | head -n 1 | cut -d '"' -f 2)
        
        if [ "$PKG_VERSION" != "$CARGO_VERSION" ]; then
            echo "❌ Version mismatch detected!"
            echo "package.json: $PKG_VERSION"
            echo "Cargo.toml: $CARGO_VERSION"
            exit 1
        fi
        echo "✅ Versions match ($PKG_VERSION)."

        echo "🦀 Running Lint & Format in Docker..."
        docker compose run --rm dev bash -c "cargo check --locked && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
        ;;
    test)
        echo "🧪 Running Tests in Docker..."
        docker compose run --rm dev cargo test --workspace
        ;;
    coverage)
        echo "📊 Running Coverage Measurement in Docker..."
        docker compose run --rm dev cargo tarpaulin --workspace --engine Llvm --out Lcov --output-dir target/coverage
        ;;
    run)
        shift
        echo "🏃 Running custom command in Docker: $*"
        docker compose run --rm dev "$@"
        ;;
    all)
        echo "🧐 Checking Version Sync..."
        PKG_VERSION=$(grep '"version":' packages/cli/package.json | head -n 1 | cut -d '"' -f 4)
        CARGO_VERSION=$(grep '^version =' packages/cli/Cargo.toml | head -n 1 | cut -d '"' -f 2)
        
        if [ "$PKG_VERSION" != "$CARGO_VERSION" ]; then
            echo "❌ Version mismatch detected!"
            echo "package.json: $PKG_VERSION"
            echo "Cargo.toml: $CARGO_VERSION"
            exit 1
        fi
        echo "✅ Versions match ($PKG_VERSION)."

        echo "🦀 Running Lint & Format in Docker..."
        docker compose run --rm dev bash -c "cargo check --locked && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
        echo "🧪 Running Tests in Docker..."
        docker compose run --rm dev cargo test --workspace --locked
        ;;
    *)
        echo "❌ Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac

if [ "$GITHUB_ACTIONS" = "true" ]; then
    echo "🧹 Correcting permissions for CI cache..."
    # The container runs as root, but host needs access to files for caching
    # We use a temporary container to fix permissions of the mounted volumes
    docker compose run --rm --entrypoint chown dev -R $(id -u):$(id -g) /usr/local/cargo/registry /usr/local/cargo/git /workspace/target /workspace/.sccache
fi

echo "✨ Checks completed successfully in the Docker environment!"
