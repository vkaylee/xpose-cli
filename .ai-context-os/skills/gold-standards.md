# 🥇 Gold Engineering Standard (v1.0.0)

> This standard defines high-quality, professional engineering practices required for all AI-Context-OS contributors.

## 1. Structural Integrity
- **Modularity**: Files MUST NOT exceed **200 lines**. If a file hits 180 lines, plan for a split immediately.
- **Naming Convention**: All files and directories MUST follow `kebab-case`.
- **Zero-Dead-Code**: Never leave commented-out code or unused variables in the distribution zone.

## 2. Documentation & Language
- **English-Only**: All comments, documentation, and logic descriptions MUST be in English.
- **Auto-Discovery**: Every directory must contain a `README.md` or a skill definition file that is machine-readable by AI agents.

## 3. Validation Phase
- **Mandatory Audit**: Running `npx ai-context-os-audit` is a hard requirement before any commit. 
- **Verification Proof**: AI agents must provide a summary of the audit results when claiming a task is done.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
