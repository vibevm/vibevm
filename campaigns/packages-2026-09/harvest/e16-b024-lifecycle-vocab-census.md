# E-V2-B024 — Census of the two lifecycle vocabularies

**What was measured.** Two parallel ways the project says “what state a spec
chunk is in”: the **specmap** link-map lifecycle status (`planned` / `disputed` /
a claimed `retired`) on a *section*, and the **progress** marker vocabulary
(`@stage/state`, incl. the `void` tombstone) on a *fact*. Measured against the
owner’s fork (2026-08-01): *derive specmap’s lifecycle stages from progress
markers instead of declaring them twice* — with `disputed` the open question
(it has no progress analog).

## Headline

1. **The specmap lifecycle status is a specified-but-unbuilt, uncarried,
   unconsumed vocabulary.** In the root generated `specmap.json`, **0 of 6025**
   `spec_units` carry `status`; **0** carry `disputes`. The parser, the JTD
   schema, and the generated enum all agree on exactly `{planned, disputed}`;
   `retired` exists **only in spec prose**, never in code.
2. **It has exactly one consumer, and that consumer only displays it.**
   `explain.rs:42-58` renders `[PLANNED]` / `[DISPUTED ↔ #anchor]` into a text
   line. Nothing freezes edges, gates, warns, or does coverage math on it —
   `PROP-014:199` says so verbatim (“specified, not built … edges into disputed
   units are not frozen … no coverage math over spec units exists”).
3. **`disputed` has zero live carriers** anywhere (corpus, fixtures, example).
   Its only syntactic occurrence is a synthetic unit test.
4. **The progress side is the opposite: densely carried and heavily consumed.**
   **13 950** markers across the measured perimeter; `void` (the progress
   tombstone) is implemented, rollup-defined, and has a live carrier
   (`PROP-029:43`, retired by B-031).
5. **The derivation the owner wants is cheap and almost lossless for `planned`/
   `retired`; `disputed` is the real fork** — it is the only value neither
   vocabulary can express via the other.

---

## §3.1 — The specmap side

Engine: `…/core-ai-native-specmap/src/` (`mdspec.rs`, `index.rs`,
`generated/specmap/mod.rs`); schema `…/schemas/specmap.jtd.json`.

### 1. The status vocabulary that actually exists

| Source | Accepted status values | `retired`? |
|---|---|---|
| Parser `mdspec.rs:98-115` | `planned`; `disputed(#anchor)` | **no** — unknown-status branch (110-114) names only `planned`/`disputed(#anchor)` |
| Schema `specmap.jtd.json:67-72` | enum `["planned","disputed"]` | **no** |
| Generated enum `generated/specmap/mod.rs:140-146` | `Planned`, `Disputed` | **no** |

**Parser and schema agree exactly — there is no parser-vs-schema divergence to
report.** `retired` is **not in the code on any of the three layers**. It
appears only as *specification prose*: `BROWNFIELD-PROTOCOL-v0.1.xml:94`
(“`req r2 disputed(#other-anchor)` · retired (tombstone)”) and
`PROP-014-specmap-bidirectional-traceability.xml:199` (“`retired` a tombstone”).
`PROP-014:199` itself labels the entire lifecycle-status feature
**“specified, not built.”** The backlog’s `retired` is an aspiration that was
never implemented — see §“Discrepancies with the backlog.”

### 2. How many carriers (root `specmap.json`)

```
total spec_units : 6025
with status      : 0     (0 of 6025)
with disputes    : 0     (0 of 6025)
with kind        : 0
with revision    : 0
```

Not one unit carries a lifecycle status — or even a `kind`/`revision` kind-line.
The host corpus is entirely “legacy-unmarked” units in the generated index.

### 3. Who consumes `status` / `disputes`

Searched the whole tree (excl. `vibedeps/`, `.vibe/`, `target/`, `.wt/`,
`vendor/`) for code that *reads* spec-unit `status`/`disputes` and *acts* on it.

| Site | What it does | Acts on status? |
|---|---|---|
| `explain.rs:42-58` | Renders ` [PLANNED]` / ` [DISPUTED ↔ #d]` into a display string | **display only** — no behaviour change |
| `testgate.rs:40,159` | (false positive) reads `BaselineEntry.status` from `tests-baseline.json` — a *different* `status` field | no — unrelated struct |
| every other `conflicts_with` hit | clap `#[arg(conflicts_with = …)]` — CLI-arg mutual exclusion | no — unrelated concept |

**Consumers that freeze edges / account coverage / gate / warn on spec-unit
status: none. Measured: N=0.** This matches `PROP-014:199` word for word:
suspect detection (`index.rs:118-144`) reads only `revision`/`pinnedR`;
“no coverage math over spec units exists, so `planned` scope is neither
reported separately nor penalized.”

### 4. Where status comes from in the spec text

The kind line is the first non-blank line after an **anchored heading**
(`### Title {#anchor}`), a backticked declaration
(`mdspec.rs:53-59`, parse at `98-115`):

```
`req r1`                 → ratified (status absent)
`req r1 planned`         → Planned
`req r2 disputed(#other)` → Disputed + disputes="other"
```

**Live corpus example: none. Measured: N=0.** No `planned`/`disputed`
kind-line token exists anywhere in the tree — every `planned`/`disputed` hit is
English prose or the terraform campaign discussing this very decision. The only
syntactic exercise of the grammar is the synthetic unit test
`mdspec/tests.rs:47-64` (`kind_line_parses_kind_revision_status`) on a
hand-written 3-unit string.

*Why carriers are 0 (observed, not assumed):* the host corpus writes its
kind-declarations inline on **fact anchors** — `##req-foo \`req r1\` @spec/done`
(33 files) — and the parser honours kind lines only under **anchored headings**,
treating `##<id>` lines as untyped facts (`mdspec.rs:281-293` sets
`kind/status/revision = None`). So the `` `req rN` `` tokens the corpus does
write never reach the specmap `kind`/`status` fields. (A handful of standalone
`` `req r1` `` lines exist under headings in the ai-native package’s own specs,
e.g. `MCP-CORE-v0.1.xml`, `PROP-014` — all `req rN` with no status word, and not
in the host scan.)

---

## §3.2 — The progress-marker side

Engine: `crates/progress-core/src/` (`element.rs`, `model.rs`, `rollup.rs`).

### 1. The exact vocabulary (by code, not memory)

- **Stages** (`model.rs:14-24`; `ALL` `122-130`; `as_str` `132-142`):
  `idea`, `spec`, `impl`, `test`, `doc`, `freeze`, `unknown` — **7**.
- **States** (`model.rs:36-52`; `ALL` `150-156`; `as_str` `158-166`):
  `hold`, `plan`, `work`, `done`, `void` — **5**.
- **Actions** (`model.rs:57-62`): `continue`, `drift`, `rework`, `remove` — 4.
- **Audiences** (`model.rs:67-71`): `user`, `author`, `dev` — 3.
- Shorthand lexer `@stage` / `@stage/state`: `element.rs:212-256`; bare `@stage`
  → state `work`, except `@unknown` → `hold` (`element.rs:246-250`).
- Any value outside these enums is a validation error, never a silent pass-through
  (`element.rs:144-204`, `model.rs:3-4`).

### 2. How `void` is handled

`void` is the progress tombstone — “the unit no longer asserts anything”
(`model.rs:41-52`). The contract is in `rollup_key` (`model.rs:254-281`):
`void` **short-circuits** to `VOID_KEY = (u8::MAX, u8::MAX)` (`model.rs:241`)
*before the stage is consulted*, so it sorts **above every real pair regardless
of stage**.

- **What depends on it:** the worst-of rollup — `rollup_doc` (`rollup.rs:37-44`)
  and `rollup_project` (`rollup.rs:57-63`), both `min_by_key(rollup_key)`.
- **Effect:** a `void` unit does not govern a document that still has live
  units; a document whose every unit is `void` is itself `void` (no special
  case written down). Pinned by property tests `model.rs:346-412` and
  `rollup.rs:343-364`.
- **Live carrier:** `PROP-029-fully-qualified-addresses.xml:43` (`##SCOPE-HOST`,
  retired 2026-08-04 by B-031) carries both `<status stage="spec" state="void">`
  and `@spec/void` — the unambiguous tombstone. (`vibevm/vibespecs/boot/00-core.xml:32` and
  `PROP-043:327` only *describe* the syntax.)

### 3. Corpus frequency (perimeter: `spec/**`, `packages/**` excl.
`vibedeps`/`.vibe`/`target`/`.wt`/`vendor`, root `*.md`)

**Shorthand `@stage/state` — 13 635 total** (`@spec://` foreign directives
correctly skipped: 154). Bare `@stage` (no `/state`, defaulted) folded into its
default bucket; bare counts: `spec` 62, `unknown` 8, `impl` 5, `freeze` 1.

| pair | count | | pair | count |
|---|---:|---|---|---:|
| `@impl/done` | 8622 | | `@unknown/hold` | 8 |
| `@spec/done` | 3933 | | `@spec/void` | 3 |
| `@doc/done` | 657 | | `@test/plan` | 2 |
| `@spec/work` | 324 | | `@spec/hold` | 1 |
| `@impl/plan` | 47 | | `@freeze/work` | 1 |
| `@impl/work` | 13 | | `@idea/plan` | 1 |
| `@doc/work` | 12 | | `@freeze/done` | 11 |

**`<status …/>` elements — 315 total:**

| pair | count | | pair | count |
|---|---:|---|---|---:|
| `spec/done` | 147 | | `freeze/done` | 2 |
| `impl/done` | 97 | | `doc/work` | 2 |
| `doc/done` | 48 | | `test/plan` | 2 |
| `spec/work` | 9 | | `spec/void` | 1 |
| `impl/work` | 4 | | `impl/plan` | 1 |
| | | | `spec/plan` / `idea/work` | 1 each |

**Baskets that matter for derivation:** `plan`-state (specmap `planned` analog)
≈ **54 markers**; `void`-state (specmap `retired` analog) ≈ **4 markers**
(1 unambiguous live tombstone).

---

## §3.3 — Derivation table (the main product)

| specmap status | progress marker combination that expresses it | facts in the basket | what is lost in the derivation |
|---|---|---|---|
| `ratified` (absent default) | any non-`void`, non-`plan` marker whose state ∈ {`done`,`work`,`hold`} at the unit’s stage | the overwhelming majority (~13 900 of 13 950) | nothing specmap-specific — it is the default |
| `planned` | state = **`plan`** (`@impl/plan`, `@test/plan`, `@idea/plan`, `<status …/plan>`) | **~54** | the **stage** collapses — `@impl/plan` and `@spec/plan` both become stage-agnostic `planned` |
| `disputed` | *(no analog — see §3.4)* | **0** | n/a |
| `retired` *(unbuilt in specmap)* | state = **`void`** | **~4** (1 live tombstone: `PROP-029:43`) | the **stage** collapses; `void`’s “split into heirs / cancelled, no replacement” rationale text is lost |

### The trap — section vs fact (measured, not skipped)

A progress marker lives on a **fact** (granularity `item`/`paragraph`/`cell`,
`model.rs:88-97`); a specmap status lives on a **section** (the anchored-heading
unit). Deriving “section = one state” is ambiguous wherever a section’s facts
disagree.

**Measured: 101 sections** (of 2528 marker-bearing heading-spans) carry markers
with **≥ 2 distinct states**.

This is **not** fatal to derivation: progress already defines a deterministic
pick — **worst-of rollup** (`min_by_key(rollup_key)`, `rollup.rs:44`), so a
section with `{@impl/done, @spec/work}` resolves to `spec/work`. The cost is
collapsed granularity: 101 sections fold a spread of states into the single
least-advanced one, and the per-fact disagreement (often the interesting signal
— “this section is half-done”) disappears. A derivation that wants to preserve
it would have to carry the rollup *and* flag the mix.

---

## §3.4 — `disputed`: three outcomes, each with its evidence

1. **Is there any carrier of `disputed`?** **0 live.** Root `specmap.json`
   0/6025; no fixture or `specmap.example.json` carries it; the only syntactic
   occurrence is the synthetic test `mdspec/tests.rs:47-64`. Measured: N=0.
2. **Is there a “conflicting-section pair” mechanism (`conflicts_with`)?**
   **Does not exist.** The `disputes` field (`specmap.jtd.json:73-78`,
   `mod.rs:186-189`) is a free-form string holding the other anchor; nothing
   validates the pair is mutual, and nothing reads it but `explain.rs` display.
   `PROP-014:199`: “the pairing is a unit field and **not yet a spec↔spec
   `conflicts_with` edge**.” Every other `conflicts_with` token in the tree is
   an unrelated clap CLI attribute.
3. **Can progress markers express “оспорено”?** **No.** Enumerated vocabulary
   (`model.rs`): states `{hold, plan, work, done, void}`, actions
   `{continue, drift, rework, remove}` — none denotes “contested by a
   conflicting counterpart.” The nearest, `hold`, means *parked* and carries no
   notion of a counterpart or pairing.

**Three outcomes:**

- **(a) `disputed` stays specmap’s own vocabulary.** Cost: keeps a *second,
  declared* lifecycle vocabulary — the very thing the owner’s “derive, don’t
  declare twice” direction wants to remove. Evidence: 0 carriers (§3.4.1) and 0
  acting consumers (§3.1.3) mean it is pure overhead *today*; but it is the only
  home for the `conflicts_with` pairing concept (§3.4.2), which progress cannot
  express (§3.4.3).
- **(b) add a progress state analog** (e.g. `disputed`/`contested`). Cost:
  enlarges the closed progress vocabulary (`model.rs:36-52`) and forces a
  decision on its place in the rollup order (`rollup_key`, `model.rs:254-281`) —
  a project-wide vocabulary change. Evidence: progress has nothing for it today
  (§3.4.3); a marker is a single `(stage,state)` pair and has no field for the
  *counterpart anchor*, so the pairing would still need a new mechanism.
- **(c) drop `disputed` entirely.** Cost: removes the only address for the
  brownfield principle “contradiction is data” (`BROWNFIELD-PROTOCOL` B3,
  `v0.8.0/…/BROWNFIELD-PROTOCOL-v0.1.xml:17`) and the adjudication workflow
  (supersede / scope-split / stay-open, `:98`). Evidence: 0 live carriers (§3.4.1)
  mean nothing breaks today; what breaks is the *future* workflow the moment a
  real conflict appears.

**Recommendation (one line, with the number):** with **0 carriers and 0 acting
consumers on both sides**, outcome **(a)** — leave `disputed` in specmap as the
single non-derived value — costs nothing today and preserves the one concept
(`conflicts_with` pairing) neither vocabulary can otherwise express; revisit at
the first real disputed unit. *(Decision is the owner’s.)*

---

## Discrepancies with the backlog

1. **`retired` is specified, not built.** The backlog/brownfield prose lists
   `retired (tombstone)` as a specmap unit status
   (`BROWNFIELD-PROTOCOL-v0.1.xml:94`; `PROP-014:199`). The **code disagrees on
   all three layers**: absent from the parser (`mdspec.rs:98-115`), the schema
   enum (`specmap.jtd.json:67-72`), and the generated enum
   (`mod.rs:140-146`). `PROP-014:199` self-documents the whole feature as
   “specified, not built.” This is not a contradiction to smooth over — it is
   the load-bearing fact the census exists to surface: **the specmap lifecycle
   vocabulary is an unbuilt specification with no carriers and no consumers.**
   (If `retired` is ever wanted, the progress side already has the working
   tombstone — `void` — with a defined rollup and a live carrier.)
2. **No parser-vs-schema divergence.** They agree exactly on `{planned,
   disputed}`. Nothing to reconcile there.

---

## Caveats — what this census does NOT measure

- **Frequency instrument.** Counts are a regex mirror of the parser grammar
  (`element.rs:212-256`), not the parser itself. Backtick/quoted *prose
  mentions* of the syntax (e.g. `00-core.xml:32` defining `@spec/void`,
  `PROP-043:327`) are counted as markers, which slightly **over-counts rare
  values** (notably `void`); the dominant buckets (`impl/done`, `spec/done`,
  …) are unaffected. The `void` basket is therefore “≈4, 1 unambiguous live
  tombstone,” not an exact headcount.
- **“Section” in the §3.3 trap** is approximated as a markdown heading span;
  the progress model’s granularity rules (document/section/paragraph/item/cell)
  are richer, so 101 is an *upper-bound* estimate of sections whose facts
  disagree, not a parser-exact figure.
- The ai-native language **twins** (go/ts/rust `crates/vendor/core-ai-native-specmap`)
   were confirmed to be vendored copies of the same engine and were not
   re-measured independently.
- This census measures **feasibility and loss** of deriving specmap status
   from progress markers; it does **not** assess whether doing so is desirable.
   That judgement is the owner’s.
