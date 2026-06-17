# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-17 (session save). Branch `main` @ `47dbd2a`, level with both
mirrors (`gitverse` = `anarchic/vibevm`, `github` = `anarchic-pro/vibevm`).
Working tree clean. A Discipline-Sweep grammar refactor of the new features is
**PAUSED at the P2 boundary** — the owner cleared the "to the end" goal, so the
P3–P6 work below is the documented continuation, **not** a standing mandate:
resume only on explicit owner direction._

> **`spec/WAL.md` is the canonical living state; its "Active campaign" section
> is authoritative for this refactor.** If this snapshot and the WAL disagree,
> the WAL wins. Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files →
> `spec/WAL.md`), then read this. The **git log is the authoritative per-item
> record** — every campaign commit cites its sweep §ref.

---

## TL;DR

vibevm's two newest features — the VVM (`vibe man`, PROP-019) and the agentic /
skill surface (PROP-018) — are being driven deeper into the AI-Native
Discipline by the standing [`DISCIPLINE-SWEEP-v0.1`](spec/terraforms/DISCIPLINE-SWEEP-v0.1.md),
run as a phase-gated RAID. The **foundation landed gate-green and is on both
mirrors**: P0 (stale-doc fixes), P1 (mechanical Tier-1 wins), P2 (Class-B
newtypes) — nine commits on top of the prior checkpoint `38eef21`. The owner
then cleared the goal, pausing the campaign at the P2 boundary. The headline
phase **P3 (Class-F error enums)** plus P5/P4/P6 remain, fully specified below.

## Where work stands

- **Branch `main` @ `47dbd2a`**, both mirrors in sync, working tree clean.
- **Campaign PAUSED** — owner cleared the goal 2026-06-17. Not abandoned;
  documented for a clean resume, but no longer auto-driven.
- **Landed (each gate-green, mirrored):**
  - **P0** (`498ec15`) — corrected stale module docs (`cli/man.rs` claimed
    verbs "land in later slices"; `vibe-mcp/lib.rs` claimed 2 tools + an
    `Fn` registry, really 4 tools behind the `McpTool` trait).
  - **P1** — tests-out split of `man/mod.rs` 583→525, out of the `[540,600]`
    danger band (`5e2cae4`); a single `ForcedKind` `#[command(flatten)]`
    replacing 4 copied `--tag/--branch/--commit` triplets (`28e854c`);
    pub-doctest gate widened to 4 zero-gap crates (`1a1013d`); a `require_tty`
    helper for the remove/gc pickers (`8b21cf7`).
  - **P2 (Class-B newtypes)** — closed `Mirror` enum (`cb4abd4`);
    `InstallRecord.profile: Profile` not `String`, closing validate-then-discard
    (`acfaed8`); one `short_commit` not two (`e02a0d3`).
- **Gate state:** the full Tier-0 floor was green at the P1 boundary
  (self-check exit 0; conform 0/0/0; specmap 545u / 546e / 0 orphans;
  test-gate xfail-strict; fast-loop 20/20). Each P2 commit was verified
  (clippy `-D warnings` + the affected tests + conform + specmap). The next
  full floor is due at the P3 boundary.

## Active blocker & the human action that clears it

**None.** Tree clean, mirrors synced. The campaign is paused by owner choice;
the only open decision is whether/when the owner resumes P3.

## EXACT resume recipe (only if the owner resumes the campaign)

The WAL "Active campaign" section is authoritative; this mirrors it. Resume
order (value-ordered; the owner explicitly chose **maximal scope** — implement
the affinity dispatcher, full doctest drain + flips for both crates):

1. **P3 — Class-F error enums (the headline).** One `thiserror` enum per
   fallible domain layer, each `#[spec(implements = "spec://…")]` with every
   `#[error("…")]` ending in the Class-F tail `(violates spec://…; fix: …)`;
   `anyhow` stays only at the binary edge (`main.rs` / command dispatch). The
   whole new-feature surface is `anyhow`-only today, so the `err-req`/`err-msg`
   gates pass vacuously — this phase makes them bite. Layers → anchors (all
   confirmed `req`-marked, no `pin-into-unmarked-unit` risk):
   - `model.rs` → `ModelError` — `Selector::parse` (EmptySelector, #selectors),
     `Profile::parse` (UnknownProfile, #build). Callers' `?` auto-converts to
     anyhow → no caller edits. **First verify `thiserror` is a `vibe-cli`
     dependency** (present in vibe-mcp; check vibe-cli's `Cargo.toml`, add if
     absent).
   - `store.rs` → `StoreError` (#layout); `placer.rs` → `PlaceError`
     (#instances); `source.rs` → `ResolveError` (#selectors / #provenance);
     `git.rs` → `GitError` (#build) + tighten its `pub` fns to `pub(crate)`;
     `mod.rs` → `ManError` (#surface) for the ~6 user-facing decisions
     (NotInstalled, NoActiveVersion, UnknownMirror, …).
   - PROP-018: `agentic.rs` → `RelayError` (#relay); `pkgskill.rs` →
     `PackageSkillError` (#vibe-skill).
   - Model them on `vibe-mcp::{ToolError, ServerError}` (lib.rs) — the existing
     Class-F exemplar (every message already carries the violates/fix tail).
   - Optional contract-symmetry: `VersionId::parse`/`from_validated` split,
     deferred from P2 to land with `ModelError`.
2. **P5 — PROP-018 grammar.** Implement the §2.3 affinity dispatcher (req r2,
   owner chose "implement") + typed `AffinityError` naming the right backend;
   route the MCP path through `BackendOutcome` (unify the two §2.8 transports);
   add a `skill_template.md` ↔ `default_tools()` cross-check test (assert every
   tool the template names is actually served); dedup the byte-duplicated
   `resolve_project_root` (in `commands/agentic/mod.rs` and
   `commands/skill/mod.rs`). PROP-018 newtypes belong here too: IntentStatus
   markers (the `pending`/`done` literals in `agentic.rs`), `SkillOrigin`
   enum (`skill/mod.rs`), `SkillStatus` enum (`pkgskill.rs` **with**
   `install.rs` — cross-cutting, do both or neither).
3. **P4 (LAST) — Class-G doctest drain + gate flips (maximal).** Relocate the
   canonical `#[cfg(test)]` examples into rustdoc doctests on the public types
   (TOML round-trip for serde structs; parse one-liner for string-shaped;
   variant/`Default` for bare enums; construct-and-Display for error enums),
   then add `vibe-mcp` (22-type gap) and `vibe-cli` (83-type gap, live `health`
   figure) to `GATED_PUB_DOCTEST` in `xtask/src/conform.rs`. Do it last so it
   documents the final post-P3/P5 types. Per crate: doctests →
   `cargo test -p <c> --doc` → `conform check` (deletions-only freeze if any) →
   `cargo xtask specmap` regen → topic commit.
4. **P6 — REPORT + checkpoint.** Refresh `terraform/health/latest.json` (its
   git diff is the health delta), bump the WAL standing line, write the closing
   sweep REPORT, mirror.

Per-phase discipline: edit via Edit/Write only; run `cargo fmt --all` after any
argument-collapsing edit (it tripped the floor once); gate each phase with the
full Tier-0 floor (`self-check.sh` via **Git Bash** + conform + specmap +
test-gate + fast-loop); topic commits per Rule 3 citing
`spec://vibevm/terraforms/DISCIPLINE-SWEEP-v0.1#tierN`; mirror only with the WAL
already updated (never mirror past a stale WAL — the lag this session opened
with).

## Non-obvious findings (this campaign)

- **`CommitHash` newtype considered and DECLINED** (rationale in `e02a0d3`):
  a recorded commit only ever arrives from trusted git output / `state.toml`,
  so the newtype would have no untrusted parse boundary — ceremony without an
  invariant, which the Discipline (and the audit's own skeptic flag) warns
  against. The real, non-ceremonial win — unifying the two `short_commit`
  functions — was taken instead.
- **Open anomaly (task #13)** — a `cargo test` / xtask run once rewrote the
  tracked file `docs/guides/agent-mcp-quickstart-opencode.md` (a `flow-wal` →
  FQDN `org.vibevm.wal` edit). Restored in P0; did **not** recur during the
  P1 full floor (which ran `cargo test --workspace`), so it is not a
  deterministic `cargo test` write — suspect `cargo xtask health` or a
  non-deterministic test. Bisect when convenient and file to
  `terraform/registry/debt.json`.
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`
  (UTF-8 round-trip corruption); `git commit` via `-F - <<'MSG'` heredoc;
  `self-check.sh` through Git Bash, never WSL; mirrors via `cargo xtask
  mirror` (ff-only), never `git push origin`; `core.filemode=false`.

## Repository map

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = `cargo xtask`
├─ CLAUDE.md / AGENTS.md / GEMINI.md   identical; the 4 rules + boot pointer
├─ CONTINUE.md               this cold-resume snapshot
├─ specmap.json              traceability index (545 units / 546 edges)
├─ crates/
│   ├─ vibe-cli/src/commands/man/   THE VVM MODULE (PROP-019) — see table
│   │   └─ tests.rs          NEW (P1): tests-out of mod.rs, carries its scope!
│   └─ vibe-mcp/src/{agentic,pkgskill}.rs   PROP-018 relay + skill projection
├─ spec/
│   ├─ common/PROP-019-version-manager.md   VVM design (v2); anchors req-marked
│   ├─ common/PROP-018-agentic-standalone-modes.md
│   ├─ terraforms/DISCIPLINE-SWEEP-v0.1.md  the standing recurring sweep
│   └─ WAL.md                canonical living state (+ "Active campaign" section)
├─ terraform/                health/ (collector), registry/ (debt/baselines), golden/
├─ tools/                    self-check.sh, first-run.{sh,ps1}
└─ xtask/src/conform.rs      CONFORM_GATED / GATED_PUB_DOCTEST / ENV_ROOTS consts
```

**The VVM (PROP-019) lives in `crates/vibe-cli/src/commands/man/`** (post-P1/P2
line counts): `mod.rs` 525 (dispatch + read verbs + install/use/env/doctor;
`ManEnv`; selector resolution) · `env.rs` 486 (shims + `EnvPersister`;
`path_with_prefix`) · `remove.rs` (remove + gc; `require_tty` callers) ·
`source.rs` (`Mirror` enum; find/clone/resolve; `external_path`) · `model.rs`
(Kind, VersionId, Selector, `Profile` [now serde], Origin, InstallRecord,
State) · `store.rs` (layout + `current` + state.toml) · `install.rs`
(`perform_install`) · `placer.rs` (diff-copy) · `tools.rs` · `selfloc.rs`
(`derive_self`) · `builder.rs` (`CargoBuilder` + the single `short_commit`) ·
`git.rs` · `tests.rs`. `vibe vars` = `commands/vars.rs` + `cli/vars.rs`.

## Architectural / policy decisions in force

- **The four non-negotiable rules** (`CLAUDE.md`, PROP-000 §12): attribution
  (human-authored only), Conventional Commits, group-by-meaning, autonomy on
  routine changes only.
- **PROP-019 VVM v2** (in force 2026-06-17): install/switch unit = a whole
  immutable instance; active version = the live `current` pointer; a managed
  `vibe` derives root/home from `current_exe`; diff-copy placement; shim dir
  prepended to PATH; derived paths plain (no `\\?\`).
- **Source is multi-homed** (PROP-016): gitverse + github, both canonical;
  roll out with `cargo xtask mirror` (ff-only), never `git push origin`.
- **The package registry is a separate split-host** (PROP-000 §7), github
  `vibespecs`, used only by `vibe registry publish`; token never echoed.
- **Two enforcement gates** — conform (a finding fails CI; baseline only
  shrinks) + specmap orphan ratchet. resolvo (PROP-017) is the default solver.
- **The Discipline Sweep** ([`DISCIPLINE-SWEEP-v0.1`](spec/terraforms/DISCIPLINE-SWEEP-v0.1.md))
  is the standing recurring guardian above the gates: collector-first
  (`cargo xtask health`); gates are the floor, the collector a guide.

## Recent commit chain (newest first)

```
47dbd2a docs: checkpoint the grammar-refactor campaign (P0-P2 landed)   (this save's base)
e02a0d3 refactor(cli): one short_commit, not two                        (P2)
acfaed8 refactor(cli): store the build profile as Profile, not String   (P2)
cb4abd4 refactor(cli): a closed Mirror enum for the source-mirror vocabulary (P2)
8b21cf7 refactor(cli): extract require_tty for the remove/gc pickers     (P1)
1a1013d build(conform): widen the pub-doctest gate to 4 zero-gap crates  (P1)
28e854c refactor(cli): flatten the forced-kind args into one ForcedKind  (P1)
5e2cae4 refactor(cli): split man dispatch tests out of mod.rs            (P1)
498ec15 docs: correct stale module docs in vibe-cli man + vibe-mcp       (P0)
38eef21 docs(continue): cold-resume refresh @ 7550cde                    (prior checkpoint)
1a03d57 docs(wal): refresh checkpoint — record two shim fixes
7550cde fix(cli): strip the Windows \?\ verbatim prefix from derive_self
b22edd9 fix(cli): prepend the VVM shim dir on PATH so it wins
567efce docs(continue): cold-resume checkpoint — VVM v2
705251c docs(wal): session save — VVM v2 current phase
c6e65bf docs(readme): document the VVM first run
eecb46e chore(tools): add first-run bootstrap scripts
f106683 feat(cli): VVM v2 — git-incremental clone + linked rebuild
8910f8e feat(cli): vibe vars — reconcile actual vs environment
f70a922 feat(cli): VVM v2 — current_exe truth + stale-env warning
34c8250 feat(cli): VVM v2 core — instances, live current, diff-copy
d6b1039 docs(spec): PROP-019 v2 — instances, live current, diff-copy
```

## Quick-start

```sh
# Tier-0 floor (run before any sweep work — never sweep on a red tree)
bash tools/self-check.sh                 # via Git Bash, NOT WSL — check $?, not a tail pipe
cargo xtask conform check                # 0 new against the baseline (0/0/0)
cargo xtask specmap --check              # 0 suspects / warnings / gated orphans
cargo xtask test-gate                    # nextest, xfail-strict
cargo xtask fast-loop --enforce-budget   # every cell builds+tests < 60s

cargo xtask health                       # advisory facts → terraform/health/latest.json
cargo xtask mirror --check               # verify both mirrors are in sync
cargo xtask mirror                       # fan main+tags to both mirrors (ff-only)
```

Session-resume phrase: `восстанови сессию` — restores state and **reports, then
waits for direction** (the CLAUDE.md contract). With the goal cleared, the
campaign does not auto-resume; the WAL "Active campaign" section names the
candidate next step (P3) for that report. The WAL supersedes this snapshot
wherever they diverge.
