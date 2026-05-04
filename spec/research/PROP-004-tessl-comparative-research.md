# PROP-004 — Tessl comparative research and vibevm roadmap deltas

**Status.** Research document — self-contained, evergreen reference. Not implementation-locked. Each numbered roadmap delta in §6 maps to a future PROP / FEAT / milestone update; this file does not itself ratify those. Companion to [PROP-000](../common/PROP-000.md), [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md), and [PROP-003](../modules/vibe-resolver/PROP-003-dep-evolution.md).

**Purpose.** Tessl ([`https://www.tessl.io`](https://www.tessl.io), docs at [`https://docs.tessl.io`](https://docs.tessl.io)) is a commercial platform launched 2025 that occupies adjacent ground to vibevm: managing AI-coding-agent context (skills, documentation, rules) as versioned, evaluated, distributable software artefacts. They have a non-trivial product surface and ~3,000 published skills; understanding what they do well — and what they don't — is load-bearing intelligence for vibevm's own roadmap. This document captures a complete-as-of-2026-05 inventory of Tessl, identifies the gaps where vibevm trails, the gaps where vibevm leads, and translates the actionable subset into concrete roadmap entries.

**Source corpus.** Materials sourced from the public documentation index `https://docs.tessl.io/llms.txt` (high-level inventory) and the concatenated full corpus `https://docs.tessl.io/llms-full.txt` (intended for LLM consumption — Tessl publishes their own docs in this format), supplemented by the marketing site `https://www.tessl.io`. Direct quotes from the corpus appear in fenced blocks throughout. Re-fetch the canonical URL list under §7 to refresh this research; Tessl ships product changes on a rolling basis (changelog version 0.78.0 as of capture date).

**Reading shape.** §1 — what Tessl is, in their own framing. §2 — primitives (skills, rules, docs, tiles, scenarios, evals, workspaces). §3 — CLI surface. §4 — file formats and on-disk shapes. §5 — features vibevm does **not** today have, with depth on each. §6 — roadmap deltas with priority and crate placement. §7 — source URLs. §8 — what Tessl does **not** do (areas where vibevm leads). §9 — reading-list of related projects.

---

## 1. What Tessl is — in their own words {#what-tessl-is}

Tessl's one-sentence pitch:

> "Tessl is a platform for managing context for coding agents, treating agent skills and context as software with a complete lifecycle: build, evaluate, distribute, and optimize."

The problem statement they lead with:

> "AI agents are now writing real production code. But as libraries change, APIs evolve, and conventions drift, agents struggle to stay correct. The issue isn't the underlying models, but how context is created, updated, assessed and distributed."

Their argument for *why this needs platform infrastructure rather than ad-hoc markdown files*:

> "most teams treat agent skills as static artifacts: markdown files, copied prompts, or repos you clone and hope stay relevant. That approach works briefly, then breaks."

The "vibecoding" failure mode that Tessl positions itself against — the intent-to-code chasm:

> "When you prompt an agent without structure, you get vibecoded output: The agent assumes what you want instead of asking. It hallucinates APIs from stale training data. There's no way to verify the result matches your intent."

Their framing of the lifecycle they offer is four-stage — **Use → Create → Evaluate → Distribute** — built around the premise that agent context is itself software and deserves the same engineering rigor (versioning, tests, releases) as application code:

> "treats agent context as managed software with the same rigor you apply to your codebase."

Tessl markets itself as the closest commercial equivalent to "what GitHub did for code, for agent context." Whether they fulfill that ambition is a separate question; the *category* they're trying to occupy is well-defined and overlaps directly with vibevm's M1.5+ trajectory.

## 2. Primitives — the Tessl object model {#primitives}

Tessl introduces five interlocking content types and several operational concepts. Each is canonical in their docs.

### 2.1 Tile {#tile}

A **tile** is the unit of distribution — a packaged container that bundles together skills, documentation, and rules. From the glossary:

> "A tile is a structured container of context that can include a combination of skills, documents, and rules."

A tile has its own version (semver), name in `workspace/name` form, optional `describes` field linking to an upstream package via PURL (Package URL spec — see §2.7 below), and a manifest at `tile.json`. A single tile can hold multiple skills (each is a `SKILL.md` file referenced from the manifest), multiple documents, and multiple rules. Tiles are what `tessl install` and `tessl publish` operate on; `tessl.json` (project-level) tracks which tiles a project depends on.

Mapping to vibevm: roughly equivalent to a vibevm **package** — `flow:wal`, `feat:welcome-page`, etc. The distinction is that vibevm packages today carry one *kind* (flow / feat / stack / tool) while Tessl tiles can mix kinds inside one container.

### 2.2 Skill (`SKILL.md`) {#skill}

A skill is a procedural workflow:

> "Skills are procedural guides that teach agents how to perform specific workflows. Unlike documentation (which explains what something is), skills explain how to do something step-by-step."

The on-disk format is a markdown file *literally named* `SKILL.md` with strict frontmatter:

```markdown
---
name: database-migration-helper
description: When you need to create and manage database migrations.
---

# Database Migration Helper
[step-by-step procedural content]
```

Frontmatter fields documented as required:

> "`name`: The skill identifier (lowercase, hyphens only); `description`: Clear description of the trigger when skill should activate - this is critical for skill discovery by agents"

The `description` field is load-bearing because it doubles as the **activation trigger** the agent matches against (see §2.10). Skill body length is capped at 500 lines per their structural validator.

Mapping to vibevm: closest to a vibevm flow's main protocol document (`spec/flows/wal/WAL-PROTOCOL.md`), but with explicit frontmatter discoverability metadata that vibevm flows lack today.

### 2.3 Rule {#rule}

A rule is a coding standard that agents always follow:

> "Your team's coding standards" delivered as rules that activate immediately, unlike skills which load "when relevant."

Rules live in a tile's `rules/` subdirectory as markdown files. Tessl describes them as covering "error handling patterns, validation requirements, response format conventions, security best practices, naming conventions." Unlike skills, rules have no activation trigger — they are *always* loaded into the agent's context when the project is open (see §2.10's "eager push" mode).

Mapping to vibevm: closest to the foundation layer in `spec/boot/00-core.md` and `spec/boot/90-user.md` — rules-of-the-house that every session sees. vibevm has no separate authored "rule" content type; everything in `spec/boot/` is functionally equivalent.

### 2.4 Documentation (`docs/`) {#doc}

A doc is library knowledge — API references, framework concepts, usage examples — kept versioned alongside the upstream package version:

> "Current library knowledge via MCP" loaded "on-demand (lazy pull)."

Docs are referenced from `tile.json`'s `docs` field, typically pointing at `docs/index.md` as an entry point. They're agent-queryable through Tessl's MCP server: the agent calls `query_library_docs` when it wants to consult them, and gets back relevant sections.

Mapping to vibevm: no direct equivalent. vibevm packages today carry their own protocol documents (spec/...), but there is no separate "library reference docs" content type, and there is no on-demand query interface — the entire effective spec corpus is materialised into the project tree at install time.

### 2.5 Scenario (`scenario.json` + `task.md` + `criteria.json`) {#scenario}

A scenario is the unit of evaluation. It has three files:

- **`task.md`** — the task brief presented to the agent during evaluation.
- **`criteria.json`** — a weighted checklist rubric used to score the agent's output (per-criterion pass/fail/partial).
- **`scenario.json`** — fixture metadata: repo URL, commit reference, context-exclusion globs, capability tag.

On-disk layout after `tessl scenario download` (verbatim from their docs):

```
evals/
└── <7-char-hash>-<slug>/
    ├── task.md
    ├── criteria.json
    └── scenario.json
```

Scenarios are generated server-side from real artefacts — commits, PRs, or skill documentation:

> "Tessl analyses the commit diff and produces a task description (what an agent would be asked to do) and a scoring rubric (how to judge whether the agent did it correctly) — essentially reconstructing the intent of the original change as an agent task."

The internal prompts driving generation are not exposed; this is platform-provided opacity.

Mapping to vibevm: no equivalent at all. vibevm's `vibe check` does static spec linting; there is no concept of *executing* the spec against an agent and scoring the output.

### 2.6 Eval / Evaluation {#eval}

An eval is the act of running a scenario and comparing agent performance:

> "reviews check your skill against structural best practices, and scenario evals measure how much the skill improves agent performance on real tasks."

Two distinct eval types:

- **Skill review** — purely static / LLM-as-judge analysis of the skill artefact itself, scoring on three axes (validation / implementation / activation — see §2.11).
- **Scenario eval** — runs the scenario twice: once as a *baseline* without the skill loaded, once *with the skill injected*, then compares the resulting criterion scores to quantify the delta the skill provides.

Sample command shape:

```bash
tessl eval run ./my-skill --agent=claude:claude-sonnet-4-6 --label "baseline-vs-fix"
```

Multiple Claude model variants can be specified: `claude-sonnet-4-6` (default), `claude-opus-4-6`, `claude-sonnet-4-5`, `claude-opus-4-5`, `claude-haiku-4-5`.

Output structure documented per scenario:

- Individual criterion scores (checklist items with pass/fail/partial)
- Aggregate task completion score
- Comparison between baseline and with-context runs
- Cost analysis across different models

Mapping to vibevm: nothing. The `vibe-eval` crate does not exist today (and is not in the M1 / M1.5 roadmap).

### 2.7 PURL (Package URL) and `describes` linkage {#purl}

A tile can declare which upstream open-source package it documents:

```json
"describes": "pkg:pypi/fastapi@0.116.1"
```

The format is the [Package URL spec](https://github.com/package-url/purl-spec) — `pkg:<type>/<namespace>/<name>@<version>`, supporting `pkg:npm`, `pkg:pypi`, `pkg:cargo`, `pkg:gem`, `pkg:maven`, `pkg:docker`, `pkg:github`, etc.

When `describes` is set, the `docs` field is required.

This is what enables Tessl's headline marketing claim:

> "Tessl's registry indexes over 3,000 skills and hosts documentation for 10,000+ OSS packages, keeping agent context version-matched to your code and dependencies."

> "Teams using Tessl saw up to 3.3× improvement in correct API usage across open-source libraries"

The mechanism: when the agent works on FastAPI 0.116.1, Tessl serves docs version-matched to that exact version, so it cannot hallucinate APIs from stale training data.

Mapping to vibevm: vibevm has *capability* tags (`capability:wal-protocol`) and now (per PROP-003) *interface* tags (`interface:build-system`), but no equivalent of "this package documents an external upstream artifact at version X." This is a meaningful absence — vibevm packages today are entirely self-contained, with no formal way to declare "I am the spec-driven companion to FastAPI 0.116.1."

### 2.8 Workspace {#workspace}

A workspace is a multi-tenant collaboration boundary on Tessl's hosted registry:

> "A collaborative space where teams manage and share private context."

Three documented roles:

- **Member** — read-only access to workspace tiles; can install and use them.
- **Publisher** — can publish tiles (public or private) and run scenarios / evals.
- **Admin** — workspace management; can run `tessl workspace add-member --username <user> --role <role>`.

Tiles default to *private* on publish; making one *public* requires explicit `--public` and Tessl reviewer approval.

Mapping to vibevm: vibevm has no role model at all today. The `vibespecs` GitHub org is open-publish for any maintainer with push access; there is no hierarchy.

### 2.9 Context — the umbrella concept {#context}

"Context" is Tessl's umbrella term for everything an agent reads when working on a task — skills + rules + docs combined:

> "the collective guidance provided to agents through skills, documentation, and rules. The platform treats agent context as managed software with the same rigor you apply to your codebase."

The framing implies a lifecycle: **create context → evaluate context → distribute context → use context → measure context → improve context**. This is the marketing arc; the operational primitives that fulfil each stage are the CLI commands in §3.

### 2.10 Three context-delivery modes — the load-bearing distinction {#delivery-modes}

The most distinctive architectural choice in Tessl is the explicit three-mode delivery model:

| Type | When loaded | Mechanism |
|---|---|---|
| 📚 **Docs** | On-demand (**lazy pull**) | Agent calls `query_library_docs` MCP tool when relevant |
| 📋 **Rules** | Always (**eager push**) | Loaded into every agent session unconditionally |
| 🔧 **Skills** | When relevant (**lazy push**) | Agent matches `description` against current task and auto-loads matching skill |

In their words:

> "Skills use 'lazy push' — they're automatically loaded when relevant: Agent recognizes this matches the debug-api-endpoints skill. Skill is loaded with its step-by-step workflow."

> "The `description` in your frontmatter is critical for skill discovery — make it specific about when the skill should be used"

Skill activation hinges on the **description** field — there is no formal trigger DSL, just a natural-language description that the agent's own model evaluates against the current task / files / conversation. This makes activation quality directly proportional to description quality, which is why one of the three review-score axes (§2.11) is dedicated to scoring the description.

Mapping to vibevm: vibevm today has only one delivery mode — **eager-push of materialised files**. Every package's `files_written` is copied into the project tree at install time and stays there until uninstall. There is no concept of "this package's content is loaded only when the agent decides it's relevant" or "this package's content is queryable on demand." vibevm's PROP-003 §2.5 introduces context-based subskill activation and §2.5.2 lists LLM-inferred activation as a fourth channel, but the delivery mechanism is still file-system materialisation; nothing in vibevm streams content into a live agent context.

### 2.11 Review scoring rubric (three axes) {#review-rubric}

When `tessl skill review ./path` runs, the skill is scored on three components:

- **Validation Score** — purely static. Per their checks: "YAML frontmatter is valid, required fields present, metadata completeness, line count ≤500."
- **Implementation Score** — LLM-as-a-judge: "conciseness, actionability, workflow clarity, progressive disclosure." (They use Claude under the hood; not officially confirmed but inferred from infrastructure context.)
- **Activation Score** — LLM-as-a-judge on the frontmatter description: "specificity, completeness, trigger term quality, distinctiveness conflict risk."

Threshold language:

> "90%+ Review Score: Skill conforms well to best practices; 70-89%: Good skill, may have minor improvements needed; Below 70%: Likely needs work"

Exact aggregation formula and per-axis weights are *not* documented publicly (we asked the corpus directly; no answer). One of the validation checks they expose verbatim:

> "✔ skill_md_line_count - SKILL.md line count is 152 (<= 500)"

Mapping to vibevm: `vibe check` provides static linting (manifest validity, WAL freshness, boot directory, lockfile/disk consistency, REVIEW marker aging) but has no LLM-judge component, no scoring rubric, no quality threshold concept. Each finding is binary error/warning/info; there's no aggregate "this package is 87% production-ready" output.

### 2.12 `--optimize` auto-improvement loop {#optimize}

```bash
tessl skill review ./path --optimize --max-iterations 5 --yes
```

What it does:

> "Runs in a loop until the skill scores 100% or a set number of iterations has passed."

> "automatically makes the suggested changes and re-runs the review"

Default `--max-iterations 3`, hard cap 10. `--yes` opts out of confirmation per iteration so the loop is fully autonomous. This is meaningfully more aggressive than typical "linter with --fix" tooling because the LLM is empowered to edit the markdown itself — rewording descriptions, restructuring workflows, deleting redundant content — based on the activation/implementation rubric scores.

Mapping to vibevm: nothing equivalent. PROP-002 makes `vibe check --fix` deferred to v1+ (only landing once the dead-anchor / orphan-anchor checks come online), and the planned `--fix` is non-LLM (mechanical edits to safe-to-change patterns).

### 2.13 Codebase-readiness eval — eval from real commits {#codebase-readiness}

The most architecturally distinctive Tessl primitive. Instead of synthetic tests, scenarios are generated from the project's *actual git history*:

```bash
tessl scenario generate <org/repo> --commits=<sha1>,<sha2> --context="src/**/*.py"
tessl scenario generate <org/repo> --prs <pr-num1>,<pr-num2>
tessl repo select-commits <org/repo> --keyword=migration --since=2026-01-01 --count=20
```

The flow:

1. Operator picks a meaningful recent commit or PR (Tessl provides `tessl repo select-commits` to browse).
2. Tessl reads the diff, generates a `task.md` describing what an agent would have been asked to do, and a `criteria.json` scoring rubric reconstructed from what the diff actually changed.
3. The scenario is run twice (baseline / with-context) using `tessl eval run`.
4. Output shows per-criterion scoring + aggregate + cost analysis.

> "Instead of synthetic tests, you base evaluations on real work that's already been done."

Why this matters: it makes the eval *grounded in your domain* rather than in some generic skill-quality rubric. A team can quantify "our context configuration improves a Claude Sonnet agent from 47% to 89% on this class of changes," using their own past work as the oracle.

Mapping to vibevm: no equivalent. `vibe build` (planned in M1.5.3) will generate code from feat × stack pairs, but there is no measurement layer — no concept of "did the agent actually do this PR correctly with this context."

### 2.14 Multi-model A/B comparison {#multi-model}

Each `tessl eval run` invocation specifies one agent:

```bash
tessl eval run ./my-skill --agent=claude:claude-sonnet-4-6
tessl eval run ./my-skill --agent=claude:claude-opus-4-6
tessl eval run ./my-skill --agent=claude:claude-haiku-4-5
```

Tessl provides a meta-skill `tessl-labs/review-model-performance` that orchestrates "automatic side-by-side comparison and gap diagnosis" across model variants in one guided flow (invoked as `/eval-improve`).

Mapping to vibevm: planned in M1.5.1 (LLM provider abstraction supporting Anthropic + OpenAI + OpenRouter + Ollama), but no eval / comparison surface is in scope today.

### 2.15 MCP server and `query_library_docs` {#mcp}

Tessl exposes itself to agents through Model Context Protocol. On `tessl init`, Tessl auto-detects the agent in use:

> "auto-detects Claude Code, Cursor, Gemini, Codex, Copilot CLI, Copilot in VSCode"

…and writes the MCP configuration so the agent's startup picks up Tessl as a context provider. The MCP server exposes at minimum one tool:

> "`query_library_docs` — read tiles from `tessl.json` and use it as context when generating code"

The agent calls this tool at its discretion, which is the core mechanism behind the *lazy pull* model. Skills, by contrast, are auto-loaded when the agent's own model decides the task description matches a skill's `description` field.

Mapping to vibevm: there is no `vibe-mcp` crate today. Agent integration is purely file-system-side — vibevm writes `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and `spec/boot/*.md`, and the agent reads them at session start. No live query path; no on-demand context loading; no protocol-level integration.

### 2.16 Snyk-driven security gating {#snyk}

When skills are published or installed:

- Snyk scans for vulnerabilities at publish time.
- Critical/high findings *don't block* installation, but the CLI prompts before continuing.
- `--dangerously-ignore-security` flag suppresses the prompt for automated workflows.
- Version-pinned install supports `tessl install <pkg>@<commit-sha>` for deterministic re-installation of the exact scanned bytes.

Mapping to vibevm: no security-scan layer at all. PROP-002 §2.1 enforces `content_hash` integrity (force-pushes are caught), but there is no upstream-vulnerability check before install.

### 2.17 Auto-update {#auto-update}

The Tessl CLI auto-updates itself in the background:

- Default check interval: 180 minutes (3 hours), configurable via `TESSL_AUTO_UPDATE_INTERVAL_MINUTES`.
- `0` disables.
- Updates happen at command exit, so commands are not interrupted.
- Disabled in CI environments automatically.
- Logs at `~/.tessl/auto-update.log`.

Files touched on update: `~/.tessl/` config + binary at `~/.local/bin/tessl` (POSIX) or equivalent on Windows.

Mapping to vibevm: no auto-update. `vibe` is treated as a static binary the user manages.

### 2.18 GitHub / GitLab integration {#git-integration}

Tessl integrates with both GitHub and GitLab:

- Workspace-level connection through OAuth (via "workspace settings").
- Integration enables distributing context tiles to repositories automatically — *exact mechanism not publicly documented*.
- The index references "GitHub Actions review, linting, and publishing" but the corpus does not contain workflow YAML or action names. This appears to be a feature that exists but is not currently in the public docs.

Mapping to vibevm: vibevm uses `[[registry]] = "https://github.com/vibespecs"` as a registry; the connection is plain `git fetch`. No OAuth wiring, no workspace-level integration. PROP-002 §6 sketches future supply-chain attestation but it's M2-territory.

### 2.19 Progressive enhancement: spec-driven development skill {#spec-driven}

Tessl publishes a meta-skill `tessl-labs/spec-driven-development` that *is itself a skill teaching agents the spec-driven flow*:

> "The agent interviews you: What endpoints do you need? How should authentication work? What happens when a resource isn't found?"

Process: agent asks clarifying questions → writes specification documents → user-review checkpoint → implementation → verification. This closes what they call the "intent-to-code chasm."

Mapping to vibevm: vibevm is *fundamentally* a spec-driven tool — `flow:wal`, `flow:atomic-commits`, `flow:sync-from-code` all encode similar disciplines. The conceptual overlap is significant; the difference is that vibevm encodes these as *first-class packages* in its own registry, while Tessl encodes one as a *meta-skill* on top of its general skill machinery.

### 2.20 `tessl outdated` and update workflows {#outdated}

```bash
tessl outdated   # show installed tiles with available upstream updates
```

Output format is not documented in detail, but the function is the obvious cargo / npm equivalent: "what's installed, what's the latest available."

Mapping to vibevm: no `vibe outdated` command. `vibe update --all` blindly re-resolves; no read-only "what would change" preview at the registry level.

## 3. CLI surface — complete documented inventory {#cli}

Captured verbatim from the docs corpus (some commands are reconstructed from usage examples since not all are exhaustively listed; flagged with [inferred] where applicable):

### 3.1 Authentication and setup

- `tessl login` — device-code auth flow.
- `tessl logout` — clears credentials.
- `tessl whoami` — auth status check.
- `tessl init` — initializes Tessl in a project; writes `tessl.json`; auto-detects the coding agent in use and configures MCP.
- `tessl cli update` — updates the CLI to latest.
- `tessl cli update --target <version>` — updates to specific version.

### 3.2 Package management

- `tessl install <package>` — installs to `.tessl/tiles/`.
- `tessl install --global <package>` — installs to `~/.tessl/`.
- `tessl install file:./path` — installs from local filesystem.
- `tessl install <github-url> --skill <name>` — installs specific skill from a GitHub repo. Multiple `--skill` flags supported (e.g. `--skill pdf --skill pptx`).
- `tessl install <github-url>/tree/<branch>/<path>` — install from specific branch / subtree.
- `tessl install <pkg>@<version>` — install pinned semver version.
- `tessl install <pkg>@<commit-sha>` — install pinned commit (used for reproducing exactly-scanned versions).
- `tessl install <pkg> --dangerously-ignore-security` — bypass Snyk-finding prompt.
- `tessl uninstall <package>` / `--global`
- `tessl list` — list installed tiles and skills.
- `tessl outdated` — list installed tiles with available updates.
- `tessl search <query>` — registry search.
- `tessl search` — interactive search.

### 3.3 Skill management

- `tessl skill new` — interactive wizard.
- `tessl skill new --name <n> --description <d> --workspace <w> --path <p>` — non-interactive.
- `tessl skill import ./path` — import existing skill from local dir.
- `tessl skill import ./path --workspace <w> --public` — import and publish public.
- `tessl skill lint ./path` — structural validation.
- `tessl skill review ./path` — quality review with three-axis scoring.
- `tessl skill review ./path --optimize` — auto-improvement loop.
- `tessl skill review --optimize --yes ./path` — non-interactive auto-improvement.
- `tessl skill review --optimize --max-iterations <n> ./path` — bound iterations (max 10).
- `tessl skill publish ./path --workspace <w>` — publish private.
- `tessl skill publish ./path --workspace <w> --public` — publish public (requires reviewer approval).

### 3.4 Tile management

- `tessl tile new --name <n> --path <p>` — scaffold new tile.
- `tessl tile lint ./path` — validate tile structure.
- `tessl tile pack --output <dir> ./path` — package tile for distribution.
- `tessl tile publish ./path --workspace <w>` — publish to registry.
- `tessl tile publish ./path --skip-evals` — skip evaluation gate at publish.

### 3.5 Scenario management

- `tessl scenario generate <path/to/tile> --count=<n> --workspace=<w>` — generate scenarios from a skill.
- `tessl scenario generate <org/repo> --commits=<sha1>,<sha2>` — from commit diffs.
- `tessl scenario generate <org/repo> --prs <num1>,<num2>` — from PR diffs.
- `tessl scenario generate <org/repo> --commits=<...> --context="<globs>"` — narrow context to a subset of paths.
- `tessl scenario list [--mine]` — recent generations (optionally only your own).
- `tessl scenario view <id>` / `--last` — inspect a generation.
- `tessl scenario download <id>` / `--last` — download to disk.
- `tessl scenario download --output <dir> <id>` — custom output location.
- `tessl scenario download --strategy <merge|replace> <id>` — merge with existing or replace.

### 3.6 Eval management

- `tessl eval run ./my-skill` — run skill evaluation.
- `tessl eval run ./my-skill --agent=claude:<model>` — specify model variant.
- `tessl eval run ./my-skill --label "<text>"` — annotate the run.
- `tessl eval run ./evals/ --workspace=<w>` — codebase-readiness eval against a directory of scenarios.
- `tessl eval run ./evals/ --context-pattern="<globs>"` — override context patterns at run time.
- `tessl eval run ./evals/ --context-ref=<sha>` — pin commit reference.
- `tessl eval list` — list runs with status.
- `tessl eval view <id>` / `--last` / `--json` — inspect a run.
- `tessl eval retry <id>` — rerun a failed eval.

### 3.7 Repository analysis

- `tessl repo select-commits <org/repo>` — interactive commit browser.
- `tessl repo select-commits <org/repo> --keyword=<term>` — filter by message keyword.
- `tessl repo select-commits <org/repo> --author="<name>"` — filter by author.
- `tessl repo select-commits <org/repo> --since=<YYYY-MM-DD>` — date filter.
- `tessl repo select-commits <org/repo> --count=<n>` — limit commits shown.

### 3.8 Workspace management

- `tessl workspace create <name>`
- `tessl workspace add-member --username <u> --role <r> --workspace <w>`

## 4. File formats and on-disk shapes {#formats}

### 4.1 `tile.json` (canonical from docs)

```json
{
  "name": "myworkspace/database-migration-helper",
  "version": "1.0.0",
  "summary": "Helper for managing database migrations",
  "docs": "docs/index.md",
  "describes": "pkg:pypi/fastapi@0.116.1",
  "private": true,
  "skills": {
    "database-migration-helper": {
      "path": "SKILL.md"
    }
  }
}
```

Required fields: `name` (workspace/tile-name format), `version` (semver). Optional: `summary`, `docs` (path), `describes` (PURL — when set, `docs` becomes required), `private` (default `true`), `skills` (mapping of skill-name → {path: SKILL.md path}).

### 4.2 `tessl.json` (project manifest)

```json
{
  "name": "my-project",
  "dependencies": {
    "tessl/pypi-fastapi": {
      "version": "0.116.0"
    }
  }
}
```

Project-level manifest tracking installed tiles. Equivalent of `package.json` / `Cargo.toml` in shape and role.

### 4.3 `SKILL.md`

```markdown
---
name: database-migration-helper
description: When you need to create and manage database migrations.
---

# Database Migration Helper

[procedural workflow content, max 500 lines]
```

Required frontmatter: `name`, `description`.

### 4.4 Scenario directory layout

```
evals/
└── <7-char-hash>-<slug>/
    ├── task.md              # task brief presented to agent
    ├── criteria.json        # weighted checklist rubric
    └── scenario.json        # fixture: repo URL, commit ref, context globs
```

Field-level details for `scenario.json` and `criteria.json` are not exhaustively documented (we asked; the docs do not enumerate). The corpus describes them as "fixture with repo URL, commit ref, and context exclude patterns" and "weighted checklist rubric" respectively.

### 4.5 Tile package layout

```
tile/
├── tile.json
├── docs/
│   └── index.md
├── skills/
│   ├── skill-1/
│   │   └── SKILL.md
│   └── skill-2/
│       └── SKILL.md
└── rules/
    └── coding-standards.md
```

### 4.6 `.tessl/` directory (project-local)

```
.tessl/
└── tiles/
    └── <workspace>/
        └── <tile-name>/
            └── [tile contents, expanded from registry tarball]
```

Plus `.tessl/auto-update.log` for the auto-update mechanism.

### 4.7 `~/.tessl/` directory (user-level)

Used for global package installations and credential storage. Exact subdirectory layout not documented in detail.

### 4.8 Environment variables

- `TESSL_AUTO_UPDATE_INTERVAL_MINUTES` — auto-update frequency. Default 180. Set to `0` to disable.
- `TESSL_TOKEN` — alternative to `tessl login` for headless / CI flows.

## 5. The gap — what Tessl does that vibevm does not {#gaps}

In rough descending order of impact on vibevm's strategic position. Each item identifies the Tessl capability, summarises what would need to land in vibevm to match it, and estimates the architectural slice.

### 5.1 MCP server / live agent context provider

**Tessl capability.** A `query_library_docs` MCP tool exposes installed tiles to agents at runtime. The agent decides what to load and when, based on the current task. Documentation never sits in the agent's context unless asked for. This is what makes "10K+ documented packages" tractable — only the relevant slice loads at any moment.

**vibevm gap.** Today vibevm is a file-system-side tool. Spec content is materialised at install time and lives in `spec/`. The agent reads everything that's there, all the time. There is no way to tell the agent "you have access to package X but it doesn't load until you ask."

**Slice.** A new `vibe-mcp` crate exposing an MCP server (well-defined Anthropic spec at [`https://modelcontextprotocol.io`](https://modelcontextprotocol.io)). Initial tools to expose:

- `query_package(name, version?)` — return package metadata + summary + capability list.
- `read_subskill(package, subskill_path, language?)` — fetch a specific subskill's content (composes nicely with PROP-003 §2.5).
- `list_capabilities(query?)` — discover capabilities and interfaces in the project's effective spec.
- `materialise_subskill(package, subskill_path)` — write subskill content to project tree on demand (composes with PROP-003 §2.5.2 LLM-inferred activation).

`vibe init` would write the appropriate MCP config block into Claude Code's `.claude/settings.json` (per [`https://docs.claude.com/en/docs/claude-code/mcp`](https://docs.claude.com/en/docs/claude-code/mcp)) and analogous configs for other agents (Cursor, Gemini, Codex). The hardest part is not the protocol — it is choosing what tools to expose and how subskill on-demand materialisation interacts with the lockfile.

**Estimated effort.** 2-3 weekends. Maps cleanly to a new crate; no breaking changes to existing crates.

**Priority.** **HIGH**. This is the single largest gap and the one that most strategically positions vibevm as a "Claude-native" tool. Maps to roadmap entry **M1.7** (see §6).

### 5.2 Quality evaluation framework — review + scenario evals

**Tessl capability.** Two evaluation modes:

- **Skill review** — three-axis static / LLM-judge scoring (validation / implementation / activation), with a `--optimize` loop that auto-edits the skill until it scores 100%.
- **Scenario eval** — runs the skill against generated test scenarios twice (baseline / with-context), measures the delta in agent task completion.

**vibevm gap.** `vibe check` covers structural lints; that's it. There is no concept of "is this package good," no LLM-judge component, no aggregate quality score, no auto-fix loop, no measurement of whether a package actually improves agent performance.

**Slice.** Two new crates and a new top-level command:

- `vibe-eval` crate — the eval engine. Static checks first (line-count, frontmatter validity, description specificity heuristics — non-LLM). Then `vibe-eval` upgrades with an LLM-judge mode once M1.5 lands the LLM provider abstraction. Scenario evals come last because they require the LLM tool-use loop.
- `vibe review <pkgref>` — runs the static review. Outputs a 0-100 score per axis + aggregate.
- `vibe review <pkgref> --optimize` — gated behind LLM availability. Auto-edits the package's source files until scores converge. **This needs careful UX** because vibevm packages are spec-content, not single-file skills — the auto-editor needs to understand multi-file boundaries.
- `vibe scenario generate <repo> --commits=<...>` — lifts the Tessl pattern for vibevm's own context. Reads commits via the `vibe-registry` git backend, produces scenarios that go into a `evals/` directory analogous to Tessl's.
- `vibe eval run <pkgref> --scenarios=evals/` — runs scenarios twice, scores, reports.

**Estimated effort.** Static review: 1 weekend. LLM-judge mode: 2 weekends (depends on M1.5.1 LLM abstraction). Scenario gen + eval run: 4-6 weekends (deepest part — needs scoring rubric format, cost reporting, multi-model orchestration).

**Priority.** **HIGH** for the static review surface (cheap, immediate value, no LLM dependency). **MEDIUM** for the scenario-eval framework (depends on M1.5). Maps to roadmap entries **M1.8** (review) and **M2.7** (scenario evals).

### 5.3 PURL `describes` linkage to upstream packages

**Tessl capability.** A tile can declare `describes = "pkg:pypi/fastapi@0.116.1"` — the tile documents that exact upstream package version. Enables their headline marketing claim of "version-matched documentation for 10K+ OSS packages" and the 3.3× API-correctness improvement.

**vibevm gap.** vibevm packages have no notion of "I document an external upstream artefact." `flow:wal` is its own thing, version-incremented on its own merits. There's no way to author `feat:fastapi-app-skeleton` and bind it to FastAPI 0.116.1 such that consumers who use a different FastAPI version see a clear version-mismatch warning.

**Slice.** A small, focused PROP. `vibe-package.toml` gains an optional field:

```toml
[package]
describes = "pkg:pypi/fastapi@0.116.1"
```

`vibe-core` parses PURLs (a small parser; spec at [`https://github.com/package-url/purl-spec`](https://github.com/package-url/purl-spec)). `vibe outdated --upstream` (see 5.13) flags packages whose `describes` upstream has shipped a new version. `vibe install` does not enforce upstream-version matching by default (the consumer's project may legitimately be on a different version), but `vibe check` gains an optional warning when project-declared upstreams don't match.

**Estimated effort.** 1 weekend (manifest field + parser + lockfile field + check).

**Priority.** **MEDIUM**. The value is real but unlocks slowly — it needs the registry to actually grow library-companion packages. Worth landing the field early so the option exists. Maps to roadmap entry **M1.9**.

### 5.4 Three-mode delivery: lazy-pull docs / eager-push rules / lazy-push skills

**Tessl capability.** Three different ways content reaches the agent, each with appropriate semantics for its content type. Rules always loaded; skills loaded by description match; docs loaded only on agent's explicit query.

**vibevm gap.** Eager-push of materialised files is the only mode. Every package's content sits in `spec/` permanently after install.

**Slice.** Composes with §5.1 (MCP) and PROP-003 (subskills). The mapping:

- vibevm's `spec/boot/*` files are *eager push* (always-loaded rules-of-the-house) — already correct.
- vibevm's `spec/flows|feats|stacks/...` files are *eager push* in current shape — Tessl-style we'd want them to be *lazy push* via skill-style description matching, not auto-materialised everywhere.
- vibevm's library docs (new content type, not yet present) would be *lazy pull* via the `vibe-mcp` `query_package_docs` tool.

This is a subskill-system extension. Each subskill (or top-level package content unit) declares a `delivery` mode in its manifest:

```toml
[delivery]
mode = "eager"     # always materialised — current default
mode = "lazy-push" # auto-materialised when agent matches description; trigger lives in `description`
mode = "lazy-pull" # never materialised; fetched only via `vibe-mcp` query_subskill
```

`vibe-mcp` enforces the mode at install / query time. The lockfile records the resolved mode per subskill.

**Estimated effort.** 2-3 weekends, mostly in `vibe-mcp` + lockfile schema bump.

**Priority.** **HIGH** *but* gated on §5.1 landing first. Maps to roadmap entry **M2.8**.

### 5.5 Codebase-readiness eval — scenarios from real commits

**Tessl capability.** Scenarios are generated from a project's actual git history. `tessl scenario generate <repo> --commits=<...>` reads the commit diff, generates a task brief and scoring rubric, runs an agent against the project at the pre-commit state, and scores.

**vibevm gap.** No eval framework at all today. Even after §5.2 lands, scenario generation from git history is its own slice.

**Slice.** Composes with §5.2's `vibe-eval` crate. Scenario generation reads commit diffs through the `vibe-registry::ShellGit` backend, composes a generation prompt to the LLM, gets back `{task.md, criteria.json, scenario.json}`. Pin the format to be drop-in compatible with Tessl's so cross-tooling is possible.

**Estimated effort.** 2 weekends. Hard part is making the generated rubrics actually meaningful for vibevm-shaped artefacts (specs, not code).

**Priority.** **MEDIUM**. Lands in **M2.9**.

### 5.6 Multi-model A/B comparison

**Tessl capability.** `--agent=claude:claude-sonnet-4-6` etc.; meta-skill orchestrates side-by-side runs across model variants in one flow.

**vibevm gap.** M1.5.1 plans LLM provider abstraction; multi-model comparison is not in scope.

**Slice.** Once `vibe-llm` exists with the provider trait, add `vibe eval run --agents=<model1>,<model2>,<model3>` that runs the same scenario per model, captures cost / latency / score per run, and renders a comparison table.

**Estimated effort.** 1 weekend on top of M1.5.1. Composes naturally.

**Priority.** **MEDIUM**. Lands in **M2.7**.

### 5.7 Agent auto-detection at `vibe init`

**Tessl capability.** `tessl init` detects which coding agent is in use (Claude Code, Cursor, Gemini, Codex, Copilot CLI/VSCode) and writes appropriate config for each.

**vibevm gap.** `vibe init` writes the same files (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`) regardless of which agent is in use. There is no detection.

**Slice.** Detection probes:
- Claude Code: presence of `~/.claude/` directory or `claude` binary on PATH.
- Cursor: presence of `.cursor/` in any ancestor directory.
- Gemini: presence of `.gemini/` or `gemini` CLI.
- Codex / Copilot: VSCode extension fingerprints.

Per detected agent, write the corresponding agent-specific instruction file plus the appropriate MCP server config (composes with §5.1).

**Estimated effort.** 1 weekend. Works without §5.1 (just writes appropriate instruction files) but synergistic with it.

**Priority.** **LOW** when §5.1 hasn't landed. **MEDIUM** once it does — auto-detect determines which MCP-config gets written.

### 5.8 Workspace + role-based access control

**Tessl capability.** Workspaces with Member / Publisher / Admin roles; private-by-default tiles; public publication gated by Tessl reviewer approval.

**vibevm gap.** No role model. The `vibespecs` GitHub org has open-publish for any maintainer with push access. No fine-grained access control beyond standard GitHub permissions.

**Slice.** Two layers:
- **GitHub-side**: Use standard GitHub teams + branch protection rules. Already supported by GitHub; vibevm need not implement anything.
- **vibevm-side**: A `[[workspace]]` block in `vibe.toml` declaring role-aware policy at the project level. `vibe registry publish` consults this. This is more about *advertising* the role model than enforcing — registry-side enforcement is delegated to GitHub.

**Estimated effort.** Small if relying on GitHub primitives (~1 weekend); larger if implementing vibevm-native role enforcement.

**Priority.** **LOW** for v1. Standard GitHub mechanisms are sufficient. Document the recommendation in `docs/authoring-{flow,feat,stack}.md` and move on.

### 5.9 Snyk-style security scanning

**Tessl capability.** Snyk vulnerability scanning at publish time; install-side prompt on critical/high findings; `--dangerously-ignore-security` bypass.

**vibevm gap.** No security-scan layer. PROP-002 §2.1's `content_hash` integrity check catches force-pushed bytes but not upstream vulnerabilities.

**Slice.** Several open questions:
- What does a "vulnerability" mean for a spec-content package? It's not arbitrary code; it's text + hint files. The threat surface is *prompt injection / data exfiltration via subtle wording* — different from standard CVE-style vulnerabilities.
- A novel research direction: an LLM-judge "is this package attempting prompt injection" check at install time, with a prompt-shape similar to constitutional AI's harm-detection.

**Estimated effort.** Unclear — needs a research slice first to define the threat model. Not actionable as a single slice today.

**Priority.** **LOW** for v1. Park as M3+ research. Maps to roadmap entry **M3.1** (research-only).

### 5.10 Skill / package auto-update

**Tessl capability.** Background auto-update with configurable interval, CI-aware disable, log file.

**vibevm gap.** No auto-update. User-driven `vibe update` only.

**Slice.** Trivial in shape but high-friction: a daemon-style background process. POSIX cron / Windows Task Scheduler integration. Lockfile may change without explicit user action — surprise factor matters here, esp. for a tool whose value proposition is *audit trail*.

**Priority.** **LOW**. Tessl markets this as DX polish, but for a spec-driven tool where every install is meant to be deliberated, silent updates may be the wrong default. Worth a one-time `vibe update --check` cron-friendly command, but not a daemon.

### 5.11 PR-time review integration (GitHub Actions)

**Tessl capability.** "GitHub Actions review, linting, and publishing" mentioned in their index but not detailed in the public corpus. From context, this is plausibly something like: `tessl-action-review` runs on PRs that touch `tile.json` or `SKILL.md`, posts a review comment with the score, blocks merge below 70%.

**vibevm gap.** No CI / GitHub Actions integration shipped today.

**Slice.** Once §5.2 lands, ship two GitHub Actions:
- `vibevm-action-check` — runs `vibe check --json` on PR diffs touching `vibe-package.toml` or spec content.
- `vibevm-action-review` (post-§5.2) — runs `vibe review --json` on changed packages, posts as a PR comment.

Hosting under `vibespecs/actions/` keeps the existing decentralised model.

**Estimated effort.** 1 weekend per action.

**Priority.** **MEDIUM** — wait for §5.2's static review surface to exist, then ship.

### 5.12 Tile / skill discoverability — registry search beyond `vibe install`

**Tessl capability.** `tessl search <query>` queries name + description; registry UI has filters.

**vibevm gap.** No `vibe search`. The user has to know what package they want. With ~3 demo packages today this is fine; with 100+ it won't be.

**Slice.** Smallest viable: `vibe search <q>` walks all configured `[[registry]]` URLs, lists packages whose `vibe-package.toml` `description` matches. Not real index — just `git ls-remote` + `vibe-package.toml` fetch per repo. Slow but correct. Indexing can come later.

**Estimated effort.** 1 weekend.

**Priority.** **MEDIUM**. Tracks adoption: useful at 20+ packages, essential at 100+.

### 5.13 `vibe outdated` — read-only "what's newer" preview

**Tessl capability.** `tessl outdated` lists installed tiles with available upstream updates. Standard cargo / npm equivalent.

**vibevm gap.** `vibe update --all` exists but blindly re-resolves; no preview-only mode.

**Slice.** `vibe outdated [--upstream]` walks the lockfile, calls `MultiRegistryResolver::list_versions` per package, renders a table:

```
Package              Current  Latest  Status
flow:wal             0.1.0    0.2.1   update available
flow:atomic-commits  0.1.0    0.1.0   up to date
```

`--upstream` checks `describes` PURL targets too (composes with §5.3).

**Estimated effort.** 1 weekend.

**Priority.** **MEDIUM**. Cheap, immediate UX win. Maps to roadmap entry **M1.10**.

### 5.14 Spec-driven-development as a meta-skill

**Tessl capability.** `tessl-labs/spec-driven-development` is itself a skill teaching agents the spec-driven workflow.

**vibevm gap.** None — vibevm's *entire identity* is spec-driven development. The conceptual gap is not real, but the *packaging* is: vibevm could publish equivalent packages (`flow:spec-driven-design`, `flow:requirements-gathering`) as first-class registry content rather than implicit-in-the-tool.

**Slice.** Author + publish 2-3 spec-driven workflow flows. Authoring exercise, no code.

**Priority.** **LOW** — improves registry depth but not core capability. Lands organically as authoring practice grows.

## 6. Roadmap deltas {#roadmap-deltas}

The following entries should be added to `ROADMAP.md` between M1.6 (Multi-registry polish) and M2 (Production-readiness), with appropriate cross-references to PROP-002, PROP-003, and this PROP-004. Each is a milestone in its own right; the order below is the recommended priority.

### M1.7 — `vibe-mcp` server (Claude-native context provider)

Maps to §5.1.

- New `vibe-mcp` crate exposing an MCP server over stdio.
- Tools: `query_package`, `read_subskill`, `list_capabilities`, `materialise_subskill`.
- `vibe init` writes the appropriate MCP config to `.claude/settings.json`, `.cursor/mcp.json`, etc., based on agent detection (composes with M1.11 below).
- New manual smoke `manual-tests/M1.7-mcp-claude-code-smoke.md` walking a full Claude Code → MCP → vibevm round-trip.

### M1.8 — `vibe review` static quality scoring

Maps to §5.2 (static portion only).

- New `vibe-eval` crate with non-LLM checks: line-count, frontmatter validity, description specificity heuristics, capability/interface declaration completeness.
- `vibe review <pkgref>` command outputs a 0-100 score per axis (validation / implementation / activation, mirrored from Tessl's three-axis rubric).
- Threshold conventions: 90%+ ready for publish, 70-89% ship-with-warnings, <70% blocks publish unless `--accept-low-quality`.
- `vibe review --json` for CI consumption.

### M1.9 — `describes` PURL linkage

Maps to §5.3.

- Optional `[package].describes` field accepting PURL syntax.
- PURL parser in `vibe-core` (small; the spec is fixed).
- Lockfile records the upstream PURL.
- `vibe check` warns when the project's effective dep tree includes a `describes` package whose upstream version doesn't match what the project declares (tracked through some new `[upstream]` block in `vibe.toml` or inferred from sibling manifests).

### M1.10 — `vibe outdated`

Maps to §5.13.

- Read-only preview command.
- `--json` output for CI.
- `--upstream` mode using §5.3's `describes` field.

### M1.11 — Agent auto-detection at `vibe init`

Maps to §5.7.

- Probes for Claude Code, Cursor, Gemini, Codex, Copilot.
- Writes per-agent config files appropriately.
- Composes with M1.7 to write MCP config for detected agent.
- Falls back to writing all three (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) when no agent is detected.

### M2.7 — `vibe review --optimize` and multi-model comparison

Maps to §5.2 (LLM-judge portion) + §5.6.

- Requires M1.5.1 LLM provider abstraction.
- LLM-judge mode for the implementation / activation review axes.
- `vibe review --optimize` auto-edit loop (analogous to `tessl skill review --optimize`).
- `vibe eval run --agents=<m1>,<m2>` — run scenarios per model, render comparison.

### M2.8 — Three-mode delivery (eager / lazy-push / lazy-pull)

Maps to §5.4.

- Subskill manifest gains `delivery = "eager" | "lazy-push" | "lazy-pull"`.
- `vibe-mcp` enforces lazy-pull by serving on demand; `vibe-install` materialises eager + lazy-push (the latter only when the agent's task description matches the package's `description`).
- Lockfile schema bump to record per-subskill delivery mode.

### M2.9 — Scenario generation from real commits

Maps to §5.5.

- `vibe scenario generate <repo> --commits=<...>` reads diffs via `vibe-registry::ShellGit`, composes generation prompts, produces `{task.md, criteria.json, scenario.json}` triples in `evals/`.
- Format pinned to be drop-in compatible with Tessl's scenario layout for cross-tooling.

### M2.10 — `vibe search` registry inspector

Maps to §5.12.

- Walks configured `[[registry]]` URLs.
- Naive search: registry → enumerate repos → fetch `vibe-package.toml` per match → filter by description regex.
- Caches results in `~/.vibe/search-cache/`.

### M3.1 — Security review (research)

Maps to §5.9.

- *Research only.* Define the threat model for spec-content packages (prompt injection, data-exfiltration via wording, capability misrepresentation).
- LLM-as-judge harm-detection prompt.
- Not action-eligible until threat model is defined.

### Side quest — author spec-driven-development meta-flows

Maps to §5.14.

- Author and publish `flow:spec-driven-design`, `flow:requirements-gathering`, `flow:design-review`.
- No code work.

## 7. Source URLs and re-fetch procedure {#sources}

The Tessl product surface is on a rolling release schedule. To refresh this research:

### 7.1 Canonical primary sources (the LLM-targeted docs)

- **`https://docs.tessl.io/llms.txt`** — high-level documentation index. Best entry-point for inventory checks. Updated whenever they ship doc changes.
- **`https://docs.tessl.io/llms-full.txt`** — concatenated full corpus. This is what we used for verbatim quotes in this document. Largest single-source.
- **`https://docs.tessl.io/sitemap.md`** — full URL listing, useful for discovering pages not in `llms.txt`.

### 7.2 Marketing / overview pages

- **`https://www.tessl.io`** — top-level marketing site. The "3.3× improvement" claim and the "10K+ packages indexed" stats are sourced here.
- **`https://docs.tessl.io/concepts/what-is-tessl`** — pitch and problem statement (currently 404s with redirect to home; check redirect targets when refetching).
- **`https://docs.tessl.io/concepts/how-tessl-works`** — context delivery model, lazy push / pull / eager push table.
- **`https://docs.tessl.io/getting-started/quickstart`** — install + workflow.
- **`https://docs.tessl.io/skills/creating-skills`** — `SKILL.md` format, frontmatter requirements.
- **`https://docs.tessl.io/skills/skill-evaluation`** — review rubric details.
- **`https://docs.tessl.io/evals/scenario-generation`** — scenario format.
- **`https://docs.tessl.io/reference/cli-commands`** — CLI inventory.
- **`https://docs.tessl.io/reference/configuration`** — file format details (`tile.json` / `tessl.json`).
- **`https://docs.tessl.io/reference/glossary`** — defined terms.
- **`https://docs.tessl.io/reference/changelog`** — version history. Capture the version number when refetching so any deltas have a reference point.

### 7.3 Indirect / context sources

- **`https://github.com/anthropics/skills`** — Anthropic's published skills repo, mentioned in Tessl docs as an installable source. Useful background on the broader skills-ecosystem shape.
- **`https://github.com/package-url/purl-spec`** — Package URL spec, the basis for Tessl's `describes` field and our M1.9 entry.
- **`https://modelcontextprotocol.io`** — MCP specification, target of M1.7's `vibe-mcp` crate.
- **`https://docs.claude.com/en/docs/claude-code/mcp`** — Claude Code's MCP integration docs, target for `vibe init`'s config-writing step.

### 7.4 Capture date and version

- **Capture date:** 2026-05-04.
- **Tessl CLI version visible in changelog:** 0.78.0.
- **Materials retrieved:** `llms.txt` index + `llms-full.txt` corpus (queried with multiple targeted prompts to extract verbatim passages on each subsystem).

When refreshing this PROP, append a new `### 7.5 Refresh history` block recording: refresh date, capture-time Tessl version, what changed materially since last capture, what roadmap deltas need updating. Don't rewrite the existing inventory in place — keep it as a historical baseline.

## 8. Where vibevm leads — Tessl gaps {#vibevm-leads}

The opposite direction. These are decisions in vibevm's architecture that Tessl either has not made, has made differently in a way we believe is wrong, or has not yet exposed publicly.

### 8.1 Decentralised git-native registry

PROP-002's `[[registry]]` / `[[mirror]]` / `[[override]]` model is git-as-registry. Anyone hosts their own org. Mirrors are transparent. Tessl's registry is centralised on their hosted service — there is no documented mechanism for a team to run their own registry endpoint. This is an anti-pattern for vibevm's audience (regulated, air-gapped, sovereignty-conscious teams).

### 8.2 Content-hashed identity

PROP-002 §2.1 makes `content_hash` part of identity. A force-pushed tag is *caught* on next install. Tessl's `tessl install <pkg>@<commit-sha>` provides version-pinning but no integrity check at install time — the commit SHA is trusted on faith.

### 8.3 SAT-class solver + cargo-features-style optional components + subskills

PROP-003 specifies a libsolv-backed SAT solver, cargo-tradition feature semantics, vibevm-native subskills with four orthogonal activation channels, and BCP-47 sidecar i18n. Tessl's tile model is *flat* — no features, no subskills, no conditional content, no language localisation. From their docs:

> "Skills are presented as atomic, indivisible units. The `tile.json` manifest maps skill names to file paths but shows no conditional logic or component-level configuration."

This is meaningful. Once vibevm packages exceed trivial complexity, the conditional-activation surface PROP-003 sketches becomes essential — and Tessl has not yet built it.

### 8.4 Strict provenance trail in the lockfile

vibevm's `vibe.lock` schema v2 carries `registry`, `source_url`, `source_ref`, `resolved_commit`, `content_hash`, `dependencies`, `overridden`, `boot_snippet`, `files_written` per package. Tessl's `tessl.json` carries name + version per dependency — no provenance, no integrity hash, no resolution metadata. This is a meaningful audit-trail gap.

### 8.5 Manual-test smoke protocol

PROP-000 §14 makes runnable smoke-tests in `manual-tests/` a first-class part of the spec lifecycle. Tessl appears to have nothing equivalent — their evaluation framework is the `tessl eval` engine, not authored markdown protocols you walk by hand against a live registry.

### 8.6 Token-secrecy invariant + redaction discipline

PROP-000 §20 and the publish-token loader's per-host file precedence + redaction in Display/Debug + URL-credential scrubbing in error messages give vibevm publish-side the kind of token-handling discipline that takes years to retrofit. Tessl's `TESSL_TOKEN` env var is documented; the surrounding redaction discipline isn't visible in their docs.

### 8.7 Self-host capability

vibevm's `tools/self-check.sh` and `vibe.toml` at the repo root let vibevm `vibe check` itself. Tessl, as a hosted product, doesn't dogfood in the same way (they have their own QA infrastructure, not visible to users).

### 8.8 Spec-corpus-as-runtime-input

vibevm packages contribute *content* to the project's effective spec corpus — the spec is what the agent reads at session start. Tessl's tiles deliver content too, but on a different model: skills are workflows the agent loads on-trigger, not foundational spec. The two are complementary; vibevm's continuous-corpus model is qualitatively different and arguably better for projects where the spec is *itself* the source of truth (vs Tessl's "spec is one of many inputs").

## 9. Reading list — adjacent projects and concepts {#reading}

For future context refresh, comparable systems worth tracking:

- **Anthropic's `claude-skills`** at <https://github.com/anthropics/skills> — Claude-native skills repository; some overlap with Tessl's `SKILL.md` format. Note this is the format Tessl piggybacks on.
- **Cursor `.cursorrules`** — IDE-specific rules format. Architecturally analogous to Tessl rules but tied to one editor.
- **Continue.dev `config.yaml`** — open-source agent-config model; older than Tessl, more limited surface.
- **GitHub `.github/copilot-instructions.md`** — Microsoft's standardised location for Copilot context. Standardisation play.
- **OpenAI Function calling / Tools** — the runtime mechanism agents use; Tessl's MCP work converges on a similar model.
- **Model Context Protocol** — <https://modelcontextprotocol.io> — Anthropic's open spec; vibevm's M1.7 target.

---

*This PROP is research only — it ratifies no implementation. Each §6 roadmap delta becomes its own PROP / FEAT when prioritised. When Tessl ships material changes, refresh this document via §7's procedure rather than rewriting it; the historical baseline has independent value.*
