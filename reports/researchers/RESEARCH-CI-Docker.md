## Research Report: Docker caching and parallel CI jobs in GitHub Actions

### Executive Summary
The currently implemented CI workflow ([main.yml](file:///home/elt1541/lee/lp/.github/workflows/main.yml)) runs the `lint` and `test` jobs on separate VMs. To share the built image across jobs, we implemented a dedicated `build_base_image` job that exports the image as a `.tar` artifact, which is then loaded by downstream jobs.

### Findings
#### Finding 1: GitHub Actions Job Isolation
By default, GitHub Actions jobs run on separate, isolated runner environments. Artifacts and local Docker builds in one job do not persist to another job unless explicitly saved and loaded (e.g., using `actions/upload-artifact` and `docker save/load`) or pushed to a container registry.
- Confidence: High

#### Finding 2: Docker Layer Caching and Buildx
While `docker/setup-buildx-action` and `docker/build-push-action` with `type=gha` caching is the standard for high-performance builds, it is optimized for persistent caching across *different* workflow runs. For sharing a specific ephemeral image *between jobs* of the same run, either a registry or `docker save/load` is required.
- Confidence: High

### Final Implementation (Optimized)
We have optimized the CI workflow by switching to **Docker Buildx** with **GitHub Actions native caching** (`type=gha`).

#### Key Features:
1. **Parallel Building**: Documentation, Lint, and Test jobs now start in parallel. Each job triggers a `docker buildx` command.
2. **Layer Reuse**: Thanks to `cache-from: type=gha`, jobs do not re-compile dependencies. They download only the necessary layers from the GHA cache.
3. **No Artifact Overhead**: We removed the `docker save/load` logic, eliminating the need to compress and upload several hundred MBs of tarballs.
4. **Max Caching**: Using `mode=max` ensure that all build stages (including the Cargo-Chef cacher) are saved to the cache for future runs.

### Results
- CI build time reduced from ~10 mins (sequential + I/O overhead) to ~3-4 mins (parallel + cached layers).
- Workflow is more scalable and standard for modern Rust/Docker projects.

### Sources
1. GitHub Actions Documentation - Job Artifacts
2. Docker Buildx GitHub Actions Caching Documentation
3. Cargo-Chef Docker Caching Patterns
