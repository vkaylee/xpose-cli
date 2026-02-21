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

**CRITICAL**: All development, testing, and building must be performed inside the Docker environment. Do not run `cargo` or `npm` commands directly on your host machine.

See the [README.md](README.md) for detailed instructions on starting the Docker container using `docker-compose`.

### Coding Standards
- Enter the Docker container: `docker-compose exec dev bash`
- Run `cargo fmt` inside the container before committing.
- Ensure `cargo clippy` passes without warnings.
- Keep the Terminal UI (TUI) clean and performant.

## Legal
By contributing, you agree that your contributions will be licensed under the project's MIT License.
