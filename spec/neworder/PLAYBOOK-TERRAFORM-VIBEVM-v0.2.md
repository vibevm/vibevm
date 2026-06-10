# PLAYBOOK — Terraforming vibevm, v0.2

**Audience: the agent (Claude Code) working inside the vibevm repository.** This document is your task plan and operating prompt for the terraform effort. It does not replace the repository's own constitution — read order and precedence are in §0. Execute phases strictly in order; a phase is complete only when every acceptance command exits 0.

**v0.2 changelog.** Brownfield revision (see `BROWNFIELD-PROTOCOL-v0.1.md`): Phase −1 is now *inventory, not gate*; the only absolute precondition is compilation; all test gates use non-regression (xfail-strict) semantics via `xtask test-gate`; aspiration harvest, debt registry, conflict scan, and characterization capture added; Phase 6 gains the carry-over reconciliation; §7 added (operating amid debt and dispute).

---

## 0. Boot, precedence, and conduct {#boot}

1. Run the repository's normal session boot first (`CLAUDE.md` → `spec/boot/*` → `spec/WAL.md`). The repo's four rules and session protocols apply to every terraform session.
2. **Precedence:** owner's live instructions > repo constitution (`CLAUDE.md`, `spec/boot/`) on *process* > this playbook on *task content and order* > the discipline package documents (Charter, PROP-014, BROWNFIELD, guides, ENGINE, LEDGER) as *reference*.
3. **Context budget (axiom A2 applies to reading):** do not preload the whole package. Each phase lists what to read; pull other sections only when needed.
4. **Frozen surfaces — never edit without the owner's explicit instruction in the current conversation:** `VIBEVM-SPEC.md`, `spec/boot/00-core.md`, `spec/boot/90-user.md`, `refs/**`. PROP documents are editable through normal PR discipline; `PROP-003` edits in Phase 1 are **additions-only** (anchors and marker lines; no prose rewording).
5. **Stop-and-ask triggers** (halt the phase, write state to WAL, ask): any lockfile schema change; deleting a test or editing `tests-baseline.json` outside the promotion protocol (§7.2); any edit touching a frozen surface; force/history operations; CI changes beyond the jobs specified here; two consecutive failures of the same acceptance gate (→ §8).
6. **Every session ends** with the repo's session-end checkpoint (WAL update; `CONTINUE.md` on wind-down) plus one line in `terraform/LOG.md`: date, phase, commits, gate status.
7. Uncertainty: follow the repo's uncertainty protocol (re-read spec → book → analogs → `<!-- REVIEW -->` + conservative choice). Never silently invent semantics.

## Phase −1 — Inventory: freeze reality {#inventory}

*Read: BROWNFIELD §2–§6.*

The single absolute gate: **`cargo build --workspace` exits 0.** If it does not, fixing compilation is the only permitted work (P1 debt), then restart this phase. Everything else below is *recorded*, not required.

- [ ] Record-only test run: `cargo nextest run --workspace --no-fail-fast` (install if absent; fallback: `cargo test --no-fail-fast` with stdout parsing). Triage every non-passing test into `terraform/registry/tests-baseline.json` (`failing-known` / `flaky` / `obsolete`), each linked to a debt entry.
- [ ] Seed `terraform/registry/debt.json` (+ generated `DEBT.md`): import all open AUDIT.md findings; add failing/flaky tests; add `<!-- REVIEW -->` markers and load-bearing TODO/FIXME (judgment: skip cosmetic ones, record the skip rule used).
- [ ] Aspiration harvest → `terraform/registry/intent.json` (+ `INTENT.md`): WAL "Next"/"Known issues"/"Decisions pending", `TASKS.md`, ROADMAP open milestones, CONTINUE.md next-steps. Every item gets an id, a source pointer, links where obvious.
- [ ] Conflict scan over `spec/**`: heuristic pass (duplicate anchors; MUST/MUST-NOT collisions on shared subject windows) + LLM-proposed semantic conflicts (proposals only). Each finding → `disputed-spec` debt entry with evidence quotes from both units. **Resolve nothing.**
- [ ] Characterization capture: golden transcripts for currently-passing observable flows (fixture e2e + the `manual-tests/` scenarios that pass today), normalized for volatile fields, under `terraform/golden/`.
- [ ] `terraform/BASELINE.md`: commit hash, counts (tests by status, debts by kind/severity, intents, conflicts, golden flows), build times, crate list.
- [ ] Commits: topic-grouped (`docs(terraform): …` / `test(terraform): …`).

**Acceptance:** build exits 0; all five registry/golden artifacts exist; a second inventory run is a no-op diff (determinism); the owner has reviewed and dispositioned at minimum every **P1** debt and every `disputed-spec` entry's *existence* (adjudication itself can wait).

## Phase 0 — Tooling skeleton {#phase0}

*Read: PROP-014 §2.1–2.5; GUIDE-RUST §5; BROWNFIELD §4.*

- [ ] `crates/specmark/`: no-op attribute macros `#[spec(...)]`, `#[verifies(...)]`, `specmark::scope!` — parse-validate the grammar (URI shape, verb set, `r` integer, `reason` required for `deviates`), inject a rustdoc "Spec:" line, expand to the item unchanged.
- [ ] `xtask specmap`: markdown unit parser (anchors, kind/revision/**status** lines, body hashes) + syn-based item/attribute scanner → canonical `specmap.json`; `--check` = regenerate + byte-diff.
- [ ] `xtask test-gate`: nextest-based diff against `tests-baseline.json`; fails on **newly-failing** and on **unexpectedly-passing-unpromoted** (BROWNFIELD §4). This command replaces every bare `cargo test` in later acceptance lines.
- [ ] `xtask tripwire`: given the changed-paths set of the working tree / PR, list debt entries whose tripwires fire. Warn-only.
- [ ] `schemas/specmap.jtd.json` + wire-type codegen.
- [ ] CI: `specmap-check` and `test-gate` jobs, **non-blocking**.

**Acceptance:** `cargo xtask specmap && cargo xtask specmap --check` exits 0 twice on the untouched tree; `cargo xtask test-gate` exits 0 against the freshly recorded baseline; `cargo test -p specmark` green.

## Phase 1 — Pilot: PROP-003 §2.6.1 × `vibe-resolver/src/conditional.rs` {#phase1}

*Read: GUIDE-SPEC-AUTHORING §1–4 (incl. lifecycle statuses); PROP-014 §2.2–2.3.*

- [ ] Additions-only edit to PROP-003 §2.6.1: anchored `req` unit markers for (i) fixed-point monotonicity, (ii) predicate grammar, (iii) host-invariance. Status `ratified` where implemented; `planned` where the text specifies unbuilt behavior (boolean composition). Open as a PR for owner review; do not merge without approval.
- [ ] Tag `conditional.rs`: `implements` on `ConditionalPredicate` and `parse`; `deviates` (+reason) for unimplemented boolean composition referencing the `planned` unit; `#[verifies]` on its tests.
- [ ] **Drift drill** (two demonstration commits, then revert the spec change): (a) semantic edit + `r` bump → `xtask specmap --check` reports the suspect edges; re-affirm; green. (b) typo edit without bump → hash warning fires.
- [ ] `xtask trace explain vibe_resolver::conditional::ConditionalPredicate::parse --text` emits the correct subgraph, including the `planned`/`deviates` relationship.

**Acceptance:** drill behaviors reproduced and captured in the PR description; explain output reviewed by owner; index deterministic; `xtask test-gate` exits 0.

## Phase 2 — Backfill `vibe-resolver` {#phase2}

*Read: PROP-014 §2.7, §4 Phase 2; LEDGER §6 (query kind 4); BROWNFIELD §5, §7.*

- [ ] Mine the latent corpus: `git log --all --pretty='%H %s' --grep='spec://'` → seed `terraform/specmap-proposals.json` (commit → files → URIs, evidence pointers).
- [ ] Crate sweep (bounded sub-task): for every public item in `vibe-resolver`, propose ≤3 edges with one-line evidence quotes from **both** sides and a confidence mark; items without spec-side evidence → `candidate orphan`. Output proposals only; **edit no source file in this step.** Where the only matching unit is `disputed`, mark the proposal `blocked-on-dispute`.
- [ ] Affirmation passes: implement only owner-APPROVED proposals; one commit per module; commit bodies cite the URIs (and debt ids where edges touch disputed/planned units); `cargo xtask specmap --check && cargo xtask test-gate` before each commit.
- [ ] Flip the ratchet: `specmap-check` becomes **blocking for `vibe-resolver` only** (ratchet file lists exempt crates).

**Acceptance:** coverage report ≥ 90% of the crate's **ratified, non-disputed** `req` units both implemented and verified; `planned` and `disputed` scopes reported separately with zero unexplained gaps; orphan list empty or dispositioned into `debt.json`; blocking gate green; `xtask test-gate` exits 0.

## Phase 3 — Cells v0 on the canonical pair {#phase3}

*Read: GUIDE-RUST §1–3, §7; BROWNFIELD §6.*

- [ ] Add `#[cell(...)]` manifests to the solver pair behind `DepSolver` (confirm the concrete pair with the owner: if `SatDepSolver` is not in tree yet, use `NaiveDepSolver` + the next real seam pair).
- [ ] Flag registry module at the composition root (`vibe-cli`): selection data with provenance chain; the **only** module reading selection flags.
- [ ] Differential oracle: property test over fixture graphs asserting pair agreement on solvable inputs (documented-divergence list otherwise), `#[verifies]`-tagged on the governing REQ.
- [ ] Interim lints (`xtask conform-lite`): flag-reads-outside-registry; cell-imports-sibling.

**Acceptance:** `cargo xtask conform-lite` exit 0; oracle test green in CI; characterization unchanged — golden transcripts from `terraform/golden/` reproduce byte-identically (volatile fields normalized), except where a debt/intent record in the PR declares a deliberate change; `xtask test-gate` exits 0.

## Phase 4 — conform engine MVP {#phase4}

*Read: ENGINE-CONFORM (whole — it is short); BROWNFIELD §7.*

- [ ] `conform-core` (fact model, store keyed by file-hash+producer, SARIF emit) + `conform-frontend-rust` (syn). No T-sem frontends in this phase.
- [ ] Port the three checks: flag-sites (R-001), import-graph isolation (R-002), unsafe-gate; retire `conform-lite`.
- [ ] Baseline file `conform-baseline.json` (pre-existing findings frozen; file may only shrink). Determinism test: run twice, byte-diff SARIF.
- [ ] Frontier behavior: findings only within ratchet scope; outside scope, facts are still extracted (B5 — no cliffs).
- [ ] CI: `conform` job non-blocking workspace-wide, **blocking for `vibe-resolver`**.

**Acceptance:** `conform check --baseline conform-baseline.json --scope crates/vibe-resolver` exit 0; determinism test green; incremental rerun after a 1-file touch re-extracts exactly 1 file (assert via producer log); `xtask test-gate` exits 0.

## Phase 5 — Ledger MVP (local only) {#phase5}

*Read: LEDGER §2–6.*

- [ ] `.ledger/` store (facts class wired to conform's extraction; one interpretations kind: `explain.item` prose render behind `xtask trace explain --prose`).
- [ ] Epoch wiring per LEDGER §3; provenance line on every render; telemetry counters (hit rate, cost, rot-rate plumbing).
- [ ] **Not shipped, not signed, not exposed** — local/CI cache only; `.ledger/` in `.gitignore`.

**Acceptance:** second identical `--prose` call is a cache hit (telemetry shows it); editing `Cargo.lock` invalidates the render (epoch test); facts survive the epoch change; `xtask test-gate` exits 0.

## Phase 6 — Expansion, reconciliation, report {#phase6}

*Read: BROWNFIELD §8–§9.*

- [ ] Ratchet order: `vibe-core` → `vibe-install` → `vibe-registry` → remaining crates → `vibe-cli` last; per crate: conform pass + specmap backfill in the same touch (each item handled once); debts whose tripwires fire during a touch are pulled into that crate's scope or explicitly re-dispositioned.
- [ ] Wire suspects/orphans/disputes into the periodic audit as category-C entries (PROP-013 shape); run one full audit.
- [ ] **Aspiration reconciliation:** every `intent.json` item reaches `done | rescoped (→ spec URI or debt id) | rejected (reason)`. Unaccounted count must be **zero**.
- [ ] `terraform/REPORT.md`: metrics vs `BASELINE.md` — coverage by status, wish-ratio of activated rules, conform findings burn-down, tests-baseline shrinkage, debt burn-down slope, disputed adjudication half-life, ledger hit rate, LLM-$ per merged change trend — plus the honest list of what the discipline got wrong (feeds v0.2 of every package document).

**Acceptance (beta exit):** all gates blocking workspace-wide except explicitly exempted crates; `intent unaccounted = 0`; REPORT delivered; owner sign-off.

## 7. Operating amid debt and dispute {#debt-ops}

1. **No drive-by fixes.** A known-failing test outside the phase's declared scope stays failing; pulling it in is an explicit scope change with its debt id in the commit body.
2. **Promotion protocol.** An unexpectedly-passing test is promoted by editing `tests-baseline.json` + closing/annotating its debt in the same commit (`test(terraform): promote <id>`), never silently.
3. **Disputed specs.** Never implement *against* a disputed unit to "settle" it; edges into disputed pairs are frozen (exempt from suspect-clearing), and work that must touch them carries the dispute's debt id. Adjudication is the owner's act, recorded as supersede / scope-split / stay-open per BROWNFIELD §5.
4. **Planned units.** Gaining a first real `implements` edge flips `planned → ratified` in the same PR; coverage math never penalizes labeled plans.
5. **Tripwires are read, not muted.** If `xtask tripwire` flags an entry on your change set, address it in the PR description: pulled-in, re-dispositioned, or consciously deferred (one line each).

## 8. Failure protocol {#failure}

A gate failing twice on honest attempts is **information, not an obstacle to route around**. Halt the phase; write `terraform/FINDINGS.md` (what was attempted, exact failing output, your best hypothesis, which package document is probably wrong); update WAL; ask the owner. Do not weaken a gate, edit a baseline outside §7's protocols, fork an acceptance criterion, or reinterpret a command's exit code to proceed — gaming a gate is the one unforgivable move in this plan, because the gates are the experiment.

---

*This playbook is beta. Where it conflicts with reality, reality wins — through §8, on the record, never silently.*
