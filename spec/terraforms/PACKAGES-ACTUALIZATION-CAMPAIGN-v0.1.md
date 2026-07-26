# Packages-Actualization Campaign v0.1 — wave 2: the authored packages measure themselves {#root}

<status stage="impl" state="work" comment="RATIFIED 2026-07-26 with all six §4.5 amendments adopted; Phase A open"/>

**status: RATIFIED 2026-07-26 · PHASE A OPEN · all six [§4.5](#amendments) amendments adopted · wave 2 of the Progress-Control programme, the sibling of [SPEC-ACTUALIZATION-CAMPAIGN-v0.1](SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md) (wave 1, host `spec/`, closed out 2026-07-26)**

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

## 5. Phases {#phases}

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

*Entry:* per IMPL task, unit stability. Opus executes; Fable reviews.
Wave-2 DRIFT tasks differ from wave 1's in one way that matters: a fix inside
a package's crates must be **vendored forward** to every family member that
copies it (`cargo xtask sync-engines`), and the task's acceptance says so
explicitly or the fix ships to one consumer and not the others.

### Phase T — Test coverage by swarm (owner amendment, 2026-07-26) {#phase-t}

*Entry:* E closed. *Executor:* a swarm of the **running harness's own**
subagents — a deliberate exception to §2's «no fractality», ruled by the owner
2026-07-26 and recorded rather than assumed. *Reviewer:* the boss, per packet.

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

## 8. Deferrals {#deferrals}

*(empty)*

## 9. REPORT {#report}

*(empty — filled at close-out against §6)*
