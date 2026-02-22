# Contributing to xpose

First off, thank you for considering contributing to `xpose`! It's people like you who make the open-source world such an amazing place.

## How Can I Contribute?

### Reporting Bugs
If you find a bug, please create a GitHub issue. Include:
- A clear description of the problem.
- Steps to reproduce.
- Your OS and Rust version.

### Suggested Enhancements
We welcome ideas for new features! Please open an issue to discuss your proposal before starting implementation.

### Pull Requests
1. Fork the repo.
2. Create your feature branch (`git checkout -b feature/AmazingFeature`).
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4. Push to the branch (`git push origin feature/AmazingFeature`).
5. Open a Pull Request.

## Development Setup

**CRITICAL**: All development, testing, and building must be performed inside the Docker environment to ensure platform parity and zero-defect deployments.

### 🐳 The Docker Utility
We use `./run-in-docker.sh` as the primary entry point for all quality checks. See [docs/DOCKER.md](docs/DOCKER.md) for detailed technical info on our caching architecture.
- **Linting**: `./run-in-docker.sh lint` (Format & Clippy).
- **Testing**: `./run-in-docker.sh test` (Workspace tests).
- **Verify All**: `./run-in-docker.sh all` (Runs both).

### 🛡️ Quality Gates
- **Pre-commit**: A Git hook is installed to automatically run `lint` before every commit.
- **CI Parity**: The script is used directly in GitHub Actions with `NO_BUILD=true` to ensure that local and CI results are identical.
- **Caching**: The system uses persistent volumes and `sccache` for near-instant incremental builds. Use `export TARGET_DIR=./target` if you want to share artifacts with the host (Linux only).

### Coding Standards
- Always use the `./run-in-docker.sh` tool before claiming a task is done.
- Follow the "Protocol-First" approach documented in `.ai-context-os/PROJECT_OS.md`.
- Ensure all tests pass 100% before requesting a review.

## Legal
By contributing, you agree that your contributions will be licensed under the project's MIT License.
