# 🐳 Docker Infrastructure & Caching Strategy

This project uses a unified Docker-based workflow to ensure 100% environment parity between local development and CI/CD.

## 🚀 Unified Workflow (`run-in-docker.sh`)

All quality checks are funneled through the `./run-in-docker.sh` entry point.

| Command | Action |
| :--- | :--- |
| `lint` | Runs `cargo fmt` and `cargo clippy`. |
| `test` | Runs `cargo test --workspace`. |
| `run <cmd>` | Runs an arbitrary command inside the `dev` container. |
| `all` | DEFAULT. Runs `lint` then `test`. |

## 🏗️ Hybrid Caching Architecture

We leverage a hybrid approach to balance performance on different host platforms (macOS/Windows/Linux) and persistence in CI environments.

### 1. Cargo Registry (`~/.cargo/registry`)
- **Strategy**: Bind-mounted from the host machine.
- **Goal**: Share downloaded dependencies between host and container. If you run `cargo build` on your host, the container won't need to re-download anything.

### 2. Build Artifacts (`/workspace/target`)
- **Local (Named Volume)**: We use a Docker Named Volume (`target-cache`).
  - **Why?**: Sharing `target/` via bind-mount is extremely slow on macOS/Windows (I/O overhead) and causes permission/compatibility issues across platforms. Named volumes run at native speed.
- **CI (Host Path)**: In CI, we map this to a physical directory (`/tmp/target_cache`).
  - **Why?**: GitHub Actions jobs are ephemeral. Named volumes are lost between jobs. By using a physical path, we can persist it across jobs using `actions/cache@v4`.
- **Permissions (CI)**: Since Docker runs as root, files in `/tmp` are root-owned. `run-in-docker.sh` automatically runs `chown -R` at the end of the script in CI mode to restore access to the host runner user.

### 3. Sccache (Compilation Cache)
- **Tool**: `sccache` is enabled by default inside the container.
- **Config**: Rooted at `/workspace/.sccache`.
- **Strategy**: Similar to the target directory, it uses a Named Volume locally and a cached host-path in CI.

## 🛡️ Git Hooks (Pre-commit)

A pre-commit hook is installed at `.git/hooks/pre-commit`.
- **Logic**: Executes `./run-in-docker.sh lint`.
- **Enforcement**: Blocks commits if code is unformatted or contains clippy warnings.

## 🏁 Overrides

You can override the caching behavior using environment variables:

```bash
# Force using a local directory for target (Linux only suggested)
export TARGET_DIR=./target
./run-in-docker.sh

# Skip image rebuild (Fast-track)
NO_BUILD=true ./run-in-docker.sh
```
