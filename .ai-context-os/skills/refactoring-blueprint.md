# 🏗️ Refactoring Blueprint (v1.0.0)

> This skill empowers AI agents to maintain the **200-line modularity rule** through proven structural patterns.

## 1. Trigger Conditions
- Any file exceeding **180 lines** during a task.
- Nested logic exceeding **3 levels** of indentation.
- Over-sized functions (exceeding 40 lines).

## 2. Extraction Strategies
### A. Functional Extraction
- Identify logical blocks and move them to local helper functions at the end of the file.
- If the file remains over 200 lines, promote helpers to a separate `utils/` or `services/` file.

### B. Component Splitting (Frontend)
- Break large components into sub-components.
- Use the **One Component Per File** principle.

### C. Logic Separation
- Move business logic to dedicated `useCases` or `domain` layers.
- Keep entry points (controllers/routes) lightweight.

## 3. Diamond Constraint
- Refactoring must be done **predictively**. Do not wait for a violation to be caught by the audit tool. If your proposed edit will push a file over 200 lines, refactor *first*.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
