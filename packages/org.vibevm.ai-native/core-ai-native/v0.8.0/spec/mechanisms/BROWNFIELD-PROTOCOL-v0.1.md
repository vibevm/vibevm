# BROWNFIELD — Terraforming unfinished projects, v0.1 {#root}

<status stage="spec" state="done"/>

##status-line **Status.** Beta. @impl/done

##CLOSES-THE-HEALTHY-BASELINE-GAP Closes the package's most serious v0.1 gap: the playbook assumed a healthy baseline ("all gates green") on a project that — like every real project mid-flight — has failing tests, unimplemented specs, unfulfilled plans, and contradictory statements. @impl/done

##UNFINISHEDNESS-IS-A-MODELED-STATE This document makes unfinishedness a *modeled state*, not an exception. @impl/done

##mars-was-empty-real-codebases-are-inhabited Mars was empty; real codebases are inhabited — terraforming an inhabited world needs relocation protocols, not bulldozers. @spec/done

##amendments-carried-lead **Amendments carried by this document:** @impl/done

- ##AMENDMENT-CHARTER-GAINS-A6 Charter gains axiom **A6**; @impl/done
- ##AMENDMENT-GUIDE-SPEC-AUTHORING-LIFECYCLE GUIDE-SPEC-AUTHORING gains lifecycle statuses; @impl/done
- ##AMENDMENT-PROP-014-STATUS-AND-CONFLICTS PROP-014 gains unit status + `conflicts_with` edges; @impl/done
- ##AMENDMENT-PLAYBOOK-REVISED-TO-V0-2 the Playbook is revised to v0.2. @impl/done

---

## 1. The problem, stated precisely {#problem}

##on-an-unfinished-project-lead On an unfinished project: @impl/done

- ##problem-a-tests-fail-and-have-failed (a) some tests fail and have failed for a while; @spec/done
- ##problem-b-ratified-units-unimplemented-by-plan (b) some ratified spec units have no implementation *by plan*; @spec/done
- ##problem-c-intentions-must-survive-migration (c) the WAL/TASKS/ROADMAP carry intentions that must survive migration; @spec/done
- ##problem-d-drift-and-mutually-exclusive-statements (d) the spec corpus contains drift and mutually exclusive statements. @spec/done

##demanding-global-health-never-starts A migration discipline that demands global health before starting will never start; one that ignores these states will entrench or lose them silently. @spec/done

##BOTH-FAILURE-MODES-ARE-A1-VIOLATIONS Both failure modes are A1 violations. @impl/done

## 2. Principles {#principles}

- ##PRINCIPLE-B1-INVENTORY-NOT-GATE **B1 — Inventory, not gate.** The only absolute precondition is "the workspace compiles." Everything else is recorded with a status at Phase −1; thereafter every gate means **monotone non-regression against the inventory**, in both directions (see §4, xfail-strict). @impl/done
- ##PRINCIPLE-B2-ASPIRATION-LEGAL-ONLY-WHEN-LABELED **B2 — Aspiration is legal only when labeled.** Unimplemented intent is a first-class tracked object (`planned` spec units, intent records), never ambient knowledge. The migration carries a **carry-over guarantee**: at exit, every harvested intention is `done | rescoped | rejected(reason)` — zero unaccounted. @impl/done
- ##PRINCIPLE-B3-CONTRADICTION-IS-DATA **B3 — Contradiction is data.** Conflicting spec units are recorded (`conflicts_with` + `disputed` status) with evidence; normalization **never resolves conflicts inline**. Adjudication is an explicit owner act. @impl/done
- ##PRINCIPLE-B4-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **B4 — Characterization is the truth-of-record where truth is uncertain.** Golden transcripts of currently-passing observable behavior pin "don't break it" independently of whether tests or specs are trustworthy. @impl/done
- ##PRINCIPLE-B5-MONOTONE-UTILITY **B5 — Monotone utility.** Every tool yields useful output at 0% migration and improves continuously to 100%. No cliffs: queries outside the migrated frontier degrade to best-effort facts with an explicit "outside frontier" mark, never to errors. @impl/done

##principles-mechanize-what-vibevm-does-socially These mechanize what vibevm's own AUDIT.md / PROP-013 already do socially: dated findings, severities, dispositions (`fixed / filed / accepted / open`), carry-forward. @impl/done

##DEBT-REGISTRY-IS-MACHINE-READABLE-AUDIT The debt registry below **is** machine-readable AUDIT. @impl/done

## 3. The registries {#registries}

##registries-are-committed-human-diffable-lead Committed, human-diffable ground truth under `discipline/registry/` (the shipped tools' default path — override by flag; distinct from the intent *ledger*, which is uncommitted derived cache): @impl/done

##REGISTRY-TESTS-BASELINE **`tests-baseline.json`** — exact-match input for the test gate: @impl/done

```json
{ "test": "vibe_registry::git_backend::shell::clone_over_ssh",
  "status": "failing-known",            // passing | failing-known | flaky | obsolete
  "since": "2026-05-23", "debt": "DBT-0007" }
```

##REGISTRY-DEBT-JSON **`debt.json`** (+ generated human view `DEBT.md`) — unified deficiency record: @impl/done

```json
{ "id": "DBT-0007", "kind": "failing-test",   // failing-test | unimplemented-req |
                                              // disputed-spec | orphan-code | stale-doc
  "severity": "P2",                           // PROP-013 scale
  "evidence": ["tests-baseline:…", "spec://org.vibevm.core/vibevm/...#req-...~r1"],
  "disposition": "filed",                     // fixed | filed | accepted | open
  "tripwires": ["touch:crates/vibe-registry/src/git_backend/**", "rev:spec://…#req-…"],
  "sunset": "evidence window 60d — re-disposition at next audit" }
```

##REGISTRY-INTENT-JSON **`intent.json`** (+ `INTENT.md`) — the aspiration inventory, harvested from WAL "Next"/"Known issues", `TASKS.md`, ROADMAP open milestones, `<!-- REVIEW -->` markers, TODO/FIXME: @impl/done

```json
{ "id": "INT-0031", "source": "spec/WAL.md#next 2026-05-23",
  "text": "first full PROP-013 audit run", "links": ["spec://org.vibevm.core/vibevm/common/PROP-013"],
  "state": "open" }                           // open | done | rescoped | rejected
```

##TRIPWIRES-ARE-CHEAP-AND-MECHANICAL Tripwires are cheap and mechanical: a check that warns when a change touches a debt's watched paths or revs a watched unit — debt resurfaces exactly when it becomes relevant, instead of rotting in a file nobody reopens. @impl/done

## 4. The test gate — xfail-strict semantics {#test-gate}

##test-gate-diffs-the-run-against-the-baseline-lead `xtask test-gate` (runner: cargo-nextest, MIT/Apache-2.0; fallback: libtest stdout parsing) diffs the run against `tests-baseline.json` and fails on either of: @impl/done

1. ##GATE-FAILS-ON-NEWLY-FAILING **Newly failing** — a `passing` test failed: regression, fix or revert. @impl/done
2. ##GATE-FAILS-ON-UNEXPECTEDLY-PASSING-UNPROMOTED **Unexpectedly passing, unpromoted** — a `failing-known` test passed: the baseline is stale. Promote it (remove the entry, close/annotate the linked debt) in an explicit commit. Silence here is how baselines become graveyards; the strict mode makes the registry shrink truthfully. @impl/done

##FLAKY-ENTRIES-ARE-QUARANTINED `flaky` entries are quarantined (run, reported, never gating) with a debt record and a sunset — flakiness is debt, not weather. @impl/done

##DRIVE-BY-FIXES-ARE-PROHIBITED Drive-by fixes of known-failing tests outside a phase's scope are prohibited: either pull the debt into scope explicitly or leave it; "while I was here" repairs destroy the experiment's accounting. @impl/done

## 5. Spec lifecycle and the conflict protocol {#spec-lifecycle}

##UNIT-STATUSES-ARE-KIND-LINE-GRAMMAR Unit statuses (kind line grammar, see GUIDE-SPEC-AUTHORING amendment): `req r2` (default: ratified) · `req r1 planned` · `req r2 disputed(#other-anchor)` · retired (tombstone). @impl/done

- ##STATUS-PLANNED `planned`: zero coverage is *expected*; coverage reports count planned scope separately; gaining a first `implements` edge prompts a status flip in the same PR. @impl/done
- ##STATUS-DISPUTED `disputed`: recorded pair with `conflicts_with` edge + a `disputed-spec` debt entry holding the evidence quotes. Detection: a crude heuristic pass (duplicate anchors; MUST/MUST-NOT keyword collisions on a shared subject window) plus LLM-proposed semantic conflicts — proposals only, interpretations class in the ledger. **No inline resolution during normalization** — a silent semantic merge is worse than an honest contradiction. @impl/done
- ##ADJUDICATION-HAS-THREE-OUTCOMES Adjudication (owner act), three outcomes: **supersede** (loser retired with tombstone → winner), **scope-split** (both refined with explicit applicability contexts), **stay open** (rare; the dispute itself becomes load-bearing documentation). @impl/done
- ##WHILE-DISPUTED-EDGES-ARE-FROZEN While disputed: edges into the pair are **frozen** — exempt from suspect-clearing and from coverage penalties; implementations carry the dispute's debt id in commit bodies. Presumption (not resolution): the more specific, more recently revised unit is *presumed* current for read purposes, displayed with the presumption label. @impl/done

## 6. Characterization of record {#characterization}

##CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME At inventory time, capture golden transcripts for currently-passing observable flows (the `manual-tests/` scenarios + fixture-driven e2e): exact CLI output, exit codes, written-file trees, normalized for volatile fields. @impl/done

##CHARACTERIZATION-IS-A-STABILITY-ORACLE-NOT-A-CORRECTNESS-CLAIM These are stability oracles, not correctness claims — they may pin bugs, and that is the point (a pinned bug is a visible debt; an unpinned bug is a landmine). @impl/done

##PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED Phase gates that previously said "behavior unchanged" now mean "characterization snapshots unchanged, except where a debt/intent record says we changed it deliberately." @impl/done

## 7. Frontier and monotone utility {#frontier}

##RATCHET-FILE-IS-THE-FRONTIER The ratchet file is the frontier. @impl/done

##contract-per-tool-lead Contract per tool: @impl/done

- ##CONTRACT-SPECMAP-AND-TRACE `specmap`/`trace` on untagged items → facts + "outside frontier"; @impl/done
- ##CONTRACT-CONFORM `conform` → findings only within scope, baseline frozen elsewhere; @impl/done
- ##CONTRACT-EXPLAIN `explain` → degrades from full chain to best-effort facts, provenance line says which. @impl/done

##A-TOOL-THAT-ERRORS-OUTSIDE-THE-FRONTIER-IS-FORBIDDEN A tool that errors on the unmigrated 90% of the repo would make the discipline hostage to its own completion — B5 forbids it. @impl/done

## 8. Carry-over guarantee and exit accounting {#carry-over}

##PHASE-6-CLOSES-WITH-ASPIRATION-RECONCILIATION Phase 6 (playbook v0.2) closes with **aspiration reconciliation**: every `intent.json` item reaches `done | rescoped (→ new spec URI or debt id) | rejected (reason recorded)`. @impl/done

##REPORT-PUBLISHES-THE-EXIT-NUMBERS The REPORT publishes: debt burn-down slope, disputed half-life, baseline shrinkage, and `intent unaccounted = 0` as a hard exit criterion. @impl/done

##EVERYTHING-PLANNED-LANDS-OR-IS-LET-GO "Everything planned eventually lands or is consciously let go" is thereby a checkable property of the migration, not a hope. @impl/done

## 9. Governance {#governance}

- ##DEBT-ENTRIES-CARRY-SUNSETS Debt entries carry sunsets and are re-dispositioned at audit runs — symmetric with rule sunsets (Charter R-050): debt that nobody re-reads is wish-ratio's evil twin. @impl/done
- ##ANTI-ENTRENCHMENT-CLOSE-QUOTA **Anti-entrenchment escape:** if the debt count flatlines while the frontier advances for two consecutive phases, a per-phase close-quota activates (each subsequent phase must close K debts, K set by the owner). Ratchets guard against regression; quotas guard against the ratchet becoming a museum. *Specified, not built: nothing detects a flatline and nothing activates a quota — no flatline comparison across phases, no per-phase quota counter and no value for K exists in any engine crate, stack CLI or host registry. What ships is the input the rule would read: `debt.json` carries a count and a disposition per entry, and the phase-to-phase debt totals are published by hand in the terraform REPORT. K itself is still an open question in §11.* @spec/done
- ##ACCEPTED-DEBT-BUDGET-PER-CRATE Accepted-debt budget per crate (error-budget idea, SRE lineage — concepts only): exceeding the budget blocks new `accepted` dispositions in that crate until something burns down. @impl/done

## 10. Prior art {#prior-art}

- ##PRIOR-ART-PYTEST-XFAIL-STRICT pytest `xfail(strict=True)` (the unexpectedly-passing signal — idea), @spec/done
- ##PRIOR-ART-BASELINES-AND-RATCHETS lint/violation baselines and ratchet patterns from large-repo practice (idea), @spec/done
- ##PRIOR-ART-SRE-ERROR-BUDGETS SRE error budgets (idea), @spec/done
- ##PRIOR-ART-VIBEVM-AUDIT-AND-PROP-013 vibevm's own AUDIT.md + PROP-013 (direct ancestor — this document is its mechanization), @impl/done
- ##PRIOR-ART-FEATHERS-CHARACTERIZATION-TESTS Feathers' characterization tests (B4's foundation). @spec/done

## 11. Open questions {#open}

1. ##OPEN-CONFLICT-HEURISTIC-PRECISION Conflict-heuristic precision: the MUST/MUST-NOT window match will false-positive; tune on the real corpus, report precision in Phase −1 findings. @spec/done
2. ##OPEN-QUOTA-K-AND-DEBT-BUDGETS Quota K and per-crate debt budgets: numbers from REPORT data, not taste. @spec/done
3. ##OPEN-INTENT-AUTO-LINK-TO-ROADMAP Should `intent.json` auto-link ROADMAP milestone anchors once ROADMAP is unit-ified? (Lean yes; cheap; after Phase 1.) @spec/done
4. ##OPEN-INDEX-ABSORPTION Index absorption: registries stay as files in v0.x for diff-reviewability; folding statuses/conflicts into `specmap.json` as the single store is a PROP-014 v0.2 decision. @spec/done

---

##UNEXERCISED-FIELD-STATUS-OR-POLICY-IS-REMOVED *Any registry field, status, or policy here not exercised by Playbook (v0.2) Phase 2 is either removed or annotated in place as **specified, not built** — never carried as unmarked aspiration; yes, the brownfield document eats its own rule.* @impl/done
