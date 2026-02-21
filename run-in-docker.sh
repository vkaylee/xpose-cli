#!/bin/bash
set -e

echo "🐳 Building Docker image for environment parity..."
docker build -t xpose-builder .

echo "🦀 Running Lint & Format in Docker..."
docker run --rm -v $(pwd):/workspace xpose-builder bash -c "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"

echo "🧪 Running Tests in Docker..."
docker run --rm -v $(pwd):/workspace xpose-builder cargo test --workspace

echo "✨ All local checks passed successfully in the Docker environment!"
