# Phase T — every assertion earns three tests {#root}

<status stage="spec" state="plan" comment="drafted 2026-07-26 from the owner's rulings; not ratified"/>

**Placement:** between Phase E (coding) and Phase F (the credibility report).
**Executor:** a swarm of the running harness's **own** subagents (Claude Code,
ZCode — whichever the session runs under). **Not fractality** (owner ruling
2026-07-26, a deliberate exception to plan §2's «no fractality», recorded here
so it is a decision and not a drift).
**Reviewer:** the boss agent, on every packet.

---

## 0. Why the placement is E→F and not anywhere else {#placement}

Phase E is the last phase that **changes code**. Writing tests against code that
drift tasks are still correcting means writing them twice.

Phase F asks the mandate's question — *does the AI-native discipline hold itself
to its own rule?* Today F could only answer it by prose judgement. With T in
front of it, **F reports measured verification coverage per practice**. That is
the whole argument for the phase existing where it does.

## 1. Scope — and the two thirds of the corpus that are not in it {#scope}

Measured 2026-07-26 over the observed corpus (264 files, 10 825 facts):

| segment | files | facts | in Phase T |
|---|---|---|---|
| host `spec/` (18 vibevm crates behind it) | 58 | 4 923 | **yes** |
| `ai-native` packages (7 of 10 carry crates) | 80 | 2 040 | **yes** |
| `world` flows — prompt-only, **no crates at all** | 126 | 3 862 | **no** — §1.2 |
| **total** | **264** | **10 825** | **6 963 in scope** |

### 1.1 Three tests per fact is the ceiling, not the target {#ceiling}

6 963 × 3 = **20 889 tests**, against 2 009 `#[test]` functions in the host
today. That is the **upper bound before triage**, and quoting it as a plan is
the error this campaign has made four times. §2 exists to replace it with a
measurement.

### 1.2 `world` flows are out of scope, and that is not a demotion {#world-out}

*Owner ruling 2026-07-26: option (a), with (b) named as successor.*

A `world` fact is a behavioural contract with no callable surface —
*«the session reads `spec/WAL.md` before doing anything else»*. There is nothing
to invoke and no value to assert. Forcing three tests onto 3 862 such facts
produces **11 586 tests with no oracle**, which is worse than none: it looks
like coverage.

They stay verified by plan §3.1's three-source join (the package's own
artifacts / the host's observed conformance / the installed reality), which was
designed for exactly this genre.

**The successor is named, not deferred into fog.** Some `world` facts *are*
mechanically checkable — §3.1's source 1 («a boot snippet claims a rule; the
protocol document it cites must exist and say what the snippet says it says») is
pure mechanism. So **§2's triage emits a third bucket** — *`world` facts that a
checker could decide* — as the ready-made input to a future checker phase. It
costs nothing beyond the triage that must happen anyway, and it turns prose into
runnable capital, which is the discipline's own second law.

## 2. Step 1 is a triage, and it gates the phase {#triage}

**No test is written until the triage is done.** For every in-scope fact,
classify into exactly one bucket:

| bucket | meaning |
|---|---|
| **T-testable** | a public surface exists whose behaviour the fact constrains |
| **T-untestable** | the fact asserts a *reason*, a *rejected alternative*, a *revisit trigger*, or a design intent — decision-record content, which has no runtime behaviour |
| **T-checker** | mechanically decidable but not by a unit test (file existence, cross-document agreement, config shape) |

The third bucket is §1.2's successor input; the second is expected to be large
— the corpus is full of `##…-why` / `##…-rejected` / `##…-revisit` units by
design, and pretending they are testable is how a swarm produces volume.

**Exit of step 1:** the three counts, recorded. **That number, not 20 889, is
the phase's workload**, and no packet is dispatched before it exists.

## 3. The oracle rule — the single decision the phase's value rests on {#oracle}

A cheap worker told *«write three tests for this assertion»* **and shown the
implementation** will write tests that restate the implementation. They pass
forever and prove nothing. This is not a risk; it is the default outcome.

**The fact's text is the oracle. The code is not.**

A packet gives the worker:

- the fact's **text verbatim** and its `spec://…#ANCHOR` URI;
- the **public signature(s)** the fact governs — types, names, doc comments;
- the crate's public API surface;
- **not the function bodies.**

Two hard rules follow:

- ##ORACLE-EXPECTED-BEFORE-RUN **The expected value is written before anything is executed.** A
  test whose expected value was copied from a run of the code is invalid,
  however green it is.
- ##ORACLE-NOT-DERIVABLE-IS-UNTESTABLE **If the expected value cannot be derived from the fact's text, the
  fact is not T-testable** — the worker returns it to the triage with a
  reason. It does not guess.

## 4. Three tests means three *kinds* {#kinds}

*Owner ruling: three different kinds; more where the value is visible.*

| kind | what it pins |
|---|---|
| **canonical** | the fact holds on the input it is about |
| **boundary** | it holds at the edge of its domain — empty, single, maximum, first/last, off-by-one |
| **negative** | the failure the fact names actually occurs: the wrong input is rejected, and with the error the fact names |

A fact with no stated failure mode replaces the third kind with a **property or
differential test** (scaffold class D), or the worker delivers **two kinds and
says why the third does not exist**. Two honest kinds beat three where one is
invented.

## 5. Falsifiability — sampled, and here is what it is for {#falsifiability}

*Owner ruling: selectively, at least one test seen failing during the writing.*

The purpose is **not** per-test mutation coverage. It is to catch the case where
the test does not work at all — where the thing is not even lexically
expressible and the worker has written something that cannot fail.

##FALSIFY-ONE-PER-PACKET **Per packet, at least one test must be exhibited red.** The cheap
mechanism: after the test passes, perturb its **expected value** to a wrong one,
confirm red, restore, and record both outputs in the packet's report. This
proves the test executes and its assertion is live.

##FALSIFY-STRONGER-AVAILABLE The stronger form — perturb the **code** instead, proving the
test binds to the behaviour and not merely to itself — is available where a
packet's fact is important enough to pay for it. It is not the default.

A packet that reports no red exhibit is **not accepted**.

## 6. Where the tests live {#location}

*Owner question 2026-07-26: a neighbouring file, so it is visible what to run
and what it belongs to, without kilometre-long files. Answer: yes, and the
repository already works this way.*

`crates/progress-core/src/cache/tests.rs` and `src/lib/tests.rs` are already
sibling test modules rather than `#[cfg(test)]` blocks inside the source. Phase
T takes the same shape with **a distinct name**:

```
crates/<crate>/src/<cell>.rs           the cell
crates/<crate>/src/<cell>/tests.rs     hand-written tests, untouched by T
crates/<crate>/src/<cell>/spec_tests.rs   ← Phase T writes ONLY here
```

Four things this buys, and the fourth is the reason:

- a worker **cannot clobber** a hand-written test — it owns one file;
- what to run and what it belongs to is visible from the path;
- no file grows to tens of thousands of lines;
- **the tier boundary already exists**: `spec_tests` versus everything else,
  with no extra bookkeeping. §7 depends on this.

## 7. Tiering — assigned at authoring time, not derived afterwards {#tiers}

*Owner ruling 2026-07-26: «использовать [ярусы]. Но при первом написании будем
делать полное покрытие, долго и мучительно. И тесты сразу делить, к какому они
ярусу относятся.»*

**The second sentence is the one that changes the design.** An earlier draft of
this section had the tiers *derived* after a full timing run. That is wrong for
the same reason everything else in this campaign has been wrong: a derived
classification over 20 000 tests is a second statement with its own writer, and
producing it means re-reading every test. **The tier is a property of the test,
known by whoever writes it, and it costs one token at authoring time.**

- ##T-TIER-AT-WRITE **The worker assigns the tier as it writes the test.** Never a later
  pass, never inferred from a stopwatch.
- ##T-FIRST-PASS-IS-FULL **The first authoring pass is full coverage** — every T-testable
  fact, slowly and deliberately. Tiering governs what runs *when*, never what
  gets *written*.
- ##T-MEASURE-VALIDATES **The timing run still happens, and its job changes**: it no longer
  produces the tiers, it **audits** them. A test in the fast tier that is not
  fast is a mis-assignment to fix, and the run is what finds it.

### 7.1 The three tiers {#tier-defs}

| tier | admits | runs in |
|---|---|---|
| **fast** | pure, in-process, deterministic, no I/O | the dev loop **and** the floor |
| **floor** | touches a tempdir or a fixture; still deterministic and quick | the floor |
| **slow** | property/fuzz at real sample counts, differential oracles, anything needing an external tool (`gopls`, `tsc`, `rust-analyzer`) or a network | on demand and scheduled |

The third tier already has a live precedent: DRIFT-036's probe-guarded steps
filter exactly this class and print what they dropped. Those tests are **slow
tier by nature**, and Phase T should place them there rather than leave them
filtered by a probe.

### 7.2 How tier and kind are carried — and why not in the tag {#carriers}

`#[spec(…)]`'s grammar is **closed**: `<verb> = "<uri>" [, r = N] [, reason = "…"]`.
It takes no arbitrary keys, so `tier =` / `kind =` would be a **grammar change in
a package crate — a release event**, propagated to six packages, for a phase that
has not started. Deferred deliberately; §7.3 records the upgrade path.

Until then both ride the two carriers Phase T already has:

- ##T-CARRIER-VERIFIES **The edge:** `#[specmark::verifies("spec://…#FACT")]` — sugar that
  already exists **specifically for tests** and is shorter than the general
  form. This is the carrier §10's coverage count reads.
- ##T-CARRIER-KIND **The kind:** a test-name prefix — `canonical_…`, `boundary_…`,
  `negative_…`, `property_…`. Greppable, so §10's «three *distinct* kinds»
  becomes a mechanical check instead of a human reading tests. **Without this
  the exit gate is not checkable at all**, which is why it is not optional.
- ##T-CARRIER-TIER **The tier:** `#[ignore]` marks the **slow** tier — cargo-native,
  and `--include-ignored` is the full run. `fast` vs `floor` is the file:
  `spec_tests.rs` and `spec_tests_io.rs` beside the cell (§6), so the dev loop
  selects by target and the split stays visible from the path, which was the
  reason for the sibling file in the first place.

### 7.3 The upgrade path, named so it is a decision and not a drift {#tier-upgrade}

If selecting or reporting by tier/kind **through specmap** becomes needed — a
coverage report broken down by kind, say — the answer is to add `kind` and
`tier` keys to the `#[spec]` grammar, not to build a second index off the
naming convention. That is a release event and it waits for a reason.

**Revisit when:** the naming convention is observed drifting (a test whose
prefix and content disagree), or a report is wanted that a grep cannot produce.

### 7.4 The number to beat {#baseline}

Today's floor, measured 2026-07-26 after DRIFT-036: **154 s warm over 36 steps**,
of which the eleven newest are **30.7 s**. Its longest single suite is
**24.75 s** across 2 009 tests. Cold is materially worse — seven package
workspaces each own a `target/`.

Phase T adds tests of the order of ten times the current count. **The floor
cannot absorb that untiered**, which is the whole reason this section exists.

## 8. Languages — Rust pilots {#languages}

*Owner ruling: Rust first; the other two once it works.*

The carrier exists in all three and is already documented for tests:

| language | carrier | extractor |
|---|---|---|
| Rust | `#[spec(verifies = "spec://…")]` | `specmark` |
| TypeScript | `/** @verifies spec://… */` | `tools/ts-extract` |
| Go | `//spec:verifies <uri> r=<N>` — the guide places it **above Test/Fuzz/Example functions** | `tools/go-extract` |

**Go is blocked until F-086 closes** (DRIFT-036): its packages vendor an engine
byte-identical to the authored v0.7.0 while declaring `^0.8`, so its edges would
be resolved by a tool that cannot see fact grain.

## 9. What the phase must not do {#never}

- ##NEVER-READ-BODY Never show the implementation body to a test-writing worker. §3 is the
  phase's value and this is how it is lost.
- ##NEVER-INVENT-EXPECTED Never let an expected value come from running the code. An
  undeliverable expectation means the fact is not T-testable.
- ##NEVER-FORCE-THREE Never force a third kind that does not exist. Two honest kinds and a
  reason is the correct output.
- ##NEVER-TOUCH-HAND-TESTS Never write outside `spec_tests.rs`. Hand-written tests are not the
  swarm's to edit.
- ##NEVER-WEAKEN-GATE Never weaken an assertion, a golden, or a gate to make a new test
  pass. A conflict is a finding.
- ##NEVER-ACCEPT-WITHOUT-RED Never accept a packet with no red exhibit (§5).

## 10. Exit gate — enumerating this phase's own steps (amendment A1) {#exit}

1. **Triage complete**: every in-scope fact in exactly one of §2's three
   buckets, counts recorded.
2. **Coverage**: every **T-testable** fact carries **≥3 `verifies` edges of
   distinct kinds**, or a recorded exception with its reason. *Checkable
   because §7.2 makes kind greppable — a gate that needs a human to read the
   tests is not a gate.*
2b. **Every test carries a tier**, assigned when it was written (§7.1), and the
   timing run **audits** the assignment rather than producing it: a fast-tier
   test that is not fast is a defect to fix, not a boundary to redraw.
3. **The existing detector is green**: no unit marked `test/done` with zero
   `verifies` edges — `progress-core/src/evidence.rs:61` already computes this
   and has never had input.
4. **Every packet carries its red exhibit** (§5).
5. **The T-checker bucket exists as a file** under `campaigns/<id>/harvest/` —
   the successor phase's input (§1.2).
6. **Timing measured and tiers cut from it**, both recorded (§7).
7. **`baseline.json` written** at phase close (amendment A6).

## 11. Predictions — each naming the step that tests it (amendment A5) {#predictions}

1. **T-untestable is a large fraction of the in-scope corpus — over a quarter.**
   The host tree is dense with decision records by design. *Tested by:* §2's
   triage counts.
2. **The dominant swarm failure is the tautological test**, not the wrong test.
   *Tested by:* the reviewer's sampling — a test that still passes when the
   fact's expected value is perturbed is tautological, and §5's exhibit makes
   the sample cheap.
3. **The full suite exceeds the per-cell budget by more than 10×**, forcing a
   real tier cut rather than a nominal one. *Tested by:* §7 step 1's wall-clock.
4. **At least one fact turns out to be false, not merely untested** — a test
   written from the fact's text fails against working code. *Tested by:* every
   packet, and such a case is a **drift finding**, not a failing test: the fact
   goes back through sync-from-code.

## 12. Prerequisites {#prereqs}

- **Phase E closed.** Tests are not written against code drift tasks are still
  correcting.
- **F-086 closed** (DRIFT-036) — for the Go part specifically.
- **Fact anchors citable from code** — done: DRIFT-032 and DRIFT-034. Until
  those landed, `#[spec(verifies = "spec://…#UPPER-FACT")]` did not compile,
  and this phase was not expressible.
