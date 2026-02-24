#!/bin/bash
set -e

# Configuration
API_URL="http://127.0.0.1:8787"
ADMIN_SECRET="my-secret-token"
DEVICE_ID="test-device-$(date +%s)"

echo "🚀 Starting Integration Tests for Key Server..."

cleanup() {
    echo "🧹 Cleaning up..."
    kill $WRANGLER_PID 2>/dev/null || true
}
trap cleanup EXIT

# 2. Start Wrangler Dev in background
echo "📥 Starting Wrangler Dev (Local Mode)..."
# Use --local to avoid remote dependency
wrangler dev --config wrangler.jsonc --local --persist --d1 DB --port 8787 --non-interactive --ip 127.0.0.1 > wrangler.log 2>&1 &
WRANGLER_PID=$!
echo "Wrangler PID: $WRANGLER_PID"

# Wait for server to start
MAX_RETRIES=30
COUNT=0
while ! curl -s $API_URL > /dev/null; do
    sleep 1
    COUNT=$((COUNT + 1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo "❌ Server failed to start. Logs:"
        cat wrangler.log
        exit 1
    fi
done
echo "✅ Server is up!"

# 3. Test /api/config
echo "🧪 Testing /api/config..."
CONFIG=$(curl -s $API_URL/api/config)
echo "Response: $CONFIG"
echo $CONFIG | grep -q "min_cli_version"

# 4. Test /api/stats (Empty)
echo "🧪 Testing /api/stats (Initial)..."
STATS=$(curl -s $API_URL/api/stats)
echo "Response: $STATS"
echo $STATS | grep -q "\"total\":0"

# 5. Add a test tunnel (Admin)
echo "🧪 Adding test tunnel..."
ADD_RES=$(curl -s -X POST -H "Authorization: Bearer $ADMIN_SECRET" \
    -H "Content-Type: application/json" \
    -d '{"id": "t1", "name": "test-tunnel", "token": "tok1"}' \
    $API_URL/admin/tunnels)
echo "Response: $ADD_RES"
echo $ADD_RES | grep -q "\"success\":true"

# 6. Test /api/stats (1 Available)
echo "🧪 Testing /api/stats (After add)..."
STATS=$(curl -s $API_URL/api/stats)
echo "Response: $STATS"
echo $STATS | grep -q "\"available\":1"

# 7. Request Tunnel
echo "🧪 Testing /api/request..."
REQ_RES=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"device_id\": \"$DEVICE_ID\", \"port\": 8080}" \
    $API_URL/api/request)
echo "Response: $REQ_RES"
echo $REQ_RES | grep -q "\"success\":true"
echo $REQ_RES | grep -q "\"token\":\"tok1\""

# 8. Send Heartbeat
echo "🧪 Testing /api/heartbeat..."
HB_RES=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"device_id\": \"$DEVICE_ID\"}" \
    $API_URL/api/heartbeat)
echo "Response: $HB_RES"
echo $HB_RES | grep -q "\"success\":true"

# 9. Test /api/stats (1 Busy)
echo "🧪 Testing /api/stats (After request)..."
STATS=$(curl -s $API_URL/api/stats)
echo "Response: $STATS"
echo $STATS | grep -q "\"busy\":1"

# 10. Release Tunnel
echo "🧪 Testing /api/release..."
REL_RES=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"device_id\": \"$DEVICE_ID\"}" \
    $API_URL/api/release)
echo "Response: $REL_RES"
echo $REL_RES | grep -q "\"success\":true"

# 11. Test Error Case: Restricted Port (22)
echo "🧪 Testing /api/request (Restricted Port 22)..."
ERR_RES=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"device_id\": \"$DEVICE_ID\", \"port\": 22}" \
    $API_URL/api/request)
echo "Response: $ERR_RES"
echo $ERR_RES | grep -q "\"success\":false"
echo $ERR_RES | grep -q "restricted"

echo "🎉 All Integration Tests Passed Successfully!"
