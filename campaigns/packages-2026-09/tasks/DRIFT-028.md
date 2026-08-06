# DRIFT-028 — a unit can be void, and names can be asked about {#root}

<status stage="impl" state="plan" ref="DRIFT-028"/>

**Status:** ready — requested by `vibevm-project`, decided with the owner 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** progress-core (model) + cli (a new verb)
**Unit-stability check:** PROP-043 §3.4 and §3.10 gain anchors, written by the
**reviewer** after this lands (§5). No existing anchor moves.

## 1. Goal {#goal}

A unit that no longer asserts anything can say so, and it stops being counted as
work; and a name can be asked about before it is minted.

## 2. Contract {#contract}

Requested by the downstream **`vibevm-project`** (a development-state tracker
that builds a work graph from the marked corpus). Its problem, in its words:

> Когда единица корпуса нарушает правило «одно утверждение — один факт», её
> приходится разбивать на две. Имя разбитой единицы не переименовывается и не
> переиспользуется, поэтому на её месте остаётся надгробие. … Такие единицы
> сейчас помечают `@spec/done`, и это прямо неверно.

Consequences it names: stage folding counts a tombstone as work, progress
counters are inflated, and garbage collection is impossible because nothing can
be asked "which tombstones does nobody cite any more".

## 3. Current state {#current}

Measured 2026-07-26 — contradict me if a number is wrong:

- `State` (`crates/progress-core/src/model.rs:36-41`) is exactly
  `Hold · Plan · Work · Done`; `State::ALL` (`:139`) is an array of **4**;
  `as_str`/`parse` (`:141-152`) round-trip through it.
- **`rollup_key` (`:220-237`) returns a `(u8, u8)` pair — stage first, then
  state — compared lexicographically.** `Hold=0 < Plan=1 < Work=2 < Done=3`.
- So the state axis only breaks ties *within* a stage, and **stage dominates**.
  That is why "treat a tombstone like `done`" does not work: an
  `@spec/<tombstone>` would still drag a document to `spec` by its stage alone.
- `##ROLLUP-UPWARD` says the computed status is "worst-of its children per the
  §3.3 order", which describes the stage half and not the pair.
- Uniqueness is already a `check` error (§3.8, §8.3: "a duplicate id"), across
  **both** anchor forms — `##<ID>` facts and `{#slug}` headings share one
  namespace.

## 4. Required behavior {#behavior}

### 4.1 `void` — the state, and how it sorts {#void}

Add `State::Void`, spelled `void` — as in a **void contract**: without effect.
Not the programming sense (still works, discouraged); the unit does not operate
at all. It is either split into heirs and left as a pointer to them, or
cancelled with no replacement; the text survives only so the name is not reused
and inbound links do not break.

**`void` sorts above every other `(stage, state)` pair, regardless of stage.**
`rollup_key` short-circuits on it. That is the whole design and it is the
owner's decision, not yours to re-open:

- `worst-of {spec/void, impl/plan}` = `impl/plan` — the live part governs.
- `worst-of {done, void}` = `done`.
- `worst-of {void}` = `void` — a document whose every unit is void **is** void,
  and that falls out rather than being special-cased.

This buys what "exclude it from the rollup" buys without inventing a unit that
is outside the rollup, a concept §3.10 does not have.

**State the invariant as a test, not as a comment:** for every
`Stage::ALL × State::ALL` pair, `rollup_key(s, t) < rollup_key(any, Void)`
unless `t == Void`. That property is the contract; a sentinel constant is only
how it is implemented.

Touch points the requester enumerated and I confirmed: the `State` enum,
`as_str`, `State::ALL` (4 → 5), and `rollup_key`. **Check for anything else
that assumes four states** — a match arm, an array length, a serialized
projection — and report what you found, even if it is nothing.

### 4.2 `vibe progress names` — the live query {#names}

`names` lists every name the observed corpus currently holds; `names --check
<name>…` reports which of the given candidates are taken. Purely algorithmic —
no model, no judgment, no verdicts read.

1. **Both anchor forms, one namespace.** Facts (`##<ID>`) and heading anchors
   (`{#slug}`) collide with each other in `check`, so a registry that reported
   only facts would bless a candidate that collides with a heading. Report both,
   and say which kind each is.
2. **Say what scope uniqueness actually has.** Determine from the code whether
   `check` enforces duplicate-id per file or corpus-wide, make `names` report at
   that same scope, and **state the scope in the output header**. If the two
   disagree the query lies, quietly, to a tool that mints names off it.
3. `--check` exits non-zero if any candidate is taken, so it can gate a script;
   it names each taken one with where it is held.
4. `--json` for machine use.

### 4.3 What this task does NOT do {#not}

- **No heir pointer.** The requester records that with its own inline element,
  which our parser already treats as opaque. Do not model it.
- **No persistent burned-names registry.** While tombstones are kept, the corpus
  *is* the registry — a `void` unit is not deleted, so a live query sees it. The
  persistent artefact becomes necessary only when garbage collection starts
  deleting tombstones, and it should be designed with that GC rather than
  before it. Do not start it here.

Edge cases: `names` on an empty corpus prints a header and nothing else.
`--check` with no candidates is an error naming the flag.

Error paths: unchanged.

## 5. Boundaries {#boundaries}

- **Never edit `spec/**`.** PROP-043 §3.4 gains `##STATE-VOID` and §3.10 gains
  the sorting rule; the **reviewer** writes both under sync-from-code. Put your
  proposed wording in §9.
- Do not change `Stage`, the action vocabulary, or what `check` treats as a
  duplicate. This task adds a state and a query; it changes no existing verdict
  and no existing gate.
- Do not migrate any existing `@spec/done` tombstone to `void`. Which units are
  void is the requester's judgment about its own corpus, not ours to guess.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- progress check --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress names --campaign campaigns/packages-2026-09
```

- The rollup invariant is a **property test** over all `Stage::ALL ×
  State::ALL`, not three hand-picked cases.
- Test the three worked examples in §4.1 by name; report the actual results.
- Round-trip: `State::parse(s.as_str()) == Some(s)` for all five, and the
  existing round-trip tests pick the new variant up through `ALL` — confirm
  they did rather than assuming it.
- `names` over the live corpus: report the total and the fact/heading split.
- `--check` against one name you know is taken and one you know is free:
  report both outputs verbatim and the exit codes.
- **The corpus must be unchanged.** `progress check` clean, ledger still
  **4 490 confirmed / 0 drift / 3 unverifiable** over 275 files. This task adds
  a vocabulary value; it marks nothing.
- Discipline: `cargo fmt --all`, clippy clean, no AI attribution.

## 7. Analogies {#analogies}

`Action` in the same file is the shape for adding a vocabulary value with
`as_str`/`parse`/`ALL`. `commands/progress/seal.rs` (DRIFT-026) is the shape for
a verb that reads the campaign zone without re-parsing the world;
`commands/progress/baseline.rs` is the shape for one that walks the corpus.

## 8. Stop rule {#stop}

If `check`'s duplicate rule turns out to be **per-file** rather than
corpus-wide, `names` cannot honestly answer "is this name taken" for the whole
tree: **STOP after implementing §4.1, report the scope you measured, and return.**
Shipping a corpus-wide query over a per-file rule would hand the requester a
registry that is wrong in exactly the cases it exists for.

Budget signal: past ~5 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable). The requester offered three rollup options and
  asked us to choose; all three were shaped by the belief that the rollup runs
  on state. It runs on the `(stage, state)` pair with stage dominant, which
  breaks two of the three — so the answer is a fourth option they did not list,
  and the owner took it. They also asked for `deprecated` and flagged the
  collision with its programming sense themselves; the owner chose `void`, as
  in a void contract.
- **executed 2026-07-26 — §8 fired. §4.1 landed; §4.2 (`names`) NOT built.**
  The duplicate-id rule is **per-file**, measured rather than assumed:
  `check_anchor_laws` (`crates/progress-core/src/parse/anchors.rs:11-17`) opens
  a fresh `HashMap` per `ParsedDoc`, seeds it only with that document's own
  unit anchors, and is called once per file from `parse_document`
  (`parse/mod.rs:45`); `progress check` (`commands/progress.rs:110-153`) then
  only sums the per-document `doc.issues`. There is no cross-file id registry
  anywhere in the progress path — grepping `DuplicateId` reaches exactly one
  other producer, `vibe-spec`'s merged-view gate (`vibe-spec/src/gate.rs:54`),
  which is a different subsystem and also runs over a *single* effective
  document. So a corpus-wide `names` would have answered "free" for names that
  are only free in some other file — wrong in exactly the cases the query
  exists for — and §8's stop applies verbatim.

### 9.1 Proposed PROP-043 wording, for the reviewer's sync pass {#wording}

Not written by this task (§5 keeps `spec/**` off-limits). Offered as drafts.

**§3.4, new anchor `##STATE-VOID`:**

> @fact:STATE-VOID `void` — the unit no longer asserts anything. Named for a
> **void contract**: without effect, not the programming sense of "still
> works, discouraged". The unit was either split into heirs and left as a
> pointer to them, or cancelled with no replacement; its text survives only so
> that its name is not reused and inbound links do not break. A `void` unit is
> not work outstanding and not work completed — it is no claim at all, and
> §3.10 sorts it accordingly. Marking one is the author's judgment about their
> own corpus; nothing derives it.

**§3.10, appended to the rollup order:**

> `void` is the one value outside the `(stage, state)` order: it sorts **above
> every other pair regardless of stage**, so worst-of never returns it while any
> live unit remains. `worst-of {spec/void, impl/plan}` is `impl/plan` — the live
> part governs, and the tombstone's stage does not drag the document back to
> `spec`. `worst-of {done, void}` is `done`. A document whose every unit is
> `void` **is** `void`, which falls out of the same rule rather than being a
> special case. This is a property of the pair and not of the state axis alone:
> giving `void` the top state slot *within* its stage would leave `@spec/void`
> still governing by its stage.
