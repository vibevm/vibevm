# D2 — campaign-plans + comparative-research repairs

_Fourteen `prose-edit` obligations over
`packages/org.vibevm.world/campaign-plans/v0.1.0/` (8) and
`packages/org.vibevm.world/comparative-research/v0.1.0/` (6). Prepared
2026-07-29. The boss reads the diff and decides; nothing here is a closure._

**Route check, run before any edit.** All fourteen carry
`closure_route = prose-edit`, `release_event = false`, `cross_package = false`,
read straight from `campaigns/packages-2026-09/run/state/obligations.json`:

```
$ python -c "...json.load(open('campaigns/packages-2026-09/run/state/obligations.json'))..."
F-133 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-135 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-144 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-155 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-156 | route= prose-edit | release_event= False | cross_package= False | falsifier= mixed
F-173 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-195 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-221 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-222 | route= prose-edit | release_event= False | cross_package= False | falsifier= mixed
F-291 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-292 | route= prose-edit | release_event= False | cross_package= False | falsifier= mixed
F-293 | route= prose-edit | release_event= False | cross_package= False | falsifier= mixed
F-295 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
F-296 | route= prose-edit | release_event= False | cross_package= False | falsifier= host
```

No obligation is out of route by its **field**. Several are out of route by
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side)'s **judgement** — route (b), the
rule is sound and the host does not comply — and those are recorded, not
edited.

**The standing perimeter.** Every absence claim below was searched over the
host tree, not the package: `spec/`, `legacy-spec/` (25 archived terraform
plans + 8 research plans + `discipline/` + `neworder/`), `campaigns/`,
`crates/`, `xtask/`, `tools/`, `discipline/`, `research/`, plus
`packages/**` and `vibedeps/**` where the claim concerns a sibling package.
**Three Phase-C verdicts died on exactly this** — see F-144, F-195 and the
`EXECUTING` finding under F-133 — because their greps were rooted at
`spec/terraforms/ legacy-spec/terraforms/ campaigns/` and never reached
`legacy-spec/research/`, which holds 8 more plan documents.

---

## F-133 — the ledger's own worked anecdote miscounts its source, and eleven sibling anchors are sound rules the host does not keep

**Outcome:** EDITED (1 anchor of 13; 12 recorded OUT-OF-ROUTE — see below)
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\spec\flows\campaign-plans\execution-ledger.md`

**Re-verification:**

The anecdote at `##a-real-chain-anonymized` makes three checkable claims. Two
are false and one is unsupported.

```
$ sed -n '778,800p' legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md
## 10. Deferred, named (not this campaign)

- **vibe-native binary delivery** (install-time build + shims/PATH; a future
  PROP — today's answer is documented cargo, D6).
- **DEBT.md / INTENT.md generated views** (BROWNFIELD §3 names them; …
- **Engine-code consolidation into discipline-core** …
- **`vibe trace` as a product command** (the xtask note) …
- **typescript-ai-native symmetry** …
- **Owner-court**: copying the machine-quirks list into
  `spec/boot/90-user.md` (owner-owned file).
```

Six bullets, not seven — `legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md:782-793`.

```
$ sed -n '22,26p' legacy-spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md
Mandate (owner, 2026-07-07): take everything listed in
`SELF-SUFFICIENCY-PLAN-v0.1.md` §10 (plus the standing `vibe-registry`
line-budget note from CONTINUE.md) and plan its implementation. For
typescript-ai-native symmetry, do NOT wait for the full VibeVM TypeScript
pilot — build a small test demo project instead …
```

The package quotes this as *verbatim* — it is not. The real mandate names the
file and the section and carries a standing extra note; the package's version
generalises both away (correctly, for an anonymized example — but then it is
not verbatim). `legacy-spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md:22-26`.

```
$ sed -n '5p' legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md
**status: EXECUTED · ARCHIVE — do not execute.** … Original: *PLANNED · not
started · the convergence of two audits …*
```

No authoring date anywhere in the status line, so «weeks later» has nothing to
measure against; the follow-up's mandate is dated 2026-07-07 and the source
plan is undated.

**Perimeter searched:** for the undated-source claim —
`legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md` in full (the status
block at :1-20 and §10 at :782-793); the follow-up
`legacy-spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md:1-60`.

**What changed and why:** four surgical corrections inside one fact, all of
them things the evidence falsifies and nothing else. `seven named deferrals`
→ `six`; `each of the seven` → `each of them`; `were, verbatim,` → `were, in
substance,` (the sentence may keep its generalised quote, it may not call it
verbatim); `weeks later` → `later` (the interval has no support). Line count,
line width, marker style and the `##a-real-chain-anonymized` id are unchanged.
I did **not** act on the verdict's fourth sub-claim, that the chain «is not
anonymized in this repository at all» — a shipped package anonymising an
internal anecdote is doing exactly what «anonymized» means, and the source
plans existing under their own names in vibevm's tree does not falsify it.

**The other twelve anchors — OUT-OF-ROUTE, §3.6 route (b).** Every one is a
prescription of the record discipline that the host's two live campaigns do
not currently keep. Softening any of them is the *профанация* §3.6 exists to
prevent, so the package does not move:

- `THE-LINES-TAIL-IS-REFRESHED-AT-EVERY-LATER-BOUNDARY`,
  `THE-STATUS-LINE-MUST-NEVER-LAG-THE-TREE` — the verdict's own words are «the
  rule even supplies the sentence to write» and «the host has both the failure
  and the repair on record». That is route (b) stated in the verdict.
- `A-SUMMARY-BLOCK-IS-PREPENDED-AT-CLOSE`,
  `EXECUTION-RECORD-THE-PER-PHASE-DELTAS`,
  `HONESTY-SAY-NO-TARGET-MOVED-WHERE-IT-IS-TRUE`,
  `RUN-THE-ACCEPTANCE-SCRIPT-BEFORE-WRITING-THE-REPORT`,
  `SUM-AT-CLOSE-PREPEND-THE-EXECUTION-RECORD`,
  `SUM-COMMIT-MAPS-BIND-HASHES-TO-PLANNED-SUBJECTS` — sound rules, no live
  instance. Non-adoption, not falsification.
- `A-FUTURE-READER-NEEDS-THIS-FILE-PLUS-GIT-LOG` — the self-sufficiency
  closure property the whole document exists to buy. The live campaign moving
  its verdicts into `campaigns/packages-2026-09/run/` is a host choice; the
  rule that the ledger must be enough is not repaired by naming more homes.
  Route (b), and worth the owner's eye: the live plan does at least name the
  zone from its own §10 quick-start
  (`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3303-3307`).
- `A-DEFERRAL-LIVES-IN-THE-PLAN-FILE-UNTIL-DRAINED` — **route (c), not (b).**
  The host does the opposite *deliberately and in writing*: `BACKLOG.md:24`
  records that «`ZONE-LIFETIMES` says `run/` is throwaway after close-out»,
  which is why deferrals live outside the disposable zone. §3.6 puts (c) with
  the owner, so this is neither mine to edit nor mine to close.
- `ROW-STATE-EXECUTING`, `SUM-THE-STATUS-LINE-IS-THE-LIFECYCLE` — **the
  verdict's basis is false**; see the perimeter finding recorded under F-144.

**New obligations noticed:**

1. **The `EXECUTING` verdicts rest on a perimeter miss** (affects F-133 anchors
   `ROW-STATE-EXECUTING` and `SUM-THE-STATUS-LINE-IS-THE-LIFECYCLE`, and F-144
   `STATUS-BLOCK-THE-LIFECYCLE-STATE`). Recorded in full under F-144.
2. **The package's execution-record example diverges from its source and I
   deliberately did not touch it.** The fenced block at
   `execution-ledger.md:58-65` says «Two predictions falsified … the other four
   held»; its source
   `legacy-spec/terraforms/SHRINK-PLAN-v0.1.md:7` says «Three predictions
   falsified» and then enumerates two («…the stale-trio premise…, the ≥1/3
   deviates rate…, and nothing else»). The package's copy is *internally
   consistent* (two named, two counted) and the source is the inconsistent one;
   the package also genericises «The WAL carries the full checkpoint» to «The
   journal carries the checkpoint», which is correct for a consumer with no
   WAL. Editing `Two` → `Three` would import the source's own arithmetic bug.
   **The host-side defect is real and belongs to a host task: SHRINK-PLAN's
   execution record says three and lists two.**
3. **The verdict for F-155 undercounts the archive.** It says «five archived
   plans that DO open with a Phase 0»; `grep -rniE '^#+ .*phase 0'` over all
   three plan homes returns **nine** archived plans plus MCP-SOVEREIGNTY's
   equivalent «Wave 0». Recorded under F-155.

---

## F-144 — the lifecycle-vocabulary verdict is false on a perimeter miss, and the quick-start-command verdict is false outright

**Outcome:** RE-JUDGE: confirmed (2 anchors); OUT-OF-ROUTE (6 anchors)
**Files touched:** none

**Re-verification:**

The verdict's load-bearing sentence is «`EXECUTING` occurs nowhere in the
repository as a status». Its stated perimeter was
`spec/terraforms/ legacy-spec/terraforms/ campaigns/` — 27 plan files. There
are **8 more plan documents in `legacy-spec/research/`**, and two of them use
the word exactly as the format prescribes:

```
$ grep -rn '^\*\*status: EXECUTING\|^_Status: EXECUTING\|Original status: \*EXECUTING' spec/ legacy-spec/ campaigns/ --include=*.md
legacy-spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md:5:**status: EXECUTING (2026-07-15) · autonomous full-arc mandate — STUDY → design-doc → Spec 1 → Spec 2 → implementation …**
legacy-spec/research/SETTINGS-HOME-AND-GLOBAL-REGISTRY-PLAN-v0.1.md:5:**status: EXECUTED (finished 2026-07-23) · ARCHIVE … Original status: *EXECUTING (2026-07-20) · owner-directed · decisions LOCKED (§2) …*
```

```
$ ls legacy-spec/research/*.md | wc -l
8
$ ls legacy-spec/terraforms/*.md | wc -l
25
$ ls spec/terraforms/*.md | wc -l
2
```

The second falsification is `QUICK-START-CONFIRM-THE-TREE`. The verdict says
«`grep -rn 'git log --oneline'` across all three plan homes returns **0** — not
one of the 7 archived quick-start blocks confirms the tree that way». It
returns 13, and six of them are quick-start blocks confirming the tree in the
package's own prescribed form, comment and all:

```
$ grep -rn 'git log --oneline' spec/terraforms/ legacy-spec/terraforms/ legacy-spec/research/ campaigns/ --include=*.md
legacy-spec/terraforms/AINATIVE-ANALYSIS-RAID-v0.1.md:279:git log --oneline -1                 # 3227fde — matches the status line
legacy-spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md:387:git -C /c/Users/olegc/gits/vibevm status -sb && git log --oneline -8
legacy-spec/terraforms/HYBRID-LINKING-PLAN-v0.1.md:284:git log --oneline -1                 # a9fdd63 — matches the status line
legacy-spec/terraforms/PACKAGE-TREE-PLAN-v0.1.md:330:git log --oneline -1                    # bf2897b — must match the status line
legacy-spec/terraforms/SETTINGS-SYSTEM-IMPL-PLAN-v0.1.md:119:git log --oneline -1                              # сверить
legacy-spec/terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md:180:git log --oneline -1                              # сверить status line
legacy-spec/terraforms/TREE-TUI-PLAN-v0.1.md:185:git log --oneline -1                 # 6473ecb — matches the status line
  (+ 6 hits in campaigns/packages-2026-09/harvest/, not quick-start blocks)
```

`CAMPAIGN-PLAN-FORMAT.md:207` prescribes
`git log --oneline -3        # tree must match the status line`. Four archived
plans run it with the comment «matches the status line» and two more with the
Russian «сверить status line». The practice is not absent; it is the
document's own sentence, realised.

**Perimeter searched:** all three plan homes and both campaign zones —
`spec/terraforms/*.md` (2 live plans), `legacy-spec/terraforms/*.md` (25),
`legacy-spec/research/*.md` (8), `campaigns/**/*.md`; globs `*.md` throughout.
The Phase-C grep reached the first, second and fourth and **not the third**.

**What changed and why:** nothing. `STATUS-BLOCK-THE-LIFECYCLE-STATE` and
`QUICK-START-CONFIRM-THE-TREE` are **true as written** and are recorded
`RE-JUDGE: confirmed`. What survives of the lifecycle verdict after the
perimeter is repaired — that the archive annotates with SUPERSEDED / CLOSED /
BACKLOGGED, and that the two live plans say RATIFIED and AUTHORED … IN FLIGHT
— is host non-compliance with a sound three-state prescription, and the
document already admits annotation at `CAMPAIGN-PLAN-FORMAT.md:78-79`
(«Owner review may annotate the state»). Route (b).

The remaining six anchors are the same shape and do not move:
`STATUS-BLOCK-THE-TREE-THE-PLAN-WAS-WRITTEN-AGAINST`,
`STATUS-BLOCK-THE-COLD-START-FLAG`,
`BASELINE-AND-EXIT-STATE-ARE-EXACT-COUNTS-THAT-RECONCILE`,
`EVERY-RISK-CARRIES-A-DETECTION-SIGNAL-AND-A-PLAN-B`,
`NON-GOALS-ARE-NAMED-SO-THEY-STAY-VISIBLE`,
`SUM-THE-PLANS-STANDING-OBLIGATIONS`. Each is a required element of the plan
skeleton; each is attested repeatedly in the archive and missing from the two
live plans (`grep -rniE 'written against tree' spec/terraforms/*.md` → 0,
`grep -rn 'cold-executable' spec/terraforms/*.md` → 0,
`grep -rniE 'exit state' spec/terraforms/*.md` → 0,
`grep -rniE '^#+ .*non-goals' spec/terraforms/*.md` → 0). A format document
does not stop requiring a section because the current campaign skipped it.

**New obligations noticed:**

4. **Host task — the two live plans are missing four required §skeleton
   elements.** `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` and
   `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md` carry no «written
   against tree», no cold-executable flag, no exit-state count and no Non-goals
   section, all of which `CAMPAIGN-PLAN-FORMAT.md` §1/§4/§10 require and the
   archive supplies. This is the same shape as the owner ruling already
   recorded at `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3299`
   for the quick-start block — «the rule is sound, and this plan had none» —
   and should be resolved the same way.
5. **Two Phase-C verdicts in this obligation are measurably false and were
   used to open drift rows.** Both are perimeter failures of the kind
   [§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
   names. Worth a sweep of every other Phase-C verdict whose grep was rooted at
   `legacy-spec/terraforms/` alone.

---

## F-155 — six sound phase-gate laws the live campaigns do not keep; the package is not what is wrong

**Outcome:** OUT-OF-ROUTE (§3.6 route (b), all 6 anchors)
**Files touched:** none

**Re-verification:** the verdicts are substantially *right* about the host, and
that is precisely why the package must not move.

```
$ grep -rniE '^#+ .*phase 0' spec/terraforms/*.md legacy-spec/terraforms/*.md legacy-spec/research/*.md
legacy-spec/terraforms/AGENTIC-TCG-RUST-PLAN-v0.1.md:739:## 5. Phase 0 — spikes (no commits; gates for everything after)
legacy-spec/terraforms/AGENTIC-TCG-TS-PLAN-v0.1.md:589:## 5. Phase 0 — spikes (no commits; gates for everything after)
legacy-spec/terraforms/CONVERT-PLAN-v0.1.md:79:## 1. Phase 0 — hygiene and honest ledgers (one sitting)
legacy-spec/terraforms/DEFERRALS-CLOSEOUT-PLAN-v0.1.md:305:## 3. Phase 0 — probes and spikes (no commits; gate for everything after)
legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md:348:### Phase 0 — Baseline + version bump (machinery, no content change)
legacy-spec/terraforms/SHRINK-PLAN-v0.1.md:25:## 1. Phase 0 — hygiene and one-shots (one sitting)
legacy-spec/terraforms/SPECMAP-UNIT-MOBILITY-PLAN-v0.1.md:243:### Phase 0 — Verify + ratify (GATING; spec + facts, no code)
legacy-spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md:349:## 5. Phase 0 — spikes (no commits; a red spike rewrites the decision here first)
legacy-spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md:132:### Phase 0 — SPIKE: proc-macro path-dep across the `exclude` boundary (GATING)
  (+ legacy-spec/terraforms/MCP-SOVEREIGNTY-PLAN-v0.1.md:501 «## 5. Wave 0 — spikes (no commits; gates for everything after)»)

$ for f in spec/terraforms/*.md; do echo -n "$f : "; grep -c 'Phase 0' "$f"; done
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md : 1
spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md : 0
```

Two things follow, and they point opposite ways. **Nine** archived plans open
with a Phase 0 (the verdict said five), six of them stating the no-commits
clause in the heading itself — so the rule is the house style, richly attested.
And the two live campaigns have none — so the host is the non-compliant side.
Where an archived Phase 0 did commit, the plan says so in its own text:
`legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md:374` carries
«**Commits:** `build(packages): bump the discipline packages to 0.3.0`» under
its Phase 0, and `legacy-spec/terraforms/SHRINK-PLAN-v0.1.md:34` records
«baseline 130 → 129 — the `GitBackend` doctest is Phase 0's only shrink».

**Perimeter searched:** `spec/terraforms/*.md`, `legacy-spec/terraforms/*.md`,
`legacy-spec/research/*.md`, `campaigns/**/*.md` — every plan home in the
repository, glob `*.md`.

**What changed and why:** nothing, and this is the decision rather than an
omission. All six anchors are normative laws, not descriptions:
`EVERY-CAMPAIGN-OPENS-WITH-A-PHASE-THAT-PRODUCES-NO-COMMITS` sits under the
heading «Phase 0 — spikes and probes, no commits»;
`ANY-PHASE-BOUNDARY-IS-A-SAFE-STOP` is titled «The safe-stop law»;
`ROW-ELEMENT-COMMIT-SET` and `RESUME-THE-STATUS-LINE-NAMES-THE-STATE` are
required elements; the two `SUM-` anchors restate the first two. Every verdict
in this obligation reduces to «the live campaigns do not do this», which is
§3.6 route (b) exactly: the package does not move, the compliance work is the
host's. The host has already accepted that reasoning once for a sibling anchor
— `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3299-3301` records
the owner ruling «the rule is sound, and this plan had none» and fixed the
*plan*.

**New obligations noticed:** the F-155 verdict's «five archived plans» is nine
(above) — a count error inside a Phase-C verdict, not inside the package.

---

## F-195 — the quick-start verdict is now false: the host already yielded to this exact anchor by owner ruling

**Outcome:** RE-JUDGE: confirmed (1 anchor); OUT-OF-ROUTE (2 anchors)
**Files touched:** none

**Re-verification:** `COLD-A-LITERAL-QUICK-START-BLOCK` — «a literal
quick-start block». The verdict says «`grep -ci 'quick.start'` returns **0 in
both live plans**» and «`grep -rniE 'quick.start' campaigns/packages-2026-09/*.md`
is 0 as well».

```
$ for f in spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md; do echo -n "$f : "; grep -ci 'quick.start' "$f"; done
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md : 6
spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md : 4
```

Six and four, not zero. Both live plans carry the section under its canonical
title, and the wave-2 one says in its own words why:

```
$ sed -n '3297,3310p' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
## 10. Quick-start for the executing session {#quick-start}

*Added 2026-07-29 by owner ruling: `flow:campaign-plans`'
`##COLD-A-LITERAL-QUICK-START-BLOCK` requires it, the rule is sound, and this
plan had none. Every line prints a number — none of them is quoted from here.*

    ```sh
    python campaigns/packages-2026-09/tasks/summary.py         # verdicts by zone; the campaign's headline
    …
    bash tools/self-check.sh; echo "EXIT=$?"                   # the gate panel — 0 before anything
    ```
```

The host read this anchor, ruled the rule sound, and repaired **itself**. That
is §3.6 route (b) run to completion, and it settles the anchor as confirmed.

**Perimeter searched:** `spec/terraforms/*.md` (both live plans, whole files),
`legacy-spec/terraforms/*.md`, `legacy-spec/research/*.md`,
`campaigns/packages-2026-09/*.md`; also
`grep -rniE '^#+ .*quick.start'` over all three plan homes, which returns 17
sections — 2 live, 13 archived terraform, 2 archived research (the verdict
said 7, again a `legacy-spec/research/` miss).

**What changed and why:** nothing. One anchor is now true and is recorded for
re-judgement. The other two do not move:
`PHASE-ZERO-COMMITS-NOTHING-AND-GATES-EVERYTHING-AFTER` and
`BOUNDARY-WRITE-THE-COMMIT-MAP-ENTRY` are the boot snippet's restatements of
the `phase-gates.md` and `execution-ledger.md` laws ruled route (b) under
F-155 and F-133, and the same file carries `##NEVER-COMMIT-DURING-PHASE-ZERO`
at `40-flow-campaign-plans.md:76` — a *Never* rule. Softening a Never because
two campaigns skipped Phase 0 is the failure mode §3.6 names outright.

**New obligations noticed:** the same owner-ruling treatment that fixed the
quick-start gap is the obvious disposition for the four other missing skeleton
elements recorded under F-144, and for the absent Phase 0 / commit maps
recorded under F-155.

---

## F-221 — the host adopted both rules into its own boot contract, then departed from one of them in writing

**Outcome:** OUT-OF-ROUTE (§3.6 route (b) on one anchor, route (c) on the other)
**Files touched:** none

**Re-verification:** the decisive fact is that the host's *own* boot contract
carries this package's snippet verbatim, both rules included:

```
$ sed -n '103,126p' spec/boot/STATIC.md
## At every phase boundary {#boundary}

1. Run the full gate panel; the floor must be green.
2. Write the phase's commit-map entry in the execution ledger —
   hashes, subjects, what each commit confirmed or falsified.
3. Refresh the plan's status line ("Phase N landed, floor green,
   next: Phase N+1").
4. Escalate anything only the owner can decide as a review point:
   OPEN, then RESOLVED with the ruling verbatim.

## Never {#never}
…
- Never carry a deferral outside the plan file — the deferrals
  ledger is where deferrals live, and the next campaign's mandate
  drains from it.
```

`BOUNDARY-REFRESH-THE-STATUS-LINE` — the host agrees with the rule (it is
`spec/boot/STATIC.md:108-109`) and is simply out of compliance: the wave-2
plan's line at `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:5`
says PHASE A OPEN while its LOG at `:1780` records PHASE C OPENED. A rule the
consumer has adopted and then not executed is the definition of route (b).

`NEVER-CARRY-A-DEFERRAL-OUTSIDE-THE-PLAN-FILE` — the host departs
**deliberately, in writing, and cites this flow by name while doing it**:

```
$ sed -n '15,28p' BACKLOG.md
| ##REL-DEFERRALS `campaigns/<id>/deferrals.md` | **one campaign's** tails; dies with the zone | the next campaign's mandate (`campaign-plans` law) |
…
- ##TASKS2-OUTLIVES-THE-ZONE **It lives at the repository root because a campaign zone is
  disposable.** `ZONE-LIFETIMES` says `run/` is throwaway after close-out and
  wave 1's already is. A finding about where the product should go outlives the
  campaign that noticed it.
```

**Perimeter searched:** `spec/boot/**` (the compiled host boot lane, where the
snippet lands), `BACKLOG.md`, `TASKS.md`, `spec/terraforms/*.md` (both live
plans' Deferrals sections — `PACKAGES-ACTUALIZATION:3020` «*(empty)*» and
`SPEC-ACTUALIZATION:1293` «*(empty — drained into `campaigns/<id>/deferrals.md`
at close-out)*»), `campaigns/*/deferrals.md`.

**What changed and why:** nothing. The first anchor is route (b) — the rule is
sound, the consumer adopted it, the consumer lapsed; softening «refresh the
status line» because two plans went stale would delete the only rule that
would have caught it. The second is route (c) and therefore **the owner's**,
not mine: §3.6 says the exception is written down on the host side and the
fact is then re-judged confirmed with the exception named. The host side is
already written (`BACKLOG.md:19` and `:25-28`); what is missing is the
*naming* of it as an exception to this flow, and §3.6 puts that ruling with
the owner «wherever the exception is a policy choice rather than a note». This
is plainly a policy choice — it turns on zone disposability.

**New obligations noticed:**

6. **Route-(c) paperwork, host side.** `BACKLOG.md` and the two live plans
   depart from `##NEVER-CARRY-A-DEFERRAL-OUTSIDE-THE-PLAN-FILE` with a stated
   reason but without marking it as an exception to `flow:campaign-plans`.
   Marking it converts a silent gap into a recorded choice and lets the anchor
   re-judge confirmed. Owner ruling required.
7. **`spec/boot/STATIC.md` carries the `../flows/campaign-plans/…` relative
   links** at `:98-101` — the compiled-lane defect the plan's
   [§4.5](../PHASE-D-BATCH-PLAN.md#release) already tracks as the F-134/F-142
   family. Noted, not touched.

---

## F-291 — "single lifecycle indicator" is false in any project that also runs the progress markup

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\spec\flows\campaign-plans\execution-ledger.md`

**Re-verification:** three carriers of the campaign's lifecycle state exist in
the live campaign, and they do not agree.

```
$ sed -n '1,5p' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
# Packages-Actualization Campaign v0.1 — wave 2: the authored packages measure themselves {#root}

<status stage="impl" state="work" comment="RATIFIED 2026-07-26 with all six §4.5 amendments adopted; Phase A open"/>

**status: RATIFIED 2026-07-26 · PHASE A OPEN · all six §4.5 amendments adopted · wave 2 …**

$ sed -n '1,5p' campaigns/packages-2026-09/run/RESUME.md
# RESUME — campaign `packages-2026-09`

_Generated 2026-07-28T06:05:24Z — do not edit; regenerate with `vibe progress resume`._

**Phase:** C
```

The `<status …/>` element's `comment` restates the lifecycle («Phase A open»),
so it is a second carrier and not merely markup; `run/RESUME.md:5` is a third
and says Phase C. `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md:3` does
the same («plan in execution: A, B, L and C closed; Phase D … opened»), so the
pattern holds across both live plans, not just one.

**Perimeter searched:** both live plans in full
(`spec/terraforms/*.md`, lines 1-6 for the carriers), the generated
`campaigns/packages-2026-09/run/RESUME.md`, and the archived plans
(`legacy-spec/terraforms/*.md:3`) — every one of which carries a `<status …/>`
element on line 3 alongside its prose status line.

**What changed and why:** one word. `single` → `authoritative`. The evidence
falsifies the *count*, not the norm: there really are two or three carriers in
any project that also installs the progress markup, and one of them
(`run/RESUME.md`) is generated and marked «do not edit». Calling the status
line **authoritative** is true, keeps the single-writer discipline that the
rest of the document depends on — a derived restatement that disagrees is then
plainly the derived side's bug — and weakens nothing. I rejected the larger
rewrite (naming the element and the generated view) because §3.6's minimality
rule applies and because naming another package's markup element here would
couple two flows that ship separately.

**Note for the boss, since this one is a judgement:** this can also be read as
route (b) — «the flow says keep one indicator, the host grew three, fix the
host». I ruled (a) because the two extra carriers are not sloppiness: the
`<status …/>` element is a *different installed flow's* required markup and
`run/RESUME.md` is generated by `vibe progress resume`. The sentence was
written before either existed. Reversing this to (b) costs one revert of one
word.

**New obligations noticed:** all three carriers in the live wave-2 campaign
disagree today (element «Phase A open», line «PHASE A OPEN», `run/RESUME.md`
«Phase C», LOG «PHASE C OPENED» at `:1780`). That is the host obligation
already recorded under F-221 and F-133; it is not repaired by this edit.

---

## F-292 — the README promises a deference from a release that has arrived and did not carry it

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\README.md`

**Re-verification:** the claim is «`flow:core-ai-native` ships a campaign-form
document that defers to this package **from its next release**». The next
release has landed and does not defer.

```
$ rg -n 'campaign-plans' packages/org.vibevm.ai-native/
No matches found

$ ls -d packages/org.vibevm.ai-native/core-ai-native/*/
packages/org.vibevm.ai-native/core-ai-native/v0.7.0/
packages/org.vibevm.ai-native/core-ai-native/v0.8.0/

$ grep -n -A3 'core-ai-native' vibe.lock | head
108:name = "core-ai-native"
109-group = "org.vibevm.ai-native"
110-version = "0.8.0"
111:source_url = "file:///C:/Users/olegc/git/v/vibevm/packages/org.vibevm.ai-native/core-ai-native/v0.8.0"

$ for f in .../core-ai-native/*/spec/05-CAMPAIGN-FORM.md vibedeps/flow-core-ai-native/*/spec/05-CAMPAIGN-FORM.md; do …; done
packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/05-CAMPAIGN-FORM.md : 100 lines | campaign-plans=0 defer=0 supersed=1
packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/05-CAMPAIGN-FORM.md : 117 lines | campaign-plans=0 defer=0 supersed=1
vibedeps/flow-core-ai-native/0.8.0/spec/05-CAMPAIGN-FORM.md                  : 100 lines | campaign-plans=0 defer=0 supersed=1
```

The single `supersed` hit is `…/05-CAMPAIGN-FORM.md:36`, «the rule that the
project's living state supersedes» — about the WAL, not about this package.

**Perimeter searched:** **both** authored version slots
(`packages/org.vibevm.ai-native/core-ai-native/v0.7.0/**` and `v0.8.0/**`,
every file, via ripgrep — not just the campaign-form document), the installed
copy `vibedeps/flow-core-ai-native/**`, and `vibe.lock` for which slot a
consumer actually reads. Zero occurrences of the string `campaign-plans`
anywhere in the `org.vibevm.ai-native` group.

**What changed and why:** the sentence now states what is true — that
`flow:core-ai-native` ships its own `spec/05-CAMPAIGN-FORM.md` restating part
of this format, and that the two are not reconciled — instead of promising a
deference that a shipped release was supposed to carry and does not. The
document is named in prose, **not** linked: a `../../…` path into another
package resolves in this dev tree and in nothing a consumer installs.
`##THIS-PACKAGE-IS-THE-CANONICAL-HOME-OF-THE-FORMAT`, the normative claim
directly above, is untouched — it is the section's point and it is sound.

**The anchor id no longer matches its prose**
(`##core-ai-native-defers-from-its-next-release` now leads a sentence about
non-reconciliation). I did not rename it: `RULE-ANCHORS-IMMUTABLE` and this
brief both forbid changing the addressable set, and an id is an address, not a
claim. Flagging it so the boss sees it deliberately rather than as an
oversight.

**New obligations noticed:**

8. **The duplication itself is unresolved and lives in the other package.**
   `core-ai-native`'s `spec/05-CAMPAIGN-FORM.md` restates the artifact-role
   table this package owns. Closing that is an edit to `core-ai-native` plus a
   version bump plus `cargo xtask sync-engines` — a release event, outside
   both this obligation's route and my perimeter.
9. **`vibedeps/flow-core-ai-native/0.8.0/spec/05-CAMPAIGN-FORM.md` is 100
   lines where the authored `v0.8.0` is 117.** The installed slot is labelled
   0.8.0 but carries the v0.7.0 content of this file. Phase D's
   [§3.5](../PHASE-D-BATCH-PLAN.md#vendored) says installed copies already
   differ everywhere because of Phase B markup and carries no drift signal —
   but a 17-line gap on a file whose Phase B markup is anchor prefixes is
   worth one look before anyone trusts `vibedeps/` for this package.

---

## F-293 — "each section with a worked mini example" is ten of fifteen, and the verdict's own count was off by one

**Outcome:** EDITED
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\README.md`

**Re-verification:** I counted it fence-aware, because a naive count is fooled
by the `### D4 — …` heading *inside* §6's fenced example.

```
$ python  # enumerate '### N — ' headings, skip lines inside ``` fences, count fenced blocks per section
15-section skeleton, fence-aware:
  § 1 Title and status line                          lines 68-86  fenced_example=1
  § 2 Execution record (prepended at close)          lines 87-98  fenced_example=0
  § 3 The mandate                                    lines 99-112  fenced_example=1
  § 4 Target arithmetic                              lines 113-126 fenced_example=1
  § 5 Current-state facts                            lines 127-142 fenced_example=0
  § 6 Decisions D1–DN                                lines 143-162 fenced_example=1
  § 7 Predictions                                    lines 163-170 fenced_example=0
  § 8 Phases                                         lines 171-179 fenced_example=0
  § 9 Risks and fallbacks                            lines 180-187 fenced_example=0
  §10 Non-goals                                      lines 188-197 fenced_example=0
  §11 Quick-start for the executing session          lines 198-211 fenced_example=1
  §12 Whole-campaign acceptance                      lines 212-222 fenced_example=1
  §13 Review points                                  lines 223-229 fenced_example=0
  §14 Execution ledger                               lines 230-235 fenced_example=0
  §15 Deferrals ledger                               lines 236-244 fenced_example=0

WITH fenced example  : [1, 3, 4, 6, 11, 12] -> 6
WITHOUT fenced example: [2, 5, 7, 8, 9, 10, 13, 14, 15] -> 9
```

«Canonical fifteen-section» is **exact** and stays: fifteen `### N — ` headings,
anchors `{#s1-status}` … `{#s15-deferrals}`.

Of the nine without a fence, four carry a worked example inline in prose —
§5 the stale-trio (`CAMPAIGN-PLAN-FORMAT.md:136-141`), §7 «P3 — fewer than 10
test expectations break…» (`:166-167`), §9 «flaky network: re-probe…»
(`:182-184`), §10 «does NOT extend the gate to the two remaining modules…»
(`:194-196`). **Five carry no example of any kind: §2, §8, §13, §14, §15.**

**This differs from the verdict, which said four.** The verdict listed §15
among the seven fenced sections; §15 (`:236-244`) has no fence and no inline
example — two bullets and a pointer to `execution-ledger.md §deferrals`, where
the worked deferral example actually lives. So the true tally is **ten of
fifteen**, not eleven. The verdict's conclusion — that «each» is false — holds;
its arithmetic did not.

**Perimeter searched:** the whole of
`packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md`
(284 lines), every `### N — ` section and every fenced block, plus a read of
each of the nine unfenced sections for inline examples.

**What changed and why:** one word of quantifier. `each section with a spec and
a worked mini example` → `each section with a spec and most with a worked mini
example`. «Each» is what the evidence falsifies; the fifteen-section count, the
five roles and the rest of the bullet are untouched. I chose «most» over a
literal «ten of fifteen» deliberately: a README that hard-codes a count of
examples in a sibling document mints a new drift row the first time anyone adds
one, which is the defect this whole campaign exists to drain.

**New obligations noticed:**

10. **Five of the skeleton's fifteen sections carry no example** — §2 Execution
    record, §8 Phases, §13 Review points, §14 Execution ledger, §15 Deferrals
    ledger. Four of them are legitimately pointers to sibling documents that do
    carry the example; §15 is the odd one, since it duplicates the deferrals
    spec that `execution-ledger.md:151-169` already worked. Not a defect I was
    asked to repair, and adding examples would change the document rather than
    correct it — recorded for whoever decides whether the README's promise or
    the document should move.

---

## F-135 — the genre definition admits one of the five studies that name it; the five laws themselves are not what is wrong

**Outcome:** EDITED (1 anchor of 11; 10 recorded OUT-OF-ROUTE)
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\comparative-research\v0.1.0\spec\flows\comparative-research\COMPARATIVE-RESEARCH-PROTOCOL.md`

**Re-verification:** the edited anchor says a research document is a study **of
one external system**. Four of the five documents that name this flow are not
that, and each names the flow in its own header:

```
$ for f in <the five located studies>; do sed -n '1,12p' "$f"; done
legacy-spec/research/PROP-004-tessl-comparative-research.md:1
  # PROP-004 — Tessl comparative research and vibevm roadmap deltas     ← ONE system

legacy-spec/research/action-systems-vscode-idea.md:1
  # Action systems in VSCode and IntelliJ IDEA — a comparative study {#root}
  :5  **Genre:** research (comparative-research flow) — non-binding, evergreen.   ← TWO systems

legacy-spec/research/settings-system-vscode-idea.md:1
  # Comparative research: системы настроек VSCode vs IntelliJ — clean-room study …
  :6  Comparative-research genre: quote-first, two-way gaps, deltas-not-decrees.
  :9  **Subject A:** VSCode …  :10 **Subject B:** IntelliJ Platform …            ← TWO systems

packages/org.vibevm.fractality/…/spec/refs/notes/RLM-SYNTHESIS.md:1
  # RLM-SYNTHESIS — what the field knows, what fractality takes {#root}
  :4  form per D-R6 (flow:comparative-research: two-way gaps, numbered
  :6  deltas, re-fetch list). Synthesized … from the 11 study notes …            ← A FIELD

packages/org.vibevm.fractality/…/spec/refs/notes/FUGU-SYNTHESIS.md:1
  # FUGU-SYNTHESIS — what Sakana's orchestrator teaches the fabric {#root}
  :5  Sources: the four study notes …                                            ← A FIELD
```

**Perimeter searched:** `rg -i 'comparative.research' -l` over
`legacy-spec/research/`, `packages/org.vibevm.fractality/`, `spec/`,
`campaigns/`, glob `*.md` — every document in the repository that names the
flow, plus the fractality per-source notes directory
(`packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/`, 22
files). The one-system reading survives only in the Tessl study and in the
per-source notes, where `D-R7 — one project, one note` makes it an explicit
local decision rather than the genre's definition.

**What changed and why:** the definition now admits what the genre actually
is — one external system, **or, at the same rigor,** a small set compared side
by side, or a synthesis over several such studies. Nothing else in the file
moved. This is not a softening: «at the same rigor» is doing load-bearing work,
the five laws are untouched, and the document's own Law 3 already presumes a
second subject (your project). The clause I removed was a *subject count*, not
a quality bar.

**The other ten anchors — OUT-OF-ROUTE.** All ten are the five laws or their
lead-ins: `EVERY-DOCUMENT-OBEYS-ALL-FIVE-LAWS`,
`LAW-ONE-THE-DOCUMENT-MUST-STAND-WHEN-ITS-SOURCES-VANISH`,
`LAW-ONE-QUOTE-VERBATIM-WITH-AN-ACCESS-DATE`,
`LAW-ONE-A-FUTURE-READER-RECONSTRUCTS-WITHOUT-FETCHING`,
`LAW-TWO-PRESENT-THE-QUOTE-THEN-ANALYZE`,
`LAW-TWO-QUOTE-THEN-JUDGE-EVERY-TIME`,
`LAW-FOUR-FINDINGS-BECOME-NUMBERED-PRIORITIZED-HOMED-DELTAS`,
`LAW-FIVE-THE-DOCUMENT-CLOSES-WITH-THE-RE-FETCH-LIST`,
`REFETCH-IN-WHAT-ORDER`, `REFETCH-WHAT-VERSION-THIS-CAPTURE-REFLECTS`. They
split two ways and neither way is mine:

- **Route (c)** for the two quoting laws — the host has an **owner directive**
  against recording source text, with the rationale recorded verbatim. See
  F-295 / F-296 below for the citations. Softening Law 1 or Law 2 here would
  strip the quote-first discipline out of every consumer's copy in order to
  match one host's legal posture.
- **Route (b)** for the rest — sound rules, partially or wholly unpractised
  (priority on 3 of 5 delta sets, subject-version on 2 of 4 re-fetch lists, a
  re-fetch *order* on 0 of 4). Non-adoption, not falsification.

**New obligations noticed:**

11. **`LAW-FIVE-THE-DOCUMENT-CLOSES-WITH-THE-RE-FETCH-LIST` contradicts
    `from-research-to-roadmap.md`'s `A-RESEARCH-DOCUMENT-ENDS-IN-A-DELTA-TABLE`
    and the template's own §-order.** Two of them cannot both be last. Repaired
    on the roadmap side under F-156 below; recording it here because the
    obligation registry has these on two different rows and a reader of either
    alone would not see the collision.

---

## F-156 — the package tells itself two different things about what a study ends with

**Outcome:** EDITED (1 anchor of 6; 5 recorded OUT-OF-ROUTE)
**Files touched:** `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\comparative-research\v0.1.0\spec\flows\comparative-research\from-research-to-roadmap.md`

**Re-verification:** this one needs no host at all — the package contradicts
itself three ways, inside three files that ship together.

```
$ sed -n '18p' packages/org.vibevm.world/comparative-research/v0.1.0/spec/flows/comparative-research/from-research-to-roadmap.md
##A-RESEARCH-DOCUMENT-ENDS-IN-A-DELTA-TABLE A research document ends in a table of numbered deltas. @impl/done

$ sed -n '139,141p' packages/org.vibevm.world/comparative-research/v0.1.0/spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md
##LAW-FIVE-THE-DOCUMENT-CLOSES-WITH-THE-RE-FETCH-LIST The document closes with **every source URL, its access date, and
the subject's version at capture** — the exact list needed to refresh
the study later. @impl/done

$ sed -n '44,46p' packages/org.vibevm.world/comparative-research/v0.1.0/spec/flows/comparative-research/research-template.md
**Reading shape.** §1 the subject in its own words · §2 capability
inventory · §3 where we trail · §4 where we lead · §5 numbered
deltas · §6 open questions · §7 re-fetch list.
```

The delta table cannot both «end» the document and sit at §5 of seven, and
Law 5 says the re-fetch list closes it. The package's own copy-ready skeleton
sides with Law 5. The host corpus agrees with the skeleton: Tessl's §6 deltas
are followed by §7, §8 and §9; RLM-SYNTHESIS's §3 deltas by §4 and §5.

On «table»: only `legacy-spec/research/action-systems-vscode-idea.md:725`
is literally one (`| Δ | Delta | Answers | → prospective PROP-039 REQ |`). The
other four are numbered lists — `**D-01 [P0, …]**`, `**RD-1 (P1) …**`,
`**FD-1** …`, `### M1.7 — …`. The package's own Law 4 asks only for «numbered
roadmap deltas, each with a priority and a target home» and says nothing about
a table, so «table» was over-specified against its own law too.

**Perimeter searched:** all four files of
`packages/org.vibevm.world/comparative-research/v0.1.0/` (the contradiction is
package-internal), plus the five located studies in `legacy-spec/research/` and
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/` for the
shape and position of their delta sets.

**What changed and why:** `ends in a table of` → `carries a table or list of`.
Two words, both falsified — one by the package's own Law 5 and skeleton, one by
four of five studies. The pipeline diagram, the three verdicts and everything
downstream are untouched; the sentence's job was to say a delta set exists, and
it still does.

**The other five anchors — OUT-OF-ROUTE, route (b).** `ROW-VERDICT-REJECT` (no
delta anywhere has been rejected — the branch is unwalked, which does not
falsify a three-verdict pipeline), `ROW-VERDICT-DEFER`,
`AN-ACCEPTED-DELTA-LANDS-AS-A-RECORDED-DECISION`, `RECORD-FIELD-WHY` and
`RECORD-FIELD-WHEN-TO-REVISIT`. Four of the five fail on the same missing
thing: **no accepted delta in this repository carries a revisit trigger.**
`grep -rn 'When to revisit'` over `ROADMAP.md`, `spec/`, `legacy-spec/`,
`campaigns/` and the fractality tree hits only inside the
`decision-records` package that *defines* the field. That is a host gap in a
field a sibling package owns — deleting the requirement from this package would
make the discipline describe the lapse.

**New obligations noticed:**

12. **The revisit-trigger gap is a host obligation spanning two flows.**
    `flow:decision-records` defines «When to revisit — a trigger: metric +
    threshold + where it is observed»
    (`packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md:78`),
    this flow requires it on every accepted delta, and zero landings carry one.
    The nearest instance, fractality's INVENTORY S8 «too young to adopt now»,
    names no trigger.
13. **The reject branch of the pipeline has never been walked** — zero
    `Verdict: REJECTED` deltas repository-wide, against three fractality study
    notes that keep `## Non-adoptions (named)` sections. Those are declined
    *source ideas*, not deltas that reached review and lost, so the archive-of-
    roads-not-taken property this package promises is untested.

---

## F-173 — four sound laws in the boot snippet, none of them falsified by the corpus that ignores them

**Outcome:** OUT-OF-ROUTE (§3.6 routes (b) and (c), all 4 anchors)
**Files touched:** none

**Re-verification:** the verdicts measure real gaps, and every one of them is a
gap on the *consumer's* side of a rule that is right.

- `LAW-QUOTE-FIRST-CRITIQUE-SECOND` — quote-first is practised where quoting
  is allowed (the Tessl study opens §1 with the subject's pitch before any
  analysis, 27 blockquote lines); the **access-date** half is zero. But the
  reason quoting is absent from the rest of the corpus is a written owner
  directive, not neglect — see F-295 / F-296. Route (c): the exception belongs
  on the host side, and the owner rules.
- `LAW-DELTAS-NOT-DECREES` — «each with a priority and a target home».
  Numbering 5/5, target home 5/5, priority **3/5**
  (`legacy-spec/research/action-systems-vscode-idea.md:725` has no priority
  column; FUGU-SYNTHESIS carries `[need-gate]` instead). A law met by three of
  five is a law with two violations, not a false law. Route (b).
- `LAW-THE-RE-FETCH-LIST` — «every source URL with access date, plus the
  subject's version at capture». Dates: 4/4. Version: 2/4 —
  `PROP-004-tessl-comparative-research.md:905` gives «Tessl CLI version visible
  in changelog: 0.78.0» and the fractality selection note pins per source,
  against `action-systems-vscode-idea.md:778` whose Commit column reads
  `_not captured_` for both subjects. Route (b), and the law's own stated
  purpose is exactly what the two failures cost.
- `NEVER-LET-A-STUDY-SILENTLY-GO-STALE` — no study has ever been stale-flagged
  or refreshed. The Tessl study at `:908` instructs its own future reader to
  append a «### 7.5 Refresh history» block; `grep -nE '^### 7[.]5'` on that
  file returns nothing, and its capture is dated 2026-05-04 against Tessl CLI
  0.78.0 while the tree is at 2026-07. Route (b), and this is a **Never** rule
  — the one anchor here it would be most damaging to soften, because the
  measurement is precisely the failure it exists to prevent.

**Perimeter searched:** `legacy-spec/research/*.md` (8 files, including the
three archived studies), the fractality notes directory
(`packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/`, 22 files)
and its `spec/refs/INVENTORY.md`, `ROADMAP.md`, `spec/**`, `campaigns/**`,
`spec/boot/STATIC.md` (the host's compiled copy of this snippet, at :149 and
:171). Globs `*.md` throughout.

**What changed and why:** nothing. Every verdict here reduces to «the corpus
does less than the law asks». §3.6 route (b) says the package does not move;
route (c) says the owner rules where the departure is a written policy choice,
which it is for the quoting half.

**New obligations noticed:**

14. **The Tessl study never grew the refresh block it told itself to grow**
    (`legacy-spec/research/PROP-004-tessl-comparative-research.md:908`, no
    `### 7.5`), and it is two months past its capture. It is the single
    stale-flag candidate in the repository and would be the first exercise of
    `NEVER-LET-A-STUDY-SILENTLY-GO-STALE`.

---

## F-222 — the verdict was true when written and is false now: Phase D wave 1 already repaired the fragment under F-297

**Outcome:** RE-JUDGE: confirmed (1 anchor); OUT-OF-ROUTE (1 anchor)
**Files touched:** none

**Re-verification:** the verdict's load-bearing sentence is «**Law 5 is not
demonstrated at all: the fragment has no re-fetch section.**» It has one.

```
$ sed -n '186,191p' packages/org.vibevm.world/comparative-research/v0.1.0/spec/flows/comparative-research/research-template.md
## 7. Re-fetch list
- quarry.example/docs/remote — the remote-cache pitch quoted in §2.1 —
  accessed 2026-07-01.

**Capture date:** 2026-07-01. **Subject version at capture:** Quarry 3.2.
```

That is Law 5 demonstrated in full: URL, access date, capture date and subject
version at capture. The other four hold as the verdict itself concedes — dated
verbatim quotes at `:154-156` and `:163-165`, §3.1 trail against §4.1 lead, and
D1 at `:181-184` numbered, `**Priority:** MEDIUM`, `**Target home:** a future
caching spec section`, closing «Ratified here: nothing.»

**The repair is already recorded, and by this campaign.** The Phase C cache
carries `MINIATURE-LAW-FIVE` as **confirmed**, with this note in its evidence:

```
$ python -c "…run/cache.json… → verdicts.MINIATURE-LAW-FIVE"
v  -> confirmed
ev -> [ '…/research-template.md:189  - ##MINIATURE-LAW-FIVE a closing re-fetch list carrying the capture date and the',
        '…/research-template.md:181  ## 7. Re-fetch list',
        "CLOSED F-297 (Phase D, wave 1, route a). The worked fragment was introduced as obeying
         all five laws and obeyed four: Law 5 asks for a closing re-fetch list with the subject's
         version at capture, and the fragment stopped at 5. Repaired on the STRONGER side - the
         fragment gained the section its own skeleton defines, quoting the one source it already
         cites - so the example now demonstrates the law instead of the caption being softened to
         match a fragment that broke it." ]
```

So F-222's first anchor is a **second row on an already-closed defect**,
measured on the caption (`the-fragment-obeys-all-five-laws-lead`) where F-297
measured it on the bullet (`MINIATURE-LAW-FIVE`). The caption is now true and
should re-judge confirmed.

**Perimeter searched:** the whole of `research-template.md` (211 lines, the
skeleton at :22-110 and the worked fragment at :151-191), and
`campaigns/packages-2026-09/run/cache.json` +
`campaigns/packages-2026-09/run/state/corpus.json` for every recorded verdict
on that file's anchors.

**What changed and why:** nothing, and nothing should. Editing here would
either restate F-297's repair or — far worse — soften the caption to «obeys
four», which is exactly what F-297 explicitly refused to do.

`SUM-NUMBER-PRIORITIZE-AND-HOME-EVERY-DELTA` is the second anchor and is
OUT-OF-ROUTE, route (b): «Number, prioritize, and home every delta; ratify
none» scores numbered 5/5, homed 5/5, prioritized **3/5**, ratify-none 4/5
across the corpus. Same measurement as `LAW-FOUR-…` under F-135 and the same
disposition.

**New obligations noticed:**

15. **Phase C produced two drift rows for one defect and F-297 closed only
    one.** `MINIATURE-LAW-FIVE` re-judged confirmed at the wave-1 closure;
    `the-fragment-obeys-all-five-laws-lead` — the caption directly above the
    same bullet list, in the same file — was left open in a separate
    obligation. Worth a check for other anchors whose sibling closed in wave 1
    while they did not; the merge key `(document, type)` would not have joined
    them because F-297 typed its row differently.

---

## F-295 — a written owner directive on legal exposure, colliding with a shipped law; the owner rules, not the boss

**Outcome:** OUT-OF-ROUTE (§3.6 route (c))
**Files touched:** none

**Re-verification:** the host's departure from
`NEVER-PARAPHRASE-WHERE-A-QUOTE-CAN-STAND` is not neglect. It is an owner
directive with its rationale quoted verbatim, in two places.

```
$ grep -n -A3 'CLEAN-ROOM-RULE' spec/boot/90-user.md
38:## Third-party research code — clean-room rule (owner directive, 2026-07-07)
40:##CLEAN-ROOM-RULE **`eth-sri/type-constrained-code-generation` … is inspiration-only —
   NEVER a code source.** … no code from that repository may be copied, adapted line-by-line,
   or ported into this project — license and patent exposure. The working method is clean-room:
   study what their code achieves, then write STRUCTURALLY DIFFERENT code that reaches the same
   behavior (identical results are fine; borrowed expression is not). … Apply the same posture to
   any other research repository until the owner explicitly clears its license/patent status.

$ sed -n '160,174p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md
The owner's clean-room order, operative verbatim: «мы именно вычленяем и
понимаем идеи, мы не должны копировать код. … нужно понять его
смысл и реализовать Clean Room Implementation. … копирование кода
приведет к юридическим последствиям». Methods described in papers
are implementable freely; reference *code* is inspiration-only
regardless of its license; article text is cite-only. **Never:**
port lines, adapt file-by-file, or paste source text into notes.
```

The measurable consequence the verdict cites is real: **0 blockquote lines
across the 22 files** of the fractality notes directory, against 27 in the
archived Tessl study.

**Perimeter searched:** `spec/boot/90-user.md` (the user-owned boot file, whole
file), `packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/` (the
research plans, D-R4 through D-R7),
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/INVENTORY.md` (the
clean-room register), and the notes directory
`…/spec/refs/notes/` (22 `.md` files) for blockquote lines.

**What changed and why:** nothing, and it is not mine to change. §3.6 route (c)
is explicit — «the exception is **written down on the host side** … The owner
rules on (c) wherever the exception is a policy choice rather than a note.»
Legal and patent exposure is a policy choice by any reading. Softening
`NEVER-PARAPHRASE-WHERE-A-QUOTE-CAN-STAND` in a **shipped** package would push
one project's legal posture onto every consumer that installs the flow.

**A refinement the owner will want, because it narrows the collision.** The
clean-room directive is not a blanket ban on quoting. Its own text splits the
cases: «Methods described in papers are implementable freely; reference *code*
is inspiration-only regardless of its license; **article text is cite-only**».
Cite-only *permits* the dated quote. So the collision is confined to subjects
that are **source-code repositories**, and it does not touch documentation,
marketing or article subjects — which is why the Tessl study quotes at scale
and stays inside both rules. A route-(c) exception scoped to code-repository
subjects would close this anchor without weakening the law for anyone.

**New obligations noticed:**

16. **The clean-room exception is unwritten *as an exception to this flow*.**
    `spec/boot/90-user.md:38-40` states the directive but never names
    `flow:comparative-research` or which of its laws it overrides; the flow's
    boot snippet is compiled into the same lane at `spec/boot/STATIC.md:149`
    and states the opposite. A cold session boots both and is given no rule for
    which wins. That is the host-side record §3.6 (c) requires and it does not
    exist yet.

---

## F-296 — the same collision, stated from the protocol's side; same ruling

**Outcome:** OUT-OF-ROUTE (§3.6 route (c))
**Files touched:** none

**Re-verification:** the anchor is
`##critiquing-a-paraphrase-is-critiquing-a-straw-man` —
«Critiquing a paraphrase is critiquing a straw man: the paraphrase is already
your reading, and a reader cannot check your judgement against a summary you
wrote» (`COMPARATIVE-RESEARCH-PROTOCOL.md:90-93`). The host's corpus does
exactly the shape this sentence calls a straw man, and says so in its own text:

```
$ sed -n '15p' legacy-spec/research/action-systems-vscode-idea.md
…Everything here is in our words; short verbatim snippets (`file:line`) ground claims,

$ sed -n '4,6p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/INVENTORY.md
Clones and downloads live under the host `/refs/` tree, which is gitignored
wholesale — nothing third-party is ever committed. The host clean-room
[rule] … take*, never its text or code shapes.
```

Same directive, same rationale, same citations as F-295 — the two obligations
are one collision measured from two sides (the boot snippet's `Never` and the
protocol's justification for it).

**Perimeter searched:** identical to F-295, plus
`legacy-spec/research/action-systems-vscode-idea.md` and
`legacy-spec/research/settings-system-vscode-idea.md` (the two studies that
state the in-our-words posture in their own headers), and `.gitignore:34-37`
(`/refs/` gitignored, which is why no consumer of a clone can reach the
sources at all).

**What changed and why:** nothing, for the reasons under F-295. I want to be
explicit that I considered and rejected the available edit: rewording
`critiquing-a-paraphrase-is-critiquing-a-straw-man` to admit clean-room
paraphrase would delete the *reason* Law 2 exists from every consumer's copy in
order to record one project's legal constraint. That is §3.6's «easy
direction», named in the owner's own word.

Phase C's own framing already anticipated this and I am not overturning it:
«Recorded as drift on the campaign's own line: where the host's own written
contract contradicts the flow, it is drift, whatever the merit of the host's
reason.» It is drift; the *closure* is the owner's ruling, not a prose edit.

**New obligations noticed:** none beyond #16 above — F-295 and F-296 close
together or not at all, and a ruling on one is a ruling on both.

---

## Summary of dispositions

| id | package | outcome | anchors edited / total |
|---|---|---|---:|
| F-133 | campaign-plans | EDITED (partial) | 1 / 13 |
| F-144 | campaign-plans | RE-JUDGE: confirmed (2) + OUT-OF-ROUTE (6) | 0 / 8 |
| F-155 | campaign-plans | OUT-OF-ROUTE | 0 / 6 |
| F-195 | campaign-plans | RE-JUDGE: confirmed (1) + OUT-OF-ROUTE (2) | 0 / 3 |
| F-221 | campaign-plans | OUT-OF-ROUTE (b + c) | 0 / 2 |
| F-291 | campaign-plans | EDITED | 1 / 1 |
| F-292 | campaign-plans | EDITED | 1 / 1 |
| F-293 | campaign-plans | EDITED | 1 / 1 |
| F-135 | comparative-research | EDITED (partial) | 1 / 11 |
| F-156 | comparative-research | EDITED (partial) | 1 / 6 |
| F-173 | comparative-research | OUT-OF-ROUTE | 0 / 4 |
| F-222 | comparative-research | RE-JUDGE: confirmed (1) + OUT-OF-ROUTE (1) | 0 / 2 |
| F-295 | comparative-research | OUT-OF-ROUTE (c) | 0 / 1 |
| F-296 | comparative-research | OUT-OF-ROUTE (c) | 0 / 1 |

**Six anchors edited across four files; four anchors re-judged confirmed on
falsified verdicts; the rest recorded rather than edited.** The shape is not an
accident and it is worth one sentence: these two packages are **normative flow
documents**, so most of their drift verdicts read «the host does less than the
rule asks», which §3.6 routes to the host and not to the package. The edits
that did land are the four kinds §3.6 route (a) names — a miscounted anecdote,
a false claim about a sibling package, a wrong internal count, and two
definitions the package's own siblings contradict.

**Four Phase-C verdicts in this batch are measurably false today**, three of
them on the perimeter rule §6.1 exists to prevent:

| verdict claim | truth | why it failed |
|---|---|---|
| «`EXECUTING` occurs nowhere in the repository as a status» | 2 plans use it | grep never reached `legacy-spec/research/` (8 plans) |
| «`git log --oneline` … returns 0 … not one of the 7 archived quick-start blocks» | 13 hits, 6 of them quick-start blocks confirming the tree | same |
| «`grep -ci 'quick.start'` returns 0 in both live plans» | 6 and 4 | the host fixed itself by owner ruling 2026-07-29 |
| «the fragment has no re-fetch section» (F-222) | it has one | F-297 repaired it in Phase D wave 1 |

**Files touched, all four:**

- `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\README.md`
- `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\campaign-plans\v0.1.0\spec\flows\campaign-plans\execution-ledger.md`
- `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\comparative-research\v0.1.0\spec\flows\comparative-research\COMPARATIVE-RESEARCH-PROTOCOL.md`
- `C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\comparative-research\v0.1.0\spec\flows\comparative-research\from-research-to-roadmap.md`

No file outside the two package directories was modified. No `git` command was
run. No `##ANCHOR` id was added, removed or renamed — **three now carry prose
their id no longer describes** (`core-ai-native-defers-from-its-next-release`,
`A-RESEARCH-DOCUMENT-IS-A-SELF-CONTAINED-STUDY-OF-ONE-SYSTEM`,
`A-RESEARCH-DOCUMENT-ENDS-IN-A-DELTA-TABLE`), which is the deliberate cost of
`RULE-ANCHORS-IMMUTABLE` and is flagged here so the boss sees it as a choice.
No relative link into another package was added.
