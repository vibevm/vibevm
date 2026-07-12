# Discipline Sweep — grammar refactor of the new features (RAID REPORT)

_Run 2026-06-17, branch `main`. One RAID run of the standing
[`DISCIPLINE-SWEEP-v0.1`](../../spec/terraforms/DISCIPLINE-SWEEP-v0.1.md),
sweeping the two newest features — the VVM (`vibe man`, PROP-019) and the
agentic / skill surface (PROP-018) — deeper into the AI-Native Discipline.
Base tip `47dbd2a` (P0–P2, the prior checkpoint); this run landed P3–P6 on
top. The §8 close-out: phase ledger, what landed, the gate state, the
deferrals with their reasons, and the honest-mistakes list._

> The git log is the authoritative per-item record — every campaign commit
> cites `spec://vibevm/terraforms/DISCIPLINE-SWEEP-v0.1#tierN` and its sweep
> item. The WAL is the canonical living state; this REPORT is the run's
> close-out snapshot.

## Phase ledger

| Phase | What | Status | Tier |
|---|---|---|---|
| P0 | stale module-doc fixes | landed earlier (`498ec15`) | §2b |
| P1 | mechanical Tier-1 wins (tests-out split, ForcedKind, gate-widen, require_tty) | landed earlier | §1a/§1b |
| P2 | Class-B newtypes (`Mirror`, `Profile`, one `short_commit`) | landed earlier | §3a |
| **P3** | **Class-F error enums (the spine)** | **landed this run** | §3g RAID / §1e |
| **P5** | **PROP-018 grammar** (affinity dispatcher, dedup, cross-check, IntentStatus) | **landed this run** | §3g RAID / §3a/§3b |
| **P4** | **Class-G pub-doctest drain + gate flip (vibe-mcp)** | **landed this run** | §1b/§1c |
| **P6** | **REPORT + health refresh + checkpoint + mirror** | **this document** | §8 |

A full Tier-0 floor (self-check, conform, specmap, test-gate, fast-loop) ran
green at every phase boundary (P3, P5, P4) before the next phase opened.

## P3 — Class-F error enums

The whole new-feature surface was `anyhow`-only, so `err-req` / `err-msg`
passed vacuously. This phase gave every fallible domain layer one `thiserror`
enum, each `#[spec(implements = …)]` with every `#[error]` carrying the
Class-F `(violates spec://…; fix: …)` tail; `anyhow` stays only at the binary
edge (the `run_*` dispatch in `man/mod.rs`, the `InferenceBackend` trait).

| Layer | Enum | Anchor | Commit |
|---|---|---|---|
| `man/model.rs` | `ModelError` | #selectors / #build | `9b90f55` |
| `man/store.rs` | `StoreError` | #layout | `a03dde6` |
| `man/placer.rs` | `PlaceError` | #instances | `4fd8b3d` |
| `man/source.rs` | `ResolveError` (`#[from] GitError/StoreError`) | #provenance / #selectors | `7d1ee8e` |
| `man/git.rs` | `GitError` (+ `pub`→`pub(crate)`) | #build | `8fb5bf2` |
| `man/error.rs` (new cell) | `ManError` | #surface | `a8f07d8` |
| `vibe-mcp/agentic.rs` | `RelayError` | PROP-018 #relay | `931f805` |
| `vibe-mcp/pkgskill.rs` | `PackageSkillError` | PROP-018 #vibe-skill | `ce3d0e3` |

`ManError` landed in its own `man/error.rs` cell rather than `mod.rs` so the
dispatch file stays clear of the `[540, 600]` file-length danger band P1
pulled it out of (it sits at 536). The IO layers fold `toml` errors to a
string `detail` rather than a typed `#[from]`, decoupling from the `toml`
error-type version; `io::Error` is carried via `#[source]`.

`install.rs`, `remove.rs`, `tests.rs` needed no edits — they are pure callers
at the anyhow edge, where `?` auto-converts each typed error.

## P5 — PROP-018 grammar

- **Affinity dispatcher (req r2 §2.3, `4e249cc`).** `ActiveBackend` names which
  backend is live (§2.1); `check_affinity` refuses an op reached through a
  backend it has no affinity for, naming the one it needs — typed
  `AffinityError` (NeedsAgent / NeedsStandalone). Both `explain` transports
  now run the check.
- **Unified transports (§2.8, same commit).** Added `BackendOutcome::Inline`
  and an `InlineBackend` (the MCP transport), so the one `Intent`-producing op
  drives both the CLI file-relay and the MCP tool behind the `InferenceBackend`
  seam — the MCP path no longer hand-rolls its JSON outside the abstraction.
- **`resolve_project_root` dedup (`cfdf213`).** One copy in `commands`, was
  byte-identical in `agentic` and `skill`.
- **skill_template ↔ default_tools cross-check (`dee16dc`).** A test asserting
  every served MCP tool is named in the usage skill — turns the §2.9 prose
  contract into runnable capital.
- **`IntentStatus` newtype (`7cf57c5`).** Closes the pending→done mailbox
  vocabulary; the marker line is rendered once, not re-typed in writer + drainer.
- **Shared dry-run status projection (`4861cba`).** The duplicated
  `match (status, dry_run)` block lifted into one `preview_status`.

## P4 — Class-G pub-doctest drain

**vibe-mcp drained and gated.** Its 27 public types each gained a compiled
doctest (construct-and-Display for the error enums, serde round-trips for the
JSON-RPC wire shapes, descriptor-name asserts for the tool cells, submit
round-trips for the backends); `vibe-mcp` then entered `GATED_PUB_DOCTEST` at
zero gap (`conform check`: 0 new). Commits `c5fff3a`, `273e58f`, `dbe593d`.
All 39 vibe-mcp doctests compile and run under `cargo test -p vibe-mcp --doc`.

## Deferred / declined (with reasons)

The sweep is honest about what it did **not** do — a silent skip reads as a
bug, a recorded one as a decision (CONFORM_EXEMPT principle):

- **vibe-cli pub-doctest gate — DEFERRED (DBT-0021).** `vibe-cli` is a bin
  crate with **no lib target**, so `cargo test --doc` cannot compile its
  doctests ("no library targets found"). Flipping its gate would enforce 86
  **uncompiled** examples — prose-shaped-as-code that CI can never verify,
  the exact opposite of the rule's "teach by a *compiled* example" contract
  and a Law-2 violation. The real fix is a lib target or visibility tightening
  (a bin's `pub` is not an API) — an owner-level structural call, recorded in
  the `GATED_PUB_DOCTEST` comment and `debt.json`. The owner's stated maximal
  scope named vibe-cli; this empirical finding (bin crate, doctests don't run)
  is the conservative interpretation, flagged here per the uncertainty protocol.
- **`SkillStatus` newtype — DEFERRED.** The 9-value install/uninstall status
  vocabulary is shared and **behaviorally matched** (`matches!`, `==`) across
  four serialized report types in two crates (PROP-015 `AgentInstallReport` /
  `SkillInstallReport`, PROP-018 `PackageSkillReport`) and the vibe-cli mcp
  walkers. Typing it carries a wire-contract surface (4 serialized fields +
  golden transcripts) and a scope/naming/cross-domain design call that belongs
  to the owner, not a grammar sweep. The contained win it would have enabled —
  the duplicated dry-run transform — was taken as `preview_status` (`4861cba`).
- **`SkillOrigin` newtype — DECLINED.** A display-only label (`"project"` /
  member-path / `"<kind>:<name>"`) with no enforced invariant and no
  behavioral branch — ceremony, the same posture as the P2 `CommitHash`
  decline.

## Gate state at close

- `conform check` — **0 / 0 / 0** (0 findings, 0 frozen, 0 new); 16 crates
  gated, 4 exempt.
- `GATED_PUB_DOCTEST` — widened to 6 (vibe-core + the four P1 zero-gap crates +
  **vibe-mcp**).
- `specmap --check` — clean: 545 units / 561 edges / 0 suspects / 0 warnings /
  0 gated orphans.
- `test-gate` — green, xfail-strict (1204 results, 0 failed, 3 skipped).
- `fast-loop` — 20/20 cells within the 60s budget.
- full `self-check.sh` — green (fmt, all workspace tests + doctests, clippy
  `-D warnings`, `vibe check` 0/0/0).

## Honest mistakes / lessons

- **rustfmt re-wraps multi-line `use` and `let-else`.** Two `style(…)` fixup
  commits (`7d59f4e`, `e71827f`) landed because hand-written argument-collapsing
  edits weren't fmt-clean; running `cargo fmt --all` *before* each commit (not
  after) would have avoided them. (The CONTINUE recipe already warned of this.)
- **The plan under-counted the `SkillStatus` blast radius.** It read as
  "pkgskill.rs + install.rs"; the status vocabulary is in fact matched in five
  files across two crates and two PROP domains — which is *why* it was
  deferred, not forced through.
- **bin-crate doctests are a real constraint the plan missed.** The P4 worklist
  named "vibe-cli (83-type gap)" without noting vibe-cli has no lib target;
  the gap is structural, not a drain backlog.

## Note — concurrent owner commit, task #13

Commit `8065afb` ("docs(guides): point the flow:wal probe at the fqdn repo")
landed mid-run, authored by the owner in a parallel terminal. It corrects the
opencode quickstart's `git ls-remote` probe from the retired `flow-wal` repo
to the canonical `org.vibevm_wal` — the same file the prior checkpoint flagged
as task #13 (a stray `flow-wal → fqdn` rewrite). The file now carries the
correct fqdn content as a deliberate commit, and the rewrite did **not** recur
across this run's many `cargo test` / `self-check` passes. Recorded dormant in
`debt.json` (DBT-0022) for traceability; the underlying "what rewrote it" was
never root-caused but is non-reproducing.
