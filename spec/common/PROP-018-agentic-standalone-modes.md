# PROP-018 — Agentic and standalone modes {#root}

**Status:** proposed 2026-06-16 — owner-requested design session. The MVP
slice this PROP authorises is named in §4; everything heavier is parked in
§6 (far backlog). This is the spec home for vibevm's *product modes* — a
new, cross-cutting concept, distinct from PROP-006's *session* postures
(see §1.3).
**Related:** [PROP-015](../modules/vibe-mcp/PROP-015-mcp-integration.md)
(the MCP server + agent-integration machinery — the `Agent` enum, the
per-agent config/skill writers, and `vibe mcp install` — that this PROP
reuses and extends), [`VIBEVM-SPEC.md` §3.2](../../VIBEVM-SPEC.md) (the
committed *CLI-first, agent-agnostic* posture and the
deterministic/probabilistic split this PROP formalises into modes),
[`VIBEVM-SPEC.md` §10.4](../../VIBEVM-SPEC.md) (the future `vibe-llm`
provider layer — the standalone built-in inference backend §2.2 names),
[PROP-006](PROP-006-operating-modes.md) (session operating postures — a
*different* concept, §1.3), [PROP-003 §2.5](../modules/vibe-resolver/PROP-003-dep-evolution.md)
(subskill *delivery* into the project tree — distinct from agent-skill
*projection*, §2.5), [PROP-017 §8](../modules/vibe-resolver/PROP-017-resolvo-resolver.md)
(a sibling far-backlog).

---

## 1. Motivation {#motivation}

### 1.1 The problem {#problem}

vibevm has commands that are pure algorithm (`install`, `check`, `list`)
and commands that genuinely need reasoning (explain, build, review). The
algorithmic ones already run from a bare terminal with no LLM
(`VIBEVM-SPEC.md` §3.2). The reasoning ones raise a question of *who does
the reasoning, and how*.

vibevm is almost always invoked *by* a coding agent (Claude Code,
OpenCode, Codex) that already holds a capable LLM, the live context, and
the tools. For reasoning work in that setting the agent is the right
**executor** — but vibevm is the right **author** of the task. vibevm
carries stable, algorithmic knowledge of its own domain (the spec-driven
model, the dependency and package rules, the discipline), so an
instruction it composes is more informative and more trustworthy than one
the agent would improvise from scratch. This PROP gives the two their
natural roles — vibevm composes the domain-grounded instruction, the agent
carries it out — and names the operating contexts so the codebase branches
on them cleanly. (vibevm can also reason with *no* agent present, via a
built-in `vibe-llm` engine — `VIBEVM-SPEC.md` §10.4, far-backlog §6 — but
that is standalone mode; it is not what makes agentic mode worthwhile.)

### 1.2 The two modes — one axis {#axis}

The modes are not two codebases. They are one question: **where does an
operation's reasoning happen?**

- **agentic** — vibevm is driven by a host agent during that agent's own
  work. For a step that needs reasoning, vibevm composes a domain-grounded
  instruction and *delegates it back* to the agent, which executes it on
  its LLM with the live context vibevm lacks. The split is by strength, not
  a workaround: the agent is the better executor in-session, vibevm the
  better author of the instruction. (Pure-algorithm operations still run
  directly — agentic is about the *reasoning* steps.)
- **standalone** — vibevm stands on its own. Reasoning runs on vibevm's
  *own* backend: algorithmic where the work allows, and — when `vibe-llm`
  lands — a built-in inference engine. Today the standalone backend has no
  LLM, so the only standalone functionality this PROP ships is the
  **non-reasoning** one: projecting skills into agents (§2.6).

The unifying statement (§2.1): **a mode is a choice of inference backend.**
Non-reasoning operations behave identically in both modes; reasoning
operations branch on the backend. This is the seam everything else hangs
off.

### 1.3 What this is NOT — PROP-006 {#not-prop-006}

[PROP-006](PROP-006-operating-modes.md) defines *session operating
postures*: codewords ("move fast and break things") that change **how an
agent behaves within one work session** (whether to pause for
confirmation, how freely to spend tokens). Those govern the *agent's*
conduct. PROP-018 modes govern **where vibevm gets inference** — a property
of *vibevm's* execution, orthogonal to any session posture. A session can
be in "move fast" posture while vibevm runs in agentic mode; the two never
collide. Do not overload one onto the other.

## 2. Decisions {#decisions}

### 2.1 A mode is a choice of inference backend {#mode-is-backend}

`req r1`

**Decision.** Mode is not a global flag the user sets; it is **inferred per
operation from how vibevm was reached and what backend is available.** An
operation declares whether it needs inference; if it does, the active
backend decides the realisation:

- reached as a **subprocess of an agent** (CLI one-shot or MCP call) →
  the **relay backend** (§2.7): delegate the intent back to that agent.
- reached **standalone** with a built-in engine available (future) → the
  **built-in backend**: run inference in-process via `vibe-llm`.
- reached **standalone** with no engine (today) → a reasoning operation
  **fails loud** with "this needs an inference backend; run me under an
  agent, or wait for the built-in engine," and a non-reasoning operation
  runs normally.

### 2.2 The pluggable inference backend {#pluggable-backend}

`req r2`

**Decision.** Inference sits behind one trait, `InferenceBackend`, so an
operation never names a provider. An operation that needs reasoning
constructs an `Intent` (a structured prompt + the inputs it needs) and
hands it to the active backend. Two backends are foreseen:

- **`RelayBackend`** (agent mode) — vibevm authors the `Intent` and
  *parks* it for the calling agent to execute, returning "delegated"
  (§2.7). Not a stopgap: in agent mode the agent is the right executor.
- **`BuiltinBackend`** (standalone mode; far backlog §6) — runs the
  `Intent` on `vibe-llm` in-process, for when no agent is present.

This is deliberately **not** over-built: the trait exists so reasoning
operations are written once against an abstract backend, and the standalone
engine slots in later without touching them. Operations with a single
natural home (skill-install is standalone-only; a "rewrite this spec
section" op may be agentic-only until the engine exists) simply do not
offer the other backend (§2.3).

### 2.3 Per-operation backend affinity {#affinity}

`req r2`

**Decision.** Each operation declares an **affinity**: `agentic-only`,
`standalone-only`, or `both`. Affinity is a property of the *work*, not a
user choice — scanning a manifest is `standalone-only` (pure algorithm,
needs no agent); a free-form "explain this project in prose" is
`agentic-only` until the built-in engine exists; a task expressible either
as a deterministic pass or as reasoning is `both`. The dispatcher refuses
an operation invoked through a backend it has no affinity for, with a
message naming the right one.

### 2.4 Agent-installable artifacts are declared separately from the package kind {#skill-decl}

`req r3`

**Decision.** A package declares which of its files are **skills** for
agents in a dedicated manifest section — **not** by introducing a
package kind of its own. The kind register (`package_ref.rs`,
`VIBEVM-SPEC.md` §4.1) stays closed to skills. Rationale: skills can
live inside a package of *any* kind and be structured any way. A `tool`
package `vim` can ship the tool itself **plus** a skill for driving vim
— one self-contained package, two artefact classes. Kind answers "what
is this package"; the new section answers "what does it project into an
agent." (This unit's original text sketched MCP servers as a second
any-kind section; that half is SUPERSEDED — MCP servers became their own
`mcp` kind with their own laws, owner resolution 2026-07-07:
[PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md). The skill law
here is unchanged.)

The MVP section is an array-of-tables, matching the manifest's existing
`[[requires_any]]` / `[[registry]]` / `[[mirror]]` shape:

```toml
[[skill]]
name        = "vim"                 # becomes the skill dir name in the agent
path        = "skills/vim"          # file or dir (relative to package root) = the skill body
description = "Drive vim from an agent"   # optional; shown in listings
agents      = ["claude", "opencode"]      # optional; default = all skill-supporting agents
```

A sibling `[[mcp]]` table (command / args / target agents) is specified the
same way but is **near-term, not MVP** (§6) — the schema is reserved here so
the vim-style "tool + mcp + skill" package is expressible end to end.

### 2.5 Skills are an orthogonal projection, not a delivery mode {#projection}

`req r3`

**Decision.** Installing a skill into an agent is a **projection**: read the
declared skill body from the package (in `vibedeps/…` once installed) and
write it into each target agent's skill directory in that agent's own
convention (`.claude/skills/<name>/…`, `.opencode/skills/<name>/…`,
`.agents/skills/<name>/…` — the paths PROP-015 §2.6 already resolves). This
is distinct from PROP-003 §2.5 subskill *delivery* (which materialises
content into the **project tree**). Skill projection materialises **out of**
the workspace, into the **agent**. The two share no code path beyond the
`Agent` skill-path resolver.

### 2.6 Standalone MVP — `vibe skill install` {#vibe-skill}

`req r3`

**Decision.** A new command family projects package-declared skills into
agents, reusing PROP-015's agent machinery (`Agent` enum, detection, the
idempotent skill writer, the per-(agent, scope) report records):

- **`vibe skill list`** — skills declared by installed packages.
- **`vibe skill install [--agent …] [--scope project|user|both] [<pkgref>] [<skill>…]`**
  — project skills into agents. **Default: all declared skills**; narrow
  with explicit skill names or a pkgref. Idempotent, `--dry-run`, confirm
  (or `--assume-yes`), per-(agent, scope) report — the same lifecycle and
  merge discipline as `vibe mcp install` (PROP-015 §2.7).
- **`vibe skill uninstall …`** — the inverse; strips only vibevm-projected
  skills, leaves foreign skill dirs untouched.

This is the **only standalone functionality v1 of this PROP ships.** It
needs no LLM, so it works today, agent-present or not.

### 2.7 The agentic relay — delegate intent back to the caller {#relay}

`req r4`

**Decision.** When a reasoning operation runs under the relay backend, it
does not act. It writes an `Intent` — a markdown prompt with light
frontmatter (id, source command, created-at, status) — to a **single-slot
mailbox**, the project-local `.vibe/agentic/command.md` (§3), and returns a
pointer telling the caller to drain it. The **consumer seam is one
command**, `vibe command`: it prints the pending `Intent` to stdout and
clears the slot (consume-on-read; the spent intent is archived to
`.vibe/agentic/command.done.md`). Re-running with an empty slot prints "no
pending command" and exits `0`.

Two properties make the two-step (produce → `vibe command`) worth its
seam rather than just printing the intent from the producer:

1. **Uniformity.** *Any* vibevm command that discovers mid-run it needs
   reasoning parks an intent the same way — not only `vibe agentic …`
   commands. The agent learns one drain verb, not per-command stdout
   parsing.
2. **Decoupling.** Producer and consumer need not be the same invocation,
   which is what lets a future deterministic command (`vibe build`) park a
   reasoning step and exit, the agent draining it afterward.

**MVP carries no write-back** (`req r4`): the relay is fire-and-forget. The
calling agent orchestrates the conversation — if it wants vibevm to see the
result, *it* arranges that with a follow-up command. The installed skill
(§2.9) states this contract explicitly so agents do not wait for a channel
that is not there. (Full bidirectional conversations are §6.)

### 2.8 One operation, two transports {#transports}

`req r5`

**Decision.** A reasoning/agentic operation is defined **once**, as a
transport-agnostic core (the `Intent`-producing function over a project
context), and exposed by **two thin adapters**:

- **One-shot CLI** (`vibe agentic <op>`) — stateless, one process per call.
  An intent is delivered through the §2.7 file relay. Best when vibevm is
  touched once and discarded — e.g. an agent scanning a directory of
  hundreds of vibevm projects for a quick fact. All per-session state
  (future conversation/context, §6) is lost on exit, by design.
- **MCP server** (`vibe mcp serve`, PROP-015 §2.1) — persistent,
  zero-latency, can hold session context. The same op is registered as an
  MCP tool; an intent is returned **synchronously in the tool result**, so
  no file mailbox is needed on this path. Best for sustained work inside
  one project.

The choice is the **agent's**, by situation, and the skill (§2.9) teaches
the heuristic. The core never knows which adapter called it.

### 2.9 The vibevm-usage skill teaches the protocol {#usage-skill}

`req r5`

**Decision.** The skill `vibe mcp install` already projects
(`skill_template.md`) gains a section that teaches an agent: (a) the
transport heuristic (one-shot CLI for a quick/one-off or a wide scan; MCP
server for sustained in-project work) — §2.8; (b) the relay contract: some
`vibe …` commands park reasoning instead of doing it; after such a command,
run `vibe command`, then **carry out the returned instruction yourself**;
(c) there is **no automatic write-back** — if the result should reach
vibevm, the agent issues the follow-up itself. The skill stays *data, not
code* (PROP-015 §2.6).

### 2.10 `vibe agentic explain` — the MVP demonstrator {#explain}

`req r4`

**Decision.** The first `vibe agentic` operation, `explain`, exercises the
whole relay with zero real risk. Run under an agent, it parks an `Intent`
to `.vibe/agentic/command.md` of roughly:

> **Task — explain this project.** In ≤3 short paragraphs, tell the reader
> what this project is and does. Sources, in priority order: (1) `README.md`
> at the project root — summarise it; (2) if `vibe.toml` is present, fold in
> what its structure reveals (the package `kind`, what it `requires`, what
> it `provides`). If `README.md` is absent, say so and explain from
> `vibe.toml` alone. Write for a developer seeing the repo for the first
> time. Do not invent features the sources do not support.

`vibe agentic explain` does no LLM work and reads no file content itself; it
only composes the intent (it *may* check which of `README.md` / `vibe.toml`
exist to tailor the prompt). The agent then runs `vibe command`, gets this
instruction, and produces the explanation on its own LLM. Affinity:
`agentic-only` until the built-in backend exists (§2.3).

## 3. The `.vibe/agentic/` relay directory {#vibevm-dir}

`req r4`

**Decision.** Agentic relay state lives under the existing project-local
`.vibe/` scratch root, in a dedicated **`.vibe/agentic/`** subdirectory
(created on demand) — one dot-dir, not two. `.vibe/` is already vibevm's
project-local scratch space (`init.rs` scaffolds `.vibe/cache/` for the
package cache) and is already git-ignored by its own `.vibe/.gitignore`
(`*`), so the relay inherits that ignore for free: no `vibe init` change,
and no second near-homonym dot-dir sitting beside `.vibe/`. Subdirectories
disambiguate the two concerns — `.vibe/cache/` is the package cache,
`.vibe/agentic/` is the agent↔vibevm relay channel (and the future home of
the §6 conversation state). MVP contents:

- `.vibe/agentic/command.md` — the single pending intent (absent when none).
- `.vibe/agentic/command.done.md` — the last consumed intent (archive aid).

The relay path is an internal detail: the installed skill (§2.9) teaches
the agent the `vibe command` verb, never the path, so the location carries
no external contract and can move freely. A future `vibe cache clean` must
scope to `.vibe/cache/` — never the whole `.vibe/` — so cache eviction
cannot nuke an in-flight relay intent.

## 4. MVP scope — what this PROP authorises now {#mvp}

1. **Manifest** — the `[[skill]]` section in `vibe-core` (§2.4), parsed,
   validated, round-tripped; `[[mcp]]` schema reserved but not wired.
2. **Standalone** — `vibe skill list` / `install` / `uninstall` (§2.6)
   over the existing agent machinery.
3. **Agentic core** — `InferenceBackend` + `Intent` + `RelayBackend`
   (§2.2, §2.7); affinity (§2.3).
4. **Agentic relay** — `.vibe/agentic/command.md` mailbox (§3); `vibe command`
   consumer (§2.7); `vibe agentic explain` producer (§2.10).
5. **Dual transport** — the explain op exposed as both `vibe agentic
   explain` (CLI) and an MCP tool (§2.8).
6. **Skill** — `skill_template.md` updated to teach the protocol (§2.9).

Crate placement (flagged to owner): a dedicated **`vibe-agentic`** crate for
§2.2/§2.3/§2.7 core (it will grow per §6), with adapters in `vibe-cli` and
`vibe-mcp`. Lighter alternative: fold the core into `vibe-mcp` for the MVP
and extract later.

## 5. Out of scope (now) {#out-of-scope}

- **A fifth package kind** — explicitly rejected (§2.4).
- **`[[mcp]]` bundled-server install** — schema reserved (§2.4), wiring is
  near-term, not MVP.
- **Built-in inference** — `BuiltinBackend` waits on `vibe-llm`
  (`VIBEVM-SPEC.md` §10.4); MVP relay-only.
- **Write-back / conversations** — §6.
- **Changing PROP-006** — untouched (§1.3).

## 6. Far backlog {#far-backlog}

Parked deliberately; recorded so the MVP's seams are cut to admit them:

- **Full vibevm↔agent conversations.** A request/response protocol shaped
  like the OpenAI Chat/Responses API: write-back, multi-turn, and full
  multi-agency — calling agents open any number of conversations; vibevm
  keeps a fast cache and the context each conversation needs. This is where
  the §2.7 relay grows a return channel and the §2.8 MCP transport grows
  session state.
- **An OpenCode-style console.** A persistent vibevm session with
  `--resume <id>`, reachable both from an agent (e.g. Claude Code) and
  interactively by a human at a terminal.
- **`BuiltinBackend`** over `vibe-llm` (§2.2) — the standalone inference
  engine that lets reasoning operations run with no agent present.
- **`[[mcp]]` bundled-server projection** (§2.4) — install a package's
  bundled MCP server into agents alongside its skills.

(Sibling far-backlogs: PROP-017 §8. If these lists keep growing, a
consolidated backlog doc may be warranted — not today.)

## 7. Acceptance {#acceptance}

- `vibe-core` parses and round-trips `[[skill]]`; an unknown key fails
  (`deny_unknown_fields`); a `[[skill]]` with a missing `path` is a typed
  manifest error citing this PROP.
- `vibe skill install` projects a fixture package's declared skill into each
  skill-supporting agent under the right path, preserves foreign skill dirs,
  is idempotent, and reports per-(agent, scope); `uninstall` is its inverse;
  `list` writes nothing.
- `vibe agentic explain`, run with a fixture project, writes a well-formed
  `.vibe/agentic/command.md` (frontmatter + the §2.10 prompt) and writes no other
  state; `vibe command` prints it, archives it to `command.done.md`, and
  empties the slot; a second `vibe command` reports "no pending command"
  and exits `0`.
- The same explain op invoked through the MCP transport returns the intent
  in the tool result and touches no mailbox file.
- A reasoning operation invoked standalone with no engine fails loud with
  the §2.1 message; `vibe skill install` invoked standalone succeeds.
- The projected `SKILL.md` contains the §2.9 protocol section; the existing
  PROP-015 acceptance still holds.
