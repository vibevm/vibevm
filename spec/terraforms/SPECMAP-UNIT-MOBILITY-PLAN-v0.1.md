# SPECMAP Unit-Mobility Plan v0.1 — moving spec units across package boundaries with their edges intact

**status: PLANNED · not started · unblocks the cultural-pattern extraction refactoring · executes cold from a fresh session**

> **Read-first / boot.** Written to be executed cold. Boot the normal way
> (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md` →
> `CONTINUE.md`), then read this whole file. It is self-contained: the
> current-state facts (with `file:line`), the decisions and their rationale,
> the phase-by-phase recipes, the risks, and the acceptance gates are all
> here. The git log is the authoritative per-item record; the WAL is the
> canonical living state and supersedes this plan if they diverge.
>
> **Sibling documents.** The mechanism this plan extends is
> `spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014` (bidirectional
> spec↔code traceability). The prior move of the engine itself is
> `spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md` (done, though its own
> header still says "not started"). This plan is the enabler for the
> cultural-pattern extraction refactoring — the one that pulls general
> programming-culture material out of the vibevm host specs into reusable
> packages (redbook and friends). That refactoring is the CONSUMER; this plan
> is its safety rail.
>
> **Parent model.** This plan is the **first consumer of
> `spec://vibevm/common/PROP-031#root`** (algorithmic refactoring — the codemod
> engine). The `move` command (Phase 3) is built as the **first operation of
> that PROP's engine**, not a bespoke tool, and inherits its typed-command /
> atomic / dry-run / gated contract (PROP-031 §2.3, §2.6). Read PROP-031 first;
> this plan instantiates it on the discipline tier.

---

## 0. Why this exists (the reframe)

The cultural-pattern extraction refactoring performs, thousands of times over,
one primitive operation: **it moves a spec unit across a package boundary.** A
paragraph about Conventional Commits leaves `spec://vibevm/common/PROP-000#…`
and lands at `spec://org.vibevm.world/<pkg>#…`. The load-bearing question is
not "did the text move" — a human can see that — but **"do the code edges that
cite the moved unit keep resolving after it moves?"** If they do not, the move
silently severs the spec↔code linkage that PROP-014 exists to protect, and the
gate stays green while the map rots.

The user's framing names the target precisely: *the connection must travel with
the meaning* — a moved unit is a capsule of `{text + its spec-unit identity +
the edges that point at it}`, transferred as one transaction, not a text move
followed by a separate edge-repair pass.

**The surprise from the ground truth (this is the good news, and it reshapes the
plan): most of the machinery already exists and is tested.** The specmap engine
already resolves cross-package edges, already tracks revisions + content hashes,
already computes suspects, already detects dangling edges, already classifies
drift. What is genuinely missing is far narrower than the PROP-014 design doc
(a v0.1 proposal with open questions) suggests. This plan builds only the real
gaps — and the single highest-leverage one is not a feature at all, it is
**wiring the already-clean host index into the gate.**

## 1. Decisions

### D1 — Eager-retarget first; redirect deferred (the capsule needs no new engine feature for MVP)

There are two ways to make "the connection travels with the meaning" true:

- **(A) Eager retarget.** The move rewrites the inbound *code* edges to the new
  URI in the same change. The moved unit resolves at its new package address via
  `[[external_specs]]` (which already works, §2); the old address ceases to
  exist along with its now-rewritten edges. The edges literally travel with the
  meaning, atomically.
- **(B) Redirect stub.** The old anchor becomes a tombstone-alias to the new URI
  (`<!-- RELOCATED: → spec://… -->`); code edges are left untouched; the resolver
  follows the redirect. Buys deferred / parallel retargeting.

**Decision: (A) is the MVP; (B) is a Tier-2 enhancement.** Two facts make (A)
sufficient and cheap: (1) `[[external_specs]]` already resolves the moved unit's
new address with zero engine work (§2, §3); (2) **the specmap graph is
code→spec only** — inbound edges to a *cultural* unit are the handful of code
tags that cite it, not a sea of references (prose `spec://` cross-links are not
graph edges — see D3). So eager retarget touches few sites per unit and needs no
redirect-follow machinery. Redirect (B) earns its engine cost only if the
refactoring parallelises moves across a swarm and cannot retarget edges
atomically; it is specified here (Phase 4) but not on the critical path.

### D2 — Gate the existing index (the foundation, and it is host-only)

The host **already** commits a clean `specmap.json` (0 dangling, 0 suspects,
§2), but **nothing enforces it**: `cargo xtask specmap --check` is not in
`tools/self-check.sh` nor in CI (§2). The index can therefore drift silently,
and — critically — a refactoring that severs an edge would leave the gate green.
Wiring `specmap --check` into the floor is the load-bearing prerequisite for
every capsule move, and it is a **few lines in `self-check.sh` with no engine
rebuild.** This is Tier 0.

### D3 — Prose spec→spec links must become graph edges, not just gated (PROP-031 §3.3)

The specmap graph extracts edges from **code tags only** (`#[spec]` / `scope!`
via `rscan`); `mdspec` parses spec *units* (anchored headings), not the inline
`spec://` references that appear in prose — boot snippets, `CLAUDE.md`, other
PROP bodies. Moving a unit therefore breaks those prose links *silently as far
as specmap is concerned*. Since boot snippets and the four-rules contract cite
exactly the cultural units the refactoring will move, this gap is real.

**Decision (revised under PROP-031 §3.3):** extend `mdspec` to capture prose
`spec://` references as a first-class specmap edge kind (e.g. `references`) —
**not** merely a standalone checker. The reason is strategic: a checker only
*gates* prose links (goes red when one dangles); a graph edge gates **and makes
them refactorable** — `rename-address` (PROP-031 §2.6) then retargets a prose
citation exactly as it retargets a code tag, which is precisely what the
cultural-pattern refactoring needs when it moves a unit that boot snippets cite.
A standalone `vibe check` cell remains the acceptable *tactical stopgap* if the
engine edit must wait, but the strategic target is the edge. Tier 1.

## 2. Current-state facts (verified 2026-07-13; do not re-discover)

All paths repo-relative. Engine source is **authored** in
`packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/`
and **vendored** (byte-identical, held so by `sync-engines`) into
`…/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap/`. Cite the
authored copy; never edit the vendored one.

**Host traceability state — EXISTS and is CLEAN.**

- `specmap.toml` (repo root, tracked): `namespace = "vibevm"` (`:14`),
  `scan_roots = ["crates/*", "xtask"]` (`:18`), `spec_roots = ["spec"]` (`:21`),
  `root_spec_docs = ["VIBEVM-SPEC.md"]` (`:26`), `exempt = [vibe-graph, vibe-llm,
  vibe-wire, xtask]` (`:38-43`), and **one** external spec:
  `[[external_specs]] namespace = "core-ai-native"`,
  `root = "vibedeps/flow-core-ai-native/0.7.0/spec"` (`:54-56`).
- `specmap.json` (repo root, tracked): the committed index. `SCHEMA = 2`
  (`index.rs:29`). Verified: `"suspects": []` and **zero** `dangling-edge`
  warnings in the committed file. So today every host `spec://vibevm/…` and
  `spec://core-ai-native/…` edge resolves.
- **The gate does NOT enforce it.** `tools/self-check.sh` runs (in order) fmt,
  test, clippy, `vibe check` (`:103-104`), `conform check` (`:112`),
  `sync-engines --check` (`:119`), the core/rust/mcp package gates (`:126-182`),
  and the packages' `rust-ai-native-specmap --gate` self-traces (`:152-156`,
  `:171-182`). There is **no** host-wide `cargo xtask specmap --check` step
  (grep of `tools/` + `.github/`: none). The host index is buildable and clean
  but ungated.

**The engine — what ALREADY works (do not rebuild).**

- **Cross-package resolution (PROP-014 §7.1) — DONE.** `Config.external_specs:
  Vec<ExternalSpec>` (`config.rs:63`; `ExternalSpec { namespace, root }`
  `config.rs:98-105`). `mdspec::scan_external_units` mints an installed
  package's units under that package's namespace (`mdspec.rs:355-389`).
  `index::build_with_scanner` feeds them into the revision map so "an edge into
  `spec://<pkg>/…` resolves instead of dangling, and its pin can go suspect —
  but never enters this project's index" (`index.rs:64-76`). Proven by
  `external_specs_resolve_edges_without_entering_the_index` (`index.rs:537-586`).
- **Revisions + content-hash + suspect (PROP-014 §2.2) — DONE.** `mdspec`
  parses the kind line `` `req rN [planned | disputed(#anchor)]` ``
  (`mdspec.rs:64-125`); each unit carries `contentHash = sha256(LF-normalised
  span)` (`mdspec.rs:278`, `lib.rs:54-66`). `index` computes suspects when a
  pin `p < unit.r` (`index.rs:119-145`) and warns on `pin-ahead-of-unit` /
  `pin-into-unmarked-unit`. Proven by `suspects_dangling_and_pin_ahead_are_
  detected` (`index.rs:491-532`).
- **Full index + dangling + drift + `--check` — DONE.** `index::build` composes
  spec units + code items + edges deterministically (`index.rs:55-175`);
  `dangling-edge` warning when no unit carries an edge's anchor
  (`index.rs:95-105`); `classify_drift` emits revision-bump→suspect and
  unbumped-hash signals (`index.rs:198-287`); `check` byte-compares against the
  committed file (`index.rs:324-358`).
- **Orphan-coverage ratchet (`--gate`) — DONE.** `run_gate` +
  `run_ratchet_gate` (`rust-ai-native-specmap/src/lib.rs:54-101`).

**The engine — what is MISSING.**

- **No redirect / tombstone for a unit that moves *between documents*
  (PROP-014 §7.3, "Lean: redirect stubs" — an open question).** `mdspec` has no
  `RELOCATED`/redirect parsing; `index` has no redirect-follow. Anchors are
  minted immutable per file (`mdspec.rs:273`); a unit that leaves its file takes
  its address with it and the old address simply stops resolving.
- **No `move` operation.** The CLI is exactly `--check` | `--gate` | (default)
  write (`rust-ai-native-specmap/src/main.rs:19-42`); the xtask shim is
  `run_specmap(repo_root, check)` (`xtask/src/specmap.rs:11-13`). There is no
  command that relocates a unit, retargets its edges, or writes a redirect.
- **Prose `spec://` links are not graph edges** (D3). `mdspec` inventories units
  from headings only; inline references in prose are invisible to the graph.

**Gotchas to carry.**

- **Namespace granularity.** The host's one `external_specs` entry uses the
  short namespace `"core-ai-native"` (`specmap.toml:55`) while the *package's
  own* `scope!` tags use the fully-qualified
  `spec://org.vibevm.ai-native/core-ai-native/…`. The host tags that resolve
  through this entry must cite the **short** form. When the refactoring adds a
  new external spec for `org.vibevm.world/<pkg>`, the `namespace` field must
  match exactly the `<package>` segment the host tags cite — verify per package,
  do not assume.
- **Engine changes are expensive; host changes are cheap.** M1 (gate) and M4
  (external_specs upkeep) touch only `self-check.sh` + `specmap.toml` — **no
  engine rebuild.** M2/M3 (redirect, move) edit the authored engine → they bump
  `core-ai-native` and `rust-ai-native-lang` versions, require
  `cargo xtask sync-engines` (not `--check`) to refresh the vendored copies, and
  a `vibe install` re-materialise into `vibedeps/`. Sequence host-only work
  first.
- **Self-hosting bootstrap care.** The engine gates the very repo it lives in.
  `core-ai-native-specmark` / `-grammar` are exempt from their own gate (the tag
  bootstrap pair). Any engine change is validated against the package's own
  gates (`self-check.sh` steps 7-9) before the host consumes it.
- **PROP-030 embedded registry interaction.** In-repo packages now live under
  `packages/…` as well as materialised `vibedeps/…`. An `external_specs.root`
  can point at either; for a package the refactoring creates in-tree, pointing
  at `packages/org.vibevm.world/<pkg>/<ver>/spec` avoids a materialise round-trip
  during development (confirm the engine walks it — it is a plain dir walk,
  `mdspec.rs:367`, so it will).
- **Machine quirks (this box).** Edits via editor tools only (PS 5.1 corrupts
  UTF-8-no-BOM); commits via `git commit -F - <<'MSG'`; `self-check.sh` through
  Git Bash, check the REAL exit code (`; echo "EXIT=$?"`), never a `| tail`'d
  pipe.

## 3. What "done" must not re-build (inventory carried from §2)

Cross-package resolution, revisions, content hashes, suspects, dangling
detection, drift classification, the deterministic committed index, and the
orphan ratchet are **built and tested**. This plan touches them only to (a) call
them from the host gate (M1), (b) add a redirect edge kind (M2, optional), and
(c) drive them from a new `move` command (M3). No re-implementation.

## 4. The missing features (tiered)

| Tier | id | Feature | Surface | Cost |
|---|---|---|---|---|
| **0** | **M1** | Gate the host index — `specmap --check` in `self-check.sh` (dangling=0, suspect=0) | host (`self-check.sh`) | **low** (no engine rebuild) |
| **0** | **M4** | `external_specs` upkeep protocol for packages the refactoring creates | host (`specmap.toml`) | low |
| **1** | **M5** | Prose `spec://` links as first-class specmap edges (D3 → graph-edge, PROP-031 §3.3) | engine (`mdspec`) | medium |
| **1** | **M3** | The refactoring engine's first operations — `rename-address`, then `move-unit` = composition (PROP-031 §2.6) | engine + CLI | high (version bump + vendor + materialise) |
| **2** | **M2** | Redirect / tombstone for parallel/deferred moves (PROP-014 §7.3) | engine | high |

**The critical insight: Tier 0 alone makes the refactoring safe.** With M1
gating the clean index and M4 keeping resolution current, a capsule move done
*by hand* (cut the unit, paste into the package spec, rewrite the few inbound
code edges, add the `external_specs` entry) is **fully gated** — `specmap
--check` goes red the instant an edge dangles or a pin goes suspect. M3 makes
that fast and idempotent; M2 makes it parallel; neither is required to start.

## 5. Phases

The floor (`self-check.sh` exit 0) must be green at every phase boundary; each
phase lands its own topic-grouped commit(s).

### Phase 0 — Verify + ratify (GATING; spec + facts, no code)

**Why first.** PROP-014 is a **design proposal v0.1** whose §7.1/§7.3/§2.2 are
open questions, and §335 removes any mechanism unexercised by Phase 2. But §2
above shows §7.1 (cross-package) and §2.2 (revisions/suspect) are in fact
**implemented and tested** — the doc lags the code. So Phase 0 is a
reconciliation, not a design-from-scratch.

**Recipe.**
1. Re-verify the §2 facts against the live tree (the `file:line` above); record
   any drift. In particular: `bash tools/self-check.sh; echo "EXIT=$?"` (must be
   0) and `cargo xtask specmap --check` (must be clean — the committed index
   matches the tree). If `--check` is red, regenerate + commit before anything
   else, so the baseline is provably clean.
2. **Ratify the design deltas in PROP-014** (authored in
   `core-ai-native/…/mechanisms/PROP-014-…md`): promote §7.1 (cross-package,
   DONE) and §2.2 (revisions, DONE) from "open question" to "decision — shipped";
   record the §7.3 decision (redirect deferred, eager-retarget is the MVP path,
   D1); record D3 (prose-link scope). This is an engine-package spec edit → it
   bumps `core-ai-native` and re-vendors, so batch it with Phase 3 if no code
   rides on it sooner, OR land it as a docs-only bump now. **Owner decision
   point.**
3. **Decide D3** (prose-link gating: manual vs checker). Verify whether
   `crates/vibe-check/src/checks/` already validates any `spec://` prose links
   (it does not appear to — `redirect_block.rs` is registry redirects, not spec
   links); if absent, M5 is new work.
4. **Acceptance:** baseline provably clean; PROP-014 deltas ratified or queued;
   D3 decided.

### Phase 1 — Gate the host index (M1) + resolution-upkeep protocol (M4) — TIER 0, host-only

**Goal.** The clean host `specmap.json` becomes a floor invariant, and there is
a written protocol for keeping `external_specs` current as packages are created.

**Recipe.**
1. **Wire `specmap --check` into `tools/self-check.sh`** as a new step (after
   `conform check`, before the package gates — it reuses the build cache). The
   exact command mirrors the existing xtask idiom:
   `cargo xtask specmap --check` (which calls `run_specmap(repo_root, check=true)`
   → `index::check`, `rust-ai-native-specmap/src/lib.rs:22-44`). It fails on any
   drift, which now includes **any new dangling edge or suspect** introduced by
   a future move. Add a matching comment block (the file documents each step).
2. **Confirm the vacuity trap is not masking emptiness** (`index.rs:409-419`):
   the host scans real crates, so `code_items > 0`; assert the step reports a
   non-vacuous count.
3. **Write the `external_specs` upkeep protocol** into `specmap.toml`'s comments
   (and this plan's §8): *when a package is created that hosts units the host
   code cites, add an `[[external_specs]]` entry whose `namespace` equals the
   `<package>` segment the tags cite and whose `root` points at the package's
   spec tree (`packages/…/spec` in-tree, or the materialised `vibedeps/…/spec`
   slot).* Note the short-vs-fully-qualified namespace gotcha (§2).
4. **Acceptance:** `self-check.sh` now runs `specmap --check` and is exit 0;
   deliberately break one edge (retarget a `#[spec]` to a missing anchor) and
   confirm the gate goes **red** with a `dangling-edge` drift line; revert.
   `cargo xtask specmap --check` clean.

**Commit:** `build(specmap): gate the host traceability index in self-check`.

> **This phase is the whole safety rail.** After it, the refactoring can move
> units by hand and the gate catches every severed edge. Everything below is
> acceleration.

### Phase 2 — Prose `spec://` links as graph edges (M5) — TIER 1

**Goal.** Moving a unit can no longer silently break a `spec://` reference in a
boot snippet or a PROP body — and those references become *refactorable*, not
just gated (D3, PROP-031 §3.3).

**Recipe.**
1. Choose the surface: (a) a new `vibe-check` cell
   (`crates/vibe-check/src/checks/spec_links.rs`) that scans `spec/**`, the root
   spec docs, and the boot snippets for inline `spec://…#anchor` references and
   resolves each against the union of host units + `external_specs` units; or
   (b) a new specmap edge kind `references` extracted by `mdspec` so prose links
   enter the graph and dangle-check for free. **(b) is the strategic target**
   (PROP-031 §3.3): only a graph edge is refactorable by `rename-address`, so a
   later address rename retargets prose citations for free. **(a) is the
   tactical stopgap** — lighter, host-local, no engine bump — acceptable only if
   the engine edit must wait, and superseded by (b) later.
2. Wire it into `self-check.sh` (if (a)) — it rides `vibe check`, already a step.
3. **Acceptance:** a deliberately-dangling prose `spec://` link is caught; real
   links pass; `self-check.sh` exit 0.

**Commit:** `feat(check): gate prose spec:// links against the unit inventory`.

### Phase 3 — The refactoring engine's first operations (M3) — TIER 1, engine + CLI

**Goal.** Perform the capsule transfer as a typed, gated operation, so a
~70-unit refactoring is mechanical and idempotent instead of hand-surgery — and
build it as the **first operations of the PROP-031 engine**
(`spec://vibevm/common/PROP-031#algebra`), not a bespoke command, so the second
operation is composition rather than a rewrite.

**Build `rename-address` first (the purest instance).** Before `move`, ship
`rename-address <from-uri> <to-uri>`: retarget every citing edge (code tags now;
prose edges once M5 lands) to a new `spec://` address, atomically and gated. It
has no text relocation and no cross-package step, so it validates the entire
typed-command / dry-run / atomic / `specmap --check`-gated loop (PROP-031 §2.3)
on minimal surface. **Then `move-unit` = `rename-address` ∘ relocate-text ∘
external-specs-upkeep** — a composition, per PROP-031 §2.6. This also directly
serves the owner's cost thesis: a bare address rename (the commonest refactor)
becomes a tool call, not an LLM file-walk.

**Design (`move-unit`).** `rust-ai-native-specmap move <from-uri> --to <package-spec-doc>[#anchor]
[--retarget|--redirect] [--dry-run]`:
1. **Locate** the source unit's span (anchored heading → next same-or-higher
   heading, the exact `mdspec` span rule, `mdspec.rs:230-241`).
2. **Relocate** the span text into the target package's spec doc, minting the
   anchor there (slug immutable — same `{#anchor}`, only the `<package>/<docpath>`
   segment changes).
3. **Retarget (default, D1 strategy A):** rewrite every inbound code edge
   (`#[spec]` / `#[verifies]` / `scope!` whose URI is `<from-uri>`) to the new
   URI, found by scanning `scan_roots`; bump the source unit's revision is moot
   (it is leaving), but bump the *pins* to the moved unit's current `r`.
   **Or redirect (`--redirect`, D1 strategy B, requires M2/Phase 4):** leave a
   tombstone at the old anchor and do not touch edges.
4. **Upkeep:** add/update the `[[external_specs]]` entry for the target package
   in the host `specmap.toml` (M4), so the retargeted edges resolve.
5. **Report** to stdout: unit moved, N edges retargeted, external_specs entry
   added, and the residual suspect list (should be empty after retarget).
6. `--dry-run` prints the plan and touches nothing.

**Engine changes** (authored `core-ai-native-specmap`, then `sync-engines` +
version bump + materialise): a new `mv`/`capsule` module driving `mdspec` span
extraction + a code-edge rewriter (syn-aware, reusing `rscan`'s tag grammar);
new CLI subcommand in `rust-ai-native-specmap/src/main.rs`; xtask passthrough.

**Acceptance.** On a fixture tree: `move --dry-run` shows the plan; the real move
relocates the unit, rewrites exactly the inbound edges, adds the external_specs
entry, and leaves `specmap --check` **clean** (0 dangling, 0 suspect); a second
run is idempotent (unit already moved → no-op with a clear message). The package
gates (`self-check.sh` 7-9) stay green.

**Commits (topic-grouped):** `feat(specmap): the unit-move capsule command` +
`chore(specmap): vendor-sync + bump for the move command` +
`build(deps): re-materialise vibedeps for the specmap bump`.

### Phase 4 — Redirect / tombstone (M2) — TIER 2, optional, engine

**Goal.** Enable deferred / parallel moves: a swarm relocates units without
atomically retargeting edges, and the old addresses keep resolving via redirect
until a later sweep collapses them.

**Recipe.**
1. **Format** (PROP-014 §7.3, "PROP-012 flavour"): a tombstone marker under the
   old anchor, e.g. `<!-- RELOCATED: #<anchor> → spec://<pkg>/<docpath>#<anchor>
   -->`, keeping the anchored heading line so the old `spec://…#anchor` still
   parses to a unit.
2. **Engine:** `mdspec` parses the marker into a `redirects_to` field on the
   unit; `index` follows redirects when resolving an edge's target (max-hops +
   cycle detection), and adds two gate classes: **broken redirect** (target
   gone) and **redirect cycle**.
3. **Collapse pass:** a `specmap collapse-redirects` mode (or a flag on `move`)
   that rewrites edges from a redirected old address to the final target and
   removes the tombstone — the Tier-1 eager form applied late. This is the
   refactoring's "scaffold-cleanup" phase.
4. **Acceptance:** an edge into a redirected old anchor resolves through the
   redirect (no dangling); a broken/cyclic redirect is caught; `collapse` leaves
   the tree with direct edges and no tombstones; `specmap --check` clean
   throughout.

**Commit:** `feat(specmap): redirect stubs for deferred unit moves (PROP-014 §7.3)`.

### Phase 5 — Checkpoint

`self-check.sh` exit 0 (real code), `cargo xtask specmap --check` clean, `vibe
check` clean, package self-traces clean. Update `spec/WAL.md` (current-state
lead + a session section) and rewrite `CONTINUE.md`. Commit topic-grouped
(`docs(wal)` + `docs(continue)`). **Mirror is outward-facing — present it and
HOLD for the owner's explicit word** (`cargo xtask mirror`); do not mirror
autonomously.

## 6. End-state (what "done" looks like)

- The host `specmap.json` is a **floor invariant**: `self-check.sh` runs
  `specmap --check`, so no move can sever an edge and stay green (M1).
- `external_specs` has a documented upkeep protocol and an entry per package the
  refactoring creates (M4); every retargeted edge resolves cross-package.
- Prose `spec://` links are gated (M5) or a recorded, accepted manual concern.
- **Tier 1:** `rust-ai-native-specmap move` performs the capsule transfer
  (relocate + retarget + upkeep + report), idempotently (M3).
- **Tier 2 (optional):** redirect stubs enable parallel moves with a late
  collapse pass (M2).
- **Deferred / not built:** the runtime MCP exposure of the metamodel (PROP-014
  §2.8), `vibe explain --prose`, and signing (§2.8.4) — unrelated to unit
  mobility; out of scope here.

## 7. Risks & fallbacks

- **The committed host index is stale at start (Phase 0).** If `specmap --check`
  is red before any work, a prior manual regen was skipped. Fallback: regen +
  commit, confirm the diff is only genuine tree drift, then proceed. Do NOT wire
  the gate over a dirty baseline.
- **Namespace mismatch on a new external_specs entry (M4).** If the entry's
  `namespace` ≠ the `<package>` segment the tags cite, edges dangle after a move.
  Fallback: read one retargeted edge's URI and set `namespace` to its
  `<package>` segment verbatim (§2 gotcha).
- **Engine bump ripples (M2/M3).** A specmap engine change must go through
  `sync-engines` + version bump + `vibe install` re-materialise, and the package
  self-trace gates must stay green. Fallback: land M1/M4/M5(a) first (host-only,
  no bump) so the refactoring can start before any engine work.
- **Prose-link checker false positives (M5).** Inline `spec://` inside fenced
  code / examples must be excluded (the same `fence_mask` posture `mdspec`
  already uses, `mdspec.rs:131-146`). Reuse it.
- **Move command corrupts a span (M3).** The span rule is subtle (fenced sample
  headings, `mdspec.rs:554-569`). Fallback: `--dry-run` is mandatory in the
  refactoring protocol; the human reviews the plan before the write, and
  `specmap --check` is the backstop.
- **Reversal cost.** M1 is one commit, trivially revertible. M3/M2 are
  engine+package changes; each phase is its own commit so a bad phase rolls back
  without losing the prior ones. The refactoring itself works in `neworder2/` +
  topic commits (its own plan), independently revertible from this one.

## 8. Plugging into the refactoring — the capsule protocol on a live example

Take `#conventional-commits` (`spec/common/PROP-000.md:138`, `### 12.2
Conventional Commits {#conventional-commits}`). Suppose it moves to a new
package `org.vibevm.world/conventional-commits`.

**By hand, under Tier 0 (M1+M4) — safe today after Phase 1:**
1. Cut the unit span from `PROP-000.md` into the package's spec doc, keeping the
   `{#conventional-commits}` anchor; its URI becomes
   `spec://org.vibevm.world/conventional-commits#conventional-commits` (or a
   tidier anchor — but immutable once published).
2. Find inbound **code** edges: `grep -rn 'PROP-000#conventional-commits'
   crates/ xtask/`. Rewrite each `#[spec]`/`scope!` URI to the new address.
   (Cultural units like this often have **zero** code edges — commit discipline
   is enforced by process, not Rust — in which case there is nothing to retarget
   and the graph is undisturbed.)
3. Add to host `specmap.toml`: `[[external_specs]] namespace =
   "org.vibevm.world/git-conventional-commits"` (or the short segment the tags cite),
   `root = "packages/org.vibevm.world/git-conventional-commits/v0.1.0/spec"`.
4. Handle inbound **prose** links (boot snippets, `CLAUDE.md`, the four-rules
   contract cite `spec://vibevm/common/PROP-000#…`): rewrite them to the new
   address — gated iff M5 landed, manual otherwise.
5. `cargo xtask specmap --check` → must stay clean. Green = the capsule arrived
   intact; red = an edge dangled, fix before commit. Commit as one topic unit.

**Under Tier 1 (M3):** steps 1-3 collapse to
`rust-ai-native-specmap move spec://vibevm/common/PROP-000#conventional-commits
--to packages/org.vibevm.world/git-conventional-commits/v0.1.0/spec/GUIDE.md
--retarget --dry-run`, review, drop `--dry-run`. Step 4 still needs M5.

**Under Tier 2 (M2):** add `--redirect` to parallelise across a swarm; a late
`collapse-redirects` sweep makes edges direct and removes tombstones.

**This is exactly the "connection travels with the meaning" the refactoring
needs** — and Tier 0 already delivers it, gated, with no engine work.

## 9. Quick-start (for the executing session)

```sh
# boot, then verify the starting floor + baseline:
bash tools/self-check.sh; echo "EXIT=$?"          # must be 0
cargo xtask specmap --check                         # must be clean — record the summary line
grep -n 'specmap' tools/self-check.sh || echo "specmap NOT yet gated (expected pre-M1)"

# Phase 1 (M1): add a `cargo xtask specmap --check` step to self-check.sh, then
bash tools/self-check.sh; echo "EXIT=$?"            # exercises the new gate
# prove it bites:
#   retarget one #[spec] to a missing anchor → self-check RED (dangling-edge) → revert
```

## 10. Acceptance for the whole campaign

- **Tier 0 (unblocks the refactoring):** `specmap --check` is a `self-check.sh`
  step and bites on a severed edge (M1); `external_specs` upkeep protocol
  written (M4); D3 decided (M5).
- **Tier 1:** prose links gated (M5, if chosen); `rust-ai-native-specmap move`
  performs an idempotent, gate-clean capsule transfer (M3).
- **Tier 2 (optional):** redirect stubs + collapse pass work end to end (M2).
- `self-check.sh` exit 0, `cargo xtask specmap --check` clean, `vibe check`
  clean, package self-traces clean, the 11 host dogfooders still traced.
- WAL + CONTINUE updated; mirror held for the owner's explicit word.
- **The refactoring can move any cultural unit and the gate proves the edges
  travelled with it.**
