# Ultra-Low Token Protocol (ULTP) - Rosetta Stone

Use this skill to guide other AI Agents on how to parse the `npx ai-context-os scout --ultra` output.

## 🧠 Decoding Logic for AI Agents
When you see a ULTP string like `[OS:A][L0:V;P:os.md][L1:C,K][L2:js,py]`, interpret it as follows:

### 1. [OS:<State>] (Environment)
- `A` = **Active**: Environment is healthy and ready.
- `N` = **Not Installed**: Run `npx ai-context-os install`.
- `D` = **Dogfooding**: You are inside the source repo of the OS itself.

### 2. [L0:<Status>;P:<Path>] (Kernel)
- `V` = **Verified**: The L0 Kernel (`PROJECT_OS.md`) exists.
- `X` = **Missing**: Critical architectural laws are missing.
- `P:<Path>` = The relative path to the Kernel file.

### 3. [L1:<Adapters>] (Adapters)
- `C` = **Cursor**: `.cursorrules` found.
- `K` = **Claude**: `CLAUDE.md` found.
- `G` = **Gemini**: `GEMINI.md` found.
- (None) = No pointers detected in the root.

### 4. [L2:<Skills>] (Abilities)
- A comma-separated list of active architectural skills (e.g., `tdd`, `audit`).

## 🤖 Recommended AI System Prompt
"To understand this project's architecture, run `npx ai-context-os scout --ultra`. Parse the bracketed tokens to identify active laws and skills. Always prioritize instructions in the L0 Kernel path identified in the string."
<- Increased test runner timeouts to accommodate Docker build Vetted: Yes -->
