# `flow:licensing` — a deliberate product licence posture

A vibevm `flow` package that installs a **licensing posture** into a
project: choose the product's own licence on purpose (not whatever a
scaffold dropped in), keep every third-party dependency
permissive-only, and — where the intent is to open the product
later — start from an honest placeholder EULA that states the
relicense intent plainly.

This is guidance, not legal advice; a lawyer signs off on the real
licence.

This package ships three pieces of content, a skill, and a boot
snippet:

- `spec/flows/licensing/LICENSING-PROTOCOL.md` — the postures, the
  placeholder EULA, the permissive-only dependency rule, the
  third-party carve-out, keeping statements in sync, and why
  relicensing is an owner decision.
- `spec/flows/licensing/eula-template.md` — a copy-ready
  proprietary-with-relicense-intent skeleton with clause-by-clause
  commentary and an adaptation table.
- `spec/flows/licensing/dependency-licenses.md` — the allow/deny
  table, the pre-adoption check over the full transitive graph, and
  the "weight is not a licence concern" rule.
- `spec/skills/draft-eula/` — an installable skill that drafts or
  reviews the posture end to end.
- `spec/boot/60-flow-licensing.md` — boot snippet: the two standing
  truths (one stated product licence; permissive-only deps) and the
  never-do list.

## Install

```bash
vibe install flow:licensing
```

## Uninstall

```bash
vibe uninstall flow:licensing
```

Uninstalling removes every file the package wrote, including the boot
snippet and the skill. User-owned files are never touched.

## The EULA-to-open path

The placeholder posture is not a dead end — it is a way-station. Its
relicense-intent clause says, truthfully, that the owner means to
open the product under a named permissive licence at a future,
undecided date. When that date comes, the placeholder is replaced
wholesale by the target licence's official text and every manifest
field moves with it, in one recorded commit. This collection itself
walked that path: its packages ship under UPL-1.0, the licence the
origin project's placeholder named as its intended destination.

## Composition

- `flow:decision-records` — the licence choice and any relicensing
  are recorded decisions with reasons; an allowed copyleft exception
  is one too.
- `flow:secrets-hygiene` — a sibling one-place policy; both reward a
  mechanical check in CI over a prose promise.
- `flow:health-audit` — a periodic audit line re-runs the dependency
  licence listing, catching a dependency that relicensed between
  versions.
- `flow:attribution-policy` — the two together define how the
  repository presents itself: who authored it, and under what terms
  it may be used.

## Philosophical background

Extracted from the origin project's licensing decision: a
source-available proprietary EULA with an explicit intent to
relicense under UPL-1.0, plus the permissive-only dependency
invariant. The collection's spirit is the book *AI-native
development*, shipped in Russian inside `flow:redbook` at
`spec/book/ru/`.

## License

UPL-1.0. See `LICENSE.md`.
