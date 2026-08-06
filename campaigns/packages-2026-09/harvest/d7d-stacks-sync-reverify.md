# D7d — re-verifying the `sync-from-code` verdicts across the three language stacks

_Phase D, wave 7, batch D7d. Sixteen obligations over 45 unrouted anchors in
`rust-ai-native-lang/v0.7.0`, `go-ai-native-lang/v0.1.0` and
`typescript-ai-native-lang/v0.6.0`. Every one closes through
[`sync-from-code`](../PHASE-D-BATCH-PLAN.md#routes), where **the owner approves
every spec diff** — so a re-verdict that edits nothing needs no approval and a
correction does. This file is evidence and a recommendation; **no package file
was edited, and no verdict JSON was written.** The verdict itself is the boss's._

**Measured at** `HEAD = 9f79acf1` (`fix(campaign): the last two boss-closable
obligations, and neither one moved a package`, 2026-07-29). Working tree clean
apart from one untracked sibling harvest file. Every count below names the
command that produced it, per wave 6's lesson that a recorded figure decays.

**Route check, run first per [`##ROUTE-BEFORE-FALSIFIER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).**
All sixteen report `closure_route: sync-from-code`. Enumerated with the batch
script over `run/state/obligations.json` minus `run/state/routing.json`:
**16 obligations · 45 anchors** — F-140 (10), F-154 (6), F-161 (5), F-166 (4),
F-167 (4), F-185 (3), F-210 (2), F-215 (2), F-216 (2), F-270, F-273, F-275,
F-279, F-280, F-281, F-284 (1 each).

**The standing perimeter.** Unless an entry narrows it, every search was run
from the repository root over: `packages/**` **including
`packages/org.vibevm.fractality/**`** (the second complete project that adopted
this discipline — [§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)),
`vibedeps/**`, `crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`,
`terraform/**`, `research/**` (including the three bootstrapped consumers
`research/rust-demo`, `research/ts-demo`, `research/go-demo` with their own
`vibedeps/`, `conform.toml`, `specmap.json` and `discipline/`), `campaigns/**`,
`legacy-spec/**`, `fixtures/**`, `schemas/**`, `docs/**`, `manual-tests/**` and
the repository root's own `*.md` / `*.toml` / `*.json` / `*.sh` / `*.ps1`, minus
`**/target/**`, `.git/**`, `**/node_modules/**` and `campaigns/*/run/**`.
`refs/**` is searched but reported separately — a third-party study corpus, not
our shipped surface.

**The parallel-corpus rule this batch is cut by.** The three `-lang` stacks are
one fact projected per language. Per
[the release queue §B](../PHASE-D-RELEASE-QUEUE.md#stacks), the recurring shape
is *a Go-specific truth stated family-wide*, so **every fact that appears in
more than one stack is judged in all its copies and reported per stack**. Where
a verdict says it was restated to match its siblings, the whole set is
re-verified and never the row —
[§3.7's corollary](../PHASE-D-BATCH-PLAN.md#compliance-blindness).

**One standing PATH fact.** The Go exhaustiveness linter is installed at
`C:\opt\gotools`; `campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md`
carries an annotation saying its «tool did not spawn» lines are a PATH artefact
of the capturing machine. **Any verdict resting on that harvest file is suspect
by default** — F-166 rests on it explicitly.

---

## Owner ruling received mid-batch {#ruling}

_Received 2026-07-31, after the Rust stack had been written and while the Go
entries were being drafted. It changes what may count as evidence at all, so it
is recorded here with the point at which it arrived, and every Go and TypeScript
outcome below was written or re-checked against it._

1. **The `-lang` packages are built first and foremost for EXTERNAL consumers** —
   language support that VibeVM's clients will use in other trees we cannot see.
   **`go-ai-native-lang` and `typescript-ai-native-lang` are NOT adopted by this
   host and must not be**; Go in particular is a deliberately unused prototype
   specification. **This repository is therefore not a valid test bench for
   them.** A verdict that convicts a Go or TypeScript sentence because *this
   repo* does not do, dispatch, install or instantiate the thing **is measuring
   the wrong consumer and is FALSE on that ground alone.**
2. **The legitimate bench for those two is the package's own tree and its own
   tests** — «we can only check our Go packages by tests». Concretely:
   `tools/go-extract/test/fixtures/{clean,dirty}/`,
   `tools/ts-extract/test/fixtures/{clean,dirty}/`,
   `tools/ts-oracle/test/fixtures/proj/`, every `crates/*/tests/*.rs` and in-`src`
   test module, and the package's own cards, guides and `vibe.toml`.
3. **`rust-ai-native-lang` is the exception.** Part of VibeVM itself is written in
   AI-Native Rust, so for the Rust stack the host genuinely is a consumer and
   host evidence counts. It is judged as before — **and its reasoning is not
   carried across to Go or TypeScript**, which is the parallel-corpus trap
   running in the other direction.
4. **Skill-directory evidence is void** for this purpose: `.claude/skills/`,
   `.agents/skills/`, `.opencode/skills/` hold whatever this host's agents use,
   and an absent Go skill says nothing about the Go package.
5. **`legacy-spec/**` is legacy** — excluded from the perimeter and from every
   count below.

**Re-check of the Rust entries written before the ruling arrived.** Points 1–2
bear on three passages, and none of them changes a *Rust* outcome — the Rust
findings all rest on package-own artefacts (each stack's `bench.rs`, its own
tests, its own conform gate, its own `vibe.toml`, the core ATLAS) plus, legitimately
per point 3, the host:

- **F-154 `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS`** — the *TypeScript* note cited
  `research/ts-demo/eslint.config.js`. **That ground is now void.** Re-checked on
  the legitimate bench: `grep -rn "createRule\|ESLintUtils"` over the TypeScript
  package's own tree returns nothing either, so the observation survives on
  package-own evidence — but the TS copy is **not** convicted here and is left
  to a re-judgement, not an edit. The *Rust* outcome (no `dylint`,
  `declare_lint` or `LateLintPass` anywhere, including the Rust package's own
  crates) is unaffected.
- **F-154 `##NAMES-ARE-TOKEN-PROGRAMS`** — the *Go* note rested on
  `research/go-demo` instantiating `{Batch}{Planner}`. Void as a ground, benign
  in direction: it was cited to show the Go sentence is **not** drift, and the Go
  copy carries no verdict here in any case. The Rust conviction stands on
  package-own evidence: `{Variant}{Seam}` appears in no file under
  `packages/org.vibevm.ai-native/rust-ai-native-lang/**`, only in
  `core-ai-native`'s `legacy-projections/`.
- **F-154 `##PROSE-NEAR-CODE-…`** — «no adopter authors a `#[spec(documents)]`
  edge» is legitimate for Rust (point 3). The `ts-demo` and fractality legs of
  that count are corroboration only and are not load-bearing.
- **F-215 `##TARGET-WARM-COMPLETE`** — the primary evidence is each package's
  **own** `bench.rs` having no `complete` field, which is exactly point 2's bench.
  The `research/tcg-bench/reports/` leg is host corroboration for Go and
  TypeScript and is not needed for the finding.

No other appended passage rests on host adoption, host dispatch, host
installation or a host-side instance count.

---

## F-215 — the `complete` target has no instrument in any of the three stacks; the large-workspace warning does exist and the verdict said it did not

**Outcome:** MIXED — 1 SURVIVES (and survives in **all three** stacks, not one) · 1 FALSE PREMISE, DIFFERENT DEFECT
**Anchors:** 2 of 2, both in `rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md`
- `##TARGET-WARM-COMPLETE` (`:249`) → **SURVIVES** — and so do its Go and TypeScript copies
- `##LARGE-WORKSPACE-CONSUMERS-ARE-WARNED` (`:258`) → **FALSE PREMISE, DIFFERENT DEFECT**

**Perimeter searched:** the standing perimeter, for `complete_p50` / `complete_ms` /
`complete_p95` / `completion_ms` / `complete_latency` over `*.rs` `*.ts` `*.go`
`*.json` `*.md` `*.toml`; the three `bench.rs` files read in full; every file in
`research/tcg-bench/reports/`; and — for the second anchor — the three stacks'
sibling agentic briefs `spec/*/tools/vibe-agentic-tcg-*.md`, which is where D5's
F-160 found the Go warning the Go verdict had also missed.

**The verdict's own command, re-run:** the verdict quotes none. It cites
`REPORT-2026-07-07-rust-baseline.md:49-53` and `serve.rs:220`; both were read.

### Anchor 1 — `##TARGET-WARM-COMPLETE` → SURVIVES, in all three stacks

**Per stack — this is the whole point of the entry.** The verdict itself says
«The TypeScript twin was already recorded drift for exactly this … and so was
the Go one», which is the restated-for-consistency signal
[§3.7's corollary](../PHASE-D-BATCH-PLAN.md#compliance-blindness) says to
re-verify as a set. **All three copies are in this batch and all three hold** —
this is the batch's one family where consistency propagated a TRUE premise:

| stack | anchor | posted | measured by |
|---|---|---|---|
| rust | `TCG-ORACLE-RUST-v0.1.md:249` `##TARGET-WARM-COMPLETE` | `complete` p50 < 300 ms, `@impl/done` | **nothing** (this obligation) |
| go | `TCG-ORACLE-GO-v0.1.md:243` `##TARGET-COMPLETE` | `complete` p50 < 300 ms, `@impl/done` | **nothing** (F-167, below) |
| typescript | `TCG-ORACLE-v0.1.md:153` `##TARGET-WARM-VALIDATE-AND-COMPLETE` | warm `validate` p50 < 150 ms **and** `complete` p50 < 200 ms, `@impl/done` | validate only (F-284, below) |

**What the measurement shows.** The word `complete` occurs **zero times** in all
three bench harnesses:

```
$ for s in rust-ai-native-lang/v0.7.0/crates/rust-ai-native-tcg \
           go-ai-native-lang/v0.1.0/crates/go-ai-native-tcg \
           typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-tcg; do
    grep -ci "complete" packages/org.vibevm.ai-native/$s/src/bench.rs; done
0
0
0
```

Each harness times exactly one op — `validate` — and reports only its
distribution:

- rust `bench.rs:248` `.validate(&case.file, …)`, emitting `cold_init_ms`,
  `validate_p50_ms`, `validate_p95_ms` (`bench.rs:350-352`);
- typescript `bench.rs:100` `oracle.validate(…)`, emitting `cold_init_ms`,
  `validate_p50_ms`, `validate_p95_ms` (`bench.rs:132-134`);
- go `bench.rs:139` `.validate(&case.file, …)`, emitting per-case `warm_ms`
  (`bench.rs:224`) and **no percentile of any kind** —
  `grep -n "pct\|percentile\|p50\|p95\|sort" .../go-ai-native-tcg/src/bench.rs`
  returns no output, so the Go harness cannot measure even the neighbouring
  `##TARGET-WARM-VALIDATE` it is nominally the instrument for.

No committed report carries a `complete` row:

```
$ grep -rn "complete" research/tcg-bench/reports/
research/tcg-bench/reports/REPORT-2026-07-07-with-tools.md:7:(`tcg-typescript validate/scope/complete/type`, full artifact path)
```

One hit, and it is a prose list of the tool's verbs, not a measurement. The Rust
baseline's whole summary is
`{"cold_init_ms": 2535, "validate_p50_ms": 0, "validate_p95_ms": 65, "agreement": "9/9"}`
(`research/tcg-bench/reports/bench-rust-2026-07-07-baseline.json`); the
TypeScript baseline's top level is `ts_version`, `cases`, `agreement_pct`,
`cold_init_ms: 561.81`, `validate_p50_ms: 19.32`, `validate_p95_ms: 21.17`
(`research/tcg-bench/reports/bench-2026-07-07-baseline.json`). Over the whole
standing perimeter the only hit for a complete-latency field name is campaign
bookkeeping — `campaigns/packages-2026-09/harvest/d1-rust-ts-lang-repairs.md:608`
*proposes* adding `complete_p50_ms` / `complete_p95_ms`, which is a proposal and
not an instrument.

The verb itself ships in all three (`Cmd::Complete` at
`rust-ai-native-tcg/src/main.rs:58`, `go-ai-native-tcg/src/main.rs:51`,
`typescript-ai-native-tcg/src/main.rs:59`), so this is a **missing instrument
for a real op**, not a target on a phantom verb.

**Proposed correction (NOT APPLIED)** — one shape, three copies; the
`@impl/done` marker is the false part rather than the number:

- rust `TCG-ORACLE-RUST-v0.1.md:249` →

  ```
  - ##TARGET-WARM-COMPLETE `complete` p50 < 300 ms — posted, not yet
    measured: the bench harness times `validate` only
    (`crates/rust-ai-native-tcg/src/bench.rs` emits `cold_init_ms`,
    `validate_p50_ms`, `validate_p95_ms` and no `complete` field). @spec/done
  ```
- go `TCG-ORACLE-GO-v0.1.md:243-244` → the same clause naming
  `crates/go-ai-native-tcg/src/bench.rs`, which records per-case `warm_ms` and
  no percentile at all.
- typescript `TCG-ORACLE-v0.1.md:153-154` → **split the clause; do not demote
  the whole anchor.** The `validate` half is measured and met
  (`validate_p50_ms: 19.32` against a posted `< 150 ms`); only the `complete`
  half is uninstrumented. Suggested: keep the `validate` clause `@impl/done`
  with the measured figure cited, and mark the `complete` clause
  «posted, not yet measured».

### Anchor 2 — `##LARGE-WORKSPACE-CONSUMERS-ARE-WARNED` → FALSE PREMISE, DIFFERENT DEFECT

The verdict reads: *«**no consumer is warned** … The 60 s first-request ceiling
lives only in plan prose at `legacy-spec/terraforms/AGENTIC-TCG-RUST-PLAN-v0.1.md:884`
… Nothing warns anybody, and `legacy-spec/` is an archive no living contract may
rest on.»* **Both halves are false on the shipped package's own surface**, by
the same perimeter miss D5 caught on the Go twin (F-160, catch *(e)*): the
warning is one document over, in the sibling brief, inside this very package.

```
$ sed -n '190,195p' packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md
- ##RISK-COLD-INIT-ON-LARGE-WORKSPACES **Cold init on large workspaces.** 14.7 s cache-cold on a MINIMAL
  crate on this box (sysroot indexing dominates; 2.5 s warm). On big
  consumer trees the first answer may exceed the product's 60 s
  first-request cap — documented, with the eager-init-at-serve-start
  posture and the degraded flag as the mitigations; targets move only
  with a recorded REPORT reason. @spec/done
```

Large-workspace consumers **are** warned, at the spec layer, in a shipped
document of the same package — and the 60 s figure does **not** live only in
`legacy-spec/`; it is at `vibe-agentic-tcg-rust.md:192`. The eager-init half the
verdict already conceded is real (`crates/rust-ai-native-tcg/src/serve.rs:220-223`:
comment «The relay owns the session: boot the analyzer up front so the host's
FIRST frame can be validate/scope/…», then `RustOracle::spawn(&root, QUIESCENCE_BUDGET)`).

**The different defect, and it is the one D5 recorded for Go:** the **60 s number
is supported by nothing in this stack**, and the stack's own two other figures
disagree with it.

```
$ grep -rn "QUIESCENCE_BUDGET" packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-tcg/src/lib.rs
33:pub const QUIESCENCE_BUDGET: std::time::Duration = std::time::Duration::from_secs(45);
```

**45 s shipped** (`lib.rs:33`, used by `spawn_oracle` at `lib.rs:381` and by
`serve` at `serve.rs:223`), **< 15 s posted** one anchor down
(`##TARGET-COLD-INIT-TO-QUIESCENT`, `TCG-ORACLE-RUST-v0.1.md:250`), **60 s
asserted here**. Three numbers, one product — exactly the conflict D5 recorded
for Go, whose shipped constant is `READINESS_BUDGET = 45 s`
(`go-ai-native-tcg/src/lib.rs:32`). The only other `60` in the Rust stack's code
is a test's own spawn budget (`rust-ai-native-tcg-bridge/tests/live_oracle.rs:44`,
`Duration::from_secs(60)`), a test parameter and not a product ceiling.

**Per stack:** rust → the warning exists (`vibe-agentic-tcg-rust.md:190-195`),
the number is unsupported · go → identical sentence, **already demoted by D5's
F-160**, whose clause at `TCG-ORACLE-GO-v0.1.md:260-269` names all three figures
· typescript → **no twin**;
`grep -rn "LARGE-WORKSPACE\|large.workspace\|first.request\|ceiling" packages/…/typescript-ai-native-lang/v0.6.0/spec/`
returns one unrelated hit (`typescript-ai-native-sweep/SKILL.md:18`, «the floor
is the floor, the sweep is the ceiling»). The TypeScript stack's own budget
constant is `ORACLE_TIMEOUT = 30 s` (`typescript-ai-native-tcg/src/lib.rs:29`).

**Proposed correction (NOT APPLIED):** keep the prescription and the *warned*
clause, and repair the **number**, which is the only false part — replace «the
product's 60 s first-request ceiling» with a reference to the tcg brief's
`##RISK-COLD-INIT-ON-LARGE-WORKSPACES` and the shipped 45 s quiescence budget at
`crates/rust-ai-native-tcg/src/lib.rs:33`. The `60 s` → `45 s` substitution is
the classic sync-from-code spec diff and is **for the owner**; it should be taken
in the same approval as the identical Go clause, so the family does not end up
carrying two different ceilings.

**Recommendation per anchor:**
- `##TARGET-WARM-COMPLETE` → **drift stands, correction prepared** (and the same
  correction is owed to the Go and TypeScript copies — F-167, F-284)
- `##LARGE-WORKSPACE-CONSUMERS-ARE-WARNED` → **drift stands, correction prepared
  — but on a different fact than the verdict states.** The verdict's own reason
  must be replaced before the diff is shown, or the owner will be asked to
  approve the demotion of a sentence that is true.

---

## F-280 — «both hops» is one hop in all three stacks, and the TypeScript copy of the same sentence was judged `confirmed`

**Outcome:** SURVIVES — and the parallel-corpus finding runs the *other* way from the batch's usual shape: the defect is family-wide and only **two of the three copies were caught**.
**Anchors:** 1 of 1 —
`rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-PROTOCOL-RUST-v0.1.md#REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS`
**Perimeter searched:** the standing perimeter, narrowed to the three `-lang`
stacks and their three `-mcp` siblings for the *thing* rather than the string —
every `tests/` directory of every crate in all six packages
(`find … -path "*/tests/*" -name "*.rs"`), every in-`src` test module
(`grep -rln "#\[cfg(test)\]"`), a tree-wide hunt for checked-in stream fixtures
(`-name "*.jsonl" -o -name "*transcript*" -o -name "*golden*" -o -name "*.snap" -o -name "*recorded*"`,
minus `target/`), and a call-graph check on the serve entry point.

**The verdict's own command, re-run:** the verdict quotes none. Its two cited
files (`rust-ai-native-tcg-bridge/src/client/tests.rs:1`, `src/oracle/tests.rs:1`)
were read and both say what it says.

**Per stack:** rust → SURVIVES (this obligation) · go → SURVIVES, same sentence,
`TCG-PROTOCOL-GO-v0.1.md:158` (F-210, below) · typescript →
**the same defect, judged `confirmed` at Phase C** —
`TCG-PROTOCOL-v0.1.md:151 ##REPLAY-GOLDENS-PIN-BOTH-SIDES`, not in this batch
and not touched. Recorded below.

**What the measurement shows.**

*The inner hop is real in all three.* Each bridge crate replays its client and
oracle layers over scripted transports, language-server-free by design:

```
$ grep -rn "scripted\|Scripted" --include=*.rs .../{rust,go,typescript}-ai-native-lang/v*/crates/*-tcg-bridge/src/
rust  …/src/client/tests.rs:1  //! Replay tests: the whole client layer against scripted transports —
rust  …/src/oracle/tests.rs:1  //! Oracle-layer replay tests: op semantics over scripted transports —
rust  …/src/lib.rs:5           //! The unit suite is rust-analyzer-free (replay over scripted
go    …/src/client/tests.rs:1  //! Replay tests over a scripted transport — the whole client layer,
go    …/src/client/tests.rs:14 pub(crate) struct Script {
go    …/src/lib.rs:5           //! The unit suite is gopls-free (replay over scripted transports); the
ts    …/src/transport.rs:312   /// A no-node double: scripted responses per op.
```

*The outer hop has no goldens in any of the three, and the check is a call-graph
one rather than a name search.* The outer hop is defined by the document itself
as `host ⇄ <stack>-ai-native-tcg serve`
(`TCG-PROTOCOL-GO-v0.1.md:12-16 ##DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR`), NDJSON
duplex with `{proto, id, op, params}` frames. Its entry point is `run_serve`
(`rust-ai-native-tcg/src/serve.rs:215`, `go-ai-native-tcg/src/serve.rs:227`), and
**nothing but `main.rs` calls it**:

```
$ grep -rn "run_serve\|serve::run" --include=*.rs \
    packages/org.vibevm.ai-native/{rust,go,typescript}-ai-native-lang/v*/crates/ | grep -iE "test"
(no output)

$ grep -rn "ORACLE_PROTOCOL\|\"proto\"" --include=*.rs \
    packages/org.vibevm.ai-native/{rust,go,typescript}-ai-native-lang/v*/crates/ | grep -i test
(no output)
```

No test in any of the three stacks constructs an outer frame at all. The whole
test surface of the three driver crates is:

| stack | driver-crate tests |
|---|---|
| rust | `tests/finding_parity.rs` + `src/lib/tests.rs` (14 unit fns: `derivation_mirrors_the_engine_scanner`, `completions_finalise_prefix_max_and_the_ban`, `seam_resolution_walks_sibling_then_lib`, …) |
| go | `tests/finding_parity.rs` + `src/lib/tests.rs` (7 unit fns: `cell_of_prefers_the_cells_dir…`, `completions_cut_by_prefix_and_max…`, …) |
| typescript | `tests/oracle_e2e.rs` — a **live** test, not a replay: «End-to-end over the REAL chain: SystemOracle → node → LanguageService … Node-dependent by design» (`oracle_e2e.rs:1-5`) |

And there is no checked-in recorded stream anywhere in the three stacks to
replay from:

```
$ find packages/org.vibevm.ai-native/{rust,go,typescript}-ai-native-lang -not -path "*/target/*" \
    \( -name "*.jsonl" -o -name "*transcript*" -o -name "*golden*" -o -name "*.snap" -o -name "*recorded*" \)
(no output)
```

**The one artefact that looks like the missing thing and is not.** All three
`-mcp` siblings ship `tests/server_replay.rs`
(`go-ai-native-mcp/…/tests/server_replay.rs`, `rust-ai-native-mcp/…`,
`typescript-ai-native-mcp/…`). It is a genuine replay over a scripted
transport — but of the **MCP JSON-RPC loop**, a different hop in a different
package: «the whole JSON-RPC loop over the scripted transport — initialize,
tools/list carrying the declared inventory, a malformed line answered without
killing the loop…» (`go-ai-native-mcp/…/tests/server_replay.rs:1-5`). It does not
pin the `{proto, id, op, params}` outer shape this anchor is about, and the
anchor scopes itself to «this package's tests» in any case. Naming it here so
the next pass does not re-discover it and mistake it for the golden.

**Proposed correction (NOT APPLIED):** keep the inner-hop clause verbatim, and
say the outer half honestly. Suggested for
`TCG-PROTOCOL-RUST-v0.1.md#REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS`:

```
##REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS Recorded LSP
transcripts pin the INNER hop in this package's tests (r-a-free in the
unit suite: `crates/rust-ai-native-tcg-bridge/src/client/tests.rs`,
`src/oracle/tests.rs`). The OUTER shape is not yet pinned — no test
drives `run_serve` over recorded frames; `crates/rust-ai-native-tcg`
carries `tests/finding_parity.rs` and unit tests only. @spec/done
```

Go's copy (`TCG-PROTOCOL-GO-v0.1.md:158`) takes the identical clause with
`gopls` for `r-a` and `go-ai-native-tcg-bridge/src/client/tests.rs` for the
inner-hop citation. **The TypeScript copy is not this obligation's to change**
and is recorded as a new obligation below.

**New obligation noticed — the parallel corpus is judged inconsistently, not
just written inconsistently.**
`typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-PROTOCOL-v0.1.md:151`
`##REPLAY-GOLDENS-PIN-BOTH-SIDES` — «Replay goldens on **both sides** (recorded
streams checked into the package tests) pin the CURRENT shape» — is the third
copy of this sentence and was judged **`confirmed`**, on evidence that is
inner-hop-only plus a fact-parity test:

```
"v": "confirmed", "ev": [
  ".../typescript-ai-native-tcg-bridge/src/transport.rs:312  /// A no-node double: scripted responses per op.",
  ".../transport.rs:330  fn typed_ops_shape_their_requests_and_parse_their_results() {",
  ".../tools/ts-oracle/test/oracle.test.ts:381  test(\"fact parity with ts-extract on the same fixture file (D1)\", …)",
  "located in the perimeter on the refs above, machine-verified against the files they cite" ]
```

Both refs are the **inner** side; neither is a golden of the outer shape, and no
«recorded stream» is checked into that package either (the `find` above covers
it). So two stacks carry a drift verdict on this sentence and the third carries a
confirmation, on the same facts. **No obligation covers the TypeScript anchor**,
so no verdict could be re-judged against a change to it and it was not touched —
but it is the clearest instance in this batch of the release queue's warning
running backwards: not a family-wide edit that would break working sentences, but
a family-wide *defect* where one member escaped judgement.

**Recommendation per anchor:** `##REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS`
→ **drift stands, correction prepared** (and the Go copy takes the same
correction under F-210; the TypeScript copy needs a re-judgement, not an edit).

---

## F-275 — the card's BETA self-description is false in Rust and **true in both siblings**

**Outcome:** SURVIVES IN **RUST** ONLY — the Go and TypeScript copies of this sentence are correct and must not be touched.
**Anchors:** 1 of 1 —
`rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md#card-is-beta` (`:9`)
**Perimeter searched:** the standing perimeter for `replacement-has-oracle` /
`ReplacementHasOracle` and `cell-has-oracle` / `CellHasOracle` over `*.rs` and
`*.toml`; the three stacks' `crates/*-ai-native-conform/src/lib.rs` `build_rules`
functions read in full; the three `spec/cards/INDEX.md` rows for this card; the
three sibling `scaffold-d-differential-oracle.md` cards.

**The verdict's own command, re-run:** the verdict quotes none. Its three cited
lines (`scaffold-d-differential-oracle.md:78`, `INDEX.md:13`, `structure.rs:175`)
were read and all three say what it says.

**Per stack:** rust → **SURVIVES** · go → **correct as written** · typescript →
**correct as written**. This is the release queue's shape with the language
swapped: a *Rust*-specific falsehood, where a family-wide edit would break two
working sentences.

**What the measurement shows.** The deciding fact is which rules each stack's
gate actually mounts:

```
$ for st in rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform \
            go-ai-native-lang/v0.1.0/crates/go-ai-native-conform \
            typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-conform; do
    grep -n "out.push(Box::new" packages/org.vibevm.ai-native/$st/src/lib.rs; done

rust : 59 FlagSites · 64 CellIsolation · 65 UnsafeGate · 68 SeamHasDoctest
       71 PubDoctest · 74 ErrorEnumCitesReq · 77 CellHasOracle
       78 ErrorMessageCitesReq · 81 FileLength · 84 NoUnwrapInDomain · 87 AmbientEnv
go   : 53 GoUnsafeInDomain · 57 GoCellIsolation · 59 FileLength
ts   : 50 TsUnsafeInDomain · 52 TsCellIsolation · 57 FileLength
```

**Rust — the sentence is false.** `##card-is-beta` (`:9`, `@impl/done`) reads
«*This card is itself BETA (its checker is specified but **the conform rule is
not yet implemented in the pilot**)*». A conform rule for this card **is**
implemented and mounted: `out.push(Box::new(rules::CellHasOracle));` at
`crates/rust-ai-native-conform/src/lib.rs:77`, defined at
`crates/vendor/core-ai-native-conform/src/rules/structure.rs:175` under the id
`"cell-has-oracle"`. The card's own registry row says so —
`spec/cards/INDEX.md:13`: «**shipped**: oracle-presence (`cell-has-oracle`,
rust-ai-native-conform); the oracle itself stays authored». And the card's own
`##CHECKER` row (`:78`, `@spec/done`) states the true, narrower fact correctly:
the unimplemented rule is `replacement-has-oracle` — which is indeed absent
everywhere:

```
$ grep -rn "replacement-has-oracle\|ReplacementHasOracle" --include=*.rs --include=*.toml \
    packages vibedeps crates research | grep -v "/target/"
(no output)
```

So one document says two different things about itself two rows apart, and the
`@impl/done` one is the wrong one.

**Go — correct as written, and its INDEX row agrees.** `##card-is-beta`
(`go-…/spec/cards/scaffold-d-differential-oracle.md:12`) reads «*This card is
itself BETA (its conform checker is specified; the pilot instance is the
`research/go-demo` fuzz differential).*» It does not claim a shipped rule at
all, and the Go gate mounts three rules, none of them `CellHasOracle`
(`grep -n "CellHasOracle" go-ai-native-conform/src/lib.rs` → no output). Its
INDEX row is consistent: «specified (pilot: `research/go-demo` fuzz
differential)» (`go-…/spec/cards/INDEX.md:19`).

**TypeScript — correct on the conform half.** `##card-is-beta`
(`typescript-…/spec/cards/scaffold-d-differential-oracle.md:11`): «*its checker
is specified but not yet implemented — there is no TypeScript pilot codebase
yet*». The TS gate mounts three rules and none is `CellHasOracle`, and its INDEX
row reads «specified (pilot)» (`typescript-…/spec/cards/INDEX.md:13`).

**Proposed correction (NOT APPLIED)** — Rust only, one line, keeping the BETA
verdict and repairing the *reason* for it:

```
##card-is-beta *This card is itself BETA: its oracle-presence half ships
(`cell-has-oracle`, mounted in `rust-ai-native-conform`), and its
replacement-time checker `replacement-has-oracle` is specified and not yet
implemented — see the ##CHECKER row.* @impl/done
```

`go-…/scaffold-d-differential-oracle.md:12` → **none — correct as written.**
`typescript-…/scaffold-d-differential-oracle.md:11` → **none — correct as
written** on the fact this obligation is about.

**Recommendation per anchor:** `##card-is-beta` (rust) → **drift stands,
correction prepared.** The diff must be scoped to the Rust card; the two sibling
cards are already right and are not part of this closure.

---

## F-279 — the README ships-list points at a schema the package does not carry, under a crate name that was renamed

**Outcome:** SURVIVES — both halves, and the second half is the exact rename artefact D5's F-277 corrected one bullet above.
**Anchors:** 1 of 1 —
`rust-ai-native-lang/v0.7.0/README.md#SHIPS-SPECMAP-WIRE-SCHEMA` (`:41-43`)
**Perimeter searched:** the standing perimeter for `specmap.jtd.json` by name
(`find . -name "specmap.jtd.json"`), for a `schemas/` directory or `*.jtd.json`
inside the package and inside its install slot `vibedeps/stack-rust-ai-native-lang/0.7.0/`,
for a `specmap-core` directory anywhere (`find . -type d -name "specmap-core"`),
the package's `vibe.toml`, and the Go and TypeScript READMEs for the same claim.

**The verdict's own command, re-run:** the verdict quotes none. Its three refs
(`README.md:41`, `schemas/specmap.jtd.json:1`,
`crates/vendor/core-ai-native-specmap/src/generated/mod.rs:1`) were read.

**Per stack:** rust → SURVIVES · go → **no twin**; `README.md` names
`go-ai-native-specmap` as an engine (`:29`) and makes no wire-schema claim ·
typescript → **no twin, and no README at all** —
`ls packages/…/typescript-ai-native-lang/v0.6.0/README.md` →
`No such file or directory`, which independently corroborates the release
queue's F-115.

**What the measurement shows.** The bullet reads: «`schemas/specmap.jtd.json` —
the wire schema of `specmap.json` (the generated types in
`specmap-core/src/generated/` derive from it …). @status:impl/done». Both paths are
wrong, in two different ways.

*(a) The schema is not in the package, nor in what a consumer installs.*

```
$ find . -name "specmap.jtd.json" -not -path "./target/*" -not -path "./.git/*"
./schemas/specmap.jtd.json

$ ls packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/
Cargo.lock  Cargo.toml  LICENSE.md  README.md  crates  spec  specmap.toml  target  vibe.toml

$ ls vibedeps/stack-rust-ai-native-lang/0.7.0/
Cargo.lock  Cargo.toml  LICENSE.md  README.md  crates  spec  specmap.toml  vibe.toml
```

One copy in the whole tree, at the **host root** — no `schemas/` directory in the
package, none in the install slot, and no `schema` key in the package's
`vibe.toml` (`grep -n "schema" …/vibe.toml` → no output). A `##SHIPS-…` bullet
in a package README is a claim about what that package ships; this file is not
in it.

*(b) `specmap-core` is a pre-rename name, exactly as `crates/specmark` was.*

```
$ find . -type d -name "specmap-core" -not -path "./target/*" -not -path "./.git/*"
./.vibe/cache/org.vibevm/core-ai-native/v0.6.0/crates/specmap-core
./.vibe/cache/org.vibevm/discipline-core/v0.4.0/crates/specmap-core
… (10 hits, every one a `.vibe/cache/` copy of a SUPERSEDED package slot)
```

Every `specmap-core` in the tree is a cached copy of an older version. The
crate that ships in this package is `crates/vendor/core-ai-native-specmap`, and
its generated module is at `…/core-ai-native-specmap/src/generated/`. This is the
same `core-ai-native-*` family-stem rename D5's F-277 corrected in the bullet
immediately above — which now reads
«`crates/vendor/core-ai-native-specmark`, taken under the dependency alias
`specmark`» (`README.md:39-42`). The two bullets are adjacent and one of them is
still pre-rename.

*(c) What is true, and the README already half-knows it.* The generated types do
ship, and their own header states where the schema lives:

```
$ head -5 .../crates/vendor/core-ai-native-specmap/src/generated/mod.rs
// Generated by `cargo xtask codegen`. DO NOT EDIT.
//
// Each submodule is generated by `jtd-codegen` from the matching
// `*.jtd.json` schema under `schemas/` at the repo root. Editing
// this file by hand will be overwritten on the next codegen run.
```

«at the repo root» — the *dev* repo's root, which for this package is the host.
The bullet's own parenthetical («regeneration is a maintainer dev-op in the
package's dev repo») concedes the same thing while the lead still says the
package ships the file.

**Proposed correction (NOT APPLIED):** the honest shape is that the package ships
the *generated types*, not the schema. Suggested for `README.md:41-43`:

```
- ##SHIPS-SPECMAP-WIRE-SCHEMA **The `specmap.json` wire types**
  (`crates/vendor/core-ai-native-specmap/src/generated/`) — generated by
  `jtd-codegen` from `schemas/specmap.jtd.json`, which lives in the package's
  dev repo rather than in this package; regeneration is a maintainer dev-op
  there. @impl/done
```

**Recommendation per anchor:** `##SHIPS-SPECMAP-WIRE-SCHEMA` → **drift stands,
correction prepared.** No sibling copy exists, so this is a single-stack diff
with no family consequence.

---

## F-281 — the fourth house-lesson property is asserted nowhere in any stack, and three of the six copies of that claim were judged `confirmed`

**Outcome:** SURVIVES — and the defect is family-wide across six anchors in three stacks, of which only three carry a verdict.
**Anchors:** 1 of 1 —
`rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md#RISK-WINDOWS-CHILD-LIFECYCLE` (`:203-205`)
**Perimeter searched:** the standing perimeter — **including
`packages/org.vibevm.fractality/**`** per
[§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness) — for
the *thing* rather than the string:
`surviving.?pid|no.?zombie|process_table|sysinfo|pgrep|tasklist|OpenProcess|try_wait|GetExitCodeProcess`
over `*.rs` `*.go` `*.ts` `*.sh` `*.ps1`, minus `target/` and `node_modules/`.
Plus the six sibling copies of the claim across the three stacks' mechanism docs
and agentic briefs.

**The verdict's own command, re-run:** the verdict quotes none. Its three refs
(`rust-ai-native-tcg-bridge/src/lib.rs:129`, `client.rs:348`,
`oracle/tests.rs:99`) were read and all three hold.

**Per stack — six copies of one claim, and the judgement is not consistent:**

| stack · document | anchor | verdict at HEAD |
|---|---|---|
| rust · `TCG-ORACLE-RUST-v0.1.md:202` | `##GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY` | drift → **already demoted by D5's F-192** |
| rust · `vibe-agentic-tcg-rust.md:203` | `##RISK-WINDOWS-CHILD-LIFECYCLE` | drift — **this obligation** |
| go · `TCG-ORACLE-GO-v0.1.md:221` | `##GRACEFUL-EXIT-IS-THE-LSP-DANCE` | drift — **F-167, below** |
| go · `vibe-agentic-tcg-go.md:200-202` | `##RISK-WINDOWS-CHILD-LIFECYCLE` | **`confirmed`** |
| typescript · `vibe-agentic-tcg-ts.md:184-186` | `##RISK-WINDOWS-CHILD-LIFECYCLE` | **`confirmed`** |
| typescript · `TCG-ORACLE-v0.1.md:138` | `##SHUTDOWN-IS-THE-ONLY-SANCTIONED-EXIT` | **`confirmed`** |

All six name «no-zombie assertions» among the house lessons that *apply*, and
three of them were confirmed on the same facts that made the other three drift.

**What the measurement shows.** Three of the four named properties hold, exactly
as the verdict concedes: verbatim-free paths into URIs
(`rust-ai-native-tcg-bridge/src/lib.rs:129 pub fn verbatim_free`, asserted at
`src/oracle/tests.rs:99` — «verbatim leaked into a URI: {uri}»), kill-on-drop
(`client.rs:348 let _ = self.child.kill();`), and the shutdown/exit dance. The
fourth does not.

```
$ grep -rniE "surviving.?pid|no.?zombie|process_table|sysinfo|pgrep|tasklist|OpenProcess|try_wait|GetExitCodeProcess" \
    --include=*.rs --include=*.go --include=*.ts --include=*.sh --include=*.ps1 \
    packages vibedeps crates xtask tools discipline terraform research fixtures \
  | grep -v "/target/" | grep -v "/node_modules/"
```

returns hits in exactly three classes, and **none of them is an assertion about
a tcg oracle child**:

1. **Doc comments, not assertions** —
   `go-ai-native-lang/…/go-ai-native-tcg-bridge/tests/live_oracle.rs:3` («shutdown
   with no zombie») and its `go-ai-native-mcp` twin; the Rust equivalent is what
   D5's F-192 already recorded.
2. **Implementation-side reaping** — `typescript-ai-native-tcg-bridge/src/transport.rs:206
   self.child.try_wait()` under a comment at `:223` («Kill-on-drop: no zombie node
   children (TCG-ORACLE §6)»), plus its vendored and cached copies. The TypeScript
   transport is in fact the *most* careful of the three at the implementation
   layer — and it is still implementation, not a test assertion.
3. **The wave-6 perimeter catch, and it is a genuine one that nevertheless does
   not rescue the anchor** —
   `packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-pod/tests/loopback.rs:288-299`
   is a real, working no-zombie assertion in this repository:

   ```
   // The job object must reap the worker within moments of the pod dying.
   let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
   loop {
       let mut system = sysinfo::System::new();
       let target = sysinfo::Pid::from_u32(worker_pid);
       system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
       if system.process(target).is_none() { break; }
       assert!(std::time::Instant::now() < deadline,
           "worker {worker_pid} still alive 5s after its pod died — F5 regressed");
   ```

   It asserts the property for **fractality's own pod/worker pair**, not for a
   rust-analyzer / gopls / node child. So it does **not** falsify this anchor —
   but it is the single most useful thing this search found, because it means the
   technique is already proven in-tree and Phase E does not have to invent it.
   Recorded rather than used, because widening a perimeter is only legitimate
   when the thing found is the thing claimed.

**Proposed correction (NOT APPLIED):** the fix is one word-group in a
four-element list, and it should land in the **same diff across all six copies**
or none, since three of them currently read `confirmed`. Suggested for
`vibe-agentic-tcg-rust.md:203-205`:

```
- ##RISK-WINDOWS-CHILD-LIFECYCLE **Windows child lifecycle.** The house lesson set applies
  (verbatim-free paths into URIs, kill-on-drop, shutdown/exit dance —
  Phase-0-proven and each asserted in the bridge's unit suite). The
  no-zombie property is implemented (kill-on-drop, `client.rs:348`) and
  **not yet asserted**: no test in any of the three stacks probes the
  process table for a surviving child. @impl/done
```

The Go and TypeScript briefs take the identical clause. **This is the one place
in the batch where a family-wide edit is the right move** — but only because the
fact is false in all three, which the measurement above establishes rather than
assumes.

**Recommendation per anchor:** `##RISK-WINDOWS-CHILD-LIFECYCLE` (rust) → **drift
stands, correction prepared.** Flagging for the boss that the three `confirmed`
copies need a re-judgement in the same pass, or the family ships two answers to
one question.

---

## F-216 — a dead evidence id (already corrected in the TypeScript twin), and a present-tense claim on a name a shipped binary already holds

**Outcome:** SURVIVES — both anchors, and both are Rust-only: the Go and TypeScript siblings are correct or already corrected.
**Anchors:** 2 of 2, both in `rust-ai-native-lang/v0.7.0/spec/rust/tools/rust-ai-native-tcg.md`
- `##DERIVED-FROM-THE-EVIDENCE` (`:11`) → **SURVIVES**
- `##RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL` (`:35`) → **SURVIVES**

**Perimeter searched:** the standing perimeter for `DR1-014` as an anchor
(`##FINDING-DR1-014` / `##DR1-014`) and as a citation, over `*.md`; the full
`##FINDING-DR1-*` roster of `core-ai-native/v0.8.0/spec/appendix/ATLAS.md`;
`R2C-005` and `DR2-012`; the three stacks' token-level tool briefs
(`rust-ai-native-tcg.md`, `go-ai-native-tcg.md`, `typescript-ai-native-tcg.md`);
the package's `vibe.toml` `[[binary]]` table and `crates/rust-ai-native-tcg/src/main.rs`.

**The verdict's own command, re-run:** the verdict quotes none. It cites
`ATLAS.md:105`, `:107` and `:184`; all were read and all three resolve.

### Anchor 1 — `##DERIVED-FROM-THE-EVIDENCE` → SURVIVES

**Re-measured, because a count in a verdict decays.** The ATLAS's `DR1-*` roster
is **23 authored ids over the range 001–024, with exactly one gap, and the gap
is 014**:

```
$ grep -o "##FINDING-DR1-[0-9]*" .../core-ai-native/v0.8.0/spec/appendix/ATLAS.md | sort -u
##FINDING-DR1-001 … ##FINDING-DR1-013   ##FINDING-DR1-015 … ##FINDING-DR1-024
```

(`DR1-013` at `:45`, `DR1-015` at `:181` — the file is not in numeric order, so
«the roster steps 013 → 015» is only true of the id space, not of the lines.)
The two ids beside it resolve: `##FINDING-DR2-012` at `ATLAS.md:105` and
`##FINDING-R2C-005` at `:107`.

`DR1-014` exists nowhere as an anchor:

```
$ grep -rn "##FINDING-DR1-014\|##DR1-014" --include=*.md packages vibedeps crates spec research campaigns legacy-spec
(no output)
```

and survives as a citation in exactly two live documents plus campaign
bookkeeping: this brief (`:11`), the TypeScript guide (`GUIDE-AI-NATIVE-TYPESCRIPT.md:39`),
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:2583`, and cached copies
under `research/rust-demo/.vibe/cache/` and
`packages/org.vibevm.fractality/*/.vibe/cache/` (superseded slots, per §3.3).

**Per stack — and the TypeScript copy is already fixed, which is the whole
recommendation here.** `GUIDE-AI-NATIVE-TYPESCRIPT.md:39
##ADVANTAGE-1-TCD-EXISTS-FOR-TYPESCRIPT` carries D5's F-168 clause verbatim:
«*One of those two evidence ids does not resolve: `R2C-005` is authored
(`##FINDING-R2C-005` in the core ATLAS), `DR1-014` is not — the roster runs
`DR1-013`, then `DR1-015`, and no document in the tree defines a `DR1-014`. The
claim itself stands on the cited paper and on R2C-005; the second id is a dead
reference.*» The Rust brief is the second of the two documents and has not had
it. There is no Go copy.

**Proposed correction (NOT APPLIED)** — mirror the TypeScript clause exactly, so
the family carries one wording. For `rust-ai-native-tcg.md:11`, append to the
existing sentence:

```
*One of the three cited ids does not resolve: `R2C-005` (ATLAS `##FINDING-R2C-005`)
and `DR2-012` (`##FINDING-DR2-012`) are authored; `DR1-014` is not — the roster
runs 001–013 and 015–024 with 014 the only gap, and no document in the tree
defines it. The 74.8 % result stands on DR2-012 and on the cited paper; the
second id is a dead reference.*
```

### Anchor 2 — `##RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL` → SURVIVES

The chain the anchor sits in is: `##NO-TYPE-AWARE-TOOL-EXISTS-FOR-RUST` (`:33`)
— «no type-aware constrained-generation tool exists» — then `:35`,
«`rust-ai-native-tcg` **is** that missing tool for Rust, delivered as a vibevm
component». Present tense, `@spec/done`. A binary of exactly that name ships and
is a different tool:

```
$ grep -n -B1 -A3 'name = "rust-ai-native-tcg"' .../rust-ai-native-lang/v0.7.0/vibe.toml
44:[[binary]]
45:name = "rust-ai-native-tcg"
46:crate = "crates/rust-ai-native-tcg"
47:description = "the agentic type oracle: a persistent enriching relay (serve) over the consumer's
    rust-analyzer, one-shot validate/scope/complete/type forms, and the differential bench harness"

$ head -4 .../crates/rust-ai-native-tcg/src/main.rs
//! bin `rust-ai-native-tcg` — the agentic type oracle's CLI face
//! (TCG-PROTOCOL-RUST v0.1): the persistent `serve` relay, the
//! one-shot validate/scope/complete/type forms (the agent-without-MCP
//! path and the debug surface), and the bench harness.
```

This is the same collision class D5's F-282 corrected on the TypeScript
`scaffold-d` card, and its ruling applies here in reverse: the shipped binary is
the **consultation** oracle; the **token-level generation** capability this brief
is about is `stage="spec"`, not built. A reader who follows the name lands on a
real tool that is not the tool the sentence promises.

**Per stack — Rust is the outlier, and the two siblings show why.** Both Go and
TypeScript hold the same token-level brief under the same shared name and
neither makes a present-tense identity claim:

```
$ head -9 .../typescript-ai-native-lang/v0.6.0/spec/typescript/tools/typescript-ai-native-tcg.md
##status-line *Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**, and
**dispositioned VERY-FAR-FUTURE by the owner (2026-07-07)**: token-level
(logit-mask) TCG requires an inference substrate vibevm does not have …*

$ head -11 .../go-ai-native-lang/v0.1.0/spec/go/tools/go-ai-native-tcg.md
##status-line *Status: vision / component brief — **DELIBERATELY HELD AT STUB DEPTH**, …*
```

`grep -rn "IS-THAT-MISSING-TOOL\|is that missing tool"` over all three stacks'
`spec/` returns the Rust line and nothing else. The Rust brief's own `##status-line`
and `##VERY-FAR-FUTURE-DISPOSITION` (`:5`, `:7`) say the same thing the siblings
say — so the document contradicts itself four lines apart, and `:35` is the line
that is wrong.

**Proposed correction (NOT APPLIED)** — future tense, and name the collision so
nobody resolves it by reading the binary:

```
##RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL `rust-ai-native-tcg` is the name
reserved for that missing tool for Rust — to be delivered as a vibevm component
so the swarm's weak agents generate well-typed Rust by construction rather than
by retry. The binary shipping under that name today is the AGENTIC consultation
oracle (`vibe.toml` `[[binary]]`, `crates/rust-ai-native-tcg`); the token-level
generation tier this brief specifies is held VERY-FAR-FUTURE per ##status-line
and is not built. @spec/done
```

**Recommendation per anchor:**
- `##DERIVED-FROM-THE-EVIDENCE` → **drift stands, correction prepared** — and the
  wording already exists in the TypeScript twin, so the diff is a transcription.
- `##RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL` → **drift stands, correction
  prepared.** Rust-only; the Go and TypeScript briefs need nothing.

---

## F-154 — six Rust guide facts: five survive, one rests on a false premise, and **the per-stack split is different for every one of them**

**Outcome:** MIXED — 5 SURVIVES (of which 3 are **RUST ONLY**, 2 are **RUST + TYPESCRIPT**) · 1 FALSE PREMISE, DIFFERENT DEFECT
**Anchors:** 6 of 6, all in `rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md`

| anchor | line | outcome | true in |
|---|---:|---|---|
| `##NAMES-ARE-TOKEN-PROGRAMS` | 57 | SURVIVES — **rust only** | go (instantiated), ts (clause absent) |
| `##POSITION-IS-A-RESOURCE` | 59 | SURVIVES — **rust + typescript** | go (claims only the length half) |
| `##SCAFFOLD-B-TYPED-BUILDERS` | 68 | SURVIVES — **rust only** | go (different claim) |
| `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` | 72 | SURVIVES — **rust + typescript** | go (claims no custom linter) |
| `##PROSE-NEAR-CODE-IS-CHECKED-OR-TRUST-LABELED` | 113 | **FALSE PREMISE, DIFFERENT DEFECT** | — (a family-wide defect the verdict mis-names) |
| `##DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL` | 127 | SURVIVES | ts twin judged `unverifiable` |

**Perimeter searched — and this is the entry where widening mattered most.**
Every one of the six verdicts names its perimeter as «the host's crates AND
`research/rust-demo/`», which is exactly the scope
[§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness) says
misses the second adopter. So every search below was re-run over the standing
perimeter **including `packages/org.vibevm.fractality/fractality/v0.1.0/`**,
which is a Rust project that adopted this discipline — it carries its own
`conform.toml`, `specmap.toml`, `discipline/registry/` and Cargo workspace:

```
$ ls packages/org.vibevm.fractality/fractality/v0.1.0/
AGENTS.md  CLAUDE.md  Cargo.lock  Cargo.toml  GEMINI.md  LICENSE.md  README.md
conform-baseline.json  conform.toml  crates  …  specmap.toml  discipline/registry
```

**The widening changed nothing for these six** — but that is a result, not a
formality: it is what makes the five survivals safe to act on.

**The verdict's own commands, re-run:** the verdicts quote none; they cite
`ATLAS.md:55`, `cards/INDEX.md:37`, `codemod.rs:14`, `budget.rs:143/146`,
`conform.toml:36`, `rules/mod.rs:50`, `rust-ai-native-conform/src/lib.rs:137`,
`diagnostics.rs:44`, `specmark-grammar/src/lib.rs:53`, `registry.rs:63`. All were
read.

### 1. `##NAMES-ARE-TOKEN-PROGRAMS` (`:57`) → SURVIVES, **rust only**

The claim: «Canonical cell name is **computed** from the manifest
(`{Variant}{Seam}`)». Over the full perimeter, that form occurs **only** in
`core-ai-native`'s legacy projections:

```
$ grep -rn "Variant}{Seam" --include=*.rs --include=*.md --include=*.toml --include=*.go --include=*.ts \
    packages crates xtask tools spec research discipline terraform | grep -v "/target/" | grep -v "\.vibe/cache"
…/core-ai-native/v0.7.0/spec/legacy-projections/GUIDE-{CPP-MISRA2008,CPP-MODERN,CPP-TRADITIONAL,GO,JAVA,KOTLIN,PYTHON,TYPESCRIPT}-v0.1.md
…/core-ai-native/v0.8.0/spec/legacy-projections/GUIDE-…  (the same eight, one version up)
```

Sixteen hits, all in one package's legacy-projection directory, none of them
Rust. No Rust cell type in the host, in `research/rust-demo`, or in fractality
is named that way, and no lint checks names against a manifest — the card that
would (`rule-closed-vocabulary-naming`, R3-004) is listed in every stack's index
as a *pending* card (`rust-…/spec/cards/INDEX.md:28`: «candidate future card
`rule-closed-vocabulary-naming`»; `core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md:7`
names all seven pending rule cards).

**Per stack — and this is the batch's clearest trap.**
- **go `GUIDE-AI-NATIVE-GO.md:213-214`** makes the *identical* claim
  («Canonical cell type name is computed from the manifest: `{Variant}{Seam}` →
  `BatchPlanner`») — **and the Go consumer instantiates it**:
  ```
  research/go-demo/internal/seams/seams.go:54          type Planner interface {
  research/go-demo/internal/cells/batchplanner/planner.go:12   type BatchPlanner struct{}
  research/go-demo/internal/cells/naiveplanner/planner.go:13   type NaivePlanner struct{}
  ```
  `{Batch}{Planner}` and `{Naive}{Planner}`, literally, in the discipline's own
  Go demonstration. The anchor is judged **`confirmed`** and correctly so.
- **typescript `GUIDE-AI-NATIVE-TYPESCRIPT.md:125`** **drops the `{Variant}{Seam}`
  clause entirely** — it says only «one name = one referent across the contract
  surface; no shadowing, no synonym pairs; structural tokens from a closed
  vocabulary». Not vulnerable to this defect at all; judged `confirmed`.

**A family-wide edit deleting `{Variant}{Seam}` would delete a Go sentence with a
live demonstration behind it.** This is the release queue's warning, met exactly.

**Proposed correction (NOT APPLIED)** — Rust only, and *narrow*: drop the
computed-name clause, keep every other clause (they are norms, not claims about
artefacts). Suggested for `:57`:

```
- ##NAMES-ARE-TOKEN-PROGRAMS **Names are token programs** (R3-004, R-020). Across
  contract surfaces: one name = one referent repo-wide; **no shadowing, no
  synonym pairs**. Structural tokens come from a closed vocabulary. Length is
  free; ambiguity is not. (Short closure-local bindings are exempt …) *The
  computed `{Variant}{Seam}` cell-name form is the Go and legacy-projection
  convention; Rust cells are not named from the manifest and no lint checks
  them — `rule-closed-vocabulary-naming` is a pending card.* @impl/done
```

`go-…/GUIDE-AI-NATIVE-GO.md:213` → **none — correct as written, with an instance.**
`typescript-…:125` → **none — the clause is not there.**

### 2. `##POSITION-IS-A-RESOURCE` (`:59`) → SURVIVES, **rust + typescript**

The claim is a conjunction: «A conform check warns on files over a length
threshold **and on invariant-bearing comments in the middle third**.» The first
half ships and is mounted in all three gates (`FileLength`, at
`rust-ai-native-conform/src/lib.rs:81`, `go-…:59`, `typescript-…:57`; the host's
`conform.toml:36 max_file_lines = 600`). The second half is prose only:

```
$ grep -rn "middle third\|middle_third\|middle-third" --include=*.rs packages crates \
  | grep -v "/target/" | grep -v "\.vibe/cache"
…/core-ai-native-conform/src/rules/budget.rs:119   /// pages badly and buries invariants in its middle third; prefer
…/core-ai-native-conform/src/rules/budget.rs:147            its middle third buries invariants — prefer more, smaller, \
```

Two hits per copy of the engine: `:119` is a **doc comment** and `:147` is the
`file-length` rule's own **message string**. Nothing inspects comment position.

**Per stack:**
- **go `:236`** claims **only** the length half — «A conform check warns on files
  over the length budget. @status:impl/done». **Correct as written**; judged `confirmed`,
  correctly.
- **typescript `:128`** claims **both**, in the same conjunction as Rust — «warns
  on files over a length threshold **and on invariant-bearing comments in the
  diluted middle third**» — and is judged **`confirmed`**. Same defect, opposite
  judgement. Not in this batch; recorded.

**Proposed correction (NOT APPLIED):** rust `:59` → drop «and on
invariant-bearing comments in the middle third», or mark it as the unbuilt half.
The TypeScript copy needs the identical edit and a re-judgement first.
`go-…:236` → **none — correct as written.**

### 3. `##SCAFFOLD-B-TYPED-BUILDERS` (`:68`) → SURVIVES, **rust only**

The verdict conceded typestate and newtypes and named sealed traits and
`PhantomData` as absent. Re-measured over the **widened** perimeter — host
`crates/`, all packages, `xtask/`, `tools/`, `research/**` and fractality —
both are still zero:

```
$ grep -rln "PhantomData" --include=*.rs packages crates xtask tools research \
  | grep -v "/target/" | grep -v "\.vibe/cache"
(no output)

$ grep -rn "mod sealed\|trait Sealed\|: Sealed\b\|private::Sealed" --include=*.rs \
    packages crates xtask tools research | grep -v "/target/" | grep -v "\.vibe/cache"
(no output)
```

`#[must_use]` is real and plentiful (11 occurrences in `crates/vibe-actions/src/action.rs`
alone), so the fact is a partial rather than a fiction: two of its named
constructions ship and two are absent from every Rust project in the repository,
including the second adopter.

**Per stack:** go `##SCAFFOLD-B-TYPED-SURFACES:255` makes a different, correct
claim («**Go's defined types are nominal for free** — `type AccountID string`
does not interchange with `string`…»), judged `confirmed`. typescript
`##SCAFFOLD-B-TYPED-SURFACES:137` claims «phantom-type-parameter builders» and
«sealed unions» and is judged `confirmed` — whether `research/ts-demo` carries
either is a question this batch does not own (D5's F-168 established the demo is
five `.ts` files) and is recorded, not asserted.

### 4. `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` (`:72`) → SURVIVES, **rust + typescript**

Two of three ship: REQ-citing messages (`rules/mod.rs:50`,
`format!("violates REQ {uri}: {why}; fix surface: {fix_surface}")`) and SARIF
(`rust-ai-native-conform/src/lib.rs:137`). The third does not — there is no
custom lint anywhere in the tree:

```
$ grep -rn "dylint\|declare_lint\|declare_clippy_lint\|LateLintPass" --include=*.rs --include=*.toml \
    packages crates xtask tools research | grep -v "/target/" | grep -v "\.vibe/cache"
(no output)
```

**Per stack — and the split is the same as anchor 2:**
- **go `:284-286`** carefully says «custom checks emit the same grammar; conform
  emits SARIF» and names **no custom linter at all**. **Correct as written**;
  `confirmed`.
- **typescript `:141`** claims «Custom `@typescript-eslint` rules whose messages
  cite the violated `spec://` REQ and the fix surface» — and there are none.
  `grep -rn "createRule\|ESLintUtils"` over the TypeScript stack and
  `research/ts-demo` returns nothing, and the demo's config is stock:
  ```
  $ cat research/ts-demo/eslint.config.js
  // Flat config at the cards' Band-3 baseline: typescript-eslint
  // recommended over src/. The conform gate owns the discipline-specific
  // structural rules; eslint owns the generic lint layer beneath them.
  import tseslint from "typescript-eslint";
  export default tseslint.config({ ignores: […] }, ...tseslint.configs.recommended);
  ```
  The config's own comment concedes it: the discipline-specific rules live in
  conform, not in eslint. Judged **`confirmed`**. Same defect, opposite judgement.

**Proposed correction (NOT APPLIED):** rust `:72` → «custom clippy lints name the
rule and the remedy» becomes the unbuilt clause, e.g. «*custom clippy lints
naming the rule and the remedy are specified and unbuilt — no `dylint` or
`declare_lint` ships and the floor's clippy step is stock*». `go-…:284` →
**none.** The TypeScript copy needs the parallel edit **after** a re-judgement.

### 5. `##PROSE-NEAR-CODE-IS-CHECKED-OR-TRUST-LABELED` (`:113`) → FALSE PREMISE, DIFFERENT DEFECT

The verdict reads: *«`#[spec(documents)]` edges occur **zero times** in the
host's crates … **The documents edge is in the model and is used by nothing.**»*
The second sentence is false. The edge is **consumed by a shipped, mounted
conform rule** and named in that rule's own remedy text:

```
$ sed -n '168,192p' .../crates/vendor/core-ai-native-conform/src/rules/diagnostics.rs
                // A `#[spec(documents = …)]` edge is the prose-free alternative
                // to a compiled example.
                if attrs.iter().any(|a| a.contains("documents")) {
                    continue;
                }
                …
                        &format!("add one doctest on `{name}` showing canonical use, or a \
                                  #[spec(documents = \"…\")] edge"),
```

That is `pub-doctest`, mounted in the shipped Rust gate at
`crates/rust-ai-native-conform/src/lib.rs:71` (`rules::PubDoctest`). So the edge
is not «in the model and used by nothing» — it is a first-class escape hatch of a
live gate, and a demotion written on the verdict's premise would print
«unbuilt» over a mounted rule.

**What is actually wrong, and it is two things the verdict does not name:**

*(a) no adopter authors one.* The only real `documents = …` usages in the tree
are the proc-macro's own test and its vendored copies
(`core-ai-native-specmark/tests/usage.rs:30`,
`research/*/vibedeps/**/usage.rs:28`). Zero in host `crates/`, zero in
`research/rust-demo`, zero in `research/ts-demo`, zero in fractality. The
mechanism is live and unexercised — which is a different and weaker statement
than the verdict's.

*(b) the two other halves of the sentence have no mechanism at all.* «making
drift detectable via **spec-rev bumps**» — no spec-rev machinery exists
(`grep -rniE "spec.rev|specrev|rev_bump"` over `crates/`, the Rust stack's
`crates/` and `schemas/` returns only `vibe-cli`'s unrelated `--rev` git flag and
a `Self::Rev(_)` dependency variant). And «or **explicitly trust-labeled**
(verified / unverified / aspirational)» has **zero** implementation anywhere:

```
$ grep -rniE "aspirational" --include=*.rs --include=*.go --include=*.ts --include=*.toml --include=*.json \
    packages crates xtask tools research discipline terraform | grep -v "/target/" | grep -v "\.vibe/cache"
(no output)
```

**Per stack — the trust-label half is family-wide and judged three different
ways.** The same «or explicitly trust-labeled (verified / unverified /
aspirational)» clause appears in all three guides:
`rust:113` (drift, this anchor), `go ##RULE-BEHAVIORAL-CLAIMS-ARE-CHECKED-OR-LABELLED:466`
(**`confirmed`**), `typescript ##RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED:209`
(**drift, F-140 below**). One clause, no mechanism, three verdicts.

**Proposed correction (NOT APPLIED):** the diff the owner should see is *not* the
one this verdict implies. Suggested for `:113`:

```
##PROSE-NEAR-CODE-IS-CHECKED-OR-TRUST-LABELED Therefore prose near code is
**machine-checked** — doctests for behavioral claims (`seam-has-doctest`,
`pub-doctest`, both mounted), with a `#[spec(documents)]` edge accepted as the
prose-free alternative (`core-ai-native-conform/src/rules/diagnostics.rs`) —
or **explicitly trust-labeled**. *Two clauses are specified and unbuilt: no
spec-rev bump machinery exists, and no trust-label vocabulary
(verified / unverified / aspirational) is implemented or checked anywhere; no
project in this repository has yet authored a `#[spec(documents)]` edge.*
@impl/done
```

### 6. `##DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL` (`:127`) → SURVIVES

`R-060` is cited and authored nowhere:

```
$ grep -rn "R-060" --include=*.md --include=*.rs packages spec crates research campaigns \
  | grep -v "/target/" | grep -v "\.vibe/cache" | grep -v vibedeps
.../rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:127   *(R-060, retained.)*
.../typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:231   (R-060, projected)
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:2385   … R-021 and R-060 … cited by name whose cards are unauthored
crates/vibe-cli/src/registry.rs:63   /// the R-060 flag-matrix generator is its Phase 4+ runtime consumer.
```

Two citations, one campaign record, and one host doc comment naming a **future**
consumer («its Phase 4+ runtime consumer»). No card, no `##FINDING-` anchor in
the ATLAS, no conform rule. Same class as the `R-021` gap D5's F-191 recorded and
the `R-001`-without-an-ATLAS-entry gap its F-168 recorded.

**Per stack:** typescript `##MATRIX-IS-AUTHORED-DATA:231` cites the same id and is
judged **`unverifiable`**, with a reason that agrees on the facts — «a rule
projected from the Rust guide with **no checker on either side** and no instance
in the demo consumer». Third judgement class on one id. No Go copy.

**Proposed correction (NOT APPLIED):** the prescription is sound; the `@impl/done`
and the bare id are not. Suggested: `*(R-060 — cited as retained; no card and no
ATLAS entry is authored for it, and no checker enforces declared matrices.)*
Declared test matrices, never 2^n. @status:spec/done`

**Recommendation per anchor:**
- `##NAMES-ARE-TOKEN-PROGRAMS` → **drift stands, correction prepared — RUST ONLY.**
- `##POSITION-IS-A-RESOURCE` → **drift stands, correction prepared — rust + the
  unjudged TypeScript copy; the Go copy is correct.**
- `##SCAFFOLD-B-TYPED-BUILDERS` → **drift stands, correction prepared — RUST ONLY.**
- `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` → **drift stands, correction prepared —
  rust + the unjudged TypeScript copy; the Go copy is correct.**
- `##PROSE-NEAR-CODE-IS-CHECKED-OR-TRUST-LABELED` → **drift stands, correction
  prepared on a DIFFERENT defect.** The verdict's own reason must not reach the
  owner unamended: acting on it would mark a mounted rule unbuilt.
- `##DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL` → **drift stands, correction prepared.**

---

# The Go stack — judged under the [owner ruling](#ruling)

_Every entry below states which bench each finding rests on. **`research/go-demo`,
the host's `crates/`, `.claude/skills/` and `legacy-spec/**` are not evidence
about `go-ai-native-lang`.** The legitimate bench is the package's own tree and
its own tests — principally `tools/go-extract/test/fixtures/{clean,dirty}/`
(each with its own `conform.toml`, `specmap.json`, cells and registry), the
fifteen `crates/**` test modules, and the package's own guides, cards and
`vibe.toml`._

---

## F-166 — two of the four convictions are host-bench artefacts and fall; the other two survive on the package's own fixtures

**Outcome:** MIXED — 2 SURVIVES · 2 **FALSE**
**Anchors:** 4 of 4, all in `go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md`

| anchor | line | outcome | why |
|---|---:|---|---|
| `##BASELINE-RACE-DETECTOR-GATES-TESTS` | 139 | **FALSE** | both grounds void: a package-rooted capture artefact + host non-configuration |
| `##CONFORMANCE-IS-MADE-LOUD` | 191 | SURVIVES | the package's own extractor, gate and **clean fixture** |
| `##RELEASE-MAP-IS-FREE` | 357 | **FALSE** | «no Go binary in this tree» is host evidence — and the mechanism demonstrably works |
| `##TWO-TIERS-NEVER-CONFUSED` | 381 | SURVIVES | no tier machinery in the package's own engine |

**Perimeter searched:** the package's own tree in full —
`crates/**` (including `crates/vendor/core-ai-native-conform`),
`tools/go-extract/**` with both fixtures, `spec/**`, `vibe.toml`, `README.md` —
for `-race`, `var _ ` conformance assertions, `//go:build`, and
`Tier` / `"tier"` / `T-lex`. Host artefacts are reported **only as
corroboration** and are marked as such.

### `##BASELINE-RACE-DETECTOR-GATES-TESTS` (`:139`) → **FALSE**

The verdict: «the floor's test step is `go test ./...` with no `-race`, **and it
fails outright** — the package root is a Cargo workspace, not a Go module, so
`./...` resolves to nothing (`harvest/go-ai-native-lang-floor.md`). The MUST
configuration is **neither configured nor reachable**.»

**Both grounds are void, for two independent reasons.**

*(a) «it fails outright» is a capture artefact.* The harvest names its own cwd:

```
$ head -3 campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md
_Captured 2026-07-28 against `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`._
$ go vet ./...
pattern ./...: directory prefix . does not contain main module or its selected dependencies
```

That directory is a **Cargo** workspace. Running a Go floor there and recording
the result as the mechanism's behaviour is the third instance of the trap §6.1
records as «a package-rooted floor run read as a pilot's chain». *(Corroboration
only, not a ground under the ruling: pointed at a real Go module the same binary
reports `floor: all green (7 step(s) run, 0 disabled by policy)`, EXIT=0 — the
full transcript is under F-273 below.)*

*(b) «neither configured» is host evidence, and the ruling voids it.* Whether
this repository configures `go test -race` says nothing about a package this
repository deliberately does not adopt.

**And the anchor does not claim what the verdict tests it against.** It reads:
«**The race detector gates tests:** `go test -race` is the MUST configuration
**for any package that starts a goroutine**; findings are failures, not
warnings.» That is a rule addressed to a *consuming* Go package. It does not say
the shipped floor adds the flag, and no external consumer's conformance is
observable from here.

**On the legitimate bench, one weaker observation remains, and it is offered as
an observation rather than as the verdict's defect.** The string `-race` occurs
**nowhere** in the package (`grep -rn "race"` over the package returns only
substring hits on `trace` / `traceability`), so the floor the package ships does
not itself enforce the MUST it states — and the package's own dirty fixture does
start a goroutine (`tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go`:
`go func() { _ = home }()   // naked_go`), caught there as the `naked_go` census
kind rather than race-tested. Whether a guide rule for consumers must be
enforced by the shipped floor is a design question, not a drift finding, and it
is the boss's to take.

**Per stack:** Go-specific by construction — Rust has no race detector and the
TypeScript floor has no analogue.

**Proposed correction (NOT APPLIED):** **none.** The description is a rule and it
is stated correctly. If the boss wants the observation recorded, it belongs as a
Phase-E item («add `-race` to the Go floor's test step»), not as a spec diff.

**Recommendation:** `##BASELINE-RACE-DETECTOR-GATES-TESTS` → **re-judge
`confirmed`.** No edit, no diff, no owner approval.

### `##CONFORMANCE-IS-MADE-LOUD` (`:191`) → SURVIVES

«every cell carries the compile-time assertion, **and conform checks its presence
(T-syn)**.» The second clause fails on the package's own bench, three ways:

*(a) the gate mounts three rules and none of them is it:*

```
$ grep -n "out.push(Box::new" .../crates/go-ai-native-conform/src/lib.rs
53:    out.push(Box::new(rules::GoUnsafeInDomain::new(config.go.cells_dir.as_deref())));
57:        out.push(Box::new(rules::GoCellIsolation::new(cells_dir)));
59:    out.push(Box::new(rules::FileLength { max_lines: config.max_file_lines }));
```

*(b) the extractor discards the construct before conform can ever see it:*

```
$ sed -n '458,466p' .../tools/go-extract/extract.go
			for _, name := range s.Names {
				if name.Name == "_" {
					continue // conformance assertions et al.
				}
```

A `var _ Seam = (*Impl)(nil)` declaration is `_`-named by construction, so no
fact is produced for it. A presence rule could not be mounted without changing
the extractor first.

*(c) — and this is the decisive one — the package's own CLEAN fixture omits the
assertion.* That fixture is the package's own model of a compliant cell («the
clean fixture: a well-formed cell under the discipline», `greet.go:1-3`; its
policy comment reads «same topology as dirty, **zero findings**»):

```
$ grep -rn "var _ " .../tools/go-extract/test/fixtures/
(no output)
```

So on the only bench the ruling admits, the package's own exemplar of «every cell
carries the compile-time assertion» does not carry it, and nothing would notice.

**Proposed correction (NOT APPLIED):** keep the prescription; strike or qualify
«and conform checks its presence (T-syn)», naming `extract.go:460-462` as the
reason a checker cannot be mounted without an extractor change. `@spec/done`.

### `##RELEASE-MAP-IS-FREE` (`:357`) → **FALSE — the description is right**

The verdict: «**no Go binary is built anywhere in this tree** — the only Go is a
`go run` sidecar source — so `runtime/debug.ReadBuildInfo` embeds nothing and the
A1 chain the fact says comes free has no artifact to come from.»

**«anywhere in this tree» is the wrong tree.** Under the ruling, whether *this*
repository builds a Go binary is not evidence about a package built for external
consumers — and the anchor is in any case a statement about the **Go toolchain**,
not about any repository: «Every Go binary embeds `runtime/debug.ReadBuildInfo`
(VCS revision, dirty flag, module versions), readable from the artifact (`go
version -m`).»

*Corroboration, offered because a toolchain claim is cheap to demonstrate and
expensive to argue about — output written to the scratchpad, never into the
repo:*

```
$ go build -o <scratchpad>/reconcile.exe ./cmd/reconcile      # from a Go module
BUILD OK
$ go version -m <scratchpad>/reconcile.exe
… reconcile.exe: go1.26.5
	path	reconcile-demo/cmd/reconcile
	mod	reconcile-demo	(devel)
	build	vcs=git
	build	vcs.revision=b61eb191993ebb7b4531a9e2b70d7a43875ace67
	build	vcs.time=2026-07-31T10:47:17Z
	build	vcs.modified=true
```

**VCS revision · dirty flag · module versions · read with `go version -m`** —
the sentence's four items, in the order it lists them, from the exact command it
names. The release map is free because the Go toolchain makes it free; no
consumer of this package has to do anything for the claim to hold.

The one clause not demonstrated is the tail of the A1 chain
(*binary → build info → specmap@commit → REQ*): no shipped tool joins
`vcs.revision` to `specmap.json`. But the sentence says that chain «needs zero
extra machinery», which is a statement about what is *not* required — and it
does not claim a tool exists.

**Proposed correction (NOT APPLIED):** none — correct as written.

**Recommendation:** `##RELEASE-MAP-IS-FREE` → **re-judge `confirmed`.**

### `##TWO-TIERS-NEVER-CONFUSED` (`:381`) → SURVIVES

«build tags (`//go:build`) … are confined to registry/adapter files, never inside
cell bodies **(T-lex)**; runtime flags … read once into a config struct in `main`
and passed down.» The norm is fine and unfalsifiable from here — no external
consumer is visible. **What fails on the package's own bench is the parenthetical
`(T-lex)`, which names a tier the engine does not have:**

```
$ grep -rniE "\bTier\b|\"tier\"|T-lex|build_tag|go:build" --include=*.rs .../go-ai-native-lang/v0.1.0/crates/
(no output)
```

Repo-wide, the tier vocabulary survives only as doc-comment prose — `T-syn` at
`core-ai-native-conform/src/facts.rs:157` — and **`T-lex` appears in no source
file at all**. The package's own fixtures use no build tags either
(`grep -rn "go:build" tools/go-extract/test/fixtures/` → no output), so the
package ships no demonstration of the confinement it prescribes.

**Note the scope this leaves.** The verdict's own framing («the two tiers rest on
the tier vocabulary C1a found unbuilt … so «T-lex» names nothing that exists») is
the surviving half and is package-internal. Everything the verdict might have
inferred from `research/go-demo`'s flag handling is void and is not used.

**Proposed correction (NOT APPLIED):** keep the norm; drop or qualify `(T-lex)`.
Same class as `R-021` / `R-060` / `R-001`-without-an-ATLAS-entry — a tier label
cited as if it named a shipped check.

**Recommendation per anchor:** `##BASELINE-RACE-DETECTOR-GATES-TESTS` →
**re-judge `confirmed`** · `##CONFORMANCE-IS-MADE-LOUD` → drift stands,
correction prepared · `##RELEASE-MAP-IS-FREE` → **re-judge `confirmed`** ·
`##TWO-TIERS-NEVER-CONFUSED` → drift stands, correction prepared.

---

## F-167 — four Go oracle facts, all four surviving on the package's own tree and tests

**Outcome:** SURVIVES ×4 — 1 **GO ONLY**, 2 family-wide (reported under F-215 and
F-281), 1 package-internal. **The ruling changes none of them**, because every
ground here is the package's own code, its own spec, or its own tests.

| anchor | line | outcome | bench |
|---|---:|---|---|
| `##QUANTITIES-ARE-CAMPAIGN-MEASURED` | 17 | SURVIVES | the package's own tree ships no measurement |
| `##RESOLUTION-GOPLS-ON-PATH` | 34 | SURVIVES — **GO ONLY** | the package's own resolver vs its own §1 |
| `##GRACEFUL-EXIT-IS-THE-LSP-DANCE` | 201 | SURVIVES | the package's own tests |
| `##TARGET-COMPLETE` | 243 | SURVIVES | the package's own `bench.rs` |

**The verdict's own command, re-run:** none quoted. Cited files (`bench.rs:1`,
`go-ai-native-tcg-bridge/src/lib.rs:155`, `oracle.rs:360`, `client.rs:357`,
`tests/live_oracle.rs:3`) were all read.

### `##QUANTITIES-ARE-CAMPAIGN-MEASURED` (`:17`) → SURVIVES

«Where the sibling Rust mechanism cites measured spike facts, this one names the
same quantities as campaign-measured: **the Phase-7 live chain and the bench
harness record them**; a target moves only with a committed REPORT reason.»

This is a claim about the package's own development record, not about a
consumer's conformance, so it is inside the ruling's bench. **The package ships
the instrument and no measurement.** `run_bench` is parameterised on a
consumer-supplied corpus and report path — `Cmd::Bench { corpus, report }`
(`crates/go-ai-native-tcg/src/main.rs:77-84`) →
`run_bench(&root, &corpus, &report)` (`bench.rs:106`) — and the package carries
no corpus, no report and no ledger entry of its own:

```
$ find .../go-ai-native-lang/v0.1.0 -not -path "*/target/*" \( -iname "*corpus*" -o -iname "*baseline*" \)
.../crates/go-ai-native-tcg/src/bench.rs                       # the harness
.../crates/vendor/core-ai-native-conform/src/baseline.rs       # the conform ratchet, a different artefact
```

The document's own §8 confirms it from the other side: where the Rust mechanism
carries five `##SPIKE-*` anchors with measured numbers
(`TCG-ORACLE-RUST-v0.1.md:238-244`: «init handshake ~10 ms», «init-to-quiescent
14.7 s cache-COLD … 2.5 s warm», «completion ~19 ms at 118 entries»), the Go
document carries **no spike section at all** — it jumps from
`##TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED` (`:236`) straight to
`##posted-targets-lead` (`:239`). So the sentence says the quantities are
recorded while its own document records none.

*(Corroboration, explicitly not a ground under the ruling: the host's
`research/tcg-bench/` carries two corpora and two baselines — TypeScript, 7
cases, and Rust, 9 — with no `corpus-go`, no Go report and no Go entry among the
three `REPORT-2026-07-07-*.md`.)*

**Proposed correction (NOT APPLIED):** «…names the same quantities as
campaign-measured — *the bench harness (`go-ai-native-tcg bench`) is the
instrument; no Go corpus or baseline has yet been taken, so the figures below are
posted targets rather than measured ones*; a target moves only with a committed
REPORT reason.» `@spec/done`.

### `##RESOLUTION-GOPLS-ON-PATH` (`:34`) → SURVIVES — **GO ONLY**

Entirely package-internal: the spec's own ordered list against the package's own
resolver. §1 lists four steps and this anchor is step 1:

```
$ sed -n '30,38p' .../spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md
##resolution-order-lead Resolution order, run from the project root … :
1. ##RESOLUTION-GOPLS-ON-PATH `gopls` on PATH; @impl/done
2. ##RESOLUTION-GOBIN `$GOBIN/gopls`, then `$(go env GOBIN)/gopls`; @impl/done
3. ##RESOLUTION-GOPATH-BIN `$(go env GOPATH)/bin/gopls`; @impl/done
4. ##RESOLUTION-HARD-FAILURE hard failure … `go install golang.org/x/tools/gopls@latest`.
```

The shipped resolver runs five, and the first **hard-fails without ever probing
PATH**:

```
$ sed -n '139,155p' .../crates/go-ai-native-tcg-bridge/src/lib.rs
/// Resolve the CONSUMER's gopls (ORACLE-GO §1): the env override, then
/// PATH, then `$GOBIN`, then `$(go env GOPATH)/bin`, then the
/// recipe-carrying refusal. …
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#resolution")]
pub fn resolve_gopls(root: &Path) -> Result<PathBuf, TcgBridgeError> {
    if let Ok(overridden) = std::env::var(GOPLS_ENV_OVERRIDE) {
        let p = PathBuf::from(&overridden);
        if p.is_file() { return Ok(verbatim_free(&p)); }
        return Err(TcgBridgeError::GoplsMissing {
            detail: format!("{GOPLS_ENV_OVERRIDE}={overridden} is not a file"),
        });
    }
    // PATH: probe the bare name.
```

**This matters to an external consumer directly**, which is the ruling's own
standard: a client whose `GO_AI_NATIVE_GOPLS` is stale gets a hard refusal from a
tool whose published resolution order says PATH is tried first. And the
`#[spec(implements = …#resolution)]` edge makes the traceability graph record
agreement where the two texts disagree.

**Per stack — the Rust copy is exactly right, which is why this must not be
edited family-wide.** Rust's §1 is three steps —
`##RESOLUTION-RUSTUP-WHICH` (`:32`), `##RESOLUTION-PATH` (`:33`),
`##RESOLUTION-HARD-FAILURE` (`:34`) — and `resolve_rust_analyzer`
(`rust-ai-native-tcg-bridge/src/lib.rs:137-158`) does precisely that, in that
order, **with no env override**. TypeScript has no ordered list at all, only
`##RESOLUTION-FAILURE-IS-A-RECIPE-CARRYING-ERROR` (`TCG-ORACLE-v0.1.md:47`).

**Proposed correction (NOT APPLIED):** Go only, an insertion rather than a
rewrite — fold the override into step 1's text, or mint a step 0 mirroring the
resolver's own doc comment («the env override, then PATH, then `$GOBIN`, …»),
noting that PATH is **not** probed when the override is set and is not a file.
*(Minting a new `##ANCHOR` is a `RULE-ANCHORS-IMMUTABLE` question and is the
boss's.)* rust → **none — correct as written.** typescript → **none.**

### `##GRACEFUL-EXIT-IS-THE-LSP-DANCE` (`:201`) → SURVIVES (family-wide)

«the no-zombie property **is test-asserted**». The dance and the backstop ship
(`oracle.rs:360` `shutdown`, `client.rs:357` `impl Drop … kill(); wait()`); the
assertion does not, in this package's tests or in any other. The full sweep —
including the `packages/org.vibevm.fractality/**` widening and the finding that
the only real process-table assertion in the repository belongs to fractality's
own pod/worker pair and therefore does not rescue this anchor — is under
**F-281** and is not repeated. **Six copies of this claim exist across the three
stacks and three are judged `confirmed`;** the table is in F-281. Every leg of
that evidence is a package's own test, so the ruling does not touch it.

### `##TARGET-COMPLETE` (`:243`) → SURVIVES (family-wide)

The Go harness records per-case `warm_ms`, computes **no percentile of any kind**,
and the word `complete` appears zero times in it. Full evidence, and the
identical Rust and TypeScript findings — all three on each package's **own**
`bench.rs` — are under **F-215**.

**Recommendation per anchor:** `##QUANTITIES-ARE-CAMPAIGN-MEASURED` → drift
stands, correction prepared · `##RESOLUTION-GOPLS-ON-PATH` → **drift stands,
correction prepared — GO ONLY** · `##GRACEFUL-EXIT-IS-THE-LSP-DANCE` → drift
stands, correction prepared **as part of the six-copy family** ·
`##TARGET-COMPLETE` → drift stands, correction prepared **as part of the
three-stack family**.

---

## F-185 — three Go conform-frontend facts; all three survive on the package's own config type and its own fixtures, and the third is worse than the verdict says

**Outcome:** SURVIVES ×3. No parallel corpus — the Rust and TypeScript
`conform-frontend-*` briefs make none of these claims. **The ruling changes
nothing**: every ground is the package's own `GoConfig`, its own engine, its own
fixtures' `conform.toml`, or its own rule roster.
**Anchors:** 3 of 3, all in `go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.md`
- `##RULE-SEAM-ERROR-CONTRACT` (`:41`) → SURVIVES
- `##CONFORM-TOML-GAINS-A-GO-SECTION` (`:107`) → SURVIVES
- `##EVERY-PACKAGE-GATED-OR-EXEMPT` (`:114`) → SURVIVES — **and on a second ground the verdict does not name**

**Per stack:** none.
`grep -rn "gated-or-exempt\|gated_packages\|EVERY-.*GATED\|seam-error-cites-req"`
over `typescript-…/spec/typescript/tools/conform-frontend-typescript.md` and the
Rust stack's `spec/rust/tools/*.md` returns nothing.

### `##RULE-SEAM-ERROR-CONTRACT` → SURVIVES

The brief lists «the seam-error contract (`seam-error-cites-req` — a seam's
closed error set carries its REQ URI, guide §5)» among the frontend's **rules**.
That id is nothing:

```
$ grep -rn "seam-error-cites-req" --include=*.rs --include=*.toml --include=*.json packages crates research
(no output)
```

What ships is a census **kind** inside `go-unsafe-in-domain`
(`crates/vendor/core-ai-native-conform/src/rules/go.rs:145-149`,
`"seam_error_missing_req" if !in_test => …`), emitted by
`tools/go-extract/extract.go:534`. **The package's own two fixtures are the
proof, and they are the ruling's bench exactly:** the dirty fixture's error type
omits the field on purpose —

```
$ sed -n '17,21p' .../tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go
// PlanError lacks a Spec field on purpose (seam_error_missing_req).
type PlanError struct { Code int; Err error }
```

— and the clean fixture's carries it and renders the REQ —

```
$ sed -n '15,25p' .../tools/go-extract/test/fixtures/clean/internal/cells/greet/greet.go
// GreetError is the seam's closed failure set.
type GreetError struct { Code int; Spec string; Err error }
func (e *GreetError) Error() string {
	return fmt.Sprintf("greet: %d: violates REQ %s", e.Code, e.Spec)
}
```

So the mechanism is real and is exercised by the package's own tests **under the
name `seam_error_missing_req`**. The brief names it as a rule that does not
exist, borrowing the **Rust** pattern — `error-enum-cites-req`
(`diagnostics.rs:314`) and `error-message-cites-req` (`:235`) are real rule ids
*there*.

**Proposed correction (NOT APPLIED):** «the seam-error contract (shipped as the
census kind `seam_error_missing_req` inside `go-unsafe-in-domain`, not as a rule
of its own — guide §5)».

### `##CONFORM-TOML-GAINS-A-GO-SECTION` → SURVIVES

The brief documents `[go]` as carrying «`roots` … `cells_dir` … `seams_pkg` …
`registry_pkg` … **`gated_packages` / `[[exempt]]`** … **and the file budget**».
`GoConfig` is `deny_unknown_fields` with six keys, none of them those three:

```
$ sed -n '104,131p' .../crates/vendor/core-ai-native-conform/src/config.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GoConfig {
    pub roots: Vec<String>,
    pub exclude_substrings: Vec<String>,
    pub cells_dir: Option<String>,
    pub seams_pkg: Option<String>,
    pub registry_pkg: Option<String>,
    pub floor_disable: Vec<FloorDisable>,
}
```

`gated_packages` occurs zero times repo-wide in any `.rs` or `.toml`. Because the
struct is `deny_unknown_fields`, **an external consumer who writes the section as
documented gets a hard config error**, not an ignored key — which is precisely
the class of defect the ruling says matters most, since we cannot see that
consumer fail.

**The package's own fixtures write only the declared keys**, which is the bench's
own answer:

```
$ cat .../tools/go-extract/test/fixtures/clean/conform.toml
# The clean fixture's policy — same topology as dirty, zero findings.
roots = []

[go]
roots = ["."]
cells_dir = "internal/cells"
```

Note also that `max_file_lines` is a **root-table** key, not a `[go]` one — so
«and the file budget» is misplaced as well as the other two being absent, which
the verdict does not separate out.

**Proposed correction (NOT APPLIED):** list the six real keys, add
`exclude_substrings` and `floor_disable` which the brief omits, and move the file
budget to the root table: «…the file budget is the root table's
`max_file_lines`, shared with the other stacks. There is no `[go]`-scoped gating
list — `gated_crates` / `[[exempt]]` are root-table keys and drive the Rust gate
only.»

### `##EVERY-PACKAGE-GATED-OR-EXEMPT` → SURVIVES, **and the verdict understates it**

The verdict says only that «the invariant is stated over `gated_packages` and the
shipped config field is `gated_crates`» — a naming slip. **The invariant is real,
and it never runs on a Go check**, which is the larger half.

It exists, with a test:

```
$ sed -n '259,266p' .../crates/vendor/core-ai-native-conform/src/config.rs
    /// The gated-or-exempt tree invariant: every crate on disk under this
    /// policy's roots is classified exactly once — gated or
    /// exempt-with-a-reason, never both and never neither … 
    pub fn validate_against_tree(&self, root: &Path) -> Result<()> {
```

Its only callers are **Rust**:

```
$ grep -rn "validate_against_tree" --include=*.rs packages crates research \
  | grep -v "/target/" | grep -v "\.vibe/cache" | grep -v "conform/src/config.rs"
.../rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:119
.../rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:188
.../rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/init.rs:335,375   (tests)
.../rust-ai-native-mcp/v0.7.0/…                                                (the twin package)
```

`go-ai-native-conform` never calls it, and neither does
`typescript-ai-native-conform` (in that package the symbol appears only in the
vendored engine's own definition and unit tests). It also reads
`self.gated_crates`, `self.exempt` and `self.roots` — root-table, crate-shaped
fields — and the package's own fixtures set `roots = []`, so there is nothing for
it to classify even in principle.

So the anchor is wrong three ways: the unit is a crate, not a package; it is not
enforced «on every check» for Go at all; and «exactly as for the sibling stacks»
is true of **one** sibling, not two.

**Proposed correction (NOT APPLIED):**

```
##EVERY-PACKAGE-GATED-OR-EXEMPT The gated-or-exempt tree invariant
(`Config::validate_against_tree`) is a CRATE invariant over the root table's
`gated_crates` / `[[exempt]]`, enforced on every **Rust** check
(`rust-ai-native-conform`). The Go frontend does not invoke it and there is no
`[go]`-scoped equivalent — a Go package enters the gate through the census
baseline, not through a gating list. @spec/done
```

**Recommendation per anchor:** all three → **drift stands, correction prepared**;
`##EVERY-PACKAGE-GATED-OR-EXEMPT`'s prepared text must go beyond the verdict's
naming slip or the diff will fix a word and leave the false claim standing.

---

## F-210 — the product client is gone (re-grounded), and «both hops» is one hop

**Outcome:** SURVIVES ×2. The ruling **voids the first verdict's stated ground**
but not its conclusion, which re-verifies on the packages' own trees.
**Anchors:** 2 of 2, both in `go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md`
- `##ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS` (`:34`) → SURVIVES, **on a restated ground**
- `##REPLAY-GOLDENS-PIN-BOTH-HOPS` (`:158`) → SURVIVES

### `##ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS` → SURVIVES, ground restated

The verdict's whole text is «the named artifact **was not located on the wide
perimeter, which includes research/go-demo, the real Go consumer**» — a bare
not-found resting explicitly on the host demo, which the ruling voids twice over
(host evidence, and `research/go-demo` is not «the real Go consumer» for a
package built for external clients).

**The conclusion survives on a ground the ruling admits**, because the artefact
the sentence names is a **vibevm crate**, not a consumer's: «One language-generic
product client (`vibe-tcg`'s `OracleRegistry` link) drives all three relays». Its
existence is a fact about our own shipped surface:

```
$ grep -rn "OracleRegistry\|oracle_registry" --include=*.rs --include=*.go --include=*.ts . \
  | grep -v "/target/" | grep -v "^./refs/"
(no output)
```

Zero source files of any language, in any package. The crate was retired with the
whole multiplexed-product topology —
`spec/modules/vibe-mcp/PROP-026-tcg-tool-family.md` `##TOPOLOGY-RETIRED` and
`##TCG-CRATE-DELETED` (`:42`) — and D5's F-214 recorded the same for the Rust
twin, whose demotion clause is live at
`TCG-PROTOCOL-RUST-v0.1.md#ONE-PRODUCT-CLIENT-DRIVES-BOTH-RELAYS`.

**Per stack, and one difference that must reach whoever writes the diff.** The Go
sentence says «drives **all three** relays» where Rust says «drives **BOTH**
relays», so the two are not word-identical and no shared replacement fits both.
More importantly, **D5's Rust clause credits «one of the two proofs» — the two
`live_chain.rs` tests in `rust-ai-native-mcp` and `typescript-ai-native-mcp`.
There is no Go `live_chain.rs`**: `go-ai-native-mcp/crates/go-ai-native-mcp/tests/`
carries `server_replay.rs` only. So for Go *neither* named proof exists and the
honest clause is shorter and stricter than its sibling's. The surviving posture
is the same one layer over — `pub struct TcgSession`
(`go-ai-native-mcp/crates/go-ai-native-mcp/src/tools_tcg.rs:32`).

### `##REPLAY-GOLDENS-PIN-BOTH-HOPS` → SURVIVES

Package-own throughout, so the ruling does not touch it. The inner hop is real
(`crates/go-ai-native-tcg-bridge/src/client/tests.rs:14 pub(crate) struct Script`;
the unit suite is gopls-free per `lib.rs:5`). The outer hop — defined by this
document as `host ⇄ go-ai-native-tcg serve`
(`:12-16 ##DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR`) — is pinned by nothing: its
entry point `run_serve` (`serve.rs:227`) is called from `main.rs` alone, no test
in any of the three stacks constructs a `{proto, id, op, params}` frame, and no
recorded stream is checked into any of them. Full evidence under **F-280**.

**Proposed corrections (NOT APPLIED):** for the first, mirror D5's F-214 clause
in structure and **not** in text — credit `TcgSession`, name PROP-026's two
anchors for the deletion, and state that on the Go side *neither* pinning
mechanism exists. For the second, the Go transcription of F-280's clause: keep
the inner-hop half, say the outer shape is unpinned.

**Recommendation per anchor:** both → **drift stands, correction prepared.** The
first must carry a **restated reason** — the verdict's own ground is void under
the ruling even though its conclusion holds.

---

## F-270 — the counter does reset, the package's own test says so in its name, and the Rust twin says the same thing and was judged `confirmed`

**Outcome:** SURVIVES — package-internal throughout; the ruling does not touch it.
**Anchors:** 1 of 1 —
`go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md#OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET` (`:123`)

The anchor: «versions never repeat **or reset** within a session (a monotonic
counter per document, never derived from content)». The counter is per-entry in a
map, and clearing an overlay removes the entry:

```
$ sed -n '181,195p' .../crates/go-ai-native-tcg-bridge/src/oracle.rs
    /// `update {file, content|null}` → set/clear an overlay.
    pub fn update(&mut self, rel: &str, content: Option<String>) -> Result<u64, TcgBridgeError> {
        match content {
            Some(text) => self.open_or_update(rel, text),
            None => {
                if self.docs.remove(rel).is_some() { … "textDocument/didClose" … }
                Ok(0)
            }
        }
    }
```

`open_or_update` bumps only on the `Some(doc)` arm of `self.docs.get_mut(rel)`
(`oracle.rs:141-145`); after a `remove`, the next open takes the `None` arm and
starts at 1. **The package's own test states the behaviour in its name** — which
is the ruling's bench at its most direct:

```
$ sed -n '29,30p' .../crates/go-ai-native-tcg-bridge/src/oracle/tests.rs
fn overlay_versions_are_monotonic_and_close_resets() {
```

Implementation and test agree with each other and disagree with the spec line —
which is why this is a `contradiction` with `falsifier: self` rather than a
`reality-mismatch`, and why no consumer, host or external, is needed to settle it.

**Per stack — the third inconsistently-judged family in the batch.**
`rust-ai-native-lang/v0.7.0/spec/rust/mechanisms/TCG-ORACLE-RUST-v0.1.md:118`
`##OVERLAY-RULE-VERSIONS-NEVER-REPEAT` carries **the same sentence** over **the
same code** (`rust-ai-native-tcg-bridge/src/oracle.rs:184` has the identical
`if self.docs.remove(rel).is_some()`) and is judged **`confirmed`**. The
TypeScript bridge has no `docs.remove` equivalent and no matching anchor.

**Proposed correction (NOT APPLIED)** — Go, and the identical text for Rust after
its anchor is re-judged:

```
- ##OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET versions never repeat within an
  overlay's lifetime (a monotonic counter per open document, never derived from
  content); clearing an overlay (`update {content: null}`) closes the document
  and a later reopen starts again at 1 —
  `crates/go-ai-native-tcg-bridge/src/oracle.rs:186-193`, and the bridge's own
  `overlay_versions_are_monotonic_and_close_resets` test; @impl/done
```

**Recommendation:** **drift stands, correction prepared** — and the Rust twin
needs a re-judgement in the same pass, or the family ships two answers to one
question.

---

## F-273 — the floor gloss names four steps and a `build` that does not exist; the package's own README gets it right, and the Rust copy is worse and judged `confirmed`

**Outcome:** SURVIVES — package-internal throughout: a shipped constant against a
sentence about it, corroborated by the package's own README.
**Anchors:** 1 of 1 —
`go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.md#FLOOR-REMAINS-THE-TRUTH` (`:48`)

The anchor: «The floor (`go-ai-native floor` → **gofmt/vet/build/test**) remains
the truth, verbatim.» The shipped step list is seven long and carries no `build`:

```
$ sed -n '28,36p' .../crates/go-ai-native-cli/src/floor.rs
const STEPS: &[&str] = &[ "gofmt", "vet", "tests", "staticcheck", "conform", "specmap", "test-gate" ];
```

The compile rides inside `go test ./...` (`floor.rs:127-129`); there is no
separate build verb. **The package's own README is the correct side of the
contradiction**, and makes the correction a transcription rather than a
judgement:

```
$ sed -n '20,24p' .../go-ai-native-lang/v0.1.0/README.md
  - ##SHIPS-GO-AI-NATIVE-UMBRELLA `go-ai-native` — … `floor` (the seven-step
    verification floor: gofmt → vet → tests → staticcheck+exhaustive → conform →
    specmap → test-gate), …
```

*(Corroboration, not a ground: run against a real Go module the binary prints
`floor: all green (7 step(s) run, 0 disabled by policy)`, EXIT=0, with
`staticcheck ./... && exhaustive ./...`, `go-ai-native-conform check`,
`go-ai-native-specmap --check` and `test-gate` each printing their own header —
so the seven steps are not merely declared in a constant, they execute.)*

An external consumer told the floor is «gofmt/vet/build/test» will not look for
the three discipline steps that actually decide its verdict — which is the whole
point of the sentence.

**Per stack — the Rust copy is the same defect and further off.**
`rust-ai-native-lang/v0.7.0/spec/rust/tools/vibe-agentic-tcg-rust.md:48` reads
«The floor (`rust-ai-native floor` → **cargo check**) remains the truth», and the
Rust floor's shipped steps are `cargo fmt --all --check` (`floor.rs:57`),
`cargo test --workspace` (`:64`), `clippy` (`:78,87`), `conform check` (`:92`),
`specmap --check` (`:99`), `test-gate` (`:111`) and `fast-loop` (`:125`) —
**seven steps, and `cargo check` is not among them.** That anchor is judged
**`confirmed`**. No TypeScript copy exists.

**Proposed correction (NOT APPLIED):** go `:48` → name the seven steps
(gofmt · vet · tests · staticcheck+exhaustive · conform · specmap · test-gate)
and note that the compile rides inside `go test ./...`. rust `:48` → the parallel
seven-step list, **after that anchor is re-judged.**

**Recommendation:** **drift stands, correction prepared** — a transcription from
the package's own README.

---

# The TypeScript stack — judged under the [owner ruling](#ruling)

_`typescript-ai-native-lang` is not adopted by this host and must not be, so
`research/ts-demo`, the host's `crates/` and `.claude/skills/` are **not evidence
about it**. The legitimate bench is the package's own tree and its own tests:
`tools/ts-extract/test/fixtures/{clean,dirty}/` and `tools/ts-extract/test/extract.test.ts`,
`tools/ts-oracle/test/fixtures/proj/` and `tools/ts-oracle/test/oracle.test.ts`,
the ten `crates/**` test modules, and the package's own guide, cards and `vibe.toml`._

---

## F-140 — the largest obligation in the batch, and **all ten anchors are FALSE**: the verdict is one measurement over `research/ts-demo`

**Outcome:** **FALSE ×10** — the whole obligation. Every one of the ten anchors
carries the *same* verdict reason, and that reason is a tool-count over the host's
demo project. Under [the ruling](#ruling) point 1 that is measuring the wrong
consumer, and the obligation falls on that ground alone.
**Anchors:** 10 of 10, all in `typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`
— `##SCAFFOLD-E-PER-CELL-FAST-LOOP` (`:140`), `##SCAFFOLD-G-EXECUTABLE-EXAMPLES` (`:142`),
`##SCAFFOLD-I-CODEMODS` (`:144`), `##EXHAUSTIVENESS-OVER-E-IS-ENFORCED` (`:154`),
`##RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED` (`:209`),
`##REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE` (`:217`),
`##CHARACTERIZATION-GOLDENS-PIN-LEGACY-BEHAVIOR` (`:219`), `##MATRIX-TOOLING` (`:229`),
`##TYPE-LEVEL-TEST-TOOLING` (`:235`), `##RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS` (`:237`).

**The verdict, verbatim and identical on all ten:**

> **restated 2026-07-28, and the first verdict was mine to get wrong.** It was
> confirmed on a count of the named tooling in `research/ts-demo/` that had not
> excluded `node_modules`, `.vibe/cache/` or `vibedeps/` … Measured over the
> demo's own sources only, **every one of the named tools returns zero**: vitest,
> fast-check, expectTypeOf, tsd, Twoslash, ts-morph, jscodeshift, assertNever.
> Its `package.json` devDependencies are `@types/node`, `eslint`, `prettier`,
> `typescript`, `typescript-eslint` and its test script is literally
> `node --test`; its two test files are `src/cells/{farewell,greeting}/index.test.ts`.

**Every clause of that is a measurement of `research/ts-demo`.** There is no
second ground. The obligation was already restated once for a measurement error
inside that same demo (a `node_modules` / `.vibe/cache` leak); the ruling says the
demo was never the bench.

**Perimeter searched (the legitimate one):** the package's own tree for each of
the eight named tools over `*.ts`, `*.json` and `*.rs`, excluding `target/`,
`node_modules/` and `spec/` (so that the guide's own prose could not answer for
itself); plus the package's own test harnesses and fixture `package.json` /
`tsconfig.json`.

**What the ten anchors actually say, and why the demo could not falsify them
even if it were the bench.** They are descriptions of the *TypeScript ecosystem's*
tooling and of rules the discipline imposes on a **consuming** project:

- `##SCAFFOLD-E-PER-CELL-FAST-LOOP` — «`tsc --noEmit` per project … + `vitest` for
  the cell; the agent loop is edit → `tsc --noEmit -p <cell>` + `vitest run <cell>`».
- `##SCAFFOLD-G-EXECUTABLE-EXAMPLES` — «**Twoslash** … `@example` JSDoc blocks …
  `expectTypeOf`/`tsd` for type-level examples». *Rule:* every public seam carries
  ≥1 type-checked example.
- `##SCAFFOLD-I-CODEMODS` — «`ts-morph` / `jscodeshift` codemods for "add a cell," …».
- `##MATRIX-TOOLING`, `##TYPE-LEVEL-TEST-TOOLING`, `##REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE`,
  `##CHARACTERIZATION-GOLDENS-PIN-LEGACY-BEHAVIOR` — the same shape: named,
  publicly available npm packages offered to a consumer, plus a rule.
- `##EXHAUSTIVENESS-OVER-E-IS-ENFORCED`, `##RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED`,
  `##RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS` — pure `*Rule:*` statements.

**Not one of them says «this repository uses X» or «this package ships X».** A
count of what `research/ts-demo` has installed is orthogonal to every one.

**And the package's own bench does not contradict them either** — which is worth
recording, because it is the search that would have mattered:

```
$ for t in vitest fast-check expectTypeOf tsd Twoslash ts-morph jscodeshift assertNever; do
    grep -rl "$t" .../typescript-ai-native-lang/v0.6.0 --include=*.ts --include=*.json --include=*.rs \
      | grep -v "/target/" | grep -v node_modules | grep -v "/spec/"; done
(no output for any of the eight)
```

The package's own harness is deliberately dependency-free — its extractor
contract tests run on the Node built-in runner:

```
$ head -9 .../tools/ts-extract/test/extract.test.ts
/**
 * Contract tests for the ts-extract NDJSON protocol (protocol 1).
 * Run the extractor as a child process over the committed fixture tree —
 * the exact invocation shape the Rust bridge uses …
 */
import { test } from "node:test";
$ grep -n '"test"' .../tools/ts-extract/package.json
7:    "test": "node --test \"test/*.test.ts\""
```

**That is a deliberate posture, not a violation.** A package whose job is to
*specify* a discipline for other people's trees has every reason to keep its own
internal harness free of the very devDependencies it recommends to consumers —
and, per the ruling, its own tests are the bench for *what it ships*, not a
sample of *how a consumer should build*. Nothing in these ten sentences is
falsified by it.

**Per stack:** the Rust guide carries structurally identical scaffold-E/G/I and
matrix/type-level entries naming Rust tooling; the Go guide likewise. **None of
the three is convicted here**, and the Rust twins are not carried across — point
3 of the ruling.

**Proposed correction (NOT APPLIED):** **none — correct as written**, all ten.

**Recommendation per anchor:** all ten `##SCAFFOLD-E-PER-CELL-FAST-LOOP`,
`##SCAFFOLD-G-EXECUTABLE-EXAMPLES`, `##SCAFFOLD-I-CODEMODS`,
`##EXHAUSTIVENESS-OVER-E-IS-ENFORCED`, `##RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED`,
`##REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE`, `##CHARACTERIZATION-GOLDENS-PIN-LEGACY-BEHAVIOR`,
`##MATRIX-TOOLING`, `##TYPE-LEVEL-TEST-TOOLING`,
`##RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS` → **re-judge `confirmed`.** Ten
verdicts, no edit, no spec diff, no owner approval.

**One consequence the boss should see, because it is not this obligation's to
fix.** The *closed* D5 obligation F-168 wrote demotion clauses into this same
document that rest on the same void bench — `##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE`
(`:175`) now reads «*Specified, not built: no such table exists in the stack, in
the host or in `research/ts-demo`…*», and the two `##TIER-*` clauses name the
demo's devDependency list. Those clauses are already published prose in a shipped
package and are host-grounded. Recorded, not touched.

---

## F-161 — five TypeScript facts: one falls with the ruling, two survive on the package's own gate, one is another package's defect, one is a broken cross-reference

**Outcome:** MIXED — 3 SURVIVES · 1 **FALSE** · 1 SURVIVES — ROUTE (b)
**Anchors:** 5 of 5, all in `typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`

| anchor | line | outcome | bench |
|---|---:|---|---|
| `##TSCONFIG-DEFECT-CATCHERS` | 90 | **FALSE** | the verdict is one read of `research/ts-demo/tsconfig.json` |
| `##NO-IF-FLAG-IN-DOMAIN-CELLS` | 167 | SURVIVES | the package's own `build_rules` |
| `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` | 177 | SURVIVES | the package's own `build_rules` |
| `##TOOLING-ASYMMETRY-STATED-HONESTLY` | 258 | SURVIVES — **ROUTE (b)** | `core-ai-native` contradicts itself |
| `##AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` | 274 | SURVIVES | the package's own sibling brief |

### `##TSCONFIG-DEFECT-CATCHERS` (`:90`) → **FALSE**

The verdict, whole: «five flags named as defect-catchers and marked @status:impl/done,
and **the discipline's own demonstration project sets none of them:
`research/ts-demo/tsconfig.json` carries the four mandatory beyond-strict flags
and neither `noUnusedLocals`, `noUnusedParameters` nor
`noFallthroughCasesInSwitch`.** Nothing else enforces them either.»

**That is one read of one host file.** Under the ruling it is measuring the wrong
consumer, and the anchor is in any case a **prescription for a consumer's
tsconfig**, sitting in a list of them:

```
$ sed -n '88,90p' .../spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md
- ##TSCONFIG-STRICT-TRUE `"strict": true` — bundles the eight base flags …
- ##TSCONFIG-BEYOND-STRICT **Beyond `strict` (NOT included, all mandatory here):** …
- ##TSCONFIG-DEFECT-CATCHERS **Defect-catchers:** `noUnusedLocals`, `noUnusedParameters`,
  `noFallthroughCasesInSwitch`, `noImplicitReturns`, `allowUnreachableCode: false`. @impl/done
```

Five real `tsc` compiler options, named as a mandate. Nothing claims any
particular tsconfig in this repository sets them.

**On the legitimate bench there is nothing to contradict it either.** The package
generates no tsconfig — `grep -rn "noUnusedLocals\|noUncheckedIndexedAccess\|exactOptionalPropertyTypes\|noFallthroughCasesInSwitch\|noImplicitOverride"`
over `crates/` returns no output, so `typescript-ai-native init` writes no
compiler options a reader could check the list against. The package's one
`tsconfig.json` is a **language-service test fixture**, not a discipline exemplar
(`tools/ts-oracle/test/fixtures/proj/tsconfig.json`: `"include": ["src"]`,
`"types": []`), and it sets `strict`, `noUncheckedIndexedAccess`,
`exactOptionalPropertyTypes` and `erasableSyntaxOnly` — the beyond-strict tier —
without the defect-catchers, which is what a minimal oracle fixture would do.

**Proposed correction (NOT APPLIED):** none — correct as written.
**Recommendation:** → **re-judge `confirmed`.**

### `##NO-IF-FLAG-IN-DOMAIN-CELLS` (`:167`) and `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` (`:177`) → SURVIVES ×2

Both name **R-001** as the enforcing rule, and both are settled entirely inside
the package — the ruling does not touch them.

```
$ sed -n '48,61p' .../crates/typescript-ai-native-conform/src/lib.rs
pub fn build_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut out: Vec<Box<dyn Rule>> = Vec::new();
    out.push(Box::new(rules::TsUnsafeInDomain));
    if let Some(cells_dir) = &config.typescript.cells_dir {
        out.push(Box::new(rules::TsCellIsolation::new(cells_dir, &config.typescript.seam)));
    }
    out.push(Box::new(rules::FileLength { max_lines: config.max_file_lines }));
    out
}
```

Three rules. `FlagSites` — the rule whose `id()` **is** `"R-001"`
(`crates/vendor/core-ai-native-conform/src/rules/structure.rs:33-35`) — is
exported by the vendored engine (`rules/mod.rs:24`) and constructed **only in
that engine's own tests** (`rules/tests.rs:51`, `:238`). It is never mounted on
this language, and — unlike Rust — **no configuration could mount it**: the TS
`build_rules` has no `registry_file` branch at all, whereas the Rust gate has one
(`rust-ai-native-conform/src/lib.rs:55-63`, gated on `config.registry_file` +
`config.registry_gated_crate`). So `##NO-IF-FLAG-IN-DOMAIN-CELLS`'s «(R-001)»
names a rule the TypeScript gate cannot run, and
`##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED`'s «else `deviates` + reason» clause
has nothing to fire against.

**Per stack — and this correction matters beyond this obligation.** D5's F-168
recorded that `FlagSites` «is constructed **only in the engine's own tests** — a
repo-wide search for `FlagSites {` outside `rules/tests.rs` returns nothing in
any stack or `mcp` package». **That is true of the TypeScript package and false
repo-wide**: the search was package-scoped. The Rust gate mounts it at
`rust-ai-native-conform/src/lib.rs:59`. So R-001 is a *TypeScript-and-Go* gap,
not a family-wide one, and the Rust twins D5 flagged as «knowingly half-demoted»
(`GUIDE-AI-NATIVE-RUST#FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE`, `#TWO-TIERS-OF-FLAGS`)
may be correct as written. Recorded; neither is in this batch and neither was
touched.

**Proposed correction (NOT APPLIED):** for `:167`, qualify the rule id — «(R-001;
the `FlagSites` rule exists in the shared engine and is not mounted on the
TypeScript gate, which registers `ts-unsafe-in-domain`, `ts-cell-isolation` and
`file-length`)». For `:177`, mark the `deviates` clause as the unenforced half.
Rust → **none, pending re-verification of its own twins.**

### `##TOOLING-ASYMMETRY-STATED-HONESTLY` (`:258`) → SURVIVES — **ROUTE (b)**, §3.6(b)

The anchor cites «~74.8% compile-error reduction; ~94% of TS compile errors are
type-level». The verdict's own restatement already concedes the framing was
wrong — «**the verdict stands and its REASON was wrong.** The «~74.8 %» is not a
cache artefact» — and re-measurement confirms what it then found:

- **`~74.8 %` is published by `core-ai-native` v0.8.0 in a live slot** —
  `spec/appendix/CONTRADICTION-MAP.md:28` heads section C-4 «"Type-constrained
  decoding cuts errors **74.8%**" vs "no Rust implementation exists"».
- **The same package's ATLAS gives different figures for the same result** —
  `spec/appendix/ATLAS.md:105` `##FINDING-DR2-012`: «reduces compilation errors by
  **75.3%** (synthesis) and **70.2%** (translation)».
- **The `~94 %` is the corpus's own number, quoted correctly** — ATLAS `:105`:
  «on average 94% of compilation errors result from failing type checks».

So `core-ai-native` disagrees with itself across its own two appendices, and the
TypeScript guide is faithful to one of them. **The defect is not this package's**
— it is a `contradiction` inside `core-ai-native`, and repairing the TypeScript
sentence alone would make the family disagree in a new place. §3.6(b): the
consuming document does not move; a host/other-package obligation is recorded.

**Proposed correction (NOT APPLIED):** none in this package. The obligation
belongs on `core-ai-native/v0.8.0/spec/appendix/CONTRADICTION-MAP.md:28` and
`…/ATLAS.md:105`, which must agree on one figure before any projection is edited.

**Recommendation:** → **drift stands, route (b)** — record a `core-ai-native`
obligation; do not edit this package.

### `##AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` (`:274`) → SURVIVES

Purely package-internal, and settled by reading the two documents:

```
$ grep -n "##AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT" .../GUIDE-AI-NATIVE-TYPESCRIPT.md
274:… (two arms, weak model, mechanical verification — see the sibling brief §6) …

$ grep -n "^## " .../spec/typescript/tools/vibe-agentic-tcg-ts.md
140:## 4. Staged ambition {#staged-ambition}
158:## 5. Licensing posture {#licensing}
169:## 6. The honest risk register {#risk-register}
```

The pointer says §6; §6 is «The honest risk register». The two-arm battery it
means is at `:144` — «MCP + one-shot delivery, measured by the two-arm battery» —
inside **§4, Staged ambition**. A cross-reference two sections off, and the kind
of defect an external consumer meets immediately.

**Proposed correction (NOT APPLIED):** «see the sibling brief §4» — or, better and
rename-proof, cite the anchor rather than the section number, which is the same
lesson D5's F-282 drew from the Go card that carried no tool name.

**Recommendation per anchor:** `##TSCONFIG-DEFECT-CATCHERS` → **re-judge
`confirmed`** · `##NO-IF-FLAG-IN-DOMAIN-CELLS` → drift stands, correction
prepared · `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` → drift stands,
correction prepared · `##TOOLING-ASYMMETRY-STATED-HONESTLY` → **drift stands,
route (b)** · `##AGENTIC-BATTERY-IS-THE-FIRST-MEASUREMENT` → drift stands,
correction prepared.

---

## F-284 — the TypeScript `complete` target has no instrument, on the package's own bench

**Outcome:** SURVIVES — and it is the third member of the three-stack family
reported under F-215. **The ruling does not touch it:** the evidence is this
package's own `bench.rs`.
**Anchors:** 1 of 1 —
`typescript-ai-native-lang/v0.6.0/spec/typescript/mechanisms/TCG-ORACLE-v0.1.md#TARGET-WARM-VALIDATE-AND-COMPLETE` (`:153`)

The anchor is a conjunction: «warm `validate` p50 < 150 ms **and** `complete` p50
< 200 ms on demo-class trees. @status:impl/done». **Half of it is measured and met; half
has no instrument.**

```
$ grep -ci "complete" .../crates/typescript-ai-native-tcg/src/bench.rs
0
$ sed -n '43,49p' .../crates/typescript-ai-native-tcg/src/bench.rs
struct BenchReport {
    …
    cold_init_ms: f64,
    validate_p50_ms: f64,
    validate_p95_ms: f64,
}
```

The harness times `oracle.validate(…)` alone (`bench.rs:100-101`) and emits three
fields (`:132-134`). The `complete` verb ships — `Cmd::Complete` at
`crates/typescript-ai-native-tcg/src/main.rs:59` — so this is a missing
instrument for a real op, not a target on a phantom verb.

**Per stack:** identical in all three; rust `##TARGET-WARM-COMPLETE` and go
`##TARGET-COMPLETE` are F-215 and F-167 above, and the word `complete` is absent
from all three `bench.rs` files. **This is the batch's one family where all three
copies are equally wrong**, which is worth saying because it is the exception to
the release queue's warning rather than an instance of it.

**Proposed correction (NOT APPLIED)** — and unlike its two siblings, **do not
demote the whole anchor**: the `validate` half is measured and comfortably met.

```
- ##TARGET-WARM-VALIDATE-AND-COMPLETE warm `validate` p50 < 150 ms — measured at
  19.3 ms on the committed battery — and `complete` p50 < 200 ms, posted but not
  yet measured: the bench harness times `validate` only
  (`crates/typescript-ai-native-tcg/src/bench.rs` emits `cold_init_ms`,
  `validate_p50_ms`, `validate_p95_ms` and no `complete` field). @spec/done
```

*(The 19.3 ms figure is from the host's `research/tcg-bench/reports/bench-2026-07-07-baseline.json`
— host corroboration, offered because it is the only recorded run of this
package's own harness. If the boss prefers the clause to rest on nothing outside
the package, drop the parenthetical and keep the second half of the sentence.)*

**Recommendation:** `##TARGET-WARM-VALIDATE-AND-COMPLETE` → **drift stands,
correction prepared** — as part of the three-stack family, and with the
`validate` half kept.

---

# Tally

## The sixteen, by outcome

| id | package | anchors | outcome |
|---|---|---:|---|
| `F-140` | typescript-ai-native-lang | 10 | **FALSE ×10** — the verdict is one tool-count over `research/ts-demo` |
| `F-154` | rust-ai-native-lang | 6 | MIXED — 5 SURVIVES (3 rust-only), 1 FALSE PREMISE, DIFFERENT DEFECT |
| `F-161` | typescript-ai-native-lang | 5 | MIXED — 3 SURVIVES, 1 **FALSE**, 1 SURVIVES — ROUTE (b) |
| `F-166` | go-ai-native-lang | 4 | MIXED — 2 SURVIVES, 2 **FALSE** |
| `F-167` | go-ai-native-lang | 4 | SURVIVES ×4 (1 go-only, 2 family-wide) |
| `F-185` | go-ai-native-lang | 3 | SURVIVES ×3 — one worse than the verdict says |
| `F-210` | go-ai-native-lang | 2 | SURVIVES ×2 — the first on a **restated** ground |
| `F-215` | rust-ai-native-lang | 2 | MIXED — 1 SURVIVES (all three stacks), 1 FALSE PREMISE, DIFFERENT DEFECT |
| `F-216` | rust-ai-native-lang | 2 | SURVIVES ×2 — both rust-only |
| `F-270` | go-ai-native-lang | 1 | SURVIVES — the rust twin is identical and `confirmed` |
| `F-273` | go-ai-native-lang | 1 | SURVIVES — the package's own README is the correct side |
| `F-275` | rust-ai-native-lang | 1 | SURVIVES — **RUST ONLY**; both siblings correct |
| `F-279` | rust-ai-native-lang | 1 | SURVIVES — a stale schema path and a pre-rename crate name |
| `F-280` | rust-ai-native-lang | 1 | SURVIVES — the ts copy is the same defect, judged `confirmed` |
| `F-281` | rust-ai-native-lang | 1 | SURVIVES — 6 copies, 3 of them `confirmed` |
| `F-284` | typescript-ai-native-lang | 1 | SURVIVES — third member of the `complete`-target family |

**45 anchors, all 45 examined. Route check: all sixteen `sync-from-code`; none
out of route. No package file was edited, no verdict JSON written, no `git`
command that writes was run.**

## The two counts the brief asks for

**Verdicts that turned out FALSE — the description is right and the anchor should
be re-judged `confirmed` with no edit and no owner approval: 13 of 45.**

| anchor | obligation | why it fell |
|---|---|---|
| `GUIDE-AI-NATIVE-TYPESCRIPT#SCAFFOLD-E-PER-CELL-FAST-LOOP` | F-140 | ruling ①: the whole verdict is a tool-count over `research/ts-demo` |
| `…#SCAFFOLD-G-EXECUTABLE-EXAMPLES` | F-140 | same reason, same verdict text |
| `…#SCAFFOLD-I-CODEMODS` | F-140 | " |
| `…#EXHAUSTIVENESS-OVER-E-IS-ENFORCED` | F-140 | " |
| `…#RULE-BEHAVIORAL-CLAIMS-ARE-MACHINE-CHECKED` | F-140 | " |
| `…#REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE` | F-140 | " |
| `…#CHARACTERIZATION-GOLDENS-PIN-LEGACY-BEHAVIOR` | F-140 | " |
| `…#MATRIX-TOOLING` | F-140 | " |
| `…#TYPE-LEVEL-TEST-TOOLING` | F-140 | " |
| `…#RULE-PUBLIC-SURFACES-CARRY-TYPE-LEVEL-TESTS` | F-140 | " |
| `GUIDE-AI-NATIVE-TYPESCRIPT#TSCONFIG-DEFECT-CATCHERS` | F-161 | ruling ①: one read of `research/ts-demo/tsconfig.json` |
| `GUIDE-AI-NATIVE-GO#BASELINE-RACE-DETECTOR-GATES-TESTS` | F-166 | both grounds void — a package-rooted floor capture, and host non-configuration |
| `GUIDE-AI-NATIVE-GO#RELEASE-MAP-IS-FREE` | F-166 | «no Go binary in this tree» is host evidence; the mechanism demonstrably works |

**Eleven of the thirteen fall to the owner ruling; two would have fallen anyway**
— `##RELEASE-MAP-IS-FREE` is a Go-toolchain property demonstrated with
`go version -m`, and `##BASELINE-…`'s floor evidence is a capture artefact this
campaign has now paid for three times.

**Anchors that survive in one stack while a sibling copy is correct as written:
8 of 45** — the release queue's warning, met eight times:

| anchor | survives in | correct as written in |
|---|---|---|
| `GUIDE-AI-NATIVE-RUST#NAMES-ARE-TOKEN-PROGRAMS` | **rust only** | go (`{Batch}{Planner}` is the Go convention), typescript (clause absent) |
| `GUIDE-AI-NATIVE-RUST#POSITION-IS-A-RESOURCE` | rust (+ ts, unjudged) | **go** — claims only the length half |
| `GUIDE-AI-NATIVE-RUST#SCAFFOLD-B-TYPED-BUILDERS` | **rust only** | go — a different, correct claim about defined types |
| `GUIDE-AI-NATIVE-RUST#SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` | rust (+ ts, unjudged) | **go** — names no custom linter |
| `scaffold-d-differential-oracle.md#card-is-beta` | **rust only** | go, typescript — neither gate mounts `cell-has-oracle` |
| `TCG-ORACLE-GO-v0.1.md#RESOLUTION-GOPLS-ON-PATH` | **go only** | rust — its §1 matches its resolver exactly; ts has no list |
| `rust-ai-native-tcg.md#DERIVED-FROM-THE-EVIDENCE` | **rust only** | typescript — already carries D5's F-168 dead-id clause |
| `rust-ai-native-tcg.md#RUST-AI-NATIVE-TCG-IS-THAT-MISSING-TOOL` | **rust only** | go, typescript — both briefs say «DELIBERATELY HELD AT STUB DEPTH» |

**Six of the eight are Rust-only and one is Go-only** — so the release queue's
«a Go-specific truth stated family-wide» is the *minority* shape in this batch.
The dominant shape is the reverse: a **Rust**-specific falsehood in a sentence
whose Go and TypeScript copies are right, because Rust is the stack this
repository actually consumes and therefore the stack whose sentences were written
against the most machinery.

## The finding that is not in either count: one fact, several verdicts

**Seven families carry one defect across two or three stacks and were judged
differently in each.** In every case the copies not in this batch are `confirmed`
or `unverifiable`, so a correction applied to the batch's copy alone would leave
the family shipping two answers:

| the fact | drift (this batch) | judged otherwise |
|---|---|---|
| replay goldens pin the OUTER hop | rust `TCG-PROTOCOL-RUST#REPLAY-GOLDENS-…`, go `TCG-PROTOCOL-GO#…` | ts `TCG-PROTOCOL-v0.1.md:151 ##REPLAY-GOLDENS-PIN-BOTH-SIDES` → **`confirmed`** |
| the no-zombie property is asserted | rust `vibe-agentic-tcg-rust#RISK-WINDOWS-CHILD-LIFECYCLE`, go `TCG-ORACLE-GO#GRACEFUL-EXIT-…` (+ rust `TCG-ORACLE-RUST#…`, demoted in D5) | go `vibe-agentic-tcg-go#RISK-WINDOWS-CHILD-LIFECYCLE`, ts `vibe-agentic-tcg-ts#…`, ts `TCG-ORACLE-v0.1#SHUTDOWN-IS-THE-ONLY-SANCTIONED-EXIT` → **all three `confirmed`** |
| overlay versions never reset | go `TCG-ORACLE-GO#OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET` | rust `TCG-ORACLE-RUST:118 ##OVERLAY-RULE-VERSIONS-NEVER-REPEAT` → **`confirmed`**, same code |
| the floor gloss | go `vibe-agentic-tcg-go#FLOOR-REMAINS-THE-TRUTH` | rust `vibe-agentic-tcg-rust:48` → **`confirmed`**, and further off (`cargo check` is not a step) |
| a middle-third comment check | rust `GUIDE-AI-NATIVE-RUST#POSITION-IS-A-RESOURCE` | ts `GUIDE-AI-NATIVE-TYPESCRIPT:128` → **`confirmed`**, same conjunction |
| custom lint rules cite the REQ | rust `GUIDE-AI-NATIVE-RUST#SCAFFOLD-F-…` | ts `GUIDE-AI-NATIVE-TYPESCRIPT:141` → **`confirmed`** |
| `R-060` | rust `GUIDE-AI-NATIVE-RUST#DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL` | ts `GUIDE-AI-NATIVE-TYPESCRIPT:231 ##MATRIX-IS-AUTHORED-DATA` → **`unverifiable`** |

**None of the `confirmed` copies was touched** — §3.1 closes an obligation by
editing *and re-judging every anchor in its list*, and an anchor no verdict covers
cannot be re-judged. They are named here so the boss can decide whether each
family is re-judged together or corrected only where a verdict exists.

## Corrections to earlier campaign records that this pass produced

1. **D5's F-168 note on `FlagSites` is package-scoped and wrong repo-wide.** It
   records that the rule «is constructed **only in the engine's own tests**» in
   any stack or `mcp` package. The **Rust** gate mounts it —
   `out.push(Box::new(rules::FlagSites { … }))` at
   `rust-ai-native-conform/src/lib.rs:59`, activated by `registry_file` +
   `registry_gated_crate`. So `R-001` is a TypeScript-and-Go gap, not a
   family-wide one, and the two Rust twins D5 flagged as «knowingly
   half-demoted» may be correct as written.
2. **`harvest/go-ai-native-lang-floor.md` is a package-rooted capture** and its
   failures are a fact about the cwd, not the mechanism. Its own header says so.
   Two verdicts in this batch rested on it; one of them (`##BASELINE-RACE-…`)
   falls entirely.
3. **D5's F-168 demotion clauses now live in the shipped TypeScript guide on a
   bench the ruling voids** — `##FLAG-REGISTRY-IS-TYPED-DATA-WITH-PROVENANCE`
   (`:175`) and the two `##TIER-*` clauses cite `research/ts-demo`'s
   devDependency list and file count as evidence about a package this host does
   not adopt. Published prose, another obligation's, recorded and not touched.
4. **F-190's release-queue note («the verdict is half false») has a sibling
   shape here**: `##TARGET-WARM-VALIDATE-AND-COMPLETE` (F-284) is likewise half
   true — its `validate` clause is measured and met at 19.3 ms against a posted
   150 ms — so its correction must split the sentence rather than demote it.

## What a Phase-E build would close, cheapest first

1. **A `complete`-latency field in the three bench harnesses** re-judges three
   anchors in three packages (F-215, F-167, F-284) — the same two lines each,
   beside the `validate` percentile that already exists.
2. **One no-zombie assertion per bridge** re-judges up to six anchors across the
   three stacks (F-281, F-167, and the three `confirmed` copies). The technique is
   already proven in this repository at
   `packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-pod/tests/loopback.rs:288-299`.
3. **One outer-frame replay test per stack** (`run_serve` over recorded
   `{proto, id, op, params}` frames) re-judges F-280, F-210 and the TypeScript
   copy.
4. **`R-021`, `R-060`, `R-001` and `T-lex`** — four rule/tier ids cited across the
   corpus with no card, no ATLAS entry, or (for `R-001`) no mount on two of three
   languages. Author them or stop citing them.
5. **`DR1-014`** — one dead evidence id, now in one package rather than two
   (F-216; the TypeScript copy already carries its clause).

## Verification

```
$ git status --porcelain -- packages/ crates/ research/ spec/ discipline/ terraform/
(no output)
```

**No package file, no host source file and no campaign state file was modified.**
The only file this task wrote is this one. Two commands were run against a
consumer tree — `go-ai-native floor` (read-only) and one `go build` whose output
went to the session scratchpad, never into the repository — and both left the
tree clean.

**Measurement window.** The batch opened at HEAD `9f79acf1` and closed at
`b61eb191`; six commits landed in between, all of them campaign documents.
`git diff --stat 9f79acf1..b61eb191 -- packages/ crates/ research/ spec/ discipline/ terraform/`
is empty, so no measurement above straddles a change to its subject.
