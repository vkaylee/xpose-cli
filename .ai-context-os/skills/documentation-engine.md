# 📖 Documentation Engine (v1.0.0)

> This skill ensures that documentation is never an afterthought and stays perfectly synced with implementation.

## 1. Documentation-as-Code
- **Sync Point (Atomic Documentation)**: Every change to logic, structure, or protocol MUST be recorded in documentation *simultaneously* with the code change. No commit is complete without updated docs.
- **Self-Documenting Code**: Prefer clear variable naming and small functions over heavy inline comments.

## 2. Machine-Readable Context
- Maintain indices in `docs/` for AI agents to perform rapid lookup.
- Ensure all technical decisions are logged in the `docs/adr/` (Architecture Decision Records) directory.

## 3. Language Governance
- Strictly enforce the **English-only** rule in all `.md` files.
- Use clear, concise language. Avoid jargon where simple words suffice.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
