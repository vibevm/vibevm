# TASKS — vibevm, active work

Live checklist for the current work-slice. Each item is a logical commit
(Conventional Commits per [PROP-000 §12.2](spec/common/PROP-000.md#conventional-commits);
grouped by meaning per §12.3).

**Status key:** `[ ]` queued · `[~]` in progress · `[x]` done.

**Where the numbers live.** This file never carries counts. The campaign's own
two commands do:

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
```

---

## How this file relates to the four that resemble it

Since 2026-06 vibevm's work-slices are **campaigns**, not loose checklists, and
four documents divide the job. This file is the *shortest* of them — the slice
in flight, nothing else:

| Document | Holds |
|---|---|
| `TASKS.md` (this file) | The slice in flight — each line a commit waiting to be made |
| [`TOOLING-MAP.md`](TOOLING-MAP.md) | The wave order and the owner forks each wave carries |
| [`BACKLOG.md`](BACKLOG.md) | Findings triaged P1/P2/P3 that nobody is working on yet |
| [`campaigns/packages-2026-09/BATCH-PLAN.md`](campaigns/packages-2026-09/BATCH-PLAN.md) | The running campaign's phase/batch mechanics |

A line here is a *commit*; a line in the map is a *build*; a line in the
backlog is a *finding*. When they disagree, the backlog entry carries the
owner's ruling and wins.

---

## Current slice: draining the backlog (2026-08-06)

The owner's course of 2026-08-05 stands: **drain `BACKLOG.md` first, stay away
from the tests.** Every row is measured against the authored tree before any
work starts on it — over three days that has stopped nineteen builds of things
already built.

- [x] `fix(vibe-index)` + `build(self-check)`: **[B-008] closed.** One of twenty
      workspace members declared no licence at all, against a norm PROP-000 §3
      has carried since the relicensing and an owner-maintained ledger states as
      fact. Nothing checked it — not the panel, not conform, not `vibe check` —
      which is why it drifted for months. The crate joined its siblings and the
      norm got its checker, reading the member list out of the workspace
      manifest so a crate added tomorrow is covered. Proven not blind: it fails
      on a copy of the tree as it stood an hour earlier.
- [x] `test(vibe-cli)`: **AUDIT `-01` closed — the oldest open finding.** The
      default path (`vibe init` with no registry flag → `vibe install`) had no
      e2e at all, and that is the hole finding `-02` shipped through for eight
      phases. The harness had existed since Phase 3; what was missing was a test
      that declares its registry where a real user's lives — the machine-global
      home — and asserts the project manifest stays empty, which is what stops
      it becoming a copy of the test that already exists.
- [x] `feat(vibe-resolver)`: **[B-045] closed.** The kind prefix is validated
      after resolution, `uninstall` and `update` take a bare short name from the
      lockfile alone, the redirect verbs keep the requirement with its reason
      recorded beside the code, and the citations moved. `SolveError` left
      `lib.rs` for its own module on the way — the file had been ten lines under
      budget before any of this, the hazard `##B054-THE-CLASS` names.
- [x] `docs(campaign)`: **[B-047]'s first item measured** — nineteen of
      twenty-nine capabilities keep their substance outside `vibe-cli` and ten
      do not, the largest being the whole version manager; two of five MCP tools
      hold the norm, one duplicates a renderer, two have no CLI sibling.
      Evidence, no verdicts: the design call is a separate step.
- [x] `fix(campaign)`: **the stability report stops printing a vacuous zero.**
      It compares two fields inside the cache, so a spec edited since the last
      scan is invisible to it — met live on a document carrying 92 verdicts.
      It now names every judged file whose cached digest no longer matches its
      bytes. Filed and fixed as AUDIT `2026-08-06-02`.
- [x] `docs(audit)` ×3 + `docs(backlog)`: three measurements corrected against
      the tree — `-10`'s sweep is 27 files and 169 occurrences rather than 12
      and ~40; B-007's ADR adoption tripled in five days while the row waited;
      `-10`'s coupling to B-045 is discharged and its question reframed.
- [ ] *(owner ruling — filed as **AUDIT `2026-08-06-01`, P1**)* A third of the
      campaign's verdicts carry no evidence of their own: 4 151 of 11 862 have
      as their entire evidence a blob shared with other verdicts, and one of
      them was measurably false while the campaign's own per-fact pass on the
      same claim said so. Three questions are put and none is answered here.

### What the next session picks up, in order

1. The remaining live backlog rows that are **not** owner-parked: **B-019**
   (b: a command node in the map; c: the error-variant node, whose systems
   boundary the owner asked to be decided before implementation), **B-032**
   (the planning-granularity protocol), **B-046** (multi-language composition —
   three options, the owner's pick), and **B-047**'s remaining two items now
   that the census exists.
2. `AUDIT` `-04` (one ruling about an unfinished TUI subsystem, not 57
   judgements) and `-14` (the index's wire contract, owner-court).

---

## Previous slice: волна Г — CLOSED WHOLE 2026-08-05

Ordered by the owner's ruling of 2026-08-05: **the gate holes first, then
registry hygiene, then B-056, then волна Г whole.** Every item is done. Волны
А, Б and В closed whole (2026-08-04/05); Г closed 2026-08-05, so **all four
waves of `TOOLING-MAP.md` §4 are closed** and what remains there is
`##WAVE-PARKED`, which is outside the waves by construction.

Two of Г's four closed by correcting a claim rather than by building what the
line asked for, and that is worth carrying forward: F-132 asked for tags in a
file that does not exist, and B-040's last landing was declined on a
measurement that the reading itself produced.

### The two gate holes — closed first, because everything built after them is built under them

- [x] `feat(conform)`: the discipline engine runs over its own package
      sources (B-057) — a policy and a ratchet baseline per live slot, seven
      panel runs off one binary, and the mcp slots' authored-crate
      denominator derived from `sync-engines.toml` rather than spelled.
- [x] `fix(specmap)`: a declared `[[external_specs]]` root that is not on
      disk announces itself instead of resolving twelve citations into
      nothing (B-058 half 2). One edit in the neutral engine; a warning, not
      a refusal — the resolution layer's «not yet installed» tolerance is
      deliberate and stays.
- [x] `feat(check)`: the installed copies get a freshness signal (B-058
      half 1) — a `local-source-freshness` cell over the lockfile's own
      source hashes. No new panel step: the panel already runs `vibe check`.
- [x] `docs(backlog)`: B-059 filed (conform's exclusions match a different
      path than the one conform prints); B-057 and B-058 closed with what
      the build actually measured.
- [x] `chore(vibedeps)`: rematerialise after the package edits — the very
      reinstall the new signal asked for.

### Registry hygiene — CLOSED WHOLE 2026-08-05

The record said five files. Measured: **28** — 20 stale (1214 verdicts between
them) and 8 never judged. The instrument built for it reduced 1214 flagged
verdicts to **19 that had actually moved**.

- [x] `feat(campaign)`: `tasks/text-stability.py` — which judged facts moved,
      instead of re-reading everything. Two blind spots found and fixed the
      same day (list facts, then numbered ones), and every seal re-verified
      after each fix.
- [x] `docs(campaign)`: the evidence sweep, delegated by the
      `WORLD-WORKER-BRIEF` split — workers gather rows stamped `PENDING`,
      which the merger refuses; the boss writes every verdict.
- [x] `chore(campaign)`: merged and sealed, never chained. **272 files, 0
      stale, 0 unjudged**; six drifts found, all documents that outlived their
      subject.

### B-056 — multiple inheritance of contract documents, and the plugin form

Four owner rulings closed the SHAPE on 2026-08-04. The build design is
authored and judged: [`spec/design/multiple-sources-and-plugins.md`](spec/design/multiple-sources-and-plugins.md).
**This is the next build.** Four landings, each standing alone:

- [x] `docs(design)`: the build design over the four rulings — measured basis,
      the section rule for a sequence, the recursion law that already exists,
      and the cut below.
- [x] `feat(vibe-spec)`: `fold_sources(contract, &[sources])` — the fold takes
      a sequence; `fold_source` stayed as its degenerate case, and every
      existing fold test passed through the new path unchanged, which is what
      the kept name was for.
- [x] `fix(vibe-spec)`: the pipeline passes every `#source` in declaration
      order and names the source that fails to resolve rather than the seed.
      **Closed [B-055](BACKLOG.md#b-055).**
- [x] `feat(vibe-spec)`: the cycle law reached `#source` through the SAME
      three-colour walker (one `visit`, one colour map, one `is_contract`),
      and the fold became recursive under it — **with an include guard the
      design had not foreseen**: node dedup is not text dedup, and a diamond
      duplicated the shared source until the guard landed.
- [x] `feat(vibe-spec)`: resolver enumeration for the glob, sorted by
      (name, slot) so the result never depends on directory read order; then
      the glob wired through to the fold, with **one** function computing a
      document's `#source` edges for both the guard and the fold.
- [x] `fix(vibe-spec)`: two sources DEFINING the same section the contract
      never declared no longer pass silently. The gate could not be the
      catcher — it tolerates a repeated heading by design and holds no
      provenance by then — so the check sits in the fold, per level, as a
      fallback after the fact gate. The fold machinery and the collision
      tests moved to their own files: the 600-line budget is a neutral key
      and counts every file, tests included.

### Registry debt this slice created — CLOSED 2026-08-05

- [x] `chore(campaign)`: **19 verdicts over two files** — 10 re-judged in
      [the B-056 design](spec/design/multiple-sources-and-plugins.md) and 9
      judged fresh (2 design corrections, 7 in PROP-035 §7.3). Both sealed;
      `text-stability.py` reports 0 stale, 0 owed.
      **The debt statement was wrong twice, and both corrections are the
      lesson.** *(i)* It counted 13 new facts; only 8 could ever enter the
      registry — the transport law lives under `campaigns/`, a structural
      exclusion in the scanner, and `BACKLOG.md` matches no include glob, so
      five of the named facts are in files the campaign cannot observe.
      *(ii)* It said all ten moved for one reason, the `@spec/plan` →
      `@impl/done` flip. Nine did; the tenth
      (`##fold-source-only-collision`) lost a whole sentence — the one the
      build refuted — and its prior verdict's evidence named a mechanism that
      does not do the job. A re-judgement that had trusted the summary would
      have re-stamped it.
- [x] `docs(vibe-spec)` + `docs(backlog)`: the refuted sentence's **other two
      homes** — `merge.rs`'s module header and append loop, and B-056's
      `##B056-ODR-PARALLEL`. Four homes, of which the landing had corrected
      two, and only two of the four are inside the corpus at all.

### Волна Г proper

- [x] `docs(specmap)`: **the F-132 schema debt, closed honestly.** The debt
      named a file that does not exist; the real defect was one clause of a
      normative rule. PROP-014 §2.3's exclusion half is real, and its «the
      generator input is the taggable unit instead» half is a decision nobody
      executed and nothing can execute: zero of seven schemas carry an
      address, every scanner compares the extension literally against `rs` or
      `md`, and the edge model hangs an address off a code SYMBOL, which a
      JSON document has none of. The cheap fix stayed a wish; the claim, both
      config twins and the verdict were corrected instead, and
      [B-060](BACKLOG.md#b-060) carries the route with the honest reason its
      line estimate does not converge.
- [x] `chore(campaign)`: that fact had been judged `confirmed` on evidence for
      **one of its two clauses** — both refs addressed the exclusion, the
      designation clause had none. A sentence carrying two independent claims
      needs a ref per claim.
- [x] `docs(design)`: **the B-040 build design**
      ([`spec/design/typed-seams.md`](spec/design/typed-seams.md)), shaped by
      a question that crosses the census's five categories — where does the
      tree state an obligation on a caller or an implementor, in prose, with
      nothing checking it. Two of its own claims were refuted while writing
      it and both are recorded: `progress-core` cannot adopt `vibe-core`'s
      `ContentHash` (the separability law forbids it), and `serde(transparent)`
      is not forced by the reason its docblock gives.
- [x] `docs(vibe-settings)`: **L5 — the file-watch seam is a shape.**
      `Watcher` has no production implementation, its docblock said the host
      carries one, and its `implements` edge makes the map report the REQ as
      built — coverage claimed by the shape rather than delivered by it
      ([B-061](BACKLOG.md#b-061)).
- [x] `refactor(vibe-publish)`: **L1 — `ValidatedOrg`.** The forgotten
      `validate_scope` is now a compile error, because the side-effecting
      methods take an argument only that check can mint. Two things the design
      did not ask for came out of the build: the mint is now **once** per path
      where the orchestrator and redirect-create each ran the check twice, and
      the new table test asserts what the type cannot — that an adapter
      *claiming* a scope really enforces it, since a future override could
      satisfy every signature while the guard disappears.
- [x] `refactor(vibe-core)`: **L2 — validation at the wire boundary.** Four
      newtypes adopt `Group`'s spelling. **Five values in the tree were not
      hashes** — one lockfile fixture and four `sha256:x` in
      `vibe-workspace`'s freshness tests — all fixed as values, no grammar
      widened. `From<String> for ContentHash` had to go (the blanket `TryFrom`
      makes an unchecked `From` conflict with a checked `TryFrom`), which
      removes an unchecked constructor from the public API for free.
- [x] `refactor(vibe-actions)`: **L3 — the builder's three obligations moved
      into the signature** — name and description to `Action::builder`,
      `invoke` to `build`. Three `ActionBuildError` variants became compile
      errors; `EmptyPresentation` stayed, because an empty `&'static str` is a
      valid one. **`action.rs` went 600 → 565 lines** — the refactor bought
      budget back instead of spending it. The packet's own count was wrong and
      the worker corrected it before editing: 15 chains inside `vibe-actions`,
      **2 in `vibe-cli`**, reported with addresses rather than reached for.
- [x] `fix(progress-core)`: **L4 — declined, and the reading that declined it
      paid.** The comparison the digest newtype was justified by takes
      `processed_hash` out of the campaign record as untyped JSON, so a newtype
      on the other side cannot type-check it *at all* — zero yield at the one
      site carrying the argument, against ~60 sites in `progress-core` and 29
      in `vibe-cli`. Recorded as a decision. The same reading found what the
      site *did* owe: an absent `processed_hash` read as a match, so a record
      with verdicts and no note of what they were judged against projected as
      **fresh**. Five lines, plus a test that keeps a missing hash and a
      missing date separately reportable.
- [x] `docs(map)` + `docs(backlog)`: **волна Г closed whole.** B-005 and B-010
      were already built — and B-010's row still read `open` a day after the
      commit that closes it verbatim, B-011's `planned` while wave А is closed
      whole. Both corrected against the tree. Five stale statements in
      `BACKLOG.md` in one day is the measurement [B-062](BACKLOG.md#b-062)
      needed and lacked when it was filed.
- [x] `chore(campaign)`: `typed-seams.md`'s **35 facts judged and sealed** —
      against built landings, which is what the deferral was for, and it paid.
      Gathered evidence came back 21 SUPPORTS / 11 PARTIAL / 3 NO-CODE, and
      the eleven were three different things: five describe the pre-landing
      basis (tense, not error), three carry numbers this session's own builds
      moved, two prescribe the landing that was later declined, and one clause
      was simply wrong. Ten facts still said `@spec/plan` about work landed
      hours earlier — **the same defect this slice had just criticised in
      B-010's disposition, in a document written the same day by the same
      hand.** Corrected first, then judged: registry 0 stale, 0 owed.

### The Phase E exit gate — measured 2026-08-05, and it needs one ruling

The plan's gate is «task queue drained or explicitly deferred; floor green;
`report --view todo` matches the deferrals file exactly». Two of three are met:
the four waves are closed and the floor is green (bare panel, tail read).

The third now has a number instead of a guess: **273 files, 267 `done`, 6
`work`.** The six, classified rather than lumped:

- **Three designs of closed waves** — `map-format-change.md` (волна В),
  `new-rule-classes.md` (волна Б батч 3), `seam-error-and-assertion-parity.md`
  (волна Б батч 2). Their builds landed; the document state did not move with
  them. Same class as B-010's `open`, one level up.
- **Two manual tests** — `MT-02-vibe-tree-tui.md`, `MT-03-vibe-prefs-tui.md`,
  `impl/work` because a manual test is unrun until someone runs it.
- **One draft spec** — `PROP-010-local-package-cache.md`, whose own status says
  «the S5 open questions need an owner design session».

- [ ] *(owner ruling)* Whether the three closed-wave designs move to
      `state="done"`, and whether the two manual tests and PROP-010's draft are
      **deferred by decision** — which is what the gate's own wording asks for
      and what would let Phase E close. Not decided here: «closed whole» for a
      wave and «done» for its design are not obviously the same claim, and
      волны Б and В still carry the map's `@doc/work` while the WAL calls them
      closed. That disagreement is itself the thing to rule on.
- [ ] `chore(campaign)`: **31 facts in `typed-seams.md` await first judging.**
      Deliberately not self-judged in the authoring session — B-056's design
      was, and this slice had to correct one of those verdicts. Judging them
      against the built landings is the stronger reading, so they wait for the
      builds.

### M-PARITY bar 2 — two named builds left, both owner-deferred

- [ ] *(P3, owner-ruled «don't build now, don't drop the promise»)* the Rust
      `dylint` and Go `analysis.Analyzer` custom-lint vehicles — `{#b-050}`.
- [ ] *(deferred, cost measured)* the Rust deviation-reason text — ~33
      frontend sites + a frontend version bump — `{#b-053}`.

---

## Tombstone — what stood here until 2026-08-04

Until this rewrite the file carried the checklist of **Phase A of the
decentralized-registry refactor** (spring 2026): per-package repos,
multi-registry / mirror / override schemas, lockfile v2, the resolver crate,
the publish tool, the live three-package migration to GitHub. That slice
finished; its checklist is in `git log`, its contract in
[PROP-002](spec/modules/vibe-registry/PROP-002-decentralized-registry.md).

Two lines never got ticked, and both were **resolved by evolution rather than
by the commit they named** — recorded here so the absence is not read as debt:

- `test(e2e)`: «update `cli_e2e.rs` against the new fixture layout» — that
  monolith no longer exists. It split into per-surface suites under
  `crates/vibe-cli/tests/` and the fixture helper moved into their shared
  `common` module.
- `docs(commands)`: «`vibe build` / `vibe sync` / `vibe show` / `vibe check`
  reference docs» — `docs/commands/` now holds twenty-odd files including
  `show.md` and `check.md`; `build` and `sync` are not commands this CLI grew.

**Two lines of the retired checklist are cited by `file:line` in the
campaign's frozen evidence** (`tasks/evidence/`, batches W1a and W6d). Those
citations now point at different content, so the lines they quoted are kept
here verbatim rather than left dangling — the evidence is historical and stays
untouched by policy, and this is the route back to what it read:

- was `TASKS.md:19` — `- [x] docs(guides): create DEV-GUIDE.md and
  RUNTIME-GUIDE.md scaffolds at repo root.`
- was `TASKS.md:56` — `- [x] feat(packages-live): migrate three v0.1.0 flows
  to per-package repos in the vibespecs organization on GitHub` (published
  2026-04-29, all three tagged `v0.1.0`).
