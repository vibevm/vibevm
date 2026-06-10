# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-10 at session end (context-budget checkpoint at 58%).
Branch **`new`**, working tree clean. The terraform effort (the Big
Refactoring) is mid-Phase-2; this file carries everything the next
session needs to continue without re-deriving._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and
> the WAL disagree, the WAL wins. Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

The Big Refactoring = executing
[`spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
on branch `new` (cut from `main` @ `99c63de`; `main` receives nothing
until merge). One session delivered, in order: **Phase −1** (inventory:
xfail-strict test baseline, debt/intent registries, conflict scan,
golden characterization, BASELINE) with same-day owner acceptance;
**four adjudications** under explicit owner sanction incl. frozen boot
files; **Phase 0** (crates `specmark-grammar` / `specmark` /
`specmap-core`, `schemas/specmap.jtd.json` → `vibe-wire`, xtask
`specmap [--check]` / `test-gate` / `tripwire`, first committed
`specmap.json`); **Phase 1 prep** (canonical house-style URIs — the
indexer's full-path URIs would never have joined the repo's citation
style — plus the suspects table, drift classification, `xtask trace
explain`); **Phase 1 pilot + drift drill** (PROP-003 §2.6.1 unit-ified
additions-only; first production tags in
`crates/vibe-resolver/src/conditional.rs`; both drill signals fired and
reverted; dossier `terraform/PHASE1-PILOT.md`); **Phase 2 started** —
the latent-corpus mining is done (results below), the crate sweep is
NOT (deliberately left whole for a fresh context).

**Owner's standing directions this session:** no side-branch PRs —
everything lands on `new`, decisions/confirmations happen in
conversation; frozen surfaces may be edited under in-session sanction
(used for boot files); "продолжай с максимальным уровнем ризонинга".

## Where work stands

- **Branch `new`** at `630ba3b` (after this checkpoint: two more docs
  commits), pushed to `origin/new`. Gate green: `self-check.sh` all
  four steps; `cargo xtask specmap --check` clean (**443 units / 17
  tagged items / 19 edges / 0 suspects / 6 warnings** — the six are
  `pin-into-unmarked-unit` from specmark's own usage tests, by design
  until PROP-014 is unit-ified); `cargo xtask test-gate` green (1051
  results, xfail-strict).
- **Drill commits live in history as documentation:** `4395d3b` pilot →
  `b3a947c` drill (a) → `73b6e81` drill (b) → `4afe716` revert.
- **The owner keeps dropping discipline-package documents into
  `spec/neworder/`** mid-session (TS/Python guides, then three C++
  guides + README edits). The tripwire (DBT-0016 watch) catches them;
  commit such drops promptly (`2399ccd`, `630ba3b` are the precedents).

## Active blocker / owner inputs pending

1. **Pilot judgment calls** (PHASE1-PILOT.md §1, in-chat confirm):
   (a) the extra `design` unit in PROP-003 §2.6.1; (b) fixpoint /
   host-invariance units deliberately uncovered from this module;
   (c) drill ran over grammar, not fixpoint. Not blocking Phase 2
   steps 1–2; blocking the affirmation pass only if answers change
   the unit set.
2. **Phase 2 affirmation** (the real gate): once
   `terraform/specmap-proposals.json` exists, the playbook hard-requires
   owner APPROVE per proposal before any `#[spec]` is written.
3. **CI bullet** (Phase 0, deferred): repo has no CI infrastructure;
   introducing one is a Rule 4 owner decision.

## Next steps — finish Phase 2 (exact recipe)

1. Boot, then read [`terraform/PHASE1-PILOT.md`](terraform/PHASE1-PILOT.md)
   and playbook §phase2 + PROP-014 §2.7/§4-Phase-2.
2. **Write `terraform/specmap-proposals.json`** (proposals ONLY — edit
   no source file). Schema: `{ schema: 1, scope, note, mined_commits[],
   proposals[] { id PRP-NNNN, item, item_kind, file, verb, uri,
   confidence: high|medium|low, evidence_code: "one-line quote",
   evidence_spec: "one-line quote", status: "pending" },
   candidate_orphans[] { item, file, reason } }`. ≤3 edges per item;
   no spec-side evidence → candidate orphan; disputed-only target →
   `blocked-on-dispute` (none disputed today).
3. Use the **Phase 2 staging data** below (already mined/surveyed —
   do not redo).
4. Report the proposal table to the owner for APPROVE/reject in chat.
5. After APPROVE: affirmation pass — write the tags, one commit per
   module, commit bodies citing URIs; `cargo xtask specmap --check` +
   `cargo xtask test-gate` before each commit.
6. Then the ratchet decision (blocking specmap-check for vibe-resolver)
   and the Phase 2 acceptance math (≥90% of ratified non-disputed req
   units implemented+verified; planned/disputed reported separately).

### Phase 2 staging data (mined this session — reuse, don't redo)

**Latent corpus** (`git log --all --grep='spec://'` ∩ vibe-resolver
files; 106 spec://-citing commits total, 4 touched the resolver):

| commit | subject | URIs cited | resolver files |
|---|---|---|---|
| `4395d3b` | the PROP-014 pilot | PROP-003#req-conditional-{grammar,composition} | conditional.rs (already tagged) |
| `c5c4fe6` | group-qualified package identity (PROP-008 Ph.2) | PROP-008-qualified-naming | lib.rs, local/multi providers, naive.rs |
| `9b662c5` | mandatory [package].group | PROP-008#group | naive.rs |
| `b794e7a` | unify manifests into vibe.toml | PROP-007 | lib.rs, providers, naive.rs |

**Public surface** (from lib.rs + module heads; line refs are current):

- `lib.rs`: `ResolvedNode` (carries `group` per PROP-008 §2.3 — the
  doc-comment says so), `ResolvedGraph` (+iter/roots/find),
  `DepProvider` trait, `DepSolver` trait, `DepProviderError`,
  `SolveError` (variants VersionConflict / ConflictsDeclared /
  CapabilityUnmet / DisjunctionUnsatisfiable — error-as-contract
  candidates), pub(crate) SolverState. Module doc cites
  **PROP-002 §2.8 (depsolver), §2.9 (capability vocabulary)** verbatim.
- `activation.rs`: module doc "Subskill activation evaluator —
  PROP-003 §2.5.2"; `ActivationContext` (+add_present/add_provides),
  `ActivationOutcome`, `evaluate(...)`.
- `features.rs`: module doc "Feature expansion engine — PROP-003 §2.4";
  `FeatureValue::parse`, `FeatureExpansion::merge`, `FeatureRequest`,
  `expand_features`, `validate_features_table`, `FeatureError`.
- `naive.rs` (780 lines): "Naive depth-first solver… see crate docs for
  pinned limitations"; `NaiveDepSolver` (+new/provider).
- `local_registry_provider.rs`: DepProvider over LocalRegistry
  (`--registry <path>`); `LocalRegistryProvider::new`.
- `multi_registry_provider.rs`: DepProvider over MultiRegistryResolver
  (registry/mirror/override dispatch); `MultiRegistryProvider::new`.

**Spec-side anchors to propose against** (all unmarked except the
pilot five — proposals into unmarked units are fine, the affirmation
will surface `pin-into-unmarked-unit` honestly *if pins are used*;
consider proposing unpinned edges into unmarked units):

- PROP-002 (`modules/vibe-registry/PROP-002`): `#solver` (§2.8 —
  DepSolver trait / resolvo-primary / NaiveDepSolver fallback),
  `#capability` (§2.9 — provides/requires/requires_any vocabulary).
- PROP-003 (`modules/vibe-resolver/PROP-003`): `#features` (§2.4 →
  features.rs), `#subskill-activation` (§2.5.2 → activation.rs),
  `#solver-upgrade` (§2.1 → NaiveDepSolver-to-SAT path),
  `#migration` (§2.11 NaiveDepSolver migration), `#interface-tags`
  (§2.6 → capability matching in naive.rs), `#determinism` (§3.3),
  plus the pilot five (`#req-conditional-*`, `#design-conditional-*`).
- PROP-008 (`modules/vibe-registry/PROP-008`): `#identity`-class units
  for ResolvedNode.group (check exact anchors in specmap.json).

**Tooling note:** query units via the committed index —
`python3 -c "import json; d=json.load(open('specmap.json')); …"`
filtering `doc_path` — the session did exactly this; it is faster and
truer than re-grepping markdown.

## Non-obvious findings (this session)

- **URI canonicalization was load-bearing**: the indexer originally
  emitted `spec://vibevm/spec/<path>.md#a` while the whole repo cites
  `spec://vibevm/<dir>/PROP-NNN#a`. Fixed in `dc79001`
  (`mdspec::canonical_doc_path`); without it the pilot tags would have
  joined nothing.
- **`git commit -m` with backticks is unsafe in this shell** — command
  substitution fired twice (`206d24f` caught pre-push and amended;
  `40077bf` is pushed with one word eaten: "gains a field" should read
  "gains a `file` field"). Convention now: only `git commit -F -
  <<'MSG'`. Recorded in user-memory too.
- **Fenced code blocks leak units**: sample headings inside ``` fences
  in the guides initially became index units; `mdspec::fence_mask`
  excludes them (test covers it).
- **specmap.json includes untracked files** (walkdir knows no git):
  owner-dropped guides enter the inventory before they are committed —
  commit them promptly or `--check` drifts.
- **unbumped-hash is suppressed for unmarked units** (no revision
  discipline to audit); parent-envelope units (`#conditional-deps`)
  legitimately change hash when sub-units are inserted — nesting is by
  design.
- **Power-loss resume worked cleanly**: all tool-written files were on
  disk; only the not-yet-run build/test/commit steps needed redoing.

## Repository map (delta vs pre-terraform)

```
vibevm/
├── specmap.json                     ← committed traceability index (schema 2)
├── schemas/specmap.jtd.json         ← its wire contract → vibe-wire::generated::specmap
├── crates/
│   ├── specmark-grammar/            ← single source of the tag grammar (syn-level)
│   ├── specmark/                    ← proc-macros #[spec] / #[verifies] / scope!
│   ├── specmap-core/                ← mdspec, rscan, index, explain, testgate, tripwire
│   └── vibe-resolver/src/conditional.rs  ← first tagged production module
├── xtask/                           ← + specmap [--check] / test-gate / tripwire / trace explain
├── terraform/
│   ├── BASELINE.md  PHASE1-PILOT.md  LOG.md
│   ├── registry/    ← debt.json+DEBT.md (18), intent.json+INTENT.md (31), tests-baseline.json (3)
│   └── golden/      ← 5 hermetic flows + capture.sh (byte-deterministic)
└── spec/neworder/   ← the discipline package v0.2-beta (12 docs incl. 3 C++ guides)
```

## Decisions in force (terraform-specific, beyond the four rules)

- Playbook precedence §0.2; frozen surfaces per §0.4 (owner sanction
  was granted in-session and used; the freeze itself stands).
- xfail-strict semantics for every test gate; baseline shrinks only via
  the promotion protocol (§7.2). No drive-by fixes (§7.1).
- Tripwires are read, not muted (§7.5) — address each firing in the
  commit/PR text.
- Proposals-then-affirmation for all backfill tags (PROP-014 §2.7):
  LLM proposes, owner approves, the affirmation diff is the act.
- Anchors immutable; `spec-editorial: <anchor>` commit-body marker for
  editorial spec edits (first live use: `73b6e81`).
- Index regeneration accompanies every change that moves units/tags —
  `specmap.json` is committed, `--check` is the gate.

## Recent commit chain (newest first, this session's tail)

```
630ba3b docs(spec): three C++ guides join the discipline package
002af04 docs(wal): Phase 1 checkpoint — pilot executed, drill green
4b2f72b docs(terraform): Phase 1 pilot dossier + session log
4afe716 docs(spec): revert the drift-drill edits — pilot state restored
73b6e81 docs(spec): drift drill (b) — editorial edit, unbumped-hash fires
b3a947c docs(spec): drift drill (a) — semantic bump, suspects, re-affirmation
4395d3b feat(resolver): the PROP-014 pilot — PROP-003 units x conditional.rs tags
dc79001 feat(specmap): canonical URIs, drift diagnostics, trace explain
40077bf feat(wire): specmap schema 2 — unit file paths + the suspects table
1d6b2d4 docs(wal): Phase 0 checkpoint — tooling skeleton shipped
d718b6b docs(terraform): log the adjudication + Phase 0 session
206d24f feat(xtask): specmap, test-gate, tripwire + the first committed index
9b1804d feat(specmark): inert tags, shared grammar, and the specmap engine
f7eda9b feat(wire): specmap.json wire contract (PROP-014 §2.5)
2399ccd docs(spec): TypeScript and Python guides join the discipline package
3ba3ff3 docs(terraform): record the four adjudications in the debt registry
d090cb0 docs(spec): disambiguate PROP-003 duplicate {#phases} anchor
aa54ab4 docs(spec): PROP-002 naming reconciled to the PROP-008 fqdn default
0e57f0f docs(boot): reconcile boot snippets with split-host + fqdn reality
584c080 docs(wal): terraform Phase -1 checkpoint
23e5e73 docs(terraform): Phase -1 BASELINE snapshot + session log
581e361 docs(terraform): seed the debt and intent registries
6a549bb test(terraform): xfail-strict test baseline + golden characterization
2d4c235 docs(spec): add the Discipline terraform package v0.2-beta
ccbc3d9 docs(wal): route the Big Refactoring to branch new
```

## Quick-start

```sh
bash tools/self-check.sh                  # the four-step gate
cargo xtask specmap --check               # index gate
cargo xtask test-gate                     # xfail-strict test gate
cargo xtask tripwire                      # debt watches over the change set
cargo xtask trace explain vibe_resolver::conditional::ConditionalPredicate::parse --text
```

Session-resume phrase: `восстанови сессию`. The WAL supersedes this
snapshot wherever they diverge.
