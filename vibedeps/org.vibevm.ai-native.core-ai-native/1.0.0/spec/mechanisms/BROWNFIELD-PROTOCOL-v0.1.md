# BROWNFIELD — Terraforming unfinished projects, v0.1 {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status.** Beta. @status:impl/done

@fact:CLOSES-THE-HEALTHY-BASELINE-GAP Closes the package's most serious v0.1 gap: the playbook assumed a healthy baseline ("all gates green") on a project that — like every real project mid-flight — has failing tests, unimplemented specs, unfulfilled plans, and contradictory statements. @status:impl/done

@fact:UNFINISHEDNESS-IS-A-MODELED-STATE This document makes unfinishedness a *modeled state*, not an exception. @status:impl/done

@fact:mars-was-empty-real-codebases-are-inhabited Mars was empty; real codebases are inhabited — terraforming an inhabited world needs relocation protocols, not bulldozers. @status:spec/done

@fact:amendments-carried-lead **Amendments carried by this document:** @status:impl/done

- @fact:AMENDMENT-CHARTER-GAINS-A6 Charter gains axiom **A6**; @status:impl/done
- @fact:AMENDMENT-GUIDE-SPEC-AUTHORING-LIFECYCLE GUIDE-SPEC-AUTHORING gains lifecycle statuses; @status:impl/done
- @fact:AMENDMENT-PROP-014-STATUS-AND-CONFLICTS PROP-014 gains unit status + `conflicts_with` edges; @status:impl/done
- @fact:AMENDMENT-PLAYBOOK-REVISED-TO-V0-2 the Playbook is revised to v0.2. @status:impl/done

---

## 1. The problem, stated precisely {#problem}

@fact:on-an-unfinished-project-lead On an unfinished project: @status:impl/done

- @fact:problem-a-tests-fail-and-have-failed (a) some tests fail and have failed for a while; @status:spec/done
- @fact:problem-b-ratified-units-unimplemented-by-plan (b) some ratified spec units have no implementation *by plan*; @status:spec/done
- @fact:problem-c-intentions-must-survive-migration (c) the WAL/TASKS/ROADMAP carry intentions that must survive migration; @status:spec/done
- @fact:problem-d-drift-and-mutually-exclusive-statements (d) the spec corpus contains drift and mutually exclusive statements. @status:spec/done

@fact:demanding-global-health-never-starts A migration discipline that demands global health before starting will never start; one that ignores these states will entrench or lose them silently. @status:spec/done

@fact:BOTH-FAILURE-MODES-ARE-A1-VIOLATIONS Both failure modes are A1 violations. @status:impl/done

## 2. Principles {#principles}

- @fact:PRINCIPLE-B1-INVENTORY-NOT-GATE **B1 — Inventory, not gate.** The only absolute precondition is "the workspace compiles." Everything else is recorded with a status at Phase −1; thereafter every gate means **monotone non-regression against the inventory**, in both directions (see §4, xfail-strict). @status:impl/done
- @fact:PRINCIPLE-B2-ASPIRATION-LEGAL-ONLY-WHEN-LABELED **B2 — Aspiration is legal only when labeled.** Unimplemented intent is a first-class tracked object (`planned` spec units, intent records), never ambient knowledge. The migration carries a **carry-over guarantee**: at exit, every harvested intention is `done | rescoped | rejected(reason)` — zero unaccounted. @status:impl/done
- @fact:PRINCIPLE-B3-CONTRADICTION-IS-DATA **B3 — Contradiction is data.** Conflicting spec units are recorded (`conflicts_with` + `disputed` status) with evidence; normalization **never resolves conflicts inline**. Adjudication is an explicit owner act. @status:impl/done
- @fact:PRINCIPLE-B4-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **B4 — Characterization is the truth-of-record where truth is uncertain.** Golden transcripts of currently-passing observable behavior pin "don't break it" independently of whether tests or specs are trustworthy. @status:impl/done
- @fact:PRINCIPLE-B5-MONOTONE-UTILITY **B5 — Monotone utility.** Every tool yields useful output at 0% migration and improves continuously to 100%. No cliffs: queries outside the migrated frontier degrade to best-effort facts with an explicit "outside frontier" mark, never to errors. @status:impl/done

@fact:principles-mechanize-what-vibevm-does-socially These mechanize what vibevm's own AUDIT.md / PROP-013 already do socially: dated findings, severities, dispositions (`fixed / filed / accepted / open`), carry-forward. @status:impl/done

@fact:DEBT-REGISTRY-IS-MACHINE-READABLE-AUDIT The debt registry below **is** machine-readable AUDIT. @status:impl/done

## 3. The registries {#registries}

@fact:registries-are-committed-human-diffable-lead Committed, human-diffable ground truth under `discipline/registry/` (the shipped tools' default path — override by flag; distinct from the intent *ledger*, which is uncommitted derived cache): @status:impl/done

@fact:REGISTRY-TESTS-BASELINE **`tests-baseline.json`** — exact-match input for the test gate: @status:impl/done

```json
{ "test": "vibe_registry::git_backend::shell::clone_over_ssh",
  "status": "failing-known",            // passing | failing-known | flaky | obsolete
  "since": "2026-05-23", "debt": "DBT-0007" }
```

@fact:REGISTRY-DEBT-JSON **`debt.json`** (+ generated human view `DEBT.md`) — unified deficiency record: @status:impl/done

```json
{ "id": "DBT-0007", "kind": "failing-test",   // failing-test | unimplemented-req |
                                              // disputed-spec | orphan-code | stale-doc
  "severity": "P2",                           // PROP-013 scale
  "evidence": ["tests-baseline:…", "spec://org.vibevm.core/vibevm/...#req-...~r1"],
  "disposition": "filed",                     // fixed | filed | accepted | open
  "tripwires": ["touch:crates/vibe-registry/src/git_backend/**", "rev:spec://…#req-…"],
  "sunset": "evidence window 60d — re-disposition at next audit" }
```

@fact:REGISTRY-INTENT-JSON **`intent.json`** (+ `INTENT.md`) — the aspiration inventory, harvested from WAL "Next"/"Known issues", `TASKS.md`, ROADMAP open milestones, `<!-- REVIEW -->` markers, TODO/FIXME: @status:impl/done

```json
{ "id": "INT-0031", "source": "spec/WAL.md#next 2026-05-23",
  "text": "first full PROP-013 audit run", "links": ["spec://org.vibevm.core/vibevm/common/PROP-013"],
  "state": "open" }                           // open | done | rescoped | rejected
```

@fact:TRIPWIRES-ARE-CHEAP-AND-MECHANICAL Tripwires are cheap and mechanical: a check that warns when a change touches a debt's watched paths or revs a watched unit — debt resurfaces exactly when it becomes relevant, instead of rotting in a file nobody reopens. @status:impl/done

## 4. The test gate — xfail-strict semantics {#test-gate}

@fact:test-gate-diffs-the-run-against-the-baseline-lead `xtask test-gate` (runner: cargo-nextest, MIT/Apache-2.0; fallback: libtest stdout parsing) diffs the run against `tests-baseline.json` and fails on either of: @status:impl/done

1. @fact:GATE-FAILS-ON-NEWLY-FAILING **Newly failing** — a `passing` test failed: regression, fix or revert. @status:impl/done
2. @fact:GATE-FAILS-ON-UNEXPECTEDLY-PASSING-UNPROMOTED **Unexpectedly passing, unpromoted** — a `failing-known` test passed: the baseline is stale. Promote it (remove the entry, close/annotate the linked debt) in an explicit commit. Silence here is how baselines become graveyards; the strict mode makes the registry shrink truthfully. @status:impl/done

@fact:FLAKY-ENTRIES-ARE-QUARANTINED `flaky` entries are quarantined (run, reported, never gating) with a debt record and a sunset — flakiness is debt, not weather. @status:impl/done

@fact:DRIVE-BY-FIXES-ARE-PROHIBITED Drive-by fixes of known-failing tests outside a phase's scope are prohibited: either pull the debt into scope explicitly or leave it; "while I was here" repairs destroy the experiment's accounting. @status:impl/done

## 5. Spec lifecycle and the conflict protocol {#spec-lifecycle}

@fact:UNIT-STATUSES-ARE-KIND-LINE-GRAMMAR Unit statuses (kind line grammar, see GUIDE-SPEC-AUTHORING amendment): `req r2` (default: ratified) · `req r1 planned` · `req r2 disputed(#other-anchor)` · retired (tombstone). @status:impl/done

- @fact:STATUS-PLANNED `planned`: zero coverage is *expected*; coverage reports count planned scope separately; gaining a first `implements` edge prompts a status flip in the same PR. @status:impl/done
- @fact:STATUS-DISPUTED `disputed`: recorded pair with `conflicts_with` edge + a `disputed-spec` debt entry holding the evidence quotes. Detection: a crude heuristic pass (duplicate anchors; MUST/MUST-NOT keyword collisions on a shared subject window) plus LLM-proposed semantic conflicts — proposals only, interpretations class in the ledger. **No inline resolution during normalization** — a silent semantic merge is worse than an honest contradiction. @status:impl/done
- @fact:ADJUDICATION-HAS-THREE-OUTCOMES Adjudication (owner act), three outcomes: **supersede** (loser retired with tombstone → winner), **scope-split** (both refined with explicit applicability contexts), **stay open** (rare; the dispute itself becomes load-bearing documentation). @status:impl/done
- @fact:WHILE-DISPUTED-EDGES-ARE-FROZEN While disputed: edges into the pair are **frozen** — exempt from suspect-clearing and from coverage penalties; implementations carry the dispute's debt id in commit bodies. Presumption (not resolution): the more specific, more recently revised unit is *presumed* current for read purposes, displayed with the presumption label. @status:impl/done

## 6. Characterization of record {#characterization}

@fact:CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME At inventory time, capture golden transcripts for currently-passing observable flows (the `manual-tests/` scenarios + fixture-driven e2e): exact CLI output, exit codes, written-file trees, normalized for volatile fields. @status:impl/done

@fact:CHARACTERIZATION-IS-A-STABILITY-ORACLE-NOT-A-CORRECTNESS-CLAIM These are stability oracles, not correctness claims — they may pin bugs, and that is the point (a pinned bug is a visible debt; an unpinned bug is a landmine). @status:impl/done

@fact:PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED Phase gates that previously said "behavior unchanged" now mean "characterization snapshots unchanged, except where a debt/intent record says we changed it deliberately." @status:impl/done

## 7. Frontier and monotone utility {#frontier}

@fact:RATCHET-FILE-IS-THE-FRONTIER The ratchet file is the frontier. @status:impl/done

@fact:contract-per-tool-lead Contract per tool: @status:impl/done

- @fact:CONTRACT-SPECMAP-AND-TRACE `specmap`/`trace` on untagged items → facts + "outside frontier"; @status:impl/done
- @fact:CONTRACT-CONFORM `conform` → findings only within scope, baseline frozen elsewhere; @status:impl/done
- @fact:CONTRACT-EXPLAIN `explain` → degrades from full chain to best-effort facts, provenance line says which. @status:impl/done

@fact:A-TOOL-THAT-ERRORS-OUTSIDE-THE-FRONTIER-IS-FORBIDDEN A tool that errors on the unmigrated 90% of the repo would make the discipline hostage to its own completion — B5 forbids it. @status:impl/done

## 8. Carry-over guarantee and exit accounting {#carry-over}

@fact:PHASE-6-CLOSES-WITH-ASPIRATION-RECONCILIATION Phase 6 (playbook v0.2) closes with **aspiration reconciliation**: every `intent.json` item reaches `done | rescoped (→ new spec URI or debt id) | rejected (reason recorded)`. @status:impl/done

@fact:REPORT-PUBLISHES-THE-EXIT-NUMBERS The REPORT publishes: debt burn-down slope, disputed half-life, baseline shrinkage, and `intent unaccounted = 0` as a hard exit criterion. @status:impl/done

@fact:EVERYTHING-PLANNED-LANDS-OR-IS-LET-GO "Everything planned eventually lands or is consciously let go" is thereby a checkable property of the migration, not a hope. @status:impl/done

## 9. Governance {#governance}

- @fact:DEBT-ENTRIES-CARRY-SUNSETS Debt entries carry sunsets and are re-dispositioned at audit runs — symmetric with rule sunsets (Charter R-050): debt that nobody re-reads is wish-ratio's evil twin. @status:impl/done
- @fact:ANTI-ENTRENCHMENT-CLOSE-QUOTA **Anti-entrenchment escape:** if the debt count flatlines while the frontier advances for two consecutive phases, a per-phase close-quota activates (each subsequent phase must close K debts, K set by the owner). Ratchets guard against regression; quotas guard against the ratchet becoming a museum. *Specified, not built: nothing detects a flatline and nothing activates a quota — no flatline comparison across phases, no per-phase quota counter and no value for K exists in any engine crate, stack CLI or host registry. What ships is the input the rule would read: `debt.json` carries a count and a disposition per entry, and the phase-to-phase debt totals are published by hand in the terraform REPORT. K itself is still an open question in §11.* @status:spec/done
- @fact:ACCEPTED-DEBT-BUDGET-PER-CRATE Accepted-debt budget per crate (error-budget idea, SRE lineage — concepts only): exceeding the budget blocks new `accepted` dispositions in that crate until something burns down. @status:impl/done

## 10. Prior art {#prior-art}

- @fact:PRIOR-ART-PYTEST-XFAIL-STRICT pytest `xfail(strict=True)` (the unexpectedly-passing signal — idea), @status:spec/done
- @fact:PRIOR-ART-BASELINES-AND-RATCHETS lint/violation baselines and ratchet patterns from large-repo practice (idea), @status:spec/done
- @fact:PRIOR-ART-SRE-ERROR-BUDGETS SRE error budgets (idea), @status:spec/done
- @fact:PRIOR-ART-VIBEVM-AUDIT-AND-PROP-013 vibevm's own AUDIT.md + PROP-013 (direct ancestor — this document is its mechanization), @status:impl/done
- @fact:PRIOR-ART-FEATHERS-CHARACTERIZATION-TESTS Feathers' characterization tests (B4's foundation). @status:spec/done

## 11. Open questions {#open}

1. @fact:OPEN-CONFLICT-HEURISTIC-PRECISION Conflict-heuristic precision: the MUST/MUST-NOT window match will false-positive; tune on the real corpus, report precision in Phase −1 findings. @status:spec/done
2. @fact:OPEN-QUOTA-K-AND-DEBT-BUDGETS Quota K and per-crate debt budgets: numbers from REPORT data, not taste. @status:spec/done
3. @fact:OPEN-INTENT-AUTO-LINK-TO-ROADMAP Should `intent.json` auto-link ROADMAP milestone anchors once ROADMAP is unit-ified? (Lean yes; cheap; after Phase 1.) @status:spec/done
4. @fact:OPEN-INDEX-ABSORPTION Index absorption: registries stay as files in v0.x for diff-reviewability; folding statuses/conflicts into `specmap.json` as the single store is a PROP-014 v0.2 decision. @status:spec/done

---

@fact:UNEXERCISED-FIELD-STATUS-OR-POLICY-IS-REMOVED *Any registry field, status, or policy here not exercised by Playbook (v0.2) Phase 2 is either removed or annotated in place as **specified, not built** — never carried as unmarked aspiration; yes, the brownfield document eats its own rule.* @status:impl/done
