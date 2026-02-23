# --- Stage 1: Planner ---
FROM lukemathwalker/cargo-chef:latest-rust-1.93.1-slim-bookworm AS planner
WORKDIR /workspace
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 2: Cacher ---
FROM lukemathwalker/cargo-chef:latest-rust-1.93.1-slim-bookworm AS cacher
WORKDIR /workspace
COPY --from=planner /workspace/recipe.json recipe.json

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev musl-tools build-essential cmake curl \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/* \
    && echo '#!/bin/sh' > /usr/bin/aarch64-linux-musl-gcc \
    && echo 'exec aarch64-linux-gnu-gcc -specs /usr/lib/aarch64-linux-musl/musl-gcc.specs "$@"' >> /usr/bin/aarch64-linux-musl-gcc \
    && chmod +x /usr/bin/aarch64-linux-musl-gcc \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl \
    && cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json \
    && cargo chef cook --release --recipe-path recipe.json

# --- Stage 3: Base ---
FROM rust:1.93.1-slim-bookworm AS base
WORKDIR /workspace
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl git pkg-config libssl-dev musl-tools build-essential cmake \
    gcc-aarch64-linux-gnu \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && echo '#!/bin/sh' > /usr/bin/aarch64-linux-musl-gcc \
    && echo 'exec aarch64-linux-gnu-gcc -specs /usr/lib/aarch64-linux-musl/musl-gcc.specs "$@"' >> /usr/bin/aarch64-linux-musl-gcc \
    && chmod +x /usr/bin/aarch64-linux-musl-gcc \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl wasm32-unknown-unknown \
    && rustup component add rustfmt clippy \
    && cargo install --locked sccache --version ^0.8 \
    && cargo install --locked worker-build \
    && cargo install --locked cargo-tarpaulin

# --- Stage 4: Dev ---
FROM base AS dev
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/workspace/.sccache
COPY --from=cacher /workspace/target /workspace/target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY --from=planner /workspace/recipe.json recipe.json
CMD ["bash"]
