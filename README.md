# Cloudflare Tunnel CLI (xpose)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Version](https://img.shields.io/badge/version-0.2.0-green.svg)

A lightning-fast, zero-config, terminal UI (TUI) wrapper for Cloudflare Tunnels (`cloudflared`). Written entirely in Rust 🦀, distributed via NPM.

`xpose` takes the pain out of exposing your local development servers to the internet. It automatically handles binary downloads, tunnel management, and provides a beautiful, hacker-style real-time dashboard.

- **Development**: Docker-first workflow with unified local/CI parity. See [docs/DOCKER.md](docs/DOCKER.md).

---

## 🚀 Features

- **Zero-Config**: Just type `xpose <port>` and you're online. No accounts or complex setups needed.
- **Vivid Terminal UI**: Real-time ASCII sparkline charts (`[ ▃▅▇█▆▄ ]`), traffic monitoring (Rx/Tx), and ping latency.
- **Developer Convenience**: 
  - Generates a **QR Code** directly in your terminal for instant mobile testing.
  - **Auto-copies** the public URL to your clipboard.
  - Automatically scans common ports (3000, 8000, 8080) if no port is provided.
- **Smart Cloudflared Management**:
  - Downloads the correct statically linked binary for your OS and Architecture (Windows, macOS Intel/ARM, Linux).
  - Background "Tunnel Pooling" avoids Cloudflare limits and ensures instant connections.
- **Hardened Security**:
  - Built-in IP Rate Limiting.
  - Automatic Phishing prevention (blocks sensitive subdomains like `login`, `bank`).
  - Restricts exposure to safe development ports only.
- **Interactive Monitoring Hub**:
  - `xpose dashboard`: A full-screen TUI to manage multiple tunnels simultaneously.
- **Interactive Control**: Manage tunnels with single-key shortcuts.
  - `X`: Stop the selected tunnel (sends SIGTERM).
  - `R`: Restart the selected tunnel.
  - `Q` or `Esc`: Quit dashboard.
  - `↑/↓`: Navigate sessions.

### Multi-language Support
xpose support English (default), Vietnamese, and Chinese.
- **Auto-detection**: Automatically uses system language.
- **Manual override**: Use the `--lang` flag.
  ```bash
  xpose --lang vi
  xpose dashboard --lang zh
  ```

---

## 📦 Installation

Since `xpose` is distributed as an NPM package, installation is as simple as:

```bash
npm install -g xpose-cli
```

*(Note: The NPM wrapper automatically fetches the blazing-fast Rust binary optimized for your system).*

---

## 💻 Usage

Expose a local port to the internet instantly:

```bash
# Expose port 3000 (TCP)
xpose 3000

# Expose port 8080 via UDP
xpose 8080 --udp

# Auto-detect (scans 3000, 8000, 8080 or reads MT_TUNNEL_PORT from .env)
xpose

# Open the Interactive Management Dashboard
xpose dashboard
```

---

## 🏗️ Architecture

The project consists of two core components, both written in Rust:

### 1. The CLI Client (`packages/cli`)
A Rust application utilizing `tokio` for async operations, `reqwest` for API communication, and `crossterm`/`indicatif` for the vivid ASCII user interface. It acts as an intelligent wrapper around the official `cloudflared` binary.

### 2. The Key Server (`packages/key-server`)
A Cloudflare Worker written in Rust (`workers-rs`) using D1 (SQLite) for state management. 
- Maintains a pool of ready-to-use Cloudflare Tunnels (Quick Tunnel instances).
- Manages sub-domain leasing to connected clients.
- Enforces security rules (IP Rate Limiting, Keyword Filtering, Port Restrictions).
- Automatic garbage collection of dead tunnels via Cron triggers.

---

## 🛠️ Development Setup

If you wish to build the project from source:

### Prerequisites
- Docker and Docker Compose installed on your host machine.

### Docker Development Environment (Required)
For a consistent environment across all platforms, **all development, building, and testing must be done via Docker**. Do NOT run `cargo` or `npm` directly on your host machine.

```bash
# 1. Start the development container in the background
docker-compose up -d

# 2. Enter the container shell
docker-compose exec dev bash

# 3. Inside the container, you can run cargo/npm normally:
cd packages/cli
cargo build --release                   # Build the CLI
cargo test                              # Run tests

cd /workspace/packages/key-server
npm install                             # Install dependencies
wrangler d1 migrations apply DB --local # Run database migrations
wrangler dev                            # Start the Key Server locally

# If you need to build the standalone Linux binary:
cargo build --release --target x86_64-unknown-linux-musl
```

---

## 📝 License

This project is licensed under the MIT License. 

By using this tool, you also agree to the Cloudflare Terms of Service as `xpose` acts as a wrapper for `cloudflared` (Apache-2.0 License). The CLI will automatically fetch and store the required Cloudflare license upon first run.
