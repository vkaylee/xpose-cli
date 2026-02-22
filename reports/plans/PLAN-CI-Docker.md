# Implementation Plan: Fix CI with Docker (COMPLETED)

## 📌 User Request (VERBATIM)
> Fix CI, but all things must go in docker to fit with the host and CI

## 🎯 Acceptance Criteria (Derived from User Request)
| ID | Criterion | Verification Method |
|----|-----------|---------------------|
| AC1 | Fix CI to run successfully | Verify GitHub Actions workflow runs to completion |
| AC2 | Use Docker for all CI tasks | Verify `.github/workflows/ci.yml` uses `docker run` for both `lint` and `test` jobs |

## 📋 Context Summary
**Architecture**: The application uses a multi-stage Dockerfile to build a Rust environment (`xpose-builder`).
**Patterns**: GitHub Actions jobs run on isolated VMs. To share an image, it must be saved and loaded as an artifact, or pushed to a registry.
**Constraints**: The user wants *all* things to go in Docker to be consistent with the host and CI. The previous jobs (`lint` and `test`) must successfully run their `cargo` commands inside the `xpose-builder` container.

## Overview
Redesign `.github/workflows/ci.yml` to have a dedicated `build` job that creates the `xpose-builder` image and saves it as an artifact. Then, have parallel `lint` and `test` jobs download the artifact, load the image, and run their respective commands inside the container.

## Prerequisites
- [x] Identify the current CI failure (missing image in `test` job).
- [x] Research GitHub actions Docker sharing patterns.

## Phase 4: IMPLEMENTATION
### Tasks
- [x] Update `.github/workflows/ci.yml`
  - Agent: `tech-lead` (or direct implementation)
  - File(s): `.github/workflows/ci.yml`
  - Acceptance: AC1, AC2
  - Verification: Run workflow locally via `act` or verify YAML syntax and logic.

### Exit Criteria
- [x] `.github/workflows/ci.yml` correctly orchestrates caching and docker container execution.

## Phase 5: VALIDATION
### Tasks
- [ ] Task 5.1: Validate workflow using `act` (if available) or by dry-reading the syntax.
- [ ] Task 5.2: Verify the local equivalent (`./run-tests.sh` or `./run-in-docker.sh`) still works.

### Exit Criteria
- [ ] Changes are syntactically valid and conceptually sound for Github Actions.

## Risks
| Risk | Impact | Mitigation | Rollback |
|------|--------|------------|----------|
| Artifact upload/download is slow | Medium | Use `actions/cache` instead of artifacts, or `docker/build-push-action` with cache. | Revert `ci.yml` |
| Disk space on runner runs out | Low | Wait and see | Revert `ci.yml` |

## Rollback Strategy
`git checkout main -- .github/workflows/ci.yml`

## Implementation Notes
Use `actions/upload-artifact@v4` and `actions/download-artifact@v4` to pass the image tarball between jobs. This ensures the `xpose-builder` image built in the `build` job is exactly the one used in `lint` and `test`. 
