#!/bin/bash
set -e

show_usage() {
    echo "Usage: ./dev.sh [command]"
    echo ""
    echo "Local Development Helper — Run key-server + CLI locally"
    echo ""
    echo "Local Commands:"
    echo "  up              Start key-server-dev (port 8787) in background"
    echo "  down            Stop all dev services"
    echo "  logs            Follow key-server-dev logs"
    echo "  status          Show service status and health"
    echo "  test-api        Run quick smoke test against local key-server"
    echo "  cli [args]      Build & run xpose CLI pointed at local key-server"
    echo "  shell           Open a bash shell in the dev container"
    echo ""
    echo "Staging Commands (Cloudflare):"
    echo "  staging-setup   Create D1 database + deploy staging worker (one-time)"
    echo "  staging-deploy  Build & deploy key-server to staging"
    echo "  staging-test    Smoke test against staging worker"
    echo "  staging-cli     Run CLI pointed at staging worker"
    echo ""
    echo "  help            Show this help message"
}

COMMAND=${1:-help}

# --- Auto-detect container engine ---
if command -v podman &>/dev/null; then
    ENGINE="podman"
    COMPOSE_CMD="podman-compose"
elif command -v docker &>/dev/null; then
    ENGINE="docker"
    COMPOSE_CMD="docker compose"
else
    echo "❌ Neither podman nor docker found. Please install one."
    exit 1
fi

case "$COMMAND" in
    up)
        echo "🚀 Starting key-server-dev..."
        $COMPOSE_CMD up key-server-dev -d --build
        echo ""
        echo "⏳ Waiting for key-server to be healthy (first build may take 2-5 min)..."
        echo "   Run './dev.sh logs' in another terminal to watch progress."
        echo ""
        # Wait for healthy
        MAX_WAIT=300
        ELAPSED=0
        while [ $ELAPSED -lt $MAX_WAIT ]; do
            STATUS=$($COMPOSE_CMD ps key-server-dev --format json 2>/dev/null | grep -o '"Health":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
            if [ "$STATUS" = "healthy" ]; then
                echo "✅ key-server-dev is healthy!"
                echo ""
                echo "  API:  http://localhost:8787"
                echo "  CLI:  ./dev.sh cli 3000"
                echo "  Logs: ./dev.sh logs"
                exit 0
            fi
            sleep 5
            ELAPSED=$((ELAPSED + 5))
            echo "   ... waiting ($ELAPSED/${MAX_WAIT}s) status=$STATUS"
        done
        echo "⚠️  Timed out waiting for healthy status. Check logs:"
        echo "   ./dev.sh logs"
        exit 1
        ;;
    down)
        echo "🛑 Stopping dev services..."
        $COMPOSE_CMD down
        echo "✅ Done."
        ;;
    logs)
        $COMPOSE_CMD logs -f key-server-dev
        ;;
    status)
        $COMPOSE_CMD ps key-server-dev
        echo ""
        echo "Quick API check:"
        curl -sf http://localhost:8787/api/config 2>/dev/null && echo "" || echo "❌ Key server not responding"
        ;;
    test-api)
        echo "🧪 Running smoke tests against http://localhost:8787..."
        echo ""

        # Test /api/config
        echo "  [1/4] GET /api/config"
        CONFIG=$(curl -sf http://localhost:8787/api/config)
        echo "$CONFIG" | grep -q "min_cli_version" && echo "  ✅ Config OK" || { echo "  ❌ Config failed"; exit 1; }

        # Test /api/stats
        echo "  [2/4] GET /api/stats"
        STATS=$(curl -sf http://localhost:8787/api/stats)
        echo "$STATS" | grep -q "total" && echo "  ✅ Stats OK" || { echo "  ❌ Stats failed"; exit 1; }

        # Test admin add tunnel
        echo "  [3/4] POST /admin/tunnels (add test tunnel)"
        ADD=$(curl -sf -X POST -H "Authorization: Bearer my-secret-token" \
            -H "Content-Type: application/json" \
            -d '{"id": "dev-t1", "name": "dev-test", "token": "dev-tok1"}' \
            http://localhost:8787/admin/tunnels)
        echo "$ADD" | grep -q '"success":true' && echo "  ✅ Add tunnel OK" || { echo "  ❌ Add tunnel failed: $ADD"; exit 1; }

        # Test stats updated
        echo "  [4/4] GET /api/stats (verify tunnel added)"
        STATS2=$(curl -sf http://localhost:8787/api/stats)
        echo "$STATS2" | grep -q '"available":1' && echo "  ✅ Stats updated OK" || echo "  ⚠️  Stats: $STATS2"

        echo ""
        echo "🎉 All smoke tests passed!"
        ;;
    cli)
        shift
        echo "🔨 Building and running xpose CLI → http://localhost:8787"
        $COMPOSE_CMD run --rm \
            -e XPOSE_SERVER_URL=http://key-server-dev:8787 \
            dev bash -c "cd packages/cli && cargo run -- $*"
        ;;
    shell)
        $COMPOSE_CMD run --rm dev bash
        ;;
    staging-setup)
        echo "🔧 Setting up staging environment..."
        echo ""

        # Check for .env.staging
        if [ ! -f .env.staging ]; then
            echo "📝 Creating .env.staging from template..."
            cp .env.staging.example .env.staging
            echo "⚠️  Please edit .env.staging and set CLOUDFLARE_API_TOKEN first."
            echo "   Then re-run: ./dev.sh staging-setup"
            exit 1
        fi

        source .env.staging

        if [ -z "$CLOUDFLARE_API_TOKEN" ] || [ "$CLOUDFLARE_API_TOKEN" = "your-api-token-here" ]; then
            echo "❌ CLOUDFLARE_API_TOKEN not set in .env.staging"
            exit 1
        fi

        # Create D1 database if not already done
        if [ -z "$STAGING_D1_ID" ] || [ "$STAGING_D1_ID" = "your-staging-d1-id-here" ]; then
            echo "🗄️  Creating D1 database: tunnel-db-staging..."
            D1_OUTPUT=$($COMPOSE_CMD run --rm \
                -e CLOUDFLARE_API_TOKEN="$CLOUDFLARE_API_TOKEN" \
                dev bash -c "cd packages/key-server && wrangler d1 create tunnel-db-staging" 2>&1)
            echo "$D1_OUTPUT"
            NEW_ID=$(echo "$D1_OUTPUT" | grep -o 'database_id.*=.*"[^"]*"' | grep -o '"[^"]*"' | tr -d '"' || true)
            if [ -z "$NEW_ID" ]; then
                NEW_ID=$(echo "$D1_OUTPUT" | grep -oP '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1 || true)
            fi
            if [ -n "$NEW_ID" ]; then
                echo ""
                echo "✅ D1 database created! ID: $NEW_ID"
                sed -i "s/STAGING_D1_ID=.*/STAGING_D1_ID=$NEW_ID/" .env.staging
                sed -i "s/__STAGING_D1_ID__/$NEW_ID/" packages/key-server/wrangler.jsonc
                echo "✅ Updated .env.staging and wrangler.jsonc"
            else
                echo "⚠️  Could not auto-extract D1 ID. Please update .env.staging manually."
                exit 1
            fi
        else
            echo "✅ D1 database already configured: $STAGING_D1_ID"
            sed -i "s/__STAGING_D1_ID__/$STAGING_D1_ID/" packages/key-server/wrangler.jsonc 2>/dev/null || true
        fi

        source .env.staging

        # Run migrations + deploy inside container
        echo ""
        echo "📦 Running D1 migrations + deploying to staging..."
        $COMPOSE_CMD run --rm \
            -e CLOUDFLARE_API_TOKEN="$CLOUDFLARE_API_TOKEN" \
            -e CLOUDFLARE_ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-}" \
            dev bash -c "
                cd packages/key-server &&
                wrangler d1 migrations apply tunnel-db-staging --remote --env staging &&
                echo '' &&
                echo '🚀 Deploying key-server-staging...' &&
                wrangler deploy --env staging
            "

        # Set tunnel provisioning secrets (if configured)
        if [ -n "${CLOUDFLARE_API_TUNNEL_TOKEN:-}" ] && [ "$CLOUDFLARE_API_TUNNEL_TOKEN" != "your-tunnel-api-token-here" ]; then
            echo ""
            echo "🔐 Setting tunnel provisioning secrets..."
            $COMPOSE_CMD run --rm \
                -e CLOUDFLARE_API_TOKEN="$CLOUDFLARE_API_TOKEN" \
                -e CLOUDFLARE_ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-}" \
                dev bash -c "
                    cd packages/key-server &&
                    echo '${CLOUDFLARE_API_TUNNEL_TOKEN}' | wrangler secret put CLOUDFLARE_API_TUNNEL_TOKEN --env staging &&
                    echo '${CLOUDFLARE_ACCOUNT_ID}' | wrangler secret put CLOUDFLARE_ACCOUNT_ID --env staging
                "
            # Set tunnel domain only if configured (optional — omit for quick tunnels)
            if [ -n "${CLOUDFLARE_TUNNEL_DOMAIN:-}" ] && [ "$CLOUDFLARE_TUNNEL_DOMAIN" != "your-tunnel-domain-here" ]; then
                $COMPOSE_CMD run --rm \
                    -e CLOUDFLARE_API_TOKEN="$CLOUDFLARE_API_TOKEN" \
                    -e CLOUDFLARE_ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-}" \
                    dev bash -c "
                        cd packages/key-server &&
                        echo '${CLOUDFLARE_TUNNEL_DOMAIN}' | wrangler secret put CLOUDFLARE_TUNNEL_DOMAIN --env staging
                    "
                echo "✅ Tunnel secrets configured (with custom domain: $CLOUDFLARE_TUNNEL_DOMAIN)"
            else
                echo "✅ Tunnel secrets configured (quick tunnels — no custom domain)"
            fi
        else
            echo ""
            echo "⚠️  Tunnel secrets not set (CLOUDFLARE_API_TUNNEL_TOKEN not configured in .env.staging)"
            echo "   CLI tunnel allocation will not work until secrets are configured."
        fi

        echo ""
        echo "🎉 Staging setup complete!"
        echo ""
        echo "  Worker URL: Check output above or Cloudflare dashboard"
        echo "  Update STAGING_WORKER_URL in .env.staging"
        echo ""
        echo "  Next: ./dev.sh staging-test"
        ;;
    staging-deploy)
        echo "🚀 Deploying key-server to staging..."

        if [ ! -f .env.staging ]; then
            echo "❌ .env.staging not found. Run './dev.sh staging-setup' first."
            exit 1
        fi
        source .env.staging

        $COMPOSE_CMD run --rm \
            -e CLOUDFLARE_API_TOKEN="$CLOUDFLARE_API_TOKEN" \
            -e CLOUDFLARE_ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-}" \
            dev bash -c "
                cd packages/key-server &&
                wrangler d1 migrations apply tunnel-db-staging --remote --env staging &&
                wrangler deploy --env staging
            "

        echo ""
        echo "✅ Staging deploy complete!"
        ;;
    staging-test)
        if [ ! -f .env.staging ]; then
            echo "❌ .env.staging not found. Run './dev.sh staging-setup' first."
            exit 1
        fi
        source .env.staging

        if [ -z "$STAGING_WORKER_URL" ] || echo "$STAGING_WORKER_URL" | grep -q 'your-subdomain'; then
            echo "❌ STAGING_WORKER_URL not configured in .env.staging"
            echo "   Set it to your staging worker URL, e.g.: https://key-server-staging.xxx.workers.dev"
            exit 1
        fi

        URL="$STAGING_WORKER_URL"
        echo "🧪 Running smoke tests against staging: $URL"
        echo ""

        echo "  [1/3] GET /api/config"
        CONFIG=$(curl -sf "$URL/api/config")
        echo "$CONFIG" | grep -q "min_cli_version" && echo "  ✅ Config OK" || { echo "  ❌ Config failed"; exit 1; }

        echo "  [2/3] GET /api/stats"
        STATS=$(curl -sf "$URL/api/stats")
        echo "$STATS" | grep -q "total" && echo "  ✅ Stats OK" || { echo "  ❌ Stats failed"; exit 1; }

        echo "  [3/3] POST /admin/tunnels"
        ADD=$(curl -sf -X POST -H "Authorization: Bearer staging-secret-token" \
            -H "Content-Type: application/json" \
            -d '{"id": "staging-t1", "name": "staging-test", "token": "staging-tok1"}' \
            "$URL/admin/tunnels")
        echo "$ADD" | grep -q '"success":true' && echo "  ✅ Add tunnel OK" || { echo "  ❌ Add tunnel failed: $ADD"; exit 1; }

        echo ""
        echo "🎉 All staging smoke tests passed!"
        ;;
    staging-cli)
        if [ ! -f .env.staging ]; then
            echo "❌ .env.staging not found. Run './dev.sh staging-setup' first."
            exit 1
        fi
        source .env.staging

        if [ -z "$STAGING_WORKER_URL" ] || echo "$STAGING_WORKER_URL" | grep -q 'your-subdomain'; then
            echo "❌ STAGING_WORKER_URL not configured in .env.staging"
            exit 1
        fi

        shift
        echo "🔨 Building and running xpose CLI → $STAGING_WORKER_URL"
        $COMPOSE_CMD run --rm \
            -e XPOSE_SERVER_URL="$STAGING_WORKER_URL" \
            dev bash -c "cd packages/cli && cargo run -- $*"
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        echo "Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac
