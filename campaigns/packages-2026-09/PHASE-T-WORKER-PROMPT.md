# Phase T — the worker prompt, for an account that cannot see the other one {#root}

<status stage="spec" state="plan" comment="drafted 2026-07-26; template, not yet exercised"/>

**Why this file exists.** Two worker accounts run in parallel and **know nothing
about each other** — native sub-agent orchestration works only *inside* one
account, so there is no coordinator spanning them and no runtime channel between
them. Everything they need must therefore be **decided in advance and baked into
two self-contained prompts**. This is the template; §4 is the fill-in.

Design it sits on: [`PHASE-T-SPEC.md` §13](PHASE-T-SPEC.md#parallel).

---

## 1. What the no-coordination constraint actually changes {#constraint}

- ##W-STATIC-PARTITION-ONLY **The split is static or it does not exist.** Neither account can ask
  «which cells are yours?», so the answer must already be in its prompt as a
  literal path list. §13.2's component packing runs **before** either starts.
- ##W-SCAFFOLD-RUNS-ON-MAIN-FIRST **The scaffold pass is the boss's, on `main`, committed and pushed
  before either account is launched.** It cannot be one of the two — the other
  would have to wait for it, and waiting needs a channel neither has. Both
  prompts then name **the same starting commit**, which is the only
  synchronisation point in the whole design and it happens before parallelism.
- ##W-NEITHER-AGENT-MERGES **Neither agent merges into `main`.** Each commits on its own branch
  and stops. Not because the merge is hard — the file sets are disjoint, so git
  resolves it automatically — but because **§13.5's check requires seeing both
  branches**, and an agent that merges its own can only see one side. The
  verification is the point of the scaffold design; handing it to someone who
  structurally cannot perform it throws the design away and keeps the ceremony.

## 2. The gotcha that would otherwise cost an hour {#worktree-gotcha}

##W-LONGPATHS **`git worktree add` on this repository overflows Windows MAX_PATH** on the
deep `vibedeps/` paths, and the failure is opaque. Provisioning must use
`-c core.longpaths=true`. This is a paid-for fact from the fractality work
(F19), recorded in the host `CLAUDE.md` ledger, and it applies verbatim here.

```bash
git -c core.longpaths=true worktree add -b <BRANCH> <WORKTREE_DIR> <START_COMMIT>
```

A separate worktree is not a nicety: it gives the run its **own `target/`**, so
two accounts on one machine do not contend on the cargo build lock for every
compile.

## 3. The template {#template}

*Fill `<BRANCH>`, `<WORKTREE_DIR>`, `<START_COMMIT>` and the scope list; hand
the result to one account. Nothing else in it varies between the two.*

---

> You are writing tests for the vibevm project, Phase T of campaign
> `packages-2026-09`. You are one of **two** independent workers running at the
> same time. **You cannot see the other and must never wait for it.** Everything
> you need is in this prompt.
>
> **Set up an isolated worktree first, and use the flag — without it the command
> fails opaquely on this repository:**
>
> ```bash
> cd <REPO_ROOT>
> git fetch
> git -c core.longpaths=true worktree add -b <BRANCH> <WORKTREE_DIR> <START_COMMIT>
> cd <WORKTREE_DIR>
> ```
>
> Work only inside `<WORKTREE_DIR>` from that point on.
>
> **Your scope — you may write ONLY these files. Every one already exists and is
> already declared; you fill them, you never create or delete one:**
>
> ```
> <LITERAL PATH LIST, one per line>
> ```
>
> A file outside that list is out of bounds, including any `Cargo.toml`, any
> `mod.rs`, and anything under `spec/`, `campaigns/`, `crates/vendor/`. If you
> believe you need one, **stop and report** — that is a design question and the
> reviewer answers it. Do not add a dev-dependency; if a test needs one that is
> absent, that fact is not testable in this pass and you report it.
>
> **How to write each test — follow the routine, do not improvise.** It is
> `campaigns/packages-2026-09/PHASE-T-SPEC.md` §3.1, and §3.2's banned list and
> §3.3's worked pair are the parts to read twice. In one line: **the fact's text
> is the oracle, the code is not** — you write the expected value as a literal
> *before* running anything, and if you cannot name it from the fact's words,
> the fact is not testable and you say so instead of guessing.
>
> **Do not read the implementation body.** Read the signature, the types, the
> doc comment, and the fact. Before submitting any test, run the hand test
> (§3.4): cover the implementation and say why the assertion is right. If the
> only answer is «that is what it returned», delete it.
>
> **Every `verifies` edge carries the fact's revision** — `#[specmark::verifies("spec://…#FACT", r = N)]`,
> with `N` read from the fact, never invented. This is what makes a later
> correction to that fact **name** the tests it affects instead of leaving
> someone to grep prose for them.
>
> **Assert exactly what the fact promises and nothing more.** If it promises a
> hint, assert the hint is present — not the exact sentence. An over-specified
> test pins an implementation detail the fact never claimed, and it is the one
> kind that a legitimate code fix breaks.
>
> **Three tests per fact, of three different kinds** — `canonical_…`,
> `boundary_…`, `negative_…` (`property_…` where a fact names no failure). Two
> honest kinds and a stated reason beat three where one is invented.
>
> **Per file, one test is exhibited red — but YOU do that in a batch (below),
> not the sub-agents one test at a time.** A file with no red exhibit is not
> accepted. This is not mutation coverage; it is the check that the test runs
> at all.
>
> **Tier every test as you write it**, never afterwards: `#[ignore]` marks the
> slow tier (needs an external tool, a network, or real sample counts); a test
> touching a tempdir or a fixture goes in `spec_tests_io.rs`; everything pure
> stays in `spec_tests.rs`.
>
> **Commit on your branch. Do NOT merge, do NOT push to `main`, do NOT rebase,
> never force anything.** Conventional Commits: `test(<crate>): <why>` under 60
> characters, blank line, a body explaining why. **Never name a model, an agent,
> or an AI tool anywhere** — not in a commit message, not in a comment, not in a
> test name. That is a hard repository rule.
>
> **Verify before you report — you, once, not them:**
>
> ```bash
> cargo fmt --all
> cargo test --workspace          # the wave green after restoration
> ```
>
> **Run this with ten of your own sub-agents, and orchestrate them yourself.**
> You are the lead inside your account; sub-agent orchestration works *within*
> an account, which is exactly the level you are at. Do it like this:
>
> 1. **Split your scope list into ten sub-lists**, by the same rule that
>    produced it: group by cell, never split a cell, and keep cells that share
>    a fact together. Balance by estimated tests (≈ 3 × testable facts), not by
>    file count. **Keep the ten sub-lists** — you need them to verify.
> 2. **Your sub-agents NEVER run cargo. Not once.** Ten agents share one
>    `target/`, and cargo takes an exclusive lock on it — ten concurrent builds
>    do not run in parallel, they queue, and the parallelism you were given
>    evaporates. **They write text; you run the builds.** This costs nothing,
>    because the routine already forbids running before the expected value is
>    written (§3): authoring is build-free by construction, and running is only
>    confirmation.
> 3. **Dispatch ten sub-agents**, each with its own literal path list and the
>    same routine, and each told explicitly: *write the files, run nothing,
>    report what you wrote.* Give each the identical boundary: *write only
>    these files.*
> 4. **You run ONE wave build when they are all back:**
>    `cargo test --workspace --no-run` compiles everything at once. Route each
>    compile error back to the sub-agent that owns that file, by path. Repeat
>    until it compiles. Because their packets carry exact signatures and type
>    names, errors here should be rare — an error means the packet was thin,
>    and that is worth reporting.
> 5. **Then ONE batched red exhibit for the whole wave.** Perturb one expected
>    literal in **every** file at once, run the suite once, and confirm that
>    exactly the perturbed tests failed. Restore all of them, run once more,
>    confirm green. **Every file still gets its exhibit and the check is not
>    weakened** — it costs two builds for the wave instead of two per test.
>    **Perturb the VALUE, never the TYPE** — `3` → `4`, not `3` → `"x"` — so
>    each failure stays inside its own test instead of breaking the crate.
> 6. **Verify at your level before you commit**, exactly as your own prompt is
>    verified above: no file touched by two sub-agents, and each sub-agent
>    inside its own sub-list. A file outside every sub-list is a violation even
>    if no one else touched it.
> 7. **You commit, they do not.** One commit per sub-list, so the history shows
>    which chunk covered what.
>
> **Report:** per file — tests written, kinds used, tier assigned; every fact you
> returned as untestable with the sentence from routine step 1; the red exhibit
> for each file; every semantic problem you saw and did **not** fix; your
> ten sub-lists and the result of step 6's verification; and your branch name
> and commit hashes. **Do not fix a semantic problem** — a fact that
> looks wrong is a finding, and a test that fails against working code is the
> most valuable thing you can produce, not a failure.

---

## 4. What differs between the two prompts {#fill-in}

**Only two things.** Everything above is identical, which is what makes the two
runs independent.

| slot | worker A | worker B |
|---|---|---|
| `<BRANCH>` | `phase-t/batch-a` | `phase-t/batch-b` |
| `<WORKTREE_DIR>` | `../vibevm-t-a` | `../vibevm-t-b` |
| `<START_COMMIT>` | **the scaffold commit — the same sha for both** | ditto |
| scope list | component set A | component set B |

- ##W-BRANCH-NAMES-CARRY-NO-TOOL Branch names carry the **scope**, never the tool. The attribution
  policy forbids model, agent and AI-tool names in branch names as plainly as
  in commit messages.
- ##W-SAME-START-COMMIT **Both start from the same commit, and it is the scaffold's.** Any
  later commit and the two trees disagree about which files exist, which is the
  one thing the scaffold pass was run to guarantee.

## 5. The boss's half — the integration procedure {#boss}

**Before launching either account:** pack the components (§13.2), run the
scaffold pass (§13.3), commit it, push, and **record three artefacts** — the
scaffold sha, `scaffold.txt` (every file the pass created), and `scope-a.txt` /
`scope-b.txt` (the two path lists, verbatim as pasted into the prompts). Those
three are what §5.2 checks against; without them the checks have no denominator
and this whole design collapses into hoping.

### 5.1 Verify BEFORE merging, not after {#verify-first}

##W-VERIFY-BEFORE-MERGE The checks run on the branches, while they are still separate. Merging
first destroys the cheapest evidence — after a clean merge over an overlapping
partition, the tree looks exactly like a correct one.

```bash
S=<scaffold-sha>; A=phase-t/batch-a; B=phase-t/batch-b
D="$(mktemp -d)"

# 0. Both branches exist and both grew from the scaffold commit.
git merge-base --is-ancestor "$S" "$A" || echo "A did not start at the scaffold"
git merge-base --is-ancestor "$S" "$B" || echo "B did not start at the scaffold"

git diff --name-only "$S..$A" | sort > "$D/a"
git diff --name-only "$S..$B" | sort > "$D/b"

# 1. No file touched by both.                       MUST print nothing.
comm -12 "$D/a" "$D/b"

# 2. Each branch stayed inside its OWN declared list. MUST print nothing.
comm -23 "$D/a" <(sort scope-a.txt)
comm -23 "$D/b" <(sort scope-b.txt)

# 3. Which scaffolded files were never filled.       Cross-check, not a failure.
sort -u "$D/a" "$D/b" > "$D/ab"
comm -13 "$D/ab" <(sort scaffold.txt)
```

- ##W-CHECK-2-IS-THE-ONE-I-MISSED **Check 2 is the one an intersection test alone does not give you.**
  A worker can stray into a file the *other* worker never touched: the
  intersection stays empty and the violation is invisible. Only comparing each
  branch to **its own** list catches it. *(This procedure's first draft had
  check 1 and not check 2 — the omission is recorded because it is the same
  shape as everything else this campaign has found: a check with no denominator.)*
- ##W-CHECK-3-IS-NOT-A-FAILURE **An unfilled scaffolded file is not automatically wrong.** It is a
  cell where every fact came back **untestable**, and the worker's report says
  so. Reconcile check 3's output against those reports **by name**; a file that
  is empty and *not* in a report is the real gap.
- ##W-IF-A-CHECK-FIRES **If check 1 or 2 fires: stop. Do not merge.** The partition was wrong,
  which means the scope lists were wrong, which means the next thing to fix is
  §13.2's packing — not the branch. Merging first and sorting it out afterwards
  gives up the one moment where the two sides are still distinguishable.

### 5.2 The merge itself {#merge}

```bash
git checkout main
git merge --no-ff "$A" -m "test(phase-t): integrate batch A"
git merge --no-ff "$B" -m "test(phase-t): integrate batch B"
```

- ##W-NO-FF-ON-PURPOSE **`--no-ff` on both, including the first.** The first would
  fast-forward and the two integrations would then look different in the
  history for no reason but arrival order. Two symmetric merge commits say what
  happened.
- ##W-NEVER-REBASE **Never rebase a worker branch and never force anything.** Rebasing
  rewrites its commits, which the repository's Rule 4 forbids; the disjoint
  file sets mean there is nothing a rebase would buy anyway.
- ##W-CONFLICT-MEANS-CHECKS-LIED **A conflict here means the checks were wrong**, not that the merge is
  hard. Abort it, go back to §5.1, and find out which check should have fired.

### 5.3 After the merge — what only the merged tree can tell you {#post-merge}

```bash
cargo fmt --all
bash tools/self-check.sh ; echo "EXIT=$?"      # the REAL exit code, never a piped tail
```

- ##W-MERGED-RUN-FINDS-INTERACTIONS **This run is not a formality — it is the first time the two test
  sets run together.** Each passed in isolation; together they can collide on a
  shared fixture, a fixed tempdir name, a bound port, or global state. Disjoint
  *files* do not imply independent *tests*, and nothing before this point could
  have caught it.
- ##W-RESTORATION-IS-CHECKED-HERE **It is also what proves every red exhibit was restored.** A worker
  that perturbed an expected literal and forgot to put it back leaves a failing
  test, and this is the run that says so. No separate check is needed.
- ##W-THEN-THE-TIER-AUDIT Then the tier audit (§7 of the spec): run the full suite, compare each
  test's wall-clock against the tier it was **assigned at authoring time**. A
  fast-tier test that is not fast is a mis-assignment to fix, not a boundary to
  redraw.
- ##W-THEN-COVERAGE Then the coverage count for the exit gate: every T-testable fact
  carries **≥3 `verifies` edges of distinct kinds**, kinds read off the
  greppable name prefixes.

## 6. If the accounts are on different machines {#different-machines}

The design does not change — the split is static, so nothing needs a shared
filesystem. Replace the worktree step with a clone at the scaffold commit, and
have each account push its branch. **`main` still moves only through the boss**,
and §5's checks are unchanged.
