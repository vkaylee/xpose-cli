## Research Report: Docker caching and parallel CI jobs in GitHub Actions

### Executive Summary
The currently implemented CI workflow (`.github/workflows/ci.yml`) runs the `lint` and `test` jobs on separate VMs. The `lint` job builds the Docker image locally, but the `test` job cannot access it. To share the built image across jobs, we need to utilize a central build step with caching or registry sharing, or extract the Docker build into a standalone job that caches the image so subsequent parallel jobs can load it.

### Findings
#### Finding 1: GitHub Actions Job Isolation
By default, GitHub Actions jobs run on separate, isolated runner environments. Artifacts and local Docker builds in one job do not persist to another job unless explicitly saved and loaded (e.g., using `actions/upload-artifact`) or pushed to a container registry.
- Confidence: High

#### Finding 2: Docker Layer Caching in GitHub Actions
Using `docker/setup-buildx-action` along with `docker/build-push-action` is the standard method for building and caching Docker images in GitHub Actions. We can use `actions/cache` or GitHub's native `type=gha` cache exporter.
To pass the image across jobs, we can:
1. Push it to a registry (like GitHub Container Registry - GHCR) in a `build` job.
2. Use `actions/upload-artifact` to pass the exported `.tar` from `docker save`, but this is often slow for large images.
3. Build the image in each job but rely on aggressive layer caching (`type=gha`) so the subsequent builds are nearly instantaneous.
- Confidence: High

### Recommendations
1. **Recommended**: Create a dedicated `build-docker` job that uses `docker/build-push-action` with `type=gha` caching to build the image and push it to a local artifact or a registry, OR simply duplicate the build step in both jobs but enable `type=gha` caching so the second build is virtually free. Given the user's previous context ("redesigning the GitHub Actions workflow to have a dedicated build job that primes the cache, followed by parallel test jobs that load the cache"), the best approach is to have a `build` job that creates the image, saves it to a tarball, uploads it as an artifact, and then the test/lint jobs download and `docker load` it.
2. **Alternative**: Build the image locally in *each* job but use `buildx` with `type=gha` cache so that it doesn't actually recompile Rust dependencies but just downloads the layers.

### Sources
1. GitHub Actions Documentation - Job Artifacts
2. Docker Buildx GitHub Actions Caching Documentation
