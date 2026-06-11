# Adoption Log — Discipline v0.2 (TERRAFORM-PLAN-v0.3)

Raid-grained session log of the v0.3 adoption. Companion artifacts:
[`PREDICTIONS.md`](PREDICTIONS.md) (the pilot prediction ledger),
`REPORT.md` (written at close-out). The prior terraform's artifacts
live one level up in `terraform/` and are historical — they record
the v0.2 PLAYBOOK run and are not edited by this adoption.

---

## 2026-06-11 — Phase 0: Adopt & shim

**Scope & freeze.** Documents only (`spec/neworder/`, `packages/`,
`spec/discipline/`, boot artifacts, the two lock files); production
code frozen except the inert `spec://` URI strings and the ledger
epoch input — both metadata, no behavior. Discipline files moved
byte-verbatim (the owner's constraint: the product is not bent to
fit the pilot).

**What happened, in order:**

1. **Packaged the product.** The flat `spec/neworder/` drop became
   two packages under the in-repo local registry root `packages/`:
   - `flow:org.vibevm/discipline-core@0.2.0` — manifesto, card
     format, scaffold catalog, raid playbook, `cards/` (INDEX + the
     nine scaffold cards), `appendix/` (contradiction map, atlas),
     `legacy-projections/` (the eleven v0.1-era language guides),
     README (the drop's package README, verbatim).
   - `stack:org.vibevm/rust-ai-native@0.2.0` —
     `rust/GUIDE-AI-NATIVE-RUST.md`, `rust/tools/vibe-tcg.md`;
     `[requires]` on `flow:org.vibevm/discipline-core@^0.2`.
   New files per package: `vibe.toml`, a minimal boot snippet
   (minimal-sufficiency: boot says "cards load by trigger", it does
   not inline the corpus).
2. **Self-hosted install.** `vibe install flow:org.vibevm/discipline-core
   stack:org.vibevm/rust-ai-native --registry ./packages
   --assume-yes` — the Discipline's first carrier installed the
   Discipline through the Discipline's own tool. The stack→flow
   dependency resolved transitively. `[requires]` landed in
   `vibe.toml`, both packages in `vibe.lock` (schema 5, content
   hashes), slots materialised under `vibedeps/`, boot regenerated:
   `spec/boot/INDEX.md` now sequences 00-core → discipline-core →
   rust-ai-native → 90-user.
3. **Relocated the retained mechanisms.** PROP-014, BROWNFIELD,
   ENGINE-CONFORM, LEDGER-INTENT moved (byte-verbatim) from
   `spec/neworder/` to `spec/discipline/` — they stay inside
   `spec/**` because vibevm code carries their `implements` edges
   and mdspec scans only `spec/**`. The ~26 in-source
   `scope!`/`#[spec]` URIs were rewritten
   `spec://vibevm/neworder/…` → `spec://vibevm/discipline/…` in the
   same change set; `specmap.json` regenerated: **352 units / 170
   items / 177 edges / 0 suspects** — edge count and suspect-zero
   preserved through the relocate (prediction P0-2 holds so far).
4. **Shimmed `spec/neworder/`.** The directory now holds one
   README: the where-everything-went table, the reinstall recipe,
   and the carried-over v0.1 beta-gap notes. The duplicate
   `TERRAFORM-PLAN-v0.3.md` copy was removed — the plan's own text
   places vibevm-specific plans outside the product
   (`spec/terraforms/` is the canonical home).
5. **Pinned the pilot.** `vibevm.discipline.lock` records both
   pkgrefs + content hashes. The ledger epoch input changed from the
   old drop README to this pin file — the epoch's "discipline
   package in effect" component now tracks exactly what the pilot
   runs (cache invalidation only; the producer is deterministic).

**Honest findings (feed the REPORT):**

- `vibe.lock` `source_url` for a local-registry install is a
  machine-absolute `file:///C:/…` path. Committed, it is
  machine-specific noise; the slots being present means freshness
  holds and nothing re-fetches on a clean checkout, but the field
  should be repo-relative for in-repo registries. Logged as a debt
  candidate for the registry layer (not fixed here — Phase 0 is
  no-code-change).
- DBT-0016 (PLAYBOOK vs BROWNFIELD marker homing, tripwire
  `touch:spec/neworder/**`) fired on this change set, as designed.
  The v0.2 package dissolves the conflict's subject: the PLAYBOOK
  side is superseded by the generalized RAID playbook + this plan.
  Disposition updated accordingly.
- `[[registry]].url` accepts only git-cloneable URLs; a plain
  directory registry is CLI-flag-only (`--registry <path>`). Fine
  for the pilot (the recipe is in the shim README), but it means a
  bare `vibe install` after a `[requires]` edit cannot see
  `packages/` — re-resolve must repeat the `--registry` flag.
  Worth a PROP note when the cache (PROP-010) lands.
- **conform cached-vs-clean divergence (engine defect, found by
  this phase's gate).** The Phase-0 `conform check` flagged
  `unsafe-gate|crates/vibe-index/src/cli/stop.rs` as 1 NEW at line
  35 with the baseline entry (line 33) no longer firing. The file
  is untouched since `a9dc160` (the prior terraform's scope!
  backfill, which shifted the block +2 lines) — and that commit's
  own merge-time gate reported "0 new, 6 frozen". A clean-cache
  re-run of `cargo xtask conform check` in a worktree pinned to
  `a9dc160` reproduces **1 new** — so the merge-time green was an
  artifact of a stale `target/conform/` facts cache surviving a
  change to the very file it described. The store's
  `(file content-hash, producer)` key should have invalidated;
  it did not. Filed for the Phase-2 conform work (the engine is in
  scope there); the baseline line number is corrected 33→35 in this
  change set (same frozen finding, same count — not baseline
  growth). Discipline lesson for the REPORT: a checker whose cache
  can lie fails the scaffold-reality checklist's "cannot silently
  lie" clause — the determinism check must cover the cache path.

**Gate panel at phase close (all green):**

- `cargo xtask specmap --check` — clean: **352 spec units / 170
  tagged items / 177 edges / 0 suspects**, 6 known
  pin-into-unmarked warnings; orphan ratchet 0 gated, 6
  dispositioned (DBT-0019), 8 reasoned exemptions.
- `cargo xtask conform check` — **6 findings, 6 frozen, 0 new**
  (after the honest line correction 33→35; set and count
  unchanged).
- `cargo xtask test-gate` — **1075 results, 0 failed, 3 skipped**,
  xfail-strict green.
- `bash tools/self-check.sh` — all four steps green (`cargo fmt
  --all --check`; workspace tests; `clippy -D warnings`;
  `vibe check` 0/0/0). One fmt fix fell out of the URI rewrite
  (`specmark/tests/usage.rs`: the longer `discipline/` URI pushed
  an attribute over the line limit).

**Phase 0 exit criteria: met.** vibevm builds; the index
regenerates deterministically; 0 gated orphans; 177 edges / 0
suspects preserved; the Discipline is an installed package pinned
by `vibevm.discipline.lock`; `spec/neworder/` is a shim.
Predictions P0-1 (with the cache-defect caveat) and P0-2 recorded
with verdicts in `PREDICTIONS.md`.

---

## 2026-06-11 — Phase 1: Substrate — the fast loop (Class E)

**Scope.** Card `scaffold-e-fast-loop` adopted repo-wide. Cell
granularity for the loop is the workspace crate (18 members); the
finer `#[cell]`-manifest grain stays the modification unit, but the
isolated build+test unit Rust actually offers is the package.

**What landed.** `cargo xtask fast-loop [--cell <name>]
[--budget <secs>] [--enforce-budget]` — the card's checker
`cell-fast-loop-present`, implemented. Per cell it runs
`cargo nextest run -p <cell>` in isolation, measures wall-clock to
the verdict, parses results with the same testgate parser the
test-gate uses (the two gates cannot disagree on what a test result
is), and writes a machine-readable report to
`target/fast-loop/report.json` (derived, never committed). Test
failures always fail the command; budget overruns warn unless
`--enforce-budget` — and since the whole workspace already fits,
enforce-budget is safe to use at raid checkpoints from day one.

**Measurement (warm target, 2026-06-11):** 18/18 cells within the
60s budget — 100%, against the card's ≥90% prediction. Worst cell:
`vibe-cli` ~23s (269 results); median ~2s. Zero red cells, zero
hidden coupling. The card graduates from *specified* to
*implemented* on the pilot.

**Checker-shape finding (feeds the card's Band 2):** nextest exits
4 on a zero-test crate; the first run reported four stub/generated
cells (vibe-graph, vibe-llm, vibe-wire, xtask) as RED for having no
tests. That is a false signal — a zero-test cell's *build* is its
first signal — fixed with `--no-tests=pass`. Lesson for the card:
"builds and tests in isolation" must define the no-tests case
explicitly or every adopter rediscovers this edge.

**Gate semantics going forward.** `fast-loop --enforce-budget`
joins the raid-checkpoint panel (structure changes); it does NOT
join `tools/self-check.sh`, which already runs the full workspace
test suite — duplicating ~80s of tests into every self-check buys
nothing the panel does not already buy. Doctests are not yet in the
loop (nextest does not run them); they enter via the Phase-2 G
card, which will wire `cargo test --doc -p <cell>` into fast-loop.

**Phase 1 exit: met.** Every cell independently buildable +
testable inside budget; checker implemented and green; P1-1
recorded with verdict.

---

## 2026-06-11 — Phase 2: Diagnostics (F) + doctests (G)

**Scope.** Cards `scaffold-f-structured-diagnostics` (inline) and
`scaffold-g-doctests` (gate), engine-first: the conform engine
learns the fact shapes and rules, then the gated crates conform.
Gated set starts at the priority cells — vibe-resolver,
conform-core, specmap-core — and grows with the cell sweep, the
orphan-ratchet expansion rhythm.

**Class F landed.**

- Every conform finding now speaks the card's grammar:
  `violates REQ <uri>: <why>; fix surface: <where>`. Renderer
  (`rules::req_message`) and acceptor (`rules::matches_req_grammar`)
  live side by side so they cannot drift, and a test walks every
  rule over a violating corpus asserting grammar conformance —
  Class F applied to conform itself.
- REQ URIs cite two namespaces: `spec://vibevm/…` for
  vibevm-hosted units and `discipline://<package>/<doc>#<anchor>`
  for the installed Discipline package (version resolved against
  `vibevm.discipline.lock`). The convention note lives in
  `spec/discipline/README.md`; this is the practical first instance
  of the pending PROP-014 external-namespace amendment, citation-only
  (diagnostics cite; specmap edges still never point at package docs).
- New rule `error-enum-cites-req` (Class F): a thiserror enum in a
  gated crate must carry a `#[spec]` REQ edge. Zero findings on the
  gated set — vibe-resolver's error layer was already fully tagged
  by the prior terraform's item-grain backfill.
- `Fact::ErrorVariant` joins the fact model (enum attrs travel with
  every variant), and the frontend extracts thiserror `#[error]`
  display templates.

**Class G landed.**

- New rule `seam-has-doctest`: a `pub` item declared at a gated
  crate's root (`src/lib.rs`) is a seam and must carry a compiled
  doctest. New fact fields `is_pub` + `has_doctest` (doc-fence
  detection); frontend version bumped 1→2, which retired every old
  cache slot wholesale — the producer-keyed store doing exactly
  what it was built for.
- First run found **30 undoctested seams** (16 conform-core,
  8 more in its submodules, 2 specmap-core, 6 vibe-resolver — the
  engine measuring its own author honestly). All 30 now carry
  canonical doctests; `cargo test --doc` green on all three crates;
  the resolver's doctests show the blessed paths (DepProvider impl
  shape, NaiveDepSolver::solve over a one-package provider,
  error-display contracts).
- Doctests ride the loop: `fast-loop` now runs
  `cargo test --doc -p <cell>` per cell (nextest alone skips them);
  `tools/self-check.sh` already covers them via `cargo test
  --workspace`.

**Baseline correction (fingerprint hardening).** The unsafe-gate
fingerprint moved from `rule|file|line` to `rule|file|context#ordinal`
— the Phase-0 stop.rs lesson generalized: a line-keyed fingerprint
rots on any edit above the block, and a baseline that rots on
unrelated edits is a checker that lies. The six frozen findings were
rewritten to the new shape (same set, same count); a regression test
pins that a pure line shift no longer changes the fingerprint.

**Cache-divergence note (correcting the Phase-0 entry).** With the
engine now in hand: today's runs give identical results cached and
clean, and the store key (content-hash + producer id-version) is
sound — the v2 bump proved the producer half. The Phase-0 merge-time
green therefore most likely came from the gate panel being run
before the final backfill commit (`a9dc160` itself edited stop.rs),
not from a store defect. Lesson stands, reworded: **the gate panel
must re-run on the final tree of a series** — now standard raid
checkpoint practice in this adoption.

**Gate panel at phase close (all green):** specmap --check clean
(352/170/177/0); conform check 6 frozen / 0 new — with the two new
rules active; test-gate 1082 results / 0 failed / 3 skipped
(xfail-strict; +7 new engine tests); fast-loop --enforce-budget
18/18 within budget, doctests included; self-check all four steps.

**Phase 2 exit: met.** P2-1 recorded (pending by design —
measurement deferred), P2-2 standing; both cards' checkers
implemented and green on the gated set.
