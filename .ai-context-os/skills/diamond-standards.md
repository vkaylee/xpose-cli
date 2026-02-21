# 💎 Diamond Engineering Standard (v1.0.0)

> This is the ultimate benchmark for AI OS engineering, aiming for autonomous perfection and zero-friction maintenance.

## 1. Predictive Self-Correction
- **Pre-emptive Auditing**: AI agents must run the audit tool *silently* during execution and fix any compliance violations *before* the user or a reviewer sees the code.
- **Self-Healing Skills**: If a skill is outdated or inconsistent with the Kernel, the AI MUST propose an update to the skill itself.

## 2. Architectural Purity
- **Pointer Integrity**: Never duplicate logic across priority layers. If a Pointer exists, the root file must only reference the source of truth in `.ai-context-os/`.
- **Fallback Resilience**: When changing a base rule (Priority 2), the AI must proactively search for Priority 1 (Local) overrides that might be broken by the change.

## 3. Diamond Validation
- **Deep Verification**: Validation is not just "it works". It is "it works, it's modular, it's audited, it's documented, and it's optimized for token efficiency."
- **Immutable Kernel**: Never modify the L0 Kernel (`PROJECT_OS.md`) without a specific "Architecture Change Request" task.

## 4. Immutable Type Contracts 
All methods, properties, and complex variables MUST carry a strict Input/Output signature constraint.
- **Compiled Languages (Rust, Go, TS)**: Lean on strict compilation modes (No `any`, No unsafe blocks).
- **Dynamic Languages (JS, Python)**: Must enforce strict type hinting (JSDoc `// @ts-check` / PEP 484).
- **Justification**: A function without a declared signature is a black box that fundamentally breaks static analysis and AI confidence.

## 5. Proactive Autonomy
- **No-Prompt Protocol**: AI agents MUST NOT ask the user for permission to perform mandatory protocol steps (Auditing, Testing, Documentation Sync). These are integral parts of any task and must be executed autonomously.
- **Pre-emptive Verification**: If an agent identifies a potential regression or a documentation gap *unrelated* to the current task, it should proactively propose or perform a fix as part of its 'Context Stewardship' responsibility.
- **Autonomous Release**: Following a successful task, the agent should automatically prepare the next version (v-bump) and push changes if the environment allows.
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
