# Phase T — runbook for the operator {#root}

<status stage="spec" state="plan" comment="drafted 2026-07-26; not yet exercised"/>

**Who this is for:** the person with the two worker accounts. It says what to
open, what to paste, and what not to do. Everything technical lives elsewhere —
the method in [`PHASE-T-SPEC.md`](PHASE-T-SPEC.md), the worker prompt and the
integration procedure in
[`PHASE-T-WORKER-PROMPT.md`](PHASE-T-WORKER-PROMPT.md).

**The shape in one line:** two accounts, ten sub-agents inside each, twenty
writers, and **not one of them writes a file another one touches** — because
every file exists before any of them starts.

---

## Before you open anything {#before}

These are the boss agent's, not yours. **Do not start step 1 until all four
exist**, and the session that produces them says so explicitly:

| artefact | what it is |
|---|---|
| **the scaffold commit sha** | the commit where every test file already exists, empty and declared |
| `scope-a.txt` · `scope-b.txt` | the two path lists |
| `scaffold.txt` | every file the scaffold created — the denominator |
| `prompt-a.md` · `prompt-b.md` | the two prompts, already filled in |

Prerequisites the boss confirms before producing them: **Phase E closed**, and
the triage done (§2 of the spec) — no test is written before the triage says
which facts can carry one.

## Step 1 — account one {#step-1}

1. Open your **first** account's harness in the vibevm repository.
2. Start a **new, empty session**. Not a continuation of anything.
3. Copy **the entire contents of `prompt-a.md`** and paste it as the first
   message. Nothing else — do not add instructions of your own, and do not
   summarise it. The prompt is exact on purpose.
4. Let it run. It will create its own worktree, split its scope across ten
   sub-agents, write, and commit **on its own branch**.

## Step 2 — account two {#step-2}

The same, with **`prompt-b.md`**, in your **second** account.

- ##RB-ORDER-IRRELEVANT **Order does not matter and neither does overlap in time.** Start them
  together, or hours apart. They share nothing and wait for nothing.
- ##RB-NEVER-SAME-ACCOUNT **Never run both prompts in the same account**, even sequentially. Two
  sessions in one account may share a worktree and would then write over each
  other — the isolation is per account, by design.

## Step 3 — while they run {#step-3}

**Do nothing to the repository.** In particular:

- ##RB-NO-MAIN-MOVES **Do not commit to `main`**, do not merge, do not pull anything into
  it. Both branches grew from the scaffold commit and the checks compare
  against it.
- ##RB-NO-HELPING **Do not fix a problem an agent reports.** A reported problem is a
  finding and it goes in the ledger; fixing it mid-run puts the tree out of
  step with the branch that is being written against it.
- ##RB-EXPECT-REDS **Expect to see failing tests in the transcript.** Every file gets one
  test deliberately turned red and then restored — that is the check that the
  test runs at all, and a run with no reds in it is the suspicious one.

## Step 4 — when both have finished {#step-4}

**Come back here and say both are done.** Paste their reports if you have them
easily; the branch names alone are enough if not.

**Do not merge anything yourself.** Not because merging is hard — the file sets
are disjoint and git resolves them automatically — but because the check that
the split actually held **requires seeing both branches at once**, and it runs
*before* the merge. After a merge, a tree where the partition was violated looks
exactly like a tree where it held.

The boss then runs, in this order: the three verification checks on the separate
branches → the two `--no-ff` merges → `tools/self-check.sh` → the tier audit →
the coverage count for the exit gate.

## If something goes wrong {#trouble}

| symptom | what it means | what you do |
|---|---|---|
| `git worktree add` fails oddly | Windows MAX_PATH on the deep `vibedeps/` paths | the prompt already passes `-c core.longpaths=true`; if it was edited out, restore it |
| an agent asks which files are its own | it did not read its path list | re-paste the prompt whole; it is self-contained by design |
| an agent wants to edit `Cargo.toml`, `mod.rs`, `spec/` or `crates/vendor/` | out of bounds | tell it to stop and report; that is a design question for the boss |
| an agent reports many facts as untestable | **probably correct** | nothing — a returned fact with a stated reason is a valid, expected output |
| both agents finished very fast | the triage may have been thin | bring the reports back; the counts will say |

- ##RB-STOPPING-IS-CHEAP **Stopping is always safe.** Nothing merges until you come back, so a
  half-finished branch costs only the work not yet done. There is no state to
  unwind.

## What you never have to decide {#not-yours}

Named so you can ignore them: which files each account gets, how the ten
sub-agents inside an account divide the work, which tests are fast or slow, how
the branches merge. **All of it is computed before step 1 or after step 4.**
Your part is steps 1, 2 and 4.
