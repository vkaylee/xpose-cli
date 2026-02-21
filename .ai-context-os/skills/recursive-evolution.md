# Recursive Evolution Protocol (L4)

> [!IMPORTANT]
> This skill defines the "Legislative" capabilities of AI Agents operating within the AI Context OS.
> **Law:** AI Agents are not just executors; they are caretakers of the OS. They must self-heal and evolve the system.

## 🧬 1. The Core Mandate

The OS must adapt to the project's growing complexity without constant human intervention.
You currently have the right to modify the "memory" of the system and its "rules".

If you encounter:
1. **Recurring Failures**: An issue that caused a failure more than once, meaning the current rules failed to prevent it.
2. **Knowledge Gaps**: A task involving a technology/pattern not covered by existing `skills/*.md`.

**Action Required**: You MUST initiate an Evolution Cycle.

## 🔄 2. The Evolution Cycle

### Phase 1: Detection & Root Cause Analysis
- Ask: *Why did the current rules (`PROJECT_OS.md` or `skills/`) fail to prevent this situation?*
- Identify if the gap is in **Policy** (a rule is missing/wrong) or **Knowledge** (a skill is missing).

### Phase 2: Legislation (For Rule Gaps)
- If a rule is missing or ambiguous, draft an amendment.
- Navigate to `.ai-context-os/PROJECT_OS.md` and insert the new rule under the appropriate section.
- The rule must be clear, actionable, and testable by `audit.js` if possible.

### Phase 3: Discovery (For Knowledge Gaps)
- Stop the current coding task.
- Perform a web search or codebase search to understand the missing technology or pattern.
- Synthesize the knowledge into a standardized Markdown file.
- Format the file according to `knowledge-vetting.md` (Purity, Testing, Fidelity).
- Tag the file with `<!-- Vetted: [Date] by [Agent] -->` at the end of the file.
- Save it to `.ai-context-os/skills/<new-skill-name>.md`.

### Phase 4: Shared Memory Sync
- Once you have updated the OS or added a Skill, you MUST log the evolution event in `.ai-context-os/memory/session.md`.
- Example format:
  `- [EVOLUTION] Added skill 'svelte-5.md' to address knowledge gap. Vetted: Yes.`
  `- [LEGISLATION] Amended PROJECT_OS.md to enforce strict null checks.`

## 🛡️ 3. Safety Guidelines
- **Do not break L0 Laws**: New skills or rules must comply with core tenets (e.g., Modularity: max 200 lines, Naming: kebab-case, Language: English docs).
- **Do not guess**: If exploring a new technology, use legitimate documentation sources before codifying it into a skill.
<!-- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
