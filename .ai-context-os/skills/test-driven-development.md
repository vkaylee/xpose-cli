# 🧪 Test-Driven Development (v1.0.0)

> This skill mandates a "Test-First" culture to ensure every change is verified and stable.

## 1. The Red-Green-Refactor Cycle
1. **RED**: Write a failing test first. It must capture the core requirement or bug report.
2. **GREEN**: Write the minimal amount of code needed to pass the test.
3. **REFACTOR**: Clean up the code while keeping the test Green.

## 2. AI Execution Protocol
- When asked to "Add feature X", the first tool call should ideally be creating or updating a test file.
- AI agents must run tests locally before proposing any code changes to the user.
- **Justification for Skip**: If an agent determines that new tests are not required for a change, it MUST explicitly state the reason (e.g., "Pure documentation change", "Refactor covered by existing regression suite", "Trivial string constant update").


## 3. High-Quality Tests
- **Isolation**: Each test should verify a single behavior.
- **Readability**: Test names should clearly state the intention (e.g., `should-return-error-when-id-is-missing`).
- **NPS (Null, Path, State)**: Always test for null inputs, happy paths, and edge state transitions.

## 4. Regression Assurance
- **Full Suite Mandate**: After passing new tests, AI agents MUST rerun the *entire* existing test suite (e.g., `npm test`) to ensure no side effects were introduced.
- **Verification Proof**: Completion of a task requires evidence that the full regression suite passed in the current state.

## 5. Mutation Thinking
- **Verifiability**: A test is only as good as its ability to fail. If you change a `>=` to a `>` in your logic and the tests still pass, your tests are weak.
- **Protocol**: Periodically "break" your own code (Manual Mutation) to verify your tests are actually catching logic errors.

## 6. Exploratory Fuzzing
- **Chaos Input**: Unit tests cover known unknowns. Fuzzing covers unknown unknowns.
- **Protocol**: Inject "junk" parameters (random strings, null-like bytes, huge buffers) into the CLI to test resilience (Error Hardening). A Diamond-grade CLI must NEVER crash (Unhandled Exception); it must only report errors and exit safely.

## 6. Coverage Guardrails
- **Threshold**: Diamond-grade releases require **> 90%** logic coverage.
- **Usage**: Run `npm run test:unit` to view the coverage report. AI agents must aim for 100% "Instruction" and "Branch" coverage whenever possible.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
