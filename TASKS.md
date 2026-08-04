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

## Current slice: волна В — the map and its consumers

Ordered by [`TOOLING-MAP.md` `##WAVE-V`](TOOLING-MAP.md). Волны А and Б closed
whole (2026-08-04); this is the third.

### Measurement first — the forks stand on numbers

- [ ] `docs(campaign)`: fingerprint noise measured on the real history — raw
      text vs token stream, the number owner fork №3 is decided on
      (`harvest/e15-b019a-fingerprint-noise.md` + the re-runnable
      `tasks/fingerprint-noise.py`).
- [ ] `docs(campaign)`: the two lifecycle vocabularies censused — what carries
      specmap's `planned`/`disputed` today and what consumes it, under fork №7
      (`harvest/e16-b024-lifecycle-vocab-census.md`).
- [ ] `docs(campaign)`: the census the one format change stands on — manifest
      strictness, what travels in a package, the schema-bump route
      (`harvest/e17-map-format-census.md`).

### The one format change — three builds, one schema bump

- [ ] `docs(design)`: the boss design for the format change, standing on the
      three censuses above, with the owner's forks marked where they fall.
- [ ] `feat(core-ai-native)`: schema 2 → 3 — the code item gains its span and
      its fingerprint (B-019а), the map ships inside a package (B-016 half 1),
      the privacy tier reaches the manifest (B-017). **One change, not three.**
- [ ] `chore(packages)`: vendor the format change into the six engine copies
      and rematerialise.

### The consumers the map unlocks

- [ ] `feat(vibe)`: «объясни» over vibe's own agent interface (B-018.1).
- [ ] `feat(vibe)`: map search — the query language v0 is owner fork №6
      (B-018.2).
- [ ] `feat(vibe)`: answers about *installed* packages, fed by the
      package-shipped map (B-018.4) + fragments by fingerprint (B-016.2,
      owner fork №4).
- [ ] `feat(vibe)`: the light client to external LLMs (B-020) + the threshold
      warnings (B-021).

### Decided inside the wave, not deferred out of it

- [ ] `feat(xtask)`: B-014 — the committed host index is regenerated and its
      freshness is gated (or the «regenerate on demand only» posture is
      recorded as a decision). The engine's own doc comment already claims the
      gate; the host panel does not run it.
- [ ] `docs(backlog)`: B-024's ruling written into the entry — the vocabularies
      merge, and `disputed`'s fate is settled (fork №7).

### Волна Г — parallel, opportunistic, never blocking

- [ ] `fix(xtask)`: `mirror --check` tests **ancestry**, not equality — a
      target legitimately behind mainline stops reading as drift (B-005).
- [ ] `fix(vibe)`: a check verb that writes — `progress check` stops rewriting
      a frozen zone's state, or says in its first help line that it does
      (B-010).
- [ ] `refactor(crates)`: the pointed seam work the B-040 census earned
      (`harvest/g1-b040-seams-census.md`) — sealed traits and typestate where
      they pay, a recorded reason where they do not.
- [ ] `docs(spec)`: the F-132 schema debt.

### M-PARITY bar 2 — the four named builds between recorded-honest and built

- [ ] `feat(go-ai-native)`: the Go flag/registry rule (parity row 6).
- [ ] `fix(go-ai-native)`: the Go floor's `./...` scoping — `vet`/`tests`/
      `staticcheck` gain the exclusion `gofmt` already has (rows 8/12,
      B-048's sibling).
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
