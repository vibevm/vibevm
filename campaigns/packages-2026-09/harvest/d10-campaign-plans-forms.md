# d10 — the six missing `campaign-plans` forms, drafted for both host plans {#root}

_Written 2026-07-31. **Nothing here is applied.** This file is the only file
this pass writes; every block below is prepared for the boss, who decides
whether it lands, where, and in what wording. The owner has ruled the direction
(«привести свои два плана в строй», 2026-07-31); the application and every
verdict stay the boss's._

**Measured at HEAD `fffcb494`** (`git rev-parse --short HEAD`; the branch is
`main`, working tree clean at the time of measurement). HEAD moved once during
this pass — the first measurements were taken at `91ebf1fd` and every number
below was re-taken at `fffcb494`. The campaign is live, so the wave-2 figures
move; every one of them carries the command that reproduces it.

## What this file supplies {#scope}

`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md` §weight measures six
`flow:org.vibevm.world/campaign-plans` section forms present in the `fractality`
specspace's plans and absent from the host's own two — commit map 3/0, safe stop
3/0, whole-campaign acceptance 2/0, non-goals 3/0, risks 3/0, Phase 0 2/0 — and
records the ruling that follows: **«the practice is not abandoned; the host's own
two plans are the outlier»**, so the host's plans come into line rather than the
flow being softened.

Twelve blocks follow: six forms × two plans.

- [`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml`](../../../spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml)
  — wave 1, **CLOSED** 2026-07-26. Its six forms are **honest retrospect**,
  written at close-out on 2026-07-31 under the owner's bring-into-line ruling.
  Not one of them pretends to have been written in advance, and each says so in
  its own opening line. A retrospective risk register is a list of what
  happened; a retrospective Phase 0 is a statement of what stood before Phase A
  *and* of the fact that no Phase 0 ran.
- [`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml`](../../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml)
  — wave 2, **LIVE**, Phase D near its exit. Its Phase 0 and the closed half of
  its commit map are retrospective; its safe stop, risks, non-goals and
  whole-campaign acceptance are **real going forward** and bind the sessions
  that finish D, T, F and G.

## The anchor grammar, measured rather than assumed {#anchors}

The task order asked for `##ANCHOR` fact ids on the new sections. **Measured,
both plans carry none**, and the blocks below therefore carry `{#slug}` heading
anchors only — which is what "match the file exactly" resolves to here:

```sh
grep -c '^##[A-Za-z]' spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml      # 0
grep -c '^##[A-Za-z]' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml  # 0
grep -o '@\(spec\|impl\|doc\)/[a-z]*' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml | sort | uniq -c
#   13 @impl/done   ·   5 @spec/done   — every one of them inside prose ABOUT
#   another document's markers, never marking a fact of this plan's own
```

The reason is in `progress.toml`: the include list names
`spec/boot/[0-9]*.md`, `spec/common/**`, `spec/design/**`,
`spec/manual-tests/**`, `spec/modules/**`, `packages/org.vibevm.world/**` and
`packages/org.vibevm.ai-native/**` — and **not** `spec/terraforms/**`. Neither
plan is scanned, so `check --exhaustive` neither requires nor validates a fact
anchor in either file, and adding one would make these two the only fact-grain
documents in an unobserved directory. Both plans use `{#slug}` headings on every
section and nothing else; the fractality models do the same (`## 9. Risks and
fallbacks`, `## 10. Non-goals (named, with disposition)`, `## 12.
Whole-campaign acceptance` — headings, prose, no fact anchors).

**If the boss rules the other way**, one worked example sets the pattern — the
same block, marked:

````markdown
## 8.6 Risks and fallbacks (recorded retrospectively, 2026-07-31) {#risks}

##A-RISK-REGISTER-WRITTEN-AFTER-THE-FACT-IS-A-LIST-OF-WHAT-HAPPENED This
register was written at close-out and is labelled as such: it is not a list of
what the campaign feared, it is a list of what the campaign met. @impl/done

- ##RISK-VERIFICATION-THAT-CITES-SPEC-PROSE **R1 — a verdict whose evidence is
  another spec document.** *Fired:* F-063. @impl/done
````

Two costs to weigh before ruling: the anchors are unvalidated (nothing scans
the file, so a duplicate or malformed id is caught by no gate), and marking
these two files makes the campaign's own plan a fact-grain document inside the
corpus it measures — the `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS` trap, one
layer deeper. The blocks below are written unmarked; converting them is
mechanical.

*One consequence of quoting the blocks verbatim: **this file carries duplicate
heading anchors of its own** — `{#risks}`, `{#non-goals}`, `{#safe-stop}` and
`{#acceptance}` each appear twice, once as this file's own section heading and
once inside a ready block, where it is the anchor the block will carry after
insertion. The block anchors are the correct ones and must not be renamed; this
file is under `campaigns/`, which `DEFAULT_EXCLUDES` holds out of the corpus
always-on, so nothing scans or validates it and the duplication costs nothing.*

## Numbering: the house uses fractional sections for inserts {#numbering}

Neither plan numbers its sections the way the flow's canonical skeleton does
(wave 1 runs §0–§12, wave 2 §0–§10). The house has already solved insertion
twice, and both precedents are followed here:

1. **Fractional numbering** — wave 2's own `## 4.5 Amendments carried in from
   wave 1's close-out {#amendments}` was inserted between §4 and §5 rather than
   renumbering the plan.
2. **Append with a dated italic note naming the ruling** — this is exactly how
   the previous `campaign-plans` form landed. On 2026-07-29 the owner ruled
   `##COLD-A-LITERAL-QUICK-START-BLOCK` sound and both plans gained the block:
   wave 1's `## 12. Quick-start` (line 1420) opens *"Added 2026-07-29 by owner
   ruling: `flow:campaign-plans`' `##COLD-A-LITERAL-QUICK-START-BLOCK` requires
   it and this plan had none"*, and wave 2's `## 10.` (line 3772) does the same.
   **Every block below opens the same way**, because that is the worked
   precedent this repository already accepted.

No existing section is renumbered by any block here, and no existing line is
edited — every insert is a pure addition at a named point.

## Observations that are NOT blocks {#observations}

Two things surfaced while binding hashes to phases. Neither is one of the six
forms and neither is drafted here; both are the boss's to route.

- **Wave 2's status line lags the tree by four phases.** Line 5 reads
  `**status: RATIFIED 2026-07-26 · PHASE A OPEN …**` and line 3's `<status …
  comment="… Phase A open"/>` agrees, while the campaign is at Phase D wave 9
  (63 commits into D — see the wave-2 commit map below). The flow's
  `##THE-STATUS-LINE-MUST-NEVER-LAG-THE-TREE` and `flow:campaign-plans`'
  `##BOUNDARY-REFRESH-THE-STATUS-LINE` both bind here; the commit map is what
  makes the lag visible, which is part of why the map is the form worth landing
  first.
- **Wave 2's §8 Deferrals says `*(empty)*` while its zone's `deferrals.md`
  carries 85 lines**, including three inherited wave-1 tails. The flow's
  `##NEVER-CARRY-A-DEFERRAL-OUTSIDE-THE-PLAN-FILE` reads on this directly; wave
  1 has the same shape (§10 says *"empty — drained into
  `campaigns/<id>/deferrals.md` at close-out"*), so it may be a deliberate
  dialect rather than drift — but it is stated nowhere, and an undeclared
  dialect is indistinguishable from a lapse.

## This pass dirtied six run-state files, and did not clean them up {#side-effect}

**Reported rather than fixed, because the fix is a git write and this pass may
not make one.** Verifying the acceptance blocks meant running
`vibe progress check --exhaustive` once per zone. `progress check` is not
read-only — it writes the parse cache and the state projections:

```
 M campaigns/packages-2026-09/run/cache.json          |   25 +-
 M campaigns/packages-2026-09/run/state/campaign.json |    6 +-
 M campaigns/packages-2026-09/run/state/corpus.json   | 2346 +++----
 M campaigns/progress-2026-08/run/cache.json          | 4962 +++++++++++++--
 M campaigns/progress-2026-08/run/state/campaign.json |    6 +-
 M campaigns/progress-2026-08/run/state/corpus.json   | 3716 ++++++++++-
```

**The wave-1 zone took the larger hit and the reason matters:** `--campaign`
selects the state zone, not the perimeter, so
`progress check --exhaustive --campaign campaigns/progress-2026-08` made the
wave-1 zone's cache absorb the **whole 260-file scope** rather than wave 1's 58
files. That is the same behaviour the acceptance block's step 1 already flags —
here it is, doing damage rather than being described.

**Do not restore blindly.** HEAD moved during this pass (`91ebf1fd` →
`fffcb494`, three commits) and a sibling harvest file appeared untracked, so
another session is working the same tree; `git restore` on these paths could
discard state that session legitimately produced. If the boss confirms the
modifications are this pass's alone, the undo is:

```sh
git restore campaigns/packages-2026-09/run campaigns/progress-2026-08/run
```

**The durable lesson, worth a line wherever the campaign's tool notes live:**
`progress check` looks like a query and behaves like a write, and pointing it at
a closed campaign's zone rewrites that zone's cache to a scope the campaign
never had. A verification step that mutates the artifact it verifies is not a
verification step.

---

# SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml — wave 1, CLOSED {#w1}

**File:** `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml`, 1 437 lines at
HEAD `fffcb494` (`wc -l`). Sections today: §0 mandate (13) · §1 baseline (39) ·
§2 executors (52) · §3 layout (64) · §4 resume (80) · §5 phases (99, with A 104,
B 127, L 145, C 196, D 216, E 241, F 262, G 280, Close-out 301) · §6 recurrence
(307) · §7 dashboard (317) · §8 predictions (327) · §9 LOG (339) · §10 deferrals
(1291) · §11 REPORT (1295) · §12 quick-start (1420).

Five insert points, six blocks (8.5 and 8.6 share one point).

## SPEC-ACTUALIZATION — Phase 0 {#w1-phase-zero}

**Insert point:** inside §5, **after** the two-line §5 lead-in that ends
`Every session inside any phase obeys §4.` (line 102) and its blank line, and
**immediately before** `### Phase A — Scaffold {#phase-a}` (line 104). A reader
walking the phase list top-down must meet Phase 0 first, which is the whole
reason `flow:campaign-plans` puts it there.

```markdown
### Phase 0 — what stood before Phase A (recorded retrospectively) {#phase-zero}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##PHASE-ZERO-PRODUCES-NO-COMMITS-AND-LATER-PHASES-CARRY-FOUR-ELEMENTS`
asks every campaign to open with a phase that produces no commits, and this
plan had none. **No Phase 0 ran.** This section records what stood before Phase
A, what did Phase 0's job under another name, and what a real Phase 0 would
have caught earlier — written after the fact and labelled as such, not
back-dated into the planning prose.*

**The tree before Phase A** (§1's own baseline, verified 2026-07-24, restated
here because a Phase 0 is where a reader looks for it): Progress Control did
not exist — no `progress-core` crate, no `vibe progress` adapter, no
`campaigns/` zone, no dashboard — and `<status` appeared nowhere in the tree
except PROP-043's own dogfood markers. Host `spec/`: **91 `.md` files, 26 699
lines**. Free-form `**Status:**` lines to convert: **~55**. specmap: index live,
**34 gated orphans** in `vibe-spec`, pre-existing. Three inline grammars the new
markup had to avoid colliding with: `@spec://` (~17 uses), `#use` / `#embed` /
`#source`, and `<!-- REVIEW: -->`.

**What did Phase 0's job.** Phase A step 5 — the pilot — is a Phase 0 wearing a
scaffolding phase's clothes, and it behaved exactly as the law asks: three
documents of different genres hand-marked, **46/46 paragraphs**, one real drift
caught (`spec/design/README`'s index was incomplete), and one placement
ambiguity found (a document with no preamble under its H1) that **amended
PROP-043 §3.8 in place, before Phase B committed a single marker** — the
`##RULE-PHASE-ZERO-GATES-EVERYTHING-AFTER` shape, executed under a different
name. It landed in `ac97f26c`. One finding arrived outside every prediction: a
live power cut during the phase exposed a missing fsync-before-rename in
`write_atomic`, fixed with a tolerant cache load plus tests.

**What a Phase 0 would have re-measured, and did not.** Both of §1's headline
numbers were wrong, and both were corrected only *after* Phase B had opened —
`##PHASE-ZERO-RE-MEASURE-THE-NUMBERS` is the rule they miss:

| §1 said | `scan` / B0 measured | why |
|---|---|---|
| 91 files | **97** | six progress-control documents were authored after the baseline froze |
| ~55 status lines | **73** | the estimate missed the `**status:` and `**Status.**` variants |

Neither cost anything, because both moved in the harmless direction. The scope
itself then moved twice more on owner rulings — `8901cd05` dropped
terraforms/research/neworder, `1c48019a` dropped spec/discipline — so the
observed corpus ran **97 → 94 → 59 → 58** and the plan's own §1 denominator was
never the one the campaign executed against. *A baseline that is not re-measured
at the phase boundary is a number the report will have to apologise for.* Wave
2 inherited the lesson and re-measured its own §1 at Phase A step 1, where three
of its figures fell.
```

**Measurements behind the numbers.** 91 / 26 699 / ~55 / 34 orphans and the
three inline grammars are §1 of the plan itself (verified there 2026-07-24).
97 files, 3 684 paragraphs, 46 pre-marked and 73 converted status lines are the
`2026-07-24 · Phase B OPENED; B0 landed` LOG entry (§9). 46/46, the
`design/README` drift, the §3.8 amendment and the `write_atomic` power-cut
finding are the `2026-07-24 · Phase A CLOSED` LOG entry. The scope chain 97 → 94
→ 59 → 58 is the `PHASE B CLOSED` entry (*"94→59 terraforms/research/neworder;
59→58 discipline"*) joined to B0's 97. The two ruling commits:

```sh
git log --format='%h %ad %s' --date=short -1 8901cd05
#   8901cd05 2026-07-25 chore(progress): drop terraforms/research/neworder from scope (owner ruling)
git log --format='%h %ad %s' --date=short -1 1c48019a
#   1c48019a 2026-07-25 chore(progress): drop spec/discipline from scope (owner ruling)
```

## SPEC-ACTUALIZATION — safe stop {#w1-safe-stop}

**Insert point:** a new top-level section **after** §4's item 5 (`5. git =
second echelon: batch commits make the worst disk-loss cost one batch, never
the campaign.`, ending line 97) and **immediately before** `## 5. Phases
{#phases}` (line 99). §4 is the *step*-level crash-safety law; §4.5 is the
*phase*-level one, and the two belong together — a reader who has just been told
the maximum crash loss is one step is the reader who needs to know what a
deliberate stop costs.

```markdown
## 4.5 Safe stop — where wave 1 could halt losing nothing (recorded retrospectively) {#safe-stop}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##ANY-PHASE-BOUNDARY-IS-A-SAFE-STOP` asks every plan to
say where it can be put down, and this plan said it nowhere. Written after the
campaign closed, so it is a record of where the stops actually were — not a
promise made in advance.*

**This campaign ran on a finer grain than the law asks for.** The flow's unit is
the phase boundary; §4's unit is the **step** — mark-file, verify-unit,
close-obligation, execute-task — with `step-start` journalled before the work
and `step-done` after, so the maximum loss on any crash is one step. That is
strictly stronger, and it is why no session of this campaign ever needed the
phase-level guarantee to recover. Both grains held; both are stated here because
a stranger reading only §4 would think the campaign could be stopped anywhere,
and a stranger reading only the flow would think it could be stopped only six
times.

**What a stop at each boundary would have left, so «losing nothing» is a
statement and not a slogan:**

| Stopped after | The tree holds | What is owed |
|---|---|---|
| **A** | the `progress-core` crate, the `vibe progress` adapter, an empty campaign zone, the dashboard, and 46 pilot markers | nothing — the spec corpus is untouched |
| **B** | markers only, over 58 files / 4 880 facts / 4 944 markers, `check --exhaustive` at 0 | nothing — Phase B makes **no semantic edits** by its own law, and the legacy `**Status:**` lines were kept rather than deleted, so the pass is purely additive |
| **L** | the four legacy directories relocated to root `legacy-spec/`, every gate-binding inbound reference repointed | nothing — two of the four had zero corpus inbound to begin with |
| **C** | verdicts in the cache: 4 944 / 4 944 markers judged, 4 455 units at **93.0 %** | nothing — Phase C edits no document; a verdict lives in the cache, never in the markup |
| **D** | 302 of 311 drift rows closed; the tree at **99.7 %** | nine ledger rows, each named |
| **E** | the task queue drained: **4 486 confirmed / 1 drift / 3 unverifiable of 4 490 — 99.9 %** | one drift row that cannot close in this repository, and it is in `deferrals.md` with the reason |

**The one boundary where the floor was not green, recorded rather than
smoothed.** The Phase C close `self-check` went **red** on
`cli_pkg_cycle::install_from_git_registry`. Root-caused and proven in-session: a
`~/.vibe/registry.toml` had appeared on the machine that day, and the test
isolates `VIBE_REGISTRY_CACHE` but **not the settings chokepoint**, so the
global registries merged into the "hermetic" resolver and minted a second cache
bucket; with `VIBE_SETTINGS` pointed at an empty directory the same test passes.
The campaign's own gate (`progress check`) stayed **0** and the phase's commits
were docs-only and unrelated. Ledgered as **F-055** and fixed in Phase E — after
which *«the floor ends the phase green with no `VIBE_SETTINGS` override»*, F-055
genuinely fixed rather than worked around.
`##A-PHASE-THAT-LEAVES-THE-FLOOR-RED-IS-STILL-OPEN` is the rule this brushes,
and the only reading under which C could close is the one the ledger states: the
red was outside the phase's own diff, and it was proven so rather than assumed.

**Two things that are NOT safe stops**, both learned at cost:

- **A batch whose files are written and whose journal step is still open.** §4's
  recovery rule is not advisory — step open ⇒ `git restore` its files and redo
  the step. Steps are idempotent by construction precisely so this is cheap.
- **Delegated work committed on a filled-in task journal rather than on the
  completion notification.** Executors write the ledger as they go, so
  committing on it captures an intermediate state; doing so once left the tree
  conform-red for twenty minutes. Now a WAL Constraint, together with its
  sibling: **a gate never seen to go red is not known to work.**
```

**Measurements behind the numbers.** 58 / 4 880 / 4 944 and `check --exhaustive`
0 are the `PHASE B CLOSED` LOG entry; 4 944/4 944 and 4 455 units at 93.0 % are
the `c4f3 + PHASE C EXIT GATE` entry; 302 of 311 and 99.7 % are the wave-d2
entry; 4 486 / 1 / 3 of 4 490 = 99.9 % is the `PHASE E` entry. The red floor,
its root cause, the `VIBE_SETTINGS` proof and F-055 are the `post-gate floor
note` entry (§9, line 1097), whose commit is:

```sh
git log --format='%h %ad %s' --date=short -1 242085d4
#   242085d4 2026-07-25 docs(spec): post-gate floor red root-caused — the test drinks user state
```

The two WAL Constraints are quoted from the `PHASE E` entry's closing
paragraph.

## SPEC-ACTUALIZATION — non-goals {#w1-non-goals}

**Insert point:** two new top-level sections **after** the last line of §8
Predictions (line 337, the sixth prediction *"**The month budget holds.** …"*)
and **immediately before** `## 9. LOG (execution ledger — append per
batch/wave/phase) {#log}` (line 339). Non-goals and risks belong with the
predictions, in the plan's forward-looking half, ahead of the record half — the
order `flow:campaign-plans` gives them (§7 predictions → §9 risks → §10
non-goals). This block is §8.5; the risks block below is §8.6 and shares the
same insert point, immediately following.

```markdown
## 8.5 Non-goals (named retrospectively) {#non-goals}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##NON-GOALS-ARE-NAMED-SO-THEY-STAY-VISIBLE` asks a plan
to name what it deliberately does not do, and this plan named it nowhere. Every
line below was a real boundary the campaign held — most of them owner rulings
recorded in §9 — but none was written down as a non-goal at authoring time, and
the two that ended up costing something (the judgment axis, the doc trees) are
exactly the two that were never named. Reason and disposition on each, per
`##EVERY-NON-GOAL-CARRIES-A-REASON-AND-A-DISPOSITION`.*

- **Does NOT extend to `packages/**`.** *Reason:* one corpus at a time; the
  method had to be proven before it was scaled. *Disposition:* **wave 2**,
  [`PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml`](PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml).
- **Does NOT touch `packages/org.vibevm.fractality/**`.** *Reason:* its own
  specspace, own boot contract, own WAL; the mandate excluded it in as many
  words («the fractality specspace excluded until the owner says otherwise»).
  *Disposition:* held by the owner. *(Recorded 2026-07-31, because it turned out
  to matter: that specspace is a **second adopter** of several flows this
  programme measures, and a perimeter blind to it reads adoption as absence.)*
- **Does NOT mark or verify the relocated legacy directories.** *Reason:* Phase
  L moved terraforms / research / neworder / discipline to root `legacy-spec/`
  as historical records, not living contracts. *Disposition:* rejected outright
  — and reinforced by the owner's ruling of 2026-07-31 that `legacy-spec/**` is
  not evidence of practice in either direction.
- **Does NOT mark generated artifacts.** `spec/boot/STATIC.xml`,
  `spec/boot/INDEX.md` and `spec/WAL.xml` in session form. *Reason:* markup
  written into a generated file dies at the next `vibe install` or wind-down.
  *Disposition:* rejected; owner rulings 2026-07-24. The *authored* boot
  snippets (`00-core`, `90-user`) stay observed via `spec/boot/[0-9]*.md`.
- **Does NOT edit `spec/boot/90-user.xml`.** *Reason:* user-owned;
  `00-core`'s `NOTOUCH-90-USER` forbids it to every session. *Disposition:*
  deferred **to the owner, not to a campaign** — F-063's half was handed over in
  full rather than edited, and closed 2026-07-26 when the owner lifted the bar.
- **Does NOT decide what should *happen* to a fact.** Phase B marked what 4 917
  facts **are**; nothing in any phase asked what should be **done** about them.
  *Reason:* none was given at authoring time — this is the non-goal the campaign
  held without ever choosing it, which is why §5-F's three views came out empty
  (`freeze/plan` 0, `action="rework"` 0, `stage="idea"` 0). *Disposition:*
  deferred to wave 2 as amendment **A3(i)**, the judgment-marking pass.
- **Does NOT write the two documentation trees.** User Guide and Package Author
  Guide. *Reason:* Phase G's definition demands harvest cards and captured runs
  as input, and Phase C skipped the step that produces them. *Disposition:*
  deferred to wave 2 as amendment **A3(ii)** — the Package Author Guide in
  particular, since `packages/` is the corpus it documents.
- **Does NOT use fractality.** *Reason:* the owner's decision, verbatim in §0,
  deliberately overriding the standing delegation-first default for this
  campaign's duration — «Я хочу чтобы Fable сделала максимум высокоуровневых
  задач». *Disposition:* held by the owner; do not "optimize" it back.
```

## SPEC-ACTUALIZATION — risks and fallbacks {#w1-risks}

**Insert point:** the same point as §8.5 above — **immediately after** the §8.5
block and **before** `## 9. LOG …` (line 339).

```markdown
## 8.6 Risks and fallbacks (recorded retrospectively) {#risks}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##EVERY-RISK-CARRIES-A-DETECTION-SIGNAL-AND-A-PLAN-B`
asks every plan to name its risks with a detection signal and a plan B, and
this plan named none. **A risk register written after the campaign is a list of
what happened, not of what was feared**, and it is labelled as such: each row
below fired, and each says what detected it — including the four whose honest
answer is «nothing detected it; the next phase tripped over it». The
`fired-and-undetected` column is the useful part of a retrospective register,
because it is the part wave 2 turned into amendments.*

- **R1 — a verdict whose evidence is another spec document.** *Fired:* F-063.
  Five token-precedence anchors in PROP-002 were sealed `confirmed` on the
  evidence «token loader 3-source order matches 90-user boot facts» — another
  spec file, carrying the identical error, in a security-relevant place.
  *Detection that existed:* **none.** Phase C's exit gate checked only that
  every marker carried a verdict, never what the verdict rested on. *Fallback
  taken:* wave 2's amendment **A2** — every verdict names which source class it
  rests on, and one resting on the package's own artifacts alone is counted as
  self-referential rather than as independent confirmation.
- **R2 — the campaign's own corrections introduce drift.** *Fired:* F-065.
  Phase D authored a `Shipped:` line claiming a `Baseline::store` that had never
  been built, and the verification pass then confirmed the row on it.
  *Detection that existed:* **none** — nothing predicted that stitching could
  add false claims. *Fallback taken:* wave 2's **prediction 6**, which makes
  «zero new false claims, zero of them confirmed» falsifiable, with wave 1's
  answer on record as 1 and 1.
- **R3 — a phase whose exit gate does not check its own steps.** *Fired:* Phase
  C listed «harvest cards written while knowledge is hot» among its steps and
  gated only on «100 % of markers carry verdicts». The step was skipped, it cost
  nothing at the time, and Phase G arrived to consume an empty directory.
  *Detection:* the downstream phase, three days late. *Fallback taken:* wave 2's
  amendment **A1** — every exit gate enumerates that phase's own steps.
- **R4 — a prediction no step forces you to run.** *Fired:* prediction 1. No
  step of the plan required a `weave`, so the claim sat untested for the whole
  campaign and was measured at close-out purely to fill the report row. *Plan B
  taken:* wave 2's amendment **A5** — every prediction names the step that tests
  it, or says outright that it is scored at close-out on purpose.
- **R5 — a state projection nothing refreshes drifts silently.** *Fired:*
  `tasks.json` sat 18 tasks stale and the dashboard read five tasks for a week;
  `findings.json` survived only because it was maintained by hand all campaign.
  *Detection:* none — a projection nothing refreshes and nothing checks has no
  signal by construction. *Fallback:* the numbers of record are the ones a
  command prints, never the ones a projection holds.
- **R6 — a gate never seen to go red is not known to work.** *Fired:* twice,
  favourably — two executors ran positive controls before trusting a green
  result, and were right to. *Fallback:* now a standing WAL Constraint.
- **R7 — session death, budget exhaustion, power loss mid-corpus.** *The one
  risk this plan did carry a fallback for*, in §4, and it held: journal
  `step-start` / `step-done`, torn tail discarded, maximum loss one step.
  *Fired:* a **live power cut during Phase A**, which additionally exposed a
  missing fsync-before-rename in `write_atomic` — fixed with a tolerant cache
  load plus tests. *Verdict:* the crash-safety law is the one part of this plan
  that was written in advance and needed no amendment.
- **R8 — an un-isolated test reaches real user state.** *Fired:* F-057, filed as
  a stray cache directory and found to be a credential-precedence leg —
  `vibe search` resolves a GitHub token through four legs and the fourth is a
  file in the settings directory the test could not reach, so an un-isolated run
  attached the real publish token to its outbound request. *Detection:*
  accidental, on the third pass — F-055, F-056 and F-057 were **one forgotten
  discipline caught three times by accident**. *Measured, not inferred:* against
  a loopback listener recording header names and lengths only, **47 bytes of
  `Authorization` before the fix, none after**; exactly one test reaches that
  path and it points `api_base` at 127.0.0.1, so the investigation closed with
  no rotation needed. *Fallback taken:* DRIFT-020 converts the discipline into a
  gate.
```

**Measurements behind the numbers.** F-063's five anchors and their evidence
string, F-065's `Baseline::store`, prediction 1's untested `weave`, the empty
Phase F views (`freeze/plan` 0, `action="rework"` 0, `stage="idea"` 0), the
4 917-fact corpus figure, `tasks.json` at 18 stale tasks and the five-task
dashboard week are all §11 REPORT and its `{#report-gaps}` subsection (lines
1295–1418). The power cut, the `write_atomic` fix and the pilot's 46/46 are the
`Phase A CLOSED` LOG entry. F-055/056/057 as one discipline, the 47-byte
`Authorization` measurement, the loopback listener, the no-rotation verdict and
the two WAL Constraints are the `PHASE E` LOG entry (line 1242). A1/A2/A3/A5 and
prediction 6 are wave 2's §4.5 and §6.

## SPEC-ACTUALIZATION — commit map {#w1-commit-map}

**Insert point:** a new sub-section at the **end of §9**, after the LOG's last
line (line 1289, `around — verified again at this session's open.`) and its
blank line, and **immediately before** `## 10. Deferrals {#deferrals}` (line
1291). §9 is this plan's execution ledger, and
`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS` puts the commit map inside
it. Appending rather than interleaving leaves the LOG's chronology untouched.

```markdown
### 9.1 Commit map — hashes bound to phases (recorded retrospectively) {#commit-map}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##EACH-EXECUTED-PHASE-GETS-A-LEDGER-SECTION` and
`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS` ask each executed phase for
a commit map, and this plan carried none — the §9 LOG above records what
happened, richly, and never binds it to hashes. **The flow also says the map is
written at the boundary, not reconstructed at close
(`##THE-MAP-IS-WRITTEN-AT-THE-BOUNDARY-NOT-AT-CLOSE`), and this one was
reconstructed.** That is a real weakness of this table and the reason it is
per-phase rather than per-commit: reconstruction can bind a hash to a phase
honestly, and cannot recover what each individual commit confirmed at the
moment it landed. The confirm/falsify column below is therefore per phase, cited
to the LOG entry that recorded it while it was fresh.*

**Deviation from `##ONE-ENTRY-PER-COMMIT`, stated rather than silent:** 139
commits is past the grain where one entry per commit informs anyone. Each phase
gets its range, its count, its landmark commits and its verdict; `git log` over
the perimeter below gives the rest.

**Perimeter, so the counts are reproducible.** Measured at HEAD `fffcb494`:

    git log --reverse --format='%h %ad %s' --date=short -- \
      campaigns/progress-2026-08 spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml

**139 commits**, `b1276c39` (plan authored, 2026-07-24) → `f5248dae`
(2026-07-29). The zone catches the work commits as well as the bookkeeping ones
because §3 rides the journal in the same commit as the edits it describes; the
four Phase A commits that predate the zone are named individually below.

**Campaign commit range:** `b1276c39` … `56cccca8` (close, 2026-07-26), plus one
post-close amendment, `f5248dae`.

#### Phase A — EXECUTED 2026-07-24; 8 commits {#cm-a}

`9446a2cb` introduce progress-control markup contract (PROP-043) · `b1276c39`
author the spec-actualization campaign plan v0.1 · `edd487ba` WAL checkpoint,
scaffold phase begins · `8b181522` progress-core crate + the `vibe progress`
adapter · `38855c00` campaign zone + the read-only dashboard · `ac97f26c` pilot
markup of three genres + PROP-043 ratification and pilot amendments ·
`9a69b6f6` WAL checkpoint, scaffold closed · `60100f87` ledger Phase A close-out
+ the B0 conversion ruling.

*Confirmed:* the §5-A prediction — the pilot exposed placement ambiguities and
they amended PROP-043 §3.8 before Phase B, exactly as predicted. *Found outside
every prediction:* a live power cut exposed a missing fsync-before-rename in
`write_atomic`. *Range in perimeter:* `b1276c39`..`60100f87` = 4 (the other four
predate the campaign zone).

#### Phase B — EXECUTED 2026-07-24/25; 60 commits {#cm-b}

`60100f87`..`a1bb2111`. Opens `8d5ccc82` (the wave-1 scope config, the
campaign's first journal step); closes `a1bb2111` (boundary — exit gate green,
corpus fully marked). Landmarks: `2c98a1e6` B0, the 73-line status conversion ·
`91274c89` B1, paragraph-exhaustive markup of `spec/common` · `6714876e` the
fact-grain re-pilot · `508bbdb9` DRIFT-004, fact anchors become addressable spec
units · `5c89839b` DRIFT-005, fact inheritance end to end · `7d9dd964` B2 batch
26, the corpus is fully marked. Two scope rulings landed inside the phase:
`8901cd05` and `1c48019a`. One owner amendment was recorded here and executed
later: `c0147947`, Phase L.

*Confirmed:* the exhaustive counter caught genuinely skipped paragraphs review
alone would have missed — the wrapped prose line whose continuation opens `+ `
parses as a phantom list item, caught twice in PROP-019 instantly. *Falsified in
place:* §1's ~55 status lines (73 actual) and 91 files (97 scanned). *Gate at
the boundary:* `check --exhaustive` clean over 58 files / 4 880 facts / 4 944
markers, 0 errors 0 warnings; `self-check` all green, real exit 0.

#### Phase L — EXECUTED 2026-07-25; 7 commits {#cm-l}

`a1bb2111`..`15c5bb30`. `62406fe0` resolves the plan-file review point (the plan
stays put) · `83346e78` `f8f347d8` `9514e8fb` `1ec6a27c` batches 1–4 ·
`70f3cbdd` the legacy dirs leave the spec tree · `15c5bb30` closed.

*Confirmed:* the L2 verdict — every cited fact was already corpus-resident, so
the relocation cost the corpus nothing. `spec/neworder` and `spec/discipline`
had **zero** corpus inbound.

#### Phase C — EXECUTED 2026-07-25; 18 commits {#cm-c}

`15c5bb30`..`242085d4`. Opens `bb337e90`; batches `5c5e1058` c1 · `3570cf2b` c2
· `f2beeff4` c3a · `49d67c39` c3b · `3d237c7d` c3c · `f82582f7` LOG · `9baa7fa6`
c3d · `baffe617` c4a · `09327502` c4b · `7392fbdd` c4c · `c325d333` c4d1 ·
`dcfa6301` c4d2 · `74025dd9` c4e · `727f6840` c4f1 · `5aa5ba86` c4f2; closes
`ddf7c0ca` (c4f3 + exit gate). `242085d4` records the post-gate red floor and
its root cause (F-055).

*Confirmed, mirrored:* the §5-C prediction — drift does concentrate around
Status lines, but in the **inverse** polarity, headers promising *less* than the
tree delivers. *Measured:* **4 455 units judged — 4 141 confirmed / 311 drift /
3 unverifiable = 93.0 % / 7.0 % / 0.07 %**, the first measured actuality level
of the spec tree; 58/58 files carry campaign maps, 4 944 / 4 944 markers carry
verdicts. *Silently skipped, and it cost Phase G:* the harvest-card step.

#### Phase D — EXECUTED 2026-07-25/26; 4 commits {#cm-d}

`242085d4`..`f6c17f92`. `a1847b0d` opens (wave d1 — 212 of 311 rows in one
sweep) · `3a6370bc` wave d2, the ledger empties to nine · `0064fd4a` the parity
row that needed no ruling · `f6c17f92` d2g/d2h.

*Confirmed:* prediction 4 — convergence in **two** waves against a ≤3 bound, and
exactly two owner escalations against a ≤2 bound. *Falsified by the next phase:*
this phase authored the `Shipped:` line for a `Baseline::store` that had never
been built (F-065). *Mechanics worth the record:* 191 of the wave-d1 rows were
scripted off the C-phase verdict map, dry-run first — no model touched them.

#### Phase E and close-out — EXECUTED 2026-07-25/26; 45 commits {#cm-e}

`f6c17f92`..`56cccca8`. **E and the close-out interleave and are not separable
in the chain** — both ran on 2026-07-26 in the same sessions, and saying so is
more honest than drawing a cut. Phase E opens `2ef9d854`; DRIFT-006…022 were
opened, fourteen executed, DRIFT-015 superseded before it ran, DRIFT-020 and
-022 left queued. Close-out landmarks: `cc3109ef` the ledger regains the entries
D and E never wrote · `bfdbd7f5` F-065, nothing can write the baseline the
recurrence reads · `9f7459bd` `deferrals.md` · `fa74b775` the six predictions
scored · `eae60b3d` F and G deferred with the measurement that says why ·
`db7186ef` the baseline gains a writer · `d3482dd7` `baseline.json` ·
`1ea4815c` F-063 closes · `56cccca8` WAL session-end. One commit here belongs to
the *next* campaign: `07a38e1a`, the fact-grain specmap engine — wave 2's Phase
A step 2, which is what closes wave 1's single surviving drift row.

*Measured at close:* **4 486 confirmed / 1 drift / 3 unverifiable of 4 490 —
99.9 %**; findings 61 of 64 resolved. *Verdicts:* predictions 1–5 CONFIRMED,
prediction 6 FALSIFIED in the favourable direction — a month of plan executed in
three days, and the honest reading is that the estimate measured human-paced
reading while the work was delegated batch execution.

#### After close — 1 commit {#cm-post}

`f5248dae` (2026-07-29) — `docs(terraforms): the plans running this campaign
were the ones breaking its rule`. The previous `flow:campaign-plans` form to
land under an owner ruling: §12's literal quick-start block. This section is the
second such landing, and the ledger records both so the pattern is visible
rather than incidental.
```

**Measurements behind the numbers.** Every count is a `git rev-list --count`
over the perimeter above, at HEAD `fffcb494`:

```sh
P='campaigns/progress-2026-08 spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.xml'
git rev-list --count b1276c39^..60100f87 -- $P   #  4   Phase A (in perimeter)
git rev-list --count 60100f87..a1bb2111  -- $P   # 60   Phase B
git rev-list --count a1bb2111..15c5bb30  -- $P   #  7   Phase L
git rev-list --count 15c5bb30..242085d4  -- $P   # 18   Phase C
git rev-list --count 242085d4..f6c17f92  -- $P   #  4   Phase D
git rev-list --count f6c17f92..56cccca8  -- $P   # 45   Phase E + close-out
git rev-list --count 56cccca8..HEAD      -- $P   #  1   after close
git rev-list --count b1276c39^..HEAD     -- $P   # 139  total — 4+60+7+18+4+45+1
```

Phase A's four out-of-perimeter commits are the ones the `Phase A CLOSED` LOG
entry already names; each was re-resolved to its full subject with `git log -1`.
The per-phase verdicts and every measured figure (4 455 / 4 141 / 311 / 3 at C;
212 of 311 at d1; 4 486 / 1 / 3 of 4 490 at E) are quoted from the §9 LOG entry
that recorded them at the boundary, never re-derived here — the one thing a
reconstruction must not do.

## SPEC-ACTUALIZATION — whole-campaign acceptance {#w1-acceptance}

**Insert point:** a new top-level section appended at the **end of the file**,
after §12's closing paragraph (line 1437, ending `and its own quick-start is
that plan's §10.`). §12 is the verification quick-start; acceptance is its
natural neighbour and reads as its conclusion — the quick-start says how to
re-measure, the acceptance says what the measurement must show.

```markdown
## 13. Whole-campaign acceptance (recorded retrospectively) {#acceptance}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE`
asks every campaign for a runnable script asserting its end state, run on a
green floor at close and cited by the report, and this plan had none — §11's
REPORT scored the predictions without one. The script below was written after
the fact and **its numbers were re-measured at HEAD `fffcb494` on 2026-07-31**,
not copied from the close-out. Where the two differ, both are shown: a closed
campaign whose end state has since improved should say so rather than freeze a
figure.*

    # 1 — the gate panel. Not re-run in the pass that authored this block;
    #     it is a multi-minute build gate and running it is the executing
    #     session's job, not the drafter's.
    bash tools/self-check.sh; echo "EXIT=$?"                    # 0

    # 2 — every observed paragraph carries a marker
    ./target/debug/vibe.exe progress check --exhaustive \
      --campaign campaigns/progress-2026-08                     # clean, 0 warnings
    #   → progress check: clean (260 files, 0 warning(s))   EXIT=0
    #   NOTE: the check covers the whole progress.toml scope, not just this
    #   campaign's 58 files — --campaign selects the state zone, not the
    #   perimeter. Wave 2 shares this gate by design and it must stay at 0.

    # 3 — the host corpus's measured actuality: the campaign's headline
    python campaigns/packages-2026-09/tasks/summary.py | grep '^  host'
    #   → host (58 files)  confi 4496  drift 0  unver 3  total 4499  99.9 %

    # 4 — nothing evaporated: every leftover is a commit or a named deferral
    grep -c '^- ' campaigns/progress-2026-08/deferrals.md       # 13, each owned

    # 5 — the recurrence artifact exists, so the next run costs O(delta)
    test -s campaigns/progress-2026-08/baseline.json; echo "EXIT=$?"   # 0

**What the acceptance shows, and the one thing it cannot.**

- **The end state is better than the close recorded.** At close-out
  (2026-07-26) the ledger read **4 486 confirmed / 1 drift / 3 unverifiable of
  4 490 — 99.9 %**. Today it reads **4 496 / 0 / 3 of 4 499 — 99.9 %**. The
  single surviving drift row, `FACT-GRAIN-EVIDENCE`, closed exactly where
  `deferrals.md` said it would and nowhere else: wave 2's Phase A step 2,
  commit `07a38e1a`. The denominator moved by nine as later work minted anchors
  in host files. *A deferral that closes where its own reasoning said it would
  is the strongest evidence the deferrals ledger is doing its job.*
- **Three units stay `unverifiable` and always will here.** They are
  network-bound GitVerse legacy-registry claims; `unverifiable` was chosen over
  "probably fine" and the choice is the point.
- **What this script cannot assert is the mandate.** The owner asked to
  *actualize all the specifications*; the script proves the corpus is marked,
  judged and drift-free, and proves nothing about whether the resulting
  documents are **useful**. §11's `{#report-gaps}` is where that gap is stated:
  the campaign marked what 4 917 facts *are* and never what should *happen* to
  them, so every forward-looking view came out empty. **An acceptance script
  that passes on an empty view is telling you the view was not part of the
  contract.**
- **Step 3 is served by a tool that lives in the next campaign's zone.** Wave 1
  shipped no summary tool; the number above is printed by
  `campaigns/packages-2026-09/tasks/summary.py`, which reads the shared verdict
  cache. That is a finding, not a convenience: a closed campaign whose headline
  can only be reproduced by its successor's tooling is one re-run away from
  being unmeasurable.
```

**Measurements behind the numbers.** Steps 2–5 were run in this pass at HEAD
`fffcb494` and the outputs above are verbatim. `4 486 / 1 / 3 of 4 490` is the
`PHASE E` LOG entry (§9, line 1242); `4 496 / 0 / 3 of 4 499` is
`summary.py`'s `host` row today; `FACT-GRAIN-EVIDENCE`'s disposition is
`campaigns/progress-2026-08/deferrals.md` §1 `{#blocked}` (*"It closes when
`rust-ai-native-lang` v0.8.0 re-vendors the fact-aware engine, which is wave 2's
Phase A2"*), and the commit that closed it is `07a38e1a feat(specmap): the
fact-grain engine lands, and wave 1's last drift row closes`. The 4 917-fact
figure and the empty views are §11 `{#report-gaps}`. Step 1 was **not** run;
the block says so rather than implying a green floor nobody observed.

---

# PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml — wave 2, LIVE {#w2}

**File:** `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml`, 3 788 lines
at HEAD `fffcb494` (`wc -l`). Sections today: §0 mandate (15) · §1 baseline (34)
· §2 executors (71) · §3 decisions (85, with 3.1 at 91, 3.2 at 119, 3.3 at 136)
· §4 campaign zone (147) · §4.5 amendments (163) · §5 phases (218, with A 220,
B 287, C 305, D 335, E 349, T 357, F 380, G 390) · §6 predictions (413) · §7 LOG
(444) · §8 deferrals (3762) · §9 REPORT (3766) · §10 quick-start (3770).

**The campaign is at Phase D, wave 9 of the stitching, near its exit.** Its
Phase 0 and the A/B/C half of its commit map are retrospective; its safe stop,
risks, non-goals and whole-campaign acceptance are **real going forward** and
bind the sessions that finish D, T, F and G. Five insert points, six blocks.

## PACKAGES-ACTUALIZATION — Phase 0 {#w2-phase-zero}

**Insert point:** inside §5, **after** the heading `## 5. Phases {#phases}`
(line 218) and its blank line, and **immediately before** `### Phase A — Scope
and the fact-grain prerequisite {#phase-a}` (line 220). This plan's §5 has no
lead-in paragraph, so Phase 0 sits directly under the §5 heading.

```markdown
### Phase 0 — what stood before Phase A (recorded retrospectively) {#phase-zero}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##PHASE-ZERO-COMMITS-NOTHING-AND-GATES-EVERYTHING-AFTER`
asks every campaign to open with a no-commit phase, and this one had none.
**No Phase 0 ran.** Written after Phase C closed, so it is a record — of what
stood before Phase A, of what did Phase 0's job under another name, and of the
one thing a real Phase 0 would have spiked that nobody did.*

**The tree before Phase A** (§1's baseline, verified 2026-07-25): **37 packages,
294 `.md` files, 28 733 lines** across two namespaces — `org.vibevm.world` 27
packages / 154 files / 17 104 lines, prompt-only, no crates; `org.vibevm.ai-native`
10 / 140 / 11 629, seven of them carrying `crates/`. **Marker state: zero** —
`grep -rl "<status " packages/` returned nothing, so wave 2 started from nothing
exactly as wave 1 did. The wave-1 machinery already existed and was proven on 58
files and 4 486 units; `progress.toml` was scoped to the host tree alone. Largest
single package: `core-ai-native` at 56 files, bigger than a third of the host
corpus on its own.

**What did Phase 0's job.** Phase A step 1, and it behaved as the law asks —
three of §1's own numbers fell before Phase B committed a marker, and all three
were corrected **in place** rather than noted for later:

| §1 said | measured at A step 1 | why |
|---|---|---|
| 294 files | **286 observable** | eight extractor test fixtures, dropped by `DEFAULT_EXCLUDES` — correctly, since one of each pair is deliberately malformed and marking it would be marking a lie |
| 247 `specmark::scope!` sites | **703** (781 with the superseded slot) | 247 was the **rust family alone**; the join target Phase C verifies against is ~3× what the plan budgeted for, and Phase C's cost scales with it |
| eight packages carry `crates/` | **seven** | `core-ai-native` plus the `-lang` and `-mcp` member of each of the three language families; the three bare umbrellas carry none, which is what makes them the aggregator genre |

Observed total at A step 1: **344 files** (58 host + 286 packages), **13 916
facts**, of which **8 997 unmarked**; `progress check` **0** across both corpora.
*A plan's own numbers are the first thing a campaign about unmeasured numbers
should re-measure*, and this one did — one phase late, but before anything
landed.

**The one spike that was owed and never run.** §5-A step 2 was written as a
release: *"Re-mint `rust-ai-native-lang` (and its typescript / go siblings) at
v0.8.0 … publish, bump the host lockfile"*, and §6's prediction 4 named it **the
single longest-lead item** in the campaign. It was not a release. **The blocker
was a caret**: all three `-lang` stacks required `core-ai-native '^0.7'`, and on
a 0.x version that caret means `>=0.7.0 <0.8.0` — it excluded the very version
everything needed, which is why the lockfile pinned 0.7.0. The fix was three
pins to `^0.8`, three `sync-engines.toml` source roots to v0.8.0, and a
re-vendor: **no new version slot and no publication.** Measured before and
after: **1 041 → 5 267 spec units; fact-targeting edges 0 → 65; unresolved
77 → 12**, because 65 of those "dangling" edges were correct code tags the
unit-grain engine could not see.

*One command in a Phase 0 would have found the caret*, and prediction 4 would
then have been posed against the real work instead of against a release that
never had to happen. The residue is recorded in §5-A step 2 and is still the
owner's: whether the `-lang` slots should eventually be re-minted so a v0.7.0
slot stops carrying 0.8.0 engines.

**The corpus kept moving after Phase B opened**, which a Phase 0 would also have
settled: **344 → 308** (DRIFT-024 removed 33 `LICENSE.xml` by a file-name default
and three derived `cards/INDEX.md` indexes) **→ 259 at Phase C's gate**, as the
superseded version slots, the legacy language projections, the book and the
discovery prompt each left on their own owner ruling and their own reason. Every
removal is defensible and every one was decided mid-flight; the phase that
exists to settle a denominator before anyone counts against it is Phase 0.
```

**Measurements behind the numbers.** 37 / 294 / 28 733 and the per-namespace
split, the `grep -rl "<status " packages/` = 0 result, the 56-file
`core-ai-native` and the version-slot inventory are §1 of the plan itself
(verified there 2026-07-25, with the 247→703 and eight→seven corrections
recorded in place at ratification). 286 observable, 344 files, 13 916 facts,
8 997 unmarked and `progress check` 0 are §5-A step 1. The caret diagnosis,
`^0.7` ⇒ `>=0.7.0 <0.8.0`, the three-pin fix and the 1 041 → 5 267 / 0 → 65 /
77 → 12 measurements are §5-A step 2. The corpus chain 344 → 308 is
`progress.toml`'s own audit comment (*"the corpus is 308 files"*, audited
2026-07-26 on an owner challenge); 259 is `summary.py` at HEAD `fffcb494`
(58 host + 80 ai-native + 121 world).

## PACKAGES-ACTUALIZATION — safe stop {#w2-safe-stop}

**Insert point:** a new top-level section **after** the last line of §4.5
(line 216, the **A6** bullet ending `which is the entire argument §7.5 makes for
keeping the artifact at all.`) and **immediately before** `## 5. Phases
{#phases}` (line 218). §4 is the zone and the crash-safety protocol, §4.5 the
amendments; §4.6 is the stopping law, and it must be read before the phases it
governs. **This block is real going forward** — it binds every session that
finishes D, T, F and G.

```markdown
## 4.6 Safe stop — where this campaign can halt losing nothing {#safe-stop}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##ANY-PHASE-BOUNDARY-IS-A-SAFE-STOP` asks every plan to
say where it can be put down, and this plan said it nowhere. Unlike the
retrospective sections of wave 1's plan, **this one is forward-binding**: a
session that stops anywhere not named here has left work in a state the next
session has to reconstruct.*

**Four grains, from finest to coarsest, and all four hold.**

1. **The step** — mark-file, verify-unit, close-obligation, execute-task —
   journalled `step-start` before and `step-done` after, per wave 1's §4, which
   this campaign inherits unchanged. Step closed ⇒ its edits stand; step open ⇒
   `git restore` its files and redo it. **Maximum loss on any crash: one step.**
2. **The obligation closure** (Phase D's own unit). A closure is *edit the
   document* **and** *re-judge every anchor in its `anchors` list* through
   `merge-verdicts.py --force`, then `vibe progress seal`. Done in that order,
   the registry regenerates from the cache and shrinks by exactly that many
   rows, and stopping between closures loses nothing — `drift-registry.py` reads
   the true remainder whatever a session remembered.
3. **The wave** (d1 … d9). The reviewable unit: a wave ends when every document
   with an open incoming obligation has been through a SPEC task, the registry
   is regenerated with `--write`, and the LOG entry is written **at the
   boundary**. This is the natural place to hand the campaign to a fresh
   session.
4. **The phase boundary.** The full gate panel green — `bash tools/self-check.sh`
   → 0 — **and** `progress check --exhaustive` at 0 over **both** corpora,
   because wave 2 does not un-measure wave 1 and the two share one gate.

**What a stop at each phase boundary leaves:**

| Stopped after | The tree holds | What is owed |
|---|---|---|
| **A** | the widened scope, the campaign zone, three pilot packages marked, and the caret fix that made fact-grain edges exist (1 041 → 5 267 units, 0 → 65 fact-targeting edges) | nothing judged; nothing published |
| **B** | markers only, over 308 files. **No semantic edits by the phase's own law** — a semantic problem found became a finding, not a diff | nothing; the pass is purely additive |
| **C** | **11 346 verdicts, zero owed**, each backed by evidence resolving to a real line in a real file; `baseline.json` written | nothing — Phase C edits no document; a verdict lives in the cache |
| **D** | every drift verdict re-judged or recorded in `run/state/routing.json` as routed out of the package; every survivor carrying an owner ruling | the owner's queue — release, sync-from-code and which-side rulings, named in `PHASE-D-HOST-OBLIGATIONS.md` |
| **T, F, G** | not yet reached | — |

**Five things that are NOT safe stops.** Each fired at least once, and each is
listed with the instance rather than as a caution:

- **A closure that edited the document and did not re-judge its anchors.** The
  registry then reads the obligation as open while the defect is gone, and the
  next wave re-derives an answer that already exists. *Fired 2026-07-31:* the
  registry snapshot on disk was **two waves stale and read as open work**
  (`f2b11b0a`). *The rule that follows:* the registry is generated, never
  hand-edited; the file is a cache and the command is the number.
- **A batch whose verdicts are merged but not sealed.** `merge-verdicts.py`
  refuses to restate a verdict without `--force` by design, and that refusal has
  already caught real mistakes — a session that stops before sealing leaves the
  refusal armed against its own successor.
- **A closure that changed a document's anchor set without running
  `vibe progress mirror` first.** `merge-verdicts.py`'s `addressable()` reads
  `run/mirror/` and will refuse anchors the mirror has not seen.
- **A false `confirmed` "repaired" by editing the document.** The verdict-first
  rule: re-judge it **`drift` first**, let the registry mint the obligation and
  assign its route, and only then close it. *First live test, 2026-07-31,* and
  it paid immediately: the Go GUIDE's `gated_packages` clustered to F-166 on
  **the owner's sync route**, so its two-word swap now waits in the sync queue
  instead of having landed as an unapproved diff. *Editing first and judging
  afterwards is how a boss-route edit lands on an owner-route anchor.*
- **A wind-down that rewrites the files a finished batch cites.** *Fired
  2026-07-28:* W2's four evidence tables were verified clean at 3 unresolvable
  and re-read **65** at the next session's open — `CONTINUE.md` was overwritten
  wholesale and `spec/WAL.xml`'s `_Updated:` line rewritten *after* the tables
  were returned and committed. Not one of the 62 was a fiction, and nobody was
  left who could re-anchor them. **The durable-citation rule exists for exactly
  this**, and the controlled experiment is on record: the one batch written
  before the rule carries 116 dead refs today, and every batch written under it
  verifies clean.

**Where autonomy ends, so a stop is never a guess** (from the Phase D batch
plan §5, unchanged): a `reality-mismatch` closed through sync-from-code needs
**the owner's approval on each spec diff**; a release event goes to the owner
**before publication**; and Rule 4's red lines bind identically whether the boss
does the work or delegates it. **A finding is not a reason to stop** — it opens
an obligation and the wave continues.
```

**Measurements behind the numbers.** The four grains: grain 1 is wave 1's §4,
inherited by this plan's §4; grains 2 and 3 are
`campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md` §3.1 `{#closure}` and §7
`{#gate}`; grain 4 is this plan's §4 scope note (*"both corpora share one
`progress check` gate that must stay at 0"*). The per-boundary table: A's
1 041 → 5 267 and 0 → 65 are §5-A step 2; B's 308 files is `progress.toml`'s
audit comment and the no-semantic-edits law is §5-B; C's 11 346 and zero owed
are the `PHASE C CLOSES` LOG entry; D's routing record is §7 `{#gate}` check 1.
The `f2b11b0a` staleness, the verdict-first rule's first live test and the F-166
routing are the two 2026-07-31 LOG entries; the W2 evidence collapse (3 → 65,
62 new, 116 dead refs in the pre-rule batch) is the `2026-07-28 · The wind-down
that invalidated its own evidence` entry and the `PHASE C CLOSES` entry's
closing paragraph. The autonomy boundary is the batch plan §5 `{#stop}`.

```sh
git log --format='%h %ad %s' --date=short -1 f2b11b0a
#   f2b11b0a 2026-07-31 fix(campaign): the registry snapshot on disk was two waves stale, and it read as open work
```

## PACKAGES-ACTUALIZATION — non-goals {#w2-non-goals}

**Insert point:** two new top-level sections **after** the last line of §6
Predictions (line 442, the sixth prediction ending `against the claim.`) and
**immediately before** `## 7. LOG {#log}` (line 444). This block is §6.5; the
risks block below is §6.6 and shares the same insert point, immediately
following.

```markdown
## 6.5 Non-goals (named, with disposition) {#non-goals}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##NON-GOALS-ARE-NAMED-SO-THEY-STAY-VISIBLE` asks a plan
to name what it deliberately does not do, and this plan named it nowhere. Every
line below is a boundary this campaign is holding **today** — most of them owner
rulings recorded in §7 — and each carries its reason and its disposition per
`##EVERY-NON-GOAL-CARRIES-A-REASON-AND-A-DISPOSITION`. Wave 1's lesson is why
this is worth the page: the two boundaries it held without ever naming (the
judgment axis, the doc trees) are exactly the two that cost it a phase.*

- **Does NOT re-measure wave 1.** *Reason:* the host's 58 files stay in scope
  and their verdicts stand; the two corpora share one `progress check` gate that
  must stay at 0. *Disposition:* settled; §4's scope config.
- **Does NOT touch `packages/org.vibevm.fractality/**`.** *Reason:* its own
  specspace, own boot contract, own WAL. *Disposition:* held by the owner —
  **with a consequence this campaign has already paid.** Wave 6 proved the
  perimeter blind to a **second adopter of the discipline living inside
  `packages/`**, and half that wave's claimed absences were blind to it. The
  exclusion stands; the rule that now stands with it is that a claimed absence
  is measured over the whole tree before it becomes an obligation.
- **Does NOT mark `vibedeps/**`.** *Reason:* regenerated consumer copies of the
  same packages — marking a copy is marking nothing. *Disposition:* rejected
  outright.
- **Does NOT verify superseded version slots.** *Reason:* §3.3 — a superseded
  slot is marked, never verified; verifying frozen history costs what a live
  contract costs and buys nothing. *Disposition:* rejected; `core-ai-native`
  v0.7.0 and `redbook` v0.1.0 left the corpus by exclusion because
  `--exhaustive` cannot express "marked, never verified" (33 files, 1 908 facts
  — 23 % of Phase B's whole workload, on text nothing resolves to).
- **Does NOT admit the book, the legacy language projections, or the discovery
  prompt.** *Reason, and it is the same reason three times and is **not** size:*
  every marker earns a verdict, and `confirmed` has no meaning applied to a
  paragraph of philosophical prose, to a frozen guide nothing cites, or to a
  line of a prompt addressed to another model. *Disposition:* owner rulings
  F-091 («исключи spec/book/**»), F-080 («legacy-projections — это замороженная
  история») and F-096. **The line this draws:** every document that makes a
  claim *about* the artifact stays observed — README, boot snippet, `usage.xml`;
  only the payload leaves, and it leaves because it asserts nothing this project
  could be wrong about.
- **Does NOT soften a package to close an obligation.** *Reason:* it is the one
  answer §3.6 forbids and precisely the *профанация* §0's mandate names — the
  credibility loop cannot be closed by lowering the bar it measures.
  *Disposition:* rejected outright; and mechanically enforced, since a closure
  that does not move the registry did not happen.
- **Does NOT publish.** *Reason:* Rule 4 red line. *Disposition:* the release
  route's obligations wait for the owner, before publication, every time.
- **Does NOT use fractality.** *Reason:* the wave-1 owner decision carries over.
  *Disposition:* held by the owner, **with one recorded exception** — Phase T's
  swarm of the running harness's own subagents, ruled by the owner 2026-07-26
  and recorded rather than assumed.
- **Does NOT re-mint the `-lang` version slots.** *Reason:* called off at Phase A
  step 2 — the blocker was a caret, not a release, and the fix needed no new
  slot and no publication. *Disposition:* still outstanding and still the
  owner's; §5-A step 2 keeps the diagnosis for the day it is taken up, including
  the three things that must be settled first (publication is a Rule 4 red line;
  the host resolves these packages from a second, stale working copy; the
  network registries 401 on this machine).
```

## PACKAGES-ACTUALIZATION — risks and fallbacks {#w2-risks}

**Insert point:** the same point as §6.5 above — **immediately after** the §6.5
block and **before** `## 7. LOG {#log}` (line 444).

```markdown
## 6.6 Risks and fallbacks {#risks}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##EVERY-RISK-CARRIES-A-DETECTION-SIGNAL-AND-A-PLAN-B`
asks every plan to name its risks with a detection signal and a plan B, and this
plan named none. Every risk below has **already fired at least once** in this
campaign — which is what makes each detection signal real rather than
aspirational — and every one is still live for the phases that remain.
`##A-RISK-WITHOUT-A-FALLBACK-IS-A-WISH`.*

- **R1 — the campaign is inside its own corpus.** This campaign writes findings
  into `campaigns/**`, into this file's §7 LOG and into harvest files — all
  inside the tree it measures — so a grep for the very term a finding is about
  matches the finding. *Fired three times in two waves*, most sharply as a
  host-live count of `campaign-plans` sections that showed one hit for **every**
  form, every hit inside this plan, matching only because the LOG entry written
  the day before quoted those words in prose. *Detection:* every count over
  `spec/terraforms/` or `campaigns/` names its perimeter in the sentence that
  reports it. *Fallback:* exclude `campaigns/*/run/**` by default and report both
  numbers — with and without the campaign's own records.
- **R2 — a package-scoped search reads every successful adoption as an
  absence.** *Fired:* wave 5, where **18 claimed absences were false and 17
  fell**; and again in wave 6's mirror image, where the perimeter omitted a
  second adopter of the discipline. *Detection:* §3.7 and its mirror; every
  claimed absence is re-verified over the whole tree before it becomes an
  obligation. *Fallback:* the re-verification is the wave, not an add-on to it —
  a wave that only closes obligations and never re-tests its own premises is
  half a wave.
- **R3 — a false `confirmed` cannot be repaired by editing the document.**
  Editing first and judging afterwards produces a diff on an anchor whose route
  may be the owner's. *Detection:* the verdict-first rule — re-judge `drift`
  first, let the registry mint the obligation and assign its route. *Fallback,
  proven in its first live test on 2026-07-31:* the Go GUIDE's `gated_packages`
  clustered to F-166 on the owner's sync route and now waits in the sync queue
  **instead of landing as an unapproved diff**.
- **R4 — the exit gate depends on rulings only the owner can give.** Measured at
  HEAD `fffcb494`: **210 of 357 drift verdicts are routed out of the package**
  (route b / owner) and only 147 still owe a package repair; the release route
  alone is 10 obligations over 41 drifts and cannot close without a publication,
  which is a Rule 4 red line. *Detection:* `tasks/drift-registry.py`'s route
  table and CONVERGENCE block. *Fallback:*
  [`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md`](../../campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md)
  — the survivors become **owner-ruled deferrals rather than silence**, which is
  the only reading of the exit gate that is not a stall.
- **R5 — a generated artifact quoted from disk goes stale and reads as open
  work.** *Fired 2026-07-31:* the registry snapshot on disk was two waves stale.
  *Detection:* the registry regenerates from the verdict cache; a figure that
  disagrees with `drift-registry.py` is the file's fault, never the cache's.
  *Fallback:* regenerate before quoting — the generated file is a cache and the
  command is the number.
- **R6 — the address family cannot close by editing a package.** It needs a
  publication; the host resolves these packages from a **second, stale working
  copy**; and the network registries 401 on this machine. *Detection:* recorded
  at wave 6 — no address obligation closes without publication, on any route.
  *Fallback:* a local repoint plus a lockfile bump very likely avoids publishing
  altogether, since publication is only needed for external consumers — and that
  is the owner's call, not the executor's.
- **R7 — softening a package to close an obligation.** The failure mode with no
  natural detector, because it looks exactly like progress. *Detection,
  mechanical:* a closure re-judges its anchors through `merge-verdicts.py
  --force` and the registry shrinks by exactly that many rows, so **a closure
  that does not move the registry did not happen** — and `summary.py`'s drift
  count must fall by exactly the number of verdicts the wave's obligations
  carried, with the arithmetic shown. *Fallback:* §3.6's three legitimate
  answers (the host adopts, the host records a deliberate exception, the
  obligation is deferred with the reason on record) — and "edit the package
  until the finding goes away" is not among them.
```

**Measurements behind the numbers.** 357 drift verdicts, 210 routed out, 147
still owed, 152 obligations and the release route's 10 / 41 are
`python campaigns/packages-2026-09/tasks/drift-registry.py` at HEAD `fffcb494`
(run in this pass; output quoted verbatim). The three firings of R1 are
`PHASE-D-BATCH-PLAN.md` `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`. Wave 5's 18
false absences and 17 falls, and wave 6's mirror, are the `2026-07-29 · wave 5`
and `· wave 6` LOG entries. R3's live test and R5's staleness are the two
2026-07-31 entries. R6's three blockers are §5-A step 2 and the wave-6 entry.
R7's mechanics are `PHASE-D-BATCH-PLAN.md` §3.1 `{#closure}` and §7 `{#gate}`
check 3; §3.6's three answers are `PHASE-D-HOST-OBLIGATIONS.md` `{#answers}`.
The 33 files / 1 908 facts and the F-091 / F-080 / F-096 rulings quoted in §6.5
are `progress.toml`'s `exclude` block, which carries each ruling verbatim.

## PACKAGES-ACTUALIZATION — commit map {#w2-commit-map}

**Insert point:** a new sub-section at the **end of §7**, after the LOG's last
line (line 3760, `(d8b + d9), and the owner's queue is the whole remainder.`)
and its blank line, and **immediately before** `## 8. Deferrals {#deferrals}`
(line 3762). §7 is this plan's execution ledger; the map belongs inside it and
appending leaves the LOG's chronology untouched.

**This block has two halves and they are not the same kind of claim.** A/B/C are
executed and their map is a retrospective binding of hashes to phases; D is in
flight; E/T/F/G are **planned commit sets with subjects spelled in advance**,
which is what `##why-subjects-are-spelled-in-advance` asks for — the split of
work into commits-by-meaning happens while the whole change is visible.

```markdown
### 7.1 Commit map — hashes bound to phases {#commit-map}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##EACH-EXECUTED-PHASE-GETS-A-LEDGER-SECTION` and
`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS` ask each phase for a commit
map, and this plan carried none — §7 above records what happened, at length, and
binds it to no hash. The A/B/C entries are reconstructed at Phase D and say so;
`##THE-MAP-IS-WRITTEN-AT-THE-BOUNDARY-NOT-AT-CLOSE` is the rule they miss, and
**from Phase D's close onward this section is written at the boundary**, which
is the whole reason it exists rather than waiting for close-out.*

**Deviation from `##ONE-ENTRY-PER-COMMIT`, stated rather than silent:** 336
commits is past the grain where one entry per commit informs anyone. Each phase
gets its range, its count, its landmark commits and its verdict; the perimeter
command gives the rest.

**Perimeter, so the counts are reproducible.** Measured at HEAD `fffcb494`:

    git log --reverse --format='%h %ad %s' --date=short -- \
      campaigns/packages-2026-09 spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml

**336 commits**, `3aa8295e` (plan authored, 2026-07-25) → `fffcb494`
(2026-07-31), the campaign still open. Over the same span the repository as a
whole took **462** commits (`git rev-list --count 3aa8295e^..HEAD`), so roughly
**seven commits in ten in this repository since 2026-07-25 are this campaign's**.

#### Phase A and Phase B — EXECUTED 2026-07-26/27; 124 commits {#cm-ab}

Preceded by three plan commits: `3aa8295e` author wave 2 · `f723e430` reviewed
against what wave 1 cost · `6ad264da` ratified with all six §4.5 amendments.

`6ad264da`..`fc731127`. **A and B are not separable in this chain, and drawing a
cut would be a fiction**: A's step 2 was deferred by owner ruling («не
перевыпускай пакет, сделаем это потом») and closed later by the version sweep,
so the two phases overlap by construction. Landmarks: `30728dd7` wave 2 opens,
the packages join the observed tree · `27336263` the engine re-mint is deferred
· `3c87cd11` the pilot confirms prediction 2 · `b3ada517` the pilot marks the
aggregator · `fc1782d8` one live zone, wave 2 takes the host corpus's verdicts ·
`56172a8f` Phase B closes at zero · `fc731127` the phase boundary's baseline.

*Confirmed:* **prediction 2** — the aggregator genre needed a grammar amendment,
and the pilot fired it early exactly as §6 said it would. *Falsified in place:*
three of §1's own numbers (294 → 286 observable, 247 → 703 specmark sites,
eight → seven crate-bearing packages). *Falsified about itself:* §5-A step 2's
premise — the blocker was a caret, not a release. *Gate at the boundary:*
`progress check` 0 across both corpora; `baseline.json` written per **A6**.

#### Phase C — EXECUTED 2026-07-28/29; 146 commits {#cm-c}

`fc731127`..`ef40a1ce`. Opens `0dd240bd` (a kick-off that says what Phase C is
not) and `0acc448f` (the batch plan); `c9ae2066` gives the zone the journal it
had run a phase without; `a90cc387` C0. `ai-native` cluster: `38f9816c` C1 ·
`76c6a142` C2 · `6702441a` C3 · `106e09c5` C6 · `bf679a1c` C7 · `6d82b5cf` the
cluster closes at 80 of 80 files. `world` cluster: `d0d17e9e` W1 407 ·
`582f603e` W2 692 · `c75f4216` W3 615 · `0f4d9c94` W4 564 · `0d20fffc` W5 697 ·
`a6436a80` W6 572 · `7c674c18` **PHASE C CLOSES** (W7 603 + qualified-naming's
last 190 anchors) · `ef40a1ce` the exit gate — summary, count, baseline.

*Measured:* **10 700 confirmed / 601 drift / 45 unverifiable = 11 346, 94.3 %**
— by zone, `host` 99.9 %, `ai-native` 91.6 %, `world` 90.0 %. **6 847 / 6 847
anchors, zero owed.** *Falsified:* **prediction 1** — `world` was predicted to
measure *higher* than `ai-native` and measured **lower** (90.0 % against 91.6 %),
and the plan said in advance that an inversion would be worth a finding of its
own. *Confirmed by amendment:* **A2**'s self-referential count is real and small
— 248 of the world zone's 4 150 verdicts rest on source 1 alone, **6.0 %**.
*Method that made it hold:* the per-file slice as the unit of work, and two
instruments that refuse rather than guess (`make-slice.py`, `merge-verdicts.py`).

#### Phase D — IN FLIGHT since 2026-07-29; 63 commits so far {#cm-d}

`ef40a1ce`..HEAD `fffcb494`. Opens `6072033a` (601 drifts become 228
obligations, by a script that says how) and `33bd5b1e`. Landmarks: `d7803b97`
the routing record, without which the phase cannot converge · `8b7f240f` what
the host owes — the other half of the exit gate · `4206c61b` waves 2–4 ·
`b0a8b0d4` wave 5 and §3.7 · `1c1a3865` wave 6 · `3dab12a3` wave 7 closes ·
`3c14d6af` wave 8 · `91ebf1fd` the D9 rulings · `fffcb494` the rulings of
2026-07-31 and the publication runbook.

*Falsified at the opening, and it killed the obvious plan:* drifts were expected
to cluster by reason text; measured, only **16 texts repeat at all over 54
rows** and text-only clustering returns 552 groups for 601 rows — a reduction of
1.1×. What groups them is the **subject**: one document, one kind of defect, one
edit pass. *Falsified mid-phase, twice:* wave 5 found 18 claimed absences false;
wave 6 found the perimeter blind to a second adopter inside `packages/`.
*State at HEAD:* corpus **10 945 / 357 / 44 = 11 346, 96.5 %**, up from 94.3 %
at the Phase C gate; registry **152 obligations / 357 drifts**; **210 of 357
routed out, 147 still owed a package repair, 91 obligations with nothing left
owed**. Reproduce with `tasks/summary.py` and `tasks/drift-registry.py`; both
supersede every figure written here.

#### Phases D-close, E, T, F, G — PLANNED; subjects spelled in advance {#cm-planned}

*Not yet executed. These are the planned commit sets; the ledger binds real
hashes to them as each phase lands, and any drift between the two is itself a
recorded finding.*

- **D close** — `feat(campaign): the routing record closes, and every survivor
  carries an owner ruling` · `docs(campaign): phase D closes — the remainder,
  and who owns each row` · `chore(campaign): the phase boundary's baseline`.
- **E** — one `fix(<package>): <the drift the task closes>` per DRIFT task, each
  whose fix touches a package's crates followed by
  `chore(ai-native): sync-engines vendors the fix forward to every family
  member` — the wave-2-specific obligation, or the fix ships to one consumer and
  not the others.
- **T** — one `test(<package>): three kinds per assertion for <cell>` per
  packet, each packet exhibiting one test red · `docs(campaign): phase T closes
  — measured coverage per testable assertion`.
- **F** — `docs(campaign): the credibility report — does the discipline hold
  itself to its own rule`. One commit, one document, and a green host floor is
  not an answer to that question and may not be cited as one.
- **G** — `refactor(docs): docs/ moves to docs-legacy/ under the legacy-spec
  rule` · `feat(doc): the documentation package — cites a spec unit, never
  restates it` · `docs(campaign): phase G closes — the two guides, and the row
  spec-genres gained`.
- **Close** — `docs(campaign): wave 2 closes — the REPORT against §6's six
  predictions`.
```

**Measurements behind the numbers.** Every count is a `git rev-list --count`
over the perimeter above, at HEAD `fffcb494`:

```sh
P='campaigns/packages-2026-09 spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml'
git rev-list --count 3aa8295e^..6ad264da -- $P   #   3   plan authoring + ratification
git rev-list --count 6ad264da..fc731127  -- $P   # 124   Phase A + Phase B
git rev-list --count fc731127..ef40a1ce  -- $P   # 146   Phase C
git rev-list --count ef40a1ce..HEAD      -- $P   #  63   Phase D so far
git rev-list --count 3aa8295e^..HEAD     -- $P   # 336   total — 3+124+146+63
git rev-list --count 3aa8295e^..HEAD             # 462   the whole repository, same span
```

10 700 / 601 / 45, 94.3 %, the per-zone split, 6 847 / 6 847 and the 248 / 4 150
self-referential count are the `PHASE C CLOSES` LOG entry (line 3106); the
16-texts / 552-groups / 1.1× clustering measurement is the `PHASE D OPENS` entry
(line 3167); 10 945 / 357 / 44 and the registry's 152 / 357 / 210 / 147 / 91 are
`tasks/summary.py` and `tasks/drift-registry.py` run in this pass at HEAD
`fffcb494`. **Note the live drift:** the last LOG entry records `10 941 / 361 /
44` and `153 obligations`, which is what the tools printed when it was written;
they print `10 945 / 357 / 44` and `152` now. That is the campaign moving, not a
contradiction — and it is the reason both figures name their command.

## PACKAGES-ACTUALIZATION — whole-campaign acceptance {#w2-acceptance}

**Insert point:** a new top-level section appended at the **end of the file**,
after §10's closing paragraph (line 3788, ending `and the registry shrinks by
exactly that many rows.`). §10 is the quick-start — how a cold session begins;
§11 is the acceptance — what the campaign must show before it may close. **This
block is real going forward**: it is the contract Phase F is judged against.

```markdown
## 11. Whole-campaign acceptance {#acceptance}

*Added 2026-07-31 under the owner's bring-into-line ruling:
`flow:campaign-plans`' `##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE`
asks every campaign for a runnable script asserting its end state — run on a
green floor at close, cited by the report — and this plan had none. **The
mandate already states the criterion in words**: «this campaign is successful
when the discipline can be shown to hold itself to its own rule». This section
turns that sentence into commands. Steps 1–4 run today and their output is
shown; steps 5–8 assert phases not yet executed and are written now, before
execution, so they cannot be quietly relaxed to fit what lands.*

    # 0 — the gate panel, on a green floor at close
    bash tools/self-check.sh; echo "EXIT=$?"                          # 0

    # 1 — every observed paragraph carries a marker, over BOTH corpora
    ./target/debug/vibe.exe progress check --exhaustive \
      --campaign campaigns/packages-2026-09
    #   → progress check: clean (260 files, 0 warning(s))   EXIT=0

    # 2 — the measured actuality, per namespace and not only in total
    python campaigns/packages-2026-09/tasks/summary.py
    #   → host 4496/0/3 99.9 % · ai-native 2606/72/19 96.6 %
    #     world 3843/285/22 92.6 % (src=[1] 267, 6.4 % self-referential)
    #     ALL  10945 / 357 / 44 = 11346, 96.5 %

    # 3 — Phase D convergence, measured by the generator, never asserted
    python campaigns/packages-2026-09/tasks/drift-registry.py
    #   at close this must read: "drift verdicts still owed a package repair: 0"
    #   — or every survivor carries an owner ruling in PHASE-D-HOST-OBLIGATIONS.md
    test -s campaigns/packages-2026-09/run/state/routing.json          # the routing record

    # 4 — the recurrence artifact, written at every phase close (A6)
    test -s campaigns/packages-2026-09/baseline.json; echo "EXIT=$?"   # 0

    # 5 — Phase T: coverage measured, not claimed
    #     per-packet gate: PHASE-T-SPEC.md §10; campaign-level assertion:
    #     every in-scope testable assertion carries >=3 tests of DISTINCT KINDS
    #     (canonical, boundary, negative) and every packet exhibited one red.

    # 6 — Phase F: the credibility report exists and answers PER PRACTICE
    #     A green host floor is not an answer to this question and may not be
    #     cited as one.

    # 7 — Phase G: docs/ is gone, the doc package exists, and it cites
    test ! -d docs && test -d docs-legacy
    test -d packages/org.vibevm.doc/doc
    #     and the law that makes it worth anything: documentation cites a spec
    #     unit and never restates it; links run one way, docs -> spec.

    # 8 — nothing evaporates
    #     §9 REPORT carries a verdict on each of §6's six predictions, and
    #     campaigns/packages-2026-09/deferrals.md names every leftover with an
    #     owner and a disposition.

**Three things this acceptance deliberately does not let the campaign do.**

- **It does not let a green floor answer the mandate.** §5-F says so and step 6
  repeats it, because that substitution is the exact shape of the *профанация*
  §0 names: the host's gates are supplied *by* these packages, so citing them as
  evidence about the packages is the argument closing on itself.
- **It does not let the drift count reach zero by softening a package.** Step 3
  reads the generator's CONVERGENCE block, which counts verdicts re-judged and
  verdicts **routed out with a recorded determination** — two different numbers,
  neither of which moves when a document is edited to agree with itself.
- **It does not accept a total in place of a per-namespace figure.** Step 2
  prints all three zones, because prediction 1 is a comparison between two of
  them and a single aggregate would make it unscoreable.

**Two gaps this block surfaces rather than papers over**, and both are the
boss's to settle before it lands:

- **Phase F's document has no path.** §5-F describes the report and names no
  file, so step 6 cannot be a `test -s`. Naming it — the way `PHASE-T-SPEC.md`
  and `PHASE-G-SPEC.md` are named from §5 — is a one-line edit and it turns the
  campaign's own headline deliverable from prose into an assertion.
- **The two commands disagree by one file.** `progress check` reports **260**
  observed files; `summary.py` sums **259** with verdicts (58 + 80 + 121). One
  file is observed and carries no verdict row, or the two count differently. It
  is one file out of 260 and it is not a defect on its face — but an acceptance
  script whose two steps disagree should reconcile the difference rather than
  quote whichever number is convenient.
```

**Measurements behind the numbers.** Steps 1–4 were run in this pass at HEAD
`fffcb494` and their outputs are quoted verbatim; step 0 was **not** run (a
multi-minute build gate is the executing session's job, not the drafter's) and
the block says so nowhere by implication — it simply carries the command.
Step 5's «three kinds» and the one-red-per-packet rule are
`PHASE-T-SPEC.md` `##FALSIFY-ONE-PER-PACKET` and §4 `{#kinds}`; its exit gate is
that document's §10. Step 6's «a green host floor is not an answer» is §5-F
verbatim. Step 7's `docs/` → `docs-legacy/` move, the `org.vibevm.doc/doc`
package and the cite-never-restate law are §5-G and `PHASE-G-SPEC.md`. The
260-versus-259 difference is the two commands' own output in this pass.

---

# Summary {#summary}

Twelve blocks, ten insert points (each plan has one point carrying two blocks).
Line counts are of the ready block itself — the markdown between the fences —
not of the surrounding commentary.

| plan | form | insert point | lines |
|---|---|---|---:|
| SPEC-ACTUALIZATION (wave 1, CLOSED) | Phase 0 | §5, before `### Phase A — Scaffold {#phase-a}` (line 104) | 48 |
| SPEC-ACTUALIZATION | safe stop | new §4.5, after §4 item 5 (line 97), before `## 5. Phases` (line 99) | 55 |
| SPEC-ACTUALIZATION | non-goals | new §8.5, after §8's last prediction (line 337), before `## 9. LOG` (line 339) | 49 |
| SPEC-ACTUALIZATION | risks and fallbacks | new §8.6, immediately after the §8.5 block, same point | 67 |
| SPEC-ACTUALIZATION | commit map | new §9.1, at the end of §9 (line 1289), before `## 10. Deferrals` (line 1291) | 134 |
| SPEC-ACTUALIZATION | whole-campaign acceptance | new §13, appended at EOF (after line 1437) | 62 |
| PACKAGES-ACTUALIZATION (wave 2, LIVE) | Phase 0 | §5, after the `## 5. Phases` heading (line 218), before `### Phase A` (line 220) | 61 |
| PACKAGES-ACTUALIZATION | safe stop | new §4.6, after §4.5's A6 bullet (line 216), before `## 5. Phases` (line 218) | 79 |
| PACKAGES-ACTUALIZATION | non-goals | new §6.5, after §6's sixth prediction (line 442), before `## 7. LOG` (line 444) | 58 |
| PACKAGES-ACTUALIZATION | risks and fallbacks | new §6.6, immediately after the §6.5 block, same point | 67 |
| PACKAGES-ACTUALIZATION | commit map | new §7.1, at the end of §7 (line 3760), before `## 8. Deferrals` (line 3762) | 116 |
| PACKAGES-ACTUALIZATION | whole-campaign acceptance | new §11, appended at EOF (after line 3788) | 82 |

**878 lines of block across the twelve**, added to 1 437 + 3 788 = 5 225 lines
of plan — a 17 % growth, concentrated in the two commit maps (250 of the 878).
Counted rather than estimated, by extracting every ` ```markdown ` fence in this
file and measuring its interior:

```sh
python - <<'PY'
lines = open('campaigns/packages-2026-09/harvest/d10-campaign-plans-forms.md',
             encoding='utf-8').read().split('\n')
cur = None
for i, l in enumerate(lines, 1):
    if cur is None and l.strip() == '```markdown': cur = i
    elif cur is not None and l.strip() == '```':
        print(i - cur - 1); cur = None
PY
```

**Every line number is measured at HEAD `fffcb494`** and both plans were clean
in the working tree when measured. HEAD moved once mid-pass (`91ebf1fd` →
`fffcb494`, three commits); neither plan file was among the files those commits
touched, so the section line numbers above are unchanged, and every number in
this document was re-taken at `fffcb494` regardless.

**Order of application, if the boss lands them.** Insert **bottom-up within each
file** — EOF first, then §9.1 / §7.1, then §8.5–8.6 / §6.5–6.6, then Phase 0,
then §4.5 / §4.6 — so every line number in the table above stays valid through
the whole sequence. Applied top-down, each insert invalidates the ones below it.
