# `flow:health-audit` — the periodic health audit {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-THE-PERIODIC-HEALTH-AUDIT A vibevm `flow` package that installs the **periodic health audit**
into a project: a recurring, judgment-heavy sweep over everything the
per-commit gate is structurally blind to — uncovered code, out-of-gate
trees, drift, and slow debt — recorded as an append-only trend in
`AUDIT.md`. @impl/done

##THE-GATE-AND-THE-AUDIT-ANSWER-DIFFERENT-QUESTIONS Where the gate answers *"did this commit regress covered
code?"*, the audit answers *"what is wrong, rotting, or drifting that
no commit will ever flag?"*. @impl/done

##THE-GATE-IS-THE-FLOOR-THE-AUDIT-IS-WHAT-IT-CANNOT-SEE The gate is the floor; the audit is what the gate cannot see. @impl/done

##A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE A
milestone is never declared done on an un-audited base. @impl/done

## What ships {#ships}

##package-contents-lead This package ships three flow documents, a skill, and a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.md` — what the audit
  is and is not, the four blind spots, `AUDIT.md` as the durable home,
  dispositions and carry-forward, the living-checklist law, the
  "why not" section, and a re-derive prompt. @impl/done
- ##CONTENT-THE-AUDIT-CHECKLIST `spec/flows/health-audit/audit-checklist.md` — the categories walked
  each run (A test integrity, B rot outside the gate, C drift, D debt),
  every sub-item with what to look for, a mechanical aid, and what
  "bad" looks like. @impl/done
- ##CONTENT-RUNNING-AN-AUDIT `spec/flows/health-audit/running-an-audit.md` — the seven-step run,
  the `AUDIT.md` section format, and a worked example on an invented
  generic project. @impl/done
- ##CONTENT-THE-SKILL `spec/skills/health-audit/SKILL.md` — the `health-audit` skill: an
  agent walks the checklist and drafts the `AUDIT.md` section for
  approval. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/42-flow-health-audit.md` — boot snippet loaded at session
  start: the one-line law, the cadence, and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:health-audit
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:health-audit
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##AUDIT-MD-IS-PROJECT-STATE-NOT-PACKAGE-STATE `AUDIT.md` is **project state** — the package never creates,
deletes, or overwrites it as part of install or uninstall. @impl/done

## Composition {#composition}

- ##COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — a P1 finding too large to fix in-run is
  *filed*, and often becomes the seed of the next campaign's mandate:
  the audit inventories, the campaign drains. @spec/done
- ##COMPOSES-WAL `flow:wal` — the audit *reconciles* the WAL/checkpoint's known-issues
  list against its findings, but the findings do **not** live there.
  The checkpoint is volatile; `AUDIT.md` is the durable, append-only
  home. @impl/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — an `accepted` disposition is a decision
  record: it carries a why and a revisit trigger, not just a shrug. @impl/done
- ##COMPOSES-ATTRIBUTION-POLICY `flow:attribution-policy` — its periodic-audit line item is exactly
  one row on this checklist: grep the attribution pattern set over
  surfaces added since the last audit. @impl/done

## Philosophical background {#background}

##practice-crystallized-in-the-origin-projects-law The practice crystallized in the origin project's periodic-health-audit
law, written after a milestone shipped green — every commit passing,
hundreds of tests passing — while the initializer scaffolded broken
projects and a test asserted the broken output as correct. @spec/done

##NO-AMOUNT-OF-GATE-CATCHES-A-TEST-THAT-GUARDS-A-BUG No amount of
gate catches a test that guards a bug; only a periodic judgment sweep
does. @spec/done

##collections-spirit-is-the-redbook The collection's spirit is the book *AI-native development*,
which ships in Russian inside `flow:redbook` at `spec/book/ru/`: the
gate proves the machine did not regress; the audit is where human and
agent judgment reads what the machine cannot. @spec/done

## License {#license}

##license-line UPL-1.0. See [LICENSE.md](LICENSE.md). @impl/done
