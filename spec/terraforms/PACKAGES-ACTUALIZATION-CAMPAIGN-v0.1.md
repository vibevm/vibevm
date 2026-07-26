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

### Phase F — The credibility report {#phase-f}

The phase wave 1 does not have, and the reason this campaign exists.
One document answering the mandate directly: *does the AI-native discipline
hold itself to its own rule?* It carries the floor / conform / specmap run
output over the discipline's own crates, the measured actuality of both
namespaces, and an explicit verdict on every practice the host claims to
follow. The owner reads it and rules. **A green host floor is not an answer
to this question and may not be cited as one.**

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

## 8. Deferrals {#deferrals}

*(empty)*

## 9. REPORT {#report}

*(empty — filled at close-out against §6)*
