# `flow:licensing` — a deliberate product licence posture {#root}

<status stage="doc" state="done" audience="user"/>

@fact:package-installs-a-licensing-posture-lead A vibevm `flow` package that installs a **licensing posture** into a
project: @status:impl/done

- @fact:POSTURE-CHOOSE-THE-PRODUCTS-OWN-LICENCE-ON-PURPOSE choose the product's own licence on purpose (not whatever a
  scaffold dropped in), @status:impl/done
- @fact:POSTURE-KEEP-EVERY-DEPENDENCY-PERMISSIVE-ONLY keep every third-party dependency
  permissive-only, @status:impl/done
- @fact:POSTURE-START-FROM-AN-HONEST-PLACEHOLDER-EULA and — where the intent is to open the product
  later — start from an honest placeholder EULA that states the
  relicense intent plainly. @status:impl/done

@fact:GUIDANCE-NOT-LEGAL-ADVICE This is guidance, not legal advice; a lawyer signs off on the real
licence. @status:spec/done

@fact:package-contents-lead This package ships three pieces of content, a skill, and a boot
snippet: @status:impl/done

- @fact:CONTENT-THE-LICENSING-PROTOCOL `spec/flows/licensing/LICENSING-PROTOCOL.xml` — the postures, the
  placeholder EULA, the permissive-only dependency rule, the
  third-party carve-out, keeping statements in sync, and why
  relicensing is an owner decision. @status:impl/done
- @fact:CONTENT-THE-EULA-TEMPLATE `spec/flows/licensing/eula-template.xml` — a copy-ready
  proprietary-with-relicense-intent skeleton with clause-by-clause
  commentary and an adaptation table. @status:impl/done
- @fact:CONTENT-THE-DEPENDENCY-LICENCE-DISCIPLINE `spec/flows/licensing/dependency-licenses.xml` — the allow/deny
  table, the pre-adoption check over the full transitive graph, and
  the "weight is not a licence concern" rule. @status:impl/done
- @fact:CONTENT-THE-DRAFT-EULA-SKILL `spec/skills/draft-eula/` — an installable skill that drafts or
  reviews the posture end to end. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/60-flow-licensing.xml` — boot snippet: the two standing
  truths (one stated product licence; permissive-only deps) and the
  never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:licensing
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:licensing
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet and the skill. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## The EULA-to-open path {#eula-to-open}

@fact:the-placeholder-posture-is-a-way-station The placeholder posture is not a dead end — it is a way-station. @status:spec/done

@fact:THE-RELICENSE-INTENT-CLAUSE-STATES-THE-INTENT-TRUTHFULLY Its
relicense-intent clause says, truthfully, that the owner means to
open the product under a named permissive licence at a future,
undecided date. @status:impl/done

@fact:AT-RELICENSE-TIME-THE-PLACEHOLDER-IS-REPLACED-WHOLESALE When that date comes, the placeholder is replaced
wholesale by the target licence's official text and every manifest
field moves with it, in one recorded commit. @status:impl/done

@fact:this-collection-itself-walked-that-path This collection itself
walked that path: its packages ship under UPL-1.0, the licence the
origin project's placeholder named as its intended destination. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — the licence choice and any relicensing
  are recorded decisions with reasons; an allowed copyleft exception
  is one too. @status:impl/done
- @fact:COMPOSES-SECRETS-HYGIENE `flow:secrets-hygiene` — a sibling one-place policy; both reward a
  mechanical check in CI over a prose promise. @status:spec/done
- @fact:COMPOSES-HEALTH-AUDIT `flow:health-audit` — a periodic audit line re-runs the dependency
  licence listing, catching a dependency that relicensed between
  versions. *Specified, not built: the composition is a good one and the
  sibling does not yet hold up its end. `flow:health-audit`'s
  `spec/flows/health-audit/audit-checklist.md` carries no licence line
  anywhere — its one dependency category, D4 · Dependency staleness, names
  outdated versions and security advisories and nothing about licences, and
  the whole of that package's `spec/` returns no hit for `licen` · `copyleft`
  · `GPL` · `permissive` · `SPDX`. Nor is there a listing for such a line to
  re-run: no `deny.toml`, no `about.toml`, no SBOM. Until the line exists, this
  entry describes an intended composition rather than a running one; the body
  rule is `spec/flows/licensing/dependency-licenses.xml#RE-AUDIT-ON-A-SCHEDULE`,
  demoted on the same evidence.* @status:spec/done
- @fact:COMPOSES-ATTRIBUTION-POLICY `flow:git-attribution-policy` — the two together define how the
  repository presents itself: who authored it, and under what terms
  it may be used. @status:spec/done

## Philosophical background {#background}

@fact:extracted-from-the-origin-projects-licensing-decision Extracted from the origin project's licensing decision: a
source-available proprietary EULA with an explicit intent to
relicense under UPL-1.0, plus the permissive-only dependency
invariant. @status:spec/done

@fact:collections-spirit-is-the-redbook The collection's spirit is the book *AI-native
development*, shipped in Russian inside `flow:redbook` at
`spec/book/ru/`. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

