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

## Current slice: волна Г — the host catches up with its own discipline

Ordered by the owner's ruling of 2026-08-05: **the gate holes first, then
registry hygiene, then B-056, then волна Г whole.** Волны А, Б and В closed
whole (2026-08-04/05).

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

### Волна Г proper

- [ ] `refactor(crates)`: the pointed seam work the B-040 census earned
      (`harvest/g1-b040-seams-census.md`) — sealed traits and typestate where
      they pay, a recorded reason where they do not.
- [ ] `docs(spec)`: the F-132 schema debt. **Measured 2026-08-05, and the
      debt is not where the line said.** `schemas/specmap.jtd.json` does not
      exist; what exists is seven `*.jtd.json` report schemas, and **none of
      them carries a spec tag of any kind**. That matters because
      `conform.toml` exempts `vibe-wire` on the stated ground that «the
      generator input under `schemas/` is the taggable unit instead» — so
      the exemption's own justification is the thing that is missing.
      **The cheap fix is a wish, not a fix:** each schema has a top-level
      `metadata` block, so a `spec://` key drops in trivially — and nothing
      reads it, because the specmap scanner takes `.rs` and markdown, not
      JSON. Closing this honestly means either teaching the scanner the
      schema metadata, or correcting the claim to «excluded, input NOT
      tagged» with a reason and a route. The owner's standing word on it is
      «сделать как будет возможность».

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
