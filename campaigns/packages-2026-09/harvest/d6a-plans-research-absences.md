# D6a — `campaign-plans` + `comparative-research`: four claimed absences, re-verified before demotion

_Worked 2026-07-29. Subjects:
`packages/org.vibevm.world/campaign-plans/v0.1.0/` (3 obligations, 12 drift
verdicts) and `packages/org.vibevm.world/comparative-research/v0.1.0/`
(1 obligation, 4 drift verdicts). All four are `build-or-demote`,
`falsifier = host`, `missing-support`, rule `r-zero-instances` — every one
asserts that some form, record or artefact **has no live instance**._

_This batch is worked under
[§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
and [§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness): a demotion is the
**last** step, and a `not-found` is a fact about the search perimeter until the
perimeter has been checked. **Every entry below names the perimeter it
searched.** No code was written; no `git` command that writes was run; nothing
under `run/` was touched._

Obligations: F-163 · F-171 · F-194 (campaign-plans) · F-172
(comparative-research).

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately** — it is a third-party study
corpus, not our shipped surface.

**Why that perimeter and not the one the verdicts used.** Every verdict in this
batch was measured over three plan homes — `spec/terraforms/`,
`legacy-spec/terraforms/`, `legacy-spec/research/` — plus the campaign zones.
That is the host root's plan homes and it is **not** the set of live campaign
plans in this repository. `flow:campaign-plans` is booted by a second consumer
inside this tree: the **`fractality` specspace**, registered in
[`SPECSPACES.md`](../../../SPECSPACES.md), whose own generated boot index reads
the flow at slot 40 —

```console
$ sed -n '36,38p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
[[entry]]
path = "vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md"
kind = "static"
```

— and which carries **six live campaign plans** under
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/`. §3.7's law is
that the artefacts proving adoption live in the CONSUMER; here the consumer that
adopted the format hardest is a specspace inside `packages/`, which every
verdict's `spec/terraforms/ legacy-spec/ campaigns/` perimeter excluded by
construction.

---

## F-163 — five forms called «0 live instances»; every one of them is live, in the specspace that boots this flow

**Outcome:** RE-JUDGE: confirmed (5 of 5) — five false absences, no edit
**Anchors:** 0 of 5 moved. Unedited and confirmed: `##ROW-ROLE-LOG`,
`##PHASE-ZERO-PRODUCES-NO-COMMITS-AND-LATER-PHASES-CARRY-FOUR-ELEMENTS`,
`##the-quick-start-block-lead`,
`##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE`,
`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS`. All five are defined in
`CAMPAIGN-PLAN-FORMAT.md` (lines 42, 173, 200, 214, 232), not merely cited.
**Files touched:** none
**Perimeter searched:** the standing perimeter above, for `commit map` ·
`commit-map` · `^#+ .*quick.?start` · `^#+ .*(whole.campaign|campaign)
acceptance` · `^\*+Commits:` · `^\*+Exit:` · `^\*+Prediction` · `Phase 0` ·
`Execution ledger`. The decisive widening over every verdict in this obligation
is `packages/org.vibevm.fractality/**`, which no verdict's perimeter
(`spec/terraforms/ legacy-spec/terraforms/ legacy-spec/research/ campaigns/`)
could reach. `refs/**` reported separately at the end of the entry.

**What the search found:**

The verdict for `##ROW-ROLE-LOG` says «*`grep -niE 'commit map|commit-map'` over
both live plans and both campaign zones returns 0, against 3 archived plans
carrying the form completely*». Over the standing perimeter, with the vendored
`vibedeps/` and `.vibe/cache/` copies of this very package excluded:

```console
$ PERIM="packages vibedeps crates xtask tools spec discipline terraform research campaigns legacy-spec fixtures schemas docs manual-tests"
$ grep -rlniE 'commit map|commit-map' $PERIM *.md *.toml *.json *.sh *.ps1 \
    --exclude-dir=target --exclude-dir=node_modules --exclude-dir=run \
    --exclude-dir=.vibe --exclude-dir=vibedeps | sort -u
…
packages/org.vibevm.fractality/CLAUDE.md
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md
…
```

Three live campaign plans, in a project that boots this flow at slot 40. The
form is not approximated — it is the canonical one, under the canonical section
number and title:

```console
$ grep -c "^### Phase .* — EXECUTED .*; commit map" \
    packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md
8
$ grep -c "^- \*\*Ф[0-9]* — EXECUTED .*Commit map:\*\*" \
    packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
5
```

**`##ROW-ROLE-LOG`** asks the LOG to carry three things — per-phase commit maps,
deviations, discovered work. All three are in one ledger:

- per-phase commit maps —
  `FRACTALITY-IGNITION-PLAN-v0.1.md:1139` (`## 14. Execution ledger`), then
  `:1141`, `:1155`, `:1210`, `:1274`, `:1323`, `:1383`, `:1417`, `:1441` — eight
  `### Phase N — EXECUTED (date); commit map` sections, 19 hashes bound;
- deviations —
  `FRACTALITY-INITIATIVE-PLAN-v0.1.md:666-670` *«**Drift vs plan: one feat commit
  instead of the two planned** — the field addition, the MC subsystem, and the
  CLI attribution surface compile only together; splitting them would have
  manufactured a non-building intermediate commit for ceremony's sake.
  Ledgered, not absorbed»*;
- discovered work — `FRACTALITY-INITIATIVE-PLAN-v0.1.md:695` *«a discovered
  split: main.rs crossed the 600-line budget with the new verb surface and the
  conform gate caught it — the mc verb family moved to its own `mc_cmd.rs`
  cell; ledgered as in-phase discovered work, not silent»*.

**`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS`** — the strongest of the
five, because the live instance restates the anchor almost word for word:

```console
$ sed -n '615,618p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
## 14. Execution ledger

_Filled at each phase boundary: commit map (hash → planned subject),
what each commit confirmed or falsified, drift notes._
```

The binding is real and checkable in both directions. `FRACTALITY-IGNITION-PLAN-v0.1.md:894`
plans *«`feat(fractality): cargo workspace + core model`»*; `:1163` lands
*«`bd1e65d` feat(fractality): cargo workspace + core model»* — same string.
`FRACTALITY-INITIATIVE-PLAN-v0.1.md:690-691` books *«`6f5788a` —
`feat(fractality): initiative engine — scoreboard render (C2 Ф2)` (planned
subject #1 …)»* and `:694-695` *«(planned subject #2 + a discovered split …)»* —
the ledger names the planned subject by number. The third plan is the one CLAUDE.md calls the active
machinery plan: `FRACTALITY-RLM-PLAN-v0.1.md:225-227`
(`## 9. Ledger {#ledger}` / *«Commit map (Stage B execution, Campaign 3):»*),
30 hashes bound to Ф-numbered planned subjects. **55 hashes bound to planned
subjects across the three, against the verdict's «0 live plans».**

**`##PHASE-ZERO-PRODUCES-NO-COMMITS-AND-LATER-PHASES-CARRY-FOUR-ELEMENTS`** —
both halves are live, and the Phase 0 half is stated in the phase heading
itself:

```console
$ sed -n '735,737p' …/FRACTALITY-IGNITION-PLAN-v0.1.md
## 8. Phases

### Phase 0 — spikes and probes (no commits; findings rewrite Decisions)

$ sed -n '788p' …/FRACTALITY-IGNITION-PLAN-v0.1.md
#### Phase 0 findings — EXECUTED 2026-07-09 (all green; no commits)

$ sed -n '428p' …/FRACTALITY-INITIATIVE-PLAN-v0.1.md
**Ф0 — spikes and probes (no commits).**
```

and the four elements are carried by every later phase of the IGNITION plan —
eight `*Commits:*`, eight `*Exit:*`, nine `*Prediction:*` markers, each above
numbered steps. Phase 1 is the worked case: numbered steps 1–5 at `:871-886`,
then `*Exit:*` `:889`, `*Prediction:*` `:892`, `*Commits:*` `:894` naming three
subjects in advance. The INITIATIVE plan carries the same four under a prose
dialect — `:448` *«Planned commits: `feat(fractality): sessions — MC registry,
journal, API` · `feat(fractality): session attribution — env stamp + session
verbs`»*, with `Exit:` on the next line.

```console
$ for f in packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md; do
    printf "Commits:%s Exit:%s Prediction:%s\n" \
      "$(grep -ciE '^\*+Commits:' $f)" "$(grep -ciE '^\*+Exit:?\*' $f)" "$(grep -ciE '^\*+Prediction' $f)"; done
Commits:8 Exit:8 Prediction:9
```

**`##the-quick-start-block-lead`** — this one is false twice over. It restates
`##COLD-A-LITERAL-QUICK-START-BLOCK`, which the owner already ruled on (§7 LOG,
2026-07-29, «the first route-(b) ruling»): the host yielded and both live plans
in `spec/terraforms/` gained the section. Independently, three fractality plans
carried it all along:

```console
$ grep -rniE '^#+ .*quick.?start' <standing perimeter> | grep -v legacy-spec | grep -viE 'CONTINUE|README|guides|harvest'
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md:1087:## 11. Quick-start for the executing session
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md:554:## 11. Quick-start for the executing session
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md:393:## 11. Quick-start for the executing session {#quick-start}
packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md:198:### 11 — Quick-start for the executing session {#s11-quick-start}
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3442:## 10. Quick-start for the executing session {#quick-start}
spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md:1420:## 12. Quick-start for the executing session {#quick-start}
```

`FRACTALITY-IGNITION-PLAN-v0.1.md:1087-1098` is a literal ```` ```sh ```` block
doing exactly the three things the anchor's three sub-bullets name — confirm the
tree (`git log --oneline -5`), verify the green floor (`rust-ai-native floor`),
capture the baseline (`head -20 WAL.md`).

**`##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE`** — two live
instances, and one of them reproduces the package's own example line
character-for-character:

````console
$ sed -n '1101,1113p' …/FRACTALITY-IGNITION-PLAN-v0.1.md
## 12. Whole-campaign acceptance

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
rust-ai-native floor                                                 # exit 0 (D15)
fractality mc start && fractality mc status                          # healthy
fractality run --packet spec/examples/hello-glm.toml                 # exit 0
test -s ~/.fractality/runs/<that-run>/result.md                      # non-empty result
fractality stats                                                     # ≥5 completed runs, ≥1 swarm parent w/ 3 children
fractality questions                                                 # empty at close
ls spec/manual-tests/                                                # 5 recorded procedures with outputs
(cd ../../../.. && bash tools/self-check.sh); echo "EXIT=$?"          # host floor green
```
````

The package's own §12 example is `<gate-panel command>; echo "EXIT=$?"    # exit 0`
(`CAMPAIGN-PLAN-FORMAT.md:218`). The live plan's last line is
`bash tools/self-check.sh); echo "EXIT=$?"          # host floor green`. The
second instance is `FRACTALITY-INITIATIVE-PLAN-v0.1.md:566-578`, a five-command
block ending in the MT index.

**`refs/**`, reported separately:** `grep -rniE 'commit map|commit-map' refs/`
returns nothing. The `^#+ .*quick.?start` sweep over `refs/` returns five hits,
all third-party skill and agent docs (`refs/src/agent-scripts/**`) — not
campaign plans and not an instance of ours.

**Which layer has it, if any:** **consumer deployment**, and specifically a
*second* consumer the verdicts never entered — the `fractality` specspace, which
installs `flow-campaign-plans/0.1.0` into its own `vibedeps/` and reads its boot
snippet at slot 40. Three of its six plans (`IGNITION`, `INITIATIVE`, `RLM`)
carry every form this obligation calls absent; `RLM-RESEARCH` carries the
quick-start.

**What changed and why:** nothing, and that is the finding. Every one of the
five claimed absences is false on a perimeter that includes the whole tree.
Demoting any of them would have written «specified, not built» over a form this
repository practises in three executed campaigns and 55 hash-to-subject
bindings — §3.7's failure mode exactly, one directory deeper than the wave-5
cases. Note also that none of the five promises a *mechanism*: they prescribe a
document shape, so even had the absence survived, §3.3's demotion is the wrong
instrument and §3.6's route (b) would have been the disposition — the shape the
owner already ruled on for `##COLD-A-LITERAL-QUICK-START-BLOCK`.

**New obligations noticed:** (1) the two live host-root plans in
`spec/terraforms/` still carry **no** commit map — `grep -niE 'commit map'
spec/terraforms/*.md` returns one line, `PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:3030`,
and that is this campaign quoting its own finding. That is a genuine host
non-compliance and belongs in `PHASE-D-HOST-OBLIGATIONS.md` under §3.6(b), the
same disposition the owner gave the quick-start gap — but it does **not**
support an absence claim, because the form is live elsewhere. (2) The Phase-C
verdicts under this obligation, and under F-155 / F-195 in the earlier
`prose-edit` pass, all measured «live plans» as `spec/terraforms/*.md`. The
`fractality` specspace is a registered live sub-project of this repository with
six plans; any future measurement of «how this host writes plans» that excludes
it is wrong by 6 documents in 8.

**Verdict recommendation, per anchor:**
`##ROW-ROLE-LOG` → **demoted? no — confirmed**; three live ledgers carry all
three contents, 13 per-phase commit-map sections between them.
`##PHASE-ZERO-PRODUCES-NO-COMMITS-AND-LATER-PHASES-CARRY-FOUR-ELEMENTS` →
**confirmed**; the no-commits clause is in two live Phase 0 headings and the
four elements are carried by every later phase of the IGNITION plan.
`##the-quick-start-block-lead` → **confirmed**; six live sections, three of them
predating the owner's ruling that fixed the other two.
`##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE` → **confirmed**; two
live `sh` blocks, one reproducing this package's own example line.
`##THE-LEDGER-BINDS-HASHES-TO-THE-PLANNED-SUBJECTS` → **confirmed**; 55 hashes
bound to planned subjects, and one ledger's own lead restates the anchor.

---

## F-171 — four verdicts restated to agree with each other, agreeing on a premise the fractality plans falsify; and one sub-claim false on the verdict's own perimeter

**Outcome:** RE-JUDGE: confirmed (4 of 4) — four false absences, no edit
**Anchors:** 0 of 4 moved. Unedited and confirmed:
`##a-phase-carries-four-elements-lead`, `##why-subjects-are-spelled-in-advance`,
`##EXECUTION-STAGES-ONTO-A-PRE-DRAWN-MAP`,
`##SUM-EVERY-LATER-PHASE-CARRIES-FOUR-ELEMENTS`. All four are defined in
`phase-gates.md` (lines 59, 93, 97, 199).
**Files touched:** none
**Perimeter searched:** the standing perimeter, for `^\*+Commits:` ·
`Planned commits` · `planned subject` · `commits.by.meaning` ·
`commit.by.meaning` · `one logical unit per commit` · `drift vs plan` ·
`deltas vs plan` · `deviation`, plus a full read of `## 8. Phases` and
`## 14. Execution ledger` in the three executed fractality plans. `refs/**`
reported separately.

**Why this obligation was re-verified as a set rather than row by row.** §3.7's
corollary: *when a verdict says it was restated for consistency, re-verify the
whole set.* Three of these four say so in their own words —
`##SUM-EVERY-LATER-PHASE-CARRIES-FOUR-ELEMENTS` closes *«Same measurement as the
anatomy lead, and the summary carries its verdict»*, and
`##EXECUTION-STAGES-ONTO-A-PRE-DRAWN-MAP` reasons *«binding requires a pre-drawn
map, only 4 archived plans have one»* — the identical premise. The premise is
the one F-163 already falsified, so the family falls together, exactly as the
six `BUILD-ORDER` verdicts did in wave 5.

**What the search found:**

**`##a-phase-carries-four-elements-lead`** and its summary twin
**`##SUM-EVERY-LATER-PHASE-CARRIES-FOUR-ELEMENTS`** both rest on «the commit set
with subjects spelled in advance appears in 4 archived files and **0 live
plans**». The fourth element is spelled in advance in two live plans, once per
phase:

```console
$ grep -niE '^\*Commits:' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md
785:*Commits:* exactly one — `docs(fractality): plan v0.1 amended with Phase 0
894:*Commits:* `feat(fractality): cargo workspace + core model` ·
918:*Commits:* `feat(fractality): profiles + clean-slate worker env` ·
937:*Commits:* `feat(fractality): result collection, metering, sync run` ·
957:*Commits:* `feat(fractality): swarm — async lifecycle, budgets, kill-tree`
978:*Commits:* `feat(fractality): pod broker — permissions and questions` ·
1007:*Commits:* `feat(fractality): delegation-rules — matrix + model playbooks`.
1031:*Commits:* `feat(fractality): boss boot snippet + delegation skill` ·
```

Eight, against the verdict's zero — and the second live plan carries the same
element under the label `Planned commits:` / `Planned:`
(`FRACTALITY-INITIATIVE-PLAN-v0.1.md:448`, `:459`, `:469`). The other three
elements are in the same phases: numbered steps `:871-886`, `*Exit:*` `:889`,
`*Prediction:*` `:892` for Phase 1 alone. All four elements, in one
self-contained phase, in a live plan — which is precisely what the lead claims
and what the summary restates.

**`##EXECUTION-STAGES-ONTO-A-PRE-DRAWN-MAP`** is a three-clause sentence and all
three clauses have live instances. The pre-drawn map is the `*Commits:*` list
above. The binding is §14 (F-163, 55 hashes). The third clause — *«any drift
between the two is itself a recorded finding»* — is the one the verdict conceded
was «attested» only in the archive, and it is attested twice in the live window:

```console
$ sed -n '666,672p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
    (C2 Ф1)`. **Drift vs plan: one feat commit instead of the two
    planned** — the field addition, the MC subsystem, and the CLI
    attribution surface compile only together; splitting them would
    have manufactured a non-building intermediate commit for
    ceremony's sake. Ledgered, not absorbed (P7 bookkeeping).

$ sed -n '18,20p;24,27p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
**Commit range:** `47412ad` (campaign open) → the close commits,
19 fractality-scoped commits total (15 planned; see P7). Executed
across two sessions on 2026-07-10: …
**Per-phase deltas vs plan:** Ф1/Ф3/Ф4 each folded their planned
multi-commit shape into one feat commit (compile-coupled cells;
ledgered per phase, never absorbed). Ф2 gained one discovered split
(`mc_cmd.rs`, the conform 600-line budget). …
```

*19 landed against 15 planned*, reconciled per phase, with the drift booked
against a numbered prediction (P7). That is the anchor's whole sentence
executed.

**`##why-subjects-are-spelled-in-advance`** — two separate corrections, and the
first needs no widening at all. The verdict claims *«`grep -rniE
'commits-by-meaning|commit.by.meaning'` across the spec tree, the archive and
the git-atomic-commits package returns 0, so the phrase the rationale rests on
is the flow's alone.»* Re-run over the first perimeter the verdict itself names:

```console
$ grep -rniE 'commits.by.meaning|commit.by.meaning' spec/
spec/boot/00-core.md:23:3. ##RULE-ATOMIC-GROUPING **Group commits by meaning** — one logical unit per commit, split mixed working trees. @impl/done
```

One hit, in the host's own boot contract, as **Rule 3**, in bold, read at every
session start. The phrase is not the flow's alone; it is the host's standing
rule and the flow's rationale points at it. This is a `not-found` that fails on
its author's own stated perimeter, not on a widened one. Two further live
echoes: `packages/org.vibevm.fractality/reports/2026-10-07-21-11-defc2slice-started-plan.md:33`
and `…-21-30-defc2slice-completed-plan.md:29` both list *«commits grouped by
meaning»* as a phase acceptance item.

The second correction is the sentence's own object: *«no live plan writes
subjects in advance»* is false by the eight `*Commits:*` lines above.

**A marker note, because it changes what §3.3 could even do here.**
`##why-subjects-are-spelled-in-advance` already carries `@spec/done`
(`phase-gates.md:95`), not `@impl/done`. §3.3's closure is the move
`@impl/done → @spec/done`; there is nothing to move. Any edit here would have
had to invent a lower state, which the form does not have.

**`refs/**`, reported separately:** `grep -rniE 'commits.by.meaning' refs/`
returns nothing.

**Which layer has it, if any:** **consumer deployment** — the same fractality
specspace as F-163 — for the four elements, the pre-drawn map and the drift
findings; and the **host's own boot lane** (`spec/boot/00-core.md:23`) for the
commits-by-meaning rule the rationale cites.

**What changed and why:** nothing. The family's shared premise — «the commit set
with subjects spelled in advance has 0 live instances» — is false, and it was
propagated across four verdicts by consistency, which is the §3.7 corollary's
exact signature. One anchor additionally fails on a claim its own author could
have checked inside the perimeter he named. As with F-163, none of the four
promises a mechanism: they describe how a phase is written, so §3.3 was never
the right instrument even if the absence had held.

**New obligations noticed:** the host-root live plans in `spec/terraforms/`
carry neither `*Commits:*` nor a per-phase commit map, so the four-element rule
is genuinely unkept **there**. That is one host-compliance obligation, already
implied by F-155's route-(b) record in the `prose-edit` pass, and it is the same
disposition the owner gave the quick-start gap. It is not an absence of the
form.

**Verdict recommendation, per anchor:**
`##a-phase-carries-four-elements-lead` → **confirmed**; all four elements are
carried by every post-Ф0 phase of two live plans.
`##why-subjects-are-spelled-in-advance` → **confirmed**; the rationale's phrase
is the host's Rule 3 at `spec/boot/00-core.md:23`, and eight live phases spell
subjects in advance. Already `@spec/done`; nothing to demote.
`##EXECUTION-STAGES-ONTO-A-PRE-DRAWN-MAP` → **confirmed**; map, binding and
recorded drift all live, `19 landed / 15 planned` reconciled per phase.
`##SUM-EVERY-LATER-PHASE-CARRIES-FOUR-ELEMENTS` → **confirmed**; it restates the
lead and inherits its correction — the restatement was the mechanism by which
the false premise spread.

---

## F-194 — the boot snippet's three cold-start rules, each with a live instance in a plan that names this flow as its format

**Outcome:** RE-JUDGE: confirmed (3 of 3) — three false absences, no edit
**Anchors:** 0 of 3 moved. Unedited and confirmed:
`##COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS`, `##COLD-A-RUNNABLE-ACCEPTANCE-SCRIPT`,
`##EVERY-LATER-PHASE-ENDS-GREEN-AND-EVERY-BOUNDARY-IS-A-SAFE-STOP`. All three
are defined in `spec/boot/40-flow-campaign-plans.md` (lines 36, 42, 53).
**Files touched:** none
**Perimeter searched:** the standing perimeter, for `exit state` · `exit-state` ·
`safe stop` · `safe-stop` · `Target arithmetic` · `Boundary evidence` ·
`Floor at the boundary` · `resume pointer` · `^#+ .*(whole.campaign|campaign)
acceptance`. `refs/**` reported separately.

**The single fact that decides all three, stated before the evidence.** Both
executed fractality plans declare, in their own status lines, which format they
are written to:

```console
$ grep -n "Format: \`flow:org.vibevm.world/campaign-plans\`" \
    packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/*.md
FRACTALITY-IGNITION-PLAN-v0.1.md:9:(one file, five roles). Campaigns 2–3 draw their mandates from §15._
FRACTALITY-INITIATIVE-PLAN-v0.1.md:11:(one file, five roles) ·
```

These are not plans that happen to resemble the format. They **cite the package
by name as their contract**, in the section the package's §1 defines. A verdict
that reports «0 live instances» while two documents in the tree name this flow
as the standard they follow has measured the wrong window.

**What the search found:**

**`##COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS`** — the verdict concedes the
baseline half and says *«The exit half has no live instance at all: `grep -niE
'exit state' spec/terraforms/*.md` returns 0»*. Over the standing perimeter:

```console
$ grep -rniE 'exit state|exit-state' <standing perimeter, vendored copies excluded>
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md:32:**§4 exit state, reconciled:** 1 code package `fractality` v0.1.0 with
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md:251:Exit state of this campaign:
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md:144:**Exit state:**
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md:330:## 8. Baseline and exit state {#baseline}
packages/org.vibevm.fractality/reports/campaign-close.md:21:## What exists now (the §4 exit state, reconciled)
…
```

`FRACTALITY-IGNITION-PLAN-v0.1.md:245-268` is a `## 4. Target arithmetic`
section under the canonical number, whose baseline is five exact counts
(*«packages … = **1** (spec-only; 0 crates, 0 binaries); policy flow packages =
**0**; boss-side boot snippets/skills = **0**; recorded E2E proofs = **0**»*)
and whose `Exit state of this campaign:` is six enumerated units with exact
counts (**6 crates**, **3 binaries**, ≥ 3 playbooks, **5 recorded E2E proofs**,
0 clean-room violations).

The **reconciliation** the anchor demands — «every baseline unit ends in the
exit state or in a phase that removes it» — is then discharged unit by unit at
close:

```console
$ sed -n '32,40p' …/FRACTALITY-IGNITION-PLAN-v0.1.md
**§4 exit state, reconciled:** 1 code package `fractality` v0.1.0 with
exactly 6 crates / 3 binaries ✅ · 1 policy package `delegation-rules`
v0.1.0 with matrix + 3 playbooks ✅ · boss boot snippet + 1 skill ✅ ·
**5 recorded E2E proofs** (MT-01 sync collect-back · MT-02 3-worker
swarm · MT-03 recursive kill · MT-04 question round-trip incl. the
11-minute park · MT-05 dogfood relicense with live merges) ✅ — all
five signed off by the owner 2026-07-10 · floor green, host
self-check green, 0 clean-room violations ✅.
```

That is the SHRINK-PLAN pattern the verdict called «adopted, then dropped»,
practised in a campaign that closed on 2026-07-10.

**`##COLD-A-RUNNABLE-ACCEPTANCE-SCRIPT`** — falsified by the same evidence as
F-163's `##ACCEPTANCE-IS-A-RUNNABLE-SCRIPT-ASSERTING-THE-END-STATE`, which it
restates: `FRACTALITY-IGNITION-PLAN-v0.1.md:1101` and
`FRACTALITY-INITIATIVE-PLAN-v0.1.md:566`, both `## 12. Whole-campaign
acceptance`, both a literal ```` ```sh ```` block of end-state assertions, the
first ending in this package's own example line
(`bash tools/self-check.sh); echo "EXIT=$?"`). The verdict's own count («8
archived plans, 0 live plans and 0 campaign zones») is short by two live
instances.

**`##EVERY-LATER-PHASE-ENDS-GREEN-AND-EVERY-BOUNDARY-IS-A-SAFE-STOP`** — three
clauses, and the verdict grants only the first. All three hold:

*(i) every later phase ends with the full gate panel green* — granted by the
verdict, and independently attested per phase in both live plans: eight
`**Boundary evidence.** Floor: …` blocks in
`FRACTALITY-IGNITION-PLAN-v0.1.md` (`:1175`, `:1222`, `:1292`, `:1333`,
`:1389`, `:1422`, `:1452`) and five `Floor at the boundary: **all green**`
entries in `FRACTALITY-INITIATIVE-PLAN-v0.1.md` (`:677`, `:704`, `:727`,
`:762`, `:787`).

*(ii) any phase boundary is a safe stop* — the verdict says *«stated in 12
archived plans and **0 live ones**»*. It is stated in the **status line** of a
live plan, in the package's own §1 wording:

```console
$ sed -n '11,13p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md
Format: `flow:org.vibevm.world/campaign-plans` (one file, five roles) ·
cold-executable: any phase boundary is a safe stop; the floor is green at
every boundary. Lineage: drains DEF-1 (+ two named IGNITION leftovers)
```

Compare `CAMPAIGN-PLAN-FORMAT.md:83-84`, the package's §1 example:
*«cold-executable: any phase is a safe stop; the floor is green at every
boundary.»* Two more live statements:
`FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md:285` (*«Every phase boundary is a safe
stop; the gate panel for a docs-only phase is "specmap green + the named
artifacts committed"»*) and `FRACTALITY-FUGU-RESEARCH-PLAN-v0.1.md:119`. The
law is also carried as a shipped discipline anchor one package over —
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/04-SWEEP-PLAYBOOK.md:161`
`##ANY-SINGLE-ITEM-IS-A-SAFE-STOP`, projected into all three language stacks'
sweep skills.

*(iii) the plan plus its execution ledger are the resume pointer* — the verdict
argues the live campaign uses `PHASE-C-RESUME.md` and a generated
`run/RESUME.md` instead. In the fractality campaigns the pointer is exactly what
the anchor names: the plan's status line carries the campaign state
(`FRACTALITY-IGNITION-PLAN-v0.1.md:3` *«Status: **CLOSED** (Phases 0–6 executed
2026-07-09/10 …)»*), and each §14 entry names the phase that landed and the
next one (`FRACTALITY-INITIATIVE-PLAN-v0.1.md:687` *«Next: Ф2 (scoreboard engine
+ verbs)»*). Where a journal also exists, the package already governs the
arrangement rather than being contradicted by it —
`phase-gates.md:136` `##the-journal-points-at-the-plan`: *«the journal points at
the plan; the plan file, not the journal, carries the campaign detail»* — and
that is precisely what `FRACTALITY-IGNITION-PLAN-v0.1.md:1091` does in its own
quick-start: `head -20 WAL.md  # PLANNED/EXECUTING + next phase pointer`.

**`refs/**`, reported separately:** the `exit state` sweep over `refs/` returns
two hits, both Rust source strings in `refs/src/warp/**` (*"recording exit state
poisoned"*) — an unrelated sense of the words. The `safe.stop` sweep returns
nothing relevant. Neither is an instance of ours.

**Which layer has it, if any:** **consumer deployment** for all three — the
`fractality` specspace, which installs `flow-campaign-plans/0.1.0`, reads its
boot snippet at slot 40, and names the flow in the status line of every plan it
writes. Additionally **the host's own boot lane** for the wording under test:
`spec/boot/STATIC.md:87` and `:96-97` carry this snippet's two sentences
verbatim, so the host has adopted the rules into its contract even where its own
`spec/terraforms/` plans do not yet keep them.

**What changed and why:** nothing. Three claimed absences, three live instances,
and in two of the three the live instance is closer to the package's wording
than the archived instance the verdict offered as the lost gold standard. A
demotion here would have marked as unbuilt three rules that a campaign in this
repository executed, gated on, and closed against six weeks ago.

**New obligations noticed:** (1) the exit-state and acceptance gaps in
`spec/terraforms/`'s two plans are real and are a host obligation of the same
class the owner already ruled on for the quick-start —
`PHASE-D-HOST-OBLIGATIONS.md:74` already books the family («commit maps 3 / 0,
safe stop 12 / 0, Phase 0 five archived and none live»), and those ratios should
be re-measured against the fractality plans before that record is acted on:
measured by file over the full tree they are **4 / 3**, **13 / 3** and **12
archived / 2 live**, with the commands in this record's closing section.
(2) `##COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS` and
`CAMPAIGN-PLAN-FORMAT.md:115` `##BASELINE-AND-EXIT-STATE-ARE-EXACT-COUNTS-THAT-RECONCILE`
are the same rule in two packages' files and were judged separately on the same
false measurement; whatever is concluded about one should move the other.

**Verdict recommendation, per anchor:**
`##COLD-BASELINE-AND-EXIT-AS-EXACT-COUNTS` → **confirmed**; a live `## 4. Target
arithmetic` with an exact-count exit state, reconciled unit by unit at close.
`##COLD-A-RUNNABLE-ACCEPTANCE-SCRIPT` → **confirmed**; two live `## 12.
Whole-campaign acceptance` shell blocks, one reproducing the package's own
example line.
`##EVERY-LATER-PHASE-ENDS-GREEN-AND-EVERY-BOUNDARY-IS-A-SAFE-STOP` → **confirmed
on all three clauses**; the safe-stop law is in a live plan's status line in this
package's own §1 wording, and the plan-plus-ledger resume pointer is the one the
fractality campaigns actually use.

---

## F-172 — the research→decision pipeline measured over one study; a second research programme runs all four steps, verdict vocabulary included

**Outcome:** RE-JUDGE: confirmed (4 of 4) — four false absences, no edit. One
carries a scope caveat recorded below rather than an edit.
**Anchors:** 0 of 4 moved. Unedited and confirmed: `##ROW-VERDICT-ACCEPT`,
`##this-is-decision-records-doing-its-job`,
`##RECORD-FIELD-CONSIDERED-AND-REJECTED`,
`##THE-STUDY-NOTES-THE-ACCEPTANCE-AND-POINTS-AT-THE-ANCHOR`. All four are defined
in `from-research-to-roadmap.md` (lines 49, 65, 75, 81).
**Files touched:** none
**Perimeter searched:** the standing perimeter, for `Considered and rejected` ·
`**Rejected:**` · `Non-adoptions` · `When to revisit` · `Revisit when` ·
`revisit trigger` · `do nothing` · `status quo` · `\bAccept(ed)?\b`, **plus a
full read** of `ROADMAP.md` §M1.7–M1.11, `legacy-spec/research/PROP-004-tessl-comparative-research.md`
§6 and §8, and — opened rather than asserted about, per the LOG's note on the
previous worker — **both** `*-SYNTHESIS.md` files and the mapping deliverable
beside them. `refs/**` reported separately.

**The scoping error under all four verdicts.** Each measured the pipeline over
one study — Tessl's `PROP-004`, an **archived** document
(`legacy-spec/research/`, `<status stage="doc" state="done" comment="B0
2026-07-24: evergreen research reference (Tessl comparative study); deltas
ratify downstream"/>`) — and treated the RLM side as a second instance of the
same shortfall. There is a second, complete comparative-research programme in
this tree: the fractality specspace runs **two** research campaigns
(`FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md`, `FRACTALITY-FUGU-RESEARCH-PLAN-v0.1.md`),
authored explicitly to this flow —

```console
$ sed -n '3,7p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/RLM-SYNTHESIS.md
_Ф5 deliverable of
[`FRACTALITY-RLM-RESEARCH-PLAN-v0.1`](../../plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md),
form per D-R6 (flow:comparative-research: two-way gaps, numbered
deltas, re-fetch list). Synthesized 2026-07-11 from the 11 study
notes in this directory …
```

— and both synthesis files carry the `## 2. Two-way gaps` section the LOG
records a previous worker denying without opening them
(`RLM-SYNTHESIS.md:40`, `FUGU-SYNTHESIS.md:34`). Every step this obligation
calls absent runs there.

**What the search found:**

**`##ROW-VERDICT-ACCEPT`** — *«Accept | We will do this | Becomes a recorded
decision (below)»*. The verdict says acceptance is *«observable as a LANDING and
nowhere as a verdict»*. The verdict column is used, all three values, with the
human's words attached:

- **Accept** — `FRACTALITY-RLM-PLAN-v0.1.md:72` `## 4. Decisions (seeded from
  synthesis; finalized at commissioning) {#decisions}`, ten numbered decisions
  `D-C3-1 … D-C3-10`, each a recorded decision born from named deltas.
- **Defer** — `packages/org.vibevm.fractality/plans/postponed/PP-003-option-c-advisor-slice.md:1-6`:
  *«Status: **POSTPONED** (owner cut it from Campaign 3 Stage B — «отдельная
  задача, запланируй») · Origin: RP-C3-1 ruling (Stage B plan §1, §8); scope
  options §3; decision D-C3-7.»* — a delta marked deferred, with the deciding
  human's words verbatim and a revisit trigger at `:30` *«(revisit trigger:
  first field data — RD-10)»*. Four siblings sit beside it (`PP-001`, `PP-002`,
  `PP-004`, `PP-005`).
- **Reject** — `FRACTALITY-RLM-PLAN-v0.1.md:81-82` *«Rejected: prompt-embedded
  judgment (unauditable, untrainable).»* and `:140-142` *«Rejected: a learned
  router in v1 (RD-20 defers the lever; …).»*

And the review itself is a recorded, dated human act rather than an inference:

```console
$ sed -n '207,212p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md
- **RP-C3-1 — mandate & scope cut (RULED 2026-07-11 → Option B):**
  owner verbatim «Вариант 1. Вариант плана - B (нисхождение +
  эскалация). Вариант C с адвайзором - отдельная задача,
  запланируй.» Scope = descent + ascent (V2+V3); the advisor (V4)
  is cut to [PP-003](../../../../plans/postponed/PP-003-option-c-advisor-slice.md).
```

That is `##DELTAS-DO-NOT-RATIFY-OR-AUTO-SCHEDULE-THEMSELVES` and
`##a-human-decides-one-of-lead` executed on the record — an owner reading a
numbered table and saying which deltas travel which branch.

**`##this-is-decision-records-doing-its-job`** — *«the delta's argument … becomes
the record's Why, and the delta's number and the study's title become the
citation»*. Both halves, in both research programmes:

```console
$ sed -n '74,82p' …/FRACTALITY-RLM-PLAN-v0.1.md   # elided at the "…" below
- **D-C3-1 Need-gate verb** (RD-1, RD-2, RD-6, RD-16; FD-1): one
  auditable MC/boss call with typed verdict `inline | route |
  fold-local | spawn | escalate` + journaled reason — `route` is
  the cheap tier …
  Rejected: prompt-embedded judgment
  (unauditable, untrainable).
```

The parenthesis is the delta numbers as citation — literally the field the
anchor names. All ten decisions carry one. On the Tessl side the citation is the
study's title and link, at the living spec's own anchor:

```console
$ sed -n '12p' spec/modules/vibe-resolver/PROP-003-dep-evolution.md
##revision-r2 **Revision r2 (2026-05-04, post-PROP-004).** First revision shipped 2026-05-04 morning. Second revision shipped same day after the [PROP-004 Tessl comparative research](…) surfaced eight architectural improvements that were better folded into the design proposal *before* implementation than retrofitted later. Diff at the section level: @spec/done
```

followed by the eight, each its own anchored decision — `##r2-delivery-modes`,
`##r2-description-field`, `##r2-broadened-probes`, `##r2-llm-refactor`,
`##r2-describes-purl`, `##r2-conditional-deps`, `##r2-exclusive-groups`,
`##r2-activation-conflict` (`:14-21`). **That is the acceptance recorded as a
verdict, dated, at the spec anchor, naming the study** — not a landing from which
acceptance is inferred. The verdict for this anchor calls the Tessl citations
*«inline prose references, not `decision-records` records»*; `##revision-r2` is a
dated record of a review with eight enumerated outcomes, which is the form.

**`##RECORD-FIELD-CONSIDERED-AND-REJECTED`** — the verdict's own command,
re-run verbatim:

```console
$ grep -rn 'Considered and rejected' ROADMAP.md spec/
spec/boot/STATIC.md:254:| **Considered and rejected** | One line per alternative, each carrying its rejection reason. |
spec/modules/vibe-cli/PROP-036-package-tree.md:93:- ##decision-artifacts-rejected **Considered and rejected:** recomputing `EffectiveBoot` fresh every run —
spec/modules/vibe-progress/PROP-043-progress-markup.md:96:- ##element-name-rejected **Considered and rejected:** `progress` (HTML collision), `vp`/`prg`
spec/modules/vibe-progress/PROP-043-progress-markup.md:139:- ##freeze-rejected **Considered and rejected:** `stage="done"` (ambiguous against
spec/modules/vibe-progress/PROP-043-progress-markup.md:251:   - ##registers-rejected **Considered and rejected:** single UPPER
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:111:consumer that does not behave as promised. **Considered and rejected:**
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:129:run it. **Considered and rejected:** trusting the host's green floor as
```

Seven hits, where the verdict reports *«no hit»*. At **study-derived** landings
specifically, the field is the house style of the entire research corpus —
**fourteen** study notes carry a named non-adoption section:

```console
$ grep -rn 'Non-adoptions' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/
barkain-study.md:98:## Non-adoptions (named)
codex-first-study.md:82:## Non-adoptions (named)
conductor-study.md:60:**Non-adoptions:** NL-emitted workflows as our execution format
context-folding-study.md:63:**Non-adoptions:** FoldGRPO training is out of scope …
dnc-noise-study.md:67 · fast-rlm-study.md:89 · fugu-study.md:140 · openfugu-study.md:74
rao-study.md:61 · recursive-llm-study.md:71 · redel-study.md:85 · rlm-study.md:130
roma-study.md:97 · srlm-study.md:56 · trinity-study.md:61
```

The complete four-field record, all in one anchor, exists too —
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md:81`
carries *«Considered and rejected (owner, 2026-07-26): folding case for
duplicate detection.»* with the reason **and** *«Revisit when: a case-only
collision is observed to mislead a reader in practice.»*

On the sentence's own trailing clause — *«including "do nothing", if the study
weighed it»* — the verdict says no located study weighs it. The status-quo
alternative is weighed explicitly at three landings:
`spec/common/PROP-031-algorithmic-refactoring.md:110` `@fact:REJ-LLM-REWRITE **LLM
free-form file-rewriting (the status quo).** The problem, not a solution …`;
`spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md:166`
`@fact:REJ-WHOLE-FILE **The whole-file overwrite** (the shipped Phase-4 status quo).
Rejected …`; and
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/scaffold-d-differential-oracle.md:52`
`@fact:ALTERNATIVE-MANUAL-REVIEW *Manual review:* the status quo; fails exactly where
we need it`. The clause is conditional («if the study weighed it») and is
therefore not falsifiable by an absence in any case.

**`##THE-STUDY-NOTES-THE-ACCEPTANCE-AND-POINTS-AT-THE-ANCHOR`** — the verdict's
claim is absolute: *«the back-annotation does not exist anywhere»*. It exists, as
a whole deliverable:

```console
$ sed -n '33,40p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/FUGU-FRACTALITY-MAPPING.md
## 2. The mapping table {#table}

| Fugu finding (FD) | Ours it lands on | Verdict → action |
|---|---|---|
| FD-1 two-tier route/conduct | RD-1 need-gate; D-C3-1 | **changes** — verdict set gains `route` (cheap single-worker dispatch); applied. |
| FD-2 access lists | RD-5/RD-8; D-C3-2 | **changes** — packets gain explicit `context_from` result-refs; applied. |
| FD-3 orchestration collapse; … | RD-7 single-writer; RD-11 clean context; D-C3-5 | **changes** — sibling visibility default = isolated …; applied. |
…
```

Sixteen rows, one per delta, each naming **the decision anchor it landed at**
(`D-C3-1`, `D-C3-2`, `D-C3-5`, …) and its disposition, under a verdict
vocabulary the document declares in its own header (`:9-13`: *«confirms /
strengthens / changes / new … Changes are APPLIED to the draft in this same
commit»*). §3 then lists the applied changes by decision number. This is the
research side pointing at the decision's anchor, at delta granularity, for a
whole study.

**The scope caveat, stated rather than edited.** The practice is one document
removed from the anchor's literal *«at the delta»*: `FUGU-SYNTHESIS.md` §3's
delta list is not itself annotated — the mapping is the separate Ф6 deliverable
its own header points at (`FUGU-SYNTHESIS.md:8-10`). And the other two studies do
not back-annotate at all: `RLM-SYNTHESIS.md:8-10` says only *«The deltas PROPOSE;
acceptance happens in Stage B»*, and `PROP-004` §6's deltas carry *«Maps to
§5.x»* — a pointer **into the study**, never out to a decision. So the anchor's
form is kept once and not three times. That remainder is a consumer-compliance
gap under §3.6(b), not an absence: it is precisely «the rule is sound and the
consumer does not always keep it», and the package does not move for it.

**`refs/**`, reported separately:** the `Considered and rejected` sweep over
`refs/` returns four hits — `refs/src/bazel/**`, `refs/src/warp/**` — third-party
engineering specs practising the same field. Not instances of ours and not
counted above.

**Which layer has it, if any:** **consumer deployment**, twice over — the living
spec (`spec/modules/vibe-resolver/PROP-003-dep-evolution.md`'s `##revision-r2`
block for the Tessl programme) and the `fractality` specspace's research corpus
(two research plans, 20 study notes, two syntheses, one mapping analysis, ten
recorded decisions, five postponed-delta records) for the RLM/Fugu programme.
**Nowhere** for per-delta back-annotation inside `PROP-004` and
`RLM-SYNTHESIS.md` — a compliance gap, and the only real absence in this
obligation.

**What changed and why:** nothing. All four verdicts measured a four-step
pipeline over the one study whose own status line says its deltas *«ratify
downstream»* and which is archived, and read the fractality programme only
through two files. Opened in full, that programme runs every step the obligation
calls missing: numbered deltas, a human verdict per delta with the ruling
verbatim, accepted deltas landing as numbered decisions that cite the delta
numbers, rejected alternatives with reasons, deferrals with revisit triggers, and
a delta→anchor back-annotation table. Demoting here would have marked as unbuilt
the pipeline that produced ten of the decisions the active campaign is executing
against.

**New obligations noticed:** (1) the back-annotation gap named above —
`PROP-004` §6 and `RLM-SYNTHESIS.md` §3 carry no per-delta acceptance pointer —
is a genuine host/specspace obligation under §3.6(b), and the cheapest possible
repair: 21 RD deltas and 15 Tessl deltas each gaining one clause. Worth booking
in `PHASE-D-HOST-OBLIGATIONS.md` rather than re-derived next wave. (2)
`ROADMAP.md`'s Tessl milestones carry `Source: [PROP-004 §5.x]` and a `✅`
per-slice scope list but no revisit trigger; the record shape is kept in
`spec/modules/**` and not in `ROADMAP.md`, which is the same route-(b) gap at a
different landing. (3) `legacy-spec/research/PROP-004-tessl-comparative-research.md`
is archived while `spec/modules/vibe-resolver/PROP-003-dep-evolution.md:12` cites
it as the live justification for eight shipped design decisions; whether an
archived study may be a live citation target is a question for the owner, not
this batch.

**Verdict recommendation, per anchor:**
`##ROW-VERDICT-ACCEPT` → **confirmed**; all three verdict values are exercised on
the record, `RP-C3-1` carrying the owner's ruling verbatim and `PP-003` carrying
a deferred delta with its revisit trigger.
`##this-is-decision-records-doing-its-job` → **confirmed**; ten decisions cite
their delta numbers in parentheses and `##revision-r2` cites the study's title at
the spec anchor.
`##RECORD-FIELD-CONSIDERED-AND-REJECTED` → **confirmed**; the verdict's own
command returns seven hits, fourteen study notes carry a named non-adoption
section, and the status-quo alternative is weighed at three anchors.
`##THE-STUDY-NOTES-THE-ACCEPTANCE-AND-POINTS-AT-THE-ANCHOR` → **confirmed, with
the scope caveat recorded**; `FUGU-FRACTALITY-MAPPING.md` §2 is a 16-row
delta→anchor→disposition table, so «does not exist anywhere» is false. The two
studies that do not back-annotate are a §3.6(b) consumer obligation, not a
missing mechanism.

---

## Summary of dispositions

| obligation | anchors | moved | outcome |
|---|---:|---:|---|
| F-163 | 5 | 0 | RE-JUDGE: confirmed — 5 false absences |
| F-171 | 4 | 0 | RE-JUDGE: confirmed — 4 false absences |
| F-194 | 3 | 0 | RE-JUDGE: confirmed — 3 false absences |
| F-172 | 4 | 0 | RE-JUDGE: confirmed — 4 false absences (1 with a scope caveat) |
| **total** | **16** | **0** | **16 of 16 claimed absences did not survive** |

**No file was edited.** No demotion was applied, no `@impl/done` marker moved, no
verdict JSON was written, nothing under `run/` was touched, and no `git` command
that writes was run.

**Why the rate is 16/16 rather than the ~1-in-4 §3.7 predicts.** These four
obligations are not four independent measurements. They are one measurement —
«how does this repository write campaign plans and research documents» — taken
four times over the same window, and the window was wrong in the same way each
time: it was `spec/terraforms/` + `legacy-spec/` + `campaigns/`, which is the
host root's plan homes and not the set of live plans in this tree. The
`fractality` specspace installs `flow-campaign-plans` and `flow-comparative-research`,
boots both, names them in the status line of every plan it writes, and holds six
campaign plans, two research plans, twenty study notes, two syntheses and a
mapping analysis. **Twelve of the sixteen verdicts are falsified solely by files
under `packages/org.vibevm.fractality/`**; the other four are falsified there and
in the host as well. This is §3.7 exactly — the artefacts that prove adoption
live in the consumer — with the twist that the consumer here is a **sub-project
of the host**, not the host root, so even a perimeter that reached `discipline/`
and `terraform/` would still have missed it.

**Three of the sixteen are false without any widening at all**, on the perimeter
the verdict itself named: `##why-subjects-are-spelled-in-advance`'s
*«`grep -rniE 'commits.by.meaning' …` across the spec tree … returns 0»* (it
returns `spec/boot/00-core.md:23`, Rule 3), and
`##RECORD-FIELD-CONSIDERED-AND-REJECTED`'s *«`grep -rn 'Considered and rejected'
ROADMAP.md spec/` returns no hit»* (seven hits), which also carries
`##this-is-decision-records-doing-its-job`'s record half.

**What the boss owes the routing record, if these are accepted.** Nothing is
routed out of a package here — all sixteen anchors are recommended
**confirmed**, which is a re-judge, not a route. Three genuine host-compliance
gaps surfaced on the way and belong in `PHASE-D-HOST-OBLIGATIONS.md` rather than
in a package edit: the two `spec/terraforms/` plans carry no commit map, no
`*Commits:*` per phase, and no exit-state arithmetic; and `PROP-004` §6 plus
`RLM-SYNTHESIS.md` §3 carry no per-delta back-annotation. All three are §3.6(b) —
the rule is sound, the consumer does not keep it — and the owner has already
ruled that shape once on this same package.

**One measurement in the existing record needs re-taking before it is acted on.**
`PHASE-D-HOST-OBLIGATIONS.md:74` books the family as «commit maps 3 / 0, safe
stop 12 / 0, Phase 0 five archived and none live». Re-measured over the full
tree, counting **files**, and excluding
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` — which matches all
three patterns only because line 3030 quotes this very finding:

```console
$ P=packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans
$ grep -rlniE 'commit map|commit-map' legacy-spec/terraforms/ legacy-spec/research/ | wc -l   # archived
4
$ grep -rlniE 'commit map|commit-map' $P | wc -l                                              # live
3
$ grep -rlniE 'safe stop|safe-stop|safe stopping' legacy-spec/terraforms/ legacy-spec/research/ | wc -l
13
$ grep -rlniE 'safe stop|safe-stop|safe stopping' $P | wc -l
3
$ grep -rliE '^#+ .*(phase 0|wave 0)|^\*\*Ф0' legacy-spec/terraforms/ legacy-spec/research/ | wc -l
12
$ grep -rliE '^#+ .*(phase 0|wave 0)|^\*\*Ф0' $P | wc -l
2
```

So the true ratios are **commit maps 4 / 3**, **safe stop 13 / 3**, **Phase 0
12 archived / 2 live** — not «/ 0» in any of the three. The host obligation is
still real for the two host-root plans in `spec/terraforms/`, which carry none of
the three; the ratio that framed it as an abandoned practice is what does not
survive.
