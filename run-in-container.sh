#!/bin/bash
set -e

show_usage() {
    echo "Usage: ./run-in-container.sh [command]"
    echo ""
    echo "Commands:"
    echo "  lint       Run cargo fmt and clippy checks"
    echo "  test       Run workspace unit tests"
    echo "  integration Run API integration tests"
    echo "  coverage   Run test coverage measurement (Tarpaulin)"
    echo "  run        Run a custom command inside the container"
    echo "  all        Run both lint and test in single container (default)"
    echo "  help       Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  GITHUB_ACTIONS=true    Enable CI-optimized output and caching"
    echo "  NO_BUILD=true          Skip the build phase"
    echo "  PARALLEL_TESTS=true   Run tests in parallel"
}

# --- Auto-detect container engine ---
if command -v podman &>/dev/null; then
    ENGINE="podman"
    COMPOSE_CMD="podman-compose"
elif command -v docker &>/dev/null; then
    ENGINE="docker"
    COMPOSE_CMD="docker compose"
    export DOCKER_BUILDKIT=1
    export COMPOSE_DOCKER_CLI_BUILD=1
else
    echo "❌ Neither podman nor docker found. Please install one."
    exit 1
fi
echo "🐳 Using $ENGINE engine"

COMMAND=${1:-all}

if [[ "$COMMAND" == "help" || "$COMMAND" == "--help" || "$COMMAND" == "-h" ]]; then
    show_usage
    exit 0
fi

if [ "$GITHUB_ACTIONS" = "true" ]; then
    echo "Running in CI mode..."
    export CARGO_HOME_REGISTRY=${CARGO_HOME_REGISTRY:-/tmp/cargo_registry}
    export CARGO_HOME_GIT=${CARGO_HOME_GIT:-/tmp/cargo_git}
    export SCCACHE_CACHE_DIR=${SCCACHE_CACHE_DIR:-/tmp/sccache_cache}
    export TARGET_DIR=${TARGET_DIR:-/tmp/target_cache}
    mkdir -p "$CARGO_HOME_REGISTRY" "$CARGO_HOME_GIT" "$SCCACHE_CACHE_DIR" "$TARGET_DIR"
COMPOSE_FLAGS="--progress=plain"
    export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    export CARGO_NET_RETRY=3
    export RUST_LOG=cargo::ops=warn
else
    echo "Running in local mode..."
    COMPOSE_FLAGS=""
fi

if [ "$NO_BUILD" != "true" ] && [ "$SKIP_BUILD" != "true" ]; then
    echo "Building container image via Compose..."
    $COMPOSE_CMD build $COMPOSE_FLAGS dev
fi

check_version_sync() {
    PKG_VERSION=$(grep '"version":' packages/cli/package.json | head -n 1 | cut -d '"' -f 4)
    CARGO_VERSION=$(grep '^version =' packages/cli/Cargo.toml | head -n 1 | cut -d '"' -f 2)
    
    if [ "$PKG_VERSION" != "$CARGO_VERSION" ]; then
        echo "Version mismatch: package.json=$PKG_VERSION, Cargo.toml=$CARGO_VERSION"
        exit 1
    fi
    echo "Versions match ($PKG_VERSION)."
}

case "$COMMAND" in
    lint)
        check_version_sync
        $COMPOSE_CMD run --rm dev bash -c "CARGO_INCREMENTAL=0 cargo check --locked && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
        ;;
    test)
        TEST_FLAGS="--workspace --locked --lib --bins"
        [ "$PARALLEL_TESTS" = "true" ] && TEST_FLAGS="$TEST_FLAGS -- --test-threads=4"
        $COMPOSE_CMD run --rm dev bash -c "CARGO_INCREMENTAL=0 cargo test $TEST_FLAGS"
        ;;
    integration)
        $COMPOSE_CMD run --rm dev bash -c "cd packages/key-server && ./tests/api_tests.sh"
        ;;
    coverage)
        $COMPOSE_CMD run --rm dev bash -c "CARGO_INCREMENTAL=0 cargo tarpaulin --workspace --engine Llvm --out Lcov --output-dir target/coverage --lib --bins"
        ;;
    run)
        shift
        $COMPOSE_CMD run --rm dev "$@"
        ;;
    all)
        check_version_sync
        $COMPOSE_CMD run --rm dev bash -c "CARGO_INCREMENTAL=0 cargo check --locked && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace --locked --lib --bins"
        ;;
    *)
        echo "Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac

if [ "$GITHUB_ACTIONS" = "true" ]; then
    $COMPOSE_CMD run --rm --entrypoint chown dev -R $(id -u):$(id -g) /usr/local/cargo/registry /usr/local/cargo/git /workspace/target /workspace/.sccache 2>/dev/null || true
fi

echo "Done!"
