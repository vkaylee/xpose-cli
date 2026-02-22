#!/bin/bash
# NPM Publish Debug Script
# Run in GitHub Actions to debug npm publish issues

set -e

echo "🔍 NPM Publish Debug Script"
echo "=============================="

# 1. Check Node/npm version
echo ""
echo "📌 Node & NPM versions:"
node --version
npm --version

# 2. NPM config
echo ""
echo "📋 NPM Config:"
npm config list --json || true

# 3. Check .npmrc
echo ""
echo "📄 .npmrc file:"
if [ -f ".npmrc" ]; then
    cat .npmrc
else
    echo "No .npmrc found"
fi

# 4. Check package.json
echo ""
echo "📦 Package info:"
if [ -f "package.json" ]; then
    cat package.json
fi

# 5. NPM whoami
echo ""
echo "🔐 NPM Whoami:"
npm whoami 2>&1 || echo "Not logged in (expected for CI)"

# 6. Environment variables (masked)
echo ""
echo "🔧 Environment:"
echo "NPM_TOKEN: ${NPM_TOKEN:+***SET***}"
echo "NODE_AUTH_TOKEN: ${NODE_AUTH_TOKEN:+***SET***}"

# 7. Dry-run publish
echo ""
echo "🧪 Dry-run publish:"
npm publish --access public --provenance --dry-run 2>&1 || true

echo ""
echo "=============================="
echo "Debug complete!"
