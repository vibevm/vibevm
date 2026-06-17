# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-16 at session save. Branch `main` @ `ee9c62e`. The
PROP-018 agentic + standalone modes MVP is complete and on both mirrors
(@ `bd26156`); this save's own commits (`ee9c62e` discovery prompt +
the WAL/CONTINUE updates) roll out as the final step, leaving `main` ≡
`gitverse` ≡ `github`. Working tree clean. Full gate panel green._

> **`spec/WAL.md` is the canonical living state and its header is
> current.** If this snapshot and the WAL disagree, the WAL wins. Boot
> first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files incl. the two
> Discipline snippets → `spec/WAL.md`), then read this. The **git log is
> the authoritative per-item record** — every commit cites its reasoning.

---

## TL;DR

This session designed and shipped the **PROP-018 agentic + standalone
modes MVP** — vibevm's two product modes, turning on one axis: *where does
an operation's reasoning happen?*

- **standalone** — `vibe skill {list,install,uninstall}` projects
  package-declared `[[skill]]` files into coding agents' skill dirs
  (Claude Code / OpenCode / Codex), reusing the PROP-015 agent machinery.
  **No LLM** — works agent-present or not.
- **agentic** — vibevm composes a *domain-grounded* `Intent` and the
  calling agent executes it: `vibe agentic explain` parks the instruction
  in `.vibe/agentic/command.md`, `vibe command` drains it; the same op is
  also the `agentic_explain` MCP tool (returns inline, no mailbox). One
  core, two transports (CLI one-shot / persistent MCP).

The **narrative** was corrected mid-session by the owner and is now
consistent across every surface: vibevm *authors* the trustworthy,
domain-grounded instruction (its stable algorithmic domain knowledge makes
the prompt better than improvisation); the agent is the better *in-session
executor* (it holds the live context and tools). Division of labour by
strength — **not** vibevm offloading because it lacks an engine.

Eleven gate-green commits (`e7d5cbf` … `bd26156`) on both mirrors, plus
`ee9c62e` (the General Discovery Prompt, this session's research-mode
preamble).

## Where work stands

- **Branch `main` @ `ee9c62e`.** After this save's rollout, both mirrors
  (`gitverse` = `anarchic/vibevm`, `github` = `anarchic-pro/vibevm`) are
  level with `main`. Working tree clean.
- **No campaign in flight.** PROP-018 MVP is the last work; resolvo
  (PROP-017) remains the default solver. The next session picks the
  owner's next goal.
- **Gate panel — green.** `self-check.sh` exit 0 (fmt, all tests,
  doctests, clippy `-D warnings`, `vibe check`); `conform check` 0/0/0
  (baseline EMPTY); `specmap --check` clean (509 units / 491 edges / 0
  suspects / 0 warnings / 0 orphans); `vibe check` 0/0/0.

## Active blocker & the human action that clears it

**None.** Panel green, tree clean, mirrors synced after this save. Nothing
waits on a human action. (One standing owner decision, not a blocker:
whether to begin PROP-018 §6 far-backlog or a fresh goal — see below.)

## EXACT next-steps recipe (candidate work — the owner chooses)

No plan is mid-execution. Candidates:

1. **PROP-018 §6 far backlog** (the seams are already cut for these):
   - `[[mcp]]` **bundled-server install** — the schema is reserved in
     PROP-018 §2.4; smallest of the four, closes the vim-style
     "tool + mcp + skill" package end to end. Extend `vibe skill` (or a
     sibling) to install a package's declared MCP server into agents,
     reusing `vibe-mcp::agent_config` (`merge_json`/`merge_toml`).
   - **`BuiltinBackend`** over `vibe-llm` — standalone reasoning with no
     agent present. The `InferenceBackend` trait
     (`crates/vibe-mcp/src/agentic.rs`) is the slot; `vibe-llm` is still an
     M0 stub (`VIBEVM-SPEC.md` §10.4).
   - **Full vibevm↔agent conversations** — an OpenAI-Responses-shaped
     protocol with write-back, multi-agency, and a fast context cache.
     This is where the §2.7 relay grows a return channel and the §2.8 MCP
     transport grows session state.
   - **OpenCode-style resumable console** — a persistent vibevm session
     with `--resume <id>`, from an agent and interactively.
2. **DISCIPLINE-SWEEP** — `cargo xtask health` (offline) surfaces the
   standing backlog (file-length danger band, pub-doctest drain /
   promotion candidates, deviation-debt census).
3. **A fresh owner goal** — boots from this checkpoint with no debt.

## Non-obvious findings (this session)

- **Editing PROP prose drifts the specmap content-hash even when the
  `{#anchor}` is unchanged.** specmap tracks a content-hash per spec unit;
  changing the prose under an anchor (without bumping `req rN`) raises an
  `unbumped-hash` advisory at the next `cargo xtask specmap` regen. It is
  **editorial**, resolved by regenerating + committing `specmap.json` (and
  optionally a `spec-editorial: <anchor>` line in the commit body).
  `specmap --check` is clean once `specmap.json` matches the tree. (Learnt
  twice — once on the `.vibe/agentic` relay-dir rename, once on the
  narrative reframe of `#pluggable-backend`.) **Lesson: regen specmap after
  any PROP edit, not only after code-marker changes.**
- **clippy `enum-variant-names` (under `-D warnings`) rejects an enum
  variant whose name ends in the enum's own name.** `Command::AgenticCommand`
  failed; renamed to `Command::Drain` with `#[command(name = "command")]`
  to keep the user-facing `vibe command`.
- **A new `#[cell]` `McpTool` needs a name-reference oracle** in
  `crates/vibe-mcp/tests/tools_oracle.rs` (the `cell-has-oracle` net,
  R-040) and **a new public seam trait needs a compiled doctest**
  (`seam-has-doctest`). Both were caught by conform/self-check and added.
- **PowerShell 5.1 quirks (re-confirmed):** `Get-Content` reads UTF-8-no-BOM
  as ANSI, so `.vibe/agentic/command.md` *looked* mojibake'd in
  `Get-Content -Raw` while the `vibe` binary read/wrote it correctly — the
  file was never corrupted. And `2>&1` on a native exe wraps stderr as
  `NativeCommandError` noise (cargo still succeeded) — don't redirect
  stderr; it is captured separately.
- **Machine quirks (unchanged, still true):** edit via Edit/Write tools,
  never PS `Set-Content` (UTF-8 round-trip corruption); `git commit` via
  `-F - <<'MSG'` heredoc only; `self-check.sh` runs through **Git Bash**
  (the Bash tool here), never WSL; when reading a gate's exit code use
  `$?` / a captured `EXIT=`, never a `| tail` pipe (it masks the code).

## Repository map

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = `cargo xtask`
├─ CLAUDE.md / AGENTS.md / GEMINI.md   identical; the 4 rules + boot pointer
├─ VIBEVM-SPEC.md            owner-frozen implementation spec (edits need the owner)
├─ MEMORY.md                 pointer to spec/boot/90-user.md
├─ CONTINUE.md               this cold-resume snapshot
├─ mirrors.toml              PROP-016 source-mirror target registry (gitverse + github)
├─ conform-baseline.json     conform ratchet baseline — EMPTY (0 frozen)
├─ specmap.json              traceability index (509 units / 478 items / 491 edges)
├─ specmap-ratchet.json      orphan ratchet exempt list (6 exempt)
├─ crates/                   library/bin crates (see gate split below)
├─ xtask/                    project tooling crate (gate-EXEMPT), at workspace root
├─ spec/                     the spec tree — the source of truth
│   ├─ boot/                 session-boot files (INDEX.md manifest, 00-core, 90-user)
│   ├─ common/               PROP-000…018 (cross-cutting decisions; PROP-018 = modes)
│   ├─ modules/              per-module PROP/FEAT (vibe-registry, vibe-mcp, …)
│   ├─ discipline/           ENGINE-CONFORM, BROWNFIELD, the Discipline corpus
│   ├─ research/             PROP-004 (tessl) + DISCOVERY_PROMPT.md (this session's mode)
│   ├─ terraforms/           campaign plans (CONVERT-PLAN, PUBDOC-DRAIN, DISCIPLINE-SWEEP)
│   └─ WAL.md                canonical living state (checkpoint, rewritten each session)
├─ vibedeps/                 installed packages (flow-discipline-core, stack-rust-ai-native)
├─ fixtures/registry/        hermetic test registry (never touches a real host)
├─ tools/                    self-check.sh, jtd-codegen
├─ schemas/                  JTD schemas — the vibe-wire codegen INPUT
├─ refs/                     book/ (read-only owner reference) + src/ (cargo/uv/spec-kit)
└─ terraform/                registry/ (test+debt baselines), health/ (sweep snapshot)
```

**PROP-018 lives in these files** (the MVP surface, for a cold reader):

| Concern | File |
|---|---|
| Design | `spec/common/PROP-018-agentic-standalone-modes.md` |
| `[[skill]]` manifest type | `crates/vibe-core/src/manifest/package/skill.rs` (+ wired in `document.rs`) |
| Skill projection writer | `crates/vibe-mcp/src/pkgskill.rs`; `Agent::skills_root` in `agents.rs` |
| `vibe skill` command | `crates/vibe-cli/src/commands/skill/mod.rs`, `cli/skill.rs` |
| Agentic relay core | `crates/vibe-mcp/src/agentic.rs` (`Intent`, `InferenceBackend`, `RelayBackend`, `Affinity`, `explain_intent`) |
| `vibe agentic` / `vibe command` | `crates/vibe-cli/src/commands/agentic/mod.rs`, `cli/agentic.rs` |
| `agentic_explain` MCP tool | `crates/vibe-mcp/src/tools.rs` (+ oracle in `tests/tools_oracle.rs`) |
| Agent-facing teaching | `crates/vibe-mcp/src/skill_template.md` |

**Crate gate status** (GATED = held to the full conform rule set with a
zero-new-findings ratchet):

| Crate | Holds | Gate |
|---|---|---|
| `vibe-core` | foundation types, Manifest (now incl. `[[skill]]`), Lockfile | GATED + `GATED_PUB_DOCTEST` |
| `vibe-mcp` | MCP server + tools + agent config/skill install + **agentic relay + package-skill projection** | GATED |
| `vibe-cli` | the `vibe` binary facade (now incl. `vibe skill` / `vibe agentic` / `vibe command`) | GATED |
| `vibe-index` `vibe-install` `vibe-resolver` `conform-core` `conform-frontend-rust` `specmap-core` `vibe-registry` `vibe-workspace` `vibe-check` `vibe-publish` `env-audit` | (unchanged this session) | GATED |
| `specmark` / `specmark-grammar` | proc-macros / `spec://` grammar | GATED conform, EXEMPT specmap (bootstrap) |
| `vibe-graph` `vibe-llm` | M0 stubs | EXEMPT (stub) |
| `vibe-wire` | JTD codegen output | EXEMPT (generated) |
| `xtask` | project tooling | EXEMPT (dev tooling) |

## Architectural / policy decisions in force (long form)

- **The four non-negotiable rules** (`CLAUDE.md`, PROP-000 §12): attribution
  (human-authored only), Conventional Commits, group-by-meaning, autonomy
  on routine changes only.
- **PROP-018 — agentic + standalone modes (NEW, in force 2026-06-16).** A
  mode is a choice of *inference backend* (§2.1). agentic = vibevm authors
  a domain-grounded `Intent`, the calling agent executes it (relay backend);
  standalone = vibevm's own backend (algorithmic now, a built-in `vibe-llm`
  engine in §6 far-backlog, for when no agent is present). Skills are
  declared in `[[skill]]` **separately from the four package kinds** (§2.4)
  and projected *out of* the workspace into agents (§2.5 — distinct from
  PROP-003 subskill delivery into the project tree). The relay lives in
  `.vibe/agentic/` (a subdir of the existing `.vibe/` cache, not a new
  dot-dir — §3). Distinct from PROP-006 *session* postures (§1.3).
- **Source is multi-homed (PROP-016).** GitVerse `anarchic/vibevm` + GitHub
  `anarchic-pro/vibevm`, both public + canonical for reading. Roll out with
  **`cargo xtask mirror`** (ff-only, never `--force`), NOT `git push
  origin`. `--check` verifies sync.
- **The package registry is a separate split-host** (PROP-000 §7) — GitHub
  `vibespecs` org, auth `~/.vibevm/github.publish.token`, used only by
  `vibe registry publish`. Orthogonal to source mirrors (different orgs,
  different creds). Token is a surface-secret, never echoed.
- **Two enforcement gates.** conform (`CONFORM_GATED`, a finding fails CI,
  baseline only shrinks) + specmap orphan ratchet (every tagged item must
  `scope!` to a spec unit). resolvo (PROP-017) is the default solver.
- **The Discipline's two laws:** idiomatic inside the file / engineered
  around it; explanation capital must be runnable capital (a rule with no
  checker is a WISH).

## Recent commit chain (newest first)

```
ee9c62e docs(spec): add the General Discovery Prompt v3            (this session)
bd26156 docs(continue): PROP-018 MVP banner                        (this session)
7d5aaaa docs(wal): PROP-018 agentic + standalone modes — MVP checkpoint
050b150 docs(agentic): reframe — vibevm authors the instruction, agent executes
911409e fix(cli): rename relay drain variant for clippy enum-variant-names
aa8b66f docs(mcp): teach the agentic protocol in the vibevm skill
4cbac6c feat(mcp): agentic_explain — the MCP face of the relay
37a67b7 feat(agentic): the relay — vibe agentic explain + vibe command
ae6585e feat(skill): vibe skill — project package skills into agents
e9e17e4 docs(spec): PROP-018 — relay lives in .vibe/agentic, not a new dot-dir
27f511f feat(core): [[skill]] manifest section for agent skills
e7d5cbf docs(spec): PROP-018 — agentic and standalone modes
e7f6a6a docs(continue): forward weak-deps in banner                (prior session)
f1691d5 docs(wal): forward weak-deps done
926ac55 docs(spec): PROP-017 — forward weak-deps done
cf29ebc feat(resolver): recommends best-effort, suggests ignored
dabec2b feat(core): [recommends] + [suggests] manifest schema
3471cc5 docs(continue): port-complete banner
b650075 docs(wal): resolvo port complete — it is the default solver
be17eb7 docs(spec): PROP-017 — record the port complete
f980a16 fix(cli): relocate validate_solver doc above build_install_resolver
ee282e1 feat(cli): --solver override for the resolver fallback
ebfdd94 feat(cli): flip the default solver to resolvo
9b8bc22 feat(resolver): VersionEnumerator over real registries
eafad22 docs(continue): refresh banner — vocabulary complete
```

## Quick-start

```sh
cargo xtask specmap --check              # traceability index + orphan ratchet
cargo xtask conform check                # facts → rules → SARIF → baseline (0/0/0)
cargo xtask test-gate                    # nextest, xfail-strict
cargo xtask health                       # DISCIPLINE-SWEEP snapshot (offline)
cargo xtask mirror                       # fan main+tags to all source mirrors (ff-only)
cargo xtask mirror --check               # verify every mirror is in sync (read-only)
bash tools/self-check.sh                 # via Git Bash, NOT WSL — check $?, not a tail pipe

# PROP-018 surface
vibe skill list                          # skills declared by the project + installed packages
vibe skill install --assume-yes          # project them into agents (--agent / --scope / --skill)
vibe agentic explain                     # park an "explain this project" instruction
vibe command                             # drain the relay: print the parked instruction
```

Session-resume phrase: `восстанови сессию` — **restores state and reports,
then waits for the owner's direction** (the CLAUDE.md contract). The WAL
supersedes this snapshot wherever they diverge.
