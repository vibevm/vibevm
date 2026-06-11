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

---

## 2026-06-11 — Phase 3: Typed builders (B) + runnable contracts (C)

**Scope.** Cards `scaffold-b-typed-builders` (gate) and
`scaffold-c-runnable-contracts` (inline) on the resolver and
lockfile seams, scoped tightly per the B card's Non-Goals ("NOT
typestate everywhere — over-typing fights idiom").

**Class B landed — `CapabilityTag` on the activation seam.**
`ActivationContext.present/provides` were `BTreeSet<String>` with
`add_present(impl Into<String>)`: a caller could feed `"rust"`
where `"stack:rust"` was meant and the probe would silently never
match — the exact statistically-likely-wrong-call shape the card
names. Now the sets hold `CapabilityTag` (parse-only constructor,
`<namespace>:<name>` both halves non-empty, `Borrow<str>` so
manifest rule strings still look up directly), and the wrong call
fails `cargo check`. The migration itself was the live demo: the
compiler enumerated every call site (vibe-cli's
`build_activation_context`, the conditional-dep tests) as
mismatched-types errors. A trybuild compile-fail test
(`tests/ui/bare_string_tag.rs`) pins the wrong shape red — the
card's checker step 5, implemented. `build_activation_context`
became `Result` (parse failures are loud, not skipped), and
`TagError` carries the REQ edge per the Phase-2 F rule.

**Recognition fired, application declined (recorded per the card's
Goals/Non-Goals):** the `is_root: bool` parameter through
`process_one`/`EnqueuedPkg` matched the detector ("bool args"), but
it is cell-internal, not a seam — typing it would be the
over-typing the card forbids. The `seam-protocol-typed` conform
rule (the checker's T-sem half) needs signature facts the frontend
does not yet carry — queued as frontend v3 work; the trybuild half
stands in. The card is implemented-with-a-named-gap, honestly.

**Class C landed — three witnesses, one false start.**

- naive.rs: the roots-first output ordering contract — a
  root-flagged entry surviving into the sorted `rest` would break
  the prefix invariant `[meta].root_dependencies` builds on; now a
  debug_assert at the build site.
- lockfile.rs: `(group, name)` uniqueness witnessed at `read()` —
  `find`/`find_mut`/`remove` treat the pair as a unique key, and a
  hand-edited duplicate would make lookups position-dependent.
- features.rs: AUD-0014 closed — the doc claimed cycles are
  "detected and rejected"; they terminate silently via the `seen`
  set (the `cycles_terminate` test proves it). Lying prose is
  adversarial input (guide §8); the line now states the truth and
  cites the test. AUD-0015 closed in the same sweep (ResolvedNode
  doc cited PROP-008 §2.3 for identity; it is §2.2).
- **The false start, kept on the record:** the first draft also
  asserted root-key uniqueness at solver input. The existing
  `detects_version_conflict_across_paths` test killed it in
  seconds — duplicate roots are legal input that must surface
  VersionConflict through the normal path. A wrong contract costs
  a red test in the loop; that asymmetry is the card working as
  designed, witnessed from the failure side. handle_disjunction
  needed nothing: its loud early-return Err IS the contract.

**Gate panel at phase close (all green):** specmap --check clean —
352 units / **173 items / 180 edges** (+3: CapabilityTag, TagError,
the compile_fail scope) / 0 suspects; conform 6 frozen / 0 new;
test-gate **1083** results / 0 failed / 3 skipped (+1 trybuild);
self-check all four steps.

**Phase 3 exit: met.** P3-1 (compile-time error class) held and
demonstrated live; P3-2 (loud witnesses) held with the
counter-lesson recorded.

---

## 2026-06-11 — Phase 4: Differential oracles (D)

**Scope.** Card `scaffold-d-differential-oracle` (gate) around the
algorithmic core. The prior terraform left one fixed-case
differential oracle (the DepProvider pair over hermetic `file://`
git repos); this phase adds the property-based net and the rule.

**The property net** (`crates/vibe-resolver/tests/solver_properties.rs`).
proptest generates random acyclic package worlds (1–6 packages,
1–2 versions each, forward-only deps — cycles unrepresentable by
construction) over an in-memory `WorldProvider` (deliberately also
a Class-H registry fake). Four properties pin the solver's
observable contract, 64 cases each, milliseconds total —
comfortably inside the fast-loop budget: determinism (double-solve
byte-identity), dependency closure (every output edge lands on a
node whose version satisfies the pin), roots-first prefix +
marking, exact `=x.y.z` pinning (the lockfile reproducibility
contract). These test behavior nobody enumerated case-by-case —
the safety net a weak reader cannot derive.

**The differential socket.** `assert_solvers_agree(a, b, roots)` —
identical normalized graphs or identical error classes, anything
else fails. Today it smoke-tests naive-vs-naive (proving the
harness); Phase 7 swaps one side for the SAT solver. DBT-0011's
landing pad is now built and green.

**The rule** (`cell-has-oracle`, Class D, self-scoping). Every
`#[cell]`-manifested type must be referenced from at least one
integration test of its crate — the static approximation of "an
oracle exists"; a cell nobody's tests touch has no behavior net at
all, and replacing it merges blind. Implementing it required facts
from `crates/*/tests/` — the engine's walk grew a `tests` limb,
and that wider net immediately caught two previously-invisible
`unsafe` blocks in `vibe-publish/tests/post_hook.rs` (the
edition-2024 `env::set_var`/`remove_var` guards). Frozen into the
baseline as pre-existing reality newly visible (6 → 8 frozen, the
same legitimacy as the original six) — the context#ordinal
fingerprints from Phase 2 made the freeze line-shift-proof from
day one. 0 findings from the rule itself: all three existing cells
were already oracle-covered.

**Gate panel at phase close (all green):** specmap --check clean
(352/173/180/0 — the +3 items/edges are the property suite's
verifies tags); conform 8 frozen / 0 new with cell-has-oracle
active; test-gate green xfail-strict (+5 property tests); fast-loop
within budget; self-check all four steps.

**Phase 4 exit: met.** P4-1 (the central C-7 transfer test)
recorded as pending-with-instrumentation-ready — the first
prediction whose falsification needs an actual weak-agent run;
P4-2 holding from birth.

**Post-phase process correction (third occurrence, now fixed).**
The Phase-4 commit shipped and pushed with clippy red: a gate
behind `| tail -1` returns the pipe's exit status, not the gate's,
and `&&` sails on. Same failure shape as the Phase-3 cwd slip and
the Phase-0-noted panel-ordering gap. The raid recipe is now
explicit and in use: gates run with their own exit status captured
(`set -e` + redirect, tail the log file separately). REPORT input:
the discipline specifies checkers but not the **gate-invocation
pattern** — a verdict the caller can silently drop fails the same
cannot-silently-lie clause the engine is held to.

---

## 2026-06-11 — Phase 5: Generators (A) + simulators (H)

**Class H landed — the fixpoint simulator.**
`vibe_resolver::fixpoint_model` is a runnable reference model of
the conditional-dependency loop (solve → probe → add → re-solve,
PROP-003 §2.6): a world of `ModelPackage`s (name, provides-tags,
`trigger → dep` conditional edges), a `step()` that returns the
observable record (present-set, fired edges, added packages,
stability), and `run(max)` mirroring the production loop's
iteration cap. The monotone-lattice property — the present-set
never shrinks, WHY the loop terminates — is a debug_assert at
every step, not prose. Five behavior tests (immediate stability,
two-stage cascade, provides-triggering, joint fixpoint, cap
behavior) plus a doctest showing the canonical world.

**The model's license to exist:** the card names model-drift as
the failure mode, so `tests/fixpoint_conformance.rs` rebuilds the
loop from the production primitives (`ConditionalPredicate`
evaluated over a real `ActivationContext`, the same qualified-tag
shapes `build_activation_context` emits) and steps it in lockstep
with the model on representative worlds — per-iteration added-sets
must match exactly. The model and the production loop cannot
drift apart silently.

**Class A — recognition fired, application correctly declined.**
The plan names "transition tables, exhaustive matches" as
candidates. Honest survey: the wire-type generator (JTD →
vibe-wire, `xtask codegen` + `check-codegen` regenerate-and-diff)
is already a complete card-A instance — generator, committed
plain-Rust output, determinism check. Inside the resolver, the
activation channels LOOK near-duplicate but differ in probe
semantics (PATH probe vs glob vs env vs PURL match) — a generator
there would encode business decisions, the card's named misuse;
and the `composition` predicate tables belong to Phase 7's
formalization, where they will be evaluated against card A again.
No artificial generator was built to tick a box.

**Gate panel at phase close (all green, statuses captured):**
specmap regenerated (+6 edges — the model's and conformance
suite's spec/verifies tags), `--check` clean; conform 8 frozen /
0 new; test-gate green xfail-strict (+8 model & conformance
tests); self-check all four steps.

**Phase 5 exit: met.** P5-1 standing (conformance keeps the model
truthful; agent-behavior half deferred with P4-1), P5-2 held for
the existing generator instance.

---

## 2026-06-11 — Phase 6: Codemods (I), pilot-gated

**Scope.** Card `scaffold-i-codemods` ([E-hyp], WISH→prototype).
One real recurring multi-file change of THIS repo, implemented as
one checked atomic operation: **add-cell** — the module file with
its `#[cell]` manifest, the alphabetical `pub mod` registration in
lib.rs, and a smoke test referencing the cell so `cell-has-oracle`
is satisfied from birth. `--spec-uri` is a required parameter: a
cell without a REQ edge is an orphan the ratchet rejects, so the
codemod makes A1 true by construction, not by follow-up.

**The live demo demonstrated both arms.** The first invocation
(the SAT-solver skeleton for Phase 7, using exactly the
fixed-parameter shape from the command's help) hit a template bug —
the generated module imported `spec` but not the `cell` attribute
macro — and the post-check (`cargo check -p`) caught it and rolled
all three writes back; the tree was byte-identical to before. After
the one-line template fix the same invocation succeeded: skeleton
in place, smoke test green, conform 8 frozen / 0 new with the new
cell visible to the rules. Atomicity and the post-check are not
theoretical properties; they both fired on real inputs within five
minutes of the prototype existing.

**Weakest-tier exposure** is the documented fixed-parameter
invocation in the command help (the card's routine step 5). Free
parameterization by weak agents is the R4 build/use-boundary
measurement — deferred with the other agent-run questions.

**Phase 6 exit: met (prototype-grade by design).** P6-1 recorded:
mechanism proven both ways, capability half pending. The card's
REPORT-gated graduation: checker exists and ran; [E-hyp] stays
until the agent measurement.

---

## 2026-06-11 — Phase 7: the SAT solver (DBT-0011) + the fixpoint formalized

**The Sat cell** (`crates/vibe-resolver/src/sat.rs`, born from the
Phase-6 codemod skeleton). Design: chronological backtracking over
version *bounds* with **the naive solver as the branch checker** —
each attempt runs the full naive solve under a `BoundedProvider`
that caps conflicting packages below their previous picks, entirely
through the unmodified `DepProvider` trait (`resolve_version` with
an intersected `<bound` constraint IS "next lower candidate").
Features, conditional deps, capabilities, conflicts, obsoletes are
evaluated by exactly the code the naive path runs, so the two cells
cannot drift semantically. Termination: bounds strictly descend
over finite version sets; `MAX_ATTEMPTS` backstops. Unsatisfiable
worlds report the ORIGINAL conflict, never a backtracking artifact.

**The oracle over-delivered.** The first differential draft
asserted strict naive≡sat equality over generated worlds, on the
belief the generator only made conflict-free worlds. proptest
falsified that belief within seconds: it found a world where a root
takes a dep's highest version and another path carets a lower
major — naive's first-pick-wins trap, arising naturally. Sat
solves it. The differential is now the **dominance contract**
(naive-solvable ⇒ identical normalized graphs; naive-fail ⇒ sat may
solve — its documented superiority; sat-fail-where-naive-solves ⇒
always a bug), which is the card's "documented divergence list"
done property-grade. Four unit cases pin the trap, chained
backtracking, unsatisfiable reporting, and the conflict-free fast
path.

**DBT-0011 disposition: fixed (the backtracking half).** The
resolvo-primary half of PROP-002 §2.8 stays an owner option,
recorded as the `deviates` edge on Sat's impl. Production solver
selection (a registry flag per R-001, with provenance/birth/sunset)
is the remaining wiring — tracked by the cell sweep, deliberately
not smuggled into this phase.

**Composition formalized.** `context(...)` now parses `and` / `or`
/ `not` with parentheses and standard precedence (recursive-descent
over word-matched tokens — a key like `org.vibevm/x` can never be
split). PROP-003 `#req-conditional-composition` ratified r1-planned
→ r2; the old deviates edge became implements r2 (the asymmetric-
invalidation path: the only r1 pin on that unit was the edge this
change replaces, so the index stays suspect-free). The richer probe
forms (`if_files = …` inside `context(...)`) stay loud-Unsupported,
now recorded as a `deviates` on the grammar edge. The
virtual-capability channel (§2.5.3) remains blocked on the
owner-deferred M1.5 LLM layer; its monotone-lattice semantics are
already executable in the Phase-5 model.

**Cards on the new code:** B — Sat's choice state is
private-by-construction (bounds map + stack, no protocol surface to
mistype); C — the monotonicity witness lives in the model, the
original-conflict guarantee in a unit contract; D — the dominance
differential; G — the canonical doctest; H — the fixpoint model
covers the loop the solver participates in.

**Phase 7 exit: met.** P7-1 held (and the oracle found the naive
trap before any human enumerated it).

---

## 2026-06-11 — The priority-cell sweep (plan §4) and adoption close

**Sweep, batched per crate (the raid skeleton's batch unit), the
doctest/REQ work fanned out to four parallel authoring agents with
identical briefs and verified centrally by the gates:**

- `vibe-registry` — 6 root-seam doctests (LocalRegistry and
  compute_content_hash `no_run` — they walk the disk; the Registry
  trait shows the canonical minimal impl consumed as `&dyn`);
  all three thiserror enums gained REQ edges (RegistryError,
  GitError → PROP-002#failure-discriminator; IndexError →
  PROP-005#http, the deliberate cross-PROP cite).
- `vibe-workspace` — 3 doctests, all *runnable* (tempfile is a
  regular dep there, so the Workspace::discover example builds a
  real hermetic workspace — strictly stronger than `no_run`);
  WorkspaceError → PROP-007#nesting.
- `vibe-check` — 6 doctests (check_project `no_run`); no error
  enums exist (failures are Finding data, not Err) — N/A recorded.
- `vibe-publish` — 10 doctests (Publisher `no_run`; secrets
  discipline held: dry-run paths, fictitious endpoints);
  PublishError → PROP-002#publish.

**The widened gate immediately out-scoped the agents:** turning the
F rule on for vibe-publish flagged `HookError` (post_hook.rs) — a
file the publish agent had deliberately left alone as outside its
lib.rs brief. The rule reads the whole crate; the gap got its edge
(PROP-005#integration) within minutes. Centralized checkers beat
per-agent scope judgment — worth a line in the v0.3 raid playbook.

**Gated sets now:** seam-has-doctest and error-enum-cites-req cover
seven crates (engine three + registry/workspace/check/publish).
vibe-core stays out of F deliberately: its error trio is
DBT-0019-dispositioned and the rule has no disposition concept —
gating it would re-flag adjudicated debt (the reason lives as a
comment at the gate site).

**Final gate panel (all green, statuses captured):**
specmap --check clean — **352 units / 190 items / 198 edges /
0 suspects** (the adoption added 20 items and 21 edges net);
conform **8 frozen / 0 new** across six rules and seven gated
crates; test-gate xfail-strict green; fast-loop --enforce-budget
**18/18** with all 55+ doctests riding the loop; self-check all
four steps.

**Adoption exit criteria (plan §5): all four met.** The REPORT
(`REPORT.md`) is the synthesis and the Discipline-v0.3 input.
