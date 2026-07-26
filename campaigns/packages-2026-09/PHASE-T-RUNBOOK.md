# Phase T — runbook for the operator {#root}

<status stage="spec" state="plan" comment="rewritten 2026-07-26 for GLM writers; not yet exercised"/>

**Who this is for:** the person driving the GLM sessions. It says what to open,
what to paste, and what not to do. Everything technical lives elsewhere — the
method in [`PHASE-T-SPEC.md`](PHASE-T-SPEC.md), the packet template and the
boss's integration procedure in
[`PHASE-T-WORKER-PROMPT.md`](PHASE-T-WORKER-PROMPT.md).

**The shape in one line:** the boss prepares everything, each GLM session gets
one self-contained packet and writes only test text — **no git, no cargo, no
merging by anyone but the boss.**

---

## What changed, if you read the first version {#changed}

The first draft asked each session to create its own git worktree, orchestrate
ten sub-agents, run wave builds, route compile errors, and perturb-and-restore
across a whole batch. **All of that moved to the boss.** A GLM session's job is
now: read one packet, write assertions, report. If you find yourself pasting a
`git` or `cargo` command into a session, something has gone wrong.

## Before you open anything {#before}

These are the boss's, not yours. **Do not start until all five exist**, and the
session that produces them says so explicitly:

| artefact | what it is |
|---|---|
| **the scaffold commit sha** | the commit where every test file already exists, empty and declared |
| **the worktrees** | one per packet, already created by the boss |
| `scaffold.txt` | every file the scaffold created — the denominator |
| `scope-<name>.txt` | one path list per packet |
| `packet-<name>.md` | the packets, already filled in |

Prerequisites the boss confirms first: **Phase E closed**; the triage done (§2 of
the spec); the `verifies` extraction proven on one real test (§11.5.1); and
**one calibration packet already run against a real GLM session** (§12) — that
run's deliverable is a corrected packet template, not tests.

## Step 1 — one session per packet {#step-1}

1. Open a **new, empty** GLM session. Not a continuation of anything.
2. Paste **the entire contents of one `packet-<name>.md`** as the first message.
   Nothing else — do not add instructions of your own and do not summarise it.
   The packet is self-contained on purpose.
3. Let it run. It writes files inside the worktree the boss already made, and
   it stops.

- ##RB-ONE-PACKET-ONE-SESSION **One packet per session.** Do not paste a second packet into a
  session that has finished one; open a new one. Two packets in one session is
  how two path lists end up blurred together.
- ##RB-SUBAGENTS-ARE-OPTIONAL **If the harness offers sub-agents, they consume the same packets.**
  One session can take several packets and hand one to each sub-agent — that is
  a scheduling convenience and changes nothing about the work. If it offers
  none, open more sessions. The design does not depend on the answer.
- ##RB-ORDER-IRRELEVANT **Order does not matter and neither does overlap in time.** The
  packets share nothing and wait for nothing.

## Step 2 — while they run {#step-2}

**Do nothing to the repository.** In particular:

- ##RB-NO-MAIN-MOVES **Do not commit to `main`**, do not merge, do not pull anything into
  it. Every worktree grew from the scaffold commit and the checks compare
  against it.
- ##RB-NO-HELPING **Do not fix a problem a session reports.** A reported problem is a
  finding and it goes in the ledger; fixing it mid-run puts the tree out of step
  with the work being written against it.
- ##RB-NO-REDS-TO-SEE **You will NOT see failing tests in the transcripts, and that is
  correct.** Nothing is run during writing — the red exhibit happens later, in
  one batch, on the boss's side. *(The first version told you to expect reds.
  That belonged to a design where the writer ran the suite.)*

## Step 3 — when they have finished {#step-3}

**Come back here and say which packets are done.** Paste their reports if that
is easy; the packet names alone are enough if not.

**Do not merge anything yourself.** Not because merging is hard — the file sets
are disjoint and git resolves them automatically — but because the check that
the split actually held **requires seeing the trees separately**, and it runs
*before* integration. Afterwards, a tree where the partition was violated looks
exactly like one where it held.

The boss then runs, in this order: the three split checks on the separate
worktrees → the wave build → the batched red exhibit → integration →
`cargo fmt --all` → `tools/self-check.sh` → the tier audit → the three coverage
checks for the exit gate.

## If something goes wrong {#trouble}

| symptom | what it means | what you do |
|---|---|---|
| a session runs `git` or `cargo` | it ignored the packet's hard rules | stop it, keep the report, tell the boss — the packet text needs fixing, not the session |
| a session asks which files are its own | it did not read its path list | re-paste the packet whole; it is self-contained by design |
| a session wants to edit `Cargo.toml`, `mod.rs`, `spec/` or `crates/vendor/` | out of bounds | tell it to stop and report; that is a design question for the boss |
| a session reports many facts as untestable | **probably correct** | nothing — a returned fact with a stated sentence is a valid, expected output |
| a session invents a type or function that does not exist | it built a surface, which is forbidden | keep the output, flag it; the test should have been `#[ignore]`d instead, and the boss decides |
| everything finished very fast | the triage may have been thin, or the packets were | bring the reports back; the counts will say |

- ##RB-STOPPING-IS-CHEAP **Stopping is always safe.** Nothing integrates until you come back,
  so a half-finished packet costs only the work not yet done. There is no state
  to unwind.
- ##RB-A-BAD-PACKET-IS-THE-USUAL-CAUSE **When a session misbehaves, suspect the packet first.** It is the
  only thing the session was given. Every batch in this campaign has found a
  factual error in its own brief; the packets will be no different, and a
  session that reports one has done its job.

## What you never have to decide {#not-yours}

Named so you can ignore them: which files each packet gets, how the packets are
sized, which tests are fast or slow, how the trees integrate, when to perturb
anything. **All of it is computed before step 1 or after step 3.** Your part is
steps 1 and 3.
