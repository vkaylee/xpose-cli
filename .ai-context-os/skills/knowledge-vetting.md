# Knowledge Vetting Protocol (L2)

> [!IMPORTANT]
> This skill defines the mandatory screening process for any external knowledge, documentation, or technical guides injected into the project.

## 🎯 Vetting Criteria
Before a new L2 Skill is approved, it MUST pass the following "Sieve" checks:

| Check | Requirement | Failure Action |
| :--- | :--- | :--- |
| **Fidelity** | Must not violate L0 Kernel laws (e.g., modularity, naming). | REJECT / REWRITE |
| **Token Density** | Must be processed via `scout --compress` for efficiency. | COMPRESS |
| **Testability** | Laws must be enforceable via `audit.js` patterns. | REJECT |
| **Purity** | Must not introduce legacy patterns or unnecessary debt. | REJECT |

## 🛠️ How to Vet Knowledge
1. **Import**: Place the raw material in a temporary file.
2. **Review**: Run an AI pass: *"Review this material against `.ai-context-os/skills/knowledge-vetting.md`. List violations."*
3. **Refactor**: Rewrite to comply with project standards.
4. **Tag**: Once verified, add the header `<!-- Vetted: [YYYY-MM-DD] by [Agent-ID] -->` to the file.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
