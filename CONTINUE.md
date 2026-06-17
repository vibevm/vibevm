# CONTINUE.md — cold-resume checkpoint

> **LATEST (2026-06-16): AGENTIC + STANDALONE MODES (PROP-018) — MVP
> COMPLETE on local `main` (mirror rollout pending). Two product modes on
> one axis — where an operation's reasoning happens. STANDALONE: `vibe
> skill {list,install,uninstall}` projects package-declared `[[skill]]`
> files into coding agents (Claude Code / OpenCode / Codex), reusing the
> PROP-015 agent machinery — no LLM, works agent-present or not. AGENTIC:
> vibevm composes a domain-grounded `Intent` and the calling agent
> executes it — `vibe agentic explain` parks the instruction in
> `.vibe/agentic/command.md`, `vibe command` drains it; the same op is also
> the `agentic_explain` MCP tool (inline return, no mailbox), so one
> operation serves both the one-shot CLI and the persistent MCP server.
> Narrative (owner-reviewed): vibevm authors the trustworthy,
> domain-grounded instruction; the agent is the better in-session executor
> — division of labour by strength, not vibevm offloading for lack of an
> engine. Eight gate-green commits (`27f511f` … `050b150`); full
> `self-check.sh` green; conform 0/0/0; specmap clean. Far backlog
> (PROP-018 §6): built-in `vibe-llm` backend, full conversations, an
> OpenCode-style resumable console, `[[mcp]]` bundled-server install. See
> [`PROP-018`](spec/common/PROP-018-agentic-standalone-modes.md).
> `spec/WAL.md` is canonical and supersedes everything below.**
>
> **PRIOR (2026-06-15): the resolvo resolver
> (PROP-017) is COMPLETE — resolvo (pure-Rust CDCL SAT) is now the
> default production solver. The engine + the full existing dependency
> vocabulary (requires, `[[requires_any]]` disjunctions with
> backtracking, `[conflicts]`, `[obsoletes]`, capabilities via a closure
> pre-scan) are oracle-proven to dominate naive; production version
> enumeration (`MultiRegistryResolver::list_versions`) feeds the real
> providers; and `vibe install/update/reinstall` resolve with
> `ResolvoDepSolver` by default, `--solver <naive|sat|resolvo>` the
> fallback. The forward weak-deps are done too — `[recommends]`
> (post-solve greedy best-effort) and `[suggests]` (never auto-installed);
> `[features.exclusive]` was already in `features.rs`. ~18 resolvo commits
> on both mirrors; full `self-check.sh` green. Everything below describes
> the PRIOR source-mirror session; `spec/WAL.md` is current and supersedes
> it. Far backlog (PROP-017 §8): the reverse weak-deps
> `[supplements]`/`[enhances]`, the capability reverse-index, and the
> `[meta].solver` lockfile field.
> See [`PROP-017`](spec/modules/vibe-resolver/PROP-017-resolvo-resolver.md).**

_Written 2026-06-14 at the close of a **source-mirror hardening** session.
No campaign is in flight: CONVERT-PLAN v0.1 (Phases 0–7) and PUBDOC-DRAIN
v0.1 are both COMPLETE; the PROP-016 source-mirror system is in force and
this session made `cargo xtask mirror` faithful to its spec. Branch `main`
@ `e3546ec`, level with both mirrors (`origin/main` = `github/main` =
`e3546ec`), working tree clean, full gate panel green._

> **`spec/WAL.md` is the canonical living state and its header is current.**
> If this snapshot and the WAL disagree, the WAL wins. Boot first
> (`CLAUDE.md` → `spec/boot/INDEX.md` → its files incl. the two Discipline
> snippets → `spec/WAL.md`), then read this. The **git log is the
> authoritative per-item record** — every commit cites its reasoning.

---

## TL;DR

This session did two things, both small and both verified end-to-end:

1. **`cargo xtask mirror` now self-heals tracking refs after fan-out.**
   Root cause: the fan-out pushes by the *URL* spelled in `mirrors.toml`,
   not by a remote name, and git only advances a remote-tracking ref on a
   push to a *named* remote — so `refs/remotes/origin/main` stayed stale
   and `git status` falsely read "ahead of origin/main" right after a green
   rollout (a manual `git fetch origin` was the cure). Fix: after each
   successful branch push, `mirror` finds every configured remote whose URL
   matches the target and moves its tracking ref up to the just-pushed
   commit via `git update-ref` (no extra network round-trip — the ff-only
   push already guaranteed the host equals local `main`).
2. **The mirror system's marquee safety invariant became runnable
   capital.** "Never `--force`, fast-forward-only" (PROP-016 §6, the
   `CLAUDE.md` Rule 4 red line) was prose only — nothing *checked* it. The
   fan-out's push command now builds in one pure `push_args`, and
   `push_args_never_force` asserts it never emits `--force`/`-f`/a
   `+`-refspec for any ref shape. A rule with no checker is a WISH; this
   one now has a checker.

Two commits (`e4a9353` code, `e3546ec` spec+specmap), rolled out to both
mirrors via the improved `cargo xtask mirror` itself (dogfooded — the
`track origin/main -> e3546ec` lines appeared and `git status` came back
clean with no manual fetch).

## Where work stands

- **Branch `main` @ `e3546ec`**, `0/0` vs `origin/main`; `github/main` also
  `e3546ec`. Both source mirrors level. Working tree clean.
- **No blocker. No campaign in flight.** The WAL says: "the next session
  picks the owner's next goal." DISCIPLINE-SWEEP (`cargo xtask health`) is
  the standing instrument that surfaces the next candidate work.
- **Gate panel** (unchanged by this session — xtask is gate-exempt, the
  PROP-016 edits were prose): `conform check` — **0 frozen / 0 new**
  (baseline EMPTY); `specmap --check` — clean (474 units / 448 items / 459
  edges / 0 suspects / 0 warnings / 0 orphans / 0 dispositioned);
  `CONFORM_GATED = 16`, vibe-core in `GATED_PUB_DOCTEST`; `vibe check`
  0/0/0; full `self-check.sh` green.

## Active blocker & the human action that clears it

**None.** Panel green, tree clean, both mirrors synced. Nothing is waiting
on a human action to proceed. (Standing owner-court items below are
decisions the owner may take when they wish, not blockers.)

## EXACT next-steps recipe (candidate work — the owner chooses)

No plan is mid-execution, so these are *candidates*, not an authorised
queue. Pick one with the owner:

1. **Run the standing sweep and act on its backlog.**
   ```sh
   cargo xtask health            # writes terraform/health/latest.json (offline, deterministic)
   cargo xtask health --mirrors  # + a live mirror-sync probe (network; off by default)
   ```
   At last authoring the sweep flagged: `boot.rs` at the 600 `file-length`
   landmine (+13 in the danger band); **four zero-gap promotion
   candidates** ready to enter `GATED_PUB_DOCTEST` — `conform-core`,
   `conform-frontend-rust`, `env-audit`, `specmark-grammar`; and a
   ~260-type pub-doctest drain backlog led by `vibe-install` (9). Promoting
   a zero-gap crate = append it to `GATED_PUB_DOCTEST`
   (`xtask/src/conform.rs:77`), run `conform check` (expect 0 new), commit.
2. **Take an owner-court decision** (see Standing items) — e.g. publish the
   two Discipline packages, or open the PROP-010 design session.
3. **A fresh owner goal** — boots from this checkpoint with no debt to pay
   down first.

## Non-obvious findings (this session)

- **`git push <url>` does NOT move tracking refs; `git push <remote>`
  does.** This is the whole bug. Tracking refs follow the *fetch* config of
  a *named* remote; a raw-URL push is invisible to them. The fix records
  the post-push SHA with `git update-ref refs/remotes/<remote>/<branch>` —
  equivalent to what a `git fetch` would write, minus the round-trip,
  because an ff-only push that *succeeded* means the host now equals local.
- **Tags need no tracking-ref refresh.** They land in `refs/tags/*`
  directly (global, not per-remote), so only *branch* pushes leave a stale
  `refs/remotes/<remote>/<branch>`. The refresh skips `tags`.
- **xtask is a *recorded* gate exemption, and that is Discipline-compliant.**
  `CONFORM_EXEMPT` (`xtask/src/conform.rs:48`) pairs every non-gated crate
  with a reason; the Discipline's rule is "a deviation with no reason is a
  defect" — xtask's reason ("dev tooling, panics acceptable at the
  developer's console") makes it a decision, not a defect. So I did **not**
  add `scope!`/`#[spec]`/Class-F machinery to `mirror.rs` — no xtask module
  carries them, and adding them would break uniformity and contradict the
  record. The *right* Discipline move for exempt tooling is the pure-fn +
  unit-test (`push_args` + `push_args_never_force`), not gate ceremony.
- **Two gates, not one — a crate can be gated by one and exempt the other.**
  `specmark`/`specmark-grammar` are `CONFORM_GATED` (their code obeys
  no-unwrap etc.) yet **specmap-ratchet EXEMPT** — they are the tagging
  machinery itself and cannot depend on `specmark` to carry `scope!`
  markers (a bootstrap problem). conform reports "4 exempt", specmap "6
  exempt"; the delta is exactly this pair.
- **Machine quirks (unchanged, still true):** PowerShell 5.1 corrupts
  UTF-8-no-BOM round-trips → edit via the Edit/Write tools, never PS
  `Set-Content`; `bash` in PowerShell is WSL, so `self-check.sh` must run
  through **Git Bash** (the Bash tool here is Git Bash); `git commit` via
  `-F - <<'MSG'` heredoc only (backtick `-m` double-corrupted messages
  before); Windows UAC blocks test exes named `*install*`. When checking a
  gate's exit code, read `$?` — a `| tail` pipe masks the real code.

## Repository map

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = `cargo xtask`
├─ CLAUDE.md / AGENTS.md / GEMINI.md   identical; the 4 rules + boot pointer
├─ VIBEVM-SPEC.md            owner-frozen implementation spec (edits need the owner)
├─ MEMORY.md                 pointer to spec/boot/90-user.md
├─ CONTINUE.md               this cold-resume snapshot
├─ mirrors.toml              PROP-016 source-mirror target registry (gitverse + github)
├─ conform-baseline.json     conform ratchet baseline — EMPTY (0 frozen)
├─ specmap.json              traceability index (474 units / 448 items / 459 edges)
├─ specmap-ratchet.json      orphan ratchet exempt list (6 exempt)
├─ crates/                   19 library/bin crates (see gate split below)
├─ xtask/                    project tooling crate (gate-EXEMPT), at workspace root
├─ spec/                     the spec tree — the source of truth
│   ├─ boot/                 session-boot files (INDEX.md manifest, 00-core, 90-user)
│   ├─ common/               PROP-000…016 (cross-cutting decisions)
│   ├─ modules/              per-module PROP/FEAT (vibe-registry, vibe-mcp, …)
│   ├─ discipline/           ENGINE-CONFORM, BROWNFIELD, the Discipline corpus
│   ├─ terraforms/           campaign plans (CONVERT-PLAN, PUBDOC-DRAIN, DISCIPLINE-SWEEP)
│   └─ WAL.md                canonical living state (checkpoint, rewritten each session)
├─ vibedeps/                 installed packages (flow-discipline-core, stack-rust-ai-native)
├─ fixtures/registry/        hermetic test registry (never touches a real host)
├─ tools/                    self-check.sh, jtd-codegen
├─ schemas/                  JTD schemas — the vibe-wire codegen INPUT (the taggable unit)
├─ refs/                     book/ (read-only owner reference) + src/ (cargo/uv/spec-kit)
└─ terraform/                registry/ (test+debt baselines), health/ (sweep snapshot)
```

**The crates and their gate status** (GATED = held to the full conform
rule set with a zero-new-findings ratchet; see `xtask/src/conform.rs`):

| Crate | Holds | Gate |
|---|---|---|
| `vibe-core` | foundation types: 7 newtypes (RelPath, PackageName, ContentHash…), Manifest, Lockfile, UserConfig | GATED + `GATED_PUB_DOCTEST` |
| `vibe-index` | in-RAM index + registry server (TokenStore, RateLimiter, AppState) + scanner | GATED |
| `vibe-install` | install orchestrator (plan → apply) | GATED |
| `vibe-resolver` | dependency solver (NaiveDepSolver; SAT is a future cell) | GATED |
| `conform-core` | conformance rules + fact store + ratchet baseline | GATED |
| `conform-frontend-rust` | `syn`-based fact frontend | GATED |
| `specmap-core` | traceability index generation (specmap.json) | GATED |
| `vibe-registry` | registry domain (search, vendor, redirect-sync, git backends) | GATED |
| `vibe-workspace` | workspace / manifest / boot model (BootBand) | GATED |
| `vibe-check` | `vibe check` validation | GATED |
| `vibe-publish` | publisher (RepoCreator adapters, token redaction, redirect_sync) | GATED |
| `env-audit` | the designated env-mutation audit crate (serialized EnvGuard) | GATED |
| `vibe-cli` | the `vibe` binary facade | GATED |
| `specmark` | the `#[spec]`/`#[cell]`/`#[verifies]`/`scope!` proc-macros | GATED conform, EXEMPT specmap (bootstrap) |
| `specmark-grammar` | the `spec://` URI grammar (Verb, SpecUri, EdgeSpec) | GATED conform, EXEMPT specmap (bootstrap) |
| `vibe-mcp` | MCP server + tools + agent detection/config + skill install | GATED |
| `vibe-graph` | M0 stub — task-graph runner, unbuilt | EXEMPT (stub) |
| `vibe-llm` | M0 stub — LLM providers, land in v1.5 | EXEMPT (stub) |
| `vibe-wire` | generated code (JTD codegen output) | EXEMPT (generated) |
| `xtask` | project tooling (codegen, specmap, conform, mirror, health…) | EXEMPT (dev tooling) |

## Architectural / policy decisions in force (long form)

- **The four non-negotiable rules** (`CLAUDE.md`, PROP-000 §12): (1)
  *attribution* — this repository is human-authored; never mark any
  artefact as machine-authored (the rule's own paragraph is the only place
  the topic is discussed). (2) *Conventional Commits* — short imperative
  subject, body explaining *why*. (3) *Group commits by meaning* — one
  logical unit per commit. (4) *Autonomy on routine changes only* — routine
  work commits+pushes without asking; history rewrites, force-push, large
  blobs, CI/signing/secrets changes stop and ask.
- **Source is multi-homed (PROP-016, in force 2026-06-14).** GitVerse
  `anarchic/vibevm` and GitHub `anarchic-pro/vibevm` are both public and
  canonical for reading (RU↔GitVerse, US↔GitHub). Model: benevolent
  dictator / hub-and-spoke — mainline is the maintainer's single-writer
  local `main`; every host is a downstream read-replica. **Roll out with
  `cargo xtask mirror` (reads `mirrors.toml`, ff-only, never `--force`),
  NOT `git push origin`** (which only hits GitVerse). `--check` verifies
  sync; `--from <host>` pulls a host's accepted-PR merge into mainline
  first. After this session, fan-out also refreshes local tracking refs.
- **The package registry is a *separate* split-host** (PROP-000 §7,
  PROP-002 §2.10) — published packages live in the GitHub `vibespecs` org,
  auth is `~/.vibevm/github.publish.token` used *only* by `vibe registry
  publish`, scoped strictly to `vibespecs`. Source mirrors (SSH keys,
  `anarchic-pro`) and the registry (publish token, `vibespecs`) are
  orthogonal — different orgs, different creds. **Token discipline:** the
  publish token is a surface-secret, never echoed anywhere.
- **Two enforcement gates.** (a) The **conform gate** (`CONFORM_GATED`,
  `xtask/src/conform.rs`) — gated crates obey the full rule set
  (seam-doctests, error-enum/message-cites-req, no-unwrap-in-domain,
  ambient-env; plus universal file-length≤600, cell-isolation,
  cell-has-oracle, unsafe-gate). A new finding fails CI; the baseline only
  shrinks. (b) The **specmap orphan ratchet** (`specmap-ratchet.json`) —
  every tagged `#[cell]`/item must `scope!` to a spec unit; no orphans. A
  crate not in a ratchet's `exempt` list is gated by it.
- **DISCIPLINE-SWEEP v0.1 (standing instrument).** `cargo xtask health` is
  a no-LLM, deterministic fact collector reusing the conform frontend;
  emits `terraform/health/latest.json` (per-crate doctest coverage, the
  file-length danger band, the pub-doctest drain/promotion backlog, the
  deviation-debt census). It guides; the gates remain truth.
- **The Discipline's two laws** (`vibedeps/flow-discipline-core`): (1)
  idiomatic inside the file, engineered around the file — strictness lives
  in types/contracts/metadata/verification, not in an invented dialect; (2)
  explanation capital must be runnable capital — prose that could be a
  checker/doctest/typed API is a WISH until it is one.

## Recent commit chain (newest first)

```
e3546ec docs(spec): PROP-016 records the tracking-ref refresh        (this session)
e4a9353 feat(xtask): mirror refreshes tracking refs after fan-out    (this session)
d1e62f8 docs(boot): 90-user.md — multi-homed source, GitHub SSH, mirror rollout
3c0924e docs(spec): PROP-016 — decentralized source-mirror integration model
141cdde feat(xtask): health gains an optional --mirrors sync probe
5a1d313 feat(xtask): mirror — fan mainline out to all targets
c3d2ad2 docs(wal): record the standing DISCIPLINE-SWEEP instrument
9879321 docs(spec): DISCIPLINE-SWEEP v0.1 - the standing guardian
e6f188f feat(xtask): health collector for the discipline sweep
91bc763 docs(wal): pub-doctest debt drained — conform baseline at zero
489df90 docs(spec): PUBDOC-DRAIN v0.1 carries its execution record
53021b6 docs(core): B8 — purl and user-config types; baseline reaches zero
dbe8415 docs(core): B7 — document, redirect, i18n types teach by doctest
51b2e72 docs(core): B6 — non-registry dep declares teach by doctest
a014232 docs(core): B5 — lockfile types teach by doctest
eceabbc docs(core): B4 — subskill manifest types teach by doctest
3f9707f docs(core): B3 — consumer-side sections teach by doctest
873820b docs(core): B2 — package-role sections teach by doctest
f0067cc docs(core): B1 — foundation types teach by doctest
34d9fba docs(spec): PUBDOC-DRAIN v0.1 - drain the pub-doctest debt
13bb61c docs(wal): CONVERT-PLAN v0.1 complete — Phases 0-7
581d39f feat(mcp): vibe-mcp joins both gates — DBT-0020 closed (Phase 7.4)
34c3517 refactor(cli): split mcp.rs into a commands/mcp/ module family
fddc337 docs(wal): refine the 7.3d-ii split recipe (mcp.rs 1471)
9da4e24 test(mcp): agent-profile tests relocate to vibe-mcp
```

## Quick-start

```sh
cargo xtask specmap --check              # traceability index + orphan ratchet
cargo xtask conform check                # facts → rules → SARIF → baseline (0 frozen / 0 new)
cargo xtask conform freeze               # rewrite baseline (legal: new rule, or reviewed shrink)
cargo xtask test-gate                    # nextest, xfail-strict
cargo xtask fast-loop --enforce-budget   # per-cell first-signal < 60s
cargo xtask health                       # DISCIPLINE-SWEEP snapshot (offline)
cargo xtask mirror                       # fan main+tags to all source mirrors (ff-only)
cargo xtask mirror --check               # verify every mirror is in sync (read-only)
bash tools/self-check.sh                 # via Git Bash, NOT WSL — check $?, not a tail pipe
```

Session-resume phrase: `восстанови сессию` — **restores state and reports,
then waits for the owner's direction** (the CLAUDE.md contract). The WAL
supersedes this snapshot wherever they diverge.
