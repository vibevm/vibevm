# PROP-043 — Progress Control: inline status markup and the `vibe progress` tool {#root}

<status stage="spec" state="done" action="continue" actionstage="impl" comment="contract ratified 2026-07-24; implementation underway (campaign Phase A)"/>

`spec://vibevm/modules/vibe-progress/PROP-043`

**Status:** RATIFIED 2026-07-24 (owner, in session — «ратифицирую PROP-043»).
The markup language, the tool surface, and the data contracts below are binding;
implementation is underway (spec-actualization campaign, Phase A). Sections
marked *(provisional)* are held for the implementation task.

**Depends on / relates to** (all as *external companions* — see the
separability law, §2): the `addressable-specs` flow (anchors, `spec://` URIs),
[PROP-035](../vibe-workspace/PROP-035-spec-compiler.md) (the document IR whose
Markdown/XML dual-frontend model this markup targets),
[PROP-014 specmap](../../../vibedeps/flow-core-ai-native/0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md)
(spec↔code traceability — consumed through the evidence-provider seam, §6),
[PROP-029](../../common/PROP-029-fully-qualified-addresses.md) (`spec://`
grammar). The first consumer is the spec-actualization campaign
(`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`).

---

## 1. Motivation {#motivation}

The spec tree is the project's memory, and it has accumulated every stage of
thought — shipped contracts, half-executed plans, parked ideas, and prose that
no longer matches the code. Document-level `**Status:**` lines exist (~55 of
them) but are free-form: fifteen vocabularies, unparseable, and silent below
document granularity. The project needs:

1. a **machine-readable markup** that records, inline in the sources, where
   every document / section / paragraph / text fragment stands;
2. an **algorithmic tool** that reports the state of the whole observed tree
   and enforces exhaustiveness when a campaign demands it;
3. a substrate for the **actualization campaign**: mark every claim, verify it
   against the code, and drive the drift down — with the markup remaining
   afterwards as the steering mechanism for further development.

The durable knowledge lives **in the markup, in the sources**. Every other
artifact this PROP defines (cache, state projections, baselines) is derived
acceleration, erasable without loss of any fact (§7.5).

## 2. The separability law {#separability}

Progress Control is designed as a **standalone product hosted inside vibevm**,
extractable at any moment:

- The **core** (parser, model, rollup, renderers, cache) is its own crate with
  no dependency on vibe-core, vibe-spec, specmap, or any vibevm subsystem. Its
  input is a file tree plus a config; its output is the data contracts of §7.
- The **vibevm adapter** contributes the `vibe progress` CLI surface (§5), the
  `progress.toml` discovery, and the specmap evidence provider (§6). All
  vibevm-specific knowledge lives here.
- Non-collision with neighbouring inline grammars (`@spec://`, `#use`,
  `#embed`, `#source`) is a **convention held by tests**, not shared code: the
  core ships fixtures containing those foreign directives and asserts zero
  false matches.
- The specs of this system (this file, its templates, the owner guide) are
  self-contained: they cite neighbouring vibevm systems as external companions
  and never fold their content in.

**Never** couple the core to a vibevm crate; never let a foreign subsystem
reach into the core's parsing; never split this system's normative text across
other PROPs.

## 3. The markup language {#markup}

### 3.1 The `<status>` element {#element}

One XML-shaped element, embedded in Markdown (and, later, native in XML
documents — the frontend duality of PROP-035 §5):

```
<status stage="impl" state="work"/>                       — point marker
<status stage="test" state="plan">wrapped text</status>   — fragment wrapper
```

- A **point marker** MUST be self-closing (`/>`). An unclosed `<status …>`
  point form is not well-formed XML and is a `check` error.
- A **fragment wrapper** is the paired form around text inside a paragraph.
- Inside fenced code blocks, inline code spans, and URLs the element and the
  shorthand (§3.7) are **not recognized** — the scanner is fence-aware.

**Decision — element name `status`, not `progress`.**
**Why:** `<progress>` is an HTML5 element: GitHub-class sanitizers strip it,
and `html:true` renderers (VS Code preview) draw a literal progress-bar widget
mid-spec. `status` is not an HTML element and renders inert.
**Considered and rejected:** `progress` (HTML collision), `vp`/`prg`
(unreadable), HTML comments (invisible in raw reading, defeating the point).
**Revisit when:** the XML storage frontend lands and element naming is
re-grounded in a schema.

### 3.2 Attributes {#attributes}

| Attribute | Required | Values |
|---|---|---|
| `stage` | yes | `idea` · `spec` · `impl` · `test` · `doc` · `freeze` · `unknown` |
| `state` | yes | `plan` · `work` · `done` · `hold` |
| `action` | no | `continue` · `drift` · `rework` · `remove` |
| `actionstage` | no | any `stage` value; absent ⇒ the action targets `stage` |
| `audience` | no | CSV of `user` · `author` · `dev`; absent ⇒ `dev` |
| `comment` | no | free text |
| `ref` | no | `spec://…` URI, path, or task id (e.g. `DRIFT-012`) |

Vocabularies are **closed**. Any value outside the tables is a `check` error
with a nearest-legal-value hint (typos like `rewrok` die in CI, not in
review). New values enter only by amendment to this section.

Multiple markers on one node: at most **one** status marker (stage/state), any
number of **action** markers — a unit may legitimately need
`remove`+`actionstage="doc"` and `continue`+`actionstage="test"` at once.

### 3.3 Stages {#stages}

`idea` — a thought worth keeping, not yet specified.
`spec` — being specified / specified.
`impl` — being implemented / implemented.
`test` — being tested / tested.
`doc` — being documented / documented.
`freeze` — freezing as a deliberate process: `freeze/plan` (we intend to
freeze), `freeze/work` (final checks), `freeze/done` (frozen).
`unknown` — looked at, not understood; explicit triage demand. Distinct from
*no marker* ("nobody looked yet").

**Decision — `freeze`, not a terminal `done` stage.**
**Why:** freezing is a process (planned, executed, and later reversed —
unfreeze is an ordinary marker change back to `spec`/`idea`/`impl`; history
lives in git). A `done` stage collided semantically with the `done` state.
**Considered and rejected:** `stage="done"` (ambiguous against
`state="done"`); a boolean `frozen` flag (hides the process).
**Revisit when:** never expected; the cycle-of-improvement premise is core.

**The stage order is fixed for aggregation:**
`idea < spec < impl < test < doc < freeze`; `unknown` sits outside the order
and compares below everything. The order is a **sort key for rollup**, not a
process law: test and doc interleave in reality, and falling back (impl →
spec, spec → idea) is legal and expressed by simply changing the marker.

### 3.4 States {#states}

`plan` — intended, not started. `work` — in progress. `done` — done for that
stage. `hold` — deliberately parked (neither worked nor discarded).

### 3.5 Actions {#actions}

`continue` — unfinished; carry on (spawns tasks in the campaign).
`drift` — diverged from reality; reconcile. Operationally bound to the
**sync-from-code** flow when the code is right and the spec is stale: the
marker names the problem, that flow is the procedure.
`rework` — exists but bad; redo (pairs with the feature-flag disable path).
`remove` — bad or abandoned; delete (or demote to `idea`/`hold` archive).

`actionstage` narrows the target: `action="remove" actionstage="doc"` = "the
documentation of this is to be removed", while `stage` keeps describing the
unit itself.

### 3.6 Audience {#audience}

For whom this promise must eventually be told: `user` (writes specs in their
own project, installs dependencies; never opens the package internals),
`author` (builds packages; wants depth), `dev` (vibevm's own developers — the
spec tree itself serves them; the default). Primary use: `actionstage="doc"`
markers feed the two guides' tables of contents
(`vibe progress report --view doc --audience user|author`).

### 3.7 Shorthand {#shorthand}

`@<stage>/<state>` and `@<stage>` are macro-equivalents of a point marker:

- `@test/plan` ⇒ `<status stage="test" state="plan"/>`
- `@impl` ⇒ `<status stage="impl" state="work"/>` — bare shorthand defaults to
  `state="work"`, with exactly one exception: `@unknown` ⇒ `state="hold"`.
  (`@freeze` ⇒ `freeze/work`: "freezing now".)

**Disambiguation against the `@spec://` directive grammar** (in-place spec
citations): after `@spec` the scanner looks ahead — `://` follows ⇒ foreign
directive, not ours; `/<state>`, whitespace, or end-of-token follows ⇒
shorthand. A shorthand is recognized only as a standalone token at the start
or end of a paragraph's text, never mid-sentence, never inside code or links.

### 3.8 Placement {#placement}

Six granularities, one rule each — and no ambiguous positions:

1. **Document** — marker in the preamble, before the first heading.
   *Pilot amendment (2026-07-24):* a document that **opens with its
   heading** (the standard shape of this repo's specs) has no preamble;
   there, the standalone marker immediately after that first heading is
   the **document** marker, not a section marker.
2. **Section** — marker on its own line **immediately after the heading
   line** (for any heading other than a preamble-less file's first one,
   per the amendment above). This is the only legal standalone position
   inside a body.
3. **Paragraph** — marker **inside the paragraph's own text**: the first
   token (right after the newlines, or right after the paragraph's `##<ID>`
   anchor) or the last token (right before them).
4. **List item** *(fact amendment, 2026-07-24 — owner-directed)* — every
   item of a bulleted or numbered list is a **unit of its own**, at every
   nesting level. Its marker — shorthand or XML form alike — sits **inside
   the item's own text**: the first or last token of the item (before any
   nested sub-items, which carry their own markers).

   **Fact anchors — the anchored-when-marked law** *(owner, 2026-07-24)*.
   A stable fact address is written `##<ID>` as the **first token** of a
   paragraph or list item (double hash — deliberately distinct from the
   single-hash foreign directives `#use` / `#embed` / `#source`, and from
   headings, which require a space after the hashes). `<ID>` is
   `[A-Za-z][A-Za-z0-9_-]*`; the unit is then addressable as
   `spec://…/<doc>#<ID>`, sharing one address space with the heading
   `{#anchor}`s — a duplicate across both forms is a `check` error.
   **Every unit that carries a status marker — paragraph or list item —
   MUST also carry a `##<ID>` anchor**; a marked, anchor-less unit is a
   `check` error. The marker may stand immediately after the anchor
   (`1. ##RULE-001 rule text @freeze/done` — the owner's canonical
   example shape) or as the unit's last token.

   **Decision — two anchor-id registers (owner ruling, 2026-07-24).**
   `##UPPER-SLUG` names a **normative fact** (a law, rule, carrier,
   changelog entry — content with binding weight); `##kebab-case`
   names a **service unit** (status lines, lead-ins, connective
   prose). **Why:** the register itself carries the normativity
   signal at zero syntax cost; ratified from the PROP-029 re-pilot
   mix reviewed in session. **Considered and rejected:** single UPPER
   register (service units become shouty; the signal is lost); single
   kebab register (normative facts stop standing out; the re-pilot
   would need re-anchoring). **Revisit when:** the post-campaign fold
   (§3.9) shows the mixed registers confusing report consumers or
   check tooling.

   **Table addressing** *(proposed 2026-07-24, this session — same
   syntax, no new grammar)*. A `##<ID>` as the first token of the
   **first cell of a body row** addresses that **row**
   (`| ##ROW-PKGREF pkgref | … |`); in **any other cell** it addresses
   that **cell**; the **whole table** is addressed by the anchor of its
   lead paragraph (`the target set: ##TBL-MIRRORS`) or its section.
   Positional schemes (`r2c3`) are rejected — a row shuffle breaks
   them; a minted id travels with its row. Cells stay **exempt from the
   anchored-when-marked obligation** (mint ids only where something
   cites them); a table that is really a list of facts is deconstructed
   into an anchored list instead.
5. **Table cell** *(fact amendment, 2026-07-24)* — every non-empty body
   cell of a table is a **unit of its own**; its marker sits inside the
   cell's text, first or last token. Header rows and the delimiter row are
   structure, not units. A table whose rows are really a list of facts is
   better deconstructed into a list (§3.9); cell markup is for tables whose
   tabular shape is essential.
6. **Fragment** — paired `<status>…</status>` around text within a
   paragraph, list item, or cell — the form for an **inline fact** that
   cannot be pulled out of its sentence.

A standalone marker between two paragraphs is a **`check` error** — there is
no "nearest paragraph" heuristic. A section's span follows the owner-fixed
IR rule (PROP-035 §5): from its heading to the next heading of the same or
higher level.

### 3.9 The granularity doctrine — facts are the campaign grain {#granularity}

- **Anchored units are the maintenance granularity.** Between campaigns,
  markup lives on `{#anchor}`-ed units; reports cite `spec://…#anchor`; this
  is the stable, refactor-proof form.
- **Facts are the campaign granularity** *(fact amendment, 2026-07-24 —
  owner-directed; supersedes the paragraph grain of the original text)*. An
  actualization campaign demands verbatim exhaustiveness at the grain of
  individual **facts**, because a paragraph routinely carries several and an
  LLM pass silently skips the inner ones. Operationally:
  - every paragraph, list item, and non-empty table body cell carries its
    own marker — these are the **countable units** the exhaustive counter
    enforces;
  - a paragraph that carries **more than one fact is deconstructed** —
    rewritten, sense-preserving and wording-preserving, into a bulleted or
    numbered list with one fact per item, each item marked. Most prose is
    expected to become lists; a paragraph stays prose only when it truly
    carries one fact (or none — connective tissue);
  - a fact that cannot leave its sentence (an inline clause, an enumeration
    inside one sentence that resists splitting) is wrapped as a
    **fragment**: `<status …>the fact</status>`;
  - campaign passes still add missing `{#anchor}`s; deconstruction changes
    *form only* — semantic edits belong to the drift-correction stage,
    never to markup passes.
- After a campaign, density is folded back: a section whose units agree
  collapses to one unit marker (`check` verifies the fold is lossless);
  mixed sections stay fact-marked.

### 3.10 Inheritance and rollup {#rollup}

- **Downward (defaulting):** a node's marker covers unmarked descendants.
- **Upward (aggregation):** an unmarked node's computed status is the
  worst-of its children per the §3.3 order (`unknown` wins the bottom).
- An explicit marker always beats both directions. Reports show *explicit*
  and *computed* separately — a divergence is information, not noise.

## 4. Scope configuration — `progress.toml` {#config}

Optional dev-mode mechanics, configured by a `progress.toml` at the package
root (the `clippy.toml` pattern — tool config, not manifest pollution).
Include-style globs name what **is** observed (not gitignore-style excludes):

```toml
schema = 1
include = ["spec/**/*.md", "packages/**/*.md"]   # the default when absent
```

**Default excludes** (applied always, even under explicit includes):
`vibedeps/`, `.vibe/`, `refs/`, `fixtures/`, `campaigns/`, `target/`,
`node_modules/`, `**/vendor/`. Rationale: regenerated dependency copies must
never carry authored markup (PROP-009's install-never-edits-authored-spec
law), third-party and test-asserted content is off-limits, and the campaign
zone (§7.4) is not itself corpus. A nested package with its own
`progress.toml` owns its subtree (the host aggregates; it does not reach in) —
this is how a specspace keeps its own cadence.

## 5. The tool — `vibe progress` {#tool}

A subcommand of `vibe` (adapter over the standalone core). Native output is
XML; `--md` renders the table form (source · stage · state · action ·
comment); `--json` emits the state projections of §7.2. All subcommands are
incremental over the content-hash cache (§7.1).

- **`scan`** {#scan} — parse the observed tree, build/update the cache and
  state projections.
- **`check`** {#check} — validation gate: closed vocabularies (with
  nearest-value hints), well-formedness (unclosed point markers), placement
  rules (standalone-between-paragraphs), shorthand collisions, foreign-grammar
  non-collision, lossless folds. `--exhaustive` additionally requires **zero
  unmarked paragraphs** in scope — the campaign gate. Exit codes are stable
  for CI.
- **`report`** {#report} — the tree status: XML native, `--md` table,
  `--json`. Filters: `--view done|todo|qa|remove|doc` (the five resolution
  views: `state=done` · `action=continue` · `stage=test&state=plan|work` ·
  `action=remove` · `actionstage=doc`), `--audience user|author|dev`,
  per-file and whole-project rollups, explicit-vs-computed columns, and the
  evidence column when a provider is wired (§6).
- **`mirror`** {#mirror} — materialize the per-file cache view (campaign
  working representation; §7.1) under the campaign zone.
- **`weave`** {#weave} — algorithmic stitch of the observed corpus into one
  document for whole-context LLM loading. `--digest` emits the map form
  (headings + markers + unmarked counts — always fits); `--max-tokens N`
  shards the full form with a shard manifest. Wave-1 corpus (~27k lines)
  is expected to fit one modern window nearly whole.
- **`rescan --baseline <file>`** {#rescan} — the recurrence entry point:
  three-way compare (sources ↔ markers ↔ baseline, §7.3) emitting
  new / changed(suspect) / carried-forward unit lists, plus
  "marker changed outside any campaign" flags.
- **`resume`** {#resume} — render `RESUME.md` from the campaign journal and
  state (operates on the campaign zone when present; a no-op outside one).

## 6. Evidence providers {#evidence}

The core defines a seam: *given a unit, return external facts about it*. The
vibevm adapter wires **specmap** (PROP-014) into it: `implements` /
`verifies` / `deviates` edge counts per unit. `report` then flags
**markup-vs-reality mismatches** — e.g. a unit marked `test/done` with zero
`verifies` edges, or `freeze/done` on a specmap orphan — and `check` can gate
on the worst of them. A project without specmap runs with an empty evidence
column; nothing in the core knows the provider's shape.

Verification *verdicts* (confirmed / drift / unverifiable) are campaign data
and live in the cache and baseline — **never in the markup** (§7.5).

*Fact-grain evidence (2026-07-24, owner-directed):* the specmap side
recognises `##<ID>` fact anchors as addressable units (PROP-014 §2.1, the
fact amendment's twin), so `implements`/`verifies` edges land **per fact**
and the provider's mismatch checks apply at the campaign grain, not only
per section.

## 7. Data contracts {#data}

All formats are schema-versioned (`"schema": 1`); all writes are atomic
(tmp + rename); the journal is append-only JSONL (a torn tail line is
discarded on read).

### 7.1 Cache (per-file records) {#cache}

Per observed file: path, content-hash, extracted markers with positions,
unit/paragraph counts, unmarked count, rollup results; campaign fields when a
campaign is active: verdict per marker (`confirmed` / `drift` /
`unverifiable`), evidence refs, batch id, processed hash.

### 7.2 State projections (dashboard food) {#state}

`campaign.json` (wave, stage-of-campaign, gates, counters, `updated_at`),
`corpus.json` (per-file rollups and counts), `findings.json` (the stitching
obligation ledger), `tasks.json` (both task corpora with statuses),
`docdebt.json` (harvest cards, doc-coverage). The dashboard reads **only**
these; it computes nothing and parses no Markdown ever.

### 7.3 Baseline (inter-campaign contract) {#baseline}

`baseline.json` — per unit: URI#anchor, unit content-hash at verdict time,
verdict, evidence refs, date, named crates, marker snapshot. Invalidation:
unit hash changed ⇒ suspect; named crate has commits after the verdict date
⇒ suspect; marker diverged from snapshot without a campaign ⇒ flagged;
otherwise carry-forward (plus a small random control sample, because
code-side invalidation is deliberately coarse).

### 7.4 The campaign zone {#campaign-zone}

`campaigns/<id>/` at the repository root: `baseline.json`, `deferrals.md`,
`harvest/`, `tasks/`, and the ephemeral `run/` (journal.jsonl, state/,
RESUME.md, mirror/). Excluded from markup scope, from packaging, and from
registries — always. `run/` is disposable after close-out; the other four
survive between campaigns. Process law (journal step protocol, recovery
rules, RESUME contract) lives in the campaign plan, not here.

### 7.5 The erasure law {#erasure}

Delete every derived artifact — cache, state, journal, mirror, weave — and no
*fact* is lost: the markup in the sources carries all knowledge. The one
artifact worth keeping anyway is `baseline.json`: not knowledge but
**acceleration** — its loss returns the next run's cost from O(delta) to
O(corpus).

## 8. Parsing rules {#parsing}

- Markdown frontend first: the scanner operates on the document tree
  (headings → units per the PROP-035 §5 body-span rule), then recognizes
  `<status>` elements and shorthand in text nodes only — never inside fenced
  code, inline code, or link targets.
- **Countable units** *(fact amendment, 2026-07-24)*: inside a text block
  the scanner recognizes list items (`-` / `*` / `+` and `N.` / `N)`
  lines, with their indented continuation lines, at every nesting level)
  and table rows (`|`-delimited); the units the exhaustive counter walks
  are: plain paragraphs, the lead lines of a block before its first list
  item, each list item, and each non-empty body cell of a table. Header
  and delimiter rows of a table are structure. A marker counts for the
  unit whose text carries it (first/last token of that unit, or a
  fragment wrapper inside it).
- **Fact anchors** *(fact amendment, 2026-07-24)*: a `##<ID>` first token
  of a paragraph or list item is that unit's anchor, recorded alongside
  the heading anchors; the scanner enforces the anchored-when-marked law
  (§3.8) — a marked unit with no anchor, and a duplicate id, are `check`
  errors. `##` inside code spans/fences is opaque, as all markup is.
- The element grammar is XML: attributes quoted, point markers self-closed.
  A future XML storage frontend consumes the same attribute schema natively;
  the markup language does not change.
- Foreign inline grammars (`@spec://`, `#use`, `#embed`, `#source`,
  `<!-- REVIEW: … -->`) are opaque text to this scanner (§2).

## 9. Genre semantics {#genres}

**Every genre is in scope** — contracts, design docs, research, plans, manual
tests: a design decision the code ignores is first-class drift. Genre only
changes what the terminal looks like: a contract unit ends at `freeze/done`
with evidence; a research doc's "implementation" is its downstream deltas; a
campaign plan's own status line converts to a document marker mechanically.

## 10. Maintenance discipline {#maintenance}

After the first campaign:

- **Edit a unit ⇒ update its marker in the same commit.** `vibe progress
  check` sits in the gate panel and yellows on divergence.
- Task pipelines close the loop: an IMPL task cites markers on entry and
  updates them on exit (`impl/work → impl/done`, then `test/plan`).
- `freeze/done` requires green evidence where a provider exists (§6).
- Doc-coverage (units lacking `documents` edges / doc-view closure) ratchets
  like specmap orphans.
- Periodic re-verification runs as a recurring campaign
  (O(delta) via §7.3) and as a health-audit category between runs.

## 11. Out of scope / future {#future}

XML document storage (arrives with the PROP-035 XML frontend — this markup is
already native to it); second-wave corpora (`packages/org.vibevm.world`,
`org.vibevm.ai-native`, ~230–250 authored files) and the fractality specspace
(explicitly excluded from wave 1 by owner decision); extraction into a
standalone distributable product (the separability law keeps it cheap);
dashboard evolution beyond the minimal read-only page. (Terminology note:
this surface is always called the **dashboard** — never "storefront", a
term already taken by the vibevm store surface.)
