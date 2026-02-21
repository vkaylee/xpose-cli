# Optimized Multi-stage Dockerfile for Cloudflare Tunnel CLI (Rust🦀)

# --- Stage 1: Planner ---
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim-bookworm AS planner
WORKDIR /workspace
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 2: Cacher ---
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim-bookworm AS cacher
WORKDIR /workspace
COPY --from=planner /workspace/recipe.json recipe.json

# Install build dependencies once
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    musl-tools \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Pre-compile dependencies
RUN rustup target add x86_64-unknown-linux-musl && \
    cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# --- Stage 3: Developer Environment ---
FROM rust:1.85-slim-bookworm AS dev
WORKDIR /workspace

# Combine system dependencies and Node.js installation into a single optimized layer
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    git \
    pkg-config \
    libssl-dev \
    musl-tools \
    build-essential \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g wrangler \
    && rm -rf /var/lib/apt/lists/*

# Setup Rust components and sccache in one step
RUN rustup target add x86_64-unknown-linux-musl && \
    rustup component add rustfmt clippy && \
    cargo install sccache --version ^0.8

# Configure sccache
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/workspace/.sccache

# Sync pre-compiled layers from cacher
COPY --from=cacher /workspace/target /workspace/target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

CMD ["bash"]
