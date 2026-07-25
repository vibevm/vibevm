# Packages-Actualization Campaign v0.1 — wave 2: the authored packages measure themselves {#root}

<status stage="spec" state="done" action="continue" actionstage="impl" comment="authored 2026-07-25 on the owner's directive; awaits ratification and an opening call"/>

**status: AUTHORED 2026-07-25 · NOT STARTED · awaits the owner's ratification and opening call · wave 2 of the Progress-Control programme, the sibling of [SPEC-ACTUALIZATION-CAMPAIGN-v0.1](SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md) (wave 1, host `spec/`)**

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
| `org.vibevm.ai-native` | 10 | 140 | 11 629 | 8 of 10 carry `crates/` |
| **total** | **37** | **294** | **28 733** | |

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
each. These are the review's recommendations; the owner ratifies or strikes
them individually.*

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

1. Widen `progress.toml`; confirm `scan` sees 294 new files and reports them
   all unmarked.
2. **Close the fact-grain specmap gap first.** `core-ai-native` **v0.8.0**
   carries the fact-aware engine (`mdspec.rs` uses `is_valid_fact_id`);
   `rust-ai-native-lang` **v0.7.0** vendors the **v0.7.0** engine, which
   predates the amendment, and that is the version the host consumes. Re-mint
   `rust-ai-native-lang` (and its typescript / go siblings) at v0.8.0 with
   `cargo xtask sync-engines`, publish, bump the host lockfile, regenerate
   `specmap.json`. Only then can Phase C join fact anchors to code.
3. Create the campaign zone; pilot the loop on **three** packages of different
   genres — one prompt-only flow (`wal`), one code-bearing stack
   (`rust-ai-native-lang`), one aggregator (`redbook`).

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

*Exit gate:* 100 % of markers carry verdicts; the X/Y/Z summary recorded in
the LOG — the first measured actuality level of the packages.
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

1. `world` measures higher than `ai-native` (see Phase C).
2. The aggregator genre (`redbook`, `rust-ai-native`, the family umbrellas)
   needs at least one grammar amendment to PROP-043 §3.8.
3. ≥ 1 practice the host claims to follow turns out to be specified by a
   package but enforced nowhere — the profanation the mandate suspects,
   found concretely.
4. The fact-grain specmap re-mint (Phase A2) is the single longest-lead item
   and blocks nothing else once done.
5. Cost is comparable to wave 1 in lines (28.7k vs 26.7k) despite 5× the file
   count — packages are many small files, not few large ones.

## 7. LOG {#log}

*(empty — appended per batch / wave / phase)*

## 8. Deferrals {#deferrals}

*(empty)*

## 9. REPORT {#report}

*(empty — filled at close-out against §6)*
