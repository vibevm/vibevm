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
`specmap-core/src/generated/` derive from it …). @impl/done». Both paths are
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
  over the length budget. @impl/done». **Correct as written**; judged `confirmed`,
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
Declared test matrices, never 2^n. @spec/done`

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
