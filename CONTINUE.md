# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-11 at session end. The Big Refactoring is
**complete**: every phase of
[`spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md`](spec/neworder/PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md)
executed on 2026-06-10 (−1/0/1/2/3/4/5/6), owner-declared done,
`new` merged to `main` at **`e1da0c4`** (`--no-ff`, 181 files,
+19 247 / −389) and pushed. This file carries everything a cold
session needs._

> **`spec/WAL.md` is the canonical living state.** If this snapshot and
> the WAL disagree, the WAL wins. Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

One continuous effort delivered the whole Discipline terraform. On top
of the Phase −1/0/1 base (inventory, tooling skeleton, pilot + drift
drill): the **vibe-resolver item-grain backfill** (54 proposals, all
owner-APPROVED in chat, six per-module affirmation commits, three
honest deviates edges at the seams), the **orphan ratchet** (blocking
`specmap --check` for ten crates; 8 reasoned exemptions; per-symbol
debt dispositions), **cells v0** (`#[cell]` manifests, the vibe-cli
selection registry, a hermetic differential oracle over real `file://`
git repositories), the **conform engine MVP** (facts store with proven
1-file-diff incrementality, three rules, byte-stable SARIF, a
frozen-findings baseline), the **local ledger** (epoch-keyed `--prose`
cache, provenance lines, telemetry), **scope-grade backfill of 98
modules** (orphans 538 → 0), intent reconciliation to **0
unaccounted**, an instrumented category-C audit appended to
`AUDIT.md`, and [`terraform/REPORT.md`](terraform/REPORT.md) — metrics
vs BASELINE plus the eight-item honest list feeding the discipline
package v0.2.

## Where work stands

- **Branch `main` @ `e1da0c4`** (the merge commit), in sync with
  `origin/main`; working tree clean. `new` retained and pushed
  (`8024235`); `m1.17-workspace` retained from the earlier era.
- **No active blocker.** Everything open is owner-gated (next
  section); nothing is mid-flight.
- Gate panel, all green on merged `main`: `cargo xtask specmap
  --check` — 489 spec units / 170 tagged items / 177 edges /
  0 suspects / six known pin-into-unmarked warnings; orphan ratchet —
  0 gated, 6 dispositioned (DBT-0019), 8 exemptions with reasons;
  `cargo xtask conform check` — 0 new (6 frozen unsafe;
  scope `crates/vibe-resolver` — 0 at all); `cargo xtask test-gate` —
  1075 results, 0 failed, 3 skipped, xfail-strict; golden transcripts
  byte-identical; `tools/self-check.sh` all four steps.

## Next steps (all owner-gated; exact entry points)

1. **PROP-010 design session** (INT-0003) — close the five §5 open
   questions of the local package cache; then M1.20 implementation.
2. **SAT solver** (DBT-0011) — the deviates edges on `DepSolver`
   (crates/vibe-resolver/src/lib.rs) and `NaiveDepSolver` (naive.rs)
   point at it; the differential-oracle harness
   (crates/vibe-resolver/tests/differential_oracle.rs) is ready to
   take the naive/sat pair per GUIDE-RUST §7.
3. **`VIBEVM-SPEC.md` unit-ification** (DBT-0019) — the root spec is
   owner-frozen and outside `spec/**`, so mdspec cannot scan it;
   resolving this unblocks vibe-cli's item-grain backfill and retires
   the six ratchet dispositions.
4. **Full PROP-013 audit** (INT-0001) — category C is now machine-fed
   (see the 2026-06-10 AUDIT.md entry); the manual §2.2 breadth sweep
   remains. Floor: once per milestone.
5. **Discipline package v0.2** — REPORT.md's "what the discipline got
   wrong" is the input; `spec/neworder/` edits are the owner's to
   make (the session sanction excluded that tree, and it was
   honoured).
6. Cheap loose ends: AUD-0014 (features.rs doc-string says cycles are
   "rejected" — they terminate silently), AUD-0015 (`ResolvedNode`
   doc cites PROP-008 §2.3 where identity is §2.2) — one-line doc
   fixes, deliberately not smuggled into affirmation commits.

## Non-obvious findings (this session)

- **The ratchet caught what the sweep missed within minutes of being
  flipped** (PRP-0054, `PredicateError`): exclusions must apply to
  *items*, never to *files*.
- **scope! cannot tag the bootstrap pair**: specmark /
  specmark-grammar would need a dependency cycle; tagging them is a
  scanner-level feature. They stay exempt with that reason recorded in
  `specmap-ratchet.json`.
- **538 orphans collapsed in one pass** because module docs in this
  repo already cite their PROP homes — the scope!-grade sweep made
  existing claims machine-checkable rather than inventing edges.
- **Hermetic git fixture recipe**: `git -c init.defaultBranch=main`
  work-repo → tag versions → `git clone --bare` into
  `<orgdir>/org.vibevm.<name>.git`; `package_repo_url` composes plain
  Windows paths fine. No network, real ShellGit.
- **`git merge -F -` does not read stdin** (unlike `commit -F -`) —
  write the message to a file. The first merge attempt failed exactly
  there; the `-F - <<'MSG'` heredoc convention is for commits only.
- **specmap.json indexes untracked files** (walkdir knows no git):
  owner guide-drops (Go, 4× Java, Kotlin this session) enter the
  inventory before they are committed — commit them on sight or
  `--check` drifts. The DBT-0016 watch fires reliably.
- **`#[cell(` in doc-comments produces false cell-module positives**
  for naive text discovery; the conform engine discovers cells from
  attribute *facts* instead (and conform-lite learnt
  attribute-line-only matching before retiring).

## Repository map (delta vs pre-terraform)

```
vibevm/
├── specmap.json                  ← committed traceability index (489/170/177)
├── specmap-ratchet.json          ← orphan gate: exemptions + DBT-0019 dispositions
├── conform-baseline.json         ← frozen conform findings (6 unsafe)
├── schemas/specmap.jtd.json      ← wire contract → vibe-wire::generated::specmap
├── crates/
│   ├── specmark-grammar/ specmark/   ← tag grammar (+#[cell]) and the inert macros
│   ├── specmap-core/                 ← mdspec, rscan, index, explain, ratchet, ledger, testgate, tripwire
│   ├── conform-core/ conform-frontend-rust/  ← the conform engine (Phase 4)
│   ├── vibe-cli/src/registry.rs      ← cell-selection registry (R-001)
│   └── vibe-resolver/                ← item-grain backfilled; cells + oracle
├── xtask/                        ← specmap [--check] / test-gate / tripwire / trace explain [--prose] / conform check
├── terraform/                    ← BASELINE, PHASE1-PILOT, LOG, REPORT, registry/{debt,intent,tests-baseline}, golden/
├── .ledger/                      ← (git-ignored) interpretations cache + telemetry
└── spec/neworder/                ← discipline package v0.2-beta + 6 new language guides (owner-dropped)
```

## Decisions in force (terraform legacy, beyond the four rules)

- Gates are the merge criterion, in run order: `cargo xtask specmap
  --check` (index + orphan ratchet) → `cargo xtask conform check` →
  `cargo xtask test-gate` → `bash tools/self-check.sh`.
- Proposals-then-affirmation for every backfill tag (PROP-014 §2.7);
  `terraform/specmap-proposals.json` records APPROVEs as the audit
  trail.
- xfail-strict test semantics; the baseline shrinks only via the
  promotion protocol. No drive-by fixes.
- Tripwires are read, not muted; owner spec-drops are committed on
  sight (DBT-0016 watch).
- No CI by standing owner decision (INT-0017 rejected with reason);
  every gate is a local command.
- `.ledger/` and `target/conform/` are derived data, never committed.
- Index regeneration accompanies every change that moves units/tags;
  `specmap.json` is committed and `--check` is the gate.

## Recent commit chain (newest first)

```
e1da0c4 Merge branch 'new': the Discipline terraform — complete
8024235 docs(continue): cold-resume checkpoint — terraform complete
fabdae9 docs(wal): terraform COMPLETE checkpoint — all phases, merge recorded
c75775e docs(terraform): Phase 6 close-out — reconciliation, audit, REPORT
a9dc160 feat(spec): scope-grade backfill — the ratchet expands to ten crates
c03d4c8 feat(ledger): the local intent ledger MVP (LEDGER-INTENT v0.1)
4d7e32c feat(xtask): conform check gate; conform-lite retired
ea59ef3 feat(conform): the conform engine MVP (ENGINE-CONFORM v0.1)
5494db9 docs(spec): Kotlin guide joins the discipline package
3595ffb feat(xtask): conform-lite — the Phase 3 interim structure lints
d8e3420 test(resolver): differential oracle over the DepProvider pair
ef91162 feat(resolver): cell manifests on the canonical cells
0b387f9 feat(cli): the cell-selection registry (R-001)
bc9f9f0 feat(specmark): the #[cell] manifest attribute
58cbfb4 docs(spec): four Java guides join the discipline package
203f472 feat(specmap): the orphan ratchet — blocking for vibe-resolver
41c18ea feat(resolver): close the PredicateError orphan (PRP-0054)
18c5090 feat(resolver): affirm the approved multi-provider edges (PRP-0051..0053)
d274b04 feat(resolver): affirm the approved local-provider edge (PRP-0050)
52861c1 feat(resolver): affirm the approved activation.rs edges (PRP-0039..0049)
5fad8a7 feat(resolver): affirm the approved features.rs edges (PRP-0024..0038)
99795e4 feat(resolver): affirm the approved naive.rs edges (PRP-0010..0023)
e57411e feat(resolver): affirm the approved lib.rs edges (PRP-0001..0009)
4332f03 docs(terraform): record the owner APPROVE on all 53 proposals
5279835 docs(terraform): Phase 2 proposals - vibe-resolver crate sweep
5bcebb2 docs(spec): Go guide joins the discipline package
```

## Quick-start

```sh
cargo xtask specmap --check     # index + orphan ratchet
cargo xtask conform check       # facts → rules → SARIF → baseline
cargo xtask test-gate           # nextest, xfail-strict
bash tools/self-check.sh        # fmt, tests, clippy -D warnings, vibe check
cargo xtask trace explain <symbol|uri> [--text|--json|--prose]
cargo xtask tripwire            # debt watches over the change set
```

Session-resume phrase: `восстанови сессию`. The WAL supersedes this
snapshot wherever they diverge.
