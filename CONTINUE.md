# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-17 (P6 checkpoint). The **Discipline-Sweep grammar-refactor
RAID is COMPLETE** — P0–P6 landed gate-green on `main` and were pushed to both
mirrors (`origin`=gitverse `anarchic/vibevm`, `github`=`anarchic-pro/vibevm`).
Working tree clean. There is **no pending campaign work**; the only open items
are owner-level deferrals (below), not a standing mandate._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record** —
> every campaign commit cites its sweep §ref. The run's close-out is
> [`terraform/discipline-sweep/REPORT-2026-06-17-grammar-refactor.md`](terraform/discipline-sweep/REPORT-2026-06-17-grammar-refactor.md).
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

vibevm's two newest features — the VVM (`vibe man`, PROP-019) and the agentic /
skill surface (PROP-018) — were driven deeper into the AI-Native Discipline by a
phase-gated RAID run of the standing [`DISCIPLINE-SWEEP-v0.1`](spec/terraforms/DISCIPLINE-SWEEP-v0.1.md).
This session landed **P3 (Class-F error enums), P5 (PROP-018 grammar — the
affinity dispatcher + unified transports), P4 (vibe-mcp pub-doctest drain + gate
flip), and P6 (this checkpoint)** on top of the prior P0–P2 base `47dbd2a`. A
full Tier-0 floor ran green at every phase boundary. Three items were
deliberately deferred/declined with recorded reasons (see Next steps).

## Where work stands

- **Branch `main`**, campaign tip is the P6 checkpoint commits; both mirrors in
  sync after the P6 push; working tree clean.
- **Campaign COMPLETE** — no auto-driven work remains.
- **Gate state at close:** conform 0/0/0 (16 gated, 4 exempt); `GATED_PUB_DOCTEST`
  = 6 (vibe-mcp added); specmap clean (545 units / 561 edges / 0 orphans);
  test-gate green (1204, xfail-strict); fast-loop 20/20; full `self-check.sh`
  green (fmt, all tests + doctests, clippy `-D warnings`, `vibe check` 0/0/0).

## What landed (P3–P6, this session)

- **P3 — Class-F error enums.** One `thiserror` enum per fallible domain layer,
  each `#[spec(implements=…)]` with the `(violates spec://…; fix: …)` tail;
  `anyhow` stays only at the binary edge. vibe-cli: `ModelError`, `StoreError`,
  `PlaceError`, `ResolveError`, `GitError` (+ `pub`→`pub(crate)`), `ManError`
  (in a new `man/error.rs` cell, so `mod.rs` stays out of the file-length danger
  band). vibe-mcp: `RelayError`, `PackageSkillError`. The callers
  (`install.rs`, `remove.rs`) needed no edits — `?` auto-converts at the edge.
- **P5 — PROP-018 grammar.** Affinity dispatcher (`ActiveBackend`,
  `check_affinity`, typed `AffinityError`); MCP path unified through the
  `InferenceBackend` seam (`BackendOutcome::Inline` + `InlineBackend`);
  `resolve_project_root` deduped; skill_template↔`default_tools` cross-check
  test; `IntentStatus` newtype; `preview_status` shared dry-run projection.
- **P4 — Class-G.** vibe-mcp's 27 public types drained to compiled doctests
  (all run under `cargo test -p vibe-mcp --doc`), then armed in
  `GATED_PUB_DOCTEST`.
- **P6 —** this checkpoint, the REPORT, the `health` refresh, debt entries, the
  mirror.

## Active blocker & the human action that clears it

**None.** Tree clean, mirrors synced, all gates green.

## Next steps (owner-directed — NOT a standing mandate)

The campaign is done. The only open work is the deferrals it recorded; pick up
only on explicit owner direction:

1. **vibe-cli pub-doctest (DBT-0021).** vibe-cli is a bin crate with **no lib
   target**, so `cargo test --doc` cannot compile its doctests — gating it would
   enforce uncompiled prose (a Law-2 violation). To gate it: give vibe-cli a
   `[lib]` target (so doctests run), or tighten its internal types to
   `pub(crate)` (a bin's `pub` is not an API), which shrinks the 86-type gap
   legitimately. Both are structural; the owner chooses. *(This is the one part
   of the stated "maximal scope incl. vibe-cli" not delivered — blocked by the
   empirical bin-crate constraint.)*
2. **`SkillStatus` newtype.** The 9-value install/uninstall status vocabulary is
   shared and behaviorally matched across four serialized report types in two
   crates (PROP-015 + PROP-018) and the vibe-cli mcp walkers. Typing it is a
   wire-contract + scope/naming design call. The contained win (the duplicated
   dry-run transform) is already taken as `preview_status`.
3. **`SkillOrigin`** was declined outright (display-only label, no invariant).

## Non-obvious findings (this campaign)

- **bin-crate doctests do not compile.** `cargo test -p vibe-cli --doc` →
  "no library targets found in package `vibe-cli`". A bin crate's doctests are
  never run by cargo, so the `pub-doctest` gate (which keys on *presence* of a
  doctest) would pass while the examples rot uncompiled. The gate's value
  presumes a lib target.
- **rustfmt re-wraps multi-line `use` / `let-else` / `map_err`.** Two `style(…)`
  fixup commits resulted; run `cargo fmt --all` *before* committing argument-
  collapsing edits, not after.
- **Task #13 (the `agent-mcp-quickstart-opencode.md` rewrite) is resolved**:
  the file was committed with correct fqdn content (`8065afb`, owner) and the
  stray rewrite did not recur across this run's test passes (DBT-0022, dormant).
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`;
  `git commit` via `-F - <<'MSG'`; `self-check.sh` through Git Bash; mirrors via
  `cargo xtask mirror` (ff-only), never `git push origin`.

## Repository map

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = `cargo xtask`
├─ CLAUDE.md / AGENTS.md / GEMINI.md   identical; the 4 rules + boot pointer
├─ CONTINUE.md               this cold-resume snapshot
├─ specmap.json              traceability index (545 units / 561 edges)
├─ crates/
│   ├─ vibe-cli/src/commands/man/   THE VVM MODULE (PROP-019); error.rs = NEW (P3)
│   │   error enums: model/store/placer/source/git/error.rs (Class-F)
│   └─ vibe-mcp/src/   agentic.rs (relay + affinity dispatcher), pkgskill.rs,
│       install.rs, tools.rs, jsonrpc.rs, transport.rs — all pub-doctested (P4)
├─ spec/
│   ├─ common/PROP-019-version-manager.md / PROP-018-agentic-standalone-modes.md
│   ├─ terraforms/DISCIPLINE-SWEEP-v0.1.md   the standing recurring sweep
│   └─ WAL.md                canonical living state (+ "Active campaign" section)
├─ terraform/
│   ├─ discipline-sweep/REPORT-2026-06-17-grammar-refactor.md  this run's close-out
│   ├─ health/latest.json    advisory collector snapshot (refreshed P6)
│   └─ registry/debt.json    DBT-0021 (vibe-cli pub-doctest), DBT-0022 (task #13)
└─ xtask/src/conform.rs      CONFORM_GATED / GATED_PUB_DOCTEST / ENV_ROOTS consts
```

## Architectural / policy decisions in force

- **The four non-negotiable rules** (`CLAUDE.md`, PROP-000 §12): attribution
  (human-authored only), Conventional Commits, group-by-meaning, autonomy on
  routine changes only.
- **Class-F error grammar:** one `thiserror` enum per fallible layer,
  `#[spec(implements)]` + the `(violates spec://…; fix: …)` message tail;
  `anyhow` only at the binary edge / polymorphic seams.
- **PROP-018 backends:** affinity is enforced by `check_affinity` (not just
  declared); the two §2.8 transports (CLI file-relay, MCP inline) share one
  `InferenceBackend` seam and yield a `BackendOutcome`.
- **Newtypes earn their place by an invariant or a behavioral branch**, not by
  display alone — `SkillOrigin` declined, `CommitHash` (P2) declined, on that test.
- **Source is multi-homed** (PROP-016): gitverse + github, both canonical; roll
  out with `cargo xtask mirror` (ff-only), never `git push origin`.
- **The Discipline Sweep** is the standing recurring guardian above the gates
  (collector-first; gates are the floor, the collector a guide).

## Recent commit chain (newest first, this run on top)

```
(P6 checkpoint commits — REPORT/health/debt, WAL, CONTINUE)
2cc6ab2 chore(specmap): regen for the vibe-mcp doctest line shifts   (P4)
dbe593d build(conform): arm pub-doctest on vibe-mcp                  (P4)
273e58f docs(mcp): the server surface types teach by doctest        (P4)
c5fff3a docs(mcp): the agentic relay types teach by doctest         (P4)
5f74dc4 chore(specmap): regen for the P5 PROP-018 grammar           (P5)
e71827f style(cli): rustfmt the agentic import block                (P5)
4861cba refactor(mcp): share the dry-run status projection          (P5)
7cf57c5 refactor(mcp): IntentStatus newtype for the relay mailbox   (P5)
4e249cc feat(mcp): affinity dispatcher + unified agentic transports (P5)
dee16dc test(mcp): cross-check served tools against the usage skill (P5)
cfdf213 refactor(cli): one resolve_project_root, not two            (P5)
8c5c8fa chore(specmap): regen for the P3 Class-F error enums         (P3)
7d59f4e style(cli,mcp): rustfmt the P3 error-enum edits              (P3)
ce3d0e3 refactor(mcp): PackageSkillError for the vibe-skill projection (P3)
931f805 refactor(mcp): RelayError for the agentic relay mailbox      (P3)
a8f07d8 refactor(cli): ManError for the man command-surface decisions (P3)
7d1ee8e refactor(cli): ResolveError for the source-resolution layer  (P3)
4fd8b3d refactor(cli): PlaceError for the diff-copy placement layer  (P3)
a03dde6 refactor(cli): StoreError for the version-store layer        (P3)
9b90f55 refactor(cli): ModelError for the selector/profile parse boundary (P3)
8fb5bf2 refactor(cli): GitError for the man git seam                 (P3)
8065afb docs(guides): point the flow:wal probe at the fqdn repo      (owner, concurrent)
3092f34 docs(continue): session-save cold-resume — campaign paused at P2  (prior base)
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
waits for direction**. With the campaign complete, there is no candidate next
step beyond the owner-directed deferrals above. The WAL supersedes this snapshot
wherever they diverge.
