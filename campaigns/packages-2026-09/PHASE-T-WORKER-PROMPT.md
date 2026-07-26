# Phase T — the packet, for a writer that derives nothing {#root}

<status stage="spec" state="plan" comment="rewritten 2026-07-26 for GLM writers; template, not yet exercised"/>

**Why this file exists.** The writers are **GLM sessions** and, where the
harness offers them, their sub-agents. That is the weak-reader population the
Discipline's own results are about, and the first draft of this file was written
for a reader it will not get: it opened with a `git worktree add` recipe, asked
the writer to orchestrate ten sub-agents, run wave builds, route compile errors,
and perform a perturb-and-restore loop across a whole batch.

**This version moves every derivable decision into the packet and every stateful
loop to the boss.** What is left for the writer is the one thing only it can do:
turn a sentence into an assertion.

Design it sits on: [`PHASE-T-SPEC.md` §13](PHASE-T-SPEC.md#parallel), especially
[§13.0](PHASE-T-SPEC.md#weak-reader) and
[§13.0.1](PHASE-T-SPEC.md#packet-is-the-unit).

---

## 1. The division of labour, in one table {#split}

| | boss (Claude) | writer (GLM) |
|---|---|---|
| triage, packing, scaffold | **yes** | never |
| `git` — worktree, branch, commit, merge | **yes** | **never** |
| `cargo` — any invocation at all | **yes** | **never** |
| choosing the file, the URI, the `r=N`, the test names, the tier | **yes** | never |
| writing the assertion | never | **yes** |
| deciding a fact is not testable | never | **yes**, with the sentence |
| red exhibit — perturb, confirm, restore | **yes**, batched | never |
| reading the diff and gating | **yes** | never |

- ##W-ONE-COLUMN-IS-THE-POINT **The writer's column has two rows.** Everything else was moved out,
  and the move is not politeness toward a weak model — it is the project's own
  doctrine applied to itself: the strong author materialises the structural
  decision, the weak reader fills a named input.
- ##W-NO-DERIVED-IDENTIFIERS **The writer never types an identifier it could get wrong.** The
  `spec://` URI, the revision, the file path and the three test names arrive as
  literals in the packet. A mistyped `verifies` tag produces a test that exists
  and is invisible to every count this phase produces (§11.5) — the most
  expensive single mistake available here, removed by construction rather than
  by warning.

## 2. The packet — one component, filled by the boss {#packet}

One packet = one component = one writer, whether that writer is a whole session
or one sub-agent inside one (§13.0.1). **The packet is self-contained**: a
writer that has read nothing else can execute it.

Slots the boss fills are `<ANGLED>`. Everything else is fixed text and is
pasted verbatim — including §3, which is the phase.

---

> **You are writing Rust tests for the vibevm project. Everything you need is in
> this message. Do not read any other file for instructions.**
>
> **Rules that override anything you might infer:**
>
> - Write **only** these files. A file outside this list is out of bounds:
>   ```
>   <TARGET_FILE_PATHS, one per line>
>   ```
> - **Never run `git`.** Not `add`, not `commit`, not `branch`, not `worktree`.
> - **Never run `cargo`.** Not `build`, not `test`, not `check`, not `fmt`.
> - Never create, delete or rename a file. Every file above already exists.
> - Never edit `Cargo.toml`, any `mod.rs`, anything under `spec/`,
>   `campaigns/`, or `crates/vendor/`.
> - **Never name a model, an agent, or an AI tool** anywhere — not in a comment,
>   not in a test name, not in your report. Hard repository rule.
> - If you think you need something this list forbids: **stop and say so in your
>   report.** Do not work around it. A blocked packet reported is a good outcome.
>
> **You do not build a surface.** If a fact describes a type, a function or an
> interface that **does not exist**, do not create it and do not adapt the fact
> to what does exist. Write the test the fact describes, add
> `#[ignore = "surface does not exist: <what is missing>"]`, keep its `verifies`
> attribute, and say so in your report. That test is the specification of the
> missing work — it is a deliverable, not a failure.
>
> ---
>
> ## Your facts
>
> Each block below is one fact. **Everything in it is a literal — copy, never
> retype and never derive.** Write three tests for each, in the file named.
>
> ```
> FACT <N>
>   file:      <exact/path/to/spec_tests.rs>
>   attribute: #[specmark::verifies("<exact spec:// URI>", r = <N>)]
>   text:      "<the fact's text, verbatim>"
>   surface:   <exact signature(s) the fact is about — names and types only>
>   tests:     canonical_<given_name>
>              boundary_<given_name>
>              negative_<given_name>
>   tier:      <spec_tests.rs | spec_tests_io.rs> · <fast | slow>
> ```
>
> ## How to write each test — the routine, seven steps, five of them yours
>
> ```
> 1. Read the fact. Write ONE sentence: "given <X>, <the thing> <does Y>."
>    Cannot? STOP — return the fact as not testable, and hand back your
>    failed sentence as the reason. That sentence is the deliverable.
> 2. Underline the OBSERVABLE in Y. What could a program look at — a
>    return value, an error variant, a written file, an exit code?
>    Nothing observable? STOP — not testable.
> 3. Write the EXPECTED VALUE as a LITERAL, before running anything.
>    Not "is_ok" — the actual value. Cannot name it from the fact's
>    words? STOP: you are one step from testing the implementation.
> 4. Write the GIVEN as the smallest input that makes the fact apply.
>    Smallest = fewest fields set, shortest string, one element.
> 5. Use the test names given in your fact block above.
>    ---- stop here. Steps 6 and 7 are run by the reviewer, in a batch. ----
> ```
>
> **Step 3 is the whole method.** A literal cannot be copied from a run that has
> not happened, so writing it first is what stops the test from mirroring the
> code. You are not running anything, so there is nothing to copy from — that is
> the design, not a limitation.
>
> **Do not read the implementation body.** Read the signature in `surface`, the
> types, and the fact's text. That is enough, and it is meant to be.
>
> ## Assertion shapes that are banned
>
> Each of these passes whether or not the fact is true. If you write one, the
> packet is rejected.
>
> | banned | why it proves nothing |
> |---|---|
> | `assert!(r.is_ok())` / `is_some()` alone | every fact worth stating says *what* the value is, not that one exists |
> | `assert_eq!(f(x), f(x))` | compares the code to itself |
> | expected value obtained by calling the code under test | the oracle became the implementation |
> | a snapshot created by running the code | that is characterization: it pins current behaviour, bugs included. It never verifies a fact |
> | `assert!(true)`, an empty body, "does not panic" | passes on any implementation, including an empty one |
> | `assert!(err.is_err())` for a `negative` test | the fact names *which* failure; assert that variant, and its message where the fact quotes one |
>
> **An assertion with no literal in it must carry a one-line comment saying why
> a literal is impossible.** No comment, no assertion.
>
> ## The worked pair — read this twice
>
> The fact:
>
> > `@impl` ⇒ `<status stage="impl" state="work"/>` — bare shorthand defaults
> > to `state="work"`, with exactly one exception: `@unknown` ⇒ `state="hold"`.
>
> **Useless — this is what someone who looked at the code writes:**
>
> ```rust
> #[test]
> fn shorthand() {
>     let m = parse_shorthand("@impl").unwrap();
>     assert!(m.state.is_some());   // vacuous: true of every possible state
> }
> ```
>
> It passes if the default is `work`, `hold`, `plan`, or anything else. **The
> fact says which something, and the test does not mention it.**
>
> **Real — three kinds, each read straight out of the fact's own words:**
>
> ```rust
> #[specmark::verifies("spec://vibevm/modules/vibe-progress/PROP-043#SHORTHAND-BARE", r = 3)]
> #[test]
> fn canonical_bare_shorthand_defaults_to_work() {
>     // literal written from the fact, before anything was run
>     assert_eq!(parse_shorthand("@impl"), Ok(Marker::new(Stage::Impl, State::Work)));
> }
>
> #[specmark::verifies("spec://vibevm/modules/vibe-progress/PROP-043#SHORTHAND-BARE", r = 3)]
> #[test]
> fn boundary_unknown_is_the_one_exception_and_defaults_to_hold() {
>     // the fact says "exactly one exception" — that clause IS the boundary case
>     assert_eq!(parse_shorthand("@unknown"), Ok(Marker::new(Stage::Unknown, State::Hold)));
> }
>
> #[specmark::verifies("spec://vibevm/modules/vibe-progress/PROP-043#VOCAB-CLOSED", r = 2)]
> #[test]
> fn negative_a_typo_dies_with_a_nearest_legal_value_hint() {
>     // the fact promises a hint, not merely an error — assert the hint
>     let err = parse_shorthand("@rewrok").unwrap_err();
>     assert!(err.to_string().contains("rework"), "no nearest-legal hint: {err}");
> }
> ```
>
> **The fact's own clauses hand you the kinds.** «defaults to work» is the
> canonical case; «with exactly one exception» *is* the boundary case, named by
> the fact itself; the closed-vocabulary rule beside it supplies the negative.
> You are not inventing cases — you are enumerating the sentence.
>
> **The negative test asserts the promise, not the failure.** `is_err()` would
> have passed on a parser that returned a bare "invalid". The fact said more, so
> the test says more.
>
> **None of the three needed the implementation.** They needed the signature,
> the type names, and the sentence — which is exactly what your fact block gives
> you.
>
> ## Before you submit each test — the hand test
>
> **Cover the implementation with your hand. Can you still say why this
> assertion is right?**
>
> If the only answer is *«because that is what it returned»*, delete the test
> and go back to step 3. If the answer quotes the fact, the test is real.
>
> ## Assert exactly what the fact promises and nothing more
>
> If it promises a hint, assert the hint is present — not the exact sentence.
> An over-specified test pins an implementation detail the fact never claimed,
> and it is the one kind that a legitimate code fix breaks.
>
> ## Your report — this is half the deliverable
>
> 1. Per file: the tests you wrote, by name.
> 2. **Every fact you returned as not testable, with your failed sentence from
>    step 1.** This is expected output, not failure. Many facts are not testable.
> 3. Every test you marked `#[ignore]` because the surface does not exist, and
>    what was missing.
> 4. Every assertion you wrote with no literal, and why a literal was impossible.
> 5. **Every semantic problem you saw and did NOT fix.** A fact that looks wrong
>    is a finding. Do not fix it. A test that will fail against working code is
>    the most valuable thing you can produce.
> 6. Anything in your fact blocks that was wrong — a signature that does not
>    match, a URI that does not resolve, a name that does not exist. Say it with
>    what you observed. **Every batch in this campaign has found an error in its
>    own brief this way.**

---

## 3. What the boss fills, per packet {#fill-in}

| slot | filled from |
|---|---|
| `<TARGET_FILE_PATHS>` | the component's scaffolded files (§13.3) |
| `FACT <N>` blocks | the triage output (§2), one block per T-testable fact |
| `attribute:` | the fact's `spec://` URI and its **current** revision, read from the corpus — never typed twice |
| `surface:` | the signatures, copied out of the code by the boss |
| `tests:` | three names, composed by the boss from the fact's own clauses |
| `tier:` | §7's assignment, decided at packing time |

- ##W-BOSS-TYPES-THE-URI-ONCE **The boss extracts the URI and revision mechanically**, from the same
  source the coverage checker will read (§10.1). A URI typed by hand at either
  end is a silent hole; typed once and copied twice, it either works everywhere
  or fails visibly at the first check.
- ##W-PACKET-SIZE-IS-A-BUDGET **Size a packet by facts, not by files** — the campaign's own ×1.6
  lesson in another costume. Start at **≤8 facts (~24 tests)** per packet until a
  real run says otherwise; the calibration packet (§12) exists to produce that
  number.

## 4. The boss's half — provisioning, integration, gating {#boss}

### 4.1 Before any writer is opened {#provision}

```bash
git -c core.longpaths=true worktree add -b phase-t/<scope> ../vibevm-t-<scope> <SCAFFOLD_SHA>
```

- ##W-LONGPATHS-BELONGS-HERE **The `core.longpaths` flag is not optional and it belongs to this
  step.** `git worktree add` on this repository overflows Windows MAX_PATH on
  the deep `vibedeps/` paths and fails opaquely (F19). It used to sit in the
  writer's prompt, where the one reader who could not have diagnosed the failure
  was the one being asked to handle it.
- ##W-RECORD-THREE-ARTEFACTS **Record three artefacts or the checks have no denominator:** the
  scaffold sha, `scaffold.txt` (every file the scaffold pass created), and one
  `scope-<name>.txt` per packet (its path list, verbatim as pasted). Without
  them «it merged cleanly» is a count with nothing to compare against.

### 4.2 While the writers run {#while}

- ##W-DO-NOT-MOVE-MAIN Do not commit to `main`, do not merge, do not pull. Every worktree
  grew from the scaffold commit and every check compares against it.
- ##W-DO-NOT-HELP **Do not fix a problem a writer reports.** It is a finding and it goes
  in the ledger; fixing it mid-run puts the tree out of step with the work being
  written against it.
- ##W-EXPECT-UNTESTABLE **Expect a large untestable return rate**, and do not treat it as
  under-performance. A returned fact with a stated sentence is a valid output of
  the routine's step 1 — arguably its most honest one.

### 4.3 Verify the split BEFORE integrating {#verify-first}

##W-VERIFY-BEFORE-MERGE The checks run while the trees are still separate. Integrating first
destroys the cheapest evidence — after a clean merge over an overlapping
partition, the tree looks exactly like a correct one.

```bash
S=<scaffold-sha>; D="$(mktemp -d)"
for w in <scope names>; do
  git -C ../vibevm-t-$w diff --name-only "$S" | sort > "$D/$w"
done

# 0. Each worktree still descends from the scaffold commit.
# 1. No file touched by two writers.                    MUST print nothing.
sort "$D"/* | uniq -d
# 2. Each writer stayed inside its OWN declared list.   MUST print nothing.
for w in <scope names>; do comm -23 "$D/$w" <(sort scope-$w.txt); done
# 3. Which scaffolded files were never filled.          Cross-check, not a failure.
sort -u "$D"/* > "$D/all"; comm -13 "$D/all" <(sort scaffold.txt)
```

- ##W-CHECK-2-IS-THE-ONE-MISSED **Check 2 is the one an intersection test alone does not give you.**
  A writer can stray into a file *no other writer* touched: the intersection
  stays empty and the violation is invisible. Only comparing each tree to **its
  own** list catches it. *(This procedure's first draft had check 1 and not check
  2 — the omission is recorded because it is the same shape as everything else
  this campaign has found: a check with no denominator.)*
- ##W-CHECK-3-IS-NOT-A-FAILURE **An unfilled scaffolded file is not automatically wrong.** It is a
  component where every fact came back untestable, and the writer's report says
  so. Reconcile check 3 against those reports **by name**; a file that is empty
  and *not* in a report is the real gap.
- ##W-IF-A-CHECK-FIRES **If check 1 or 2 fires: stop.** The partition was wrong, which means
  the packing was wrong, and that is what gets fixed — not the tree.

### 4.4 The wave build, and then the batched red exhibit {#build-and-exhibit}

Both are the boss's (§13.5), and in this order:

```bash
cargo test --workspace --no-run          # one build for the whole wave
```

Route each compile error back to the writer that owns that path. **A compile
error means its fact block was thin** — a wrong signature, a missing type — and
that is worth recording, not worth a silent fix.

Then the exhibit, once, for the wave:

- ##W-PERTURB-VALUE-NOT-TYPE Perturb **one expected literal in every file at once** — the VALUE,
  never the TYPE (`3` → `4`, not `3` → `"x"`), so each failure stays inside its
  own test instead of breaking the crate.
- ##W-CONFIRM-EXACTLY-THOSE Run once. Confirm **exactly** the perturbed tests failed — not «some
  failed». A file whose perturbed test still passes has a dead assertion, and
  that file is rejected.
- ##W-RESTORE-AND-RECONFIRM Restore all of them, run again, confirm green. **This is the step
  that is easiest to half-finish**, which is why it is held by the one party
  that can see the whole loop.

### 4.5 Integration and the gate {#integrate}

```bash
cargo fmt --all                                 # writers do not format
bash tools/self-check.sh ; echo "EXIT=$?"       # the REAL exit code, never a piped tail
```

- ##W-FIRST-TIME-TOGETHER **This run is the first time the test sets run together.** Disjoint
  *files* do not imply independent *tests*: a shared fixture, a fixed tempdir
  name, a bound port or global state can collide, and nothing earlier could have
  caught it.
- ##W-NEVER-REBASE **Never rebase a writer's branch and never force anything.** Rule 4
  forbids it, and the disjoint file sets mean a rebase buys nothing.
- ##W-THEN-TIER-AUDIT Then the tier audit (§7): compare each test's wall-clock against the
  tier it was **assigned at packing time**. A fast-tier test that is not fast is
  a mis-assignment to fix, not a boundary to redraw.
- ##W-THEN-COVERAGE Then the three coverage checks (§10.1) — file, fact, tag. **The tag
  check is the one that catches a writer whose tests exist and are invisible**,
  and it is the failure this whole packet design is built to make rare.

## 5. If the writers are on different machines {#different-machines}

The design does not change — the split is static and nothing needs a shared
filesystem. Replace each worktree with a clone at the scaffold commit and have
the boss pull each tree back. `main` still moves only through the boss, and §4.3
is unchanged.
