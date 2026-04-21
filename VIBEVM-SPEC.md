# vibevm: A Software Project Manager for Spec-Driven Development with AI Agents

**Document version:** 1.0
**Status:** Implementation specification
**Audience:** Claude Code (or equivalent autonomous coding agent) implementing the project from scratch
**Owner:** Oleg Chirukhin
**License of this document:** UPL 1.0
**License of the produced software:** proprietary EULA (source-available); for third-party dependency choices, assume permissive (MIT/Apache-2.0)

---

## How to read this document

This is a complete, self-contained specification. It assumes you have not read any prior conversation about this project. Every decision is re-justified here. Every term is defined here. If something is unclear, the answer is in this document; do not invent.

You will see references to materials in `refs/`. These are study materials, not specifications:
- `refs/book/` contains the user's book chapters on AI-native development. These define the *philosophy* of the project. Read them before writing any code.
- `refs/src/maven/` is Apache Maven's source tree. Study it for ideas about lifecycle management, plugin systems, and dependency resolution. Do not copy code; we are not building Maven and we never use Maven's terminology in our own output. If there's no sources of Maven in this directory, create the directory and clone the sources from the Internets.
- `refs/src/bazel/` is Bazel's source tree. Study it for the DAG execution model and typed task graphs. Do not copy code; our DAG model is similar but not identical. If there's no sources of Bazel in this directory, create the directory and clone the sources from the Internets.
- You may `git clone` other public projects into `refs/src/` if you need to study them. Recommended candidates: `tessl` (Tessl framework), `github/spec-kit` (GitHub's Spec Kit), `astral-sh/uv` (a modern Python package manager — clean reference for fetch/resolve), `cargo` (the Rust package manager — clean reference for manifest format and lockfile design). Do not assume these projects are correct; study them as data.

This document uses the term **"the Reader"** to refer to you, Claude Code. It uses **"the User"** to refer to Oleg or whoever runs `vibe` after it ships.

This document is structured to be navigable:
- Sections 1–3 establish philosophy, motivation, and prior art. Read these first; everything later builds on them.
- Sections 4–8 specify the architecture. These define the system precisely.
- Sections 9–10 specify v1 scope and staging. These tell you what to build first.
- Sections 11–13 specify implementation details: file layouts, manifest formats, runner internals.
- Section 14 specifies how the Reader should approach building this — the development methodology.
- Section 15 is a glossary. Use it to disambiguate any term.
- Section 16 is a checklist for verification before declaring v1 complete.

---

## Section 1. Project identity

### 1.1 Name

The project is named **vibevm**. The CLI binary is named **`vibe`**. The user has trademark and domain rights for both. Do not propose alternative names.

### 1.2 Tagline

*The disciplined runtime for spec-driven vibecoding.*

This is the project's positioning. Internalize it: **vibevm is a tool that makes vibecoding work in practice by removing boilerplate decisions, not a tool that opposes vibecoding.** The user's stated philosophy is that vibecoding is a legitimate and joyful mode of building software, that humans don't and shouldn't control everything, and that good tools remove boring decisions so vibecoders can focus on the interesting parts. Design choices should serve this philosophy. When in doubt: choose convention over configuration, choose default over choice, choose "it just works" over "you can configure everything."

### 1.3 What it is

vibevm is a command-line software project manager for AI-assisted development. It lets developers install reusable building blocks (process disciplines, feature descriptions, technology stacks) into a project, and then have an AI agent compile those blocks into working code under the discipline of Spec-Driven Development.

Concretely, a developer can:
1. Initialize a vibevm project (`vibe init`).
2. Install a stack (`vibe install stack:rust-cli`).
3. Install one or more features (`vibe install feat:welcome-page`, `vibe install feat:auth-email`).
4. Install process disciplines (`vibe install flow:wal`).
5. Generate working code from the installed blocks (`vibe build feat:welcome-page`).
6. Modify code or specs and have the system reconcile drift (`vibe sync`).

The same `feat:welcome-page` produces different code depending on which stack is active. This is the core value proposition: features are abstract descriptions of intent; stacks are the concrete mappings into a particular technology context; vibevm orchestrates the assembly.

### 1.4 What it is not

vibevm is not:
- A new IDE.
- A new agent product (it uses existing ones via API).
- A hosted service. It runs entirely locally.
- A code generator with rigid templates. Generation is performed by an LLM with discipline applied through process flows.
- A replacement for npm, cargo, or other language-level package managers. It manages *specs and process discipline*, not language runtime dependencies. A vibevm project will typically also have a `package.json` or `Cargo.toml` for its language ecosystem.

---

## Section 2. Philosophical foundation

The user has written a book on AI-native development. The first three chapters are in `refs/book/`. Read them in full before writing any code. This section summarizes the load-bearing principles, but the chapters contain the reasoning and examples that explain *why* these principles hold.

### 2.1 The two-process model

A human developer and an AI agent are two cooperating processes with complementary architectures, not a master-and-tool pairing. The human has persistent memory, semantic understanding, intuition, and decision-making under uncertainty. The AI has high throughput, mechanical consistency within a session, broad shallow knowledge, and tirelessness within a context window. Productive work uses both for what each is good at.

Implications for vibevm:
- The system never assumes the AI is autonomous or always right.
- The system never assumes the human will catch every issue manually.
- Mechanical work goes to the AI; semantic decisions go to the human; the system makes the boundary explicit.

### 2.2 Files as IPC

In the human-AI development system, files are not documentation. They are the inter-process communication channel between the two processes. The AI cannot ask the human in real time; the human cannot remember everything between sessions. Files are the only persistent shared memory.

Four requirements for this IPC, all of which vibevm must support:
1. **Addressability.** Every spec section must be precisely referenceable via a `spec://module/document#section.subsection` URI.
2. **Atomicity.** Each commit/change should represent one logical unit; mixed changes break verification.
3. **Conflict protocol.** When two writers (human and AI) disagree, there must be an explicit resolution rule. The hierarchy is: human > spec > tests > code.
4. **Visibility of changes.** Both sides must know when shared state has changed. Git diff, REVIEW markers, and changelog sections serve this role.

vibevm enforces the first three structurally. The fourth is partially enforced by tooling, partially by user discipline.

### 2.3 Memory architecture

There are four levels of memory in the system:
1. **Head** — persistent for the human, invisible to the AI, the canonical source for decisions before they are written.
2. **WAL (Write-Ahead Log)** — volatile checkpoint, rewritten each session, the bridge between sessions for continuation.
3. **Specifications** — stable decisions and intents, addressable via `spec://...`, the medium-term memory shared by both processes.
4. **Code** — the artifact produced from specifications, the answer to "how" not "why."

Information flows top-down: head → WAL → spec → code. When information flows bottom-up (code changes ahead of spec), it must be reconciled via an explicit sync protocol.

The WAL is a *checkpoint*, not a log. It is rewritten, not appended. It describes the current state, not the history. History lives in git and in milestone commits.

### 2.4 The constraint that defines everything

**The AI has no memory between sessions.** Every session starts blank. WAL and specifications are the only artifacts that survive. This is the single most important fact in the system. Every design decision in vibevm should be evaluated against the question: *does this work when the AI's session restarts every morning?*

If a design depends on the AI "remembering" something across sessions, it is wrong. The AI must be able to reconstruct everything it needs from files on disk.

### 2.5 The vibecoding affirmation

The book's framing positions structured AI-assisted development against undisciplined "vibe coding." vibevm sits in a different place: it is *vibe coding made viable*. The user is not embarrassed to be vibe-coding; they want to vibe-code productively. vibevm removes the boilerplate decisions that make vibe coding fall apart at scale, while preserving the speed and joy that make vibe coding worth doing in the first place.

When choosing between two design options, prefer the one that lets a vibe coder ship a prototype faster. When the choice is between rigor and speed, choose rigor *only* when the lack of rigor will silently produce wrong results. When wrong results would be obvious, choose speed.

---

## Section 3. Prior art and positioning

### 3.1 Spec-Driven Development (SDD)

The current state of SDD as of early 2026 includes:
- **Tessl** (https://tessl.io) — a commercial platform with a Spec Registry (repository of specs for popular OSS libraries) and a Tessl Framework (closed-beta tool for spec-driven development inside agents).
- **GitHub Spec Kit** (https://github.com/github/spec-kit) — an open-source toolkit (MIT licensed) that scaffolds projects with `.specify/` directories and slash commands for AI agents. Workflow: Constitution → Specify → Plan → Tasks → Implement.
- **AWS Kiro** — an agentic IDE with spec-driven workflows.

Read what each of these does. Note their gaps.

### 3.2 What vibevm does that prior art does not

1. **Three-kinds taxonomy.** Tessl's registry distributes library-usage specs. Spec Kit produces in-project markdown with no notion of installable units. None has a clean separation of "process discipline" vs. "abstract feature" vs. "concrete stack." vibevm makes this taxonomy first-class.

2. **Stack abstraction.** A vibevm `feat` is context-free; a vibevm `stack` provides the concrete mappings. The same feat compiles to different code for different stacks. Tessl and Spec Kit have no equivalent — their specs are tightly coupled to a single implementation context.

3. **Cross-agent compatibility by design.** vibevm uses a single `spec/boot/` directory referenced by a one-line redirect from `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`. Every agent reads the same boot sequence. Other tools commit to specific agent configurations.

4. **CLI-first, agent-agnostic execution.** vibevm uses the user's API key to invoke whichever LLM is configured (Anthropic, OpenAI, OpenRouter, Ollama). It does not require any specific agent product to be installed. It can be run from a bare terminal in CI.

5. **Separation of deterministic and probabilistic work.** vibevm's CLI does deterministic work (resolution, fetching, file management, validation) without LLM calls. The LLM is invoked only for steps that genuinely require reasoning (build, sync, review). This is cheaper, faster, more debuggable, and fits the book's philosophy of cognitive load distribution.

### 3.3 What vibevm explicitly avoids

- We never use Maven's terminology, even though we have studied it. No "lifecycle," "phase," "goal," or "plugin" in user-facing or internal code (except where context demands a known term — see Section 4 for what to use instead).
- We never use Bazel's terminology directly, but we adopt its DAG execution model.
- We do not build a hosted registry in v1. The registry is a public git repository.
- We do not build a censoring system in v1. We assume one will exist later (see Section 8.5).

### 3.4 References to study

Before designing anything, the Reader should read or skim:
- All chapters in `refs/book/` (mandatory, full read).
- The Maven Lifecycle documentation (mandatory, conceptual understanding only — *do not adopt vocabulary*).
- The Bazel BUILD/Starlark model (mandatory, conceptual understanding for the DAG model).
- Tessl's Spec Registry conventions (recommended, study what specs look like as installable artifacts).
- GitHub Spec Kit's repository structure (recommended, study how it scaffolds projects).
- `cargo`'s manifest format (recommended, model for our `vibe.toml`).
- `uv`'s implementation (recommended, model for fast resolve/fetch).

If `refs/src/` does not contain Tessl or Spec Kit, clone them:
```
git clone https://github.com/github/spec-kit refs/src/spec-kit
git clone https://github.com/tessl-io/tessl-mcp refs/src/tessl-mcp  # if accessible
```

---

## Section 4. Core terminology

Use these terms and only these terms in code, in documentation, in error messages, in commit messages. Consistency matters more than perfection.

### 4.1 The four installable kinds

vibevm packages come in four kinds. A user installs them with `vibe install <kind>:<name>`.

**`flow`** — A process discipline. Modifies how the human-AI development process works in this project. Examples: WAL discipline, sync-from-code reconciliation, conventional-commits enforcement, REVIEW marker conventions. A flow contributes content to `spec/flows/<name>/` and may register one or more snippets in `spec/boot/`.

**`feat`** — A feature description. Describes *what* to build, abstractly, without committing to a stack. Examples: welcome page, email-password authentication, document search, payment checkout. A feat contributes content to `spec/feats/<name>/`. A feat is consumed by the `build` workflow, paired with an active stack.

**`stack`** — A technology context. Provides the concrete mappings from abstract feat capabilities to a specific tech stack. Examples: rust-cli, electron-local, nextjs-postgres, tauri-rust. A stack contributes content to `spec/stacks/<name>/`. A project may have multiple stacks installed but typically one is active per build.

**`tool`** — A reusable script, prompt, or utility that nodes in the build graph may invoke. Examples: a code formatter wrapper, a test runner adapter, a structured-output renderer. Tools are not workflows; they are *capabilities used by workflows*. (Tools are reserved as a kind for future use; v1 does not require them. Document the slot, do not implement.)

### 4.2 The directory layout

A vibevm project has this structure:

```
project-root/
├── vibe.toml                       # Project manifest
├── vibe.lock                       # Resolved versions
├── CLAUDE.md                       # One-line redirect to spec/boot/
├── AGENTS.md                       # Same, identical content
├── GEMINI.md                       # Same, identical content
├── spec/                           # All spec content lives here
│   ├── boot/                       # Numbered boot files (rc.d-style)
│   │   ├── 00-core.md              # Owned by user, project foundations
│   │   ├── 10-flow-wal.md          # From flow:wal
│   │   ├── 20-flow-sync-from-code.md
│   │   ├── 50-stack-rust-cli.md    # Stack contributes boot snippet
│   │   └── 90-user.md              # Owned by user, never overwritten
│   ├── flows/                      # Per-flow content
│   │   └── wal/
│   │       └── ...
│   ├── feats/                      # Per-feat content
│   │   └── welcome-page/
│   │       ├── spec.md             # Abstract description of the feat
│   │       ├── capabilities.toml   # What the feat needs from a stack
│   │       └── acceptance.md       # Stack-agnostic acceptance scenarios
│   ├── stacks/                     # Per-stack content
│   │   └── rust-cli/
│   │       ├── stack.md            # Description of this stack
│   │       └── mappings.md         # How abstract capabilities map to this stack
│   ├── common/                     # Project-wide specs (PROP-000 etc)
│   ├── modules/                    # Project module specs (PROP-001 etc)
│   └── WAL.md                      # Project state checkpoint
├── src/                            # Generated and user-owned source code
├── tests/                          # Generated and user-owned tests
└── .vibe/                          # Cache, internal state — gitignored
    ├── cache/                      # Downloaded packages cache
    └── effective.json              # Last computed effective spec (debugging)
```

The `spec/` directory is *the* spec directory. Always. Do not allow this to be configurable in v1. Every other location can be conventional; this one is fixed.

### 4.3 Workflows

A **workflow** is a named sequence of work that the user invokes via the CLI. Each workflow is defined as a subgraph of the project's task graph (see Section 5). v1 ships with these workflows:

- **`init`** — Set up a new project.
- **`install`** — Resolve, fetch, review, plan, confirm, apply for one or more packages.
- **`uninstall`** — Reverse install for a named package.
- **`update`** — Refetch and apply changes for an installed package.
- **`list`** — Show installed packages.
- **`check`** — Validate spec consistency (lint).
- **`show`** — Display computed state (effective spec, project graph, etc.).
- **`build`** — Generate code from a feat × stack combination. (LLM-invoking; v1.5 scope.)
- **`sync`** — Reconcile code-spec drift via LLM. (v1.5 scope.)

Workflows are user-facing. They are what people type. The internal implementation is a graph; that is invisible to the user except via `vibe show graph`.

### 4.4 The task graph

Internally, a vibevm project has a **task graph**: a directed acyclic graph of typed nodes. The graph is constructed at runtime from the project's configuration and installed packages. Each workflow corresponds to a query on this graph (a target node and its transitive dependencies).

**Nodes.** Each node has:
- A unique name (e.g., `load:project-toml`, `build:compile`).
- A kind (see node kinds below).
- A set of typed inputs.
- A set of typed outputs.
- An implementation appropriate to its kind.

**Edges.** An edge from node A to node B means B's input consumes A's output. Edges are typed: A's output type must match B's input type.

**Barriers.** Some nodes are pure sync points — no work, just edges. Used for coordination when many nodes must complete before one set of downstream nodes runs. v1 defines these barrier names:
- `barrier:context-loaded` — all input loading has completed.
- `barrier:plan-ready` — planning is done, awaiting confirmation.
- `barrier:approval-given` — user has approved the plan, mutating actions may begin.
- `barrier:complete` — all work for the workflow has finished.

**Node kinds.**
- `load` — read a file or compute a value from project state. Deterministic, cacheable.
- `llm` — invoke an LLM via the configured provider. Non-deterministic, opt-in cacheable.
- `prompt` — interactive: ask the user a question, get a response. Non-cacheable.
- `write` — write a file. Mutating, not cacheable.
- `shell` — run a subprocess. Mutating or read-only depending on the command.
- `barrier` — sync point with no work.
- `report` — emit structured output describing what happened.

This model is more expressive than a sequential lifecycle because it allows fan-out and fan-in naturally: three nodes that all consume the same upstream output can run in parallel.

### 4.5 The four memory levels (recap from book)

Use these terms consistently:
- **Head** — the human's memory; not in scope for vibevm to manage, but vibevm respects that it exists.
- **WAL** — the project state checkpoint at `spec/WAL.md`. Maintained per project.
- **Spec** — the corpus of files under `spec/` that are not WAL or boot. Stable, addressable, versioned in git.
- **Code** — files outside `spec/`, typically in `src/` and `tests/`. Generated or user-edited. Tracked via `Implements: spec://...` markers.

### 4.6 Other key terms

- **Effective spec** — the materialized state of all installed packages plus project configuration plus current WAL, computed at the start of each workflow. The effective spec is what the LLM "sees" during build. `vibe show effective` prints it.
- **Active stack** — the stack currently selected for build operations. A project may have multiple stacks installed; one is active by default; per-command override via `--stack`.
- **Boot snippet** — a small markdown file installed into `spec/boot/<NN>-<package-name>.md` that tells the AI what to do during session boot regarding this package.
- **Declared writes** — the explicit list of files a package will create, modify, or append to during install. Recorded in the package manifest. The basis for plan/confirm/apply.
- **REVIEW marker** — an inline marker (`<!-- REVIEW: ... -->`) in spec or code that indicates an unresolved decision the human should look at.
- **`spec://` URI** — the addressing scheme for spec content. Format: `spec://<module>/<document>#<section>.<subsection>`. Used in code comments (`// Implements: spec://...`), in cross-references between specs, and in error messages.

---

## Section 5. The task graph in detail

This section specifies the internal model. Most users never see it; plugin authors and `vibe show graph` users do.

### 5.1 The graph builder

When the user invokes any workflow, the CLI:
1. Reads `vibe.toml` and `vibe.lock`.
2. Reads each installed package's manifest from `.vibe/cache/<package>/<version>/vibe-package.toml`.
3. Reads any user-overridden manifests in the project itself.
4. Constructs a graph by:
   a. Instantiating built-in nodes (load:*, build:plan, etc.).
   b. Instantiating each plugin's contributed nodes.
   c. Resolving edges by matching declared inputs to declared outputs.
   d. Inserting nodes against named barriers per their `contributes` declarations.
   e. Validating the result is acyclic and type-correct.
5. Returns a frozen graph object.

This graph is constructed in memory; it is not persisted. It can be printed via `vibe show graph` for debugging.

### 5.2 The graph runner

Given a target node, the runner:
1. Computes the transitive closure of dependencies.
2. Topologically sorts nodes; partitions into levels of parallelizable nodes.
3. For each level, executes nodes (in parallel where possible).
4. For each node:
   - Look up cached output if the node is cacheable and a cache key matches.
   - Otherwise, execute the node's behavior.
   - Validate output type matches declaration.
   - Cache output if the node is cacheable.
   - Pass output to downstream nodes via the typed value store.
5. On any node failure, halt the workflow, emit structured error, exit non-zero.

The runner is sequential in v1 (no parallelism). Topological sort is computed but levels are executed serially. This simplifies error handling and matches the reality that most workflows have few parallelizable nodes. v2 may add parallelism.

### 5.3 The typed value system

Edges carry typed values. v1 defines this minimal type set:

| Type name | Description |
|---|---|
| `ProjectConfig` | Parsed `vibe.toml` |
| `Lockfile` | Parsed `vibe.lock` |
| `PackageRef` | A reference to an installable package (`flow:wal@0.3.0`) |
| `PackageContents` | Fetched package files in a temp directory |
| `WriteSpec` | A declared write: target path, source path, mode (create/append/replace) |
| `WritePlan` | List of `WriteSpec` with conflict resolution applied |
| `Approval` | Boolean + optional comment from user confirmation |
| `EffectiveSpec` | The materialized merged spec for current project state |
| `WAL` | Parsed contents of `spec/WAL.md` |
| `WALVerdict` | Output of WAL freshness check: { fresh: bool, age: duration, issues: list } |
| `StackSpec` | The active stack's spec content |
| `FeatSpec` | A specific feat's spec content |
| `BuildPlan` | Description of what the build node will produce |
| `CodeFiles` | Map of path → content for generated files |
| `CommandResult` | Output of a shell command: stdout, stderr, exit code |
| `Report` | Structured human-and-LLM-readable summary of a workflow's results |

These are TOML-defined schemas in the codebase. Type matching at graph-build time is a string comparison plus version-compatibility rules.

### 5.4 Plugin contribution model

A package's manifest may contribute nodes to the graph. v1 supports a *content-only* contribution model: packages declare files to write and boot snippets to install, but do not contribute executable nodes. This keeps v1 small.

v1.5 may extend this to allow packages to contribute LLM nodes (e.g., a flow that adds a `wal:checkpoint` node bound after `build:compile`). Document the extension point but do not implement it in v1.

This means: in v1, all nodes in the graph are built-in. Plugins influence the graph only by changing what content the built-in nodes operate on.

### 5.5 Workflows as graph queries

Each workflow is defined as a target node name. v1 workflows and their target nodes:

| Workflow | Target node |
|---|---|
| `init` | `init:complete` |
| `install` | `install:complete` |
| `uninstall` | `uninstall:complete` |
| `update` | `update:complete` |
| `list` | `list:report` |
| `check` | `check:report` |
| `show effective` | `show:effective` |
| `show graph` | `show:graph` |
| `show node` | `show:node` |

Each target's transitive dependencies define what the workflow does. Adding a new workflow is adding a new target node and its dependency chain.

### 5.6 The `install` workflow in detail

This is the most important workflow in v1; specify it precisely.

Subgraph for `install:complete` when invoked as `vibe install flow:wal`:

```
load:project-toml
        │
        ▼
install:resolve         (input: ProjectConfig + PackageRef → PackageRef with version)
        │
        ▼
install:fetch           (input: PackageRef → PackageContents)
        │
        ▼
install:review          (input: PackageContents → PackageContents)
                        (v1 no-op; v2: LLM censor)
        │
        ▼
install:plan            (input: PackageContents → WritePlan)
        │
        ▼
install:user-confirm    (input: WritePlan → Approval; interactive)
        │
        ▼
install:apply           (input: WritePlan + Approval → CommandResult)
        │
        ▼
install:update-lockfile (input: PackageRef + applied → Lockfile)
        │
        ▼
install:complete        (barrier)
        │
        ▼
install:report          (input: ... → Report)
```

Each named node is a built-in. `install:user-confirm` is a `prompt` node that pauses for user input. All other mutating nodes (`install:apply`, `install:update-lockfile`) only run after `install:user-confirm` produces an `Approval` with a positive value.

**M0 implementation note.** In M0 the `install` workflow is implemented procedurally inside the `vibe-install` library (`plan_install` / `apply_install` / `register_installed`) rather than executed through a formal graph runner. The node names above reflect the logical shape and map one-to-one onto library functions. `install:review` is elided entirely in M0 (no corresponding function); when M2 introduces an LLM-driven censor it will land as a new stage between `install:fetch` and `install:plan`. The graph-runner sophistication described here is a v2 deliverable — v1 ships the same semantics executed procedurally so the type system and testability benefits hold without the runner's infrastructure cost.

### 5.7 The `build` workflow in detail (v1.5 scope; document for forward compatibility)

When the user invokes `vibe build feat:welcome-page --stack rust-cli`, the subgraph:

```
load:project-toml
        │
        ▼
load:active-flows       (loads all installed flows' content)
        │
load:active-stack       (loads stack:rust-cli content)
        │
load:feat               (loads feat:welcome-page content)
        │
load:wal
        │
        ▼
load:effective-spec     (merges all of the above)
        │
        ▼
build:plan              (LLM: produce structured BuildPlan from EffectiveSpec)
        │
        ▼
build:user-confirm      (interactive: show plan, await approval)
        │
        ▼
build:compile           (LLM: tool-use loop; reads spec, writes code; produces CodeFiles)
        │
        ▼
build:write-files       (write CodeFiles to disk)
        │
        ▼
build:test              (shell: run stack-defined tests)
        │
        ▼
build:complete          (barrier)
        │
        ▼
build:report            (Report)
```

Note: in v1.5, flows that want to participate in build (e.g., `wal:checkpoint`) will need the plugin contribution model extended. v1 does not need this.

---

## Section 6. The boot directory model

The `spec/boot/` directory is the key innovation that gives vibevm cross-agent compatibility. Specify it precisely.

### 6.1 The model

`spec/boot/` is a flat directory of numbered markdown files. The numeric prefix establishes execution order. Each file is self-contained instructions for an AI agent reading the project at session start.

The user's `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, etc. each contain only this one line:

```
Read every file in spec/boot/ in filename order, then await the user's instructions.
```

This single line is the cross-agent compatibility layer. Every modern coding agent supports reading project-level instruction files. By redirecting all of them to the same source of truth, the project's behavior stays identical regardless of which agent the user runs.

### 6.2 The file numbering convention

Files are named `NN-<short-name>.md` where `NN` is a two-digit number 00-99. Convention:

| Range | Purpose |
|---|---|
| 00-09 | Project foundations, owned by the user |
| 10-49 | Flow-contributed boot snippets (process discipline) |
| 50-79 | Stack-contributed boot snippets (technology context) |
| 80-89 | Reserved |
| 90-99 | User overrides, owned by the user, never touched by `vibe` |

The exact number within a range is chosen by the package author. Conflicts (two packages picking the same number) are detected at install time and reported to the user.

The user always owns `00-core.md` and `90-user.md`. `vibe init` creates these. `vibe install` never modifies them. `vibe uninstall` never deletes them.

### 6.3 Adding and removing snippets

When a package is installed:
1. The package manifest's `boot_snippet` field declares: target filename (e.g., `10-flow-wal.md`) and source file within the package.
2. The CLI scans `spec/boot/` for two kinds of conflict:
   - **Exact-filename conflict.** Another file with the same name already exists. Abort with exit code 3 and, if possible, name the owning package (the lockfile is consulted for attribution).
   - **Numeric-prefix conflict.** Another file in the package-writable range `10-89` shares the same `NN-` prefix. This is the case §6.2 anticipates: two packages independently picked the same number. Abort with exit code 3.
   - User-owned files in ranges `00-09` and `90-99` are excluded from both checks; packages cannot target those ranges at all.
3. The CLI copies the source file to the target path.

When a package is uninstalled:
1. Look up the boot snippet declared in the package's record in the lockfile.
2. Delete that file from `spec/boot/`.
3. Files in 00-09 and 90-99 ranges are never touched.

### 6.4 Why a directory not a single file

Earlier designs considered a single `BOOT.md` with marker-delimited sections (`<!-- vibe:flow:wal:begin -->...<!-- vibe:flow:wal:end -->`). The directory approach is preferred because:
- No marker-corruption failure mode.
- Order is visible in `ls`.
- Each package owns one whole file, no merge conflicts in shared files.
- Adding/removing is a file operation, not a text-edit operation.
- Users can edit individual snippets if they want without risk of breaking adjacent snippets.

### 6.5 The attention-weight caveat

Because LLMs suffer from "lost in the middle" attention degradation, files earlier in the boot sequence get more attention weight than files in the middle. Document this explicitly in the user-facing flow authoring guide (when written): flow authors should not rely on a particular ordering for *correctness*, only for *priority*. A flow whose correctness depends on running before another flow should declare an explicit dependency, not rely on numeric ordering.

In v1, dependency declaration is not implemented; conflicts are resolved by the user picking numbers manually. v2 may add dependency-based ordering.

---

## Section 7. The package model

### 7.1 Package identity

A package is identified by `<kind>:<name>@<version>` where:
- `kind` is one of `flow`, `feat`, `stack`, `tool`.
- `name` is a kebab-case string, globally unique within its kind.
- `version` is a semver string.

Example: `flow:wal@0.3.0`, `feat:welcome-page@1.2.0`, `stack:rust-cli@0.1.0`.

In CLI commands, version is optional and defaults to "latest stable":
- `vibe install flow:wal` → installs latest stable `flow:wal`.
- `vibe install flow:wal@0.3.0` → installs exactly that version.
- `vibe install flow:wal@^0.3` → installs latest 0.3.x.

### 7.2 Package contents

A package is a directory containing:
- `vibe-package.toml` — the package manifest (required).
- `README.md` — human-readable description (required).
- Other content files referenced by the manifest (e.g., boot snippets, spec files).

### 7.3 Package manifest schema

```toml
# vibe-package.toml within a package directory

[package]
name = "wal"                        # without the kind prefix
kind = "flow"                       # one of: flow, feat, stack, tool
version = "0.3.0"
authors = ["Oleg Chirukhin <oleg@example.com>"]
license = "EULA"
description = "Write-Ahead Log discipline for human-AI development sessions"
homepage = "https://github.com/.../wal"
keywords = ["wal", "memory", "discipline"]

[compatibility]
# Minimum vibevm CLI version required
min_vibe_version = "0.1.0"
# Only relevant for feats and tools
requires_kinds = []                 # e.g., a feat might require ["stack"]

[writes]
# Files this package owns exclusively. Created on install, removed on uninstall.
files = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/session-end-hook.md",
]

# A boot snippet is a special case: declared filename, source path within package
[boot_snippet]
filename = "10-flow-wal.md"         # target path: spec/boot/10-flow-wal.md
source = "boot/10-flow-wal.md"      # source path within package

[dependencies]
# Other packages this package requires (by kind:name@constraint)
# v1 supports declaration but does not auto-install dependencies.
required = []                       # e.g., ["flow:atomic-commits@^0.1"]
conflicts = []                      # e.g., ["flow:legacy-wal"]
```

### 7.4 Lockfile schema

```toml
# vibe.lock at project root

[meta]
generated_by = "vibe 0.1.0"
generated_at = "2026-04-16T12:00:00Z"

[[package]]
kind = "flow"
name = "wal"
version = "0.3.0"
source = "git+ssh://git@gitverse.ru/anarchic/vibespecs.git#flow/wal/v0.3.0"
content_hash = "sha256:..."
boot_snippet = "10-flow-wal.md"
files_written = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/session-end-hook.md",
    "spec/boot/10-flow-wal.md",
]

[[package]]
kind = "stack"
name = "rust-cli"
version = "0.1.0"
# ... etc
```

The lockfile is the source of truth for what is installed. `vibe list` reads it. `vibe uninstall` reads it to know what files to remove. It is committed to git.

### 7.5 Project manifest schema

```toml
# vibe.toml at project root

[project]
name = "my-telegram-client"
version = "0.0.1"
authors = ["Oleg <oleg@example.com>"]

[active]
# The currently active stack (used as default for `vibe build`)
stack = "rust-cli"

[llm]
# LLM provider configuration; per-step overrides allowed in v1.5
default_provider = "anthropic"
default_model = "claude-sonnet-4-7"
api_key_env = "ANTHROPIC_API_KEY"

# Optional: per-step LLM configuration (v1.5)
# [llm.build]
# provider = "anthropic"
# model = "claude-opus-4-7"
# 
# [llm.review]
# provider = "openrouter"
# model = "meta-llama/llama-3.3-70b"
# api_key_env = "OPENROUTER_API_KEY"

[registry]
# The registry to use. v1: a git URL. v2+: may include hosted registries.
url = "git@gitverse.ru:anarchic/vibespecs.git"
ref = "main"
```

---

## Section 8. The registry

### 8.1 Registry model: a git repository

The registry is a git repository with a defined directory structure. v1 supports exactly one registry, configured per-project in `vibe.toml`.

### 8.2 Registry layout

```
vibevm-registry/
├── flow/
│   ├── wal/
│   │   ├── v0.1.0/
│   │   │   ├── vibe-package.toml
│   │   │   ├── README.md
│   │   │   └── ...
│   │   ├── v0.2.0/
│   │   │   └── ...
│   │   ├── v0.3.0/
│   │   │   └── ...
│   │   └── latest                 # File or symlink containing version string
│   └── sync-from-code/
│       └── ...
├── feat/
│   └── welcome-page/
│       └── v0.1.0/
│           └── ...
├── stack/
│   └── rust-cli/
│       └── v0.1.0/
│           └── ...
└── tool/                          # Reserved
```

Resolution: `vibe install flow:wal@^0.3` →
1. Fetch (or use cached) registry repo.
2. List `flow/wal/v*/` directories.
3. Find latest version matching `^0.3`.
4. Use contents of `flow/wal/v0.3.x/` as the package.

### 8.3 Fetching strategy

The CLI clones the registry repo into `~/.vibe/registries/<hash>/` on first use. Subsequent fetches do `git pull`. Use sparse-checkout if the registry is large; v1 may skip optimization and just clone everything.

For each package install, copy the relevant version directory into `.vibe/cache/<kind>/<name>/<version>/`. Compute and store the content hash. The cache is reusable across projects on the same machine.

### 8.4 Publishing

`vibe publish` is reserved for v2+. v1 users contribute packages by submitting PRs to the registry repo. Document this in the package authoring guide (when written).

### 8.5 Future: LLM-based censoring

A v2 feature: before applying writes, the CLI invokes an LLM to review the package contents and emit a safety analysis. The user sees both the plan (mechanical: which files will be written) and the analysis (semantic: does this look like it's trying to do something malicious or surprising).

v1 architectural hook: the `install:review` node exists in the install subgraph and is a no-op in v1. v2 replaces its implementation.

---

## Section 9. The CLI surface

### 9.1 Command summary (v1)

```
vibe init [--path <dir>] [--name <n>] [--stack <stack-name>]
    # Create project structure. --path defaults to cwd, --name defaults to the
    # directory basename, --stack pre-populates `[active]` in vibe.toml.
vibe install <pkgref> [<pkgref> ...] [--path <dir>] [--registry <path>] [--assume-yes]
    # Install one or more packages. --registry wins over the vibe.toml
    # [registry] url; --assume-yes skips the interactive confirmation
    # (required in non-TTY environments like CI).
vibe uninstall <pkgref> [--path <dir>] [--assume-yes]
    # Remove a package. Version portion of <pkgref> is ignored on uninstall.
vibe update <pkgref> | --all                           # Re-fetch and apply changes (M1)
vibe list [--kind <kind>] [--path <dir>]               # Show installed packages
vibe check                                             # Validate spec consistency (M1)
vibe show effective [--feat <name>] [--stack <name>]   # Print effective spec (M1)
vibe show graph [<workflow-name>]                      # Print task graph (M1)
vibe show node <node-name>                             # Print node details (M1)
vibe show plan <workflow-name> [args...]               # Print what would happen, don't execute (M1)
vibe registry sync                                     # Force-refresh the registry cache (M1)
vibe help [<command>]                                  # Help text
vibe version                                           # Version info
```

Every command honours the two global flags `--json` (machine-readable output) and `--quiet` (one-line summary); they are mutually exclusive. `--json` output is a stream of one or more JSON documents on stdout — `install`, for instance, emits the plan and then the report as separate top-level objects so consumers can parse the plan before approval lands.

### 9.2 Commands deferred to v1.5

```
vibe build <feat-pkgref> [--stack <stack-name>]        # Generate code
vibe sync                                              # Reconcile code-spec drift
vibe build --with-install <feat-pkgref>                # Compose install + build
```

### 9.3 Output format

The CLI defaults to a structured human-readable format that is also LLM-readable:
- Markdown-flavored with clear headers.
- Important info at the start (per the lost-in-the-middle attention rule).
- Status indicators with conventional symbols (✓, ✗, ⚠, →) — but also plain text equivalents for non-Unicode terminals.
- All `spec://` URIs displayed as clickable when the terminal supports it.

A `--json` flag produces fully machine-readable output. Skills consuming the CLI use `--json`. Humans use the default.

A `--quiet` flag reduces output to one line of summary. Useful in CI and in scripts.

### 9.4 Exit codes

- `0` — success.
- `1` — general error (file not found, parse error, etc.).
- `2` — usage error (bad command-line arguments).
- `3` — package conflict (e.g., two packages claim the same boot snippet number).
- `4` — type mismatch in graph construction.
- `5` — user declined confirmation.
- `6` — LLM provider error (rate limit, auth failure, etc.).
- `10+` — reserved for specific failure modes documented per command.

### 9.5 Configuration sources, in precedence order

1. Command-line flags (highest precedence).
2. Environment variables (`VIBE_*` prefix).
3. Project `vibe.toml`.
4. User-level config at `~/.config/vibe/config.toml`.
5. Built-in defaults (lowest precedence).

`vibe show config` prints the effective configuration with provenance for each value.

---

## Section 10. Implementation language and dependencies

### 10.1 Language: Rust

Implement vibevm in Rust. Rationale:
- Single-binary distribution (no runtime dependency on Node.js or Python).
- Cross-platform (works on macOS, Linux, Windows without per-platform installers).
- Performance is adequate for the workload (file I/O, graph computation, HTTP).
- Strong type system catches errors at compile time, fitting the philosophy.
- Excellent ecosystem for CLI development (`clap`), TOML parsing (`toml`, `serde`), HTTP (`reqwest`), git (`git2`), and async LLM calls.

The target binary should be runnable as `vibe` after a single `cargo install` or `brew install`.

### 10.2 Crate structure

```
vibevm/
├── Cargo.toml
├── crates/
│   ├── vibe-cli/                 # CLI entry point, argument parsing
│   │   └── src/main.rs
│   ├── vibe-core/                # Core types, manifest schemas, graph model
│   │   └── src/lib.rs
│   ├── vibe-graph/               # Graph builder and runner
│   │   └── src/lib.rs
│   ├── vibe-registry/            # Git-based registry: fetch, cache, resolve
│   │   └── src/lib.rs
│   ├── vibe-install/             # Install/uninstall/update logic
│   │   └── src/lib.rs
│   ├── vibe-llm/                 # LLM provider abstraction (used by v1.5+)
│   │   └── src/lib.rs
│   └── vibe-check/               # Linter for spec consistency
│       └── src/lib.rs
└── tests/                        # Integration tests
```

Each crate has clear responsibilities; cross-crate dependencies follow the diagram (cli depends on everything; everything depends on core).

### 10.3 Required external dependencies (all permissive licenses)

- `clap` (Apache-2.0/MIT) — argument parsing.
- `serde` + `toml` (Apache-2.0/MIT) — manifest parsing.
- `reqwest` (Apache-2.0/MIT) — HTTP for v1.5 LLM calls.
- `git2` (Apache-2.0/MIT) — registry git operations. (Alternative: shell out to `git`. Decide based on dependency footprint.)
- `tokio` (MIT) — async runtime for v1.5.
- `anyhow` + `thiserror` (Apache-2.0/MIT) — error handling.
- `tracing` (MIT) — structured logging.
- `dialoguer` (MIT) — interactive prompts.
- `console` (MIT) — colored terminal output.
- `sha2` (Apache-2.0/MIT) — content hashing.

Avoid GPL/AGPL/LGPL dependencies entirely. The user's license preference is permissive only.

### 10.4 LLM provider integration (for v1.5)

Implement provider abstraction in `vibe-llm` crate. v1.5 supports:
- **Anthropic** (Claude models) via the official Messages API.
- **OpenAI** (GPT models) via the Chat Completions API.
- **OpenRouter** (any model) via OpenAI-compatible API.
- **Ollama** (local models) via OpenAI-compatible API at localhost.

Each provider has a struct implementing a common `LLMProvider` trait. The trait exposes:
- A `chat` method for single-shot calls.
- A `chat_with_tools` method for tool-use loops.
- A `stream_chat` method for streaming output (v2).

Tool-use loop implementation pattern (for `build` and `sync` workflows):

```rust
// Pseudocode
async fn build_with_tools(provider: &dyn LLMProvider, context: BuildContext) -> Result<CodeFiles> {
    let mut messages = vec![system_message(context)];
    let tools = vec![read_file_tool(), write_file_tool(), list_dir_tool(), run_test_tool()];
    
    loop {
        let response = provider.chat_with_tools(&messages, &tools).await?;
        match response.stop_reason {
            StopReason::EndTurn => return Ok(extract_code_files(messages)),
            StopReason::ToolUse(calls) => {
                messages.push(response.into_message());
                let mut results = vec![];
                for call in calls {
                    let result = execute_tool_locally(&call)?;
                    results.push(tool_result_message(&call, result));
                }
                messages.extend(results);
            }
        }
    }
}
```

Tool execution must enforce that file operations are scoped to the project root. No path traversal. No reads outside the project.

### 10.5 Spec-driven development of vibevm itself

vibevm is built using vibevm's own philosophy. The `vibevm` source tree itself follows the structure described in the book: `spec/` directory with PROP/FEAT documents, WAL.md, BOOT.md (or `spec/boot/`), CLAUDE.md as redirect. The Reader writes vibevm using the same discipline that vibevm enforces. This is meta-bootstrapping but it's also the most rigorous test of the design.

---

## Section 11. Staging plan

vibevm ships in staged milestones. Each milestone is *useful on its own* — if work stops at any milestone, the user has a usable product.

### 11.1 Milestone M0: Walking skeleton

**Scope.** A minimum-viable installer that proves the file-management mechanics work.

**Commands shipped.**
- `vibe init [--path] [--name] [--stack]` — creates the §4.2 project structure: `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect files, the `spec/` tree with `boot/00-core.md`, `boot/90-user.md`, `WAL.md`, a project-level `.gitignore`, `.vibe/cache/`, `vibe.toml` (with `[active]` pre-populated if `--stack` was given), and an empty `vibe.lock`. Idempotent: a second run never clobbers user-modified files and reports each as `kept`.
- `vibe install <kind>:<name>[@version] [...] [--registry <path>] [--assume-yes]` — installs from a *local directory* registry (no git yet). Reads the package manifest, fetches into `.vibe/cache/`, plans the writes (including conflict detection), shows the plan, confirms with user, applies the writes, updates `vibe.lock`.
- `vibe list [--kind]` — reads lockfile, renders a table (default) or JSON (`--json`) or a one-line comma list (`--quiet`).
- `vibe uninstall <kind>:<name> [--assume-yes]` — reverses an install. Never touches user-owned files (`00-core.md`, `90-user.md`) or structural directories.

**Out of scope.** No git registry. No LLM. No `build`. No `sync`. No `check`. No `update`. No formal graph runner (workflows are implemented procedurally inside `vibe-install` — see §5.6's M0 implementation note).

**Verification.** Init a project, install a hand-written `flow:wal` from a local directory, verify files appear in the right places, uninstall, verify files are removed. Hand-written `flow:wal` is the test fixture. All 15 items in §16 (M0 acceptance checklist) pass.

**Estimated effort.** One weekend.

### 11.2 Milestone M1: The package manager

**Scope.** Full package manager functionality with git registry, multiple installed packages, and the consistency linter.

**Commands shipped (additive over M0).**
- `vibe install` now supports git registry as configured in `vibe.toml`.
- `vibe update <pkgref> | --all` — refetches and applies changes.
- `vibe registry sync` — refreshes registry cache.
- `vibe check` — runs the linter (see Section 12).
- `vibe show effective` — prints the materialized effective spec.
- `vibe show graph` — prints the task graph for a given workflow.
- `vibe show config` — prints effective configuration with provenance.
- `vibe help`, `vibe version` — standard.

**Plus.** The hand-written `flow:wal`, plus at least two more demo packages, are published to the registry. The registry is a real git repo on GitHub.

**Verification.** A user with no prior vibevm exposure can install vibevm, run `vibe init`, install three packages, see the effective spec, run `vibe check` and get a clean report. They never edit any vibevm-managed file by hand.

**Estimated effort.** Two to four weekends.

### 11.3 Milestone M1.5: Generation

**Scope.** The `build` workflow ships, with LLM-driven code generation via tool-use loops.

**Commands shipped (additive).**
- `vibe build <feat-pkgref> [--stack <stack-name>]` — generates code.
- `vibe sync` — reconciles drift between code and spec.
- LLM provider abstraction supports Anthropic, OpenAI, OpenRouter, and Ollama.

**Plus.** A real `feat:welcome-page` package and `stack:rust-cli` package in the registry. They are designed to be a working example. Building them produces a runnable Rust CLI welcome-page demo.

**Verification.** A user runs `vibe init`, `vibe install stack:rust-cli`, `vibe install feat:welcome-page`, `vibe build feat:welcome-page`. The generated code compiles, runs, and matches the feat's acceptance criteria.

**Estimated effort.** Three to six weekends, primarily for hardening tool-use loops.

### 11.4 Milestone M2: Production-readiness

**Scope.** Everything needed for vibevm to be safely used by people other than the author.

**Adds.**
- `install:review` becomes an LLM-driven censor.
- Plugin contribution model extends to include LLM nodes (so flows can register goals like `wal:checkpoint`).
- Authentication for private registries (token-based).
- Cross-platform build matrix (macOS, Linux, Windows binaries).
- `vibe doctor` command for diagnosing project state.
- Comprehensive error messages with actionable suggestions.

**Estimated effort.** Open-ended; depends on adoption signals.

### 11.5 Milestone M3+: Speculative directions

Documented for reference, not for v1 implementation:
- **Interpret mode.** `vibe run <feat-pkgref>` — execute the spec directly without generating code, using an LLM as the runtime interpreter.
- **Multi-stack composition.** A single feat compiled for multiple stacks simultaneously (e.g., a UI feat compiled for both web and mobile).
- **Skill layer.** Distributable Claude Code / Codex / OpenCode skills that wrap the CLI for native slash-command access.
- **Hosted registry.** Replace git-as-registry with a proper package registry server.

These are not in scope. Mention them in design only to ensure the foundation supports them later.

---

## Section 12. The linter (vibe check)

`vibe check` runs deterministic checks on the project's spec content. No LLM. Pure inspection.

Checks performed:

1. **Manifest validity.** `vibe.toml` parses and matches schema. `vibe.lock` parses and matches schema.
2. **Dead `spec://` references.** Every `spec://` URI in any spec file or in any code comment (`// Implements: spec://...`) resolves to an existing anchor.
3. **Orphan anchors.** Every `{#anchor}` defined in a spec is referenced from somewhere (spec, code, or test).
4. **Anchor uniqueness.** Each `{#anchor}` is unique within its spec file.
5. **WAL freshness.** `spec/WAL.md` modification timestamp is less than 24 hours old; warn if older.
6. **WAL well-formedness.** WAL has the required sections (Current Phase, Constraints, Done, Next, Issues).
7. **Boot directory consistency.** Every file in `spec/boot/` matches the `NN-name.md` pattern; no number conflicts.
8. **Lockfile consistency.** Every package in `vibe.lock` has its declared files present; no orphaned files in `spec/flows/`, `spec/feats/`, `spec/stacks/` that aren't in any lockfile entry.
9. **REVIEW marker aging.** Any `<!-- REVIEW: ... -->` older than configured threshold (default 14 days) is reported.
10. **Implementation coverage.** For each feat with a `build` history, files generated from it should have `Implements: spec://...` markers. Report missing markers.

Output format: structured report with severity (error, warning, info) per issue, file paths and line numbers, suggested fix when possible.

`vibe check --fix` attempts safe automatic fixes (e.g., removing dead anchor references). Document precisely what `--fix` is allowed to change; never autofix anything that loses information.

Exit code: 0 if no errors, 1 if errors, 0 with warnings displayed if only warnings.

---

## Section 13. The hand-written flow:wal package

This is the canonical demo package. Implementing it correctly is the v1 acceptance test for the package model.

### 13.1 Package contents

```
flow-wal-package/
├── vibe-package.toml
├── README.md
├── spec/
│   └── flows/
│       └── wal/
│           ├── WAL-PROTOCOL.md      # -> <project>/spec/flows/wal/WAL-PROTOCOL.md
│           ├── session-end-hook.md  # -> <project>/spec/flows/wal/session-end-hook.md
│           └── morning-routine.md   # -> <project>/spec/flows/wal/morning-routine.md
└── boot/
    └── 10-flow-wal.md                # source path; target is <project>/spec/boot/10-flow-wal.md
```

**Package layout is a mirror layout.** Every entry in `writes.files` is simultaneously (a) the path of the file inside the package directory and (b) the path at which it will be installed in the consumer's project. There is no separate `target = "…"` field per entry; `writes.files` is the single source of truth. A human author inspecting a package directory knows immediately what will appear in a consumer's project — no mapping, no rewriting, no sidecar configuration.

**Boot snippets are the one exception.** The `[boot_snippet]` table has an explicit `source` field naming the path inside the package (conventionally `boot/NN-<name>.md`) because the target is always `spec/boot/<filename>` — a fixed prefix plus the filename, not a free-form project path.

### 13.2 Manifest

```toml
[package]
name = "wal"
kind = "flow"
version = "0.1.0"
authors = ["Oleg Chirukhin"]
license = "EULA"
description = "Write-Ahead Log discipline for human-AI development sessions"
keywords = ["wal", "memory", "discipline", "session-management"]

[compatibility]
min_vibe_version = "0.1.0"
requires_kinds = []

[writes]
files = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/session-end-hook.md",
    "spec/flows/wal/morning-routine.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"

[dependencies]
required = []
conflicts = []
```

### 13.3 Boot snippet content

```markdown
# Flow: WAL (Write-Ahead Log)

This project uses WAL discipline for session continuity.

At the start of every session:
1. Read spec/WAL.md before doing anything else.
2. Verify the WAL date is current. If older than 24 hours, ask the user to confirm state.
3. Honor every constraint listed in the WAL's Constraints section.

During the session:
4. If the user makes a decision that affects future sessions, propose adding it to the WAL.
5. If you propose to violate a Constraint, stop and ask the user explicitly.

At the end of the session:
6. Update spec/WAL.md per the protocol in spec/flows/wal/session-end-hook.md.
7. The WAL must reflect the *current* state, not the history. History lives in git.

Full protocol: spec/flows/wal/WAL-PROTOCOL.md
```

### 13.4 The protocol document

The full WAL protocol document (`spec/flows/wal/WAL-PROTOCOL.md` after install) is derived from the user's book chapter 3 ("Архитектура памяти") and chapter 2 (sections on WAL). Reproduce the structure faithfully but in English. Include:
- Definition (WAL is a checkpoint, not a log).
- Required sections (Current Phase, Constraints, Done, Next, Issues).
- Update triggers (end of session, before destructive operations, on context switch).
- Size budget (target ≤ 3000 tokens, hard limit ≤ 5000 tokens).
- Conflict with the human (head wins).

This document, plus the boot snippet, is what the user installs when they run `vibe install flow:wal`.

---

## Section 14. Development methodology

The Reader (Claude Code) implements vibevm using Spec-Driven Development with WAL, single-developer mode, as described in the user's book chapters 1-3 (in `refs/book/`).

### 14.1 Project initialization

Before writing any code:
1. Create a `spec/` directory in the vibevm source tree.
2. Create `spec/WAL.md` with the initial state.
3. Create `spec/boot/00-core.md` with foundational decisions about the project.
4. Create `spec/common/PROP-000.md` with foundational technical decisions (Rust, TOML, proprietary EULA, etc.).
5. Create `spec/modules/` for module-level specs (PROP-001 onward).
6. Set up `CLAUDE.md` at the source tree root with the one-line redirect.
7. Eat your own dogfood: this layout is exactly what `vibe init` will generate. You're hand-creating it now because `vibe` doesn't exist yet.

### 14.2 The discipline

For every coding session:
1. Read `spec/WAL.md` first.
2. Read the relevant PROP/FEAT for the work at hand.
3. Make changes in small, atomic commits. Each commit message should reference the relevant `spec://` URI.
4. Update WAL at end of session.
5. If a decision needs to be made that isn't in the spec, write it to the spec first, then implement.

### 14.3 What to do when stuck

When a design question arises that this document doesn't answer:
1. Re-read the relevant section of this document.
2. Re-read the relevant chapter of the book in `refs/book/`.
3. Look at how the closest analog (cargo, uv, spec-kit) handles the same question.
4. If still unclear, *write a PROP document proposing the resolution*, mark it with REVIEW, and proceed with the most conservative interpretation. The user will review and refine.

Do not silently invent. The book is explicit about this: REVIEW marker before any decision the spec doesn't authorize.

### 14.4 Iteration order

Build M0 entirely before starting M1. Do not skip ahead. Each milestone should result in a working, tested, releasable artifact before the next one starts.

Within M0:
1. Define core types (`vibe-core` crate). All struct definitions, all schema parsing.
2. Build CLI scaffolding (`vibe-cli` crate) with stubbed command handlers.
3. Implement `vibe init`. Test it.
4. Implement `vibe install` against a local-directory registry. Test it.
5. Implement `vibe list`. Test it.
6. Implement `vibe uninstall`. Test it.
7. Hand-write `flow:wal` as a real package. Install it into a test project. Verify everything.
8. Update WAL. Tag M0 release.

Within M1:
1. Implement git registry support (`vibe-registry` crate).
2. Migrate `vibe install` to use git registry instead of local directory.
3. Implement `vibe update`.
4. Implement `vibe-check` linter.
5. Implement `vibe show` subcommands.
6. Set up the actual public registry repo on GitHub.
7. Publish `flow:wal`, plus two more demo packages (suggestions: `flow:sync-from-code`, `flow:atomic-commits`).
8. Update WAL. Tag M1 release.

Within M1.5:
1. Implement `vibe-llm` crate with provider abstraction.
2. Implement Anthropic provider with tool-use support.
3. Implement `vibe build` for the simplest possible feat × stack combination.
4. Hand-write `feat:welcome-page` and `stack:rust-cli`.
5. Verify the build produces working code.
6. Implement `vibe sync`.
7. Add OpenAI, OpenRouter, Ollama providers.
8. Update WAL. Tag M1.5 release.

### 14.5 Testing strategy

- **Unit tests** for every parsing, validation, type-conversion function. These should run in milliseconds.
- **Integration tests** for each CLI command, using a temporary directory and a fixture package. Use `assert_cmd` or similar.
- **End-to-end tests** for full workflows (init → install → list → uninstall) using a fixture local-directory registry.
- For M1.5, **golden-file tests** for build outputs: run a build with a deterministic LLM response (recorded fixture), compare output to expected files. Do not test against live LLM in CI; use recorded fixtures.

Aim for coverage that catches regressions, not coverage as a metric. Test the seams (manifest parsing, graph construction, file operations) more than the leaves.

### 14.6 Documentation discipline

For every command and every public type:
- Have a doc-comment explaining what it does and why.
- Have a usage example in the command's help text.
- For each command, have an entry in `docs/commands/<command>.md`.
- For each kind (flow/feat/stack), have an authoring guide in `docs/authoring-<kind>.md`.

Documentation is part of the deliverable. Do not defer it to "after v1."

---

## Section 15. Glossary

Terms used throughout this document, in alphabetical order. When in doubt, refer here.

- **Active stack.** The stack currently selected by default for `build` operations.
- **Barrier.** A node in the task graph with no work, used as a coordination/sync point.
- **Boot directory.** `spec/boot/`, contains numbered markdown files read by AI agents at session start.
- **Boot snippet.** A small markdown file installed by a package into `spec/boot/` to declare its presence to AI agents.
- **Build.** The workflow that generates code from a feat × stack combination via LLM invocation.
- **CLAUDE.md / AGENTS.md / GEMINI.md.** Agent-specific instruction files at project root, each containing a one-line redirect to the boot directory.
- **Code.** Source files outside `spec/`, generated or user-edited.
- **Compile.** Synonym for `build` in user-facing contexts.
- **Declared writes.** The explicit list of files a package will create or modify, recorded in its manifest.
- **Effective spec.** The materialized state of all installed packages and project configuration at a point in time.
- **Feat.** An installable kind: an abstract feature description, decoupled from any technology stack.
- **Flow.** An installable kind: a process discipline that modifies how the human-AI development workflow operates.
- **Head.** The human developer's memory; not vibevm's concern but acknowledged in design.
- **Install.** The workflow that resolves, fetches, reviews, plans, confirms, and applies a package.
- **Kind.** One of `flow`, `feat`, `stack`, `tool`. The category of a package.
- **Lockfile.** `vibe.lock` at project root, the source of truth for what is installed at exact versions.
- **LLM provider.** A configured backend that vibevm calls to invoke a language model (Anthropic, OpenAI, etc.).
- **Manifest.** Either the project manifest (`vibe.toml`) or a package manifest (`vibe-package.toml`).
- **Milestone (M0/M1/...)** A release stage with a defined feature set. See Section 11.
- **Node.** A unit of work in the task graph.
- **Package.** A named, versioned installable artifact of one of the four kinds.
- **PackageRef.** A reference of the form `<kind>:<name>` or `<kind>:<name>@<version-constraint>`.
- **Plugin.** Synonym for "package" in some contexts; "package" is preferred in user-facing text.
- **Project manifest.** `vibe.toml`, contains project-wide configuration.
- **Registry.** A git repository containing packages, structured per Section 8.
- **REVIEW marker.** An inline marker indicating an unresolved decision the human should look at.
- **Spec.** The corpus of files under `spec/` that aren't WAL or boot. Stable, addressable, versioned.
- **`spec://` URI.** The addressing scheme for spec content.
- **Stack.** An installable kind: a concrete technology context that maps abstract feats to specific code.
- **Sync.** The workflow that reconciles code changes back to spec changes via LLM.
- **Task graph.** The internal DAG used to execute workflows.
- **Tool (kind).** Reserved kind for v1; not implemented.
- **Tool-use loop.** The LLM API pattern where the model can call functions (read_file, write_file, etc.) and receive results, iterating to a final response.
- **Typed value system.** The set of named types carried on graph edges. Defined in Section 5.3.
- **vibevm.** The project as a whole.
- **`vibe`.** The CLI binary.
- **WAL (Write-Ahead Log).** The project state checkpoint at `spec/WAL.md`. Volatile, rewritten each session, the bridge between sessions.
- **Workflow.** A user-facing named operation invoked from the CLI; corresponds to a query on the task graph.

---

## Section 16. Acceptance checklist for v1

Before declaring any milestone complete, verify every item in its section.

### M0 acceptance

- [ ] `vibe init` creates a project structure that matches the layout in Section 4.2.
- [ ] `vibe init` is idempotent: running it twice in the same directory does not destroy user-modified files.
- [ ] `vibe install <kind>:<name>` from a local directory registry copies declared files to declared locations.
- [ ] `vibe install` shows a plan (what will be written) and asks for confirmation before mutating.
- [ ] `vibe install` updates `vibe.lock` correctly.
- [ ] `vibe install` errors clearly when two packages claim the same boot snippet number.
- [ ] `vibe list` reflects the lockfile.
- [ ] `vibe uninstall <kind>:<name>` removes only the files declared by that package.
- [ ] `vibe uninstall` updates `vibe.lock` correctly.
- [ ] User-owned files (`spec/boot/00-core.md`, `spec/boot/90-user.md`) are never touched.
- [ ] `flow:wal` is hand-written as a real package, installs cleanly, uninstalls cleanly.
- [ ] All commands have help text accessible via `--help`.
- [ ] Exit codes match Section 9.4.
- [ ] All output is parseable as either human-readable or `--json`.
- [ ] Test suite covers init, install, uninstall, list with at least one happy-path and one error-path test each.

### M1 acceptance (additive over M0)

- [ ] `vibe install` resolves packages from a git registry per `vibe.toml`'s configuration.
- [ ] Registry cache lives at `~/.vibe/registries/<hash>/`.
- [ ] `vibe registry sync` refreshes the cache.
- [ ] `vibe update <pkgref>` re-fetches and applies changes with diff display.
- [ ] `vibe update --all` updates every installed package.
- [ ] `vibe check` performs all checks listed in Section 12.
- [ ] `vibe check --fix` autofixes only safe issues.
- [ ] `vibe show effective` prints a complete effective spec for the project.
- [ ] `vibe show graph <workflow>` prints the task graph for that workflow.
- [ ] `vibe show config` prints all configuration with provenance.
- [ ] A public git registry is set up on GitHub with at least three packages: `flow:wal`, plus two more.
- [ ] Documentation in `docs/` covers all commands and includes an authoring guide for each kind.

### M1.5 acceptance (additive over M1)

- [ ] LLM provider abstraction in `vibe-llm` supports Anthropic, OpenAI, OpenRouter, Ollama.
- [ ] `vibe build feat:welcome-page --stack rust-cli` produces working Rust CLI code.
- [ ] The generated code includes `Implements: spec://...` markers.
- [ ] The build subgraph respects the user-confirmation node before any mutation.
- [ ] `vibe sync` produces a clean spec delta proposal from a code change.
- [ ] Tool-use loops are sandboxed: file operations cannot escape the project root.
- [ ] LLM API errors are surfaced clearly to the user.
- [ ] LLM costs are reported in the build's structured output.

---

## Section 17. Closing notes for the Reader

This document is the entire specification. If you are implementing vibevm and you find this document silent on a question, either (a) the question is outside v1 scope and should be deferred, or (b) the answer is in `refs/book/` and you should re-read the relevant chapter.

Two things are most important to internalize:

1. **The book's two-process model and the WAL discipline are the soul of this project.** Every design decision should be checked against "does this support the human-AI cooperative model with persistent memory across sessions?" If the answer is no, the design is wrong.

2. **The vibecoding-as-affirmation positioning is the project's market position.** Every user-facing decision should be checked against "does this remove friction for someone who wants to ship a prototype fast?" Tools that demand discipline before they reward you with output are doomed; tools that reward you fast and quietly enforce discipline are loved.

Work in the staging order. Commit small. Update WAL. Reference `spec://` URIs in commit messages. Use REVIEW markers liberally. Ask the user when uncertain.

Build the walking skeleton. Install your first hand-written package. Take it from there.

Good luck.

---

*End of specification document.*
*If you are reading this and have not yet read `refs/book/`, stop now and read those chapters. Resume here afterward.*
