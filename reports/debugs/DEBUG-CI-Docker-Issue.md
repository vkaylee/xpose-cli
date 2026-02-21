# Debug Report: CI Docker Issue

## Bug Characterization
| Attribute   | Value          |
| ----------- | -------------- |
| Description | The CI `test` job fails because the `xpose-builder` docker image is not found. It is only built in the `lint` job, which runs on a different runner VM. |
| Severity    | High |
| Reproduction | Run the GitHub Actions `CI` workflow. The `lint` job succeeds, but the `test` job will fail with standard docker error 'image not found'. |

## Root Cause
**The GitHub Actions workflow naturally isolates jobs into separate runner instances. The `test` job attempts to use a Docker image (`xpose-builder`) that was built locally in the `lint` job, meaning it's unavailable in the `test` job's environment.**

## Fix Hypothesis
We must redesign the CI workflow so that the Docker image is built and available for all relevant tools. Potential approaches:
1. Combine lint and test into a single job so the built image is shared.
2. Create a dedicated docker-build job that pushes to a registry or uses a cache, and the lint/test jobs pull from there. (User previously mentioned optimizing CI workflow by splitting into parallel jobs).
3. Build the Docker image in both jobs (inefficient but simple).

## Verification
- [ ] Review GitHub Actions documentation on sharing Docker images or caching.
- [ ] Implement the fix in `.github/workflows/ci.yml`.
- [ ] Verify using ACT or confirming logic.
