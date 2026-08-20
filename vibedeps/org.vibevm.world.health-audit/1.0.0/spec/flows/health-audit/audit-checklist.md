# The audit category checklist {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This is the checklist an audit run walks
breadth-first: four category groups (A–D), every sub-item spelled out
with *what to look for*, a *generic mechanical aid* (a grep, a coverage
tool, a config diff), and *what "bad" looks like*. @status:impl/done

@fact:THE-CHECKLIST-IS-LIVING The checklist is
**living** — the closing section is the law for extending it. @status:impl/done

@fact:companion-document-pointers The
run procedure that consumes this list:
[`running-an-audit.md`](running-an-audit.md); the rationale for the
whole practice: [`HEALTH-AUDIT-PROTOCOL.md`](HEALTH-AUDIT-PROTOCOL.md). @status:impl/done

@fact:TRANSLATE-EVERY-AID-INTO-YOUR-STACKS-EQUIVALENT Translate every "aid" into your stack's equivalent: @status:impl/done

- @fact:AID-THE-COVERAGE-TOOL "the coverage
  tool" is whatever your language ships, @status:impl/done
- @fact:AID-GREP "grep" is any repo-wide search, @status:impl/done
- @fact:AID-THE-CI-CONFIG "the CI config" is your pipeline file. @status:impl/done

## A — Test integrity {#a-test-integrity}

@fact:THE-GATE-PROVES-COVERED-CODE-BEHAVES The gate proves covered code behaves. @status:impl/done

@fact:GROUP-A-AUDITS-WHETHER-THE-COVERAGE-IS-REAL This group audits whether the
coverage is real and whether the tests assert the *right* thing. @status:impl/done

### A1 · Coverage gaps {#a1}

- @fact:A1-LOOK-FOR **Look for.** Production paths exercised only through a proxy, or not
  at all — especially default/happy paths everyone assumes is tested
  because the feature "works". A branch with no direct test is a branch
  that can break green. @status:impl/done
- @fact:A1-AID **Aid.** The coverage tool, read by *file and branch*, not just the
  headline percentage. Cross-check: does the critical path have a test
  that would fail if the path broke? @status:impl/done
- @fact:A1-BAD **Bad.** The initializer's default-config path has no direct test;
  the end-to-end suite drives a stand-in fixture and never touches the
  real code path it is supposed to certify. @status:impl/done

### A2 · Quarantined tests {#a2}

- @fact:A2-LOOK-FOR **Look for.** Tests that are skipped, ignored, disabled, or
  commented out. Are they red? stale? has the quarantine become
  permanent and forgotten? @status:impl/done
- @fact:A2-AID **Aid.** `grep` for your framework's skip markers (`@skip`,
  `#[ignore]`, `xit`, `it.skip`, `@Disabled`, `-` in a plan file); run
  the suite once *with* skips enabled and read what fails. @status:impl/done
- @fact:A2-BAD **Bad.** A test disabled "temporarily" two milestones ago, still
  red, now load-bearing coverage that everyone forgot is off. @status:impl/done

### A3 · Tests that encode the wrong behavior {#a3}

- @fact:A3-LOOK-FOR **Look for.** A test that asserts *current* output rather than
  *intended* behavior. It stays green while the behavior is a defect —
  detectable only by reading the assertion against the spec or intent. @status:impl/done
- @fact:A3-AID **Aid.** None mechanical — this is pure judgment. Read the highest-
  value assertions against what the feature is *supposed* to do, not
  against what it currently emits. @status:impl/done
- @fact:A3-BAD **Bad.** A test named `test_init_default` asserts the initializer
  writes a value that is itself broken; the test guards the bug. @status:impl/done

## B — Rot outside the gate {#b-rot}

@fact:THE-GATE-ONLY-CERTIFIES-WHAT-IT-RUNS The gate only certifies what it runs. @status:impl/done

@fact:GROUP-B-AUDITS-WHAT-THE-GATE-DOES-NOT-RUN This group audits everything it
does not run, and whether its reach is shrinking. @status:impl/done

### B1 · Unreached trees {#b1}

- @fact:B1-LOOK-FOR **Look for.** Anything the test command does not execute: a separate
  workspace or sub-project, standalone scripts, fixtures/examples/docs
  samples that no test parses, a second language's directory with its
  own untriggered suite. @status:impl/done
- @fact:B1-AID **Aid.** Diff *what exists on disk* against *what the gate names*.
  List every top-level tree; for each, name the CI job that touches
  it. A tree no job names is unreached. @status:impl/done
- @fact:B1-BAD **Bad.** A `fixtures/` or `examples/` directory carrying retired
  schema across two milestones; a helper sub-project whose own tests
  went red months ago and nobody saw. @status:impl/done

### B2 · Gate completeness {#b2}

- @fact:B2-LOOK-FOR **Look for.** Does the gate still cover every module and every
  target? A new module, a disabled test target, or a moved file can
  quietly carve a hole in the coverage the gate is assumed to have. @status:impl/done
- @fact:B2-AID **Aid.** Diff the CI config / test manifest against the current
  module list. Confirm the test command's glob still matches every
  package it should. @status:impl/done
- @fact:B2-BAD **Bad.** A newly added module is absent from the CI matrix; a
  `test = false` or an excluded path silently drops a package from the
  run while everything stays green. @status:impl/done

## C — Drift {#c-drift}

@fact:NOTHING-IN-GROUP-C-FAILS-A-TEST Nothing here fails a test. @status:impl/done

@fact:GROUP-C-IS-CODE-AND-ITS-DESCRIPTIONS-OUT-OF-STEP Everything here is code and its descriptions
falling out of step. @status:impl/done

### C1 · Doc drift {#c1}

- @fact:C1-LOOK-FOR **Look for.** User-facing docs, READMEs, and help text versus the
  code's actual behavior — flags that no longer exist, examples that no
  longer run, described defaults that changed. @status:impl/done
- @fact:C1-AID **Aid.** Run the doc's own examples. `grep` docs for command/flag
  names and confirm each still exists in the code. @status:impl/done
- @fact:C1-BAD **Bad.** The README's quick-start invokes a renamed command; a
  documented default contradicts the shipped one. @status:impl/done

### C2 · Spec drift {#c2}

- @fact:C2-LOOK-FOR **Look for.** A spec/design doc self-contradicting or contradicting
  another; the spec versus shipped reality; dead cross-references to
  files or anchors that no longer exist. @status:impl/done
- @fact:C2-AID **Aid.** `grep` for internal reference links / anchors and resolve
  each. Read paired documents for contradicting numbers (a version, a
  limit, a threshold stated twice). @status:impl/done
- @fact:C2-BAD **Bad.** One section says schema v4 while another says v5; a doc
  references a directory that is not on disk. @status:impl/done

### C3 · Checkpoint drift {#c3}

- @fact:C3-LOOK-FOR **Look for.** Does the living checkpoint (WAL / `CONTINUE.md` /
  status file) match the tree, the branch, and the commit chain? A
  stale checkpoint mis-orients the next session. @status:impl/done
- @fact:C3-AID **Aid.** Read the checkpoint's "current phase / next / known issues"
  against `git log --oneline` and the actual working tree. @status:impl/done
- @fact:C3-BAD **Bad.** The checkpoint claims a phase is in progress that shipped
  three commits ago; its "known issues" list omits the audit's open
  findings. @status:impl/done

### C4 · Outward drift {#c4}

- @fact:C4-LOOK-FOR **Look for.** External state the project depends on or publishes to —
  live registries, deployed config, downstream consumers — versus what
  the code now expects. @status:impl/done
- @fact:C4-AID **Aid.** Reconcile the code's assumptions against the real external
  surface: query the live endpoint, diff deployed config against the
  repo's copy. @status:impl/done
- @fact:C4-BAD **Bad.** The tool expects an org/namespace layout the live registry
  no longer matches; a downstream consumer still reads a field the code
  stopped writing. @status:impl/done

## D — Debt accumulation {#d-debt}

@fact:GROUP-D-IS-SLOW-CHEAP-AND-COLLECTIVELY-CORROSIVE Slow, individually cheap, collectively corrosive. @status:spec/done

@fact:GROUP-D-COUNTS-THE-DEBT This group counts it. @status:impl/done

### D1 · Deferred & parked items {#d1}

- @fact:D1-LOOK-FOR **Look for.** Every "deferred" / "parked" / "known issue" entry in
  the checkpoint and design docs: still valid? still wanted? silently
  overtaken by later work and never closed? @status:impl/done
- @fact:D1-AID **Aid.** `grep` for "deferred", "parked", "TODO(later)", "known
  issue"; walk each entry and re-judge it against current reality. @status:impl/done
- @fact:D1-BAD **Bad.** A parked item resolved by unrelated work months ago still
  sits open, padding the debt list and hiding the live ones. @status:impl/done

### D2 · Aging markers {#d2}

- @fact:D2-LOOK-FOR **Look for.** In-code `TODO`, `FIXME`, `HACK`, `XXX`, `REVIEW`
  markers. How old? still true? attached to code that shipped anyway? @status:impl/done
- @fact:D2-AID **Aid.** `grep -rn` the marker set; for the oldest, `git blame` the
  line to date it. A marker older than a milestone is a decision, not a
  note. @status:impl/done
- @fact:D2-BAD **Bad.** A `FIXME: temporary` from a year ago on a code path now in
  production; a `TODO` describing a design the code diverged from. @status:impl/done

### D3 · Escape hatches {#d3}

- @fact:D3-LOOK-FOR **Look for.** Suppressions of the gate's own checks — lint-disable
  comments, `#[allow(...)]`, `// eslint-disable`, `@SuppressWarnings`,
  `# type: ignore`, `--no-verify` habits. Each justified once, never
  revisited. @status:impl/done
- @fact:D3-AID **Aid.** `grep -rn` for the suppression syntaxes; for each, ask
  whether the reason still holds or the code can now be fixed properly. @status:impl/done
- @fact:D3-BAD **Bad.** A blanket suppression at file scope that now hides a real
  warning added long after the original justification. @status:impl/done

### D4 · Dependency staleness {#d4}

- @fact:D4-LOOK-FOR **Look for.** Outdated dependencies and open security advisories.
  Pinned versions drifting behind; a transitive advisory nobody saw. @status:impl/done
- @fact:D4-AID **Aid.** The dependency manager's outdated/audit command
  (`npm outdated` / `npm audit`, `cargo outdated` / `cargo audit`,
  `pip list --outdated`, `go list -u -m all`, etc.). @status:impl/done
- @fact:D4-BAD **Bad.** A dependency two majors behind with a known CVE; a lockfile
  that has not been refreshed since before the last advisory batch. @status:impl/done

## Extending the checklist {#extending}

@fact:THE-LIST-IS-A-STARTING-SET-NOT-A-CLOSED-ONE This list is a starting set, not a closed one. @status:impl/done

@fact:TWO-RULES-GOVERN-THE-LISTS-GROWTH Two rules govern its
growth, each applying when its own condition holds: @status:impl/done

1. @fact:GROWTH-RULE-A-NEW-DEFECT-CLASS-BECOMES-A-PERMANENT-ROW **A new defect class becomes a permanent row.** When a run finds a
   kind of rot no category named, add it here as a standing sub-item,
   so the same gap is never re-missed. @status:impl/done
2. @fact:GROWTH-RULE-A-MECHANISABLE-ROW-MIGRATES-INTO-THE-GATE **A mechanisable row migrates into the gate.** When a category can
   be fully checked by a script, move it out of this manual list and
   into the linter / test suite / CI — the audit is the judgment-heavy
   superset that keeps feeding the automated subset. @status:impl/done

@fact:ONE-MORE-ROW-DEPTH-OF-ADOPTION One row every maturing project eventually needs: **depth of adoption.** @status:impl/done

@fact:ADD-A-CATEGORY-MEASURING-DEPTH-OF-ADOPTION If the project claims to follow a convention — a spec discipline, a
naming law, a commit format — add a category that measures *how deep*
the adoption actually goes (how many modules truly carry it), not
merely that it is documented. @status:impl/done

@fact:adopted-is-true-at-the-surface-first "Adopted" is true at the surface long
before it is true throughout; only a depth row catches the gap. @status:spec/done

## Summary {#summary}

- @fact:SUM-FOUR-GROUPS-WALKED-BREADTH-FIRST Four groups: A test integrity, B rot outside the gate, C drift,
  D debt. Walk all four breadth-first. @status:impl/done
- @fact:SUM-EACH-ROW-NAMES-LOOK-FOR-AID-AND-BAD Each row names what to look for, a mechanical aid, and what "bad"
  looks like — translate every aid into your stack's equivalent. @status:impl/done
- @fact:SUM-A3-HAS-NO-MECHANICAL-AID A3 (tests that encode the wrong behavior) has no mechanical aid; it
  is pure judgment and often the highest-value finding. @status:spec/done
- @fact:SUM-THE-CHECKLIST-GROWS The checklist grows: new defect classes join it, mechanisable rows
  leave for the gate, and a maturing project adds a depth-of-adoption
  row. @status:impl/done
