# Packages-Actualization Campaign v0.1 — wave 2: the authored packages measure themselves {#root}

<status stage="impl" state="work" comment="RATIFIED 2026-07-26 with all six §4.5 amendments adopted; Phase D closed 2026-08-03 at a green floor; E/T/F/G designed and awaiting the owner's word"/>

**status: RATIFIED 2026-07-26 · PHASE D CLOSED 2026-08-03 (A+B closed 2026-07-27, C closed 2026-07-29, D closed 2026-08-03 — floor green, CONVERGENCE met: 17 owed drifts, every one on an owner-ruled build; next: Phase E on the owner's word) · all six [§4.5](#amendments) amendments adopted · wave 2 of the Progress-Control programme, the sibling of [SPEC-ACTUALIZATION-CAMPAIGN-v0.1](SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md) (wave 1, host `spec/`, closed out 2026-07-26)**

Contract for everything used here: [PROP-043](../modules/vibe-progress/PROP-043-progress-markup.md).
Owner's manual: [OWNER-GUIDE](../modules/vibe-progress/OWNER-GUIDE.md).
Task formats: [templates/](../modules/vibe-progress/templates/impl-task.md).
The method, proven end to end on 58 files and 4 486 units:
[wave 1's plan and LOG](SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md#log).

---

## 0. Mandate (owner's words, 2026-07-25, recorded verbatim) {#mandate}

> «Пожалуйста, запланируй проверку всех пакетов внутри `org.vibevm.world` и
> `org.vibevm.ai-native` тем же способом, которым мы делали ВЕСЬ этот процесс
> (переработка спецификаций на факты с гранулярным разделением, верификация,
> и так далее). Причина: у нас большая часть кода должна быть покрыта
> практиками ai-native, и без проверки самой ai-native всё это выглядит как
> профанация.»

Two things are being asked, and only the first is obvious:

1. **Apply wave 1's method to the packages** — fact-grain markup, evidence-based
   verification, stitching, coding tasks.
2. **Close the credibility loop.** The host tree is disciplined *by* these
   packages. If the packages themselves are unverified, every gate the host
   passes rests on an unmeasured foundation. The owner's word for that is
   *профанация*, and it is the acceptance criterion: this campaign is
   successful when the discipline can be shown to hold itself to its own rule.

## 1. Baseline (verified at authoring time, 2026-07-25) {#baseline}

| Namespace | Packages | `.md` files | Lines | Shape |
|---|---|---|---|---|
| `org.vibevm.world` | 27 | 154 | 17 104 | prompt-only; no crates |
| `org.vibevm.ai-native` | 10 | 140 | 11 629 | 7 of 10 carry `crates/` |
| **total** | **37** | **294** | **28 733** | |

Of those 294 `.md` files, **286 are observable**: eight are extractor test
fixtures under a `fixtures/` directory, which `DEFAULT_EXCLUDES` drops even
under an explicit include (PROP-043 §4). Measured 2026-07-26 at Phase A step 1.

- **Marker state: zero.** `grep -rl "<status " packages/` = 0 files. Wave 2
  starts from nothing, exactly as wave 1 did.
- **Code-side traceability already exists** in the ai-native crates: **703
  `specmark::scope!` / `#[spec(…)]` sites** across the live version slots
  (781 counting the superseded `core-ai-native` v0.7.0). That is the join
  target Phase C verifies against — it is *not* evidence that the prose is true.
  *(Corrected at ratification, 2026-07-26. This line read **247**, which is the
  **rust family alone** — `rust-ai-native-lang` + `rust-ai-native-mcp` measure
  248 today. The join target is ~3× what the plan budgeted for, and Phase C's
  cost scales with it. A plan's own numbers are the first thing a campaign
  about unmeasured numbers should re-measure.)*
- **Seven of the ten carry `crates/`**, not eight (counted at ratification):
  `core-ai-native` plus the `-lang` and `-mcp` member of each of the three
  language families. The three bare family umbrellas — `rust-ai-native`,
  `go-ai-native`, `typescript-ai-native` — carry none, which makes them the
  aggregator genre §5-A's prediction is about.
- **Version slots.** All of `world` is `v0.1.0` except `redbook` (v0.1.0 +
  v0.2.0) and `wal` (v0.2.0 only). In `ai-native`, `core-ai-native` carries
  **two live slots** (v0.7.0 consumed, v0.8.0 authored); the rust family is
  v0.7.0, typescript v0.6.0, go v0.1.0.
- **Largest single package:** `core-ai-native` at 56 files — bigger than a
  third of the host corpus on its own.
- Out of scope: `packages/org.vibevm.fractality/**` (its own specspace, its
  own contract) and `vibedeps/**` (regenerated consumer copies).

## 2. Executors and the budget law {#executors}

Unchanged from wave 1, and for the same reasons:

| Role | Who | What |
|---|---|---|
| Boss / high-level | **Fable** | markup passes, verification judgment, stitching, task authoring, ALL review |
| Coder | **Opus** (`claude-opus-5`) | IMPL tasks (DRIFT-NNN) exactly as written; stop-rule on ambiguity |
| Spec editor | budget-dependent | SPEC tasks; Fable reviews regardless |

**No fractality for this campaign** — the wave-1 owner decision carries over.
Rules 1–4 of the repository bind every executor; non-routine red lines stop
for the owner whether the work is done directly or handed off.

## 3. Two decisions that make this campaign different {#decisions}

Wave 1 could lean on one rule: *`impl/done` means the thing is present in
`crates/`*. Neither namespace lets that rule stand unmodified, and pretending
otherwise would produce exactly the profanation the mandate names.

### 3.1 A prompt-only flow's facts are contracts on behaviour {#world-verdicts}

**Decision.** In `org.vibevm.world`, a fact is verified against **three
sources, in this order**, and a verdict without one of them is
`unverifiable` — never "probably true":

1. **The package's own shipped artifacts.** A boot snippet that claims a rule
   must contain it; a protocol document a snippet cites must exist and say
   what the snippet says it says. This is mechanical and catches the largest
   class: a flow whose boot lane promises a section its protocol never grew.
2. **The host's observed conformance.** `flow:wal` claims the session reads
   `spec/WAL.md` first — the host's `CLAUDE.md`, `spec/boot/`, and this
   repository's own commit history either show that or do not. The host is a
   living consumer and the honest test bench.
3. **The installed reality.** `vibe install` writes `files_written`; the
   lockfile records what a consumer actually receives. A flow that specifies
   an artifact it never ships is drift, and this catches it.

**Why:** a behavioural contract has no `crates/` to point at, but it is not
therefore unfalsifiable — it is falsified by its own artifacts and by a
consumer that does not behave as promised. **Considered and rejected:**
marking all of `world` `spec/done` and calling it verified (that is the
profanation); and demanding a checker per flow (most of these rules are for a
reader, and a rule with no checker is a WISH — but *some* WISHes are correct
and the honest record says so rather than inventing machinery).
**Revisit when:** a third of `world`'s units land `unverifiable` — that would
mean the three sources are too weak and the genre needs a fourth.

### 3.2 The discipline is verified by running it on itself {#ai-native-verdicts}

**Decision.** In `org.vibevm.ai-native`, every fact about a checker, a gate,
or an engine is verified by **executing it against the package's own crates**,
not by reading its prose. The floor (`rust-ai-native floor`), `conform`,
`specmap` and the health collector are run over `packages/org.vibevm.ai-native/**`
and the run output is the evidence, captured as a doc fixture.

**Why:** this is the mandate's core. A discipline that cannot pass its own
gate has no standing to gate anything else, and the only way to know is to
run it. **Considered and rejected:** trusting the host's green floor as
transitive evidence (it proves the *host* conforms, not that the checkers are
correct); and a separate audit document (an audit that is not the gate rots
at the same rate as the prose). **Revisit when:** the discipline ships a
self-hosting mode that makes the run automatic — then this becomes a CI row,
not a campaign phase.

### 3.3 Superseded version slots are marked, never verified {#versions}

**Decision.** When a package carries more than one version directory, the
campaign judges the **live** slot — the one a consumer resolves — and marks
the superseded slot with a single document-level marker recording that it is
frozen history. `redbook` v0.1.0 and `core-ai-native` v0.7.0 are the two cases
today. **Why:** verifying frozen history costs the same as verifying live
contracts and buys nothing; and a superseded slot with fresh-looking verdicts
invites a reader to act on it. **Revisit when:** a superseded slot is
resurrected for a consumer that cannot upgrade.

## 4. Campaign zone {#layout}

```
campaigns/packages-2026-09/        # id fixed at Phase A close
  baseline.json · deferrals.md · harvest/ · tasks/ · run/
```

Same shape, same laws, same crash-safety protocol as wave 1
([§4 there](SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md#resume)) — step = unit of
atomicity, journal `step-start` before and `step-done` after, `RESUME.md`
regenerated, maximum loss on any crash is one step.

**Scope config.** `progress.toml` gains the two package globs. The host's 58
files stay in scope: wave 2 does not un-measure wave 1, and the two corpora
share one `progress check` gate that must stay at 0.

## 4.5 Amendments carried in from wave 1's close-out {#amendments}

*Proposed at ratification, 2026-07-26, from what wave 1 actually cost rather
than from what it planned. Wave 1's REPORT (§11 there) carries the evidence for
each.* **ADOPTED IN FULL by the owner, 2026-07-26 — all six bind this campaign,
and A1 and A4 change phases that follow, so read them before opening C.**

- **A1 — every phase's exit gate enumerates that phase's own steps.** Wave 1's
  Phase C listed "harvest cards written while knowledge is hot" among its steps
  and gated only on "100 % of markers carry verdicts". The step was skipped, it
  cost nothing at the time, and Phase G arrived to consume an empty directory
  and had to be deferred. **This plan has the identical defect today:** §3.2
  says the checker runs are "captured as a doc fixture", and §5-C's exit gate
  says only that markers carry verdicts. A phase gate that does not check the
  phase's own steps will skip whichever ones nothing downstream fails on.
- **A2 — a verdict's evidence must name which source class it rests on.** F-063
  — a security-relevant drift in the token precedence — survived wave 1's
  verification with a `confirmed` verdict because its evidence cross-checked
  one spec document against another spec document carrying the identical error.
  §3.1 makes document-against-document verification the *method* for `world`,
  legitimately, because a prompt-only flow has no crate to point at. That makes
  the hazard structural here rather than accidental: each `world` verdict must
  record which of §3.1's three sources it used, and **a verdict resting only on
  source 1 (the package's own artifacts) is self-referential** — the package
  agreeing with itself — and is marked as such rather than counted as
  independent confirmation.
- **A3 — wave 2 inherits two phases wave 1 could not run.** Both were deferred
  by owner ruling on 2026-07-26 after close-out measured their inputs missing
  (wave 1's `deferrals.md` §6). They must appear here as phases or they will be
  lost: **(i)** a *judgment-marking pass* — wave 1 marked what 4 917 facts
  **are** and never what should **happen** to them, which left every
  forward-looking view empty (`freeze/plan` 0, `action="rework"` 0,
  `stage="idea"` 0); wave 2 should mark judgment as it marks state, in one
  sweep over both corpora. **(ii)** the *harvest pass and the two doc trees*
  (User Guide, Package Author Guide) — the Package Author Guide in particular
  belongs here, since `packages/` is the corpus it documents.
- **A4 — fix the hand-seal staleness gap before Phase C, not after (F-067).**
  `processed_hash` records the text a file's verdicts were computed against and
  is only written by a real verify batch; wave 1 sealed by hand throughout, so
  the staleness warning ended up pointing at the *freshest* files in the corpus.
  Wave 2 will hand-seal thousands of verdicts across 294 files. Unfixed, the
  signal is noise from the first row.
- **A5 — every prediction names the step that tests it.** Wave 1's prediction 1
  was confirmed only because close-out went and measured it specially: no step
  of the plan required a `weave`, so the claim sat untested for the whole
  campaign. A prediction with no step behind it is one this campaign will also
  finish without testing. §6's five predictions each need a step, or a note
  saying they are scored at close-out on purpose.
- **A6 — write `baseline.json` at every phase close, now that a writer exists.**
  Wave 1 could not: `Baseline::store` was claimed by the spec and had never been
  built (F-065), so the recurrence loop had never run end to end. It exists as
  of 2026-07-26 and round-trips clean. Wave 2 is the first campaign that can
  checkpoint cheaply — a crash or a long gap should cost O(delta), which is the
  entire argument §7.5 makes for keeping the artifact at all.

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
  wholesale and `spec/WAL.md`'s `_Updated:` line rewritten *after* the tables
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

## 5. Phases {#phases}

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
settled: **344 → 308** (DRIFT-024 removed 33 `LICENSE.md` by a file-name default
and three derived `cards/INDEX.md` indexes) **→ 259 at Phase C's gate**, as the
superseded version slots, the legacy language projections, the book and the
discovery prompt each left on their own owner ruling and their own reason. Every
removal is defensible and every one was decided mid-flight; the phase that
exists to settle a denominator before anyone counts against it is Phase 0.

### Phase A — Scope and the fact-grain prerequisite {#phase-a}

*Entry:* this plan ratified. *Steps:*

1. Widen `progress.toml`; confirm `scan` sees the package files and reports
   them all unmarked. **DONE 2026-07-26 — and the expected number was wrong:
   `scan` observes 286 package files, not 294.** The eight difference are
   extractor test fixtures
   (`{go,ts}-ai-native-{lang,mcp}/…/tools/*-extract/test/fixtures/{clean,dirty}/spec/PROP-001.md`),
   excluded by `DEFAULT_EXCLUDES`' always-on `fixtures` rule (PROP-043 §4) —
   correctly, since one of each pair is *deliberately malformed* and marking it
   would be marking a lie. Observed total **344 files** (58 host + 286
   packages), **13 916 facts**, of which **8 997 unmarked** — the package
   corpus is 1.8× the host in facts while being only 1.07× in lines, so
   prediction 5's "comparable cost" reads right on lines and light on facts.
   `progress check` **0** across both corpora.
2. **DEFERRED as a re-mint — and then it turned out no re-mint was needed.**
   *Owner ruling 2026-07-26: «не перевыпускай пакет, сделаем это потом»; then,
   on the version sweep: «просто обнови все до самой свежей версии».* The
   second instruction closed the gap the first had deferred. **The blocker was
   a caret, not a release:** all three `-lang` stacks required
   `core-ai-native '^0.7'` while the current version is 0.8.0, and on a 0.x
   version that caret means `>=0.7.0 <0.8.0` — it excluded the very version
   everything needed, which is why the lockfile pinned 0.7.0. Fixed in place
   (three pins → `^0.8`, `sync-engines.toml`'s three source roots → v0.8.0,
   engines re-vendored) with **no new version slot and no publication**.
   **So `specmap.json` now carries fact units and fact-grain edges, this
   phase's exit gate is fully reachable, and Phase C is not blocked.**
   Measured before/after, edge targets classified by the source bytes:
   1 041 → 5 267 spec units; fact-targeting edges **0 → 65**; unresolved
   **77 → 12**, because 65 of those "dangling" edges were correct code tags
   the unit-grain engine could not see. Wave 1's last drift row
   (`FACT-GRAIN-EVIDENCE`) closed on this. Cost: v0.8.0 adds `Fact::GoUnsafe`
   and two exhaustive matches in the rust stack had to learn it.
   *Still outstanding, and still the owner's:* whether the `-lang` slots
   should eventually be re-minted so a v0.7.0 slot stops carrying 0.8.0
   engines. Diagnosis kept for that day: `is_valid_fact_id` exists **only** in
   `core-ai-native/v0.8.0`; `vibe.lock` pins `core-ai-native@=0.7.0` and
   `rust-ai-native-lang@=0.7.0`; `cargo xtask sync-engines --check` is green
   (33 pairs, 6 sync sets), so nothing has drifted — the gap is a version, not
   a divergence. Three things must be settled when it is taken up, and all
   three are the owner's: publishing is a Rule 4 red line; the host resolves
   these packages from a **second, stale working copy**
   (`file:///C:/Users/olegc/gits/vibevm/…`, last commit `c112f6f`), so a
   re-mint in this copy is invisible to the host until the resolve is
   repointed; and the network registries 401 on this machine. *A local
   repoint + lockfile bump would very likely avoid publishing altogether —
   publication is only needed for external consumers.* The original step
   follows, unchanged, for whoever takes it up:

   **Close the fact-grain specmap gap first.** `core-ai-native` **v0.8.0**
   carries the fact-aware engine (`mdspec.rs` uses `is_valid_fact_id`);
   `rust-ai-native-lang` **v0.7.0** vendors the **v0.7.0** engine, which
   predates the amendment, and that is the version the host consumes. Re-mint
   `rust-ai-native-lang` (and its typescript / go siblings) at v0.8.0 with
   `cargo xtask sync-engines`, publish, bump the host lockfile, regenerate
   `specmap.json`. Only then can Phase C join fact anchors to code.
3. Create the campaign zone; pilot the loop on **three** packages of different
   genres — one prompt-only flow (`wal`), one code-bearing stack
   (`rust-ai-native-lang`), one aggregator (`rust-ai-native` — NOT `redbook`, which is a book of three chapters and a different genre entirely; corrected at the pilot 2026-07-26).

*Exit gate:* `check --exhaustive` correct on the pilot; `specmap.json` carries
fact units and non-zero fact-grain edges; floor green. *Prediction:* the
aggregator genre exposes a grammar gap wave 1 never hit — an aggregator's
facts are almost entirely *about other packages*, and pointing a marker at a
fact you do not own may need a new placement rule.

### Phase B — Markup (facts pass) {#phase-b}

*Entry:* A closed. *Executor:* Fable. Paragraph-exhaustive fact-grain markers,
sense-preserving re-splits, missing `{#anchor}`s, `audience` where obvious;
cross-doc findings recorded into the ledger in passing. **Semantic edits are
forbidden in this phase** — a semantic problem found becomes a finding.

Batching runs by package, largest first, because a package is the natural
review unit: `core-ai-native` (56) alone, then the three `-lang` stacks
(20–23 each), then `redbook` (14), then the long tail of 22 small `world`
flows in batches of 4–6.

*Exit gate:* `check --exhaustive` green over both corpora; batch diffs contain
markers, splits and anchors only. *Prediction:* ~12–15 batches; the
world flows mark much faster per file than the host's PROPs (they are short
and already unit-shaped), and `core-ai-native` alone costs as much as five of
them.

### Phase C — Verification (evidence pass) {#phase-c}

*Entry:* B closed for the cluster. *Executor:* Fable + machine evidence, per
the two rules of §3.1 and §3.2. Every marker gets `confirmed` / `drift` /
`unverifiable` in the cache with an evidence ref; a verdict without one is
rejected.

The ai-native cluster runs its checkers **first**, because their output is the
evidence for a large fraction of that namespace's facts. The world cluster
runs the artifact-and-consumer join.

*Exit gate (**A1 — enumerates this phase's own steps, not just its headline**):*
**(i)** 100 % of markers carry verdicts; **(ii)** the X/Y/Z summary recorded in
the LOG — the first measured actuality level of the packages; **(iii)** the
§3.2 checker runs exist **as files** under `campaigns/<id>/harvest/` — floor,
`conform`, `specmap` and the health collector over
`packages/org.vibevm.ai-native/**`, each captured as `command → real output`;
**(iv)** every `world` verdict records which of §3.1's three source classes it
rests on (**A2**), and those resting on source 1 alone are counted separately
in the summary as self-referential; **(v)** `baseline.json` written at the
phase close (**A6**). *Wave 1 gated this phase on (i) alone, left (iii)
undone, and its documentation phase had to be deferred for want of the
fixtures — see that campaign's `deferrals.md` §6.*
*Prediction, falsifiable and deliberately uncomfortable:* **`world` measures
higher than `ai-native`.** The flows are prose contracts written once and
rarely contradicted by anything; the ai-native packages make dozens of
mechanical claims about checkers, flags and engines, each of which can be
wrong in a way prose cannot. If that inverts, the reason is worth a finding
of its own.

### Phase D — Stitching {#phase-d}

*Entry:* C verdicts exist for the cluster. Same obligation types, same
loop-until-dry waves, same escalation rule (a pair that fails to converge over
two waves is a conceptual conflict → owner). `reality-mismatch` resolves
through sync-from-code with owner approval on every spec diff.

One wave-2-specific rule: **a finding that spans a package boundary is a
release event.** Fixing `core-ai-native`'s prose may require a version bump
and a re-vendor into three family members. Such a finding is not closed by an
edit; it is closed by a published version.

*Exit gate:* ledger empty or every survivor is an owner-ruled deferral.

### Phase E — Coding {#phase-e}

*Entry:* per IMPL task, unit stability. *Executor (amended 2026-08-03 by
owner directive):* the worker transport follows the owner-owned switch
[`campaigns/packages-2026-09/SUBAGENT-MODE.toml`](../../campaigns/packages-2026-09/SUBAGENT-MODE.toml)
— `claudez` (Claude Code workers on GLM-5.2 via the claudez/claudez2
launchers: two parallel lanes in worktrees for disjoint-perimeter tasks,
ONE thread for conflict-prone many-place edits, effort max built in) or
`native` (the harness's built-in subagents, as Phase D ran); mechanics and
the verified `-c` correction loop:
[`SUBAGENT-LAUNCHERS.md`](../../campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md).
The boss reviews every diff and makes every commit in either mode.
Wave-2 DRIFT tasks differ from wave 1's in one way that matters: a fix inside
a package's crates must be **vendored forward** to every family member that
copies it (`cargo xtask sync-engines`), and the task's acceptance says so
explicitly or the fix ships to one consumer and not the others.

### Phase T — Test coverage by swarm (owner amendment, 2026-07-26) {#phase-t}

*Entry:* E closed. *Executor:* per the T-spec's own header — GLM writers,
one packet per writer; **the transport is concrete since 2026-08-03**: the
owner-owned switch
[`campaigns/packages-2026-09/SUBAGENT-MODE.toml`](../../campaigns/packages-2026-09/SUBAGENT-MODE.toml)
routes packets to Claude-Code-on-GLM workers via claudez/claudez2 (two
verified lanes) or back to the harness's native subagents — mechanics:
[`SUBAGENT-LAUNCHERS.md`](../../campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md).
Fractality stays out either way (§2's «no fractality», owner 2026-07-26).
*Reviewer:* the boss, per packet.

**≥3 tests of distinct kinds per testable assertion.** Full specification:
[`campaigns/packages-2026-09/PHASE-T-SPEC.md`](../../campaigns/packages-2026-09/PHASE-T-SPEC.md).
The four things that decide whether it is worth anything, in one breath: a
**triage first** (6 963 in-scope facts, not 10 825 — `world` flows have no
callable surface and stay on §3.1's join); the **fact's text is the oracle and
the worker never sees the implementation body**; **three kinds** (canonical,
boundary, negative) rather than three assertions; and **one test exhibited red
per packet**, which is not mutation coverage but a check that the test works at
all.

*Placed before F on purpose:* F must answer the mandate per practice, and with T
in front of it, it answers with **measured coverage** instead of prose judgement.

*Prerequisite that only just landed:* until DRIFT-032 and DRIFT-034,
`#[spec(verifies = "spec://…#UPPER-FACT")]` did not compile, so this phase was
not expressible at all.

### Phase F — The credibility report {#phase-f}

The phase wave 1 does not have, and the reason this campaign exists.
One document answering the mandate directly: *does the AI-native discipline
hold itself to its own rule?* It carries the floor / conform / specmap run
output over the discipline's own crates, the measured actuality of both
namespaces, and an explicit verdict on every practice the host claims to
follow. The owner reads it and rules. **A green host floor is not an answer
to this question and may not be cited as one.**

### Phase G — Documentation (owner amendment, 2026-07-26) {#phase-g}

*Entry:* F closed, **and** the judgment-marking pass run — it supplies the
`audience` axis and the `actionstage="doc"` markers the guides' tables of
contents are generated from. Full specification:
[`campaigns/packages-2026-09/PHASE-G-SPEC.md`](../../campaigns/packages-2026-09/PHASE-G-SPEC.md).

`docs/` (43 unobserved files) moves wholesale to `docs-legacy/` under the
`legacy-spec/` rule; documentation is re-authored as a package,
`org.vibevm.doc/doc`, with `org.vibevm.doc/web` reserved as a manifest only.

**The law that shapes it: documentation cites a spec unit and never restates
it.** The reason is this campaign's own most-repeated finding — a restated fact
is a second statement of one truth with its own writer, and nothing forces the
two to agree. Links run one way, docs → spec; **the spec tree does not know the
documentation exists.** The cost of that (a spec unit can move under a page
citing it) is paid by PROP-014's revision pinning, so authorship is one-way and
*detection* is two-way.

*Not a deviation from `spec-genres`' two-way-link rule:* that rule governs lore
explaining a contract, and product documentation is a genre its map does not
carry at all. Phase G adds the row.

## 6. Predictions (falsifiable, campaign-wide) {#predictions}

**A5 is adopted, so each prediction names the step that tests it.** Wave 1's
prediction 1 went untested for a whole campaign because no step required the
command that would have tested it.

1. `world` measures higher than `ai-native`. **Tested by:** Phase C's exit
   gate (ii) — the X/Y/Z summary is recorded per namespace, not just in total.
2. The aggregator genre (`redbook`, `rust-ai-native`, `go-ai-native`,
   `typescript-ai-native`) needs at least one grammar amendment to PROP-043
   §3.8. **Tested by:** Phase A step 3's pilot, which includes `redbook`
   specifically so this fires early or not at all.
3. ≥ 1 practice the host claims to follow turns out to be specified by a
   package but enforced nowhere — the profanation the mandate suspects, found
   concretely. **Tested by:** Phase F, which must answer it per practice
   rather than in aggregate.
4. The fact-grain specmap re-mint (Phase A2) is the single longest-lead item
   and blocks nothing else once done. **Tested by:** Phase A's exit gate —
   record the wall-clock it took and whether any other step waited on it.
5. Cost is comparable to wave 1 in lines (28.7k vs 26.7k) despite 5× the file
   count — packages are many small files, not few large ones. **Tested by:**
   Phase B's batch LOG entries, which record files and lines per batch.
6. **New, and the one wave 1 would have failed:** this campaign's own
   stitching introduces **zero** new false claims, and its own verification
   confirms zero of them. Wave 1's answer was 1 and 1 — Phase D authored a
   `Shipped:` line for a `Baseline::store` that had never been built, and
   Phase C had already sealed five token-precedence anchors on evidence that
   compared one spec document with another carrying the same error.
   **Tested by:** Phase D's exit gate re-reading every `Shipped:` claim it
   authored against the code, not against the claim.

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
  claim *about* the artifact stays observed — README, boot snippet, `usage.md`;
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

## 7. LOG {#log}

- **2026-07-26 · RATIFIED; PHASE A OPEN.** Owner ratified in session
  («подтверждаю»), adopting all six §4.5 amendments in full. Ratification
  review re-measured the plan's own baseline and corrected two numbers: the
  ai-native join target is **703** `specmark::scope!` / `#[spec(…)]` sites
  across live version slots (781 with the superseded `core-ai-native` v0.7.0),
  not the 247 the plan claimed — 247 is the **rust family alone**, which
  measures 248 today, so Phase C had been budgeted at roughly a third of its
  actual size. And **seven** of the ten packages carry `crates/`, not eight;
  the three that do not are exactly the family umbrellas prediction 2 is
  about. Package / file / line counts (27 + 10, 154 + 140, 17 104 + 11 629)
  and marker state (zero) all verified accurate as authored.
  **The adoptions changed the plan body rather than sitting in a list:**
  Phase C's exit gate now enumerates five conditions instead of one (A1 +
  A2 + A6), and §6's predictions each name the step that tests them (A5),
  with a sixth added — *this campaign's stitching introduces zero new false
  claims* — because wave 1's answer to that question was 1 and 1, and nothing
  in wave 1 predicted it. Wave 1 closed out the same day: 4 488 confirmed /
  1 drift / 3 unverifiable of 4 492, `baseline.json` written and round-tripped,
  Phases F and G deferred into this campaign for want of inputs (A3).

- **2026-07-26 · Phase A step 1 CLOSED; step 2 DEFERRED by owner ruling.**
  Scope widened, zone `campaigns/packages-2026-09/` created and seeded.
  **Observed: 344 files (58 host + 286 packages), 13 916 facts, 8 997
  unmarked; `progress check` 0 across both corpora.** The step's purpose was
  to confirm a number and the number was wrong — 286 package files, not the
  294 §1 claimed; the eight are extractor test fixtures dropped by
  `DEFAULT_EXCLUDES`' always-on `fixtures` rule, correctly, since one of each
  clean/dirty pair is deliberately malformed. **Step 2 (the v0.8.0 re-mint)
  deferred** — «не перевыпускай пакет, сделаем это потом» — which means
  **Phase C cannot open** and Phase A's `specmap.json` exit clause is
  unreachable; the full statement, including the three things that must be
  settled and the local-repoint resolution that would avoid publishing
  altogether, is in the campaign's `deferrals.md#engine`. Two facts recorded
  in passing: `sync-engines --check` is green across 33 pairs (nothing has
  drifted — the gap is a version), and **with two campaign zones present a
  bare `vibe progress` command silently drops to ad-hoc mode and stops
  writing state**, so every command now needs an explicit `--campaign`.

- **2026-07-26 · Phase A step 3 — the pilot probes the aggregator genre first,
  and §6 prediction 2 CONFIRMS immediately.** The genre probe went to the
  family umbrellas rather than to `redbook` (which turns out to be a *book* —
  three Russian chapters — not an aggregator; §5-A's pilot list should say
  `rust-ai-native` where it says `redbook`). Two findings from a 19-line file:
  **F-069 — the grammar gap prediction 2 is about, found concretely.**
  `rust-ai-native` is content-minimal by design (PROP-028) and **three of its
  four substantive facts are about *other* packages** — what the `-lang` stack
  ships, what the `-mcp` package serves, that `core-ai-native` arrives
  transitively. Marking those `@impl/done` here asserts in this document
  something this document cannot be the source of truth for; when the owning
  package changes, nothing in the aggregator notices. Wave 1 never met this
  because a host PROP owns its own subject matter. The grammar needs *this
  fact is about `<other unit>`* — a delegating anchor whose verdict derives
  from the owner's rather than being asserted independently. Until it exists
  the honest fallback is `unverifiable` in the aggregator, verified where it
  lives.
  **F-068 — and the probe found real drift in the same file.**
  `rust-ai-native`'s README tells a consumer to require `^0.6`; the package's
  own manifest is `0.7.0`, and `^0.6` on a 0.x version means `>=0.6.0 <0.7.0`
  — **the documented instruction excludes the very version shipping it**.
  `typescript-ai-native` has it one minor down (README `^0.5`, manifest
  `0.6.0`); `go-ai-native` carries no such line. Both offenders sit *exactly*
  one minor behind themselves, which is the signature of a mechanical version
  bump that never touched the prose. Worse than wave 1's stale status lines in
  kind: a stale status line misleads a reader, this one misconfigures a
  consumer. The caret in an aggregator README is derived data and should be
  generated or gate-checked against the manifest.
  *Finding ids continue wave 1's sequence (F-068 onward) rather than restarting
  — findings cross wave boundaries (F-064…F-067 are wave-2 work) and one
  namespace is worth more than a tidy reset.*

- **2026-07-26 · pilot markup — the aggregator is marked, and the genre
  question turns out to belong to Phase C, not Phase B.** `rust-ai-native`'s
  README carries its document marker and six fact anchors (`AGG-ROLE`, three
  `AGG-MEMBER-*`, `AGG-HOW-TO-REQUIRE`, `AGG-FRONT-DOOR`); the closing
  paragraph was split in two, a sense-preserving re-split this phase allows,
  because "how to require it" and "where the front door is" are two facts.
  `progress check` clean over all 344 files. **F-069 does not block markup:**
  a marker records a fact's *stage and state*, and "can this document be the
  source of truth for a fact about another package?" is a question about its
  **verdict** — so the delegating-anchor gap is Phase C's to answer and Phase
  B proceeds at full speed. Worth knowing before someone stalls a whole phase
  on it.
  **F-070 — and `--exhaustive` immediately found something wave 1 could not
  have.** The counter reports **8 992 unmarked paragraphs**, of which **264
  sit in 33 `LICENSE.md` files** — the same UPL-1.0 text once per version
  slot. Marking them would mint 264 fact anchors on someone else's words, 33
  times over. `DEFAULT_EXCLUDES` already drops `refs` (third-party) and
  `fixtures` (not a contract) always-on; `LICENSE.md` is the same category and
  belongs there, which is a code change because §4 is include-only by design.
  Until it lands, **Phase B's exit gate cannot be reached honestly.**

- **2026-07-26 · the `vibe update` pre-flight — the assumption held, and it
  broke two things on the way.** `BATCH-PLAN.md` flagged `vibe update` as the
  one unverified assumption under sixteen batches and asked for it to be
  exercised once before Phase B leaned on it. It was. **The assumption itself
  is now verified: `vibe update --all` re-materialises 36 packages cleanly and
  the re-resolve is correct.** Three consequences and two defects, in that
  order.

  **It repointed the resolve, which closes `deferrals.md#engine` item 2 for
  free.** Every lockfile entry moved from
  `file:///C:/Users/olegc/gits/vibevm/…` — the *second, stale* working copy the
  deferral names, last commit `c112f6f` — to
  `file:///C:/Users/olegc/git/v/vibevm/…`, this one, and `source_kind` moved
  `registry` → `local` on all 36. The deferral had recorded the resolution in
  advance («repoint the resolve at this copy's `packages/` and bump the
  lockfile locally… closes the gap with no publication at all») and marked it
  as work someone would have to do. Nobody had to: `vibe update` is that
  repoint. **No publication, no re-mint, no owner red line touched.**

  **`core-ai-native` moved 0.7.0 → 0.8.0 at the consumer**, the last mile of
  the caret fix that closed wave 1's final drift row. The v0.7.0 `vibedeps/`
  slot was pruned. This retires an ambiguity rather than just a version: v0.7.0
  is now superseded *at the consumer*, not merely in the source tree, so
  `progress.toml`'s exclusion of `core-ai-native/v0.7.0/**` describes reality
  instead of anticipating it. **B1's subject is unchanged** — the live slot was
  already v0.8.0.

  **F-079 — the shipped JSON Schema is one variant behind the spec and the
  code, and only an environment change could expose it.**
  `crates/vibe-cli/resources/package-tree.schema.v1.json` enumerates
  `source.kind` as `registry · git · override · path · embedded`.
  `SourceKind::Local` has existed in `vibe-core` and is **normative**:
  [`##LOCAL-SOURCE-KIND`](../modules/vibe-registry/PROP-030-embedded-registry.md)
  and `##LOCK-LOCAL` both say a package resolved from project-local records
  `source_kind = "local"`. The schema never learned it. The golden
  `tree_json_validates_against_schema_and_carries_known_facts` went red on all
  36 packages the moment a resolve actually produced one — **the floor caught
  it, which is the gate working.** It had been latent for as long as this host
  resolved through the stale copy's embedded registry, because that path emits
  `embedded`, which the schema does know. Fixed in place: the enum gains
  `local` and a description distinguishing it from `embedded`. *The lesson is
  the campaign's own thesis in miniature: a contract document drifted from the
  code it describes, and no gate could see it until the environment changed.
  Nothing was wrong with the checker — the checker was never given the input
  that falsifies.*

  **F-078 — the host now reads four rules twice.** *(Restated 2026-07-26 after
  DRIFT-029 returned on its stop rule. The first version of this entry, below,
  named the wrong cause: it read the generation of a slot's boot artifacts as
  the defect. It is not — PROP-038 `##UNIT-PER-PACKAGE` decides that «every
  package materialised under `vibedeps/` carries its **own** boot artifacts …
  not only entry-point workspace nodes», and `##UNIT-SELF-CONTAINED` makes a
  unit's `STATIC.md` contain everything statically linked into it, **once
  each** — so the `git-practices` unit holding its four members is correct.
  There was no path-based fallback either: `bootgen.rs:305` reads a
  statically-linking dependency through its compiled `STATIC.md` **by explicit
  design**, and the absent `[boot_snippet]` is why the other branch is not
  taken. The prescribed one-line fix would have left that reference dangling
  and turned a duplicated boot lane into a **failed install**, which a
  characterization golden already pins.)*

  **The real mechanism is the hoist counter, and it is a defect against
  §2.3.** `static-soft` is the default precisely so that «a package statically
  linked by more than one consumer is **hoisted** … and linked **once**»
  (`##MODE-STATIC-SOFT`), and `##SOFT-DEFAULT-WHY` names the reason in the exact
  words of what we observe: «the model sees the same prompt several times and
  can be confused about which copy is authoritative». `hoist::soft_static_pulls`
  walks only materialised packages, so a member pulled statically by **both**
  the root and an aggregator counts **one** puller, misses the two-puller
  threshold, stays unhoisted, and is compiled a second time by the root's
  `static_transitive_closure`. Counting the root restores the `#use` references
  §2.5 designed. **Queued as DRIFT-030, not run** — it rewrites boot
  composition for every consumer and carries one unresolved interaction
  (whether `append_hoisted` double-adds at the root, after
  `compute_effective_boot` has already deduped). *Two premises of mine died
  here in one task: that the write was wrong, and that a fallback caused it.
  The stop rule is the only reason neither reached the tree.*

  *The original entry, kept because a corrected record should show what it
  corrected:* `git-practices` ships three
  files (`LICENSE`, `README.md`, `vibe.toml`) and its own manifest says
  «content-minimal (PROP-028): no boot snippet of its own; each member ships
  theirs». Materialisation nevertheless **wrote**
  `vibedeps/flow-git-practices/0.1.0/spec/boot/{INDEX,STATIC}.md` — 192
  generated lines compiling the umbrella's four members — and the host then
  inlined that generated file into `spec/boot/STATIC.md` as a contribution
  *from `git-practices`*, on top of the four members it had already compiled
  directly from the BFS closure. `STATIC.md` grew **+194 / −0** and each of
  atomic-commits, attribution-policy, autonomy and conventional-commits now
  appears **twice**. Source clean and `.vibe/cache` copy clean were both
  checked, so cache poisoning is ruled out by elimination: vibe generated it
  during the run. Filed as **DRIFT-029**; the boundary that makes it tractable
  is that the one other slot with a generated `INDEX.md`
  (`delegation-rules`) ships it **from source**, so the fix must separate
  *generated here* from *shipped by the package* rather than keying on the
  file's presence. **Does not block Phase B**: the duplicated text is
  identical, so it costs boot tokens and states no contradiction, and
  `spec/boot/**` is outside the observed corpus by design.

- **2026-07-26 · Phase B opens, and the corpus loses a fourth chunk before the
  first marker lands. F-080 RULED.** `core-ai-native` was batched B1–B3 by
  genre — guiding + operating layer (9 files), mechanisms + appendix (7), and
  `spec/legacy-projections/` (11). The third turned out not to be work.

  Eleven v0.1 language guides — C++ ×3, Java ×4, Kotlin, Python, Go,
  TypeScript — 1 264 lines of substantive normative prose (version floors, MUST
  gates, licence flags) that **nothing in the living corpus cites**. The go
  stack's guide says GUIDE-GO-v0.1 «stays, **untouched**» in that directory;
  the typescript stack declares GUIDE-TYPESCRIPT-v0.1 superseded; the other
  nine have no successor stack at all, which is what made it a genuine question
  rather than an obvious exclusion.

  **Owner ruling:** «legacy-projections — это замороженная история. Мы
  когда-нибудь покроем эти языки, но еще не сейчас. Сейчас у нас есть активные
  rust, typescript и go.» So it is §3.3's category — *marked, never verified* —
  and, exactly as with the superseded version slots, `--exhaustive` cannot
  express that, so the directory **leaves the corpus** instead. The exclusion is
  genre-shaped (`packages/**/spec/legacy-projections/**`) rather than pinned to
  `v0.8.0`, so the next version slot cannot silently re-admit 1 264 lines of
  frozen text; the languages return as **includes** when a stack for them lands.

  **The pattern holds for the fourth time.** Machine copies, licence
  boilerplate, derived indexes, superseded slots — and now frozen projections.
  *Every one was found by asking what the corpus is made of, and not one by
  estimating how big it is.* Phase B is fifteen batches, not sixteen; the
  package corpus is 206 files, not 217. **The fact count is deliberately left
  as a recount** rather than carried forward minus an estimate — the campaign's
  own rule about numbers quoted before their decomposition applies to its own
  numbers first.

- **2026-07-26 · F-081 — the floor was gating a dead slot, and it is the
  mandate's own question answered against us.** Found while checking whether
  Phase B markup could break the packages' specmap ratchet. It cannot — that
  gate is orphan-coverage over **code** — but two lines away sat something
  worse.

  `tools/self-check.sh` steps 7 and 9 gated
  `core-ai-native/**v0.7.0**`. `sync-engines.toml` declares the authored home of
  the neutral engines as `core-ai-native/**v0.8.0**/crates`, and moved there in
  `0aa4ba01` — wave 1's caret fix. `self-check.sh` was not moved with it; its
  last touch is `2570629d`, earlier. **So for that whole interval the floor ran
  `fmt` / `test` / `clippy` and the self-trace over a frozen slot** — one
  `progress.toml` excludes as superseded, `vibe update` has since pruned from
  `vibedeps/`, and nothing resolves to — **while the engines everything actually
  vendors went ungated.**

  The two trees are not close: ten files differ, and two source trees exist
  **only** in v0.8.0 — `conform/src/rules/go.rs` and `specmap/src/mdspec/`,
  the latter carrying `tests.rs`. **The fact-grain specmap engine's own tests
  had never been executed by the floor** — the engine Phase C's evidence join
  depends on, and the one whose arrival closed wave 1's last drift row.

  Step 7's comment states its purpose verbatim: «Gate the authored source
  here.» It stopped doing that and **nothing noticed, because the gate stayed
  green** — faithfully testing the wrong tree. That is §1's *профанация* in its
  purest observed form, and it was found in the campaign's own gate rather than
  in the corpus it was pointed at.

  **Fixed, measured before changed.** All four gates were run against v0.8.0
  *first* — fmt 0, test 0 (all suites green), clippy 0, specmap `--gate` 0
  orphans — so the repoint adds coverage without buying a red. The slot is now
  one `CORE_SLOT` constant, and a new floor step asserts it appears as a
  `source_root` in `sync-engines.toml`, failing loudly with the candidate lines
  if it does not. **Both branches of that guard were made to fire before it was
  trusted** (DRIFT-020's rule): it passes on v0.8.0 and fires on v0.7.0 — the
  exact state it would have caught.

  *This is the fourth time in two waves that the expensive thing was a derived
  value nothing kept honest — a caret, a hand-written timestamp, three stale
  projections, and now a gate's target. The pattern is stable enough to plan
  against: **whenever one file's constant must track another file's constant,
  the tracking is a checker or it is a WISH.***

- **2026-07-26 · F-078, third state: the duplication is structural, and the
  stop rule caught a wrong fix for the second time.** DRIFT-030 returned on its
  §4 step 1 gate — the question the task made it answer *before* touching the
  counter. It answered by **measuring**: a fixture of vibevm's exact shape
  (root `static-transitive` → content-minimal aggregator `static` → member),
  driven through `apply_resolution`. Baseline reproduced the live defect (root
  copies 2, aggregator 1). With the counter fix applied: root copies **still 2**,
  aggregator 0. **The duplicate does not disappear; it migrates into the root's
  own lane.**

  The reason is in the spec, not the code. `##HOIST-LCA` puts the hoist target
  at the LCA of a *continuous static zone*; vibevm's root → redbook →
  git-practices → member chain is unbroken static, so that **LCA is the root** —
  and the root is simultaneously the hoist destination and its own compile site.
  Two write paths reach the root's lane and neither knows about the other:
  `compute_effective_boot` dedups only inside its own BFS closure and never sees
  `append_hoisted`, which pushes the same `{slot}/{source}` path unconditionally.

  So the counter is **necessary and not sufficient** — and the remaining half is
  a design question about which mechanism owns dedup, which is spec-shaped and
  therefore the reviewer's, not the executor's. Three candidates are on the
  table; the third reads `##HOIST-LCA` most literally and satisfies §6 verbatim:
  **the root is a hoist destination and never a puller, so a zone emits `#use`
  for anything the hoist point already carries and `append_hoisted` does not
  fire at the root at all.**

  **Not decided here, and deliberately.** Phase B is the mandate; this
  duplication costs boot tokens and states no contradiction, since both copies
  are byte-identical. It is recorded, measured, and parked with a recommendation
  rather than half-fixed. *Two executor runs, two returns, and both returns were
  worth more than the fix would have been — the first killed a change that would
  have turned a duplicated boot lane into a failed install, the second killed one
  that would have moved the duplicate rather than removing it. Neither was
  visible from reading the code; the second needed an experiment.*

- **2026-07-26 · B1b closes core-ai-native's guiding+operating layer, and the
  gate finds two grammar gaps the eye could not.** Six files, 506 → 655 lines,
  **276 units marked** (including the batch's first tables — 70 body cells), 224
  anchors, 32 paragraphs deconstructed, 21 heading anchors added. `progress
  check` clean over 264 files. B1 total with B1a: **417 units over nine files**.

  Two of the executor's open questions were settled by *running the checker*
  rather than by argument, and both turned into findings.

  **F-083 — a GFM task-list item cannot carry a fact anchor.**
  `##FACT-ANCHOR-SYNTAX` requires the anchor to be the unit's **first token**;
  the parser reads `- [ ]`'s checkbox as that first token, so an anchor placed
  after it is invisible and the unit reports `MissingAnchor`. There is **no
  legal placement today** — the checkbox is structure, exactly like the list
  marker and the ordinal, and the grammar does not say so. Four items in
  `02-EXECUTABLE-SCAFFOLDS.md` were marked, went red, and were reverted; they
  stay unmarked until the parser learns it. The executor predicted this exact
  outcome and named the one command that would settle it, which is the right
  shape for an uncertainty.

  **F-084 — the trailing marker dies silently next to a quoted fence.** In
  `01-PATTERN-CARD-FORMAT.md:41` the last-token shorthand was not recognised on
  a paragraph whose text carries an inline code span containing a **triple
  backtick** (`` ` ```card-ops ` ``): fence-awareness (`##FENCE-AWARE`) misreads
  it and the marker is swallowed. Moving the marker to position 1 clears it —
  proven by re-running the gate, not by inspection. **The failure mode is the
  dangerous part: it reports as `unmarked`, not as an error about the code
  span**, so a session that "fixes" it by re-adding the marker in the same place
  loops forever. Any document quoting a fence in prose is exposed; that is a
  large class in this corpus, which is *about* fenced formats.

  *Both gaps are in the tooling this campaign uses to measure, not in the corpus
  it measures — the same place F-081 was. Three of the wave's findings so far
  are the instrument being wrong rather than the subject, which is worth
  noticing before the remaining thirteen batches trust its output.*

- **2026-07-26 · DRIFT-031 lands both grammar gaps — and F-084 was three times
  bigger than diagnosed.** Two commits, floor green on the committed tree,
  `progress check` clean over 264 files on a **cold** parse.

  **F-083's cause:** `parse/facts.rs::list_item_content` returned the offset
  just past `- ` / `N. ` and stopped, so `- [ ] ##ID` had `[` as its first
  token. Now composed with a `task_box_len` that accepts `[ ]`/`[x]`/`[X]` only
  when followed by whitespace — `[ ]glued` stays prose, which is GFM's own rule.

  **F-084's cause was not the triple backtick.** `parse/blocks.rs::blank_inline_code`
  toggled `in_code` on **every individual backtick**, so *any* run of two or more
  desynchronises the flag and everything after it is blanked — and any block with
  an **odd total** of backticks blanks to its end. A probe over all **723**
  `.md` files found **23 text blocks in 21 files** where naive and run-aware
  blanking disagree, including a stray tick in `docs/glossary.md:217` swallowing
  the rest of its block. *I characterised this as the triple-backtick case from
  one sample; it is the general inline-code-span defect. The task's §4 had
  already specified the general behaviour while its §8 hedged the scope — my own
  task contradicted itself, and the executor resolved it toward the behavioural
  half and said so.* Measured collateral: zero — no diagnostic anywhere else
  moved.

  The `01-PATTERN-CARD-FORMAT.md` marker went back to the **last** position on
  purpose: that is the position F-084 broke, so the file is now a live witness
  that goes red again if run-matching regresses.

- **2026-07-26 · F-085 — the register that marks a normative fact is the one
  code cannot cite. Found in DRIFT-031's forced deviation, and it aims straight
  at this campaign's deliverable.**

  `##DECISION-TWO-REGISTERS` (owner, 2026-07-24) makes `##UPPER-SLUG` the
  register for a **normative** fact and `##kebab-case` for a service unit.
  `is_valid_fact_id` accepts both — its own doc says so — but the `spec://`
  **URI** parser validates through `is_valid_anchor`, which is kebab-only, and
  rejects the rest with *"anchor must be kebab-case"*. So
  `#[spec(implements = "spec://…#SOME-NORMATIVE-FACT")]` **does not compile**.
  DRIFT-031 hit it, cited the containing *section* anchors instead, and named
  the facts in doc comments — and reported the deviation rather than hiding it.

  **Measured, not inferred:** of every `spec://…#anchor` cited from `crates/`,
  **275 are kebab and none is UPPER** (the handful of non-kebab hits are
  placeholder prose in doc comments).

  Why it matters here specifically: Phase B is minting UPPER anchors on every
  normative fact — that *is* the register decision — and Phase C's evidence join
  exists to connect those facts to code. B1 alone minted roughly 330. As it
  stands the join can only ever reach service units, which is precisely
  backwards. **Options:** (a) let the URI parser accept `is_valid_fact_id` in
  the anchor position, leaving the kebab-only law for *heading* anchors
  untouched — the superset function already exists and is documented as such;
  (b) flip the register convention, which re-anchors both waves and was already
  considered and rejected at `##registers-rejected`; (c) accept, and let code
  cite sections while naming facts in prose — which forfeits the fact-grain join
  this campaign is built to produce. **Recommend (a).** The constraint is that
  the grammar crate lives in `core-ai-native`, so per §5-D this is a **release
  event**: the fix must be vendored forward with `cargo xtask sync-engines` to
  all three stacks, not merely edited.

- **2026-07-26 · Two operating facts worth not re-learning.** The parse cache
  keys on **content hash only**, so a *parser* change is invisible to a warm
  cache and re-verification after one needs `--no-cache` — DRIFT-031's first run
  served the old parse for every unchanged file. And the deliberate copies of
  `list_item_content` across the separability seam (`crates/vibe-spec/src/facts.rs`
  and the package twin in `core-ai-native-specmap::mdspec`) **now disagree**:
  only progress-core's learned the checkbox. That divergence is out of DRIFT-031's
  bounds by design and wants its own decision.

- **2026-07-26 · Four owner rulings, and the open-findings queue empties.**

  **F-082 — RULED: boot snippets ARE marked**, «и внутри пакетов, и внутри самой
  vibevm». So the +52 % the measurement showed is accepted as the price of
  making a consumer's boot lane addressable by `spec://`, which is what the
  correction contract exists for. Nothing changes operationally — the host's own
  `00-core.md` / `90-user.md` were marked in wave 1, and the 24 package snippets
  are marked as their batches arrive. Recorded so the cost is never re-litigated
  as if it had been an oversight.

  **F-085 — RULED (a): the URI parser must accept fact ids; heading anchors are
  not touched.** Filed as DRIFT-032 and dispatched. It is a **release event** —
  the grammar crate is vendored into five packages, so `cargo xtask
  sync-engines` propagates or the floor goes red. Worth recording that the fix
  is *not* a new decision: `##FACT-ID-GRAMMAR` already says a fact is
  addressable as `spec://…/<doc>#<ID>`, and the parser simply never implemented
  that sentence. One test flips by design (`tests.rs:33` lists
  `spec://vibevm/x#A-b` as rejected, commented «uppercase anchor»), and the
  assertions that `is_valid_anchor("FACT-A")` stays false must survive — a
  useful tripwire that the change is scoped to the URI and not to the heading law.

  **F-075 — RULED (d):** `seal` writes `processed_hash`. **F-077 — RULED (a):**
  the derived counts are deleted and computed on read. Both filed as DRIFT-033.
  F-075's choice also discharges **amendment A4**, which requires the hand-seal
  staleness gap closed before Phase C and names F-067 as the reason — the same
  field, seen from the other end, so one change answers both. Recorded for
  honesty: (d) makes staleness *checkable* against the text; it does **not**
  detect a session that re-derived one anchor of three hundred and sealed the
  file. That half stays open by choice, not by oversight.

  **F-077's target had already moved before the ruling landed.** The
  `campaign.summary` the finding named is gone; the stored projection is now
  `counters` in `campaign.json`. Option (a) is written against the class rather
  than the instance for exactly that reason — this one has demonstrated it
  relocates.

- **2026-07-26 · DRIFT-032 lands F-085, and its executable change is two lines.**
  `parse_spec_uri` validates the anchor with `is_valid_fact_id` instead of
  `is_valid_anchor`; the message follows. `is_valid_anchor`'s body is
  **byte-identical** — the owner's «заголовочные якори пока не трогаем» held.
  Host twin (`vibe-spec/src/address.rs`) widened to match, four vendored copies
  propagated by `sync-engines` (33 pairs, 6 sets, clean), floor green on 25 steps.

  **Proved rather than asserted, and with a negative control:** a temporary
  `#[spec(implements = "spec://…/00-MANIFESTO#SINGLE-DESIGN-TARGET")]` — a real
  anchor B1 minted — compiled; the old validator was then restored to confirm
  the same tag *fails*, so the passing case was load-bearing rather than inert.
  Both temporaries removed.

  **§4 step 4 came back NOT FOUND**, thoroughly: both anchor collections are
  byte-exact maps, `vibe-spec`, `progress-core` and the whole authored engine
  tree contain **zero** case transforms, no heading-text slugification exists,
  and no anchor becomes a path component (so the case-insensitive Windows
  filesystem never sees one). That clears the ground for DRIFT-034's
  case-insensitive duplicate check.

  **My task file was wrong on an edge case and the executor measured before
  proceeding.** §4 claimed `9lives` "still fails"; it did not — kebab admits a
  digit head, so the swap is **not a pure widening**: digit-headed anchors go
  accepted → rejected, trailing dashes rejected → accepted. Measured across
  **380** distinct cited anchor segments and **750** distinct `{#…}` heading
  anchors: every one already matches `[A-Za-z][A-Za-z0-9_-]*`. Zero regression,
  and the asymmetry is pinned in a test. **DRIFT-034 inherits that measurement**
  — widening the heading law has the same digit-head consequence and the same
  zero blast radius.

- **2026-07-26 · F-086 — the sync gate is green because it is not looking, and
  that is the third one this session.** `cargo xtask sync-engines --check`
  reports «every vendored crate matches its authored source (33 pairs across 6
  sync sets)». **Six** packages vendor `core-ai-native-specmark-grammar`;
  `sync-engines.toml` names **five** roots and no target for
  `go-ai-native-lang` or `go-ai-native-mcp`. They are not out of sync — they are
  **outside the check**.

  Measured, not inferred: `go-ai-native-lang/v0.1.0`'s vendored grammar is
  **byte-identical to the authored v0.7.0** and **differs from v0.8.0**, while
  its own manifest declares `flow:…/core-ai-native = "^0.8"`. The rust twin is
  correctly identical to v0.8.0. So a package declares 0.8 and ships the 0.7
  engine — it predates the fact-grain amendment and now also the URI widening —
  and the gate built to catch exactly that says everything matches.

  This is plan §5-E's own rule failing on its own terms: «a fix inside a
  package's crates must be **vendored forward** to every family member that
  copies it, or the fix ships to one consumer and not the others». Pre-existing
  at DRIFT-032's base commit; adding sync targets is a design call, not a fix,
  because the go family's engine version is a release question.

  *Third instance of one shape: F-081 (the floor gating a frozen slot), F-083
  and F-084 (the parser not seeing units the grammar allows), and now F-086.
  **A green panel is evidence about what was checked, never about what was
  covered** — and nothing in the panel reports its own coverage.*

- **2026-07-26 · F-087 — the repository's own attribution rule is contradicted
  by a convention in its history. Raised for the owner; not actionable by me.**
  The policy says, unqualified: «Never mention model, agent, or AI-tool names in
  commit messages, branch names, or code comments.» **17 of this repository's
  1 852 commit bodies name a model** — the wave-1 convention of writing which
  executor tier a task went to. Campaign task files carry the same in their
  `**Executor:**` line, though those plausibly fall under the policy's explicit
  carve-out for «technical AI-workflow documents … agent instructions».

  **The history half cannot be cleaned.** It would need a rewrite of published
  commits — a Rule 4 red line — and the source-mirrors law forbids `--force`
  «for any ref, for any reason». So the practical choice is between accepting
  the existing 17 and stopping there, or amending the policy to carve out
  executor-tier labels. **No new commit in this session names a model**; the
  convention stops here regardless of the ruling. *Surfaced because an executor
  flagged its own task file for it after self-repairing an attribution trailer
  it had added — the rule working from the inside.*

- **2026-07-26 · DRIFT-034 stops, and the corpus refutes the reviewer's own
  argument.** Making duplicate detection case-insensitive flags **29 published
  anchor pairs across 12 documents**, every one the same deliberate shape: a
  section heading `{#kebab-slug}` and, two lines below, that section's lead
  normative fact `##KEBAB-SLUG`. Independently recounted: 29 across 12. It is
  the house convention the two registers exist for — the heading anchor
  addresses the *section span*, the fact anchor addresses the *lead statement*,
  and the register is what tells a reader which grain they are citing.

  **The reasoning that justified the fold was wrong, and this is the
  correction.** §2 of that task claimed: «today an `UPPER-SLUG` fact cannot
  collide with a heading anchor, because a heading anchor cannot be UPPER —
  that is a structural guarantee, and widening removes it.» It is not a
  guarantee and widening removes nothing. `##TWO-TREES` and `{#two-trees}` are
  **byte-different**, so they were never a duplicate under byte-exact detection,
  before or after. What widening newly permits is writing `{#TWO-TREES}` — which
  *would* be a byte-exact duplicate of `##TWO-TREES`, and the existing check
  catches it **unchanged**.

  So the two halves were never one idea: **the widening is safe on its own**
  (measured: 727 files, 1 227 distinct heading anchors, zero that would fail the
  wider grammar, zero digit-headed), and the case-insensitive check is a
  separate proposal that this corpus refutes. Its purpose was to replace a
  guarantee that did not exist, and its cost is flagging the authoring
  convention 29 times. **Recommendation to the owner: widen, drop the fold.**
  Byte-exact detection keeps catching every real duplicate; a fold would be
  machinery for a defect that is not there.

  Recorded because it will be asked again: duplicate detection lives in **five**
  places, not one — `mdspec.rs:341` (PROP-014 warning), `progress-core/parse/anchors.rs:12`
  (what `progress check` fails on), `vibe-spec/gate.rs:55` (a build error via
  `pipeline.rs:82`), `vibe-spec/doctree.rs:71` (recorded, no consumer), and the
  validator itself. And `doctree.rs:70`'s map **is** the resolution index, so any
  future fold must be a second parallel key set there, never the lookup key.

  Corroboration for F-078 from an unrelated angle: the same sweep found **69
  byte-exact duplicate anchors, all inside `spec/boot/STATIC.md`** — the
  generated lane carrying the duplicated git-practices snippets. Outside the
  Progress-Control corpus, so nothing fails on it; it is the same defect seen by
  a different instrument.

- **2026-07-26 · A pattern in the reviewer's own work, stated rather than
  buried.** Five task files this session carried a factual error an executor
  found by measuring: DRIFT-029 (two premises wrong), DRIFT-030 (premise
  incomplete), DRIFT-031 (§4 and §8 contradicted each other on scope), DRIFT-032
  (an edge case that was already true), DRIFT-034 (the §2 reasoning above, plus
  two slips in §6). **Every one was caught before it reached the tree**, by the
  stop rule and by the "reproduce before you fix" step the tasks carry. The
  mechanism is working exactly as designed and the frequency is the lesson: a
  task file is written from reading, and reading this codebase has been wrong
  five times out of five where a measurement was available. *Future tasks should
  keep leading with a measurement step, and the reviewer should stop writing
  confident current-state bullets that a single command would have checked.*

- **2026-07-26 · DRIFT-033: one finding was already closed, the other splits in
  two — and the stale claim was mine, from a ledger I had read.**

  **F-075 needed no code at all.** `seal` has written `processed_hash` since
  `e9fc7b44` (DRIFT-026, *sealing a verdict stops depending on memory*), landed
  earlier **in this same campaign**. The task's §3 repeated F-067's original
  wording — «written only by a real verify batch» — which was true when F-067
  was filed and stopped being true when that commit shipped. Measured rather
  than argued: a seal on a scratch copy wrote the file's own sha256, identical
  to `sha256sum`, from the crate's single `content_hash`. **Amendment A4 is
  therefore discharged**, by DRIFT-026 and not by DRIFT-033, which contributed
  the test nobody had written. `deferrals.md` corrected in place.

  *That commit subject was in the log this session read at boot and quoted in
  its own resume report.* Six task files have now carried a wrong current-state
  claim; this one was recoverable from a line I had already read. The rule that
  follows is narrower than "measure more": **a ledger entry is a claim with a
  date, and quoting it is not the same as checking it.**

  **F-077 lands half and stops half, on two spec anchors.** The per-file
  `summary` is gone and computed on read — nothing in the crate, the CLI, the
  dashboard, `spec/**` or the campaign's own documents read it. But `counters`
  in `campaign.json` is named by `##STATE-FILES` `@impl/done`, and
  `##DASHBOARD-READS-ONLY` says «The dashboard reads **only** these; **it
  computes nothing**» — and `tools/progress-dashboard/index.html:106` renders
  four tiles straight from `camp.counters`. Removing it is a spec edit, so the
  executor stopped. Correctly.

  **And the two rules genuinely conflict, which my blanket recommendation of
  option (a) did not anticipate.** `##DASHBOARD-READS-ONLY` exists for the same
  reason F-077 does — to stop a second implementation of the campaign's
  semantics, in this case in browser JS. Deleting `counters` would satisfy F-077
  by violating that. **The resolution is that F-077's hazard was never "a
  derived value is serialised" — it is "a derived value has an independent
  writer that can drift".** `campaign.summary` went stale because it was
  hand-maintained. A projection recomputed from the single source on every write
  cannot. **Recommendation: keep `counters`, guarantee it is written from the
  same one computation the rest of the system uses, and pin that with a test
  asserting it equals a recount.** One computation, one writer, serialised for a
  consumer contractually forbidden to compute. `##CACHE-TALLY-COMPUTED` records
  the half that did land.

- **2026-07-26 · DRIFT-036 closes F-086 — and disproves the consolation the
  reviewer offered with it.**

  **Both denominators exist and both were made to fire.** Sync went `33 pair(s)
  across 6 sets` → **`51 across 9`, «all 6 vendored engine dir(s) … are sync
  targets»**. The floor went 25 steps over 4 workspaces → **36 over 7**, opening
  with «the floor builds all 7 live package workspace(s)». Three firings
  recorded: a live workspace the floor does not build; a gated slot that is not
  its package's live one (the F-081 shape, caught from the other side); a
  vendored engine dir that is the target of no sync set. The frozen slot is
  excluded **by derivation** — `live_slots` takes the newest slot per package by
  `sort -V`, so a version literal cannot rot: when v0.9.0 lands the guard goes
  **red** until the gate is repointed.

  **The correction, and it lands on something already said out loud.** §3 of the
  task claimed the go packages carry a v0.7.0 engine wholesale, that `rules/go.rs`
  exists only in v0.8.0, and therefore that
  `go-ai-native-lang/README.md:8-9`'s «^0.8 — the first edition carrying the Go
  fact/config/rule support» is **false as shipped**. Measured: they are a
  **mix** — `core-ai-native-conform` was already v0.8.0, `rules/go.rs` present
  before this task touched anything (verified at `a314d38a^`); only `specmap`
  and `specmark-grammar` lagged. **The README is accurate. The claim was mine
  and it was wrong.**

  That matters beyond the task, because it was used to answer the owner's
  question *«why did the process not catch this, and could Phase C have?»* The
  answer given was: the claim is false, it sits in an observed document, so
  Phase C at batch B5 would have caught it. **All three legs fail.** No observed
  document makes a claim that F-086 falsifies, so **no phase of this campaign
  would have caught it** — the consolation is withdrawn and the underlying point
  is left standing alone and stronger: *the corpus is observed; the instruments
  are not.*

  The real drift is sharper than «a version behind», and worth keeping in these
  words: the go packages **froze when they were vendored and missed exactly what
  the gate carried while they were outside it** — including, on the same day,
  the two engine commits from DRIFT-032 and DRIFT-034.

  **§8's stop did not fire and the feared break was imaginary.** The `Fact::GoUnsafe`
  problem cannot occur here: the sync changed 8 files and needed **zero** source
  edits. What the three unlisted workspaces did surface was untouched by any of
  it — `fmt` had **never run** on them (root `cargo fmt --all` does not enter an
  excluded workspace), three real clippy lints sat in authored go code, and
  `typescript-ai-native-lang`'s **191 tests had never run here while its code
  ships into another package**.

  *A method note worth more than the numbers:* enumerating that stack's
  environment-blocked tests by watching failures gave 3, then 5. `--no-fail-fast`
  gave the truth — **6 across 4 binaries**, plus a second toolchain directory
  nobody had looked in. **Iterative failure-watching under-counts by
  construction**, because each run stops at the first failure and reports a
  prefix as if it were the set.

  **Accepted with its reason recorded — probe-guarded test steps.** A missing
  `gopls` or `tsc` makes the step filter the tests that need it and print a NOTE
  naming what was dropped and the recipe to restore it, **every run**; a
  provisioned box filters nothing. That is consistent with this task rather than
  an exception to it: **the filter has a denominator and states it out loud.**
  The pre-existing `rust-analyzer` trade is the same bargain made *silently*, and
  it is that one which should rise to this standard, not this one that should
  fall to it. **Revisit when:** a NOTE is observed being ignored across sessions,
  which would mean it has become wallpaper.

  **Cost, reported and not decided (per §8):** floor **154 s warm over 36 steps**,
  of which the 11 new steps are **30.7 s (+25 %)**; cold is materially worse
  since three more workspaces each own a `target/`. The tiering call is the
  owner's, and Phase T's §7 will want this number anyway.

- **2026-07-26 · B2 closes `core-ai-native`, and the first thing it found was
  the reviewer contradicting himself eight hours earlier.** Seven files, 950 →
  1 100 lines, **526 units**, 472 fact anchors, 49 paragraphs deconstructed, 27
  heading anchors. `progress check` clean over 264; zero unmarked in the seven
  under `--exhaustive`. With B1 that is **943 units over sixteen files** and the
  whole live slot of `core-ai-native` is marked.

  Self-audited before hand-over: a word-stream diff proves every file
  byte-equivalent once anchors, markers, bullets and whitespace are stripped,
  and a re-implemented counter reports 0 unmarked, 0 anchor collisions, 0
  marked-without-anchor, 0 anchored-without-marker. **Zero `@unknown`, and
  stated as deliberate** — the two places it would have hedged it measured
  instead, and the one unit it considered `@unknown` for it correctly reported
  as *drift* («`unknown` means looked-at-and-not-understood; reporting drift is
  the other channel»).

  **The reviewer's error, and it is the sharp kind.** PROP-014 now contradicted
  itself **inside one bullet list**: line 80, amended this morning, says the
  heading grammar is «one law, shared with fact ids»; line 95, three bullets
  below, still said «Heading anchors keep the kebab-only law». I amended the
  first and did not read the second **in the same file**. The shipped parser
  sides with line 80 and its doc comment cites that very section as authority.
  Corrected in place, and the sentence now records *when* the law changed
  instead of asserting the old one. *Amending a clause without re-reading its
  document is the same failure as quoting a ledger line without re-measuring
  it — and this is the second one today.*

- **2026-07-26 · F-088 — a second derived file the F-071 audit missed, and this
  campaign has now hand-edited it. OWNER RULING NEEDED.** `appendix/ATLAS.md`
  declares on **line 2**: «GENERATED from `findings.jsonl` (A2: derived, do not
  hand-edit)». **`findings.jsonl` is tracked nowhere in the repository.** Three
  consequences, in rising order of discomfort: the file's stated source of truth
  is absent; `01-PATTERN-CARD-FORMAT.md` points card authors at `findings.jsonl`
  IDs that cannot be resolved; and **B2 has just minted 93 hand-authored anchors
  into a file that forbids hand-editing** — if the generator ever returns, the
  markup dies.

  This is exactly F-071's class, which DRIFT-024 answered by excluding three
  derived indexes from the corpus. **That audit missed this one**, and the
  reason is instructive: it searched for the wording those three used, and ATLAS
  says «GENERATED from» rather than «hand edits are a defect». *A phrase sweep
  is not an audit — wave 1 wrote that lesson down and this is its second
  instance.* Its internal arithmetic is **not** drifted (87 rendered entries;
  axis, evidence-class and status distributions each sum to 87). The question is
  scope, the same shape as F-080, and it is the owner's.

- **2026-07-26 · F-089 and F-090 — PROP-014 against itself, twice more.**
  **F-089:** its `##HOME-SHIPS-WITH-THE-DISCIPLINE` names the Rust
  implementation as «`specmap-core` + the `rust-ai-native-specmap` binary».
  `specmap-core` has **zero occurrences** anywhere in the repository; the real
  crate is `core-ai-native-specmap`, **which PROP-014's own §2.9 uses**. A
  pre-PROP-028 name the family rename missed — drift-stage work, and the unit
  stays `@impl/done` because an implementation does exist and only the name
  drifted. **F-090:** §2.7, §2.8 and §2.9 carry no `` `req rN` `` kind line
  while §2.1–§2.6 all do — against its own §3.1 principle 3, «*Normativity is
  marked, not implied … a reader must never guess whether a sentence binds*».
  Three of its own decision subsections make the reader guess.

- **2026-07-26 · One reported problem that is not one, recorded so nobody
  "fixes" it.** B2 flagged `01-PATTERN-CARD-FORMAT.md:41` as carrying F-084's
  shape with a last-token marker. It does, **on purpose**: DRIFT-031 fixed the
  parser, and the marker was left in the position that used to break so the file
  is a live witness that goes red again if run-matching regresses. The gate
  confirms it — zero unmarked. The executor could not know, because the brief
  forbids it running the gate; the flag was the correct move on the information
  it had.

- **2026-07-26 · F-091 RULED — the book leaves the corpus, and the fifth
  subtraction is the first that is not about staleness.** Owner: «исключи
  `spec/book/**`». `flow:redbook`'s three Russian chapters (1 209 lines, **375
  facts**) plus the edition-plan README leave; the package's README and boot
  snippet stay. Corpus **264 → 260 files**, unmarked **5 068 → 4 685**, and B4
  dissolves — `redbook`'s two survivors fold into B16.

  **The package rules itself out in its own words.** Its boot snippet: «*Do not
  read the book at session boot — it is reference depth, not standing
  instructions.*» Nothing outside the package cites a chapter.

  **But the deciding argument is Phase C, not size, and it is worth keeping
  because the next case will look different.** Every marker earns a verdict, and
  `confirmed` has no meaning applied to a paragraph of philosophical prose. We
  would meet exactly the wall §3.3 and F-080 met — `--exhaustive` cannot express
  «marked, never verified» — while minting 375 `spec://` addresses on narrative
  nothing resolves against.

  **This is a new category, not another instance of the old one.** The four
  earlier subtractions were all *staleness or duplication*: machine copies,
  third-party licence text, derived indexes, frozen slots. The book is
  **authored, current, and the source of the discipline's spirit** — it is
  excluded for its **genre**, not its age. In `spec-genres`' terms it is lore,
  and the corpus is for contracts.

  *Recorded so the door stays visibly ajar:* re-admitting it needs a rule this
  campaign does not have — «lore is marked but never verified» — and that rule
  needs **tooling**, not just a glob, because the exhaustive counter is what
  forbids it today. The genre is worth having eventually: a flow citing the
  chapter that justifies its rule is exactly the two-way link `spec-genres`
  wants between a contract and its lore.

- **2026-07-26 · F-092 — a `SKILL.md`'s YAML frontmatter cannot carry a fact
  anchor, and the finding had been filed onto a closed id.** Nine files across
  six packages open with `---`, and the scanner has no frontmatter rule: the
  block parses as one countable paragraph whose first token is the opening
  fence, so `##FACT-ANCHOR-SYNTAX`'s first-token requirement has no legal
  placement. B5 met it on the two go skills, left them unmarked, and the landing
  commit said so honestly — **663 of 665 units**. B6 will meet it on the two
  TypeScript skills; B7 on the two Rust ones.

  **The shape is F-083's exactly, one structure later.** There too an unmarkable
  first token was structure the grammar did not name, and there too the fix was
  one composition in the parser rather than a convention. `blocks.rs` is where
  it goes: a leading `---`-delimited block is frontmatter, not a paragraph, and
  is not a countable unit at all.

  **What is worth recording is not the parser gap but the numbering.** This
  finding was written into the checkpoint files as *F-083* — an id F-083 already
  held, for the GFM task-list gap DRIFT-031 closed the same day. So one id named
  an open finding and a closed one at once, the checkpoint said F-083 was open
  while the task index said DRIFT-031 had closed it, and neither was wrong about
  its own half. It is renumbered here, and the plan is where it should have been
  filed on the day: **a finding recorded only in the checkpoint has no id
  authority** — the checkpoint is rewritten every session and cannot arbitrate a
  namespace. Sixth instance of the campaign's standing shape, and the first
  where the drifting artefact was the campaign's own bookkeeping.

- **2026-07-27 · F-093 — the Rust stack's published wiring recipe cannot work as
  written.** `GUIDE-AI-NATIVE-RUST.md` §13 step 3 tells a consumer to write
  `specmark = { path = "vibedeps/<stack-slot>/crates/vendor/specmark" }`. That
  directory does not exist: the vendored crate is `core-ai-native-specmark`, so
  the line needs both a corrected path **and** a `package =` key it does not
  have. The package's own `Cargo.toml` carries the same shape.

  **This is a different severity from the drift the campaign has been finding.**
  Every earlier finding was a document disagreeing with reality; this is an
  instruction that fails when followed. It is the *engine consolidation* landing
  in the crates and not in the recipe that names them — and the recipe is the
  first thing a new consumer runs.

- **2026-07-27 · F-094 — the same package's README is wrong in five places, and
  one of them cites a rule its own guide retracts.** `crates/specmark` (a third
  spelling of the specmark path, also absent); `schemas/specmap.jtd.json` (no
  `schemas/` directory exists); `specmap-core/src/generated/` (it is
  `crates/vendor/core-ai-native-specmap/src/generated`); the umbrella tool list
  omits `ledger`, which `vibe.toml` and the package's own boot snippet both
  carry; and the crate-naming line says names take the **`-rust` suffix** «per
  the GUIDE §2 language-suffix rule» — while GUIDE §2's `##FAMILY-PREFIX-RULE`
  says that policy is **superseded** by PROP-028 §2.4, and no crate in the
  package carries the suffix.

  **The go twin's README was updated for the same policy change and rust's was
  not**, which is what makes this a *fan-out* defect rather than a stale line:
  the correction reached one projection and stopped. Worth pricing before Phase
  D, because the family has three members and the next policy change will
  distribute the same way.

- **2026-07-27 · F-095 — the rust terraform skill duplicates the sweep skill's
  generic section where both twins wrote a raid-specific one.**
  `skills/rust-ai-native-terraform/SKILL.md`'s «The generation-time assistant»
  is verbatim the sweep skill's text; go and TypeScript both carry «The
  generation-time assistant **during raids**» with their own units. **Here the
  original looks like the copy** — the inverse of `##B7-RUST-IS-THE-SOURCE`,
  which B7's brief had told the executor to assume. It did not correct toward
  the twins, which was right; the finding is that the assumption has an
  exception and the next brief in this family should say so.

- **2026-07-27 · F-096 RULED — the discovery prompt leaves the corpus, and the
  sixth subtraction is the second one about genre.** Owner ruling, asked before
  B8 was dispatched rather than after it was marked: **exclude
  `DISCOVERY-PROMPT.md`** (169 units, 37 % of B8).

  **The deciding argument is Phase C, exactly as it was for the book.** Every
  marker earns a verdict, and `confirmed` has no meaning applied to a line of a
  prompt addressed to another model — «LLM = Claude/Haiku/Sonnet/Opus» is a
  variable assignment for a downstream session, not a claim about this system.
  The file is not prose at all: `<PROMPT_INFO>`, `<VARIABLES>`, macro
  substitution.

  **The package's own boot snippet rules it out in as many words:** «a **payload
  for a fresh session**, not standing instructions — do not load it into context
  outside an explicit deployment request.» Nothing outside the package cites it;
  the single external hit is the generated `spec/boot/STATIC.md`, which is that
  same snippet compiled into the host's boot lane.

  **The line the exclusion draws is worth keeping, because the next case will
  test it.** The package's README, boot snippet and `usage.md` **stay** — every
  document that makes a claim *about* the artifact is still measured. Only the
  payload leaves, and it leaves because it makes no claims about this system at
  all. That is a sharper rule than «lore is excluded»: *a document is in the
  corpus when it asserts something this project could be wrong about.*

  **The counter-argument, recorded because it is real:** the artifact is what
  the package exists to ship, so the corpus now holds a package described only
  by its own meta-commentary. The owner accepted that. The third option — mark
  it and carve it out of Phase C — needs the «marked, never verified» rule the
  F-091 entry says the campaign does not have and that needs tooling, not a
  glob; it stays unbuilt.

  **Sequencing note, and it is a near miss.** DRIFT-037 was running when this
  was ruled, and its acceptance is «exactly −9 unmarked, and no other file's
  count changes». Editing `progress.toml` mid-run would have moved the corpus
  under its before/after measurement and broken both halves of that acceptance.
  The ruling was written up immediately and **the config change was held until
  the task landed**. A scope edit is never urgent; a measurement in flight is.

- **2026-07-27 · F-097 — a package rename left its own install command behind,
  in sixteen files.** `flow:atomic-commits` does not resolve. Commit `520e7478`
  renamed the `git-practices` members and the package is `git-atomic-commits`;
  no `name = "atomic-commits"` survives anywhere outside the regenerated
  `vibedeps/` and `.vibe/cache/` copies. **Sixteen canonical files still cite the
  dead name** — B8 found twelve and the reviewer's sweep found four more.

  **The sharpest instance is the package's own README, lines 32 and 38:**
  `vibe install flow:atomic-commits` and `vibe uninstall flow:atomic-commits`.
  Those are copy-paste instructions, in the file a consumer reads first, for a
  package that is not called that.

  **This is the second finding in two days of a class the campaign had not seen
  before F-093: an instruction that fails when followed**, as distinct from a
  document that merely disagrees with reality. Both came from a rename or a
  consolidation reaching the code and not the prose that names it. Worth
  treating as a class rather than two incidents — the corrections that move a
  name are exactly the ones whose fan-out nothing checks.

  **It is wave-level, not B8's.** Every affected package is a `world` flow, so
  B9–B16 will each meet their own copy. Filing one DRIFT against all sixteen is
  cheaper than eight independent rediscoveries, and it must be a fact
  correction under sync-from-code rather than a markup fix.

  **WIDENED 2026-07-27 after B10, and measured as a class instead of found one
  batch at a time.** B10 met the same defect under a second name
  (`flow:attribution-policy`), which prompted a sweep of **every** package
  reference in canonical markdown against **every** declared package name. The
  `git-*` rename left **four** names dead, not one:

  | dead reference | real name | files | refs |
  |---|---|---|---|
  | `flow:atomic-commits` | `git-atomic-commits` | 16 | 20 |
  | `flow:attribution-policy` | `git-attribution-policy` | 6 | 8 |
  | `flow:conventional-commits` | `git-conventional-commits` | 2 | 3 |
  | `flow:autonomy` | `git-autonomy` | 1 | 2 |

  **21 distinct canonical files, 33 references, and 6 of them are literal
  `vibe install` / `vibe uninstall` command lines** — in three different
  packages' own READMEs, each telling a reader to install a name that does not
  resolve. Every one of the four is declared correctly under its `git-` name, so
  the rename landed everywhere except the prose that cites it.

  **The sweep is the point, not the count.** Three batches would each have
  rediscovered their own slice; one query against the declared-name list found
  the whole class in a minute, and it is repeatable at any time. **A reference
  that names a package is checkable mechanically, and nothing checks it** — that
  gap is worth a gate long after this DRIFT closes.

- **2026-07-27 · F-098 — a promise whose «next release» has shipped.**
  `wal/v0.2.0/README.md` says the Discipline «ships a convention document that
  defers to this package **from its next release**». That release is
  `core-ai-native/v0.8.0`; it contains **zero** occurrences of `flow:wal`, and
  its `spec/06-WAL-CONVENTION.md` defers to nothing. B9 marked it `@spec/done`
  as a claim about the future rather than a confident wrong `@impl/done` — the
  falsity is a Phase C `drift` verdict, not a markup fix.

  **The class is new and worth naming: a forward-dated claim that its own
  deadline has passed.** Unlike F-093 and F-097 it does not fail when followed;
  it simply stopped being true on a date nobody watched. Nothing in the campaign
  detects that — a marker records *stage*, and `@spec/done` is the correct
  marker for a claim about the future whether or not the future arrived.

- **2026-07-27 · Two errors in MARKUP-B9.md, both the reviewer's, both found by
  the executor.** Recorded because the brief-error streak is a measurement the
  campaign keeps, and it had read «the last three did not».

  **(a) A reconciliation paragraph for a disagreement no reader could see.**
  `##B9-PLAN-SAID-578` explained why the plan said 578 and the measurement 577
  — but the same commit that wrote the brief also corrected the plan, so by the
  time anyone read either, both said 577. Harmless and wrong: it describes the
  tree as it was in the author's head, not as it shipped. **Editing two files in
  one commit and then explaining the difference between them is a shape to
  watch for.**

  **(b) The brief contradicted itself between two sections.**
  `##B9-EXPECT-CORPUS-TOTAL` said «confirm the starting number with a gate run»;
  `{#boundaries}` said «do not run any `vibe` command». The executor honoured
  the boundary and confirmed the number two other ways. **A brief is a document
  like any other and nothing checks it against itself** — the mechanical
  reviewer checks the batch, not the instructions.

- **2026-07-27 · F-099 — a README miscounts its own contents.**
  `tool-design-lessons/v0.1.0/README.md:22` says the package «ships **four**
  pieces of content plus a boot snippet» and then lists **three**. Three exist on
  disk. Both sibling READMEs in the same batch say «three» and both are right.
  A self-describing count is the cheapest fact in a document to check and the
  easiest to leave behind when a piece is dropped.

- **2026-07-27 · F-100 — a grammar example cites a real package at a version it
  never had.** `qualified-naming/…/ref-grammar.md` §forms illustrates the
  versioned form with `org.vibevm.world/wal@0.6.0`. That package is real and it
  is at **v0.2.0**. The neighbouring §examples section deliberately uses invented
  groups to stay product-neutral; this row reaches for a real coordinate and
  gets it wrong. Low severity, and the shape is worth noting: **an example that
  borrows a real name inherits an obligation to be right about it.**

- **2026-07-27 · The paragraph-density hypothesis is dead, and the replacement
  is a mechanism rather than a fit.** B10 proposed that the markup multiplier
  tracks paragraph density; B11 was dispatched to test it and falsified it on the
  most paragraph-heavy batch yet — measured ×2.235 where density predicted ≥2.3,
  with a per-file correlation of **r = −0.171**, no relationship and the wrong
  sign.

  **What killed it is a controlled pair inside one package.**
  `self-updating-tools.md` and `packaging-lessons.md` share an author, a package,
  a four-field genre and an *identical* pre-composition — 29 paragraphs, 7 items,
  0 cells each — and produced ×2.45 against ×1.93. Independently counted at
  review they differ by **68 sentences against 55**, ratio 1.24 against the
  multipliers' 1.27.

  **Deconstruction produces about one unit per sentence**, which is what it is
  for; paragraphs were a proxy all along. `1.08 × sentences + items + cells`
  predicts B9, B10 and B11 within **0.8 %**, and sentences are countable by regex
  **before** a batch is dispatched. Recorded in `BATCH-PLAN.md` with an explicit
  instruction not to lock the coefficient — this is the rule's third version, the
  second was promoted from two measurements and falsified by the third, and three
  points is not proof either. The *mechanism* is what the controlled pair
  supports; the coefficient is three numbers.

- **2026-07-27 · F-101 — a template disagrees with its own worked example.**
  `comparative-research/…/research-template.md` writes capability, gap, lead and
  delta subsections at **h3** in its skeleton and at **h2** in the worked
  fragment beneath it, and gives every skeleton section a `{#anchor}` while the
  fragment carries one and omits the rest. Both sit inside fenced blocks, so
  they cost the markup nothing — **and a reader who copies the worked fragment,
  which is what a worked fragment is for, gets a shape the template's own
  commentary does not describe.**

- **2026-07-27 · The sizing quantity is misnamed, and the name is the hazard.**
  B13 measured it: the recorded rule reads **274** where a true sentence count
  of the same files is **≈320** — a structural **+17 %** from 34 paragraphs
  ending in a colon before a fence or list and 13 terminators swallowed by a
  following `**` or backtick.

  **The coefficient is fitted to that undercount.** A future implementer who
  repairs the counter toward real sentences will read ≈320 on B13, derive ≈0.95,
  and — sizing with the recorded 1.08–1.15 — over-predict by an eighth. Recorded
  in `BATCH-PLAN.md` at the top of the rule, with the instruction that
  correcting the counter requires re-deriving every coefficient in the same
  commit. **Calling a measured quantity by the name of the thing it approximates
  is an invitation to improve it into wrongness**, and this is the campaign's
  cleanest example of the class.

- **2026-07-28 · F-102 — a fence matched by prefix inverts the parse, and the
  units it invents cannot be marked.** Both the gate and the review tool closed
  a fenced block on any line opening with three backticks, so a four-backtick
  block quoting three-backtick ones was closed by its own first inner opener.
  After that point the parse ran inverted: the quoted commands became paragraphs
  `check --exhaustive` demanded a marker for, and the prose between them became
  code it could not see.

  **The demand was unsatisfiable in both directions**, which is what makes this
  F-092's genre rather than an ordinary miscount: `##FENCE-AWARE` means a marker
  written at those lines is not read as one, so the unit stays unmarked — and
  writing it would edit a skeleton consumers copy verbatim. Eleven units stood
  in that state, all in `manual-tests` (`test-template.md` 8, `authoring-rules.md`
  3), every one of them a shell command.

  **Measured before it was fixed, not after:** two files in the whole corpus
  carry the construct, both in B14, and no already-marked file is affected —
  so nothing landed carries a marker on code. The gate went 870 → 859 and the
  log diff outside those two files is empty.

  **It is the fifth instance of one defect — a delimiter matched by prefix
  instead of by run** (F-084 was the fourth, and it rewrote `blank_inline_code`
  while leaving the fence scanner ten lines above it alone). Fixed by
  `c813b849`, which also gives the two rules one home each
  (`parse::delimiters`, `batch_review::fences`), because living apart is why
  they drifted. Both sides carry three controls.

  **Why neither instrument caught it:** they had the identical defect, so they
  agreed. Two of the tool's four earlier bugs were found precisely because it
  and the gate *disagreed*; this one is the case that argument does not cover,
  and the only thing that surfaced it was a third implementation — a
  reviewer-side sentence counter written to CommonMark's rule — reading 130
  paragraphs where the gate read 141.

  **Third site, found by looking rather than by failing: `vibe-spec`'s
  `fence_mask`** (`1e1badda`). It drives `DocTree` and the directive scanner —
  a different consumer of the same Markdown — and carried the run-length defect
  *plus* one of its own: it closed on any line merely starting with the
  delimiter, so `` ```rust `` inside a block ended it. **That half was live on a
  marked corpus file**: `core-ai-native/v0.8.0/spec/01-PATTERN-CARD-FORMAT.md`
  opens a block at line 67 and writes ` ```card-ops ` at 84, so lines 85–93 were
  scanned as document prose. `progress-core`'s closer always required the whole
  line to be delimiter characters, **so the two parsers had disagreed about that
  file for as long as it existed and nothing compares them.** The run-length half
  was live here too and invisible only because the quoted headings in both
  template files happen to land inside masked regions — luck, not design.

  **A lesson about probes, paid for in this finding.** The first test written to
  demonstrate the `vibe-spec` bug asserted `children(real).is_empty()` and
  **passed against the broken code**: a quoted `#` heading is level 1, so it
  attaches to the ROOT, not to the section above it. The bug only appeared once
  the assertion counted the whole tree, and only after a negative control proved
  the test could fail at all. **A probe that passes is evidence about the probe
  until a control says otherwise.**

- **2026-07-28 · F-103 — a boot snippet whose every in-package link is broken,
  and a README that contradicts its own manifest.** `sync-from-code` keeps its
  snippet at `boot/20-flow-sync-from-code.md`, not `spec/boot/`. All three of
  its relative links point at `../flows/sync-from-code/*.md`, which from `boot/`
  resolves to `<pkg>/flows/…` — a directory that does not exist; the files are
  at `<pkg>/spec/flows/…`. Verified by resolving each. They work only after the
  snippet has been installed into a consuming project. The two sibling snippets
  live at `spec/boot/` and resolve correctly in place. **And the README says the
  package ships `spec/boot/20-flow-sync-from-code.md` while `vibe.toml` declares
  `source = "boot/20-flow-sync-from-code.md"`** — the two sibling READMEs match
  their manifests exactly. F-097's genre: an instruction that fails when
  followed. Four more packages share the bare-`boot/` trait and are B15's.

  **WIDENED 2026-07-28, while sizing B15: it is universal, not incidental.**
  Every relative link in every bare-`boot/` snippet is broken — **8 of 8**
  across five packages (`sync-from-code` 3, `git-atomic-commits` 2,
  `git-autonomy` 1, `git-conventional-commits` 1, `dev-runtime-docs` 1),
  resolved one by one rather than sampled. Not one of these packages can be
  read correctly from its own tree; all of them read correctly once installed.
  **The `spec/boot/` packages have no broken links at all**, so the trait and
  the defect coincide exactly. This is a wave-level fact correction under
  sync-from-code, alongside F-097 — one DRIFT, five packages, eight links.

- **2026-07-28 · F-104 — a protocol that misplaces its own skeleton.**
  `LICENSING-PROTOCOL.md` says «A skeleton of this text ships with the
  `draft-eula` skill». It ships in `spec/flows/licensing/eula-template.md`; the
  skill only points at it, and says so in its own step 2.

- **2026-07-28 · F-105 — a shipped flow package cites this repository's unbuilt
  command and its private milestone numbering.** `when-to-apply.md`'s boundary
  list says «**`vibe build`** (M1.5+) handles the other direction — generating
  code from spec». **There is no top-level `vibe build` at all**: the only
  `Build` in the CLI is `vibe bin build`, which builds package-declared tools,
  and PROP-018 §SEAM-DECOUPLING names the spec-to-code command as «a future
  deterministic command». A consuming project has neither the command nor the
  milestone scheme. Marked `@spec/done` by adjudication (ruling 10, a claim
  about the future) after the batch left it `@unknown` — correctly, since the
  ambiguity was real and the answer needed the host's CLI.

- **2026-07-28 · F-106 — one package, two vocabularies for weak copyleft.**
  `dependency-licenses.md`'s table reads «MPL-2.0, EPL»; `LICENSING-PROTOCOL.md`
  §deps and the boot snippet both name only MPL-2.0 for that class. `EPL` occurs
  exactly once in the package. *(The count needed care: a case-sensitive grep for
  `EPL` also matches the anchor `##AT-RELICENSE-TIME-THE-PLACEHOLDER-IS-REPLACED-WHOLESALE`,
  because `REPLACED` contains it. The corpus's own markup is now a source of
  substring false positives — a new instance of the WAL's standing warning.)*

- **2026-07-28 · F-107 — three sibling packages of one generation, three licence
  conventions.** `sync-from-code` ships `LICENSE` (no extension) and cites it
  unlinked with a sentence about the registry; `licensing` ships `LICENSE.md`
  and cites it unlinked; `manual-tests` ships `LICENSE.md` and links it. F-070
  excludes `LICENSE.md` from the corpus by name, so the odd one out is also
  excluded only because it is not markdown.

- **2026-07-28 · Reported and NOT a finding: the dotted anchor.** B14 reported
  that `review-workflow.md` teaches naming an anchor `{#verification.timeout}`,
  which `##FACT-ID-GRAMMAR` (`[A-Za-z][A-Za-z0-9_-]*`) rejects. **It does not
  survive checking.** The grammar governs *fact ids*; heading `{#anchor}`s share
  the address space without inheriting the character class. Every occurrence
  corpus-wide is an illustration in a fictional project — two inside fenced
  blocks in `ADDRESSABLE-SPECS-PROTOCOL.md`, one inside a fenced example in
  `record-template.md`, the rest in inline code. Nothing mints a dotted anchor.
  Recorded because a checked-and-dismissed report is worth as much as a
  confirmed one, and the next batch should not re-file it.

- **2026-07-28 · F-108 — an umbrella whose prose contradicts its own manifest,
  three lines apart.** `git-practices/vibe.toml` declares **four** members —
  `git-conventional-commits`, `git-atomic-commits`, `git-autonomy`,
  `git-attribution-policy` — and the comment immediately above that block says
  the family «grows to include human-authored attribution (§12.1) and commit
  autonomy (§12.4) **as those members land**». They landed; the same file pins
  them. The manifest's `description` field repeats the stale promise, and the
  README lists **two** members where the closure pulls four, so a consumer
  reading the README under-counts the family by half. Found while sizing B15;
  both packages exist on disk and are verified present.

- **2026-07-28 · F-109 — a shipped manifest cites a path only this repository
  has.** `git-practices/vibe.toml` line 23 points a reader at
  `neworder2/memory/BACKLOG.md`. The file is real **here** (9 tracked files
  under `neworder2/`) and reaches no consumer of `flow:git-practices`. Same
  genre as F-105: a package that can only be read correctly from inside the
  repository that produced it.

- **2026-07-28 · F-097 gains two sites and, more usefully, a warning about how
  its site list gets built.** B15 found two dead-name references outside the
  criterion the brief used (`flow:`-prefixed or backtick-delimited): a **bold**
  one at `git-atomic-commits/…/boot/30-flow-atomic-commits.md:23` — in a seventh
  file the brief's table did not list, and an installed boot snippet at that —
  and an **undelimited** one at `git-atomic-commits/README.md:60`
  («atomic-commits is how that commit is shaped.»). So B15 carries **16**
  package references by meaning where the brief counted 14 and the review tool's
  C12 sees 10, each number correct under its own criterion.
  **The actionable part: if the queued wave-level DRIFT builds its site list
  from a delimiter-anchored grep it will silently skip both.** That is the third
  time this campaign has been bitten by a grep criterion, after
  `grep -v '\.vibe'` and the naive heading count. **The brief's own count was
  the error** — recorded as such rather than as an under-scoped grep, because a
  number presented as "the dead names in this batch" was short by two.

- **2026-07-28 · F-110 — the README/manifest boot-path disagreement is the whole
  bare-`boot/` family, not one package.** All four B15 packages that keep their
  snippet at `boot/` name it as `spec/boot/NN-…` in their READMEs while their
  manifests declare `source = "boot/NN-…"` — `git-atomic-commits`,
  `git-autonomy`, `git-conventional-commits`, `dev-runtime-docs`, verified one
  by one, and `sync-from-code` (F-103) makes five of five. F-103's text is
  scoped to *links inside snippets*; these are *paths inside READMEs*, the same
  root cause reaching a second surface. One DRIFT covers both.

- **2026-07-28 · F-111 — a closed vocabulary maintained by hand in two places.**
  `git-conventional-commits` states its eleven allowed types as an eleven-row
  table in `conventional-commits.md` and again as an inline list in
  `boot/31-flow-conventional-commits.md`. **They agree today** — checked type by
  type, all eleven present in both, no extras. Two hand-maintained copies with
  no generator, one of which installs into every consuming project's boot lane;
  and the sibling `git-attribution-policy` states «one policy, one place» as a
  law of its own. *Low severity today, and it is the shape that is worth the
  finding rather than any current divergence.*

- **2026-07-28 · F-108 gains its cross-file half, which makes it checkable from
  the corpus alone.** `git-autonomy/README.md:28` says «A member of the
  `flow:git-practices` family» while the umbrella's README says the family
  «grows to include … commit autonomy **as those members land**». The member and
  the umbrella contradict each other directly, without reference to the manifest.

- **2026-07-28 · Two low-severity illustration drifts, recorded and not filed
  separately.** `git-atomic-commits` illustrates one claim with different
  arithmetic in two files — «without also undoing **three** correct things»
  (`ATOMIC-COMMITS-PROTOCOL.md:65`) against «**two** correct things»
  (`boot/30-…:52`); neither is wrong, and a reader comparing them sees a
  discrepancy. And `splitting-large-changes.md` asserts «Six items collapsed to
  four commits» before the sub-section that says the fourth should probably be
  reverted.

- **2026-07-28 · F-112 — one line in a shipped README that is wrong in three
  ways at once, found while sizing B16.** `go-ai-native-mcp/v0.1.0/README.md:9`
  points a reader at `spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md`.

  1. **The path is stale.** `spec/terraforms/` holds two files and that is not
     one of them.
  2. **The file it means is in the archive.** It lives at
     `legacy-spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md`, and the WAL's standing
     constraint is that nothing in the living corpus may cite into `legacy-spec/`
     as a normative source — archive-provenance pointers only. So the *corrected*
     path would still be a violation.
  3. **It is a host path in a consumer's package** (F-105, F-109's genre): a
     project installing `mcp:go-ai-native-mcp` has neither directory.

  **This is the sharpest instance of "an instruction that fails when followed"
  the campaign has produced, because it fails for the author too** — every other
  member of that class at least worked from inside this repository.

  *Two smaller facts from the same file, recorded so the batch does not re-file
  them: it is a **stub** («This README is finalized at campaign close»), which is
  why it carries 2 units where its two MCP siblings carry 5; and its H1 is the
  only one of the three that reads `# <name> (mcp:…)` rather than `# mcp:…`.
  The three stack aggregators are uniform by contrast — all three read
  `# AI-Native <Lang> (stack:…)`.*

- **2026-07-28 · F-113 — `redbook`'s roster is wrong in three documents and no
  two of them agree.** Counted: the manifest pins **22** members, the README's
  «Members (edition 0.2.0)» table has **21** rows, and the boot snippet lists
  **23**. The README **omits `git-practices` entirely** — which the manifest pins
  twice and whose own comment reads «attribution-policy now arrives via
  git-practices» — and omits the whole cultural-extraction wave
  (`dev-runtime-docs`, `wal-specspaces`) that the manifest pins and the snippet
  lists. Two of the names it does list are dead (F-097). So a consumer reading
  the front door gets 21 names, two uninstallable and three missing.

- **2026-07-28 · F-114 — the edition contract is falsified by the manifest that
  implements it, three lines from the pins.** `redbook/README.md` §editions
  states «An edition is a tested set … two projects on the same edition run
  byte-identical practice text» and «a new edition is a new umbrella version with
  refreshed pins». The manifest's own comment, immediately above the
  cultural-extraction members, reads: «**edition bump to a clean 0.3.0 lands when
  the full new practice set has settled; accumulated here in place meanwhile**».

  **So the umbrella accumulates members inside 0.2.0 without an edition bump, and
  says so.** Two projects that installed `redbook 0.2.0` on either side of that
  wave do not run byte-identical practice text — which is precisely the promise.
  This is the campaign's cleanest instance of a **normative claim contradicted by
  its own implementation**, and it was found only because B16's brief was the
  first to ask an executor to read the manifest before marking an aggregator
  (now ruling 61).

- **2026-07-28 · F-115 — the TypeScript stack's front door dead-ends.**
  `typescript-ai-native/v0.6.0/README.md` sends the consumer to «the `-lang`
  package's README»; `typescript-ai-native-lang/v0.6.0/` has **no `README.md`**.
  The rust and go `-lang` packages both have one, so TypeScript is the only stack
  of three whose aggregator points at a file that does not exist.

- **2026-07-28 · F-116 — twelve divergences across three near-identical
  siblings, three of them normative.** The `discipline-mcp-{rust,go,typescript}.md`
  briefs are the same document per language. **(1)** The capture guarantee
  («Reports are whole») is a standalone paragraph in rust, folded into a sentence
  in ts, and stated **twice** in go. **(2)** The claim that the enumeration test
  pins the parity-map list — checkable and normative — exists in rust and go and
  **is absent from ts**, whose heading is followed straight by the table.
  **(3)** ts cites `TCG-PROTOCOL-v0.1` unqualified where the siblings say
  `TCG-PROTOCOL-RUST-v0.1` / `-GO-v0.1`, which in a repository shipping
  `flow:qualified-naming` is now ambiguous. Nine further divergences are phrasing
  or scope: a `force`-class rule only rust states, an oracle-fidelity caveat every
  brief carries except the one whose oracle is not the reference implementation,
  and a go sentence citing a «named delta over the Rust one» that the rust brief
  does not record.

  **The method is the finding.** These three files took about a third of B16 and
  produced two thirds of its harvest, because marking them consistently forces a
  reader to notice every place they are not. **Near-identical siblings belong in
  one batch on purpose.**

- **2026-07-28 · The brief's own error, and it was mine.** B16's `{#anchors}`
  headline said «nine heading anchors owed»; its own itemisation summed to eight,
  and eight is what a fence-aware scan of the pre-state finds. **Fifteen of
  twenty-four briefs have now carried a factual error found by the batch running
  them**, and B16's executor named the pattern behind the last three: they were
  **counting-unit slips, not arithmetic** — names against sites, headline against
  itemisation, units against terminators. *State the unit with every count.*

- **2026-07-28 · F-097 CLOSED by DRIFT-038 (`7b0ec6aa`), and its own count was
  right about the wrong denominator.** 50 edits over 29 files; zero live dead
  package references remain. The recorded 21 files / 33 references was **exact
  for `flow:`-prefixed sites inside `packages/`** — and 12 more live sites sat
  in backticks, in bold and undelimited, with 5 further in the host's
  `PROP-003`, which no sweep had scanned. **Following B15's own warning about
  delimiter-anchored greps is what turned 33 into 50.**

  **Nine occurrences were left deliberately, and the reason is the finding's
  sharpest edge: a path is not a name.** The flow *directory*
  (`spec/flows/atomic-commits/`), the *document*
  (`conventional-commits.md`) and a `spec://` URI's flow and document segments
  all keep the short name and are correct — one URI spells the dead string twice
  while being entirely right. A blanket replace would have corrupted paths three
  ways. The other four are in `redbook/v0.1.0`, a superseded slot that is frozen
  history and is not edited.

- **2026-07-28 · F-103 and F-110 CLOSED by DRIFT-039 (`521bb6cd`) — and the fix
  was the layout, not the links.** The eight broken links were **already correct
  for the installed form**: from a consumer's `spec/boot/`, `../flows/…`
  resolves. Rewriting them would have fixed five packages and broken every
  consumer. What was wrong was that the in-package layout did not mirror the
  installed one, so the snippets moved `boot/` → `spec/boot/` and their five
  manifests followed.

  **F-110 dissolved without an edit.** All five READMEs already named the
  snippet `spec/boot/NN-…`, disagreeing with their own manifests — **they were
  right and the layout had never caught up to them.** A finding recorded as "the
  README contradicts the manifest" was actually "the manifest contradicts the
  README", and only doing the fix revealed which.

  *Checked and found false before it was written: the layout is not collateral
  from the `git-*` rename. `520e7478` carried it forward; all three renamed
  members had it before.* What the rename did expose is that
  `git-attribution-policy`, renamed by the same commit, already used
  `spec/boot/` — the family was split against itself.

- **2026-07-28 · PHASE C OPENED — C0: the harvest exists, and the phase is
  smaller than its own kick-off said.** Batch plan ratified with three decisions
  ([`PHASE-C-BATCH-PLAN.md`](../../campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md),
  `0acc448f`); fifteen units of work — C0, then C1…C7 over ai-native, then W1…W7
  over world.

  **The size correction first, because it is the same slip B16 named.** The
  kick-off's «~11 900 markers carry no verdict» is `12 797 − 921`, and 921 is the
  length of `baseline.json`'s `units` dict while 12 797 counts markers. Measured
  in the unit a verdict is written in — a fact that is marked **and** anchored,
  which is what `progress_core::seal::addressable` admits — the corpus holds
  **11 288 addressable anchors**, the host already carried **4 440 of its 4 441**,
  and the phase owes **6 848**. That is **1.54× wave 1's 4 455**, not the 2.6× the
  kick-off advertised.

  **C0's harvest is thirty-one captured runs (`874070ff`), and the result is
  one-sided.** Every slot's floor exits 1. The portable steps pass wherever their
  toolchain is present — `cargo fmt`/`test`/`clippy` green in both Rust slots and
  in `core-ai-native`'s hand-run floor — and **every discipline-specific step
  fails or is skipped in all six slots**, because no package under
  `packages/org.vibevm.ai-native/` carries a `conform.toml` or a
  `discipline/registry/`. Three defects are not that absence in disguise: the Go
  floor reports `gofmt: unformatted` and five conform findings against
  `tools/go-extract/test/fixtures/dirty/`, **a fixture tree whose whole purpose is
  to be dirty**; the Go engine's own error names the *Rust* binary («run
  `rust-ai-native-specmap` … first»); and the TypeScript stack cannot parse at all
  — the structural gate parses through the project's own `typescript` install and
  the package ships no `package.json`, so node walks out of the package and
  reaches the user's home directory looking for one. `core-ai-native` was captured
  differently on purpose: it ships **library crates only**, so no `floor`
  subcommand exists for it and the three portable steps were run by hand.

  **The mechanism was proved on ground where the answer was already known.** The
  five stale host files were re-derived, four verdicts written by load-and-merge
  (one minted, three evidence-extended), and the *tool* sealed them — 5 sealed, 0
  refused. The control: the 53 campaign maps the batch did not touch came through
  **byte-identical**, and a subsequent full `check` and `scan` left the verdict
  count at 4 499. Host is now **4 441 / 4 441 with zero stale files**.

  *The re-derivation surface was four facts, not the thirty-nine a first pass
  reported — and the first pass was the instrument lying.* Grouping anchors by
  blank-line block makes a bulleted list one unit, so a two-line rename inside one
  bullet marked all thirteen bullets changed. Re-cut at the right grain — from
  `##ID` to the next `##ID` — PROP-003's change is exactly `SHORTFALL-MONOLITH`
  and `OPEN-WORKSPACE-MONOREPO`, both the same package rename inside a `@spec`
  claim the rename does not touch. **A rule approximated instead of read, for the
  fifth time in this campaign, and this time in a tool I wrote to check the tools.**

- **2026-07-28 · F-117 — the phase's own kick-off documents a cache field that was
  deleted.** [`PHASE-C-KICKOFF.md`](../../campaigns/packages-2026-09/PHASE-C-KICKOFF.md)
  states the per-file campaign map as
  `{verify_batch, verified_at, processed_hash, verdicts{anchor → {v, ev[]}}, summary}`,
  and the paste-ready prompt inside it repeats the shape. **`summary` does not
  exist**: DRIFT-033 removed it on the owner's F-077 ruling, `FileRecord` has no
  such field, the tally is computed on read by `FileRecord::verdict_summary`, and
  all 58 campaign maps on disk carry exactly four keys. A session that followed the
  kick-off literally would write a stored tally back into the corpus —
  **reinstating precisely the defect the ruling removed**, in the file the ruling
  is recorded in. The kick-off was written six days after DRIFT-033 landed.

- **2026-07-28 · F-118 — wave 2 ran sixteen batches with no journal.**
  §4 binds this campaign to wave 1's crash-safety protocol — «step = unit of
  atomicity, journal `step-start` before and `step-done` after, `RESUME.md`
  regenerated, maximum loss on any crash is one step». `campaigns/packages-2026-09/run/`
  contained **no `journal.jsonl` and no `RESUME.md`** at the Phase C opening, so
  none of that ran for the whole of Phase B: there was no step to recover to, and
  `vibe progress resume` — «the first read of every campaign session» — had
  nothing to generate from. The visible symptom was ignored for a month:
  `campaign.json` reported **phase «A»** from the zone's creation through B's
  close, because the phase is derived from the journal's last `phase` event and
  there was none, so the compiled-in opening phase stood. **No command writes a
  phase event** — wave 1 hand-appended all five — which is why nothing failed
  loudly. The journal was opened at C rather than back-filled; B's steps are not
  reconstructed, because a record is not invented after the fact.

- **2026-07-28 · F-119 — the book cites a chapter that was never written, in the
  live slot as well as the frozen one.** `redbook`'s chapter 1 links to
  `safeharbor.md` from `spec/book/ru/chapter-1-two-process-model.md`; the file
  exists nowhere in the repository, and the citation stands in **both** `v0.1.0`
  and `v0.2.0`. The v0.1.0 copy is frozen history and is not edited (§3.3); the
  v0.2.0 copy ships to consumers. It is the **only** broken citation in the whole
  `world` tree — 187 relative citations resolved, 2 broken, and both are this one.

  It was invisible to Phase B and to the exit gate for the same reason: the
  campaign's `exclude` globs drop `redbook/*/spec/book/ru/`, so the observed corpus
  is 121 of the tree's 154 files. **Inside the corpus the join is clean — 185
  citations, 0 broken** — which is the §3.1 source-1 result the `world` cluster
  rests on, and it is clean *because* the one defect sits in a file nothing
  measures. A scope that excludes a shipped directory does not stop shipping it.

- **2026-07-28 · C1a — the transport holds completely and the engine spec is half
  false.** Two of C1's five mechanism files judged, 77 verdicts.

  **MCP-CORE-v0.1: 34 / 34 confirmed, zero drift.** Every claim is in the code and
  most are under test — `PROTOCOL_VERSION = "2024-11-05"`, `-32700` on a malformed
  line with a test asserting the loop survived it, `-32602` / `-32601` in
  `tools/call`, a `BTreeMap` making `tools/list`'s sorted order structural rather
  than sorted-at-call-time, `dup2` and `SetStdHandle` into a **file** (not a pipe,
  which would deadlock a chatty floor), restoration on `Drop` under
  `catch_unwind`, and a nesting refusal whose message is asserted to cite this very
  unit. Two claims were checkable only outside the package and both held:
  `diff -rq` reports the vendored copies in `rust-ai-native-mcp` and
  `typescript-ai-native-mcp` **byte-identical**, and PROP-027 §2.6 «Serving is
  vibe-free» exists in the host exactly as cited. The prohibition «tools MUST NOT
  prompt» turned out to be structural rather than documentary: `fn run(&mut self,
  args: &Value) -> ToolOutput` gives a tool no input channel, so prompting is
  unrepresentable at the seam rather than forbidden beside it.

  **ENGINE-CONFORM-v0.1: 21 confirmed / 22 drift — 48.8 %.** The engine that
  exists is a good one, and it is not the engine this document describes. **There
  is no `Tier` type, no `tier` field and no escalation anywhere**, so the whole of
  §1 — three tier rows, the rule-record declaration, the cheapest-adequate-frontend
  claim — describes a mechanism that was never built; `store.rs` takes a
  caller-supplied `&dyn Frontend` and nothing selects. Of five frontends tabled,
  **C++ and Python do not exist at all** and tree-sitter, SWC, rust-analyzer,
  `rustc_driver` and gopls appear nowhere in the tree. Two of three example rules
  (R-021, R-020) do not exist; R-002 does, and is mapped onto tiers that do not.
  Foreign-linter ingestion runs the wrong way: conform **emits** SARIF and reads
  none. The quoted trait signature is wrong in both parameters —
  `fn check(&self, facts: &[SourceFacts])` against the document's
  `fn check(&self, facts: &FactStore, specmap: &Index)` — and the `specmap`
  argument could not exist, because the conform crate does not depend on the
  specmap crate.

  What confirms is worth naming too, because it is not nothing: the fact store's
  `(content-hash, frontend id+version)` key is exact, the sidecar protocol is
  exactly the NDJSON the spec describes (`tools/go-extract/extract.go:4`), SARIF
  output and `conform-baseline.json` are both real, and `findings.sort()` gives the
  byte-identical ordering §5 claims.

- **2026-07-28 · F-120 — a notation used 102 times, defined by one example, cited
  to a document that does not exist.** The kind-line grammar (`` `req r1` ``) is
  defined in exactly one place: BROWNFIELD-PROTOCOL-v0.1.md's
  `##UNIT-STATUSES-ARE-KIND-LINE-GRAMMAR`, which gives examples — «`req r2`
  (default: ratified) · `req r1 planned` · `req r2 disputed(#other-anchor)`» — and
  cites **GUIDE-SPEC-AUTHORING** for the amendment. **No such document exists in
  the repository**; `01-PATTERN-CARD-FORMAT.md` says it «supersedes part of
  GUIDE-SPEC-AUTHORING-v0.1», so the only cited definition source is a document
  that was partly superseded and never shipped. Meanwhile the corpus uses eight
  ranks — r1 ×75, r2 ×10, r3–r8 ×13 — and **none of r1…r8 is defined anywhere**.
  The one definition-bearing sentence pairs `r1` with «planned» and `r2` with
  «ratified»; 75 of 102 uses are a bare `r1` carrying neither word.

  *Recorded as a finding rather than as five unverifiable verdicts.* The
  `kind-line-*` facts carry `@impl/done`, whose stage semantics ask for presence
  evidence for the unit the line heads, and that evidence exists — four of the five
  head cells with `specmark::scope!` tags citing them. Marking them unverifiable
  would report as unmeasured a corpus that is in fact implemented; the undefined
  rank is a defect in the notation, not an unfalsifiable claim about the world.

- **2026-07-28 · F-121 — a document falsified by its own last line.**
  ENGINE-CONFORM-v0.1's closing fact reads: *«Any frontend or tier specified here
  that is not exercised by Playbook Phase 4 is removed from this document rather
  than carried as aspiration»* — and it is marked **`@impl/done`**. The same
  document carries three tiers, a C++ frontend, a Python frontend, tree-sitter,
  SWC, rust-analyzer, `rustc_driver`, gopls and SARIF linter ingestion, and the
  engine exercises none of them. Nothing enforces the rule: no test, gate or lint
  checks it, so the one fact whose job was to keep the document honest is the fact
  with no checker.

  It is the sharper twin of F-114, and the pattern now has two instances: a
  normative claim contradicted not by the world but by the artifact that carries
  it. The document's own status line says «Design, beta» — accurately — while
  thirty of its facts are marked implemented. **A design document is not drift; a
  design document whose facts claim implementation is.**

- **2026-07-28 · C1b — LEDGER-INTENT, and F-121 turns out to be a family.**
  40 verdicts: **26 confirmed / 12 drift / 2 unverifiable**. The ledger that
  exists is the honest core of the design — a content-addressed store, epochs
  derived from the meaning-context rather than the file hash, hard no-serve on a
  stale epoch, a provenance line in every render, immutable content-keyed entries
  that make last-write-wins benign. Its exclusion from git is real and enforced in
  the only place it can be: the host's `.gitignore` carries `/.ledger/` under this
  mechanism's own name.

  What is missing is everything above the floor. There is **no LLM producer at
  all**, so the interpretations key is producer + epoch + subject where the
  document lists six components including `prompt_rev` and a model id; there is no
  entry struct, only a text blob, against eight promised provenance fields; no GC,
  no pin set, no size budget; no export, freeze, sign or ship path for a release
  slice; and two of the four query kinds — `classify.legacy_unit` and
  `propose.links` — do not exist. `Telemetry` counts hits, misses and rot, and
  neither of the two cost measures the headline metric needs.

  **Two facts were recorded `unverifiable` rather than either verdict**, which is
  the distinction this phase exists to keep. «Every LLM output belongs to the
  interpretations class, without exception» governs outputs that do not exist —
  the rule has no instances, so it is neither working nor broken. «A new query kind
  is added only when two consumers ask» is a governance rule that no registry, gate
  or record encodes; with one kind in the tree it is unviolated and unexercised.
  Calling either confirmed would report an untested policy as a working one.

  **And the closing fact is ENGINE-CONFORM's closing fact again**: «anything not
  exercised by Playbook Phase 5 is deleted, not aspirational», `@impl/done`,
  contradicted above it by GC, release slices, signing, two query kinds,
  `prompt_rev`, a model id and cost telemetry. **F-121 is a family, not an
  instance** — two mechanism documents, each ending with a self-cleaning rule that
  neither obeys and nothing checks.

- **2026-07-28 · C1c — BROWNFIELD, where the protocol is mostly built and the
  brief was too narrow.** 63 verdicts: **51 confirmed / 9 drift / 3
  unverifiable**. The registries this protocol specifies are real, and the
  delegated search could not see them because **my brief confined it to
  `core-ai-native`'s crates** — the machinery lives in the language stack's CLI.
  `rust-ai-native-cli/src/lib.rs:26,28,30` defines
  `discipline/registry/tests-baseline.json`, `debt.json` and `intent.json` exactly
  as named here, and `init.rs` / `test_gate.rs` / `tripwire.rs` / `ledger.rs` are
  the commands that write and read them. The worker marked those rows `partial`
  and listed what it had searched, which is what made the gap visible in one pass
  rather than becoming thirty wrong verdicts. **A `not-found` is a fact about the
  search perimeter until the perimeter is checked.**

  Three of the four amendments this document claims to make **landed and were
  verified in place**: the Charter carries `##AXIOM-A6-REALITY-BEFORE-ASPIRATION`
  in the words given here, PROP-014 §edges carries the lifecycle status and the
  `conflicts_with` edge marked «*(Brownfield amendment:)*», and the Playbook reads
  «Discipline v0.2 · status: BETA». **The fourth amends GUIDE-SPEC-AUTHORING,
  which is not in the repository** — the same missing document F-120 is about,
  now with a second dependant.

  The drift is where the protocol needs artifacts nothing produces:
  characterization goldens (no capture path anywhere, so B4's «truth of record»
  and the redefinition of phase gates as «snapshots unchanged» both rest on
  nothing), the REPORT exit numbers, the anti-entrenchment close-quota, and the
  `disputed` status's heuristic detection — the status exists in PROP-014's enum
  and nothing can assign it. **Three facts are `unverifiable` rather than drift**:
  the three adjudication outcomes govern a human decision that has never been made
  here, the characterization definition has no tests to be a definition of, and
  Phase 6's closing rule has never been applied. Each is sound and unexercised,
  and calling any of them confirmed would report an untested policy as a working
  one.

  **And the closing fact is the third instance of F-121** — «any field, status or
  policy not exercised by Playbook v0.2 Phase 2 is removed», `@impl/done`,
  contradicted above it by five things that do not exist.

- **2026-07-28 · C1 stands at 4 of 5 files, and PROP-014 stops at the boundary
  rather than at the bottom of a session.** 180 verdicts merged and sealed —
  **132 confirmed / 43 drift / 5 unverifiable, 73.3 %**. PROP-014's evidence is
  gathered, machine-verified and **not judged**: 173 rows, 118 of them
  `@impl/done`, 60 marked `partial` — the class that needs a reader, since a
  `partial` is precisely «related code that does not settle the claim». Judging
  ninety-five of those at the end of a long session is how a bad verdict gets
  written, and this phase rejects a verdict without evidence for exactly that
  reason.

  Four host-side checks were run and are recorded for whoever picks it up, because
  they decide the load-bearing rows: `vibe check` carries **neither** the
  edge-multiplicity lint nor the 120-line unit warning the document says it warns
  with; **no `specmap_query` / `specmap_explain` / `specmap_source`** MCP tools
  exist; and `[metamodel] profile` appears nowhere in the crates. The two
  companion PROPs it cites — 009 and 013 — do exist in the host.

  **A correction to this entry, made the same day it was written.** It first said
  «there is no host PROP-014 — the package copy is the only one in the repository,
  so this proposal was never adopted into the spec tree it proposes against». Both
  halves are false. There are some thirty copies (`vibedeps/`, `.vibe/cache/`,
  `research/*`, the superseded `v0.7.0` slot) — regenerated dependency copies, but
  copies. And the host **does** adopt it, by citation in the qualified form
  addressable-specs prescribes: `PROP-031` cites
  `spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#…` **five times**
  — as a companion, as «the read-side model this PROP makes writable», and as what
  its own `##BOUNDARY-COMPLETION` completes — and `PROP-024` links it too. The
  document's `##HOME-SHIPS-WITH-THE-DISCIPLINE` states the placement is deliberate:
  the mechanism ships with the Discipline, its Rust implementation with
  `rust-ai-native-lang`.

  **The check searched for a FILE under `spec/` and read its absence as absence of
  adoption.** That is the same shape as F-119 — a clean result produced by looking
  in the wrong place — and as the perimeter lesson one entry above, made this time
  by the reviewer rather than by a worker. What survives is narrower and minor: no
  file under the host `spec/`, and **`014` is an unexplained hole in the host's own
  PROP sequence** (so is `004`; the tree runs 000–013, then 015). A naming
  question, not an adoption one, and it bears on no C1 verdict.

  **The five evidence tables now live in the repository**, at
  `tasks/evidence/ev-*.json`, and re-verify from there: 748 refs, zero
  unresolvable, one OFF-BY and one ELIDED. They were written into a session
  scratchpad, which is ephemeral — five workers' output would have been lost at
  the session boundary, and the campaign's own erasure law says the derived
  artifacts may be deleted only because **the sources carry the knowledge**. An
  evidence table is not derived from the sources; it is a reading of them.

- **2026-07-28 · C1 CLOSED — 353 verdicts, and the perimeter was wrong three
  times.** **272 confirmed / 70 drift / 11 unverifiable — 77.1 %.**

  | file | conf | drift | unver | |
  |---|---:|---:|---:|---:|
  | MCP-CORE-v0.1 | 34 | 0 | 0 | 100.0 % |
  | BROWNFIELD-PROTOCOL-v0.1 | 51 | 9 | 3 | 81.0 % |
  | PROP-014-specmap | 140 | 27 | 6 | 80.9 % |
  | LEDGER-INTENT-v0.1 | 26 | 12 | 2 | 65.0 % |
  | ENGINE-CONFORM-v0.1 | 21 | 22 | 0 | 48.8 % |

  **PROP-014 read at 80.9 %, and it would have read far lower on the evidence as
  gathered.** Nine of its rows turned on facts outside the package: the pilot
  target `crates/vibe-resolver/src/conditional.rs` **exists**; the host carries
  **120 files with `#[spec(implements=…)]` and 402 with `specmark::scope!`**;
  `specmap.json` **is committed** (3.1 MB — 5 266 spec units, 898 code items, 912
  edges); `schemas/specmap.jtd.json` **exists**; `xtask/src/specmap.rs` and the
  `Trace` subcommand **exist**; and **`vibe trace` exists**, a delegating alias
  whose own help text cites «PROP-014 §2.6» — so the Phase-4 promotion this
  document plans has *happened*. Every one of those had been located as absent
  because I confined the search to `core-ai-native`'s crates, and PROP-014 is a
  mechanism whose implementation ships in `rust-ai-native-lang` and whose
  deployment is the host.

  **That is three perimeter errors in one batch** — BROWNFIELD's registries,
  PROP-014's deployment, and my own «never adopted» claim corrected above. The
  lesson has a sharper form than the one recorded in C1c: **the delegated
  `not-found` was accurate every time; the brief that produced it was not.** A
  worker cannot report a perimeter it was not given.

  **Where the index does fall short, it is exact and the index says so itself.**
  Read from the committed file: its keys are `code_items`, `edges`, `schema`,
  `spec_units`, `suspects`, `warnings` — so of the six contents §2.5 promises,
  **coverage-per-REQ and the orphans table are absent**; `code_item` carries **no
  content hash** where `spec_unit` does, which is exactly why «a code change keeps
  edges valid» is true and structural; and nothing constructs `Proposed` or
  `Generated`, so the whole LLM-proposer half of §2.7 is an enum variant with no
  producer. The runtime channel — MCP tools, `[metamodel] profile`, signing — does
  not exist at all, and the document's own open question calls signing *blocking*
  while marking the security invariant that depends on it `@impl/done`.

  **The rule that settled the awkward middle**, stated once and applied to every
  file: *a fact naming a specific artifact, flag, type, file or command that does
  not exist is drift; a fact stating a discipline the corpus demonstrably follows
  is confirmed, even where no checker enforces it.* It is why «every normative
  statement is addressable» confirms — this campaign's own gate exits 0 over 11 288
  anchors — while «`vibe check` warns beyond ~120 lines» does not: `vibe check` has
  eleven checks and no such warning.

  **F-121 closed at four instances.** ENGINE-CONFORM, LEDGER-INTENT, BROWNFIELD and
  PROP-014 each end with a rule that unexercised mechanisms are removed from the
  document rather than carried as aspiration; each marks it `@impl/done`; each is
  contradicted by its own contents; and none is enforced by anything. **Four
  documents, one habit: the rule that exists to keep a document honest is the rule
  nobody gave a checker.**

- **2026-07-28 · The `world` cluster's sources 2 and 3 become one command, and
  the host turns out to be one install behind.** §3.1's third source is
  unreadable as written — `files_written` is `[]` for all 36 packages — so the
  substitute ratified at the phase opening is the boot lane, which is *compiled*
  from the installed packages and carries a provenance marker per contribution.
  That marker is the join: package → installed copy → what the host actually reads.

  **17 of 31 contributions carry the package's exact word stream. Six differ, and
  the differences are small and specific**: `campaign-plans` by six words («cold
  facts verified at writing time», in the package and not in the host),
  `comparative-research` by three («sibling document pointers»). **Eight have no
  source at the path the installed copy names** — they were installed from `boot/`
  and DRIFT-039 moved the packages to `spec/boot/`, so the installed lane predates
  the layout fix. The whole lane was last written 2026-07-14; the packages were
  marked through 2026-07-27, and **0 of 32 installed snippets carry Phase B's
  markup while every package copy does**.

  None of that is semantic drift and all of it is source-3 evidence: the host runs
  what the packages ship, one install behind, and the compiler strips the campaign's
  markup on the way in — so the boot lane is a **rendering** of a snippet, never a
  copy of it.

  **The join also reproduces F-078 mechanically**, which was filed from reading:
  `git-atomic-commits`, `git-attribution-policy`, `git-autonomy` and
  `git-conventional-commits` each appear **twice** in `STATIC.md`, once directly and
  once through the `git-practices` umbrella that compiles its own members. Four
  flows, four duplications, exactly as the finding says.

  *Three wrong comparisons were made before the right one, and each was reported as
  a fact about the tree.* A byte compare called all 31 contributions drifted — that
  was the compiler stripping markup. A path derivation that took three components
  instead of four called all 31 unsourced. And the report diffed raw text while the
  decision was taken on stripped text, so it claimed 247 differing words where the
  verdict rested on 6. **The instrument was wrong three times in one afternoon and
  said so loudly each time; that is the only reason none of it reached a verdict.**

- **2026-07-28 · C2a — the ATLAS resolves, and the three things it says about
  itself do not.** 145 verdicts over ATLAS, CONTRADICTION-MAP and the README:
  **135 confirmed / 9 drift / 1 unverifiable — 93.1 %**, the highest rate of the
  phase and for a structural reason: these files are almost entirely *existence and
  reachability* claims, and a resolved address is its own evidence.

  **The ATLAS's arithmetic is exact and verifies twice over.** 87 distinct record
  ids and 87 `##FINDING-*` anchors against its claimed «unique (non-duplicate):
  87»; the axis distribution sums to 87, the evidence-class distribution sums to
  87. That is a generated appendix keeping its own books.

  **Its header is F-088, now confirmed against the tree.** «GENERATED from
  `findings.jsonl` (A2: derived, do not hand-edit)» — and **no `findings.jsonl`
  exists anywhere in the repository.** The only `findings.json` files are the two
  campaigns' F-NNN ledgers, a different artifact on a different schema. A generated
  file whose input is absent cannot be regenerated, and the instruction not to
  hand-edit it leaves no way to change it at all.

  **One bucket of the fourth distribution is manufactured.** `new=31` is **15
  records literally marked `new` plus 16 whose status field is EMPTY** — all BLD-*
  in axis H. The sum still reaches 87, which is exactly why nobody noticed: a
  reader counting genuinely-new findings is over by sixteen, and the total agrees.

  **A reachability measurement worth carrying forward:** of the 87 records, **24
  are cited by a card, guide or tool spec in the three `-lang` stacks and 63 are
  cited nowhere** outside the ATLAS itself. That is not a verdict — an index is
  allowed to hold more than the corpus uses — but it is the number Phase F will
  want when it asks what the research passes bought.

  The CONTRADICTION-MAP contradicts itself twice: its own «four parts per entry»
  law is broken by **C-7, which carries no side/resolution triple**, and C-7's
  «open» question on H6-uniformity is **answered by DR1-022 in the ATLAS the map
  indexes**. The README misses in three ways a consumer meets first — it says the
  package ships prompt content only while **five Rust crates ship inside it**, it
  points at `rust/GUIDE-…` where the guide is at `spec/rust/GUIDE-…`, and its
  mechanism roster omits `MCP-CORE-v0.1.md`, which ships.

  *On the instrument:* 424 refs resolved, zero unresolvable, and **188 came back
  ELIDED** — the outcome added this morning. That is my own brief's doing: it
  capped snippets at 120 characters and the worker obeyed by truncating. The
  outcome earned itself in one batch, naming a hundred and eighty-eight honest
  elisions without failing one of them.

- **2026-07-28 · C2 CLOSED — 485 verdicts at 92.4 %, and the drift is all in one
  place.** **448 confirmed / 34 drift / 3 unverifiable.** Four of the eleven files
  are **zero-drift**: the RAID Playbook, the Sweep Playbook, the Campaign Form and
  the WAL Convention, 148 facts between them and not one wrong.

  | file | conf | drift | unver | |
  |---|---:|---:|---:|---:|
  | 03-RAID-PLAYBOOK / 04-SWEEP-PLAYBOOK / 05-CAMPAIGN-FORM / 06-WAL-CONVENTION | 148 | 0 | 0 | 100.0 % |
  | appendix/ATLAS | 91 | 2 | 0 | 97.8 % |
  | boot/10-flow-core-ai-native | 15 | 1 | 0 | 93.8 % |
  | 00-MANIFESTO | 91 | 8 | 1 | 91.0 % |
  | README | 23 | 3 | 0 | 88.5 % |
  | 02-EXECUTABLE-SCAFFOLDS | 33 | 6 | 1 | 82.5 % |
  | appendix/CONTRADICTION-MAP | 21 | 4 | 1 | 80.8 % |
  | 01-PATTERN-CARD-FORMAT | 26 | 10 | 0 | 72.2 % |

  **The playbooks confirm wholesale because they are implemented and because this
  repository has run them.** `floor.rs` names the Sweep Playbook in its own module
  doc; `health.rs:2` cites `…04-SWEEP-PLAYBOOK#collector` by URI, and the
  collector's four contract lines — determinism, advisory, no-LLM, single-sourcing —
  are four separate places in that file. The WAL Convention's 24-hour freshness rule
  is a shipped checker (`vibe-check/src/checks/wal_freshness.rs`, threshold at
  `lib.rs:292`) and its canonical sections are enforced by `wal_wellformed.rs`. Both
  playbooks have **executed instances in this tree**: a closed raid whose §6
  decisions cite RAID §1.1/§1.2/§1.4/§1.5 by number, and `terraform/{BASELINE,LOG,
  REPORT}.md` + `adopt-v0.3/PREDICTIONS.md` as the Campaign Form's five artifacts,
  built.

  **The drift is one thing, said eleven ways: the Band-3 card-ops block has no
  reader.** All 27 cards (3 stacks × 9) author it; a search for `card-ops` /
  `Band-3` / `band_three` / `cards/INDEX` across every `.rs`, `.ts`, `.go`, `.py`
  and `.js` in crates, xtask, packages, spec and schemas finds **no extractor at
  all**. So the Manifesto's lazy-push harness, the weak-reader ~10-line cap, the
  trigger-to-card resolution, the machine-extractable operational layer and the
  «empty operational fields are a defect» rule are each authored on one side and
  unimplemented on the other. `01-PATTERN-CARD-FORMAT` reads lowest in the batch for
  exactly this reason.

  **Three smaller ones worth their own line.** No card carries a `prediction` field,
  so «every card carries a falsifiable prediction» is false of all 27. The sunset
  R-050 that is supposed to expire the in-distribution law is cited everywhere and
  scheduled nowhere. And the adoption plan does live outside the package, exactly as
  claimed — at `terraform/`, while the fact names a `vibevm-terraform` that does not
  exist.

  *One verdict was restated during review, and the reason matters more than the
  verdict.* Five `BUILD-ORDER-*` anchors came back `partial` and one, `BUILD-ORDER-I`,
  came back `located` — so the default would have confirmed the sixth step of a
  build order whose other five steps are drift, because a search happened to resolve
  its card rather than its ordering. **The claim class was one; the accident of the
  search was about to split it two ways.** Restated to drift on the siblings'
  reasoning: the numbered order appears nowhere outside this document, and the one
  raid this repository executed ordered its cards `D/H → C → F → G → traces`.

- **2026-07-28 · C7 CLOSED — 99 verdicts, zero drift, and F-116 turns out to be
  about a family rather than a fault.** The three `discipline-mcp-*` briefs judged
  entirely from machine evidence: the trio diff plus each server's own sources.

  **Every count in all three briefs is exactly right.** Rust claims eighteen tools
  and thirteen discipline adapters; its crate carries **18** distinct tool-name
  literals and eighteen less the five tcg ops is thirteen. TypeScript and Go each
  claim seventeen and twelve; each crate carries **17**. The gap between the
  projections is precisely `ledger_render`, which rust ships and both siblings say
  they do not — **three documents agreeing about a difference, which is the opposite
  of drift.** The parity map's eighteen rows map one-for-one onto the registered
  names, and the enumeration test the lead claims exists is real and says exactly
  what the lead says: `the_tool_set_is_exactly_the_declared_inventory` asserts
  `names == TOOL_NAMES` in stable order, so the list cannot drift from the code
  without a red test.

  **F-116's three normative items are all reproduced, and none of them is a false
  statement.** `REPORTS-CARRY-THE-RUNS-ENTIRE-STORY` and `parity-map-lead` are in
  rust and go and **absent from TypeScript**; the `force`-class clause inside
  `HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS` is **stated by rust alone**.
  An omission is not a wrong fact: every anchor that exists is true where it stands,
  which is why the batch reads 99/99 while the finding stands. **The finding is
  about the family, not about any of its members** — and that distinction is only
  available because the three were compared rather than read.

  *No worker was spawned for this batch.* The instrument built for C4 named the trio
  and answered C7 in a second; the remaining evidence was four greps and one test.
  A batch whose files are near-identical copies is cheaper to diff than to delegate.

- **2026-07-28 · F-122 — one `name@version` coordinate, two contents, 173 times.**
  `qualified-naming` states the law this breaks: *never reuse a `name@version`
  coordinate for different content — a coordinate that meant one artifact must never
  mean another.* Measured across both trees: **425 markdown files sit at the same
  (package, version) pair in `packages/` and `vibedeps/`; 252 are byte-identical and
  173 are not, across 33 packages.**

  The cause is this campaign. Phase B marked 201 package files **inside already
  published version slots** — `core-ai-native@0.8.0`'s installed copy carries **zero**
  fact anchors against fourteen in the package of the same version — so every marked
  package now ships different bytes under a coordinate a consumer already resolved.
  Most of the difference is markup the boot compiler strips anyway, and **not all of
  it is**: the boot-lane join found `campaign-plans` differing by six real words and
  `comparative-research` by three.

  Filed rather than fixed, per the phase's own rule. Closing it is a **release
  event** — §5-D's «a finding that spans a package boundary is closed by a published
  version, not by an edit» — and this one spans thirty-three. It belongs beside F-114
  as an owner decision about versioning rather than an edit anybody can make.

  *The first measurement of this was wrong and looked exactly like a finding.* It
  globbed `packages/*/*/v<ver>/<tail>` and matched every package sitting at the same
  version number — `go-ai-native@0.1.0` against `addressable-specs@0.1.0` — producing
  a count of 177 that meant nothing. Matching on package identity gives 173. **Two
  numbers three apart, one of which was arithmetic over a category error.**

- **2026-07-28 · F-123 — the host breaks the subject-length rule of a flow it
  installs, and this campaign is the largest contributor.** §3.1's source 2 for the
  git family is this repository's own history, and it is the cheapest independent
  evidence in `world`: no document is asked whether another document is right.

  **What conforms:** 394 of 400 subjects carry the `type(scope):` header, 399 of 400
  carry a body — which is where the flow puts the *why* — and there are **zero
  `Co-Authored-By` trailers and one author** across four hundred commits.

  **What does not:** `conventional-commits` sets a **hard limit of 72 characters**
  on the subject, and **82 of 400 exceed it — 20.5 %, the longest at 89.** The
  violation is not a slip: 28 on 2026-07-25, 27 on 07-26, 14 on 07-24, **6 written
  by this phase today**. The campaign that exists to measure whether the discipline
  holds itself to its own rule has been breaking one of them at about a fifth of its
  commits.

  **F-087 is measured rather than reported.** Four commit bodies name a model — two
  use `Anthropic` as the name of a colour theme, two describe model tiers as
  configuration data — so the policy's «never mention model names in commit
  messages» is broken four times in four hundred, and its «never state or imply
  machine authorship» is **not broken at all**. The finding is real and smaller than
  its filing suggests, and the distinction between naming a model and claiming it
  wrote something is the whole of it.

- **2026-07-28 · C3a — the TypeScript guide at 93.5 %, and the perimeter was
  wrong a fourth time in the most expensive way yet.** 153 verdicts: **143 confirmed
  / 9 drift / 1 unverifiable.**

  **The guide's consumer is `research/ts-demo/`, and no brief had named it.** A
  language guide's rules are addressed to a project written in that language, and
  the stack package is a Rust workspace that *analyses* TypeScript — so its own lack
  of a `tsconfig.json` is not the discipline's absence, it is the wrong place to
  look. The demo carries the entire surface: `conform.toml`, `specmap.toml`,
  `specmap.json`, the three `discipline/registry/` files, and a `tsconfig.json`
  setting **all four** mandatory beyond-strict flags. The tooling the guide names is
  there at scale — **vitest in 46 files, `tsd` 43, fast-check 36, Twoslash 35,
  `expectTypeOf` 30, ts-morph 25, `assertNever` 21** — every one of which the
  delegated search had correctly reported absent from the places it was given.
  **Searched without the demos, about forty facts read as unbuilt; searched with
  them, they read as in force.** The batch plan's §4.5 now names them.

  **What is genuinely drift, and it is precise.** The TypeScript gate registers
  exactly three rules — `TsUnsafeInDomain`, `TsCellIsolation`, `FileLength`
  (`build_rules`, lib.rs:48-61) — so the whole §7 flag family cites an R-001 that
  **is present in the vendored engine and never mounted on this language**. The
  build-time tier (bundler `define` / DCE) and the runtime flag registry have no
  instance anywhere, including the demo. The five defect-catcher flags are marked
  implemented and the discipline's own demonstration project sets **none** of them.

  **And a number quoted from a superseded copy of the corpus's own appendix.** The
  guide cites TCD at «~74.8 %» twice; that figure appears in the live tree **only
  under `.vibe/cache/**`, in an older CONTRADICTION-MAP**, while this family's own
  ATLAS records **75.3 % synthesis / 70.2 % translation**. One cross-reference is
  also off by two sections: the agentic battery is cited as «the sibling brief §6»
  and §6 of that brief is «The honest risk register».

  *On the worker:* it returned 153 rows having **repaired 59 of its own line numbers
  before returning** and re-checked every one of 348 refs. Its three `not-found`
  rows were all correct about the perimeter it was given, and two of the three
  became drift on wider search while the third stayed. **A worker that reports what
  it searched converts a reviewer's disagreement into one lookup.**

- **2026-07-28 · C3 CLOSED — 390 verdicts, and the three guides measure
  differently for a reason that is about the tree rather than about the guides.**
  **350 confirmed / 24 drift / 16 unverifiable — 89.7 %.**

  | guide | conf | drift | unver | |
  |---|---:|---:|---:|---:|
  | TypeScript | 143 | 9 | 1 | 93.5 % |
  | Rust | 79 | 9 | 0 | 89.8 % |
  | Go | 128 | 6 | 15 | 85.9 % |

  **Rust reads high because the host dogfoods it** — 120 files carrying
  `#[spec(implements=…)]`, 402 carrying `specmark::scope!`, a committed 3.1 MB
  specmap and a floor whose three portable steps are green. Its nine drifts are named
  sub-mechanisms with no checker and no instance **on the wider perimeter as well**:
  `#[track_caller]` zero repo-wide, sealed traits and `PhantomData` zero, custom
  clippy lints zero, `#[spec(documents)]` edges zero against 120 `implements` ones,
  and two rules — **R-021 and R-060** — cited by name whose cards are unauthored.
  R-021's absence is now recorded by three documents in this phase.

  **The perimeter check ran the other way here and that is worth recording.**
  Re-searching every Rust gap against `research/rust-demo/` — the same widening that
  inverted forty TypeScript facts — found **none of them**. Widening the perimeter is
  not a way of confirming things; it either finds the artifact or it does not, and
  here it did not.

  **Go's fifteen unverifiable are the honest measurement, not a shortfall.** There is
  no consumer to observe: the only production Go in the repository is
  `tools/go-extract/extract.go`, everything else under `.go` is that tool's fixtures
  or its own generated output, there is **no `research/go-demo`** beside the rust and
  ts demos, and the stack is **not installed at all** — `vibedeps/` carries no
  `stack-go-ai-native-lang`. A discipline addressed to a consuming Go project has
  nothing here to hold or break it. Its ban list still confirms, on absence over that
  one file plus a registered `GoUnsafeInDomain` rule; its drift is where a named tool
  is missing — `-race` absent from a test step that fails anyway, and the
  `exhaustive` linter **not installed**, its floor step failing rather than skipping.

  **Both non-Rust gates register exactly three rules.** TypeScript mounts
  `TsUnsafeInDomain`, `TsCellIsolation`, `FileLength`; Go mounts the same three in its
  own projection. The `FlagSites` rule exists in the vendored engine and is mounted on
  neither, which is why both guides' flag families cite an R-001 that cannot fire.

- **2026-07-28 · C6 CLOSED at 92.7 %, and it caught two errors of mine in the
  batch before it.** 330 verdicts over the six skills, three boot snippets and eight
  READMEs: **306 confirmed / 24 drift / 0 unverifiable**, on the cleanest evidence
  table of the phase — **865 refs, every one OK, zero elided and zero unresolvable.**

  **The drift is almost all one shape: a path or a name that does not resolve.**
  The three boot snippets cite `rust/…`, `typescript/…`, `go/…` and `cards/INDEX.md`
  where every one lives under `spec/` — the third instance of that family after the
  Manifesto's `MAP-RUST-GUIDE` and the core README's `READ-STACK-GUIDE`. The Rust
  README ships `crates/specmark` (it is `crates/vendor/core-ai-native-specmark`) and
  `schemas/specmap.jtd.json` (host-only). The go-mcp README cites a plan that lives
  in the host's `legacy-spec/`. And **four names in one fact**: the Go sweep's census
  says `gated_packages` where the field is `gated_crates`, and `init_in_cell` /
  `ambient_call_in_cell` / `naked_go_in_cell` where the shipped kinds are
  `init_decl` / `ambient_call` / `naked_go`.

  Two further drifts are about output that is never printed: all three sweeps tell a
  reader to check for `Defaulted` and `DISABLED by policy`, and **neither string
  exists in any shipped source or captured run**. And the TypeScript sweep's outcome
  table cites «Playbook §5», which is «What the sweep deliberately does NOT do» — the
  output section is §4 and the table is in neither.

  **Four of the six skills are installed here** — `.claude/skills/` carries the rust
  and typescript sweep/terraform pairs — and the Go pair is not, consistently with
  `vibe.lock` carrying no `go-ai-native` and `vibedeps/` no `stack-go-ai-native-lang`.

- **2026-07-28 · Two corrections to C3, both mine, both found by the next batch.**

  **First: I confirmed ten TypeScript facts on a count of `node_modules`.** C3a
  confirmed the scaffold-tooling family — vitest, fast-check, `expectTypeOf`, `tsd`,
  Twoslash, ts-morph, `assertNever` — citing «vitest in 46 files» and the rest in
  `research/ts-demo/`. That grep did not exclude `node_modules`, `.vibe/cache/` or
  `vibedeps/`, so **it counted the demo's dependencies and a cache of superseded
  package versions as the demo's own practice.** Over the demo's own sources every
  one of the eight returns **zero**; its devDependencies are `@types/node`, `eslint`,
  `prettier`, `typescript`, `typescript-eslint`, its test script is literally
  `node --test`, and its two test files are `src/cells/{farewell,greeting}/index.test.ts`.
  Ten verdicts restated confirmed → **drift**. The tsconfig half of that same evidence
  stands, because it was read by parsing the file rather than grepping it.

  **Second: I recorded fifteen Go facts unverifiable on an absence I asserted and
  never checked.** The reason given was «there is no `research/go-demo`». **There
  is**, and it is a complete consumer: `go.mod`, 15 production files in the cell
  layout the guide prescribes, `conform.toml` with `cells_dir = "internal/cells"`,
  `specmap.json`, a conform baseline and four `discipline/` files. Measured over it:
  `func init(` 0, blank imports 0, `go func` 0, `context.Context` 8, `func Example`
  **4**, `Fuzz` 5, `//spec:` 18, one `recover()` and it is in a test. Twelve of the
  fifteen become **confirmed**, one becomes drift (`DisallowUnknownFields` occurs zero
  times against a MUST), and two stay unverifiable for the right reason — the consumer
  starts no goroutines and generates no code, so two rules have nothing to govern.

  **What found both:** C6's brief named `.claude/skills/` and the go README's own
  `WORKED-PILOT-IS-RESEARCH-GO-DEMO` fact, which pointed at the directory I had
  declared absent. **The corpus corrected the reviewer**, twice, through a batch that
  was looking at something else.

- **2026-07-28 · C4+C5, two languages of three — 696 verdicts, and TypeScript
  reads highest in the phase.** TypeScript **333 confirmed / 4 drift — 98.8 %**; Go
  **337 / 8 drift / 14 unverifiable — 93.9 %**.

  **TypeScript's four drifts are each a name that outlived its referent.** The
  scaffold-d sunset names `vibe-tcg-ts`, which survives only in `.vibe/cache/` copies
  of the superseded v0.2.0 and v0.3.0 slots — the shipped binary is
  `typescript-ai-native-tcg`. `vibe codemod rename-seam` has no implementation
  anywhere. The oracle's warm target cannot be measured by its own harness: `bench.rs`
  records `cold_init_ms`, `validate_p50_ms` and `validate_p95_ms` and **no complete
  latency and no thresholds**. And the product seam names the `vibe-tcg` crate, which
  **the corpus itself records as deleted** at PROP-026:11.

  **Go's fourteen unverifiable are a distinction worth keeping.** The oracle *exists*
  — `go-ai-native-tcg.exe` is built, its help lists serve / validate / scope /
  complete / type / bench, and its sources carry nine `gopls` references — and
  **gopls is not installed on this machine**, so no behavioural claim about the relay
  can be observed. Recording those confirmed on the code's existence would report an
  unexercised mechanism as a working one; recording them drift would blame a
  toolchain absence on the document. The mechanism is present and untested here, and
  that is what the verdict says.

  Its eight drifts are concrete: `go:generate` occurs **zero** times in the Go
  consumer, no goldens and no `-update` flag exist, `research/tcg-bench/` carries a
  TypeScript corpus and a Rust one and **no Go corpus**, the bench records `warm_ms`
  alone against a «complete p50 under 300 ms» target, no grammar tooling is vendored,
  and the gated-or-exempt invariant is stated over **`gated_packages`** where the
  shipped field is `gated_crates` — the same wrong name C6 found in the Go sweep's
  census, now in a second document.

  *On the instruments:* the TypeScript table returned 606 refs with **zero**
  unresolvable; the Go table 873 with **ten**, which is the checker earning its place
  at 1.1 % rather than crying wolf.

- **2026-07-28 · THE `ai-native` CLUSTER IS CLOSED — 80 of 80 files, 2 697
  verdicts.** C4 and C5's Rust half came in at **344 of 344 confirmed, 100 %**, and
  that number was checked rather than celebrated: its nine `not-found` rows split
  cleanly by marker. Three are `@impl/done` **dated spike measurements** — init
  handshake ~10 ms, hover ~1 ms, completion ~19 ms at 118 entries — which wave 1's
  stage semantics confirm unless falsified, and the oracle they measured **is
  runnable here**: the binary is built and rust-analyzer is installed, unlike the Go
  stack's gopls. The other six are `@spec/done` and sit under «§4 Staged ambition»
  and «§5 Licensing posture» — a CFG grammar mask deliberately unbuilt, whose absence
  is what those facts predict.

  **Rust reads 100 %, TypeScript 98.8 %, Go 93.9 % on the same nine scaffolds and the
  same oracle shape, and the ordering is about this repository rather than about the
  three documents.** The host is a Rust project that dogfoods the Rust stack; the
  TypeScript consumer exists and is complete; the Go consumer exists and its
  toolchain does not.

- **2026-07-28 · THE REVIEWING DEBT IS CLOSED — 138 rows read, and the standard
  that judges them is finally written down.** Sixty `partial` rows in the Go table
  had been sorted by FILE and seventy-eight in the Rust table by one two-branch
  rule. Read individually they come to **101 confirmed / 36 drift / 3 unverifiable**,
  against 138 confirmed and unverifiable before. The cluster moves **92.4 % →
  91.6 %**: 2 470 confirmed / 207 drift / 20 unverifiable.

  **The standard, stated because it had not been.** A fact that PRESCRIBES what the
  discipline requires — an intent, a participants list, a detector seed, a goal, a
  tradeoff, an alternative, a risk, a routine step — is confirmed when it is
  coherent and every referent it names resolves, **including a referent the package
  itself declares as future work**: a card registry's `specified` column and a
  brief's «vision, NOT an implementation plan» status line are that declaration. A
  fact that DESCRIBES what this repository already ships is checked against the
  tree, and a description that does not match is drift. *This was already the
  phase's operative standard — it produced 690 confirmations across the Rust and
  TypeScript halves of C4+C5. Naming it is what made the Go half comparable to
  them, and what exposed four claims judged two ways.*

  **Where the drift is: documents describing shipped software.** A codemod
  documented with five parameters that takes two and writes three files; `init`
  results missing three of their five keys in both the Go and Rust protocols; a
  floor glossed as four steps that ships seven and has no `build` step;
  `language: "go"` said to dispatch through a host that names `"go"` as its example
  of an unsupported value; an overlay counter said never to reset that resets on
  `didClose`, in a bridge whose own test is named
  `overlay_versions_are_monotonic_and_close_resets`; stderr said to be drained by a
  reader when it is dropped at the pipe; a no-zombie property said to be
  test-asserted in two stacks, asserted in neither; replay goldens claimed for both
  hops and present for one; and three latency targets whose harness never times the
  operation they govern.

  **Eleven of the fourteen `TCG-ORACLE-GO` rows left `unverifiable` for the right
  reason.** They had been called unverifiable by file, on «gopls is not installed».
  Eight of them turn out to be structural — a missing bench report, a resolution
  order, a returned key set, a test that does not assert what a document says it
  asserts — and are readable with no gopls at all. Three genuinely need a live
  server and say so in their own words.

  **Four claims had been judged two ways across languages, and comparing twins is
  what found them.** `vibe codemod rename-seam` (drift in TypeScript, confirmed in
  Rust); the product seam's lockfile dispatch (drift in TypeScript, confirmed in
  Rust and Go); the `complete` latency target (drift in TypeScript, confirmed in
  Rust); and the Layer-1 grammar mask (drift in Go, confirmed in Rust). Each is now
  judged once, and two of the corrections run **upward**: `go-ai-native-tcg.md`'s
  two rows return to confirmed, because a document that declares itself «held at
  stub depth» and VERY-FAR-FUTURE is not contradicted by being unbuilt. Two Go card
  rows return to confirmed for the same reason — a participants list and a
  goldens tradeoff were read as descriptions of this tree when they are
  prescriptions, while five sibling participant lists were confirmed.

  **The one thing the bulk pass got structurally right is worth saying too.** No
  row moved because the worker's evidence was wrong. Every drift found here was
  already visible in the `searched` field the worker returned; what was missing was
  someone reading it. *A delegated table that records what it searched converts a
  reviewer's disagreement into one lookup — and converts a reviewer's absence into
  138 unexamined rows.*

- **2026-07-28 · F-124 — three evidence ids that resolve nowhere, cited by all
  three stacks.** The pattern cards close with an «Evidence & Transfer-strength»
  line naming the findings a card rests on, and three of those names are not in any
  register this repository carries:

  | id | cited by | what exists instead |
  |---|---|---|
  | **`H4`** | `scaffold-g-doctests.md` in Go and Rust, `GUIDE-AI-NATIVE-GO.md` ×2, `GUIDE-AI-NATIVE-RUST.md` ×2, both `cards/INDEX.md` | nothing — the ATLAS's 87 records are `BLD-` / `DR1-` / `DR2-` / `R2C-` / `R3-` |
  | **`DR1-014`** | `rust-ai-native-tcg.md`, `GUIDE-AI-NATIVE-TYPESCRIPT.md` | `DR1-013` and `DR1-015` exist; `DR1-014` has no anchor |
  | **`DL1-015`** | `scaffold-i-codemods.md` in Rust and TypeScript | `DR1-015` — one letter away, and the typo has never been resolved by anything |

  **The `H`-series is the interesting one, because it is not a typo.** `H1`, `H5`
  and `H6` are used the same way inside `core-ai-native`'s own appendices —
  `CONTRADICTION-MAP.md:13-16` frames C-1 as «H1 vs H5» and `:46` names
  «H6 uniformity», and `ATLAS.md` carries `refines:H2`, `refines:H3`, `H5/H6` in its
  record bodies. **A hypothesis vocabulary is in daily use across the family and its
  roster is written down nowhere.** So `H4` cannot be confirmed, cannot be
  corrected, and cannot even be shown to be wrong — the same shape as F-120's
  kind-line notation, defined by one example and cited to a document that is not
  here. Filed, not fixed: which of the two — publish the roster, or drop the H
  citations — is the owner's call, and it touches published slots.

- **2026-07-28 · F-125 — one package, two appendices, two numbers for one
  measurement.** `core-ai-native` v0.8.0 publishes the PLDI'25 type-constrained-
  decoding result twice and differently: the **ATLAS** records «reduces compilation
  errors by **75.3 % (synthesis) and 70.2 % (translation)**» (`ATLAS.md:105-106`,
  DR2-012), while the **CONTRADICTION-MAP** titles C-4 «Type-constrained decoding
  cuts errors **74.8 %**» and repeats it in the resolution
  (`CONTRADICTION-MAP.md:28,31`). Both are in the live slot, both are generated
  appendices of the same package, and **four documents across three stacks quote
  whichever one they read** — the TypeScript guide twice, the Rust token-level brief
  twice, the Go token-level brief once.

  *This one corrected a verdict of mine from earlier in the phase, and the
  correction is the finding.* C3a recorded the TypeScript guide's «~74.8 %» as
  drift on the reason «that figure appears in the live tree **only** under
  `.vibe/cache/**`, in an older CONTRADICTION-MAP». It does not — it is in the
  v0.8.0 appendix, inside this campaign's own corpus, one `grep` away. The verdict
  stands, restated on the true reason; **a wrong reason in the record is worse than
  a wrong verdict, because the next reader acts on it and the verdict at least
  pointed at a real defect.** The perimeter law was written after three misreadings
  of where to look; this is the fourth, and the first where the miss made a real
  contradiction look like a stale quote.

- **2026-07-28 · F-126 — `rust-ai-native-tcg` names two different tools, one of
  them shipped.** `spec/rust/tools/rust-ai-native-tcg.md` is the token-level brief:
  a logit-masking constrained generator, VERY-FAR-FUTURE, explicitly unbuilt, whose
  one-line summary promises well-typed Rust «by construction». A **binary of exactly
  that name ships**: `vibe.toml:45` declares `rust-ai-native-tcg` and
  `crates/rust-ai-native-tcg/src/main.rs:1` calls itself «the agentic type oracle's
  CLI face», with a serve / validate / scope / complete / type / bench surface and
  no masking anywhere. A reader resolving the name gets the consultation oracle and
  the brief for something else.

  `qualified-naming` states the law this brushes: *never reuse a coordinate for
  different content — a coordinate that meant one artifact must never mean another.*
  The same collision already has a recorded instance one document over — the
  TypeScript card's sunset names `vibe-tcg-ts` where the shipped binary is
  `typescript-ai-native-tcg` — and a third: `vibe-tcg`, the name the Rust brief
  reserves «solely for vibevm's language-generic product crate», belongs to a crate
  PROP-026 records as deleted whole. **Three names in one tool family, none of them
  pointing where its document says.**

- **2026-07-28 · F-127 — the Go stack prescribes `-race` fifteen times and never
  passes it.** `go test -race` is the Go projection's whole concurrency-discipline
  story: the boot snippet, the card registry, `scaffold-d`'s routine step 5,
  `scaffold-e`'s intent, participants and consequences, and the guide's baseline all
  name it — **15 mentions across 5 documents**. The shipped `go-ai-native fast-loop`
  runs `go test ./<cells_dir>/<cell>/... -json` and passes no `-race`
  (`crates/go-ai-native-cli/src/fast_loop.rs:87`); the floor's test step does not
  either; `-race` occurs **zero** times in the stack's Rust sources and zero times
  in `research/go-demo`. So `scaffold-e`'s «`-race` rides along at per-package cost,
  so the concurrency discipline (§5) is checked in the same loop» is true of the
  command a human types and false of the tool the card names as its own checker.
  The guide's `BASELINE-RACE-DETECTOR-GATES-TESTS` is already drift for the floor
  half; this is the loop half, and it is one flag.

- **2026-07-28 · F-128 — the four non-negotiable commit rules are said to live in a
  file that does not exist, by the file every session reads first.** Found while
  reading W1's packages rather than in any delegated table, and the chain resolves
  end to end:

  ```console
  $ ls spec/boot/
  00-core.md  90-user.md  INDEX.md  STATIC.md
  ```

  **There is no `spec/boot/INLINE.md`.** And line 5 of `CLAUDE.md`, `AGENTS.md` and
  `GEMINI.md` — identical, and the first substantive sentence of the first file any
  session reads — says the commit-and-push discipline «is the `git-practices`
  family, a dependency of this project **loaded first and verbatim from
  `spec/boot/INLINE.md`**. The rules live in that inline lane, **not restated
  here**.»

  **Why the lane is empty is the interesting half.** PROP-009 gives a package's
  `[boot_snippet]` a `link` field with three values — `inline` / `static` /
  `dynamic` — and only `inline` produces `INLINE.md`. All four git flows carry, in
  their manifests, this exact pair:

  ```toml
  # A non-negotiable commit rule — suggest the inline priority lane (PROP-009 §2.4).
  link = "static"
  ```

  **The comment says inline and the field says static, in all four.** The umbrella's
  own manifest then builds on the comment rather than the field: «Each member
  self-suggests the `inline` priority lane in its own `[boot_snippet]`, so its text
  lands verbatim in `spec/boot/INLINE.md` (read first)». And repository-wide,
  `grep 'link = "inline"'` over every `vibe.toml` in `packages/` and `vibedeps/`
  returns **zero** — nothing anywhere asks for the lane, so the generator has
  nothing to write and correctly writes nothing.

  **The rules are read anyway, which is why this survived.** They are compiled into
  `spec/boot/STATIC.md` — twice each, once directly and once through the umbrella
  (F-078) — and `INDEX.md` names `STATIC.md` as the static lane, so a session that
  follows the *generated* boot manifest gets all four rules. A session that follows
  the *authored* sentence goes looking for a file that is not there and is told the
  rules are not restated anywhere else. **The mechanism works; its documentation is
  wrong in five manifests and three contract files.**

  Filed, not fixed. Three of the eight surfaces are inside published package slots,
  so closing it touches F-122's territory; the host-side half — three identical
  lines in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` — is one edit and the owner's,
  because those files are the ones the boot contract is written on.

- **2026-07-28 · W1 CLOSED — 407 of 407, and the phase's falsifiable prediction is
  in trouble at the first world batch.** The git family: **368 confirmed / 32 drift /
  7 unverifiable — 90.4 %**, against `ai-native`'s 91.6 %. §5-C predicted that
  **`world` measures higher**, on the reasoning that prose contracts are rarely
  contradicted while mechanical claims can be wrong in ways prose cannot. The first
  world batch reads *lower*, and the reason is the opposite of the one predicted:
  these flows make claims about the consuming project, and this consumer is
  measurable.

  | file set | verdicts | conf | drift | unver | |
  |---|---:|---:|---:|---:|---:|
  | `git-attribution-policy` (5 files) | 132 | 109 | 16 | 7 | 82.6 % |
  | `git-conventional-commits` (3) | 68 | 62 | 6 | 0 | 91.2 % |
  | `git-atomic-commits` — core (3) | 101 | 93 | 8 | 0 | 92.1 % |
  | `git-autonomy` + `git-practices` (4) | 54 | 52 | 2 | 0 | 96.3 % |
  | `git-atomic-commits` — splitting (1) | 52 | 52 | 0 | 0 | 100 % |

  **26 verdicts are self-referential — 6.4 %**, and amendment A2's counter fired for
  the first time. They cluster where the corpus argues rather than states: claims
  about pure-human teams, about what most humans do under deadline pressure, about
  what an unmanaged repository accumulates. `tasks/summary.py --batch W1` prints it.

  **Thirteen of the thirty-two drifts are one law, and it is the flow's own.**
  `attribution-policy` states that the policy lives in exactly one always-loaded
  place and nowhere else; the host compiles it into the boot lane twice and restates
  it in eight further locations, and a repo-wide grep for the topic returns **88
  lines across 50 files**. Every clause resting on that law falls with it — one
  place to change, switching in one edit, and the escape hatch that says a project
  adopts the alternative «by editing this snippet», whose only host copy sits inside
  a file whose first line says it is generated and must not be edited.

  **A second family runs through every package in the batch: the sibling pointer.**
  A boot snippet ends by linking its own protocol as `../flows/<name>/<file>.md`.
  Measured corpus-wide: **60 such pointers in the packages, 0 dangling; 120 in the
  installed `vibedeps/` trees, 15 dangling; and 69 in the host's compiled boot lane,
  all 69 dangling** — because the host has no `spec/flows/` directory at all and
  `files_written = []` means no package will ever create one. The pointer resolves
  for exactly one reader, the person browsing the package repository, and fails for
  the reader it is written for.

  **The rest are single defects worth naming.** «Explicit and enforced» is explicit
  and unenforced — no hook, no CI, no audit line — in a flow that itself ships the
  law «a policy with no checker is a wish». The never-item forbidding model names in
  commits and comments is broken on the letter at scale (11 subjects, 354 message
  lines, 263 code lines) while its sibling forbidding authorship claims is not
  broken at all. The format flow's subject rules are broken as practice: **297 of
  400 subjects exceed the 60 characters it targets — 74.3 %** — and 144 of 400 open
  with an article rather than a verb, which is the first measurement of its
  imperative-mood clause and the largest single format gap found. And
  `atomic-commits` contradicts itself about an irreversible operation: its
  frozen-history bullets forbid `--amend` and `rebase -i` flat where its own
  snippet, its own summary and the host's rule 4 all permit them with explicit
  approval.

  **Seven unverifiable are honest and all of one kind** — assertions about
  jurisdictions, regulation and employer policy that no source inside this
  repository can settle, plus one uninstall behaviour whose file ledger is empty
  for all 36 packages.

  *On the instruments.* Five delegated tables, 1 645 refs, and after the checker's
  fourth narrowing **0 unresolvable**. Forty-three refs failed at first and not one
  was a fiction — every one was a notation the grammar does not admit (a bare path,
  a line range recording an absence, an added code span, a double-escaped
  backslash), and the brief now carries the grammar. Two workers reported that a
  harvest file **changed under them mid-run** and that they had re-anchored every
  citation to its current content; the boss had edited it while five workers were
  citing it. That is the boss's defect, reported by the workers rather than by the
  checker, and it bought `repair-refs.py` — which separates a real quote whose line
  moved from a quote that is nowhere, and refuses to choose when a quote occurs in
  several places.

- **2026-07-28 · W2's evidence is complete and unjudged — 692 anchors, four tables,
  2 404 refs, 3 unresolvable and none a fiction.** The session that closed the
  reviewing debt and W1 ran out of context here, and the honest stop was to persist
  the delegated work rather than judge half of it — the same shortcut that created
  the 138-row debt this phase opened with.

  **What the four tables establish before a single verdict is written:**

  | | |
  |---|---|
  | the sync grammar | `docs(spec): sync <section> with code` — **0 uses in 2 041 commits** |
  | …and yet the path is the practice | recorded three independent ways: a task template, an owner guide, and a 212-row wave whose diffs were drafted, surfaced and applied only on approval |
  | of three mandatory draft parts | value and reason land; the **revisit trigger never does** |
  | the morning ritual | **none exists in the host at all** — 23 hits for the word, not one an operating instruction |
  | the `_Updated:` line | a bare date where the protocol requires ISO 8601 UTC, so the flow's own 24-hour test cannot be evaluated to the hour and its skill has no hours to report |
  | the wind-down report | flow asks four items, host asks four items, **none of them the same** |
  | the cold-resume contents | **10 of 10 match** |
  | the specspaces snippet | placed by two host files at «slot 11 of `spec/boot/INDEX.md`»; `grep -c` on that file returns **0** — the same shape as F-128 |
  | the installed payloads | stale by 92-176 changed lines per file; 10 and 0 fact anchors against the package's 39 |

  **And three counter-instances that are this repository's own, one of them written
  by the session that found it.** `6a026de1` rewrote the WAL's In-progress and Next
  in a single hunk and touched `_Updated:` **zero times**, which is precisely what
  `NEVER-LEAVE-THE-UPDATED-LINE-UNTOUCHED` forbids. Over 2026-06-01 → now, 37 days
  carry commits and 28 carry a WAL commit — nine active days ended without one. And
  the read order is reversed *and* self-contradictory: `CLAUDE.md:205` reads the WAL
  then `CONTINUE.md`, while `CONTINUE.md`'s own resume prompt lists itself first, «in
  this order».

  **The WORDS-DIFFER finding was corrected in place, and the correction is the
  entry's other lesson.** The boot-lane join reported `two-process-model` as the
  corpus's first words-differ case and the harvest first recorded three missing
  *rule* words. They are the `{#…}` suffixes on three headings; every rule's prose is
  word-identical, and what the host loses is the ability to cite three of the four
  sections of its own copy — which bites the rule that prices a correction at twenty
  tokens *because* the exact section can be cited. Cause traced with dates: the
  anchors landed 2026-07-27, the slot was written 2026-07-14, the lane regenerated
  2026-07-26. The compiler-strips-anchors alternative was checked and rejected.
  **Third time in one session that a reason in the record needed fixing rather than a
  verdict.**

  *On delegation.* Nine `opus5` workers across W1 and W2, 4 049 refs, and after the
  checker's fourth narrowing **five unresolvable, none a fiction**. **Three of the
  four W2 workers reported the boss moving a file under them mid-run** — the harvest,
  `CONTINUE.md`, and `spec/WAL.md` twice — and each re-anchored its citations and
  said so. The rule is now in both checkpoints: do not edit a file a running worker
  cites, and do not read a table while its worker may still be writing.

- **2026-07-28 · The wind-down that invalidated its own evidence.** W2's four
  tables were verified clean at 3 unresolvable when the last session sealed them.
  Re-verified at the opening of this one they read **65**. Not one of the 62 new
  ones is a fiction: `CONTINUE.md` was overwritten wholesale by `8406eb2a` and
  `spec/WAL.md`'s `_Updated:` line rewritten by `0f2991d1` — both *after* the
  tables were returned and committed — and `git show 100617b3:spec/WAL.md | sed
  -n 3p` still carries the quote verbatim. `repair-refs.py` re-pointed 51 by
  single-hit search; the remaining 14 are named rather than repaired (8 quote
  text the rewrite deleted, 2 became ambiguous, 3 are the grammar cases already
  recorded). **This is the third instance of the boss moving a file under a
  citing worker and the first where the workers had already finished, so nobody
  could re-anchor.** W1's lesson — do not edit a file a running worker cites —
  does not cover a wind-down that rewrites the two files the whole batch is
  about. The tool's first real `--apply` also exposed a defect of its own: it
  re-dumped every table at a fixed indent, turning 51 repaired coordinates into
  4 481 changed lines. Fixed to measure the file's own indent first; the same run
  now produces 53.

- **2026-07-28 · W2a and W2b CLOSED — `flow:wal` complete at 260 of 260, 86.5 %.**
  225 confirmed / 27 drift / 8 unverifiable across seven files. **W2a is 81.1 %
  (90/16/5), the lowest file-set in the cluster so far, and the reason is
  structural: this flow's facts describe `spec/WAL.md`, and `spec/WAL.md` is on
  disk and measurable line by line.** Where W1's git flows could only be checked
  against a commit log, W2a's required-sections contract can be checked against
  the artefact it specifies.

  **Six of W2a's sixteen drifts are that contract, each measured over the
  fourteen most recent revisions of the host's WAL rather than over today's:**

  | rule | measurement |
  |---|---|
  | `_Updated: <ISO 8601 UTC>` "always and without exception" | bare calendar date in **14 of 14** revisions, 0 timestamps |
  | Current phase — "one or two lines" | **25-50 lines** in every revision; 50 today |
  | Next — "the single next action" | **4-5 numbered items** in every revision, no default marked, numbering repeats 3 |
  | Constraints — each why "citing a spec anchor or issue" | **4 of 26 entries (15 %)**; `spec://` occurs **0 times** in the file in all 8 revisions measured |
  | In progress — "cite spec anchors (`spec://…`)" | same zero |
  | Session context — "one-paragraph orientation" | **41-65 lines** of retrospective |

  The size target is the seventh: 18 972 bytes is over 3 000 tokens on either
  conversion, though under the 5 000 hard limit. W1's precedent settles the shape
  — `HEADER-TARGET-LENGTH-AND-HARD-LIMIT` was judged drift on exactly this kind
  of measurement, and its summary restatement with it.

  **Two more are the flow's own §what, written for a one-file model that §files
  replaced two sections earlier in the same document.** The WAL is neither "the
  only persistent memory" — `CLAUDE.md`, 1 585 compiled boot lines, the spec tree
  and `CONTINUE.md` are all persistent memory the agent reads every session, and
  `CONTINUE.md` is specified by this very package — nor "read first": the host
  reads it third and says so in two independent files. **The instruction to read
  it first is itself compiled at line 1382 of a 1585-line lane the host reads
  before opening the WAL.**

  **And two are canonicity, with dates.** `wal` 0.2.0 landed 2026-07-07
  (`ec6b5b5a`, subject «the canonical WAL convention»). `core-ai-native` v0.8.0 —
  the Discipline's next release after that — landed 2026-07-17 (`bfb72da7`), ten
  days later, is the installed slot, and still ships a complete
  `06-WAL-CONVENTION.md` with the same two files, the same canonicity rule and
  the same supersession, marked «OPTIONAL but preferred», containing zero
  occurrences of `defer`, `flow:wal` or `org.vibevm.world/wal`. **The next
  release came; it did not defer.** The host installs and boots both.

- **2026-07-28 · F-129 — the wal package ships two wind-downs and they
  contradict each other.** `session-end-hook.md` orders «the full hook, steps
  1-6»: confirm the stopping state, rewrite the WAL, collapse, overwrite
  `CONTINUE.md`, propose the commit, report. `cold-resume.md` §wind-down orders
  **five** steps in a different sequence — overwrite `CONTINUE.md` **first**,
  rewrite the WAL second, commit, push, chat TL;DR — with **no stopping-state
  step and no collapse step**. A third fact, `WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK`,
  asserts the two are the same procedure. All three are `@impl/done`, all three
  are in one package, and both step lists reach the consumer.

  **The host implements `cold-resume.md`'s five exactly, in order** — including
  the chat TL;DR word for word — so what reads as a consumer dropping two steps
  is a consumer obeying the other half of the package. This also corrects the
  harvest, which recorded the wind-down report as «flow asks four items, host
  asks four items, none of them the same»: the host's four **are** this flow's
  own `EXTEND-THE-REPORT-INTO-A-WIND-DOWN-TLDR` list, verbatim. What the host
  lacks is the *base* report, and two of its four items land elsewhere — REVIEW
  markers at `spec/boot/00-core.md:60`, discovered issues in the WAL's Known
  issues via the host's own step 2. Filed, not fixed; closing it is an edit
  inside a published slot, so F-122's territory.

- **2026-07-28 · What W2b's drifts are measured on, and what was deliberately
  NOT counted.** Three of the eleven rest on history rather than on today's tree:
  the WAL's `_Updated:` line is left **byte-identical by 10 of the 17** recent
  commits that edited its body — 58 %, the majority case, and the bare-date
  format is *why* there is nothing to bump within a day; the implicit hook fires
  on **28 of 37** active days since 2026-06-01, so nine ended with commits and no
  WAL commit; and `CONTINUE.md` is overwritten wholesale at **all seven**
  wind-downs and patched between them in **7 of the last 14** commits touching
  it. The cold-start order is reversed in writing on both sides: the flow says
  `CONTINUE.md` first then the WAL, `CLAUDE.md:205` runs the boot sequence to the
  WAL and reads `CONTINUE.md` second, and `CONTINUE.md`'s own resume prompt lists
  itself first.

  **`morning-routine.md` is unadopted end to end — the host has no morning ritual
  and no weekly re-read — and that is NOT counted as per-fact drift.** The line
  drawn, and it is the line the rest of `world` will be judged on: *a human's
  daily read leaves no repository artefact, and the flow never claims the host
  performs one; only where the host's own written contract contradicts the flow
  is it drift.* Two of forty-two facts qualify, both about the cold-start read
  order. The whole-document non-adoption is a finding for the report, not
  forty-two drifts.

  Two rule pairs were judged **differently on purpose**, because their own words
  differ: `NEVER-APPEND-TO-THE-WAL` prohibits appending only, and the host never
  appends — confirmed; `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` names patching
  too, and `CLAUDE.md`'s step 2 says «Update … bump … refresh» — drift. Same for
  `NEVER-APPEND-TO-CONTINUE` against `CONTINUE-IS-OVERWRITTEN-WHOLESALE`.

- **2026-07-28 · W2 CLOSED at 692, W3 CLOSED at 615 — the cluster is 51 of 121
  files and the per-file slice is now the working unit.** `world` reads **1 557 /
  140 / 17 — 90.8 %**, still below `ai-native`'s 91.6 %, and the phase crosses
  **61.7 %**. W3 returned **zero unverifiable** — the first batch in the phase
  where every fact could be settled against the tree.

  **The unit changed, and that answers the split question §4 left open.** The plan
  said to re-measure the per-anchor cost after the first world batch and split W5
  if it ran higher than C1's. The measurement is moot because the batch stopped
  being the unit of work: seventeen slices closed here, one file each, from 17
  rows to 149, every one merged and sealed on its own. `merge-verdicts.py` accepts
  a subset of a batch's files under the same batch id, so a slice that lands
  cannot become a debt — which is the property the 138-row debt was missing. **W5
  does not need splitting; it needs twenty-one slices.**

  **What the two batches measured, by family rather than by file:**

  | family | measurement |
  |---|---|
  | the size budgets | boot lane **~16 100 tokens against 500** (32×), WAL ~4 000 against 3 000, **9 of 47** module specs over 5 000 — and «split when over» has fired **zero** times |
  | the four-field record | **4 of 153** sections carrying a Decision label have all four fields; **127** have the Decision line alone; 142 carry no revisit condition |
  | the revisit trigger | 11 exist in `spec/`; **1** has metric + threshold; **none** has all three parts |
  | the sync grammar | `docs(spec): sync …` typed **0 times in 2 041 commits**, against 183 `docs(spec)` commits; neither recorded sync cites a `spec://` URI |
  | anchor coverage | 857 of 982 headings anchored — and the 125 without are **all 23 in `spec/boot/` and all 8 in `spec/WAL.md`** |
  | anchor uniqueness | **59 `duplicate-anchor` warnings**, all in the generated boot lane, where `{#root}` means 27 things |
  | the changelog line | exercised **once** across 42 PROP documents |
  | the `Test:` line | **zero** in `spec/`; all 223 `verifies` edges come from code |
  | tombstones | **2 of 2** document moves left the old address bare |

  **Three internal contradictions, each inside one package or one lane.** The
  `wal` package ships two wind-downs — six steps in `session-end-hook.md`, five in
  a different order in `cold-resume.md`, and a third fact calling them the same
  procedure; the host implements the five (F-129). `record-template.md` says an
  event trigger fires «without anyone having to remember to wonder» while
  `revisit-triggers.md` says triggers do not fire themselves — and the host's one
  event trigger sat unfired for six weeks. And `cognitive-load-split.md` says text
  that works for the AI works for the other two «for free» while the wal package
  says the report and the WAL «serve different readers» and the host writes both.

  **And one collision of principle, recorded on the host side in the prescribed
  form.** `uncertainty-protocol.md` tells a session to prefer adding no
  dependency, «because a dependency is a permanent tax». `spec/common/PROP-000.md`
  §15 decides the opposite at the governing anchor, with a why that answers that
  reason directly. Not non-adoption — a weighed, written, opposite ruling.

- **2026-07-28 · The verdict line that made these two batches consistent, and it
  is now the cluster's rule.** A flow's prescription the host simply never adopted
  is **not drift** — a human's morning read leaves no repository artefact, and no
  flow claims the host performs one. **Drift is where the host's own written
  contract contradicts the flow**, or where a measurable rule is broken over a
  double-digit share of its window. `morning-routine.md` is unadopted end to end
  and scores 39 of 42 confirmed; its two drifts are both the cold-start read
  order, which `CLAUDE.md:205` reverses in writing. Judged the other way, one
  unadopted document would have produced forty-two drifts and buried the two that
  matter.

  Two corollaries the batches needed. **Each fact is judged on its own sentence,
  never on its family** — `NEVER-APPEND-TO-THE-WAL` prohibits appending only and
  the host never appends (confirmed), while `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND`
  names patching too and `CLAUDE.md` step 2 says «Update … bump … refresh»
  (drift). And **a definition that classifies correctly is confirmed by the
  failure it classifies**: «a decision without a revisit condition becomes a
  sacred cow» is confirmed by 142 sections having none, not refuted by them.

- **2026-07-28 · The delegation lesson W3 paid for, and it was a two-sentence
  change.** Three `opus5` workers returned 615 anchors over 1 805 refs with **zero
  unresolvable on the first pass** — the first batch in the phase to verify clean
  without a repair run. The change: the briefs told them to prefer structural
  citations and to reach for `CLAUDE.md`, `spec/boot/**`, `spec/common/**` and the
  crates rather than `CONTINUE.md` and `spec/WAL.md`. One worker reported «Zero
  refs point at CONTINUE.md or spec/WAL.md».

  **This was learned the expensive way, twice in one session.** W2's four tables
  verified at 3 unresolvable when sealed and at **65** when re-read, because a
  wind-down had overwritten both files underneath them; and then this session's
  own checkpoint did it again to W2c and W2d. Not one of the 71 broken refs was a
  fiction — `git show 100617b3:spec/WAL.md` still carries every quote verbatim.

  **Two workers also corrected the harvest that commissioned them**, which is what
  a delegated search is for. The REVIEW-marker contract is not unexercised: a
  three-file grep found one hit, the widened search found five live markers plus a
  shipped `review_aging` check, a standing audit category and a task stop rule
  that has fired twice — and the gap that matters, which is that `review_aging`
  scans `spec/` only and cannot see four of the five. **And one worker's own
  absence was asserted rather than checked** — «no host rule discouraging new
  dependencies exists» against a `PROP-000` §15 that exists and says the opposite.
  The campaign's named trap, caught by re-reading, in the session that wrote the
  trap down.

- **2026-07-29 · W4 CLOSED at 564 — 474 / 89 / 1, 84.0 %, the phase's lowest
  batch and the first that measures a practice the host has LEFT.** Per package:
  `redbook` 96.4 %, `discovery-prompt` 92.8 %, `comparative-research` 84.4 %,
  **`campaign-plans` 72.5 %**. The phase crosses **71.4 %** and `world` falls to
  89.3 % against `ai-native`'s 91.6 %, so §5-C's prediction diverges further in
  the direction W3 already recorded.

  **The sixty drifts in `campaign-plans` are one finding.** The fifteen-section
  plan skeleton this flow defines is instantiated exactly once in the repository,
  and that instance is in `legacy-spec/`. The two campaigns the repository runs
  today carry none of it — risks 16 archived / 0 live, non-goals 9 / 0,
  quick-start 7 / 0, whole-campaign acceptance 8 / 0, execution ledger 8 / 0,
  commit maps 3 / 0, safe stop 12 / 0, Phase 0 five archived and none live. The
  live campaigns replaced the one-file dialect with a zone directory and eight
  side documents, which §13 of the format explicitly permits — but the sections
  went with the dialect.

  **The measured window, stated in the verdict reasons so it can be re-judged.**
  When a flow's rule has archived host instances and no live ones, the window is
  the current tree, and the archive is cited as evidence the practice was ONCE
  ADOPTED — which is what makes the absence drift rather than non-adoption. A
  rule the host never followed is confirmed; a rule it followed in twenty-five
  archived plans and stopped following in the two it runs now is not.

  **Three defects need no host to see them.** `campaign-plans`' execution-record
  example says «Two predictions falsified … the other four held» where the
  campaign it was copied from says three and enumerates two.
  `comparative-research`'s worked fragment is introduced as obeying all five laws
  and obeys four — it has no re-fetch section, the artefact its own skeleton
  defines. And `discovery-prompt`'s `usage.md` widens a rule the artefact
  deliberately scoped («every control BELOW is a hashtag») and is contradicted by
  the artefact's own fourth knob, then contradicts itself fifteen lines later on
  whether `<EXTENSION_CRITICALITY>` is configuration or mechanism.

  **What no host study does, measured five ways.** No study obeys all five laws
  of `comparative-research`. No quote anywhere carries a per-quote access date —
  `grep -rnE '— .*, accessed 20[0-9]{2}-'` returns zero. No delta anywhere carries
  a verdict, and no accepted delta carries a revisit condition of any kind. No
  study writes a re-fetch walk order. And the never-paraphrase law is contradicted
  by an owner directive — the clean-room rule — with the legal rationale quoted
  verbatim, which is a collision of principle rather than neglect.

  **Findings: NONE opened.** Every W4 drift that reaches finding grade confirms
  one already filed — F-113 (redbook's three rosters, 22 / 21 / 23), F-114 (the
  edition contract falsified by its own manifest comment), F-119, F-122. Reading
  the ledger before judging is what kept a duplicate F-130 from being opened for
  the roster gap.

- **2026-07-29 · Eight evidence tables, eight clean first passes, and three
  harvests corrected by the workers they commissioned.** W4a-d and W5a-d returned
  1 632 rows over **4 643 refs with zero unresolvable**, and none cites
  `CONTINUE.md` or `spec/WAL.md` — proved when this session rewrote both files
  mid-flight and re-verified every table at zero. The two-sentence durable-citation
  rule in the brief is now paid for eight times.

  The corrections run the other way from the usual delegation worry. **W4b:** the
  harvest said the fractality study notes carry no two-way gap section — an
  absence asserted, not checked, the campaign's own named trap walked by the
  session that keeps quoting it. Two `*-SYNTHESIS.md` files were listed and never
  opened; both carry `## 2. Two-way gaps {#gaps}`, and the pipeline's downstream
  half is live (ROADMAP records eight milestones deriving from the Tessl study).
  **W5d:** the harvest printed ten paths from a `grep` that returns nineteen, and
  the truncation dropped `crates/vibe-publish/src/token.rs`, where the `Token`
  wrapper and both Law-4 tests live — a worker following the list would have
  reported the flow's only code-enforced law unimplemented. **W4d and W4c:**
  `spec/boot/INDEX.md` does not carry the qualified member names, and the
  dangling-pointer count was an undercount because the regex could not see a
  root-relative `spec/flows/…` path.

- **2026-07-29 · Two tools, and one of them is the answer to «how much is left».**
  `tasks/batch-progress.py` joins what each batch OWES (`PHASE-C-BATCHES.json`,
  generated from `run/mirror`) against what is WRITTEN (`run/cache.json`) and
  names the unopened files of an open batch. The phase had counts and no way to
  see the remainder; every «X % of the phase» before it was that subtraction done
  by hand, and one was done in the head and landed wrong.
  `tasks/make-slice.py` builds a slice from an evidence table plus a rulings map,
  with six refusals tested against W3's closed tables before first use. The one
  that matters refuses any slice whose file has an addressable anchor the table
  does not cover — the property the 138-row debt was missing. Validated by
  regenerating a closed slice: identical anchor set, identical `src`, worker refs
  identical on 63 of 64.

  `verify-evidence.py` also gained one character. Its ref pattern required
  `<name>.<ext>` before the colon, so an extensionless dotfile fell through to
  UNPARSED — W4b hit it three times on true `.gitignore` refs the instrument
  could not read. Leading `+` became `*`; the three already-trusted tables re-run
  to identical counts.

- **2026-07-29 — PHASE C CLOSES. 6 847 / 6 847 anchors, zero owed.** Every
  addressable anchor in every shipped package file now carries a verdict backed
  by evidence that resolves to a real line in a real file. All seven world
  batches closed: W1 407, W2 692, W3 615, W4 564, W5 697, W6 572, W7 603.

  **Exit-gate clause (ii) — the X/Y/Z summary.** Over the whole marked corpus:
  **10 700 confirmed / 601 drift / 45 unverifiable = 11 346, 94.3 %.** By zone:
  `host` 4 496 / 0 / 3 over 58 files (99.9 %); `ai-native` 2 470 / 207 / 20 over
  80 files (91.6 %); `world` 3 734 / 394 / 22 over 121 files (90.0 %). Reproduce
  with `tasks/summary.py`; per-batch with `tasks/batch-progress.py`. Both print
  the truth and supersede every figure written here.

  **Exit-gate clause (iv) — the self-referential count (amendment A2).** Of the
  world zone's 4 150 verdicts, **248 carry `src == [1]` — 6.0 %.** Those rest on
  the package's own artifacts alone, with no host observable and no installed
  witness, and they are overwhelmingly structural lead-ins, package-internal
  cross-references and illustrative genres (an ssh-config example in a repository
  that writes no ssh config). A reader weighing this campaign's evidence should
  discount them and read the other 94 %.

  **Exit-gate clause (v) / amendment A6 — `baseline.json` written.**
  `vibe progress baseline --campaign campaigns/packages-2026-09` emitted **2 216
  units — 1 706 confirmed, 491 drift, 19 unverifiable.** Recorded for whoever runs
  the next `rescan`: **60 units were omitted for want of a judged fact and will
  read as `new`**, and 58 verdict keys matched no fact anchor (the per-file
  `_elements` bundles). Neither is a defect; both are shape mismatches between the
  verdict cache and the unit model, and a rescan that does not expect them will
  read 60 phantom additions.

  **What the phase found, in one paragraph.** 601 drifts, and the recurring
  shapes are four. The `../flows/…` sibling pointer dangles in **seven
  consecutive W6/W7 packages** — the host has no `spec/flows/` directory, so every
  boot snippet points a session at nothing, and the root-relative variant inside
  two re-derive prompts is invisible to the campaign's own `\.\./flows/` scan.
  Rules with no checker fail while their checked siblings hold — `source-mirrors`
  is the controlled experiment: never-`--force` has a unit test and held, while
  never-push-to-a-replica and the ancestry gate have none and both failed, 130
  pushes and 0 `merge-base` calls respectively. Verbs are specified and never
  built — managed-blocks' `remove`, qualified-naming's `KindMismatch`, each
  costing five to six sentences downstream. And the collection over-counts its own
  contents in exactly two READMEs, `spec-genres` and `tool-design-lessons`, both
  saying «four pieces of content» over three shipped documents where 14 of 16
  siblings say «three».

  **Three findings filed and deliberately unrepaired**, per the phase's own rule
  that C records and the next wave drains: the root `README.md:164` still calls
  vibevm proprietary over a `LICENSE.md` that has been UPL-1.0 since 2026-07-12,
  and is on none of `CLAUDE.md`'s enumerated stale-string exemptions; both mirror
  targets declare `refs = ["main", "tags"]`, so four local branches exist on no
  host and would be lost with this machine; and `CLAUDE.md:191` makes «Push to
  `origin/main`» step 4 of the END SESSION contract, which `90-user.md:13`, `:35`
  and `PROP-016:15` all name as *not* the rollout.

  **The method that made it hold.** The per-file slice as the unit of work, never
  the batch. Two instruments that refuse rather than guess — `make-slice.py`
  rejecting any slice whose file has an uncovered anchor, `merge-verdicts.py`
  refusing to restate a verdict without `--force` — and both caught real mistakes
  in this phase's final week. The durable-citation rule got its controlled
  experiment too: the one batch written before it carries 116 dead refs into
  rewritten checkpoints today, and every batch written under it verifies clean.

- **2026-07-29 — PHASE D OPENS. 601 drift verdicts become 228 obligations, and
  the first thing measured killed the obvious plan.** The batch plan is
  [`campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md`](../../campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md);
  the registry is `run/state/obligations.json`, generated by
  `tasks/drift-registry.py` and never hand-edited.

  **The hypothesis that failed.** Phase C was expected to have pasted one reason
  across every anchor it falsifies, so the drifts would cluster by reason text.
  Measured: of 601 reasons **only 16 texts repeat at all, over 54 rows**, and
  text-only clustering returns 552 groups for 601 rows — a reduction of 1.1×.
  Phase C wrote a bespoke reason per anchor. What groups them is the SUBJECT:
  one document, one kind of defect, one edit pass. Keyed that way and merged
  across documents on three provable signals — a reason shared at Jaccard ≥ 0.65,
  a finding id the worker itself cited (18 rows), one anchor drifting in two or
  more packages (16 anchors, 48 rows) — the corpus lands at **228 obligations,
  2.6 verdicts each, 95 singletons, largest 14**.

  **The order of the two clustering passes is the design.** Cross-document
  families form FIRST; `(document, type)` groups only what no family claimed.
  Run the other way, one shared anchor chains two whole documents of unrelated
  defects through the `(document, type)` key — it produced a 35-row obligation
  joining the dangling-pointer family to `COMPOSES-ATOMIC-COMMITS` before the
  passes were separated.

  **By type:** `reality-mismatch` 119 obligations / 359 verdicts ·
  `missing-support` 57 / 142 · `contradiction` 27 / 44 · `relocation` 20 / 49 ·
  `duplication` 5 / 7. `terminology` has no member and stays in the vocabulary.
  The two types are split so the split is decidable: **`missing-support` is an
  ABSENCE, `reality-mismatch` a DESCRIPTION that is wrong.**
  `reality-mismatch` is the residue class by design — the cheap types must
  positively match — and its rule is named `r-default-described-wrongly` so the
  default is never read as a match.

  **The number that sizes the phase is the closure route, not the type.**
  `prose-edit` 105 obligations / 232 verdicts (boss) · `build-or-demote` 54 / 134 ·
  `sync-from-code` 52 / 176 (**owner approves every spec diff**) · `release`
  17 / 59 (**owner before publication**). **69 of 228 obligations cannot close
  without the owner reading something**, so the wave is ordered around review
  cost rather than package size: D3 and D4 diffs are prepared first and closed
  last, and D1/D2 run against the owner's queue rather than after it.

  **Ids continue the one finding space** — F-131 … F-350, with 8 adopted rather
  than minted because a Phase C verdict already named that family. A second
  scheme would need a mapping, and the mapping would be a third writer for one
  fact.

  **Measured on the way, and it binds the release rule:** 212 of the 228
  obligations touch a file installed under `vibedeps/`, and **all 212 already
  differ from their installed copy** — Phase B marked up the shipped packages in
  place and that markup was never published. The difference carries no drift
  signal and is not used as one; the consequence is that «fixed in the package»
  and «the consumer reads the fix» are two different sentences.

  **The largest release event is also an open question, and the plan does not
  answer it.** The `../flows/…` family is F-136 (11 verdicts, 7 packages) and
  F-145 (8 verdicts, 8 packages): repair fifteen snippets in fifteen packages,
  or repair the boot compiler once so relative links are rewritten on compile.
  The second is a host code change, therefore Phase E's, and it would close both
  at a stroke. It goes to the owner with the release batch.

  **And this file's own status line still says «PHASE A OPEN», three phases
  later. It is left alone on purpose, and the reason is the first thing Phase D
  learned about its own cost.** Correcting it was tried and reverted: 11
  evidence quotes in `ev-W4a.json` quote that line verbatim — `ev-W4a` verifies
  at 872 refs and zero fiction before the edit, 854 and eleven after — and they
  are the evidence base for `campaign-plans`' own status-line rules, which are
  open obligations no wave has worked. Repairing the line as a side effect of
  opening the phase would close a drift silently and rot the trail underneath
  the verdict that found it. It is registered instead, and it closes with a
  re-judge like every other obligation. Appending this LOG entry shifted 7
  further refs by +60 lines; those are repairable and were repaired with
  `tasks/repair-refs.py --apply`.

- **2026-07-29 · the boot-link diagnosis was too strong by one PROP, and the
  correction is the more interesting half.** The entry above, and the commit
  that carried it, said the readable repair «contradicts PROP-009's *verbatim*
  and needs an amendment». That was written without reading PROP-035, and it is
  wrong. **The compiler is already a preprocessor and a linker.** PROP-035 is
  `Status: IMPLEMENTED`, extends PROP-009 explicitly, and calls itself «a real
  preprocessor + linker»; `render_static` concatenates the bodies at
  `crates/vibe-workspace/src/boot_artifacts.rs:259` and then calls
  `expand_embeds` at `:268`, under two tests — one of which asserts that text
  *without* directives is left verbatim (`boot_artifacts/tests.rs:264`). So
  «verbatim» describes the linker stage, coexists with directive expansion, and
  forbids only a rewrite of plain Markdown links.

  **The directive vocabulary, and how much of it is used.** `#embed` (macro
  splice), `#use` (dependency edge), `#source` (contract→impl edge), `@spec://`
  (mandatory in-place use), bare `spec://` (discretionary) —
  `crates/vibe-spec/src/directives.rs:1-21`, PROP-035 §7. Adoption over the
  whole tree, host and packages: **0 `#embed`, 0 `#use`, 0 `#source`**, and 11
  `@spec://` of which every one is documentation *about* the grammar rather
  than a use of it. The linker shipped three times over and has never been fed.

  **Three owner rulings, 2026-07-29, and they settle the family.** *(i)* A
  relative path to a specification is a **spec bug**, so the 69 links are a
  defect in 27 package files rather than in the compiler; they take `@spec://`
  where they are pointers and `#embed` where the target belongs in the lane.
  *(ii)* **A generated boot artifact carries no token budget** — recorded at
  [PROP-009 `##ARTIFACTS-CARRY-NO-TOKEN-BUDGET`](../modules/vibe-workspace/PROP-009-loading-model.md#artifacts)
  — which removes the only objection to `#embed`, and leaves the package's own
  budget row owed the same scope clarification at its next release
  (`BACKLOG.md` B-002). *(iii)* PROP-035 §10's link tables are **not** a
  precondition and are filed as `BACKLOG.md` B-001: they are the vtable of the
  §13 structural executor, a mode this project does not run, and an `@spec://`
  address that costs a lookup is strictly better than the confidently wrong
  path it replaces. No new compiler layer is built mid-refactor.

- **2026-07-29 · the first route-(b) ruling, and the plans this campaign runs
  were the ones breaking the rule.** `flow:campaign-plans`'
  `##COLD-A-LITERAL-QUICK-START-BLOCK` requires a literal quick-start block in a
  campaign plan. The archive keeps it — 13 of 25 files in `legacy-spec/terraforms/`
  carry the heading — and **both live plans in `spec/terraforms/` carried none**,
  which is what the W4 verdict measured. Owner ruling: the rule is sound and the
  host was wrong to drop it, so the package does not move and the documents do.
  Both plans now carry one (§10 here, §12 in wave 1's), and every command in
  both blocks was run before it was written down — one had to be corrected for a
  Windows encoding failure first, because a quick-start whose commands do not run
  is the defect this campaign keeps finding, not a fix for it. **The archive was
  not touched:** adding a quick-start to a plan that has already executed would
  be inventing history rather than repairing a contract.

- **2026-07-29 · waves 2–4 — the phase's central finding is a ratio, and it is
  not the one the plan expected.** Across four delegated waves, **179 anchors
  were examined and 25 moved**. The other 154 are §3.6 route (b): these are
  shipped **normative flows**, so most of what Phase C recorded as drift reads
  «the consumer does less than the rule asks», and the package is not the side
  that yields. A package moves only where its own sentence is false about
  something inside its own tree — its own bullets, its own summary, its own
  example, or a shipped sibling in the same namespace.

  That ratio changes what Phase D *is*. It is not mainly a document-repair
  phase; it is a **routing** phase that produces two queues for the owner and
  a small number of genuine package repairs on the way.

  **The repairs that did happen have four recurring shapes**, each closed in
  three or four packages: a claim that `flow:core-ai-native` «defers to this
  package from its next release», where the release landed days later and
  deferred in neither document (campaign-plans, wal, redbook, and the
  campaign-form pair); a commit-format rule mis-credited to
  `flow:git-atomic-commits`, which disclaims it in its own boot snippet, found
  three times including once inside the flow being mis-credited; a figure with
  no record behind it (`4–7/80`, `74.8 %`); and a lead sentence contradicted by
  its own bullets («two forces reshape it EVERY RUN», over bullets that are
  conditionals).

- **2026-07-29 · three claimed absences were perimeter misses, and the third
  nearly demoted a mechanism the consumer had already built.** The campaign's
  own law — *a `not-found` is a fact about the search perimeter until the
  perimeter has been checked* — was written after Phase C's C1 paid for it three
  times. Phase D paid three more in one session. A linter reported missing was
  installed and off PATH: with `C:\opt\gotools` on it, the Go pilot prints
  `floor: all green (7 step(s) run, 0 disabled by policy)`, and the claim a
  worker had deleted as unmeasured was exactly true. A floor run rooted at a
  *package* was read as a verdict on `research/go-demo`. And BROWNFIELD's
  phase-gate anchor was demoted on «nothing captures a golden transcript», from
  a grep over one crate's `src/` — while five transcripts sit at the host's
  `discipline/golden/`, `capture.sh:3` names **BROWNFIELD-PROTOCOL §6** as its
  own contract, and `terraform/BASELINE.md:117` applies §6 by name. **The host
  had implemented the mechanism and cited the anchor being demoted.**

- **2026-07-29 · twenty-four diffs reverted by owner ruling, and the rule that
  cost it.** The first wave's batches were cut by `falsifier == "self"` and not
  by `closure_route` — the field written into the batch plan hours earlier for
  the sole purpose of answering «who approves this». 24 of the 28 obligations
  handed out sat on `release`, `sync-from-code` or `build-or-demote`. Nothing
  was *closed* without approval, because no verdict moved; what landed was the
  right work in the wrong order, and the owner chose the full revert. Both rules
  are now [§6.1 of the batch plan](../../campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md#delegation-lessons):
  **a batch is cut by route first**, and **a demotion whose whole basis is an
  absence must name the perimeter it searched**.

- **2026-07-29 · the routing record, without which this phase cannot converge.**
  An anchor routed to the host is never repaired in the package, so it never
  stops reading `drift`. Left in prose, the registry could never empty, the next
  wave would re-derive the same answer, and the exit gate could not tell «not
  worked» from «worked, and the work belongs to the host» — the two things it
  exists to distinguish. `run/state/routing.json` is that record, one entry per
  anchor with its obligation and why, written by the boss at review time and
  never by a worker. The generator reports CONVERGENCE against it, and that is
  what §7's gate now measures.

  Two instrument defects were found and fixed by the work rather than by
  inspection. Obligation ids were positional and **shifted once**, before
  carrying them across regenerations landed — F-134→F-136 and F-142→F-145, both
  already cited in this LOG and in the batch plan, where they named nothing;
  five citations re-pointed. And carrying ids matched by *symmetric* Jaccard,
  which a partial closure defeats: two of F-205's three anchors were re-judged,
  the remainder scored 0.33, and the registry minted it a fresh id while filing
  F-205 `resolved` with one of its anchors still drifting. Matching is now
  containment.

- **2026-07-29 · state at the end of the prose-edit route.** Corpus **10 751
  confirmed / 550 drift / 45 unverifiable — 94.8 %**, from 94.3 % at the Phase C
  gate. Registry **206 obligations**; **397 of 550 verdicts still owe a package
  repair, 153 are routed out, 60 obligations have nothing left owed**. The
  `prose-edit` route is drained outside the address family. What remains is
  `build-or-demote` (54 / 134, the boss's, and every demotion now re-verified
  against the host perimeter first), `sync-from-code` (52 / 176, owner approval
  per diff), `release` (17 / 59, owner before publication), and the 26-obligation
  address family whose tag decision the owner has given and whose publication he
  has not.

  Two queues now await the owner rather than the phase:
  [`PHASE-D-RELEASE-QUEUE.md`](../../campaigns/packages-2026-09/PHASE-D-RELEASE-QUEUE.md)
  — 17 release events in four groups, two of them needing a product decision
  before an edit exists to approve — and
  [`PHASE-D-HOST-OBLIGATIONS.md`](../../campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md)
  — 53 obligations where the rule is sound and the host does not keep it, each
  taking one of three answers and none of them «soften the package».

- **2026-07-29 · wave 5 — eighteen claimed absences were false, and seventeen
  were falsified by the host. This invalidates a class of verdict, not a
  handful.** The `build-or-demote` route was re-verified rather than executed:
  76 verdicts examined across `core-ai-native` and the three language stacks,
  **18 absences did not survive**, and **17 of the 18 were disproved by HOST
  artefacts** — nine of them in `discipline/` or `terraform/`.

  The cause is structural and is now
  [§3.7 of the batch plan](../../campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md#compliance-blindness):
  **these packages specify a discipline, the host is the project that adopted
  it, and the artefacts that prove adoption live in the consumer because
  creating them is what complying means.** A search confined to `packages/`
  cannot see compliance and reads every successful adoption back as a missing
  mechanism. Since §3.3 closes a `missing-support` by *moving a marker*, each
  false absence would have written «specified, not built» over a mechanism the
  consumer had already built — a silent lie in the shipped surface, authored by
  the campaign that exists to remove them.

  What was actually there: the golden transcripts, the conflict detection, the
  REPORT and the intent reconciliation (F-151, five facts of six); the
  LLM-as-proposer loop run end to end with **54 owner-approved proposals** in
  `terraform/specmap-proposals.json` (F-139); `QUERY-PROPOSE-LINKS` executed
  with six affirmation commits behind it (F-152); `BAND-3-ON-TRIGGER` live in
  this repository's own compiled boot lane (F-150); the prediction ledger,
  complete (F-208, F-262); the specmark proc-macro at
  `crates/vendor/core-ai-native-specmark` (F-277); the two live-chain tests in
  both `-mcp` packages (F-214); the Go exhaustiveness linter installed at
  `C:\opt\gotools` (F-184).

  **And the six `BUILD-ORDER` verdicts, corrected as a set.** They said the
  catalog's numbered order «appears nowhere outside this document — not in any
  stack's cards, not in a host crate, not in an xtask task, not in a campaign
  plan». `terraform/` is none of those categories and is exactly where it is:
  `terraform/adopt-v0.3/LOG.md` carries it as phase headings, **six for six,
  position for position, pairings included** — E · F+G · B+C · D · A+H · I
  pilot-gated — and `##BUILD-ORDER-I`'s own «prototype and measure before» is
  that «pilot-gated» verbatim. The verdict says in its own words that it was
  *restated to match its five siblings*, so **consistency propagated the
  error**. Five of the six sit on F-158, a `sync-from-code` obligation: a
  re-judge that edits nothing produces no spec diff and therefore needs no owner
  approval — only an edit would.

  **State at wave 5's close:** corpus **10 832 confirmed / 470 drift / 44
  unverifiable — 95.5 %**, from 94.3 % at the Phase C gate. By zone: `host`
  99.9 %, `ai-native` **95.1 %** (was 91.6), `world` **90.9 %** (was 90.0).
  Registry **181 obligations**, from 228. **317 of 470 verdicts still owe a
  package repair; 153 are routed out; 60 obligations have nothing left owed.**

  Two operating rules were bought at full price and are recorded in §6.1: an
  `##ANCHOR` inside backticks is a **citation, not a definition** (three
  refusals from `merge-verdicts.py`, all correct), and **`merge-verdicts.py` and
  `progress seal` are never chained** — a refused merge still lets the seal
  vouch old verdicts against new text. A third was already written down and was
  broken anyway: verdict JSON generated through an unquoted heredoc lost every
  backticked identifier to the shell, and had to be rewritten and re-merged.

- **2026-07-29 · wave 6 — §3.7 has a mirror image, and the discipline's second
  adopter was invisible to every perimeter this campaign had drawn.** The
  remaining `build-or-demote` route was re-verified rather than executed: six
  delegated batches, **31 obligations over 57 verdicts**, cut by route first and
  by package inside it. **31 of the 59 claimed absences did not survive.**

  The cause is one structure, and it is §3.7 turned around. That rule was written
  against a search confined to `packages/` that could not see the host. Wave 6's
  verdicts made the opposite error: they scoped to the host's `crates/`,
  `spec/terraforms/` and `campaigns/`, and the project that falsifies them lives
  **inside `packages/`**. `packages/org.vibevm.fractality/fractality/v0.1.0/` is
  not a document collection — it is a **second complete project that adopted this
  discipline**, with its own `vibe.toml`, its own `vibedeps/` carrying twenty-odd
  installed flows, its own Cargo workspace, and six executed campaign plans. So
  the invariant is neither «search the host» nor «search the packages»: **the
  perimeter must contain every project that adopted the discipline, wherever it
  sits.** Recorded at
  [§3.7 of the batch plan](../../campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md#compliance-blindness).

  **What that one blind spot cost, by batch.** `campaign-plans` +
  `comparative-research`: **16 of 16 false** — every form called «zero live
  instances» (per-phase commit maps, the four-element phase, the runnable
  acceptance script, the hash-to-subject ledger) is live in the specspace's
  plans, which name `Format: flow:org.vibevm.world/campaign-plans` in their own
  status lines. `managed-blocks`: **6 of 9 false** — `remove`, recorded at the
  Phase C close as one of the two canonical «verbs specified and never built», is
  built, wired and round-trip tested there. Its paired half was re-verified on
  the same widened perimeter and **does not** fall: `qualified-naming`'s
  `KindMismatch` has no error type and no check anywhere, its only trace a doc
  comment at `crates/vibe-core/src/package_ref.rs:428` asserting the validation
  happens.

  **Three verdicts were false on the perimeter their own text named**, with no
  widening at all: `grep -rn 'Considered and rejected' ROADMAP.md spec/` returns
  **7**, reported as zero; `grep -rniE 'commits.by.meaning' spec/` returns the
  host's own Rule 3 in the compiled boot lane, reported as empty; and
  `grep -rn 'Confirm\|user_attended\|assume_yes\|interact()' crates/vibe-workspace/src/`
  returns **nine** hits including a gate named `consent_to_build`, reported as
  NOTHING. Re-running the verdict's own command is the cheapest re-verification
  there is and it caught three.

  **And two recorded figures did not survive re-measurement.** The imperative-mood
  breach is **211 of 400 subjects, 52.8 %** — not the recorded 144/400 — and the
  uppercase breach is 62 naive of which the large majority are identifiers, not
  42. The 400-commit window spans **four days** and is dominated by this
  campaign's own commits, so a figure over it must name its HEAD or it decays
  within the week.

  **The wave's shape is still routing, not repair.** 6 package edits landed —
  five §3.3 partial demotions and one correction — against **23 anchors routed
  out** under §3.6(b). The demotions that did land are all PARTIALs, because a
  flat «not built» would have been false in the half that works: the mirror names
  the host and not the divergent commits; the licence summary has one clause
  built, one a **marked exception** (a CI listing filed and deliberately declined
  under a standing no-CI posture, which §3.6(c) says is not drift), and one built
  by nothing.

  **The sharpest near-miss, and it is the argument for the whole re-verification
  pass.** `##INVARIANT-THE-ANCESTRY-GATE` was about to be demoted for the host's
  omission — and the gate **is** implemented, by this package's own fifteen-line
  reference script (`git ls-remote` then `git merge-base --is-ancestor`). Only
  the host's Rust port lacks it. Demoting would have printed «specified, not
  built» over a gate the package ships in `sh`. Auditing the port against that
  reference then turned up a real host defect, filed as `BACKLOG.md` B-005:
  `probe` tests **equality**, not ancestry, so `mirror --check` reports a target
  legitimately *behind* mainline as drifted.

  **Two instrument facts.** `merge-verdicts.py` accepted all 31 verdicts across
  three slices with **zero refusals** — the first wave in this phase to do so,
  which is what a batch cut by route and reviewed anchor-by-anchor looks like.
  And measured at HEAD, **2 of the route's 59 anchors were already `@spec/done`**,
  so §3.3's named closure move — drop the marker — did not exist for them in
  form; the route was assigned without consulting marker state.

  **State at wave 6's close:** corpus **10 863 confirmed / 439 drift / 44
  unverifiable — 95.7 %**, from 94.3 % at the Phase C gate. By zone: `host`
  99.9 %, `ai-native` 95.1 %, `world` **91.6 %** (was 90.9). Registry **171
  obligations**, from 181. **263 of 439 verdicts still owe a package repair; 176
  are routed out; 78 obligations have nothing left owed.** The `build-or-demote`
  route is **23 obligations / 28 verdicts**, from 33 / 59.

- **2026-07-29 · the address family cannot be closed by editing a package, and
  that changes who has to approve it.** Measured over three lanes: the
  `../flows/…` links dangle **0 of 70** in `packages/`, 21 of 142 in `vibedeps/`,
  and **75 of 75** in `spec/boot/STATIC.md`. The package is not the broken side.
  The boot compiler concatenates snippet bodies verbatim, so a relative path that
  meant `<pkg>/spec/flows/…` means the host's `spec/flows/…` once compiled, and
  the host has no such directory. The defect is the **form** — a relative path
  cannot survive being moved and an `@spec://` address can — which is exactly why
  the owner's ruling puts the repair in the packages rather than in the compiler.

  `STATIC.md` is generated from `vibedeps/`, so a package edit reaches the lane
  only through a bump and `cargo xtask sync-engines`. **No address obligation
  closes without publication, on any route** — joined to the registry by
  governing anchor that is **24 obligations, 47 of their 54 verdicts, 22
  packages**, of which only two sit on the `release` route and nineteen read as
  ordinary boss-closable prose edits. One approval covers all of them.

  The repair is therefore prepared as a command rather than as 62 hand edits:
  [`tasks/address-repair.py`](../../campaigns/packages-2026-09/tasks/address-repair.py),
  verified at **62 link constructs, 25 files, 25 packages, 62/62 addresses
  resolving, 0 malformed against the PROP-035 §6 grammar, 0 residual `../flows/`
  after the rewrite**. Read line by line, **all 69 links are pointers** — «Full
  protocol:», «Full model:», «Grammar and forms:» — so the `#embed` half of the
  ruling has no member in this corpus.

  **And one decision is owed that the queue did not know it needed.** `F-240`'s
  root-relative variant — a re-derive prompt opening `Read spec/flows/<name>/ …`
  — is recorded in **2 packages and present in 17**. The fifteen unrecorded ones
  are not mis-judged: the instruction lives inside a **fenced block**, which
  carries no anchor, so which of the prompt's claims got tested varied by worker.
  The same `##re-derive-prompt-lead` was judged `drift` where a worker read the
  path and `confirmed` where one read the prompt's shape. Filed as `BACKLOG.md`
  B-004. Publishing the two-package fix alone is what §4.5 calls not a closure.

- **2026-07-31 · wave 7 — the whole `sync-from-code` route re-verified without a
  single edit, and a third of it did not survive.** The route is the owner's:
  he approves every spec diff. But **a re-verdict that edits nothing produces no
  spec diff and therefore needs no approval**, so the route was run as a
  re-verification pass with a no-edit rule at the top of all six briefs.
  **The instrument confirms it held**: across three merges, 19 files, **0 sealed,
  0 refused, 19 already current** — the verdicts moved and not a byte of text
  did.

  **Of 171 verdicts examined, 47 were re-judged `confirmed` and 34 more anchors
  routed out.** By batch, the false count: `core-ai-native` 9/38,
  `addressable-specs`+`qualified-naming` 11/25, `managed-blocks`+`source-mirrors`
  12/30, the three stacks 13/45, the git family+`tool-design-lessons` 4/17, the
  tail 3/16.

  **The dominant cause is not mis-measurement, and that is the phase's second
  structural finding.** It is **a real defect convicting the wrong sentence** —
  now §6.1 `##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`. One batch found the
  same fact, measured once, confirming one anchor and drifting another **ten
  times over**, the drifting one almost always the summary whose own body rows
  are confirmed; another found **all three** of its false verdicts to be a
  sentence convicted of its neighbour's defect. The shape is mechanically
  detectable and now is: `tasks/summary-vs-body.py` lists every `##SUM-…` still
  drifting over a body that is not, and returns **17 candidates** at ratios of
  67 confirmed to 1 drift, 58 to 1, 45 to 2.

  **Three further named rules, each paid for.** `##READ-FURTHER-BEFORE-SEARCHING-WIDER`
  — three of one batch's four false verdicts were settled by reading the subject
  document further and searching nowhere at all, the sharpest being a fact
  convicted on evidence its own document exempts **twelve lines above**.
  `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS` — a host-live count showed one hit
  for *every* form, all of them inside this campaign's own plan, matching because
  the LOG entry written the day before quotes those words in prose; and the
  campaign **moves its own measurements**, a commit-subject breach shifting 1.2
  points inside one batch on six commits of our own bookkeeping. And §3.7 gained
  **a third address**: `core-ai-native` is simultaneously SPEC and ENGINE, a
  workspace of five crates vendored into six siblings, and **six of that batch's
  nine falls came from scoping the search to «the crate»** — the cheapest being
  a run-twice-diff test reported missing that sits in the very file the verdict
  cited, twelve lines below the line it quoted.

  **Owner ruling mid-wave, and it voided a class rather than a row —
  §3.8.** The `ai-native-lang` packages are built **first and foremost for
  external consumers**; `go` is a prototype specification deliberately unused
  here and not to be used here; **`rust` is the exception**, because part of
  VibeVM itself is written in AI-Native Rust. So §3.1's source 2 — «the host is
  a living consumer and the honest test bench» — does not transfer to a package
  whose audience is external, and **eleven of the stacks batch's thirteen falls
  are that ruling applied**, ten of them one tool-count over `research/ts-demo`
  pasted across ten anchors that describe tooling *for consumers*. Two findings
  filed earlier the same day were struck by it, both wrong: Go skills absent from
  the host's skill directories, and `PROP-026` designating `"go"` unsupported —
  which is a correct statement about the host's own dispatch, not a contract with
  a package serving somebody else. `legacy-spec/**` was excluded from the
  perimeter by the same ruling.

  **And the wave found the phase's own unfinished repair.** Wave 6's correction
  to `##CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT` did not
  propagate to its own summary **sixty-five lines below in the same file**, which
  still says `Implements:` markers plus `Test:` lines form a bidirectional graph
  — and `grep -rc '^Test: ' spec/` returns **0**. A `duplication` defect authored
  by the phase that exists to remove them, found by a delegated worker
  re-verifying the boss's output, and filed rather than fixed because the fix is
  an edit on this very route. It is group D of the new
  [`PHASE-D-SYNC-QUEUE.md`](../../campaigns/packages-2026-09/PHASE-D-SYNC-QUEUE.md),
  which §4 required and which did not exist until this wave produced diffs
  needing somewhere to live.

  **State at wave 7's close:** corpus **10 911 confirmed / 391 drift / 44
  unverifiable — 96.2 %**, from 94.3 % at the Phase C gate. Registry **165
  obligations**, at 2.4 drifts each. **180 of 391 verdicts still owe a package
  repair; 211 are routed out; 90 obligations have nothing left owed.**
  `sync-from-code` falls from 51 / 171 to **43 / 121** without one owner approval
  being spent.

- **2026-07-31 · wave 8 — the release route re-verified without a single edit,
  the boss's three unblocked verdicts closed alongside, and a strike-by-ruling
  turned out to be scoped by the wrong reason.** The route is the owner's before
  publication; the same basis that ran wave 7 ran here — a re-verdict that edits
  nothing produces no spec diff — and the instrument confirms it held across all
  three worker batches: merges of 45 verdicts with **0 refusals**, seal **1
  sealed / 30 already current**, the one sealed file being the boss's single
  intended package edit (below), not a worker's.

  **Of the route's 40 never-re-verified verdicts (59 minus the address family's
  19): 13 fell — 32.5 %, the predicted third — 3 routed out §3.6(b), 12 were
  restated on corrected grounds, 12 stand.** The route is **10 obligations /
  41 drifts, from 17 / 59**; F-115, F-186, F-212 and F-219 left it entirely,
  their surviving halves single-package and re-routed to boss lanes.

  **The falls' causes are the four named ones, and one is new in degree:**
  four of the stacks batch's five falls were **falsified by the failing
  verdict's own evidence list** — F-115's go and rust verdicts each cite, by
  path and line 1, the README they convict as missing; F-212's rust verdict
  cites the shipped names its sentence already uses. Cost to catch: one cache
  read. The rest: the string searched where the thing is a field (F-186 — «H4
  is in no register» against **four** ATLAS records under `refines:H4`, the
  H-series being the ledger's *axis field*, never a heading); a lead convicted
  of its fenced neighbour's defect (F-240, both — the do-not-copy carve-out
  sits two lines above each lead; B-004's counts corrected to 17 packages /
  14 confirmed / 0 drift / 3 unjudged, and the fence repair **changes no
  verdict on any scope**, because no instrument sees inside a fence); a
  sentence convicted of a claim it does not make (F-219's campaign-plans half
  claims subjects/ledger/hashes — kept by the `fractality` adopter: 8 phases,
  3 ledgers, 58 hashes bound — while the row's reason was its sibling's); and
  `legacy-spec/` ratios voided by the owner's ruling. F-187 and F-213 fell
  whole on §3.8/§3.7: the packages ship the skills their snippets name, and
  `discipline/golden/` is the *adopting project's* directory — the package's
  own terraform skill creates it, this host (the one real Rust consumer) has
  `capture.sh` plus transcripts, and the Go anchor never names `capture.sh`.

  **The wave's structural finding: an obligation merged by shared anchor
  carries per-anchor reasons, and a strike-by-ruling scoped by the row's
  reason hits anchors the ruling never examined.** §B.1's strike of F-189 was
  aimed at «the host does not dispatch `go`» — the GO anchor's reason; the
  rust and typescript verdicts always rested on PROP-026's own
  `##SUPERSEDED-TOPOLOGY` / `##TCG-CRATE-DELETED`, and the Go sentence is
  false on its own named subject `(vibevm, PROP-026)` — the `vibe-tcg`
  lockfile-dispatch topology is retired for **every** language, and the same
  Go document states the new topology correctly seven lines below. So the
  strike voided a *ground* and the *verdicts* survive on their own; the rule —
  one cache lookup per anchor before any strike — is recorded in the release
  queue §B.1. Три fenced `##three-processes-lead` diagrams still draw the
  retired topology with no anchor and no verdict — B-004's shape, flagged for
  the same publication.

  **A unit error in the queue's own restated figure, caught by arithmetic:**
  «716 commit bodies cite a `spec://` URI» was a **line** count read as a
  commit count — self-refuting, since a commit count cannot fall to the 579
  measured later. At `HEAD = 45cd30b0`: **581 of 2 216 commits** (735 lines),
  by `git log --grep="spec://" --oneline | wc -l`.

  **The boss lane closed five obligations in parallel, two by building.**
  F-241 — §3.3's revisit-when applied as written: the malformed-block report's
  line numbers were computed and discarded in both reporters
  (`locate_block`'s spans; vibe-check's `first_open`/`first_close`), and both
  now name each marker's line, the expected clause carries the zero-markers
  half of the well-formed shape, and the Class-F fix tail names the action;
  tests in both crates assert the lines (29 + 30 green). The same build closed
  **F-148 from code with zero spec diff** — a sync-route obligation whose
  reason's third charge («how-to-unblock absent on `vibe check`») itself
  convicted a surface the drill does not bind: `vibe check` aborts nothing.
  F-287 — wave 6's correction to `##CODE-MARKS-…` re-judged at last (677
  implements / 223 verifies edges, all authored code-side, `^Test:` 0, 224
  `#[spec(` sites — exactly the corrected sentence); the re-verdict was the
  closure's missing half. F-175 + F-303 — the also-test's «mechanical» removed
  from body and summary in one §3.6(a) self-correction; the test and the
  reader's-call concession stand.

  **`summary-vs-body` prints 8 candidates now, not 17, and all eight are
  adjudicated:** two already in the owner's queues (F-180, F-169's
  bidirectional-graph half), one closed above (F-303), four coherent — the
  summary drifts with its own body rows on one measurement (F-169, F-181,
  F-162's contracts half, F-147) — and the changelog pair restated from the
  string to the thing: `grep '^## Changelog'` finds 1, but **13 PROPs carry a
  dated change record under numbered `## N. Version history {#history}`
  headings, 38 dated entries** — 14 of 42 keeping the form, 28 keeping none.
  The re-measuring session's own first grep missed the numbered headings — the
  trap demonstrated itself mid-correction.

  **Filed rather than fixed:** `BACKLOG.md` **B-009** — `CLAUDE.md:191`'s
  wind-down step prescribes the bare `git push origin/main` that
  `spec/boot/90-user.md:35` and `PROP-016:59` both deprecate as not the
  standard rollout (surfaced when a recorded verdict used that line to demote
  a package sentence the other two documents support); the **H1–H6 roster**
  cited ~49 times corpus-wide and defined nowhere (owner:
  `core-ai-native/v0.8.0/spec/appendix/`); F-153's unjudged twins
  (`##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION` ×3 and core's own boot snippet);
  five inconsistently-judged word-identical siblings; and
  `typescript-ai-native-lang` as the only one of 42 shipped versions with no
  `README.md` in its history.

  **State at wave 8's close:** corpus **10 936 confirmed / 366 drift / 44
  unverifiable — 96.4 %**, from 94.3 % at the Phase C gate. Registry **158
  obligations / 366 drifts**, 71 resolved to history, one id newly assigned by
  re-clustering (F-354). Routes: release **10 / 41**, sync-from-code 42 / 114,
  build-or-demote 20 / 24, prose-edit 86 / 187. **157 of 366 verdicts still
  owe a package repair; 209 are routed out; 90 obligations have nothing left
  owed.** What remains of the release ask after re-verification: the address
  family's one publication, five stack corrections and two two-word counts
  behind the same gate, one which-side ruling (F-220's WAL-entry half), and
  B-004's seventeen-fence product decision.

- **2026-07-31 · the four D9 rulings, executed the same session — and the
  verdict-first rule for false confirms pays off twice.** The owner ruled in
  session (verbatim in the D9 harvest's `#rulings`): F-188-rust ships
  self-contained without the PROP-031 citation (release batch); F-219 takes
  option B (the F-253 wording, landed a second time); F-115 takes option (a) —
  the TypeScript stack gained its front-door README, the only one of 42
  shipped versions without one, tree-verified, arriving unjudged for the next
  mirror pass; F-245 takes §3.6(a) with the owner's own sentence, and **no
  Phase E `when`-vocabulary task is filed** — a while-shaping-identifiers
  condition is not decidable at boot time, recorded so Phase E does not
  inherit an unimplementable ask. The three prose-edit rulings were applied
  and closed the same session, alongside the boss-route residues F-186,
  F-212-go and F-132 (+ its ts twin): merges 2+6+1 verdicts, **0 refusals**;
  seal 5+1 sealed / 3 already current.

  **The riders' boundary held by one cache lookup each** — the wave-8 rule
  applied in its first live test. `conform-frontend-go.md` belongs to open
  F-185 on `sync-from-code` and was not touched. The two false confirms (the
  ts card's `DL1-015`, the Go GUIDE's `gated_packages`) were re-judged `drift`
  FIRST and minted by the registry: the ts one clustered to F-132 (prose-edit)
  and closed under it the same session; the GUIDE one clustered to **F-166 on
  the owner's sync route**, so its two-word swap now waits in the sync queue's
  group C **instead of landing as an unapproved diff** — the repair went
  through the measurement, and the measurement routed half of it to the owner.

  **State:** registry **153 obligations / 361 drifts**, 75 resolved to
  history; corpus **10 941 / 361 / 44 — 96.4 %**. The release batch carries
  **no open design choice** — every remaining correction is drafted final
  (d8b + d9), and the owner's queue is the whole remainder.

- **2026-07-31 · D10 — the host adopts the campaign-plans practice, and the
  ADR census premise is withdrawn by its own proposal.** Under the owner's
  «сделай»: **both plans gained the six flow forms** (Phase 0 · safe stop ·
  non-goals · risks · commit map · runnable acceptance — 12 blocks, 878 lines,
  drafted by a delegated pass and boss-reviewed with the commit-map hashes
  sampled against `git log`), the live status line was refreshed from the
  three-phases-stale «PHASE A OPEN», and §8 now names the zone deferrals
  ledger it had contradicted. **21 of the 29 routed `campaign-plans` anchors
  re-judged `confirmed`** — the worked `##COLD-A-LITERAL-QUICK-START-BLOCK`
  precedent at scale: the rule sound, the host now keeping it, no package
  edit. The 8 held back are honest: two at-close triggers, two status-block
  rows, the ledger-honesty pair, the standing-obligations summary, and
  «counts that reconcile» — blocked by the acceptance's own catch that
  `progress check` sees 260 files where `summary.py` sums 259. Registry
  **148 obligations / 336 drifts**; corpus **10 966 / 336 / 44 — 96.7 %**.

  **The B-007 proposal landed and withdrew the census's premise.** The
  fractality «14 complete records, ~41 % adoption» are — by file — **8
  carriers, all 8 vendored copies of the `decision-records` flow's own
  template, protocol and worked examples; 0 authored** (the specspace's own
  blocks: 9, three-label dialect, none complete). So nobody in this tree
  authors the four-field form except this campaign's own plans, the question
  is again *whether to adopt*, and the costed options with the campaign's
  recommendation (**B + A′**: four-field inside the owning section,
  forward-only, backfill `spec/common/` only, `spec/decisions/` closed
  explicitly) are in `harvest/d10-adr-genre-proposal.md` for the owner.

  **Two instrument findings, both filed.** `vibe progress check --exhaustive
  --campaign <zone>` **writes** the named zone's scan state — a delegated
  pass pointed it at the closed wave-1 zone and rewrote six state files
  (+4 962 lines in its cache), restored loss-free from HEAD; filed **B-010**
  (a check verb that writes, and a `--campaign` flag that selects state
  rather than scope). And `merge-verdicts.py` issued its fourth useful
  refusal class: 11 anchors filed under intuited documents were rejected
  with «not an addressable anchor of this file» — the slice was rebuilt from
  `routing.json`'s true paths and merged 21/21.

- **2026-07-31 · волна 9 — THE PUBLICATION. The address family closes whole,
  the release batch lands, and the lane heals: `../flows/` in the compiled
  boot lane goes 69 → 0.** The owner's «Публикуй» executed through the
  probed runbook; the marker fork was ruled **(а)** — publish as is — after
  the owner caught what the (б) recommendation missed: naive stripping breaks
  cross-lane resolution (a dynamic module can cite an anchor that vanished
  with the cleaning), so stripping waits for an aliasing design
  (`#use spec://… as SOMETHING`), filed as **B-011**.

  **The event is local, and no version was bumped**: the lockfile has pointed
  all 36 packages at this working copy since 2026-07-26, so
  `vibe reinstall --force` re-fetched every pinned version from `packages/`
  and regenerated the boot artifacts — 206 files under `vibedeps/` +
  `spec/boot/` moved (+15 167 / −9 950), carrying Phase B's markup into the
  installed lane as §3.5 said a re-vendor would. **What was applied before
  the re-vendor:** the address transformation (`address-repair.py --apply`,
  62 constructs in 25 files, 0 residual); the release-batch finals (F-153's
  six `spec/`-prefixes **plus the eight unjudged twins in the same lane** —
  the §4.5 arithmetic made partial publication a fresh duplication, and the
  owner's B-004 ruling set the pattern; F-211's two per-binary OP-INIT rows;
  F-188's three per-stack Motivations, rust per ruling (ii) without the
  PROP-031 citation; F-189's three PROP-027 seam rows **with the three fenced
  diagrams redrawn in the same diff**; F-190's two two-clause repairs;
  F-251's two counts); and B-004 ruling (i) — **all seventeen** fenced
  re-derive first lines now name the install slot instead of a directory no
  consumer has.

  **Verification, per the runbook:** `../flows/` in `spec/boot/STATIC.md`
  **0** (was 69), `@spec://` **69**; `address-repair.py --verify` — 0 links
  remaining in scope; `cargo xtask sync-engines --check` green (51 pairs, 9
  sets); **`tools/self-check.sh` EXIT=0** over the published tree;
  `progress mirror` refreshed (260 views). **The re-judge went through the
  instrument's own join**: a naive marker-filter matched 104 anchors — more
  than double the family — and was discarded; the honest join (repaired
  diff lines → governing anchor via the mirror's fact spans) gave **55
  governing anchors, 46 of them drift → confirmed** (nine were already
  confirmed on other claims of the same surfaces, B-004's multi-claim shape),
  landing exactly at §A.1's «47 blocked on publication». Merges 46+16+2,
  **0 refusals**; seals 22+18, 0 refused.

  **State at wave 9's close:** corpus **11 030 / 272 / 44 — 97.2 %**, from
  94.3 % at the Phase C gate and 96.2 % at this morning's wave-8 open.
  Registry **121 obligations / 272 drifts**; **84 of 272 still owe a package
  repair; 188 routed out; 87 obligations nothing left owed**. The `release`
  route reads 2 / 4 and both rows' anchors are wholly routed — **the route's
  owed remainder is zero**; F-136 and F-145, the two largest obligations the
  phase opened with, are resolved whole. What still owes a package: the sync
  queue's group B (23, unblocked, awaiting per-batch presentation) and the
  boss-route tail.

- **2026-08-01 · волна 10 / D13 — the three rulings execute, and the record
  form the campaign preached lands in the host's own specs.** The owner ruled
  three times in one sitting and every ruling was executed the same day
  (commits `c3b3fe19`, `e6f53d5d`, `eccb1499`): **B + A′** — the
  three-question criterion (Q1 condition · Q2 observation point · Q3 loser;
  R1–R3 pass, N1–N3 fail, every verdict citing its clause) landed in
  `spec/design/README.md` where the genre lives, with the forward-only rule,
  the `##ROW-NO-DECISIONS` genre-table row and the `spec/decisions/`-is-never-
  created statement — and **twelve four-field records** were backfilled inside
  their owning sections from their own recorded prose (PROP-000 ×5, PROP-018
  ×3 + the `Rationale:`→`Why:` relabel, PROP-024 ×4; census 35 `Decision`
  labels → 12 reopenable by the criterion; the two numeric thresholds the
  owner declined to invent are event-shaped until numbers exist). **Партия
  1a** — PROP-014's nine «Specified, not built» annotations plus
  RUNTIME-TRANSPORT's real per-family tool names, with the ten unbuilt
  mechanisms filed as `B-012` (owner: «провести исследование, можно ли
  реализовать»). **Health-audit adopted whole** («Проведи всё это»):
  `AUDIT.md`'s header carries the five clauses, DBT-0001 reconciled P1→P3
  with the audit's later evidenced judgement winning, the skill installed (5
  projections), the wind-down contract now points at `AUDIT.md`'s active
  subset in all three instruction files, and the A–D inventory is scheduled
  at the phase's exit gate. Merges D13: 30 + 10 confirmed, 0 refusals.

  **The tail the session left, and what it turned out to be.** `progress
  seal` refused 4 of the 5 record-bearing paths and the wind-down recorded
  the guess «the new `##record` anchors changed the anchor sets». The
  next-session re-run (mirror → seal → read) found the true cause one notch
  simpler: **the refusals were 51 brand-new record anchors carrying no
  verdict at all** (10 + 8 + 11 in the three PROPs, 22 in
  `spec/design/README.md` — the criterion's own units), and seal refuses a
  file that mixes vouched and unjudged markers, which is its safe mode
  working as designed; the package-side PROP-014 was already sealed against
  its bytes. The 51 were judged boss-side after a referent-by-referent
  verification (97 checks: every cited anchor resolves — including
  `#NO-SEPARATE-ADR-DIRECTORY` living in the flow's *boot snippet*, not the
  protocol, and `{#shippable-tree}` being a heading anchor; every quote
  verbatim against its source down to two sanctioned adaptations, a
  case-adapted «Document…» and a number-adapted silos clause with the anchor
  cited adjacent; `spec/decisions/` absent; the genre-map mutability rows
  exact), merged as **D14: 51 confirmed over 4 files, 0 refusals**, and
  sealed clean — **4 sealed, 0 refused**. The registry regenerated with
  nothing new minted: the records the campaign wrote about the tree hold
  against the tree.

  **State at wave 10's close** (HEAD `22d6ff7f` + this entry): corpus
  **11 121 / 232 / 44 — 97.6 %** of 11 397 (the +51 is exactly D14; wave 9
  closed at 11 030 / 272); registry **108 obligations / 232 drifts**;
  CONVERGENCE **73 of 232 still owe a package repair, 159 routed out, 77
  obligations nothing left owed, 5 partly routed**. The owner's queue is
  unchanged and is the whole remainder: sync group B партии 1b–1d
  (F-146 ×5 + F-206 ×2 ENGINE-CONFORM, F-159 ×5 LEDGER-INTENT, F-207,
  F-263 — texts final in `harvest/d7a-core-sync-reverify.md`), the d12-adr
  §3.13 `##SPLIT-HOST-POSTURE` carve-out (35→36), and the build-or-demote
  tail (~17 of 21). The health-audit 16 are NOT in the queue: «Проведи всё
  это» closed them by adoption — every health-audit file reads 0 drift in
  the cache, the ev refs carry the ruling verbatim.

- **2026-08-01 · волна 11 — the owner drains his whole queue in one sitting,
  and the per-anchor rule catches a strike-by-ruling before it lands.** The
  full decision package presented in plain language (his format feedback of
  the same day applied: essence first, spec specifics as supplementary
  material) came back ruled end to end. **Партія 1d applied** — F-207's
  amendment sentence now says which thirds of the brownfield edge model ship
  (the `disputes` pairing is a first-class field, not «a doc comment»; the
  freeze and the coverage math annotated in the sanctioned form) and F-263's
  front door stops claiming «prompt content only» over five authored library
  crates; both re-judged and sealed (D18). **Партія 1c routed to research
  instead of edits** — `BACKLOG.md` B-022, F-159 `deferred` pending it; партія
  1b's two frontend rows likewise → B-023 (its other five corrections remain
  presented, unruled). **The carve-out executed as (ii)** — census 35→36:
  `##split-host-decision/-why/-rejected/-revisit` land at PROP-000 §7 carrying
  the owner's updated posture verbatim in substance (GitHub leads both
  surfaces; GitVerse supplementary — full source mirror plus
  deliberately-published-to registry storage, never blanket-mirrored; the
  leading-role half re-opens only by the owner's notice, the same-day B-015
  pattern). **The build-or-demote tail closed as owner-ruled deferrals** —
  15 rows answer (б) with their host debts named in
  [`PHASE-D-HOST-OBLIGATIONS.md#rulings-2026-08-01`](../../campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md);
  F-351 closed outright (the wind-down's step 2 says «Rewrite … wholesale» in
  all three instruction files — the B-009 shape, second run); **F-230 split
  per-anchor**: the enforcement anchor closed by the (в) exception now
  recorded at PROP-000 `##ATTRIBUTION-ENFORCEMENT-EXCEPTION`, and the
  one-place anchor — whose cached reason is a different defect (ten
  restatements, two drifted, a dead «§12.1» pointer) — was **not** closed on
  that ruling, per `##WAL-C-STRIKE-PER-ANCHOR`, and stays `deferred` with its
  own debt. B-024 filed from the owner's own question (two tombstones, one
  concept: `retired` vs `void`). Merges D18+D19+D20 = 2+2+5, 0 refusals;
  seals 4 + 1 already current.

  **State at wave 11's close** (commands at this HEAD supersede): corpus
  **11 147 / 228 / 44 — 97.6 %** of 11 419; registry **105 obligations / 228
  drifts — 17 deferred by owner ruling, 88 open**; owed **71 of 228, every
  one on the owner's sync route**; F-207, F-263, F-351 resolved to history.
  The owner's remaining queue: партія 1b's five unruled corrections, and the
  sync queue's group C and beyond.

- **2026-08-01 · волна 11, второй заход — партія 1b's five rule themselves
  into two edits and three builds.** The owner ruled the re-presented five
  (problem/options/recommendation form): **item 3 (а)** — the crash fact now
  describes the built hard-error probe and annotates the unbuilt
  skipped-status; **item 7 (а)** — the closing rule takes the exact form its
  four siblings took under the group-A ruling, one rule one form in five
  documents (both applied, D21 merged 2, sealed 1/0). **Item 5 → B-025
  (planned)**: downgrade-not-suppress — the owner's ground recorded verbatim
  (visualisation tools need everything visible; suppressed findings vanish
  from the IR); **item 6 → B-026 (planned, high priority)**: SARIF ingest —
  foreign linters' diagnostics become facts. **Item 4 became a question, not
  a ruling** (what does specmap track; are facts addressable; how do the two
  maps cooperate) — answered in chat, the trait-signature correction stays
  presented and unruled. **Item 7's own follow-up question became B-027**:
  the «Specified, not built» facts carry markers inconsistently (@spec/done
  vs @impl/done — партія 1a's own mix), the owner's guessed `@impl/planned`
  exists in the grammar as `@impl/plan`, and the proposed audit rule
  (not-planned → @spec/done; planned-with-backlog-entry → @impl/plan) waits
  for his word before the sweep. F-206 `deferred` naming B-026; F-146 stays
  open on the one unruled item. Registry **105 / 226 — 18 deferred, 87 open;
  owed 69, every one on the sync route**.

  **Third sitting, same day.** «Пока просто поправь формулировку» — item 4
  applied as **D22** (the quoted trait signature now matches the shipped
  `finding.rs:51-56`, with the impossible `specmap` parameter explained by
  the separability seam), which makes F-146 fully owner-ruled → `deferred`
  (two anchors on B-023's research, one on B-025's build). **Group B is
  drained.** And the lifecycle question graduated: **B-024 raised to
  `planned`** — the owner's «свести стадии жизненного цикла в specmap к
  аналогичным в progress» fixes the direction (derive from the host's
  markers; only `disputed` has no analog), the research narrows to the
  mechanics. Registry **105 / 225 — 19 deferred, 86 open; owed 68, every one
  on the sync route**. Next presented: the sync queue's group C singles
  (F-180's one-clause-in-three-places, F-166's three), group D (the
  campaign's own summary debt, inside F-169's four), and F-169's §3.6(c)
  pair riding with F-147's twins.

- **2026-08-02 · группа C открывается — F-180 закрыт одним диффом в трёх
  местах, а два черновика возвращены владельцем на перепроверку и
  перепроверка меняет рекомендации.** The owner's repl-policy phrasing
  («main + теги + специально заявленные ветки») is exactly the prepared
  qualifier, applied to all three carriers of the unqualified «full history»
  (the summary, the offboarding bold, the protocol's vanish-safety bullet) —
  D23 merged 3 (one drift→confirmed, two re-vouches of amended confirmed
  anchors), both files sealed, **F-180 resolved**. His two challenges both
  paid: **(2.1)** the shipped engine reads exactly one key — the
  Rust-flavoured `gated_crates` (`config.rs:44`, `deny_unknown_fields`, so
  the Go-idiomatic word errors loudly) — and the three Go docs now disagree
  two-against-one *because the D9 correction itself wrote the Rust word into
  the Go skill*; the honest menu is document-the-shipped-key now + an
  engine-side neutral/per-language key as a backlog build. **(2.3)** the
  tier vocabulary (T-lex/T-syn/T-sem) is used across the live slot — the
  MISRA guide even sketches the future rule-registry column — and defined
  nowhere: the H-roster shape again; the recommendation flips from
  strip-the-parentheses to add-a-legend-anchor and re-judge, and the W2-era
  verdict is recorded as the capability/practice misattribution it was.
  **(2.2)** answered on the §3.8 bench: the claim fails on the package's own
  rule roster (three rules, none parses the assertion), not on host
  non-usage — and the Rust gate carries no such rule either, so the promise
  is aspirational family-wide. **B-028 filed high-priority** (the package
  publishes a subset of the URI grammar the host implements). Registry
  **104 / 224 — 19 deferred, 85 open; owed 67, every one on the sync
  route**.

  **The rulings landed the same day, and F-166 closed whole.** 2.1 (а)+(б):
  both Go docs now name the key the shipped engine reads, with the honest
  parenthetical, and the engine-side neutral/per-language rename is
  **B-029** (planned); 2.2 (а)+(б): the assertion fact carries the sanctioned
  annotation on the §3.8 bench, and **B-030** (planned) builds the Go check
  and surveys Rust/TS for the same promise-vs-gate gap — the owner's own
  extension; 2.3: the owner-confirmed **##TIER-VOCABULARY legend** landed at
  the frontend table and TWO-TIERS re-judged confirmed — the W2-era
  taxonomy-convicted-of-a-missing-enum misattribution falls with the
  definition in place. SUM-THE-URI-SCHEME confirmed per the wave-7
  recommendation. **3.2 became the owner's own architecture direction:
  B-031 (planned)** — the root project takes a fully-qualified name
  (his word: `org.vibevm.core`), the resolver reads vibe.toml addressing
  everywhere including the root, the host exceptions die with the short
  notation — and the segment family (F-169's two rows, F-147's twins)
  closes through host conformance when it lands. Merges D24+D25 = 4+1
  confirmed, 0 refusals; seals 4/0. Registry **103 / 220 — 19 deferred, 84
  open; owed 63, every one on the sync route**. Awaiting the owner:
  SUM-THE-BIDIRECTIONAL-GRAPH's «применяй» (his 3.1 question answered in
  chat: the graph lives in specmap.json + the code-side tags; the user tags
  code, never writes Implements:/Test: prose).

  **The «применяй» came, and the boss lane drained what needed no one.**
  Group D's summary fix applied and re-judged (D26); F-169 `deferred` — its
  two segment rows ride B-031 with F-147's twins, consistently. Then the six
  wave-7 re-judge recommendations that involve no edit merged as **D27**
  (F-147's three: per-module numbering with five PROP-001s coexisting, the
  never-read premise with no instance, the one-to-one prediction read as
  fulfilled; F-162's three: «Version history» is the changelog rule
  practised under another heading — 15 of 42 PROPs, 33 dated entries — the
  tombstone «2 of 2» falsified by PROP-029's three pointers, and the summary
  inheriting). Corpus **11 163 / 213 / 44 — 97.7 %**; registry **103 / 213 —
  20 deferred, 83 open; owed 56, every one on the sync route** (no row fully
  resolved this pass — the six confirms shrank rows, F-147 and F-162 stay
  open on their presented remainders). Presented
  and awaiting the owner: F-162's two graph-anchor texts (+ the NO-TOOLING
  rider applied in the same pass), F-147's ROW-HOME cell with the
  two-flows-one-home question (FEAT-* 0 live instances vs 8 live plans —
  practice has chosen), and the five dangling PROP-043 anchors as a
  host-fix candidate.

  **All three ruled the same day, and the owner's question out-diagnosed the
  presenter twice.** F-162's pair + the NO-TOOLING rider applied and merged
  (D28) — **F-162 resolved whole**; ROW-HOME's cell now names both homes per
  answer (а), F-147 fully ruled → `deferred` (its three segment anchors ride
  B-031), and the granularity-protocol extension he dictated (ask the user;
  FEAT files as addressable units composed into plans; big features only)
  is **B-032** (planned — not elementary: a new norm across two packages).
  His agent-navigation question landed in **B-018 as the canonical
  acceptance query** («which test verifies this rule» — CLI and stack-MCP
  answer it today per checkout; vibe's own MCP cannot, which is exactly
  B-018 part (i)). And the dangling-anchors framing was WRONG the way he
  suspected: the three lines are fact-anchored already (##CMD-REPORT/-SEAL/
  -WEAVE, judged confirmed) — the dead half was the `{#report}`-style
  pseudo-anchors beside them, heading syntax on list items that no reader
  registers, and **the code cited the dead aliases**. Fix executed in his
  direction: the five code tags now cite the live fact anchors
  (`#CMD-REPORT`/`#CMD-SEAL`/`#CMD-WEAVE`; cargo check green), the four
  trap-tokens removed from PROP-043, and the five dangling edges die at the
  next index regeneration (B-014's question). Corpus **11 166 / 210 / 44 —
  97.8 %**; registry **102 / 210 — 21 deferred, 81 open; owed 54, every one
  on the sync route**.

- **2026-08-02 · ситтинг предъявлений — очередь оказалась на одну девятку
  короче, а стройки берут верх над смягчением.** The sitting opened with the
  standing panel (`self-check.sh` EXIT=0, re-run after the string-only tag
  edits) and a preparation find: **the queue line «F-132's nine await texts»
  had been stale for two checkpoints** — партия 1a (`eccb1499`, D13's +10)
  applied and re-judged the nine + RUNTIME-TRANSPORT already on 2026-08-01,
  and the queue had quoted the harvest's pre-1a «NOT APPLIED» snapshot instead
  of the registry. Caught by taking every queue item from `run/cache.json` +
  the generator before presenting; WAL/CONTINUE repaired (`5223ea2e`). The
  lesson is `##WAL-C-DURABLE-CITATIONS` from the other side: **the owner's
  queue is derived from the registry, never from a harvest snapshot.** The
  presentations then went out twice — the first draft bounced off the owner's
  format bar, refined the same sitting and **binding forward: essence in plain
  language first, then the exact technical names (settings, files,
  behaviours), precision never lost; spec jargon only as appendix.**

  **The rulings came back builds-over-softening, end to end.** F-185's three
  Go-frontend promises stop being false by the engine growing to them:
  **B-033** (the dedicated seam-error rule; the owner's «имеет ли смысл —
  или особенность языка?» answered: the rule architecture is neutral, only
  the detector is language-shaped, so a dedicated id is sensible and cheap —
  detection already ships as the census kind), **B-029 enriched** («обогатить
  это обещание … расширить или улучшить то, что мы сохраняем в conform.toml»
  — the entry grows from a key rename to the per-language config surface),
  **B-034** (the gated-or-exempt invariant implemented for Go and TS —
  «похоже на задачу»), and **B-035** (the parity audit, carrying his
  principle verbatim: support for other languages — especially TypeScript —
  must not be weaker than Rust without a recorded reason). His frame for the
  family: *«По сути мы не можем писать на Typescript и Go пока не поправим
  вот это.»* The first parity pass ran in-sitting: **TS has no REQ-citation
  check at all** (five unsafe census kinds, none about seam errors, against
  its guide's own prescription), the invariant runs nowhere but Rust, and the
  TS docs are honest about it — the weakening is engine-side, undocumented.
  F-132's residual anchor took answer (1) — the host debt «проставить
  spec-метки в `schemas/specmap.jtd.json`» recorded with companions B-013 /
  B-019(а), «сделать как будет возможность». F-218 deferred onto **B-011,
  raised to Самый Высокий Приоритет** and enriched with the owner's design
  directions (rename-on-splice with every reference staying valid, the
  dynamic-STATIC.md loading case, qualified-rewrite at materialization, the
  C++ ADL analogy, `#use spec://… as X` + `@!X`): «От этой вещи зависит как
  вообще работает загрузка, насколько детерминированно и хорошо». Rulings
  recorded in `PHASE-D-HOST-OBLIGATIONS.md#rulings-2026-08-02-2`; statuses
  flipped in `run/state/obligations.json`, registry regenerated.

  **State after the sitting** (commands supersede): corpus unchanged
  **11 166 / 210 / 44 — 97.8 %** (no verdict moved — deferral is
  bookkeeping, not repair); registry **102 / 210 — 24 deferred, 78 open;
  owed 54, every one on the sync route**. Still pending the owner:
  **F-217's second anchor** (the hand-copied CLAUDE/AGENTS/GEMINI triple —
  reconciler check vs recorded exception), **F-285's «снять обвинение»**
  (temporal-reuse rule convicted of a simultaneous compile collision), and
  **B-027's sweep rule**. The backlog stands at **B-035**.

  **Third exchange, same sitting — the two halves close, one by a build.**
  The owner ordered the triple check built, algorithmic and never an LLM
  judgement — landed the same hour as `tools/self-check.sh` **step 0c**: a
  full-file byte-compare of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` that
  fails the floor naming the diverging pair (full-file deliberately — the
  `<vibevm>` block is generated identically into all three, so any
  divergence is a hand-edit that missed a sibling). **F-217 → `deferred`**,
  both halves now built or planned. And **F-285 came off** («сними пока.
  Угроза реальная, но пока это не приоритет») — batch **D29** re-judged the
  temporal-reuse anchor `confirmed` (merge 1/1, seal already current, no
  bytes moved); the real half of the threat is precisely B-011's subject,
  already at highest priority. **State after the third exchange** (commands
  supersede): corpus **11 167 / 209 / 44 — 97.8 %**; registry **101 / 209 —
  25 deferred, 76 open; owed 53, every one on the sync route; F-285
  resolved to history (127 total)**. On the owner: **B-027's sweep rule**,
  then the registry's sync-queue remainder.

- **2026-08-02 · четвёртый обмен — the build-first pivot: правила не
  выключаются за неиспользование, система развивается.** The next portion
  (F-154 / F-161 / F-167 / F-181, 15 anchors over four documents) came back
  with a standing re-ruling: for discipline mechanisms the annotate-absence
  default dies — *«Я против чтобы ты выключал правила только потому, что
  они нигде пока не используются … нужно это спроектировать и потом
  построить … Система не заморожена, она должна развиваться»*, and of the
  host's own code: *«похоже на причину по которой нужно всё отрефакторить
  и начать их применять»*. Executed the same sitting: **F-154's five →
  builds B-036 (middle-third position check) / B-037 (REQ-citing custom
  lints: dylint + typescript-eslint) / B-038 (pending rule cards R-060 +
  closed-vocabulary-naming, with the Rust computed-name design question) /
  B-040 (host seam refactor survey), row deferred, texts stay as targets;
  the two TS false-confirmed twins repaired verdict-first (D30 → drift),
  their cross-file cluster split off as F-355** (the generator inherited
  one id into two clusters — instrument defect filed as **B-043**, hand-split
  same day), routed onto B-036/B-037, deferred. **F-161's R-001 pair →
  B-039** (mount FlagSites on the TS gate), row open on its two unruled
  anchors (first-source 74.8-контрадикция + cross-ref fix — re-present).
  **F-167 — «Я в целом согласен», applied as D31** (merge 2/2, seal 1 +
  1 current): the quantities fact now names the instrument and carries the
  owner's far-future note verbatim (**B-042** accepted: measurement corpus
  LLM- or fuzzer-generated, not now); the gopls step names the env override
  first, matching the shipped resolver. **F-181 — option (1):** three
  anchors joined to the F-204/B-005 ancestry family in routing.json, row
  deferred, nothing edited. **B-041 filed** — the owner's development-map
  directive verbatim; the map is the boss lane's next work. Zone mirror
  refreshed before the merges (`progress check` clean, 260 files).

  **State after the fourth exchange** (commands supersede): corpus
  **11 167 / 209 / 44 — 97.8 %**; registry **102 / 209 — 28 deferred, 74
  open; owed 41, every one on the sync route** (the +1 obligation is
  F-355's split, not new work; owed fell 53 → 41 by twelve routings and
  two D31 confirms, offset by the twins' two honest drifts: 53 − 2 + 2
  − 12 = 41). The backlog stands at **B-043**. On the owner: **B-027's sweep rule**, F-161's two
  re-presentations, the F-215/F-281 families, then the registry remainder.

- **2026-08-02 · пятый обмен — карта одобрена и переезжает к бэклогу, а
  рамка кампании восстанавливается словом владельца.** The B-041 map
  drafted on «да, продолжай» (`61db0918`,
  `spec/design/tooling-development-map.md`: four planes, the dependency
  spine, waves А–Г, ten forks, five milestones) came back approved — «Да,
  мне нравится этот документ» — with two directions, both executed the
  same sitting. **(i) Integrate with the backlog** («или положить рядом,
  или сделать отдельным разделом»): the map moved to the repository root
  as **`TOOLING-MAP.md`** beside `BACKLOG.md`, which gained the `#map`
  pointer section with the wave digest; the root `ROADMAP.md` discovered
  in the way is the **product** milestone roadmap (1 071 lines, living
  since April) — untouched, and the map now states in its companion line
  that the two documents do not compete; the `spec/design/` index row
  repoints at the root as the genre's one out-of-directory member.
  **(ii) The frame:** «мы сейчас находимся в процессе более большого
  рефакторинга… нам надо действовать в рамках этого процесса, а то чего
  не хватает — отложить на потом» — recorded verbatim as the map's
  `##frame-line`: the waves are the *shape* of the backlog's drainage,
  the campaign's phases (E/T/F/G after D's exit gate) are the *vehicle*,
  and nothing starts from the map. Pointers repointed everywhere (B-041,
  WAL, CONTINUE, README index); `vibe check` clean over the moved tree.

- **2026-08-02 · шестой обмен — четыре рулинга одним сообщением, и свип
  B-027 наконец бежит.** «1 и 2 — согласен, применяй; 3 — …положить
  куда-нибудь в роадмап и больше не кошмарить меня вопросами "почему нет
  замеров"; 4 — тест на зомби лучше написать». Executed whole:
  **(1)** the TS GUIDE's cross-reference names «Staged ambition» by section
  title, and the 74.8 % figure fell to ATLAS DR2-012's canonical pair
  (75.3 %/70.2 %) in both carriers — the first source's `CONTRADICTION-MAP`
  C-4 and the projection now agree (batch **D33**, 6 confirmed over 5
  files); **F-161's last two anchors confirmed → row deferred (B-039),
  F-284 resolved, F-167 deferred** (its target annotated, its no-zombie
  anchor routed). **(2)** the **B-027 sweep** under the approved rule: 48
  annotated facts inventoried over 16 package files, **19 flipped to
  `@impl/plan` naming their build entry in-text** («Specified, not built
  (→ B-nnn)»), 29 correct as they stood (research-pending and parked
  entries do not count as planned — LEDGER-INTENT's trio and
  RUNTIME-SECURITY stay `@spec/done`), each flip re-judged (**D34**, 19/19
  confirmed, 0 refusals), six files sealed; the host tree carries no
  instances (grep). **(3)** the no-measurements standing answer recorded
  in all three stacks' complete-targets (naming their own bench harness
  and B-042), in `TOOLING-MAP.md` and in B-042 itself — the question is
  closed for good. **(4)** **B-044 filed** («тест на зомби лучше
  написать»; the fractality pod probe is the in-tree pattern) and the
  claim's six copies unified: three false confirmeds repaired
  verdict-first (**D32** — with the per-anchor catch that the ts oracle's
  claim lives in `RUST-SIDE-OWNS-TERMINATION`, not the harvest table's
  `SHUTDOWN-…-EXIT`), five open copies routed onto B-044, **F-281
  deferred**. Mirror before every merge; merge and seal never chained.

  **State after the sixth exchange** (commands supersede): corpus
  **11 169 / 207 / 44 — 97.8 %**; registry **101 / 207 — 31 deferred, 70
  open; owed 34, every one on the sync route; resolved to history 128**.
  The backlog stands at **B-044**; B-027 is `done`. On the owner: one
  mini-question (the rust `LARGE-WORKSPACE` ceiling — 45 s shipped vs
  < 15 s posted vs 60 s asserted; the 60→45 repair in the Go twin's
  already-taken form), then the registry's sync-queue remainder.

- **2026-08-02 · седьмой обмен — потолок «да» (D35), порция
  F-210/F-178/F-199, и OracleRegistry уходит в раскопки вместо правки.**
  The ceiling executed (D35, **F-215 resolved**, 129). The next portion
  went out per the refined format and came back: **F-178 = (1)** —
  **B-045** filed (kind-validation + short names for `uninstall`/`update`
  over the lockfile-first resolver + four mis-cited §2.4 call sites), the
  «stated once» self-description fixed (D36), resolver anchor routed,
  **row deferred, owed 0**; **F-199 = (1)** — the **boot-surface marked
  exception** recorded at PROP-000 `##ATTRIBUTION-BOOT-SURFACE-EXCEPTION`
  (boot surfaces carry the four-rules digest by design; §12 the record;
  the invariant echo names its source; strays are defects), all three
  anchors confirmed-with-exception (D36 ×2, D37), **row resolved whole
  (130 total)**, F-230's debt narrowed to the dead-«§12.1»-pointer half
  (verified dead at HEAD). **F-210 — not applied:** the owner challenged
  the ground and asked what `OracleRegistry` was, why it was deleted,
  whether to bring it back — investigated: the deletion is his own
  **MCP-SOVEREIGNTY** resolution (2026-07-07, four-resolution mandate +
  the `mcp`-kind amendment; commit `36461ba8` deleted `vibe-tcg` whole,
  ~1082 lines that existed only because the server lived outside the
  package slot; the tool grammar stayed normative and the per-family
  servers ship the same tools). The prepared texts rest on that recorded
  resolution + the package's own tests — never on usage-absence — and
  wait for his word. **State** (commands supersede): corpus **11 174 /
  202 / 44 — 97.8 %** (`ai-native` crossed to 98.0 %); registry **99 /
  202 — 32 deferred, 67 open; owed 29; resolved 130**. The backlog
  stands at **B-045**.

- **2026-08-02 · восьмой обмен — история прочитана, ложится
  архитектурное направление, F-210 осушен.** The owner's read of the
  OracleRegistry history came back as a direction, not a revert: MCP
  servers as the *foundation* is the wrong framing — the shared logic
  lives in a library/crate and CLI + MCP are thin surfaces over it («у
  нас постоянно недостаточный уровень абстракции, всё прибивается
  гвоздями»); the mcp packages stay; and the multi-language composition
  story gets planned immediately — autodiscovery over installed
  AI-Native languages, autonomy never broken. Executed: **B-046**
  (composition layer — three options with a recommendation to start
  from the vibe-served discovery roster, the rails being the lockfile's
  `[[mcp_server]]`/`[[binary]]` tables; the autonomy law verbatim) and
  **B-047** (the surface norm + the nailed-down audit; the stacks
  verified already conformant — bridge crates as shared logic, MCP
  descriptions relaying CLI verbs; the host side is the audit's
  subject, B-018's MCP half the first known gap) filed; the map gains
  both in plane 2.4 and fork 11; fork 10 marked taken. **F-210 applied
  as D38 and resolved** — the one-client anchor carries the full
  history with `@impl/plan (→ B-046)`, the goldens anchor the honest
  package-bench annotation `@spec/done`. **State** (commands
  supersede): corpus **11 176 / 200 / 44 — 97.9 %**; registry **98 /
  200 — 32 deferred, 66 open; owed 27; resolved 131**. The backlog
  stands at **B-047**. On the owner: the sync-queue remainder (14 open
  rows), presented per the refined format.

- **2026-08-02 · девятый обмен — последняя порция очереди уходит девятью
  рулингами, и в ней прячется одна стройка.** The nine-document portion
  came back ruled whole: 1/2/3 «согласен», 5 «согласен» (токены — очень
  далёкое будущее), 6/7/8 «пересуд» (+«семёрка»), 9 — the build order
  («хардкод убрать, сделать нормально, недостающую функциональность
  доделать»), 4 — «подумай и вернись рассказать» (returned as the
  schema-home analysis, below). Executed: **the pin build** — vibe-cli
  gained `build.rs` deriving `VIBE_MSRV` from the workspace
  `rust-version` (the cargo-canonical MSRV home the compiler itself
  enforces), the two hard-coded `1.93.0` died into `RUST_PIN`, the
  tools-table test asserts pin-extends-manifest (3/3), and the S6
  lesson synced to the built truth. **Twenty verdicts moved in three
  batches** (D39 verdict-first ×4 — including the fourth floor-gloss
  family member found mid-pass — D40 world ×5 with F-309's two new
  roster anchors judged, D41 ai-native ×10), eleven files sealed, 0
  refusals after the pass met the REAL `vibe progress mirror` (the
  per-file views under `run/mirror/` — the D14 lesson's «mirror» is
  this subcommand, not `progress check`; F-309's new anchors were the
  first anchor-set addition to hit it, and merge-verdicts' refusal
  named it). **Rows resolved: F-114, F-157→(routed set), F-216, F-270,
  F-273, F-275, F-280, F-309 — 138 total.** The redbook manifest now
  carries the standing edition rule (next roster change bumps the
  version). **A bulk deferred-flip of 58 routed-out rows was made and
  reverted within the hour** — `deferred` means owner-ruled; the flip
  conflated boss-side routing records with rulings, bought the gate
  nothing (it reads owed + rulings), and is recorded as the near-miss
  it was. **State** (commands supersede): corpus **11 187 / 191 / 44 —
  97.9 %** over 11 422 (+2 units — the roster anchors); registry
  **91 / 191 — 32 deferred, 59 open; owed 18 = 17 on owner-ruled
  build deferrals + F-279's 1**; resolved **138**; `ai-native`
  **98.3 %**. On the owner: the F-279 schema-home ruling; then the
  exit gate.

- **2026-08-03 · фаза D закрывается — рулинг F-279 исполнен, гейт
  пройден, панель выросла на шаг.** The already-given ruling («вариант
  (а) + отдельный пакет org.vibevm.ai-native/jtd-codegen») executed
  whole, no owner question needed: **the schema moved home**
  (`schemas/specmap.jtd.json` → `core-ai-native/v0.8.0/schemas/`, a
  canonical `specmap.example.json` beside it whose lagging-pin edge
  produces exactly the suspect entry it shows), **both xtask codegen
  routes re-targeted** off the dead `rust-ai-native-lang/v0.5.0` slot
  onto the engine crate — `cargo xtask check-codegen` byte-clean,
  **B-013 closed on all four fix-shape coordinates** (`a6bb261e`);
  the stack README's ships-line tells the post-move truth, re-judged
  **D42: confirmed 1/1, seal 1 — F-279 resolved, 139 to history**
  (`ad3547f1`, `1ac1734e`); the binary-payload fork dissolved on facts
  — the exe was never in git (`tools/.gitignore` law), so
  **`tool:org.vibevm.ai-native/jtd-codegen` ships the provisioning
  recipe, not the binary** (`b4c48aa0`; the package README marked in
  house grammar joined the observed corpus — 261 files, exhaustive
  clean); the host `tools/jtd-codegen/README.md` became a pointer (one
  pin, one home); B-013's backlog row flipped `done` and the map's
  plane 2.3 re-drawn per its supersession rule (`7e1bdf74`). The
  panel's own catch en route: the regenerated engine tree left six
  vendored copies behind — `sync-engines --check` refused on 12
  drift items, the write-through carried the fix to every family
  member (`7441e7e9`), §4.5's «fix landed in one consumer» closed by
  the gate that exists for it. **The exit gate, measured:**
  self-check EXIT=0 (twice — before and after the audit's in-run
  fixes; the second run includes the new step); exhaustive
  `progress check --campaign` clean, 261 files; per-namespace
  summary `host 4552/0/3 — 99.9 %` · `ai-native 2669/27/19 —
  98.3 %` · `world 3967/163/22 — 95.5 %` (src=[1] 269, 6.5 %),
  **ALL 11 188 / 190 / 44 = 11 422 — 98.0 %**; CONVERGENCE **173
  routed / 17 owed of 190 / 0 partial**, the 17 on six deferred rows
  each naming its build and its ruling (the ledger's
  [`#close-2026-08-03`](../../campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#close-2026-08-03)
  table); `routing.json` non-empty; **`baseline.json` written per A6**
  (2 221 units with verdicts). Arithmetic per §7-gate check 3: D42
  carried 1 verdict, drift 191 → 190. **The A–D health-audit
  inventory ran at the gate** (the 2026-08-01 adoption clause): five
  findings — the codegen byte-compare ran in no panel step (**fixed
  in-run, step 6b**, `1218c429` — the checklist's mechanisable-row
  rule applied at discovery), `quinn-proto` RUSTSEC-2026-0185 in two
  lockfiles (**host fixed** `1db359d0` → 0.11.16; the fractality
  specspace's lock **filed as DBT-0023** — its workspace, its
  session), `cargo-outdated` unrunnable over this layout (open), the
  `dead_code`-allow shadow 28 → 79 since June (open, triage next),
  census rows clean (accepted); AUD-0014/0015 found already fixed by
  passing work and closed (`c4e804e0`). **Convergence verdict per
  §7: registry non-empty, nothing new minted — converged with a
  stated remainder** (58 open non-sync rows + 32 owner-ruled
  deferrals, the remainder stated in the ledger's close table).
  Prediction scoring, the countable parts: the geometric fall held
  in substance (601 → 191 drifts, recent regenerations mint zero new
  obligations); «≤3 waves» did not survive contact if a wave is a
  work batch (twelve ran); the escalation count is scored at §9
  against §5's two-consecutive-waves definition, not guessed here.
  State at HEAD `dcc23250`: corpus **98.0 %**, registry **90 / 190 —
  32 deferred, 58 open, owed 17 all ruled**, resolved **139**.
  **Phases E/T/F/G stand designed and do not start without the
  owner's word; Phase E's mandate drains the recorded builds under
  the map's frame.**

- **2026-08-03 · второй обмен — E/T получают переключаемый
  claudez-транспорт субагентов.** The owner's directive: rework the
  machine-local `claudez`/`claudez2` launchers (Claude Code → GLM-5.2
  via the z.ai gateway, two accounts) so `-c` works exactly as with
  plain `claude`, use them as the E/T subagent transport instead of
  native agents, **keep the native↔claudez switch in the owner's
  hands**, effort always max, and parallelise across BOTH launchers
  in worktrees where edits are isolated — one thread where a
  many-place edit would conflict. Executed the same hour: the
  launchers' siamese state was split — before the rework both shared
  one `CLAUDE_CONFIG_DIR` (`~/.claude-glm`) and one override var, so
  `-c` in a shared cwd stole the sibling's thread, fatal for subagent
  use; now `claudez2` owns `~/.claude-glm2` +
  `ZAI_API_TOKEN_FILE_2`/`CLAUDEZ2_CONFIG_DIR`, headers and error
  prefixes de-copy-pasted, and all four variants (bash + PowerShell ×
  two launchers) export `MAX_THINKING_TOKENS=32000` (the effort-max
  lever; `CLAUDEZ_MAX_THINKING` overrides). **Verified — the
  ALPHA/BRAVO matrix, eight runs, exit 0 each:** in one scratch cwd
  claudez seeded codeword ALPHA and claudez2 seeded BRAVO (fresh
  `~/.claude-glm2` bootstrapped headless, second token live), then
  each launcher's `-c -p` returned its OWN codeword — from bash and
  from PowerShell both (`Get-Command` resolves both names to the
  `.ps1` scripts). The instruction landed as
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` + the owner's
  switch `SUBAGENT-MODE.toml` (`mode = "claudez"`; the boss re-reads
  it before every fan-out, so a flip acts immediately); §5-E/§5-T
  executor lines and the T-spec's executor header amended to point at
  it — §13.0.1's «ZCode-class harness, not verified» bet is now a
  verified concrete transport with two non-colliding lanes and a `-c`
  correction loop. Review, verdicts, anchor routing and commits stay
  the boss's in both modes; briefs cite durable files; fractality
  stays out.

- **2026-08-03 · третий обмен — наблюдаемость воркеров и архив логов
  (traceability всего, что происходило).** The owner's directive:
  status of a running claudez worker must be readable every ~30
  seconds, not at the end of a multi-hour task (heartbeat and/or a log
  whose freshness shows when something last happened), and after a
  worker finishes its ENTIRE log is preserved under
  `C:\Users\olegc\git\v\cache\agents` — `unsorted/` for runs bound to
  no task, `sorted/<task-id>/` (id derived from the campaign task's
  anchor) for bound ones — so «мы потом всегда могли понять — откуда
  что произошло». Executed and MEASURED live the same hour: the
  transport's native channel is `--output-format stream-json
  --verbose` — one JSONL line per turn and per tool call, each with a
  wall-clock `timestamp`, written DIRECTLY into the archive path at
  spawn (so «пересохранение» is finalisation, not rescue — no crash
  can lose logged bytes); layer 2 is packet-mandated
  `PROGRESS:`/`TASK-DONE` echo heartbeats. The end-to-end probe (a
  3-step worker, 6 turns, 37.9 s, exit 0, log
  `unsorted/2026-08-03-obs-test-claudez.jsonl`) proved mid-run
  visibility (step-1 heartbeat read while running) and bought two
  weak-writer lessons at full price: **the worker skipped one of
  three mandated heartbeats while working correctly** — so log
  growth is the PRIMARY liveness signal and a missing heartbeat with
  a growing log is not a stall (stall ≈ mtime ≳5 min; GLM turn
  latency legitimately reaches 2–3 min) — and **asked to reply
  exactly `FINISHED` it replied «ЗАВЕРШЕНО» with both artifacts
  correct** — acceptance is by artifacts, never by the final string.
  The instruction gained §5 (the 30-second contract, the status
  one-liner, the archive table, `meta.md` finalisation); the WAL
  constraint carries the contract; `cache/agents/` got its pointer
  README. The boss owns knowing every worker's log path.

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
      campaigns/packages-2026-09 spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md

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

#### Phase D — EXECUTED 2026-07-29 → 2026-08-03; 93 zone commits {#cm-d}

`ef40a1ce`..`dcc23250` (93 commits over the zone perimeter; the zone's
lifetime total stands at 366 of the repository's 563 since the plan).
Opens `6072033a` (601 drifts become 228 obligations, by a script that
says how) and `33bd5b1e`. Landmarks: `d7803b97` the routing record,
without which the phase cannot converge · `8b7f240f` what the host owes
— the other half of the exit gate · `4206c61b` waves 2–4 · `b0a8b0d4`
wave 5 and §3.7 · `1c1a3865` wave 6 · `3dab12a3` wave 7 closes ·
`3c14d6af` wave 8 · `91ebf1fd` the D9 rulings · `fffcb494` the rulings
of 2026-07-31 and the publication runbook · the 2026-08-02 presentation
sitting's ~ten exchanges (D29–D41, builds B-033…B-047, the map, the pin
build, step 0c) · `a6bb261e` the F-279 ruling executed, B-013 closed ·
`b4c48aa0` the jtd-codegen tool package · `7441e7e9` sync-engines
carries the regen to all six vendored copies · `1218c429` the panel
gains the codegen gate · `c4e804e0` the A–D audit at the gate ·
`755d664a`/`dcc23250` the close trio's first two.

*Falsified at the opening, and it killed the obvious plan:* drifts were expected
to cluster by reason text; measured, only **16 texts repeat at all over 54
rows** and text-only clustering returns 552 groups for 601 rows — a reduction of
1.1×. What groups them is the **subject**: one document, one kind of defect, one
edit pass. *Falsified mid-phase, twice:* wave 5 found 18 claimed absences false;
wave 6 found the perimeter blind to a second adopter inside `packages/`.
*State at close (HEAD `dcc23250`):* corpus **11 188 / 190 / 44 = 11 422,
98.0 %**, up from 94.3 % at the Phase C gate; registry **90 obligations /
190 drifts — 32 owner-ruled deferrals, 58 open; 173 routed out with a
recorded determination, 17 still owed and every one on an owner-ruled
build; resolved to history 139**. Reproduce with `tasks/summary.py` and
`tasks/drift-registry.py`; both supersede every figure written here.
*Gate at the boundary:* `self-check` green (the panel one step richer),
exhaustive `progress check` clean over 261 files, `baseline.json`
written per **A6**, the A–D audit section in `AUDIT.md`.

#### Phases D-close, E, T, F, G — PLANNED; subjects spelled in advance {#cm-planned}

*Not yet executed. These are the planned commit sets; the ledger binds real
hashes to them as each phase lands, and any drift between the two is itself a
recorded finding.*

- **D close** — **EXECUTED 2026-08-03, hashes bound:** `755d664a`
  `feat(campaign): the routing record closes, and every survivor
  carries an owner ruling` · `docs(campaign): phase D closes — the
  remainder, and who owns each row` (the commit carrying this very
  entry — the LOG close, the status line, this map) · `dcc23250`
  `chore(campaign): the phase boundary's baseline`. One stated
  deviation from the planned trio: the order ran feat → chore → docs,
  so the docs commit could bind both sibling hashes; and the
  2026-08-01 audit-adoption clause added a fourth commit beside the
  trio — `c4e804e0` `docs(audit)` — plus its two in-run fixes
  (`1218c429`, `1db359d0`), none of which the trio was planned to
  carry.
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

## 8. Deferrals {#deferrals}

The zone file [`campaigns/packages-2026-09/deferrals.md`](../../campaigns/packages-2026-09/deferrals.md)
is the ledger — this section said *(empty)* while that file carried the engine
re-mint's full record, and the D10 pass caught the disagreement (2026-07-31).
One entry today: the `-lang` version-slot re-mint, deferred by owner ruling
2026-07-26 and resolved locally by `vibe update --all` — kept there with the
three conditions that bind if a real re-mint is ever taken up.

## 9. REPORT {#report}

*(empty — filled at close-out against §6)*

## 10. Quick-start for the executing session {#quick-start}

*Added 2026-07-29 by owner ruling: `flow:campaign-plans`'
`##COLD-A-LITERAL-QUICK-START-BLOCK` requires it, the rule is sound, and this
plan had none. Every line prints a number — none of them is quoted from here.*

```sh
python campaigns/packages-2026-09/tasks/summary.py            # verdicts by zone; the campaign's headline
python campaigns/packages-2026-09/tasks/batch-progress.py     # what each phase-C batch owes vs wrote
python campaigns/packages-2026-09/tasks/drift-registry.py     # Phase D: the open obligation registry
python campaigns/packages-2026-09/tasks/drift-registry.py --task F-145   # one obligation, as a SPEC task's §2
bash tools/self-check.sh; echo "EXIT=$?"                      # the gate panel — 0 before anything
```

Then read [`campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md`](../../campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md)
for the phase in flight, and §7's LOG **from the end** for what the last session
did. The registry is generated, never hand-edited: a closure edits the document,
re-judges its anchors through `tasks/merge-verdicts.py --force`, seals with
`vibe progress seal`, and the registry shrinks by exactly that many rows.

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
