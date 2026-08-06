# PROP-018 — Agentic and standalone modes {#root}

<status stage="impl" state="done" action="continue" comment="C 2026-07-25: the §4 MVP is implemented end to end (23 implements / 8 verifies across 8 sections); §6's heavier surface stays parked; fact grain 2026-07-24"/>

@fact:status-line **Status: the §4 MVP is IMPLEMENTED** (specified 2026-06-16 in an
owner-requested design session; verified against the tree 2026-07-25 by the
spec-actualization campaign — `vibe agentic explain`, `vibe command`, the
`[[skill]]` section and the `agentic_explain` MCP tool are all live, with 23
specmap `implements` and 8 `verifies` edges across eight sections). Everything
heavier stays parked in §6 (far backlog). This is the spec home for vibevm's
*product modes* — a cross-cutting concept, distinct from PROP-006's *session*
postures (see §1.3). @status:impl/done

@fact:related **Related:** [PROP-015](../modules/vibe-mcp/PROP-015-mcp-integration.md)
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
(a sibling far-backlog). @status:spec/done

---

## 1. Motivation {#motivation}

### 1.1 The problem {#problem}

- @fact:commands-split vibevm has commands that are pure algorithm (`install`, `check`, `list`)
  and commands that genuinely need reasoning (explain, build, review). The
  algorithmic ones already run from a bare terminal with no LLM
  (`VIBEVM-SPEC.md` §3.2). @status:spec/done
- @fact:reasoning-question The reasoning ones raise a question of *who does
  the reasoning, and how*. @status:spec/done

- @fact:invoked-by-agent vibevm is almost always invoked *by* a coding agent (Claude Code,
  OpenCode, Codex) that already holds a capable LLM, the live context, and
  the tools. @status:spec/done
- @fact:executor-vs-author For reasoning work in that setting the agent is the right
  **executor** — but vibevm is the right **author** of the task. @status:spec/done
- @fact:domain-knowledge vibevm carries stable, algorithmic knowledge of its own domain (the
  spec-driven model, the dependency and package rules, the discipline), so an
  instruction it composes is more informative and more trustworthy than one
  the agent would improvise from scratch. @status:spec/done
- @fact:roles-named This PROP gives the two their natural roles — vibevm composes the
  domain-grounded instruction, the agent carries it out — and names the
  operating contexts so the codebase branches on them cleanly. @status:spec/done
- @fact:standalone-aside vibevm can also reason with *no* agent present, via a
  built-in `vibe-llm` engine — `VIBEVM-SPEC.md` §10.4, far-backlog §6 — but
  that is standalone mode; it is not what makes agentic mode worthwhile. @status:spec/done

### 1.2 The two modes — one axis {#axis}

@fact:one-axis The modes are not two codebases. They are one question: **where does an
operation's reasoning happen?** @status:spec/done

- @fact:MODE-AGENTIC **agentic** — vibevm is driven by a host agent during that agent's own
  work. For a step that needs reasoning, vibevm composes a domain-grounded
  instruction and *delegates it back* to the agent, which executes it on
  its LLM with the live context vibevm lacks. The split is by strength, not
  a workaround: the agent is the better executor in-session, vibevm the
  better author of the instruction. (Pure-algorithm operations still run
  directly — agentic is about the *reasoning* steps.) @status:spec/done
- @fact:MODE-STANDALONE **standalone** — vibevm stands on its own. Reasoning runs on vibevm's
  *own* backend: algorithmic where the work allows, and — when `vibe-llm`
  lands — a built-in inference engine. Today the standalone backend has no
  LLM, so the only standalone functionality this PROP ships is the
  **non-reasoning** one: projecting skills into agents (§2.6). @status:spec/done

- @fact:UNIFYING-STATEMENT The unifying statement (§2.1): **a mode is a choice of inference backend.**
  Non-reasoning operations behave identically in both modes; reasoning
  operations branch on the backend. @status:spec/done
- @fact:THE-SEAM This is the seam everything else hangs off. @status:spec/done

### 1.3 What this is NOT — PROP-006 {#not-prop-006}

- @fact:P006-SESSION-POSTURES [PROP-006](PROP-006-operating-modes.md) defines *session operating
  postures*: codewords ("move fast and break things") that change **how an
  agent behaves within one work session** (whether to pause for
  confirmation, how freely to spend tokens). Those govern the *agent's*
  conduct. @status:spec/done
- @fact:P018-INFERENCE-SOURCE PROP-018 modes govern **where vibevm gets inference** — a property
  of *vibevm's* execution, orthogonal to any session posture. @status:spec/done
- @fact:orthogonal-example A session can be in "move fast" posture while vibevm runs in agentic
  mode; the two never collide. @status:spec/done
- @fact:NO-OVERLOAD Do not overload one onto the other. @status:spec/done

## 2. Decisions {#decisions}

### 2.1 A mode is a choice of inference backend {#mode-is-backend}

@fact:req-mode-backend `req r1` @status:spec/done

- @fact:MODE-INFERRED **Decision.** Mode is not a global flag the user sets; it is **inferred per
  operation from how vibevm was reached and what backend is available.** @status:spec/done
- @fact:OP-DECLARES An operation declares whether it needs inference; if it does, the active
  backend decides the realisation: @status:spec/done

- @fact:REACH-SUBPROCESS reached as a **subprocess of an agent** (CLI one-shot or MCP call) →
  the **relay backend** (§2.7): delegate the intent back to that agent. @status:spec/done
- @fact:REACH-STANDALONE-ENGINE reached **standalone** with a built-in engine available (future) → the
  **built-in backend**: run inference in-process via `vibe-llm`. @status:spec/done
- @fact:REACH-STANDALONE-NO-ENGINE reached **standalone** with no engine (today) → a reasoning operation
  **fails loud** with "this needs an inference backend; run me under an
  agent, or wait for the built-in engine," and a non-reasoning operation
  runs normally. @status:spec/done
- @fact:mode-inferred-why **Why:** §1.2 `##UNIFYING-STATEMENT` fixes what a mode is — *"a mode is a choice of inference backend"* — and §2.3 `##AFFINITY-OF-WORK` fixes who chooses: *"Affinity is a property of the work, not a user choice."* Mode-by-inference is that same principle one level up: which backend can serve a call is a fact about the call's reach, not a preference. @status:spec/done
- @fact:mode-inferred-rejected **Considered and rejected:** **a global mode flag the user sets** (`--mode agentic|standalone` or a `vibe.toml` key) — rejected: a user could then name a backend the operation has no affinity for, which the dispatcher must refuse anyway (`##DISPATCHER-REFUSES`), or name one that does not exist on this machine, which today is every standalone reasoning call (`##REACH-STANDALONE-NO-ENGINE`). The flag would be a way to ask for a refusal. @status:spec/done
- @fact:mode-inferred-revisit **Revisit when:** `BuiltinBackend` ships over `vibe-llm` (`##FB-BUILTIN-BACKEND`, `VIBEVM-SPEC.md` §10.4) — from that day two backends can both serve one standalone call, and *"what backend is available"* stops determining the answer on its own. Observation point: the far-backlog item closing, i.e. a `vibe-llm` inference path in the workspace. Second clause: a reach appears that the inference cannot classify — a persistent console (`##FB-CONSOLE`) or an invocation through a wrapper that hides the agent parentage — observed as a mis-chosen backend in a bug report. @status:spec/done

### 2.2 The pluggable inference backend {#pluggable-backend}

@fact:req-pluggable `req r2` @status:spec/done

- @fact:BACKEND-TRAIT **Decision.** Inference sits behind one trait, `InferenceBackend`, so an
  operation never names a provider. @status:spec/done
- @fact:INTENT-CONSTRUCT An operation that needs reasoning constructs an `Intent` (a structured
  prompt + the inputs it needs) and hands it to the active backend. @status:spec/done
- @fact:two-backends-lead Two backends are foreseen: @status:spec/done

- @fact:RELAY-BACKEND **`RelayBackend`** (agent mode) — vibevm authors the `Intent` and
  *parks* it for the calling agent to execute, returning "delegated"
  (§2.7). Not a stopgap: in agent mode the agent is the right executor. @status:spec/done
- @fact:BUILTIN-BACKEND **`BuiltinBackend`** (standalone mode; far backlog §6) — runs the
  `Intent` on `vibe-llm` in-process, for when no agent is present. @status:spec/done

- @fact:NOT-OVERBUILT This is deliberately **not** over-built: the trait exists so reasoning
  operations are written once against an abstract backend, and the standalone
  engine slots in later without touching them. @status:spec/done
- @fact:SINGLE-HOME-OPS Operations with a single
  natural home (skill-install is standalone-only; a "rewrite this spec
  section" op may be agentic-only until the engine exists) simply do not
  offer the other backend (§2.3). @status:spec/done

### 2.3 Per-operation backend affinity {#affinity}

@fact:req-affinity `req r2` @status:spec/done

- @fact:AFFINITY-DECL **Decision.** Each operation declares an **affinity**: `agentic-only`,
  `standalone-only`, or `both`. @status:spec/done
- @fact:AFFINITY-OF-WORK Affinity is a property of the *work*, not a
  user choice — scanning a manifest is `standalone-only` (pure algorithm,
  needs no agent); a free-form "explain this project in prose" is
  `agentic-only` until the built-in engine exists; a task expressible either
  as a deterministic pass or as reasoning is `both`. @status:spec/done
- @fact:DISPATCHER-REFUSES The dispatcher refuses
  an operation invoked through a backend it has no affinity for, with a
  message naming the right one. @status:spec/done

### 2.4 Agent-installable artifacts are declared separately from the package kind {#skill-decl}

@fact:req-skill-decl `req r3` @status:spec/done

- @fact:SKILL-SECTION-NOT-KIND **Decision.** A package declares which of its files are **skills** for
  agents in a dedicated manifest section — **not** by introducing a
  package kind of its own. The kind register (`package_ref.rs`,
  `VIBEVM-SPEC.md` §4.1) stays closed to skills. @status:spec/done
- @fact:ANY-KIND-RATIONALE **Why:** skills can
  live inside a package of *any* kind and be structured any way. A `tool`
  package `vim` can ship the tool itself **plus** a skill for driving vim
  — one self-contained package, two artefact classes. Kind answers "what
  is this package"; the new section answers "what does it project into an
  agent." @status:spec/done
- @fact:MCP-HALF-SUPERSEDED This unit's original text sketched MCP servers as a second
  any-kind section; that half is SUPERSEDED — MCP servers became their own
  `mcp` kind with their own laws, owner resolution 2026-07-07:
  [PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md). The skill law
  here is unchanged. @status:spec/done
- @fact:skill-decl-rejected **Considered and rejected:** **a fifth package kind for skills** — explicitly rejected (`##OOS-FIFTH-KIND`): kind answers *"what is this package"* while the section answers *"what does it project into an agent"*, and skills can live inside a package of any kind (`##ANY-KIND-RATIONALE`). **MCP servers as a second any-kind section** — proposed in this unit's original text and **superseded**: they became their own `mcp` kind with their own laws, owner resolution 2026-07-07 ([PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md), `##MCP-HALF-SUPERSEDED`). The skill law is unchanged; the MCP half is the counter-example that shows where the line falls. @status:spec/done
- @fact:skill-decl-revisit **Revisit when:** an agent-installable artefact class arrives that needs **its own lifecycle laws** — install / uninstall semantics, resolution or conflict rules of its own — rather than only a projection path. That is exactly the state that fired for MCP servers on 2026-07-07 (`##MCP-HALF-SUPERSEDED`), so the trigger has a worked precedent. Observation point: the kind register in `crates/vibe-core` (`package_ref.rs`) and `VIBEVM-SPEC.md` §4.1 — whose `##INV-VOCABULARY` already anticipates `app`; the register growing is the fired state. @status:spec/done

@fact:SKILL-TABLE-SHAPE The MVP section is an array-of-tables, matching the manifest's existing
`[[requires_any]]` / `[[registry]]` / `[[mirror]]` shape: @status:spec/done

```toml
[[skill]]
name        = "vim"                 # becomes the skill dir name in the agent
path        = "skills/vim"          # file or dir (relative to package root) = the skill body
description = "Drive vim from an agent"   # optional; shown in listings
agents      = ["claude", "opencode"]      # optional; default = all skill-supporting agents
```

@fact:MCP-TABLE-RESERVED A sibling `[[mcp]]` table (command / args / target agents) is specified the
same way but is **near-term, not MVP** (§6) — the schema is reserved here so
the vim-style "tool + mcp + skill" package is expressible end to end. @status:spec/done

### 2.5 Skills are an orthogonal projection, not a delivery mode {#projection}

@fact:req-projection `req r3` @status:spec/done

- @fact:PROJECTION-DEF **Decision.** Installing a skill into an agent is a **projection**: read the
  declared skill body from the package (in `vibedeps/…` once installed) and
  write it into each target agent's skill directory in that agent's own
  convention (`.claude/skills/<name>/…`, `.opencode/skills/<name>/…`,
  `.agents/skills/<name>/…` — the paths PROP-015 §2.6 already resolves). @status:spec/done
- @fact:DISTINCT-FROM-DELIVERY This
  is distinct from PROP-003 §2.5 subskill *delivery* (which materialises
  content into the **project tree**). Skill projection materialises **out of**
  the workspace, into the **agent**. @status:spec/done
- @fact:NO-SHARED-CODE The two share no code path beyond the
  `Agent` skill-path resolver. @status:spec/done

### 2.6 Standalone MVP — `vibe skill install` {#vibe-skill}

@fact:req-vibe-skill `req r3` @status:spec/done

@fact:SKILL-CMD-FAMILY **Decision.** A new command family projects package-declared skills into
agents, reusing PROP-015's agent machinery (`Agent` enum, detection, the
idempotent skill writer, the per-(agent, scope) report records): @status:spec/done

- @fact:CMD-SKILL-LIST **`vibe skill list`** — skills declared by installed packages. @status:spec/done
- @fact:CMD-SKILL-INSTALL **`vibe skill install [--agent …] [--scope project|user|both] [<pkgref>] [<skill>…]`**
  — project skills into agents. **Default: all declared skills**; narrow
  with explicit skill names or a pkgref. Idempotent, `--dry-run`, confirm
  (or `--assume-yes`), per-(agent, scope) report — the same lifecycle and
  merge discipline as `vibe mcp install` (PROP-015 §2.7). @status:spec/done
- @fact:CMD-SKILL-UNINSTALL **`vibe skill uninstall …`** — the inverse; strips only vibevm-projected
  skills, leaves foreign skill dirs untouched. @status:spec/done

@fact:ONLY-STANDALONE-V1 This is the **only standalone functionality v1 of this PROP ships.** It
needs no LLM, so it works today, agent-present or not. @status:spec/done

### 2.7 The agentic relay — delegate intent back to the caller {#relay}

@fact:req-relay `req r4` @status:spec/done

- @fact:RELAY-PARKS **Decision.** When a reasoning operation runs under the relay backend, it
  does not act. It writes an `Intent` — a markdown prompt with light
  frontmatter (id, source command, created-at, status) — to a **single-slot
  mailbox**, the project-local `.vibe/agentic/command.md` (§3), and returns a
  pointer telling the caller to drain it. @status:spec/done
- @fact:DRAIN-VERB The **consumer seam is one
  command**, `vibe command`: it prints the pending `Intent` to stdout and
  clears the slot (consume-on-read; the spent intent is archived to
  `.vibe/agentic/command.done.md`). @status:spec/done
- @fact:EMPTY-SLOT-OK Re-running with an empty slot prints "no
  pending command" and exits `0`. @status:spec/done

@fact:two-step-lead Two properties make the two-step (produce → `vibe command`) worth its
seam rather than just printing the intent from the producer: @status:spec/done

1. @fact:SEAM-UNIFORMITY **Uniformity.** *Any* vibevm command that discovers mid-run it needs
   reasoning parks an intent the same way — not only `vibe agentic …`
   commands. The agent learns one drain verb, not per-command stdout
   parsing. @status:spec/done
2. @fact:SEAM-DECOUPLING **Decoupling.** Producer and consumer need not be the same invocation,
   which is what lets a future deterministic command (`vibe build`) park a
   reasoning step and exit, the agent draining it afterward. @status:spec/done

- @fact:NO-WRITE-BACK **MVP carries no write-back** (`req r4`): the relay is fire-and-forget. @status:spec/done
- @fact:AGENT-ORCHESTRATES The calling agent orchestrates the conversation — if it wants vibevm to
  see the result, *it* arranges that with a follow-up command. @status:spec/done
- @fact:SKILL-STATES-NO-CHANNEL The installed skill (§2.9) states this contract explicitly so agents do
  not wait for a channel that is not there. (Full bidirectional
  conversations are §6.) @status:spec/done
- @fact:relay-why **Why:** the two-step seam buys two properties a direct print cannot, both stated at `##two-step-lead`: **uniformity** — any command that discovers mid-run it needs reasoning parks an intent the same way, so an agent learns one drain verb rather than per-command stdout parsing (`##SEAM-UNIFORMITY`); and **decoupling** — producer and consumer need not be the same invocation, which is what lets a future deterministic command park a reasoning step and exit (`##SEAM-DECOUPLING`). @status:spec/done
- @fact:relay-rejected **Considered and rejected:** **printing the intent directly from the producer**, with no `vibe command` seam — rejected for the two reasons above; it is cheaper by one command and forfeits both. **A return channel (write-back)** — **deferred, not rejected**: the MVP relay is fire-and-forget (`##NO-WRITE-BACK`), the calling agent orchestrates (`##AGENT-ORCHESTRATES`), and full bidirectional conversations are parked at §6 `##FB-CONVERSATIONS`. @status:spec/done
- @fact:relay-revisit **Revisit when:** either the **single slot overflows** — a producer runs while `.vibe/agentic/command.md` already holds an undrained intent, which the mailbox's own shape makes mechanically observable (`##FILE-COMMAND-MD`) — **or** the fire-and-forget contract starts costing a round trip, observed as an agent issuing a follow-up `vibe …` command whose only purpose is to hand a result back (the case `##SKILL-STATES-NO-CHANNEL` tells agents not to expect). Either fires §6's `##FB-CONVERSATIONS`. @status:spec/done

### 2.8 One operation, two transports {#transports}

@fact:req-transports `req r5` @status:spec/done

@fact:ONE-OP-TWO-TRANSPORTS **Decision.** A reasoning/agentic operation is defined **once**, as a
transport-agnostic core (the `Intent`-producing function over a project
context), and exposed by **two thin adapters**: @status:spec/done

- @fact:TRANSPORT-CLI **One-shot CLI** (`vibe agentic <op>`) — stateless, one process per call.
  An intent is delivered through the §2.7 file relay. Best when vibevm is
  touched once and discarded — e.g. an agent scanning a directory of
  hundreds of vibevm projects for a quick fact. All per-session state
  (future conversation/context, §6) is lost on exit, by design. @status:spec/done
- @fact:TRANSPORT-MCP **MCP server** (`vibe mcp serve`, PROP-015 §2.1) — persistent,
  zero-latency, can hold session context. The same op is registered as an
  MCP tool; an intent is returned **synchronously in the tool result**, so
  no file mailbox is needed on this path. Best for sustained work inside
  one project. @status:spec/done

- @fact:CHOICE-IS-AGENTS The choice is the **agent's**, by situation, and the skill (§2.9) teaches
  the heuristic. @status:spec/done
- @fact:CORE-ADAPTER-BLIND The core never knows which adapter called it. @status:spec/done

### 2.9 The vibevm-usage skill teaches the protocol {#usage-skill}

@fact:req-usage-skill `req r5` @status:spec/done

@fact:USAGE-SKILL-TEACHES **Decision.** The skill `vibe mcp install` already projects
(`skill_template.md`) gains a section that teaches an agent: @status:spec/done

- @fact:TEACH-TRANSPORT the transport heuristic (one-shot CLI for a quick/one-off or a wide scan;
  MCP server for sustained in-project work) — §2.8; @status:spec/done
- @fact:TEACH-RELAY the relay contract: some `vibe …` commands park reasoning instead of
  doing it; after such a command, run `vibe command`, then **carry out the
  returned instruction yourself**; @status:spec/done
- @fact:TEACH-NO-WRITEBACK there is **no automatic write-back** — if the result should reach
  vibevm, the agent issues the follow-up itself. @status:spec/done

@fact:SKILL-DATA-NOT-CODE The skill stays *data, not code* (PROP-015 §2.6). @status:spec/done

### 2.10 `vibe agentic explain` — the MVP demonstrator {#explain}

@fact:req-explain `req r4` @status:spec/done

@fact:EXPLAIN-DEMONSTRATOR **Decision.** The first `vibe agentic` operation, `explain`, exercises the
whole relay with zero real risk. Run under an agent, it parks an `Intent`
to `.vibe/agentic/command.md` of roughly: @status:spec/done

```
Task — explain this project. In ≤3 short paragraphs, tell the reader
what this project is and does. Sources, in priority order: (1) `README.md`
at the project root — summarise it; (2) if `vibe.toml` is present, fold in
what its structure reveals (the package `kind`, what it `requires`, what
it `provides`). If `README.md` is absent, say so and explain from
`vibe.toml` alone. Write for a developer seeing the repo for the first
time. Do not invent features the sources do not support.
```

- @fact:EXPLAIN-NO-LLM `vibe agentic explain` does no LLM work and reads no file content itself; it
  only composes the intent (it *may* check which of `README.md` / `vibe.toml`
  exist to tailor the prompt). @status:spec/done
- @fact:EXPLAIN-FLOW The agent then runs `vibe command`, gets this
  instruction, and produces the explanation on its own LLM. @status:spec/done
- @fact:EXPLAIN-AFFINITY Affinity: `agentic-only` until the built-in backend exists (§2.3). @status:spec/done

## 3. The `.vibe/agentic/` relay directory {#vibevm-dir}

@fact:req-relay-dir `req r4` @status:spec/done

- @fact:RELAY-DIR **Decision.** Agentic relay state lives under the existing project-local
  `.vibe/` scratch root, in a dedicated **`.vibe/agentic/`** subdirectory
  (created on demand) — one dot-dir, not two. @status:spec/done
- @fact:INHERITS-IGNORE `.vibe/` is already vibevm's
  project-local scratch space (`init.rs` scaffolds `.vibe/cache/` for the
  package cache) and is already git-ignored by its own `.vibe/.gitignore`
  (`*`), so the relay inherits that ignore for free: no `vibe init` change,
  and no second near-homonym dot-dir sitting beside `.vibe/`. @status:spec/done
- @fact:SUBDIR-DISAMBIGUATION Subdirectories
  disambiguate the two concerns — `.vibe/cache/` is the package cache,
  `.vibe/agentic/` is the agent↔vibevm relay channel (and the future home of
  the §6 conversation state). @status:spec/done

@fact:mvp-contents-lead MVP contents: @status:spec/done

- @fact:FILE-COMMAND-MD `.vibe/agentic/command.md` — the single pending intent (absent when none). @status:spec/done
- @fact:FILE-COMMAND-DONE-MD `.vibe/agentic/command.done.md` — the last consumed intent (archive aid). @status:spec/done

- @fact:PATH-INTERNAL The relay path is an internal detail: the installed skill (§2.9) teaches
  the agent the `vibe command` verb, never the path, so the location carries
  no external contract and can move freely. @status:spec/done
- @fact:CACHE-CLEAN-SCOPE A future `vibe cache clean` must
  scope to `.vibe/cache/` — never the whole `.vibe/` — so cache eviction
  cannot nuke an in-flight relay intent. @status:spec/done

## 4. MVP scope — what this PROP authorises now {#mvp}

1. @fact:MVP-MANIFEST **Manifest** — the `[[skill]]` section in `vibe-core` (§2.4), parsed,
   validated, round-tripped; `[[mcp]]` schema reserved but not wired. @status:spec/done
2. @fact:MVP-STANDALONE **Standalone** — `vibe skill list` / `install` / `uninstall` (§2.6)
   over the existing agent machinery. @status:spec/done
3. @fact:MVP-AGENTIC-CORE **Agentic core** — `InferenceBackend` + `Intent` + `RelayBackend`
   (§2.2, §2.7); affinity (§2.3). @status:spec/done
4. @fact:MVP-RELAY **Agentic relay** — `.vibe/agentic/command.md` mailbox (§3); `vibe command`
   consumer (§2.7); `vibe agentic explain` producer (§2.10). @status:spec/done
5. @fact:MVP-DUAL-TRANSPORT **Dual transport** — the explain op exposed as both `vibe agentic
   explain` (CLI) and an MCP tool (§2.8). @status:spec/done
6. @fact:MVP-SKILL **Skill** — `skill_template.md` updated to teach the protocol (§2.9). @status:spec/done

@fact:CRATE-PLACEMENT Crate placement (flagged to owner): a dedicated **`vibe-agentic`** crate for
§2.2/§2.3/§2.7 core (it will grow per §6), with adapters in `vibe-cli` and
`vibe-mcp`. Lighter alternative: fold the core into `vibe-mcp` for the MVP
and extract later. @status:spec/done

## 5. Out of scope (now) {#out-of-scope}

- @fact:OOS-FIFTH-KIND **A fifth package kind** — explicitly rejected (§2.4). @status:spec/done
- @fact:OOS-MCP-WIRING **`[[mcp]]` bundled-server install** — schema reserved (§2.4), wiring is
  near-term, not MVP. @status:spec/done
- @fact:OOS-BUILTIN **Built-in inference** — `BuiltinBackend` waits on `vibe-llm`
  (`VIBEVM-SPEC.md` §10.4); MVP relay-only. @status:spec/done
- @fact:OOS-WRITE-BACK **Write-back / conversations** — §6. @status:spec/done
- @fact:OOS-PROP-006 **Changing PROP-006** — untouched (§1.3). @status:spec/done

## 6. Far backlog {#far-backlog}

@fact:far-backlog-lead Parked deliberately; recorded so the MVP's seams are cut to admit them: @status:spec/done

- @fact:FB-CONVERSATIONS **Full vibevm↔agent conversations.** A request/response protocol shaped
  like the OpenAI Chat/Responses API: write-back, multi-turn, and full
  multi-agency — calling agents open any number of conversations; vibevm
  keeps a fast cache and the context each conversation needs. This is where
  the §2.7 relay grows a return channel and the §2.8 MCP transport grows
  session state. @status:spec/done
- @fact:FB-CONSOLE **An OpenCode-style console.** A persistent vibevm session with
  `--resume <id>`, reachable both from an agent (e.g. Claude Code) and
  interactively by a human at a terminal. @status:spec/done
- @fact:FB-BUILTIN-BACKEND **`BuiltinBackend`** over `vibe-llm` (§2.2) — the standalone inference
  engine that lets reasoning operations run with no agent present. @status:spec/done
- @fact:FB-MCP-PROJECTION **`[[mcp]]` bundled-server projection** (§2.4) — install a package's
  bundled MCP server into agents alongside its skills. @status:spec/done

@fact:sibling-backlogs (Sibling far-backlogs: PROP-017 §8. If these lists keep growing, a
consolidated backlog doc may be warranted — not today.) @status:spec/done

## 7. Acceptance {#acceptance}

- @fact:ACC-MANIFEST `vibe-core` parses and round-trips `[[skill]]`; an unknown key fails
  (`deny_unknown_fields`); a `[[skill]]` with a missing `path` is a typed
  manifest error citing this PROP. @status:spec/done
- @fact:ACC-SKILL-INSTALL `vibe skill install` projects a fixture package's declared skill into each
  skill-supporting agent under the right path, preserves foreign skill dirs,
  is idempotent, and reports per-(agent, scope); `uninstall` is its inverse;
  `list` writes nothing. @status:spec/done
- @fact:ACC-EXPLAIN-RELAY `vibe agentic explain`, run with a fixture project, writes a well-formed
  `.vibe/agentic/command.md` (frontmatter + the §2.10 prompt) and writes no other
  state; `vibe command` prints it, archives it to `command.done.md`, and
  empties the slot; a second `vibe command` reports "no pending command"
  and exits `0`. @status:spec/done
- @fact:ACC-MCP-TRANSPORT The same explain op invoked through the MCP transport returns the intent
  in the tool result and touches no mailbox file. @status:spec/done
- @fact:ACC-FAIL-LOUD A reasoning operation invoked standalone with no engine fails loud with
  the §2.1 message; `vibe skill install` invoked standalone succeeds. @status:spec/done
- @fact:ACC-SKILL-SECTION The projected `SKILL.md` contains the §2.9 protocol section; the existing
  PROP-015 acceptance still holds. @status:spec/done
