# BROWNFIELD — Terraforming unfinished projects, v0.1

**Status.** Beta. Closes the package's most serious v0.1 gap: the playbook assumed a healthy baseline ("all gates green") on a project that — like every real project mid-flight — has failing tests, unimplemented specs, unfulfilled plans, and contradictory statements. This document makes unfinishedness a *modeled state*, not an exception. Mars was empty; real codebases are inhabited — terraforming an inhabited world needs relocation protocols, not bulldozers.

**Amendments carried by this document:** Charter gains axiom **A6**; GUIDE-SPEC-AUTHORING gains lifecycle statuses; PROP-014 gains unit status + `conflicts_with` edges; the Playbook is revised to v0.2.

---

## 1. The problem, stated precisely {#problem}

On an unfinished project: (a) some tests fail and have failed for a while; (b) some ratified spec units have no implementation *by plan*; (c) the WAL/TASKS/ROADMAP carry intentions that must survive migration; (d) the spec corpus contains drift and mutually exclusive statements. A migration discipline that demands global health before starting will never start; one that ignores these states will entrench or lose them silently. Both failure modes are A1 violations.

## 2. Principles {#principles}

- **B1 — Inventory, not gate.** The only absolute precondition is "the workspace compiles." Everything else is recorded with a status at Phase −1; thereafter every gate means **monotone non-regression against the inventory**, in both directions (see §4, xfail-strict).
- **B2 — Aspiration is legal only when labeled.** Unimplemented intent is a first-class tracked object (`planned` spec units, intent records), never ambient knowledge. The migration carries a **carry-over guarantee**: at exit, every harvested intention is `done | rescoped | rejected(reason)` — zero unaccounted.
- **B3 — Contradiction is data.** Conflicting spec units are recorded (`conflicts_with` + `disputed` status) with evidence; normalization **never resolves conflicts inline**. Adjudication is an explicit owner act.
- **B4 — Characterization is the truth-of-record where truth is uncertain.** Golden transcripts of currently-passing observable behavior pin "don't break it" independently of whether tests or specs are trustworthy.
- **B5 — Monotone utility.** Every tool yields useful output at 0% migration and improves continuously to 100%. No cliffs: queries outside the migrated frontier degrade to best-effort facts with an explicit "outside frontier" mark, never to errors.

These mechanize what vibevm's own AUDIT.md / PROP-013 already do socially: dated findings, severities, dispositions (`fixed / filed / accepted / open`), carry-forward. The debt registry below **is** machine-readable AUDIT.

## 3. The registries {#registries}

Committed, human-diffable ground truth under `terraform/registry/` (distinct from the intent *ledger*, which is uncommitted derived cache):

**`tests-baseline.json`** — exact-match input for the test gate:

```json
{ "test": "vibe_registry::git_backend::shell::clone_over_ssh",
  "status": "failing-known",            // passing | failing-known | flaky | obsolete
  "since": "2026-05-23", "debt": "DBT-0007" }
```

**`debt.json`** (+ generated human view `DEBT.md`) — unified deficiency record:

```json
{ "id": "DBT-0007", "kind": "failing-test",   // failing-test | unimplemented-req |
                                              // disputed-spec | orphan-code | stale-doc
  "severity": "P2",                           // PROP-013 scale
  "evidence": ["tests-baseline:…", "spec://vibevm/...#req-...~r1"],
  "disposition": "filed",                     // fixed | filed | accepted | open
  "tripwires": ["touch:crates/vibe-registry/src/git_backend/**", "rev:spec://…#req-…"],
  "sunset": "evidence window 60d — re-disposition at next audit" }
```

**`intent.json`** (+ `INTENT.md`) — the aspiration inventory, harvested from WAL "Next"/"Known issues", `TASKS.md`, ROADMAP open milestones, `<!-- REVIEW -->` markers, TODO/FIXME:

```json
{ "id": "INT-0031", "source": "spec/WAL.md#next 2026-05-23",
  "text": "first full PROP-013 audit run", "links": ["spec://vibevm/common/PROP-013"],
  "state": "open" }                           // open | done | rescoped | rejected
```

Tripwires are cheap and mechanical: a check that warns when a change touches a debt's watched paths or revs a watched unit — debt resurfaces exactly when it becomes relevant, instead of rotting in a file nobody reopens.

## 4. The test gate — xfail-strict semantics {#test-gate}

`xtask test-gate` (runner: cargo-nextest, MIT/Apache-2.0; fallback: libtest stdout parsing) diffs the run against `tests-baseline.json` and fails on either of:

1. **Newly failing** — a `passing` test failed: regression, fix or revert.
2. **Unexpectedly passing, unpromoted** — a `failing-known` test passed: the baseline is stale. Promote it (remove the entry, close/annotate the linked debt) in an explicit commit. Silence here is how baselines become graveyards; the strict mode makes the registry shrink truthfully.

`flaky` entries are quarantined (run, reported, never gating) with a debt record and a sunset — flakiness is debt, not weather. Drive-by fixes of known-failing tests outside a phase's scope are prohibited: either pull the debt into scope explicitly or leave it; "while I was here" repairs destroy the experiment's accounting.

## 5. Spec lifecycle and the conflict protocol {#spec-lifecycle}

Unit statuses (kind line grammar, see GUIDE-SPEC-AUTHORING amendment): `req r2` (default: ratified) · `req r1 planned` · `req r2 disputed(#other-anchor)` · retired (tombstone).

- `planned`: zero coverage is *expected*; coverage reports count planned scope separately; gaining a first `implements` edge prompts a status flip in the same PR.
- `disputed`: recorded pair with `conflicts_with` edge + a `disputed-spec` debt entry holding the evidence quotes. Detection: a crude heuristic pass (duplicate anchors; MUST/MUST-NOT keyword collisions on a shared subject window) plus LLM-proposed semantic conflicts — proposals only, interpretations class in the ledger. **No inline resolution during normalization** — a silent semantic merge is worse than an honest contradiction.
- Adjudication (owner act), three outcomes: **supersede** (loser retired with tombstone → winner), **scope-split** (both refined with explicit applicability contexts), **stay open** (rare; the dispute itself becomes load-bearing documentation).
- While disputed: edges into the pair are **frozen** — exempt from suspect-clearing and from coverage penalties; implementations carry the dispute's debt id in commit bodies. Presumption (not resolution): the more specific, more recently revised unit is *presumed* current for read purposes, displayed with the presumption label.

## 6. Characterization of record {#characterization}

At inventory time, capture golden transcripts for currently-passing observable flows (the `manual-tests/` scenarios + fixture-driven e2e): exact CLI output, exit codes, written-file trees, normalized for volatile fields. These are stability oracles, not correctness claims — they may pin bugs, and that is the point (a pinned bug is a visible debt; an unpinned bug is a landmine). Phase gates that previously said "behavior unchanged" now mean "characterization snapshots unchanged, except where a debt/intent record says we changed it deliberately."

## 7. Frontier and monotone utility {#frontier}

The ratchet file is the frontier. Contract per tool: `specmap`/`trace` on untagged items → facts + "outside frontier"; `conform` → findings only within scope, baseline frozen elsewhere; `explain` → degrades from full chain to best-effort facts, provenance line says which. A tool that errors on the unmigrated 90% of the repo would make the discipline hostage to its own completion — B5 forbids it.

## 8. Carry-over guarantee and exit accounting {#carry-over}

Phase 6 (playbook v0.2) closes with **aspiration reconciliation**: every `intent.json` item reaches `done | rescoped (→ new spec URI or debt id) | rejected (reason recorded)`. The REPORT publishes: debt burn-down slope, disputed half-life, baseline shrinkage, and `intent unaccounted = 0` as a hard exit criterion. "Everything planned eventually lands or is consciously let go" is thereby a checkable property of the migration, not a hope.

## 9. Governance {#governance}

- Debt entries carry sunsets and are re-dispositioned at audit runs — symmetric with rule sunsets (Charter R-050): debt that nobody re-reads is wish-ratio's evil twin.
- **Anti-entrenchment escape:** if the debt count flatlines while the frontier advances for two consecutive phases, a per-phase close-quota activates (each subsequent phase must close K debts, K set by the owner). Ratchets guard against regression; quotas guard against the ratchet becoming a museum.
- Accepted-debt budget per crate (error-budget idea, SRE lineage — concepts only): exceeding the budget blocks new `accepted` dispositions in that crate until something burns down.

## 10. Prior art {#prior-art}

pytest `xfail(strict=True)` (the unexpectedly-passing signal — idea), lint/violation baselines and ratchet patterns from large-repo practice (idea), SRE error budgets (idea), vibevm's own AUDIT.md + PROP-013 (direct ancestor — this document is its mechanization), Feathers' characterization tests (B4's foundation).

## 11. Open questions {#open}

1. Conflict-heuristic precision: the MUST/MUST-NOT window match will false-positive; tune on the real corpus, report precision in Phase −1 findings.
2. Quota K and per-crate debt budgets: numbers from REPORT data, not taste.
3. Should `intent.json` auto-link ROADMAP milestone anchors once ROADMAP is unit-ified? (Lean yes; cheap; after Phase 1.)
4. Index absorption: registries stay as files in v0.x for diff-reviewability; folding statuses/conflicts into `specmap.json` as the single store is a PROP-014 v0.2 decision.

---

*Any registry field, status, or policy here not exercised by Playbook (v0.2) Phase 2 is removed rather than carried as aspiration — yes, the brownfield document eats its own rule.*
