# `flow:campaign-plans` — cold-executable campaign plans {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-CAMPAIGN-PLAN-FORMAT A vibevm `flow` package that installs the **campaign plan** format
into a project: any multi-commit change too big for one session is
planned as a single document that a fresh session — or a different
person — can execute with no memory of the planning conversation. @status:impl/done

- @fact:FEATURE-FROZEN-BASELINE-ARITHMETIC
  Frozen baseline arithmetic, @status:impl/done
- @fact:FEATURE-FALSIFIABLE-PREDICTIONS falsifiable predictions, @status:impl/done
- @fact:FEATURE-PHASES-GATED-ON-THE-GREEN-FLOOR phases gated on
  the project's green floor, @status:impl/done
- @fact:FEATURE-AN-EXECUTION-LEDGER-OF-HASH-LEVEL-EVIDENCE an execution ledger of hash-level
  evidence, @status:impl/done
- @fact:FEATURE-A-DEFERRALS-LEDGER-THAT-SEEDS-THE-NEXT-MANDATE and a deferrals ledger that seeds the next campaign's
  mandate. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PLAN-FORMAT `spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md` — what a
  campaign is, the five artifact roles (PLAN / BASELINE /
  PREDICTIONS / LOG / REPORT), and the canonical fifteen-section
  plan skeleton, each section with a spec and most with a worked
  mini example. @status:impl/done
- @fact:CONTENT-THE-PHASE-GATES `spec/flows/campaign-plans/phase-gates.md` — Phase 0 spike
  discipline (no commits; a red spike rewrites the affected decision
  before anything lands), phase anatomy, the safe-stop law,
  resumability, review points, discovered-necessary work. @status:impl/done
- @fact:CONTENT-THE-EXECUTION-LEDGER `spec/flows/campaign-plans/execution-ledger.md` — the record half:
  status-line lifecycle, the prepended execution record, per-phase
  commit maps, honesty rules, the closing report, the deferrals
  ledger and the lineage law. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/40-flow-campaign-plans.md` — boot snippet loaded at
  session start: when to propose a campaign, the phase-boundary
  checklist, and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:campaign-plans
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:campaign-plans
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Canonical home {#canonical-home}

@fact:THIS-PACKAGE-IS-THE-CANONICAL-HOME-OF-THE-FORMAT This package is the canonical home of the campaign-plan format. @status:impl/done

@fact:core-ai-native-defers-from-its-next-release The
AI-Native Discipline (`flow:core-ai-native`) ships its own
campaign-form document, `spec/05-CAMPAIGN-FORM.md`, which restates
part of this format; the two are not yet reconciled. @status:spec/done

## Composition {#composition}

- @fact:COMPOSES-WAL `flow:wal` — a campaign checkpoint updates the WAL's standing line
  at every phase boundary; the plan file, not the WAL, carries the
  campaign detail. @status:impl/done
- @fact:COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — each phase's commit set follows it:
  subjects are spelled in the plan, one idea per commit, and the
  ledger binds hashes to the planned subjects. @status:impl/done
- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — the plan's D-sections are decision
  records inline; rejected options carry reasons so nobody re-opens
  a settled question mid-campaign. @status:impl/done
- @fact:COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol` — review points are the campaign-scale
  REVIEW marker: OPEN with options, then RESOLVED with the owner's
  ruling verbatim. @status:impl/done

## Philosophical background {#background}

@fact:format-crystallized-over-a-dozen-real-campaigns The format crystallized over a dozen real campaigns in the origin
project — a debt drain that took a frozen baseline from 130 findings
to 10, a package-family rename across five packages, a six-wave
subsystem landing whose execution corrected two of the plan's own
decisions and said so in the ledger. @status:spec/done

@fact:each-was-executed-cold-and-the-format-kept-what-survived Each was executed cold, several
across session boundaries, and the format kept what survived contact
with execution. @status:spec/done

@fact:THE-PLAN-IS-A-PROGRAM-AND-THE-LEDGER-IS-ITS-OUTPUT Its spirit is the book's "write for the system, not
for yourself": the plan is a program for whoever runs it next, and
the ledger is that program's output (*AI-native development*, ships
in Russian inside `flow:redbook` at `spec/book/ru/`). @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [LICENSE.md](LICENSE.md). @status:impl/done
