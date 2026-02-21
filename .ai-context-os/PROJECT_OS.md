# 🚀 Project OS: AI Context Orchestrator Kernel

> [!IMPORTANT]
> **Single Source of Truth (SSOT)**
> This document defines the "Operating System" for this repository. All AI Agents (Cursor, Claude, Antigravity) must adhere to these protocols.
> **Version:** 1.0.0

---

## 1. 🧬 Core Philosophy (The "Why")

This project treats **AI Context as Infrastructure**. Just as you wouldn't deploy code without a Dockerfile, you shouldn't deploy AI without a Context OS.

### The 3 Pillars:
1.  **Orchestration over Generation**: We don't just generate code; we orchestrate how it's generated.
2.  **Context Hygiene**: We actively prevent "Context Drift" by enforcing a single source of truth.
3.  **Protocol-First**: If the protocol says "use docker", we use docker. No exceptions.

---

## 2. 🏗️ Architecture Layers (The Fallback Pattern)

This OS operates on a **Fallback Architecture (Inheritance)**. AI agents must prioritize project-specific overrides; if they do not exist, the agents will fall back to the base OS standards.

### The Execution Hierarchy
1. **Priority 1 (Custom):** Project-specific laws, skills, and documentation defined in your local repository (e.g., custom `.cursorrules` or `.local-os/skills/`).
2. **Priority 2 (Standard Built-in):** The `ai-context-os` core standards defined here.

| Layer | Component | Function |
| :--- | :--- | :--- |
| **L0 (Kernel)** | `.ai-context-os/PROJECT_OS.md` | The immutable laws and standard configurations of the orchestrator. |
| **L1 (Adapters)** | `.ai-context-os/CLAUDE.md`, `.ai-context-os/.cursorrules`, `.ai-context-os/GEMINI.md` | Translates L0 laws. Pointer files in project root reference these. |
| **L2 (Skills)** | `.ai-context-os/skills/` directory | Modular capabilities fallback (React, Rust, Python, etc.) |
| **L3 (Docs)** | `.ai-context-os/docs/` directory | Human-readable context and decision logs. |

### 2.1 Distribution Boundaries (The Sandbox)
To maintain project integrity and recursive efficiency, we distinguish between:
- **Shipped Zone:** Files defined in `package.json ["files"]`. These are delivered to the user via `npx`.
- **Internal Zone (The Sandbox):** All other directories (`docs/`, `.git/`, test scripts). These are strict "Development Only" assets and must never be included in the public package. AI agents must respect this boundary when proposing file changes.

---

## 3. 🛡️ Operational Protocols (The "How")

### 3.1 The "Gold Standard" Protocol
Before writing any code, every AI agent must:
1.  **Read L0**: Check `PROJECT_OS.md` for current constraints.
2.  **Check L1**: Verify tool-specific rules (e.g., `docker-helper.sh` usage).
3.  **Audit**: Run `npx ai-context-os audit` (or local `bin/audit.js`) to ensure the current state is compliant before and after execution.
4.  **Execute**: Perform the task strictly within the defined boundaries.
5.  **Autonomous Enforcement**: AI agents are mandated to be proactive. Verification, auditing, and documentation sync must be performed autonomously without waiting for user prompts.

### 3.2 File & Code Standards
- **Modularity**: Files must not exceed **200 lines**. Refactor immediately if they do.
- **Naming**: Use `kebab-case` for all files and directories.
- **Language**: [UNIVERSAL COMPATIBILITY] ALL project-critical documentation (`*.md`), codebase comments, and architectural files MUST be written in **English** to ensure zero-overhead AI interpretation and global scale compatibility. **[CONVERSATIONAL PERSONA]**: While logic remains in English, AI Agents SHOULD respond to the user in their initial prompt language for collaborative fluidity.
- **Type Safety**: Universal Rule. All functions, methods, and complex variables MUST have explicit input and output signatures. For JS/Python, use strict docstrings JSDoc `// @ts-check` or PEP 484 type hints. Zero-defect static linting is mandatory.
- **AI Interface**: Programmatic Rule. All CLI tools MUST provide a `--json` flag for machine-to-machine orchestration and a `--compress` flag for token-optimized context recovery.
- **ULTP Protocol**: Efficiency Rule. The Ultra-Low Token Protocol (`--ultra`) is the mandatory standard for high-frequency agent signaling. If ULTP output is invalid, agents MUST fallback to `--json`. **[PERSISTENCE]**: The latest ULTP state MUST be cached to `.ai-context-os/.ultp_state` for instant O(1) retrieval.
- **Ready Protocol**: Atomic Rule. The `--ready` flag combines ULTP (State) and Compressed MD (Logic) into a single payload for instant agent bootstrapping.
- **Vetting Protocol**: Purity Rule. All new L2 Skills MUST be vetted against `skills/knowledge-vetting.md` and tagged with `Vetted: Yes` before commitment.

### 3.3 Agent Bootstrapping & Memory
- **Bootstrapping**: [MANDATORY] Upon start, an Agent MUST silently read the `.ai-context-os/.ultp_state` file (if it exists) via file-system tools to synchronize context. DO NOT execute shell commands like `npx ai-context-os scout` for initialization unless specifically requested, to avoid interrupting the user with execution prompts.
- **Shared Memory**: [DYNAMIC CONTEXT] Significant session actions (deployments, major refactors, test results) MUST be logged to `.ai-context-os/memory/session.md` to ensure context persistence between agents. Agents MUST read this file upon start.

### 3.4 Recursive Evolution Protocol (L4)
The AI Context OS is a living system. AI Agents are granted "Legislative Rights" to evolve the OS when they encounter systemic friction.
- **Legislative Mandate**: If an agent observes a recurring failure (>2 times across sessions, eg. a specific linting error or architectural misunderstanding), the agent MUST propose a permanent rule amendment to `PROJECT_OS.md`.
- **Skill Discovery**: If an agent is tasked with a technology or framework not documented in the `.ai-context-os/skills/` directory, it MUST halt execution, research the official documentation, and author a new standard skill file according to the `knowledge-vetting.md` purity rules. 
- **Execution**: The `recursive-evolution.md` skill acts as the blueprint for self-healing and capability expansion.

### 3.5 Testing & Documentation
- **Testing**: No code is committed without passing tests. If skipping new tests, a clear justification must be provided. Every change requires a full execution of the project's regression suite to ensure zero regressions. **Diamond Requirement**: Combined test coverage must exceed **90%**.
- **Documentation**: All architectural and logic changes must be documented immediately. No documentation drift is permitted.

### 3.6 Engineering Benchmarks
- **🥇 Gold Standard**: Mandatory for all production features. Includes audit compliance, English docs, and modularity.
- **💎 Diamond Standard**: The ultimate goal. Includes predictive self-healing, architectural purity, and zero technical debt. Agents are encouraged to self-elect to Diamond mode for complex refactors.
- **Containerization**: ALL meaningful execution happens inside Docker.
- **Language**: [Define your primary languages here, e.g., TypeScript, Rust]
- **Style**: [Define your style guide, e.g., Prettier, Rustfmt]

---

## 4. 🤖 AI Agent Responsibilities

| Agent | Responsibility | Primary Instruction File |
| :--- | :--- | :--- |
| **Orchestrator** | High-level planning, evolving the OS (L4 Legislative Rights), and aligning cross-agent logic. | `PROJECT_OS.md` |
| **Implementer** | Writing code, tests, and documentation. | `CLAUDE.md` / `.cursorrules` / `GEMINI.md` |
| **Reviewer** | Validating against L0 standards. | `docs/review-checklist.md` |

---

## 5. 🔄 Evolution & Overrides

This OS is designed to evolve.
- **To Change a Rule**: Submit a PR to `PROJECT_OS.md`.
- **To Override (Temporary)**: Explicitly state "Override Protocol X due to Y" in your prompt.

> [!TIP]
> Treat this file like your project's Constitution. Change it with care, but don't be afraid to amend it as the project grows.
