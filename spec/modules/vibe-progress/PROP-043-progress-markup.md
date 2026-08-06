# PROP-043 — Progress Control: inline status markup and the `vibe progress` tool {#root}

<status stage="impl" state="done" action="continue" actionstage="impl" comment="contract ratified 2026-07-24; the §5 tool ships (all seven subcommands); the campaign it governs is in Phase D — the open tail is the §6 evidence join and the parity items of F-046"/>

@fact:self-uri `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043` @status:spec/done

@fact:status-line **Status:** RATIFIED 2026-07-24 (owner, in session — «ратифицирую PROP-043»)
and **IMPLEMENTED**: the markup language, the tool surface and the data contracts
below are binding, and the §5 tool ships with all seven subcommands (19 specmap
`implements` edges). Its first consumer — the spec-actualization campaign — is in
**Phase D** (stitching); no *(provisional)* sections remain. The known gaps are
tracked as parity items, not as unfinished specification: the §6 evidence join
and the fact-grain specmap consumption. @status:impl/done

@fact:related **Depends on / relates to** (all as *external companions* — see the
separability law, §2): the `addressable-specs` flow (anchors, `spec://` URIs),
[PROP-035](../vibe-workspace/PROP-035-spec-compiler.md) (the document IR whose
Markdown/XML dual-frontend model this markup targets),
[PROP-014 specmap](../../../vibedeps/flow-core-ai-native/0.7.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md)
(spec↔code traceability — consumed through the evidence-provider seam, §6),
[PROP-029](../../common/PROP-029-fully-qualified-addresses.md) (`spec://`
grammar). The first consumer is the spec-actualization campaign
(`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`). @status:spec/done

---

## 1. Motivation {#motivation}

- @fact:memory-accumulated The spec tree is the project's memory, and it has accumulated every stage of
  thought — shipped contracts, half-executed plans, parked ideas, and prose that
  no longer matches the code. @status:spec/done
- @fact:status-lines-freeform Document-level `**Status:**` lines exist (~55 of
  them) but are free-form: fifteen vocabularies, unparseable, and silent below
  document granularity. @status:spec/done

@fact:needs-lead The project needs: @status:spec/done

1. @fact:NEED-MARKUP a **machine-readable markup** that records, inline in the sources, where
   every document / section / paragraph / text fragment stands; @status:impl/done
2. @fact:NEED-TOOL an **algorithmic tool** that reports the state of the whole observed tree
   and enforces exhaustiveness when a campaign demands it; @status:impl/done
3. @fact:NEED-CAMPAIGN-SUBSTRATE a substrate for the **actualization campaign**: mark every claim, verify it
   against the code, and drive the drift down — with the markup remaining
   afterwards as the steering mechanism for further development. @status:impl/work

@fact:KNOWLEDGE-IN-SOURCES The durable knowledge lives **in the markup, in the sources**. Every other
artifact this PROP defines (cache, state projections, baselines) is derived
acceleration, erasable without loss of any fact (§7.5). @status:impl/done

## 2. The separability law {#separability}

@fact:SEPARABILITY-LAW Progress Control is designed as a **standalone product hosted inside vibevm**,
extractable at any moment: @status:impl/done

- @fact:SEP-CORE The **core** (parser, model, rollup, renderers, cache) is its own crate with
  no dependency on vibe-core, vibe-spec, specmap, or any vibevm subsystem. Its
  input is a file tree plus a config; its output is the data contracts of §7. @status:impl/done
- @fact:SEP-ADAPTER The **vibevm adapter** contributes the `vibe progress` CLI surface (§5), the
  `progress.toml` discovery, and the specmap evidence provider (§6). All
  vibevm-specific knowledge lives here. @status:impl/done
- @fact:SEP-NON-COLLISION Non-collision with neighbouring inline grammars (`@spec://`, `#use`,
  `#embed`, `#source`) is a **convention held by tests**, not shared code: the
  core ships fixtures containing those foreign directives and asserts zero
  false matches. @status:impl/done
- @fact:SEP-SELF-CONTAINED-SPECS The specs of this system (this file, its templates, the owner guide) are
  self-contained: they cite neighbouring vibevm systems as external companions
  and never fold their content in. @status:impl/done

@fact:SEP-NEVER **Never** couple the core to a vibevm crate; never let a foreign subsystem
reach into the core's parsing; never split this system's normative text across
other PROPs. @status:impl/done

## 3. The markup language {#markup}

### 3.1 The `<status>` element {#element}

@fact:STATUS-ELEMENT One XML-shaped element, embedded in Markdown (and, later, native in XML
documents — the frontend duality of PROP-035 §5): @status:impl/done

```
<status stage="impl" state="work"/>                       — point marker
<status stage="test" state="plan">wrapped text</status>   — fragment wrapper
```

- @fact:POINT-SELF-CLOSING A **point marker** MUST be self-closing (`/>`). An unclosed `<status …>`
  point form is not well-formed XML and is a `check` error. @status:impl/done
- @fact:FRAGMENT-WRAPPER A **fragment wrapper** is the paired form around text inside a paragraph. @status:impl/done
- @fact:FENCE-AWARE Inside fenced code blocks, inline code spans, and URLs the element and the
  shorthand (§3.7) are **not recognized** — the scanner is fence-aware. @status:impl/done

@fact:DECISION-ELEMENT-NAME **Decision — element name `status`, not `progress`.** @status:impl/done

- @fact:element-name-why **Why:** `<progress>` is an HTML5 element: GitHub-class sanitizers strip it,
  and `html:true` renderers (VS Code preview) draw a literal progress-bar widget
  mid-spec. `status` is not an HTML element and renders inert. @status:spec/done
- @fact:element-name-rejected **Considered and rejected:** `progress` (HTML collision), `vp`/`prg`
  (unreadable), HTML comments (invisible in raw reading, defeating the point). @status:spec/done
- @fact:element-name-revisit **Revisit when:** the XML storage frontend lands and element naming is
  re-grounded in a schema. @status:spec/done

### 3.2 Attributes {#attributes}

| Attribute | Required | Values |
|---|---|---|
| @fact:ROW-ATTR-STAGE `stage` @status:impl/done | yes @status:impl/done | `idea` · `spec` · `impl` · `test` · `doc` · `freeze` · `unknown` @status:impl/done |
| @fact:ROW-ATTR-STATE `state` @status:impl/done | yes @status:impl/done | `plan` · `work` · `done` · `hold` · `void` @status:impl/done |
| @fact:ROW-ATTR-ACTION `action` @status:impl/done | no @status:impl/done | `continue` · `drift` · `rework` · `remove` @status:impl/done |
| @fact:ROW-ATTR-ACTIONSTAGE `actionstage` @status:impl/done | no @status:impl/done | any `stage` value; absent ⇒ the action targets `stage` @status:impl/done |
| @fact:ROW-ATTR-AUDIENCE `audience` @status:impl/done | no @status:impl/done | CSV of `user` · `author` · `dev`; absent ⇒ `dev` @status:impl/done |
| @fact:ROW-ATTR-COMMENT `comment` @status:impl/done | no @status:impl/done | free text @status:impl/done |
| @fact:ROW-ATTR-REF `ref` @status:impl/done | no @status:impl/done | `spec://…` URI, path, or task id (e.g. `DRIFT-012`) @status:impl/done |

- @fact:VOCAB-CLOSED Vocabularies are **closed**. Any value outside the tables is a `check` error
  with a nearest-legal-value hint (typos like `rewrok` die in CI, not in
  review). @status:impl/done
- @fact:VOCAB-AMENDMENT-ONLY New values enter only by amendment to this section. @status:impl/done

@fact:MULTI-MARKERS Multiple markers on one node: at most **one** status marker (stage/state), any
number of **action** markers — a unit may legitimately need
`remove`+`actionstage="doc"` and `continue`+`actionstage="test"` at once. @status:impl/done

### 3.3 Stages {#stages}

- @fact:STAGE-IDEA `idea` — a thought worth keeping, not yet specified. @status:impl/done
- @fact:STAGE-SPEC `spec` — being specified / specified. @status:impl/done
- @fact:STAGE-IMPL `impl` — being implemented / implemented. @status:impl/done
- @fact:STAGE-TEST `test` — being tested / tested. @status:impl/done
- @fact:STAGE-DOC `doc` — being documented / documented. @status:impl/done
- @fact:STAGE-FREEZE `freeze` — freezing as a deliberate process: `freeze/plan` (we intend to
  freeze), `freeze/work` (final checks), `freeze/done` (frozen). @status:impl/done
- @fact:STAGE-UNKNOWN `unknown` — looked at, not understood; explicit triage demand. Distinct from
  *no marker* ("nobody looked yet"). @status:impl/done

@fact:DECISION-FREEZE-STAGE **Decision — `freeze`, not a terminal `done` stage.** @status:impl/done

- @fact:freeze-why **Why:** freezing is a process (planned, executed, and later reversed —
  unfreeze is an ordinary marker change back to `spec`/`idea`/`impl`; history
  lives in git). A `done` stage collided semantically with the `done` state. @status:spec/done
- @fact:freeze-rejected **Considered and rejected:** `stage="done"` (ambiguous against
  `state="done"`); a boolean `frozen` flag (hides the process). @status:spec/done
- @fact:freeze-revisit **Revisit when:** never expected; the cycle-of-improvement premise is core. @status:spec/done

- @fact:STAGE-ORDER **The stage order is fixed for aggregation:**
  `idea < spec < impl < test < doc < freeze`; `unknown` sits outside the order
  and compares below everything. @status:impl/done
- @fact:ORDER-NOT-PROCESS The order is a **sort key for rollup**, not a
  process law: test and doc interleave in reality, and falling back (impl →
  spec, spec → idea) is legal and expressed by simply changing the marker. @status:impl/done

### 3.4 States {#states}

- @fact:STATE-PLAN `plan` — intended, not started. @status:impl/done
- @fact:STATE-WORK `work` — in progress. @status:impl/done
- @fact:STATE-DONE `done` — done for that
  stage. @status:impl/done
- @fact:STATE-HOLD `hold` — deliberately parked (neither worked nor discarded). @status:impl/done
- @fact:STATE-VOID `void` — the unit no longer asserts anything. Named for a **void
  contract** — without effect — deliberately not the programming sense of
  "still works, discouraged". The unit was either split into heirs and left as
  a pointer to them, or cancelled with no replacement; its text survives only so
  its name is not reused and inbound links do not break. It is neither work
  outstanding nor work completed but **no claim at all**, and §3.10 sorts it
  accordingly. Marking one is the author's judgment about their own corpus —
  nothing derives it. @status:impl/done

### 3.5 Actions {#actions}

- @fact:ACTION-CONTINUE `continue` — unfinished; carry on (spawns tasks in the campaign). @status:impl/done
- @fact:ACTION-DRIFT `drift` — diverged from reality; reconcile. Operationally bound to the
  **sync-from-code** flow when the code is right and the spec is stale: the
  marker names the problem, that flow is the procedure. @status:impl/done
- @fact:ACTION-REWORK `rework` — exists but bad; redo (pairs with the feature-flag disable path). @status:impl/done
- @fact:ACTION-REMOVE `remove` — bad or abandoned; delete (or demote to `idea`/`hold` archive). @status:impl/done

@fact:ACTIONSTAGE-NARROWS `actionstage` narrows the target: `action="remove" actionstage="doc"` = "the
documentation of this is to be removed", while `stage` keeps describing the
unit itself. @status:impl/done

### 3.6 Audience {#audience}

- @fact:AUDIENCE-VALUES For whom this promise must eventually be told: `user` (writes specs in their
  own project, installs dependencies; never opens the package internals),
  `author` (builds packages; wants depth), `dev` (vibevm's own developers — the
  spec tree itself serves them; the default). @status:impl/done
- @fact:AUDIENCE-DOC-USE Primary use: `actionstage="doc"`
  markers feed the two guides' tables of contents
  (`vibe progress report --view doc --audience user|author`). @status:spec/done

### 3.7 Shorthand {#shorthand}

@fact:SHORTHAND-FORMS `@status:<stage>/<state>` and `@status:<stage>` are macro-equivalents of a
point marker. The **legacy** spellings `@<stage>/<state>` and `@<stage>` mean
exactly the same and are still read, so a document written before the
qualified form keeps parsing. @status:impl/done

- @fact:SHORTHAND-FULL `@status:test/plan` ⇒ `<status stage="test" state="plan"/>` @status:impl/done
- @fact:SHORTHAND-BARE `@status:impl` ⇒ `<status stage="impl" state="work"/>` — bare shorthand defaults to
  `state="work"`, with exactly one exception: `@status:unknown` ⇒ `state="hold"`.
  (`@status:freeze` ⇒ `freeze/work`: "freezing now".) @status:impl/done

- @fact:SHORTHAND-NAMES-ITS-KEY **Why the key is named.** These documents share the `@` space with
  foreign annotation grammars — JSDoc, TypeScript directives, npm scopes,
  Java annotations, and plain `x@y` addresses. Without a key the reader must
  tell its own tokens from those by a vocabulary of stages and states, which
  answers "is this mine?" with a dictionary lookup instead of with the token's
  own shape. @status:impl/done
- @fact:SHORTHAND-DISAMBIGUATION **Disambiguation against the `@spec://` directive grammar** (in-place spec
  citations) applies to the **legacy** form only: after `@spec` the scanner
  looks ahead — `://` follows ⇒ foreign directive, not ours; `/<state>`,
  whitespace, or end-of-token follows ⇒ shorthand. The qualified form needs no
  lookahead: `@status:spec/done` cannot be confused with `@spec://…`, because
  the key is stated before the value. @status:impl/done
- @fact:SHORTHAND-STANDALONE A shorthand is recognized only as a standalone token at the start
or end of a paragraph's text, never mid-sentence, never inside code or links. @status:impl/done

### 3.8 Placement {#placement}

@fact:placement-lead Six granularities, one rule each — and no ambiguous positions: @status:impl/done

1. @fact:PLACE-DOCUMENT **Document** — marker in the preamble, before the first heading.
   *Pilot amendment (2026-07-24):* a document that **opens with its
   heading** (the standard shape of this repo's specs) has no preamble;
   there, the standalone marker immediately after that first heading is
   the **document** marker, not a section marker. @status:impl/done
2. @fact:PLACE-SECTION **Section** — marker on its own line **immediately after the heading
   line** (for any heading other than a preamble-less file's first one,
   per the amendment above). This is the only legal standalone position
   inside a body. @status:impl/done
3. @fact:PLACE-PARAGRAPH **Paragraph** — marker **inside the paragraph's own text**: the first
   token (right after the newlines, or right after the paragraph's `@fact:<ID>`
   anchor) or the last token (right before them). @status:impl/done
4. @fact:PLACE-LIST-ITEM **List item** *(fact amendment, 2026-07-24 — owner-directed)* — every
   item of a bulleted or numbered list is a **unit of its own**, at every
   nesting level. Its marker — shorthand or XML form alike — sits **inside
   the item's own text**: the first or last token of the item (before any
   nested sub-items, which carry their own markers). @status:impl/done

   - @fact:FACT-ANCHOR-SYNTAX **Fact anchors — the anchored-when-marked law** *(owner, 2026-07-24;
     spelling amended 2026-08-06)*. A stable fact address is written
     `@fact:<ID>` as the **first token** of a paragraph or list item. The
     **legacy** spelling `##<ID>` means the same and is still read. @status:impl/done
   - @fact:FACT-ANCHOR-NAMES-ITS-KEY **Why the key is named.** `##<ID>` was never a heading — an ATX
     heading requires a space after the hashes, and this markup is written
     closed up — but it reads like one to a markdown linter, to a parser
     outside CommonMark, and to the eye. `@fact:` states what the token is
     instead of relying on a reader knowing what it is not. @status:impl/done
   - @fact:FACT-ID-GRAMMAR `<ID>` is
     `[A-Za-z][A-Za-z0-9_-]*`; the unit is then addressable as
     `spec://…/<doc>#<ID>`, sharing one address space with the heading
     `{#anchor}`s — a duplicate across both forms is a `check` error. The
     **address is unchanged by the spelling**: it names the id, never the
     opener. @status:impl/done
   - @fact:ANCHORED-WHEN-MARKED **Every unit that carries a status marker — paragraph or list item —
     MUST also carry a `@fact:<ID>` anchor**; a marked, anchor-less unit is a
     `check` error. @status:impl/done
   - @fact:ANCHOR-MARKER-POSITIONS The marker may stand immediately after the anchor
     (`1. ##RULE-001 rule text @freeze/done` — the owner's canonical
     example shape) or as the unit's last token. @status:impl/done

   - @fact:DECISION-TWO-REGISTERS **Decision — two anchor-id registers (owner ruling, 2026-07-24).**
     `@fact:UPPER-SLUG` names a **normative fact** (a law, rule, carrier,
     changelog entry — content with binding weight); `@fact:kebab-case`
     names a **service unit** (status lines, lead-ins, connective
     prose). @status:impl/done
   - @fact:registers-why **Why:** the register itself carries the normativity
     signal at zero syntax cost; ratified from the PROP-029 re-pilot
     mix reviewed in session. @status:spec/done
   - @fact:registers-rejected **Considered and rejected:** single UPPER
     register (service units become shouty; the signal is lost); single
     kebab register (normative facts stop standing out; the re-pilot
     would need re-anchoring). @status:spec/done
   - @fact:registers-revisit **Revisit when:** the post-campaign fold
     (§3.9) shows the mixed registers confusing report consumers or
     check tooling. @status:spec/done

   - @fact:TABLE-ADDRESSING **Table addressing** *(proposed 2026-07-24, this session — same
     syntax, no new grammar)*. A `@fact:<ID>` as the first token of the
     **first cell of a body row** addresses that **row**
     (`| ##ROW-PKGREF pkgref | … |`); in **any other cell** it addresses
     that **cell**; the **whole table** is addressed by the anchor of its
     lead paragraph (`the target set: ##TBL-MIRRORS`) or its section. @status:impl/done
   - @fact:table-positional-rejected Positional schemes (`r2c3`) are rejected — a row shuffle breaks
     them; a minted id travels with its row. @status:spec/done
   - @fact:CELLS-ANCHOR-EXEMPT Cells stay **exempt from the
     anchored-when-marked obligation** (mint ids only where something
     cites them); a table that is really a list of facts is deconstructed
     into an anchored list instead. @status:impl/done
5. @fact:PLACE-TABLE-CELL **Table cell** *(fact amendment, 2026-07-24)* — every non-empty body
   cell of a table is a **unit of its own**; its marker sits inside the
   cell's text, first or last token. Header rows and the delimiter row are
   structure, not units. A table whose rows are really a list of facts is
   better deconstructed into a list (§3.9); cell markup is for tables whose
   tabular shape is essential. @status:impl/done
6. @fact:PLACE-FRAGMENT **Fragment** — paired `<status>…</status>` around text within a
   paragraph, list item, or cell — the form for an **inline fact** that
   cannot be pulled out of its sentence. @status:impl/done

- @fact:NO-ORPHAN-MARKER A standalone marker between two paragraphs is a **`check` error** — there is
  no "nearest paragraph" heuristic. @status:impl/done
- @fact:SECTION-SPAN A section's span follows the owner-fixed
  IR rule (PROP-035 §5): from its heading to the next heading of the same or
  higher level. @status:impl/done

### 3.9 The granularity doctrine — facts are the campaign grain {#granularity}

- @fact:MAINTENANCE-GRAIN **Anchored units are the maintenance granularity.** Between campaigns,
  markup lives on `{#anchor}`-ed units; reports cite `spec://…#anchor`; this
  is the stable, refactor-proof form. @status:impl/done
- @fact:CAMPAIGN-GRAIN **Facts are the campaign granularity** *(fact amendment, 2026-07-24 —
  owner-directed; supersedes the paragraph grain of the original text)*. An
  actualization campaign demands verbatim exhaustiveness at the grain of
  individual **facts**, because a paragraph routinely carries several and an
  LLM pass silently skips the inner ones. Operationally: @status:impl/done
  - @fact:COUNTABLE-UNITS every paragraph, list item, and non-empty table body cell carries its
    own marker — these are the **countable units** the exhaustive counter
    enforces; @status:impl/done
  - @fact:DECONSTRUCTION-LAW a paragraph that carries **more than one fact is deconstructed** —
    rewritten, sense-preserving and wording-preserving, into a bulleted or
    numbered list with one fact per item, each item marked. Most prose is
    expected to become lists; a paragraph stays prose only when it truly
    carries one fact (or none — connective tissue); @status:impl/done
  - @fact:INLINE-FRAGMENT a fact that cannot leave its sentence (an inline clause, an enumeration
    inside one sentence that resists splitting) is wrapped as a
    **fragment**: `<status …>the fact</status>`; @status:impl/done
  - @fact:FORM-ONLY campaign passes still add missing `{#anchor}`s; deconstruction changes
    *form only* — semantic edits belong to the drift-correction stage,
    never to markup passes. @status:impl/done
- @fact:POST-CAMPAIGN-FOLD After a campaign, density is folded back: a section whose units agree
  collapses to one unit marker (`check` verifies the fold is lossless);
  mixed sections stay fact-marked. @status:spec/done

### 3.10 Inheritance and rollup {#rollup}

- @fact:ROLLUP-DOWNWARD **Downward (defaulting):** a node's marker covers unmarked descendants. @status:impl/done
- @fact:ROLLUP-UPWARD **Upward (aggregation):** an unmarked node's computed status is the
  worst-of its children per the §3.3 order (`unknown` wins the bottom). @status:impl/done
- @fact:ROLLUP-VOID-OUTSIDE `void` is the one value outside the `(stage, state)` order:
  it sorts **above every other pair regardless of stage**, so worst-of never
  returns it while any live unit remains. `worst-of {spec/void, impl/plan}` is
  `impl/plan` — the live part governs and the tombstone's stage does not drag
  the document back to `spec`; `worst-of {done, void}` is `done`. A document
  whose every unit is `void` **is** `void`, which falls out of the same rule
  rather than being special-cased. This is a property of the **pair**: giving
  `void` the top state slot *within* its stage would leave `@spec/void`
  governing by stage anyway, which is why two of the three options originally
  proposed for it could not work. @status:impl/done
- @fact:EXPLICIT-BEATS An explicit marker always beats both directions. Reports show *explicit*
  and *computed* separately — a divergence is information, not noise. @status:impl/done

## 4. Scope configuration — `progress.toml` {#config}

- @fact:CONFIG-FILE Optional dev-mode mechanics, configured by a `progress.toml` at the package
  root (the `clippy.toml` pattern — tool config, not manifest pollution). @status:impl/done
- @fact:INCLUDE-STYLE Include-style globs name what **is** observed (not gitignore-style excludes): @status:impl/done

```toml
schema = 1
include = ["spec/**/*.md", "packages/**/*.md"]   # the default when absent
```

- @fact:DEFAULT-EXCLUDES **Default excludes** (applied always, even under explicit includes),
  in two kinds. **By directory**, matched against any path component:
  `vibedeps/`, `.vibe/`, `refs/`, `fixtures/`, `campaigns/`, `target/`,
  `node_modules/`, `**/vendor/`. **By file name**, matched against the
  basename wherever it sits: `LICENSE.md`. @status:impl/done
- @fact:excludes-rationale Rationale: regenerated dependency copies must
  never carry authored markup (PROP-009's install-never-edits-authored-spec
  law), third-party and test-asserted content is off-limits, and the campaign
  zone (§7.4) is not itself corpus. A licence is the same case one granularity
  down — verbatim text the observing project neither authored nor is the
  source of truth for, replaced wholesale from upstream — so it needed the
  file-name kind rather than a directory. @status:spec/done
- @fact:CONFIG-EXCLUDE **Project-side `exclude`** — an optional list of globs in
  `progress.toml`, matched against the `/`-separated repo-relative path and
  applied **after** the includes and after both default kinds. It exists for
  what an include glob cannot say: *everything under this subtree except these
  named files* — a derived index, a generated projection, anything whose own
  words make a hand edit a defect. §4 is include-style so that nothing is
  observed by accident, and an **enumerated** exclude list serves that purpose
  exactly as well as an enumerated include list; both are explicit and both
  are reviewable. It must not become a wildcard escape hatch, so: a pattern
  matching **no** observed file is reported by name on every subcommand, the
  count of files it removes is printed by `scan`, an invalid glob is a clean
  error naming the pattern, and an absent key behaves as a config that never
  had one. @status:impl/done
- @fact:NESTED-OWNS-SUBTREE A nested package with its own
  `progress.toml` owns its subtree (the host aggregates; it does not reach in) —
  this is how a specspace keeps its own cadence. @status:spec/done

## 5. The tool — `vibe progress` {#tool}

- @fact:TOOL-ADAPTER A subcommand of `vibe` (adapter over the standalone core). @status:impl/done
- @fact:TOOL-OUTPUT-FORMS Native output is
  XML; `--md` renders the table form (source · stage · state · action ·
  comment); `--json` emits the state projections of §7.2. @status:impl/done
- @fact:TOOL-INCREMENTAL All subcommands are
  incremental over the content-hash cache (§7.1). @status:impl/done

- @fact:CMD-SCAN **`scan`** {#scan} — parse the observed tree, build/update the cache and
  state projections. @status:impl/done
- @fact:CMD-CHECK **`check`** {#check} — validation gate: closed vocabularies (with
  nearest-value hints), well-formedness (unclosed point markers), placement
  rules (standalone-between-paragraphs), shorthand collisions, foreign-grammar
  non-collision, lossless folds. `--exhaustive` additionally requires **zero
  unmarked paragraphs** in scope — the campaign gate. Exit codes are stable
  for CI. @status:impl/done
- @fact:CMD-REPORT **`report`** — the tree status: XML native, `--md` table,
  `--json`. Filters: `--view done|todo|qa|remove|doc` (the five resolution
  views: `state=done` · `action=continue` · `stage=test&state=plan|work` ·
  `action=remove` · `actionstage=doc`), `--audience user|author|dev`,
  per-file and whole-project rollups, explicit-vs-computed columns, and the
  evidence column when a provider is wired (§6). @status:impl/done
- @fact:CMD-MIRROR **`mirror`** — materialize the per-file cache view (campaign
  working representation; §7.1) under the campaign zone. @status:impl/done
- @fact:CMD-WEAVE **`weave`** — algorithmic stitch of the observed corpus into one
  document for whole-context LLM loading. `--digest` emits the map form
  (headings + markers + unmarked counts — always fits); `--max-tokens N`
  shards the full form with a shard manifest. **Measured 2026-07-26** on the
  58-file wave-1 corpus: the full weave is **one shard of 1 138 441 bytes**
  (≈ a third of a 1M-token window, so the sharder never had to split) and
  `--digest` is **200 454 bytes**. @status:impl/done
- @fact:CMD-RESCAN **`rescan --baseline <file>`** {#rescan} — the recurrence entry point:
  three-way compare (sources ↔ markers ↔ baseline, §7.3) emitting
  new / changed(suspect) / carried-forward unit lists, plus
  "marker changed outside any campaign" flags. @status:impl/done
- @fact:CMD-BASELINE **`baseline [--out <file>]`** {#baseline} — write the campaign's
  `baseline.json` (§7.3), the file `rescan` consumes. Projects the cache's
  **fact-grain** verdicts onto the **unit** granularity the baseline contract
  is defined at: a fact rolls up into every unit whose span carries it, the
  worst verdict wins (`drift` > `unverifiable` > `confirmed`), evidence is the
  deduplicated union, and the marker snapshot is resolved by the same code path
  `rescan` compares against. It re-verifies nothing and invents no verdict — a
  unit with no judged fact is omitted rather than filled in, so the artifact
  fails toward re-verifying. Default output is `campaigns/<id>/baseline.json`.
  @status:impl/done
- @fact:CMD-SEAL **`seal <path>…`** — record that a file's verdicts hold for
  its **current** text: sets `content_hash` and `campaign.processed_hash` to the
  digest **recomputed from disk**, plus `verified_at`. Same shape as
  `##CMD-GATE` — the caller did the real re-derivation and this records it;
  the command computes, changes and invents no verdict. Reading the cached
  `content_hash` instead of the disk would defeat the purpose, since that field
  is refreshed only by `scan` and between scans compares one stale value with
  another. It **refuses** a file whose markers are not all judged (naming the
  count and the first few), refuses a path the cache does not carry, prints
  what it is vouching for before doing it, and is a no-op with no fresh
  timestamp when the digest already matches. **Its refusal is a *coverage*
  test, not a *recency* one** — the schema carries one date per file and none
  per verdict, so "every marker has a verdict" is checkable and "every verdict
  is fresh" is not; the operator asserting the seal is the real gate (F-075).
  @status:impl/done
- @fact:CMD-GATE **`gate`** {#gate} — record one gate's verdict into the campaign's
  gate panel in `campaign.json`. The automation seam: whoever ran the real
  gate reports the result here, and the dashboard reads it back out. Spawns
  nothing and computes nothing — gates are *recorded*, never run here. @status:impl/done
- @fact:CMD-RESUME **`resume`** {#resume} — render `RESUME.md` from the campaign journal and
  state (operates on the campaign zone when present; a no-op outside one). @status:impl/done

## 6. Evidence providers {#evidence}

- @fact:EVIDENCE-SEAM The core defines a seam: *given a unit, return external facts about it*. @status:impl/done
- @fact:EVIDENCE-SPECMAP The
  vibevm adapter wires **specmap** (PROP-014) into it: `implements` /
  `verifies` / `deviates` edge counts per unit. @status:impl/done
- @fact:EVIDENCE-MISMATCH-FLAGS `report` then flags
  **markup-vs-reality mismatches** — e.g. a unit marked `test/done` with zero
  `verifies` edges, or `freeze/done` on a specmap orphan — and `check` can gate
  on the worst of them. @status:spec/done
- @fact:EVIDENCE-OPTIONAL A project without specmap runs with an empty evidence
  column; nothing in the core knows the provider's shape. @status:impl/done

@fact:VERDICTS-NOT-IN-MARKUP Verification *verdicts* (confirmed / drift / unverifiable) are campaign data
and live in the cache and baseline — **never in the markup** (§7.5). @status:impl/done

@fact:FACT-GRAIN-EVIDENCE *Fact-grain evidence (2026-07-24, owner-directed):* the specmap side
recognises `@fact:<ID>` fact anchors as addressable units (PROP-014 §2.1, the
fact amendment's twin), so `implements`/`verifies` edges land **per fact**
and the provider's mismatch checks apply at the campaign grain, not only
per section. @status:impl/done

## 7. Data contracts {#data}

@fact:DATA-DISCIPLINE All formats are schema-versioned (`"schema": 1`); all writes are atomic
(tmp + rename); the journal is append-only JSONL (a torn tail line is
discarded on read). @status:impl/done

### 7.1 Cache (per-file records) {#cache}

@fact:CACHE-RECORD Per observed file: path, content-hash, extracted markers with positions,
unit/paragraph counts, unmarked count, rollup results; campaign fields when a
campaign is active: verdict per marker (`confirmed` / `drift` /
`unverifiable`), evidence refs, batch id, processed hash. @status:impl/done

@fact:CACHE-TALLY-COMPUTED The per-file **verdict tally is computed on read**, never
stored beside the verdict map it counts (F-077, owner ruling 2026-07-26). A
stored tally is a second statement of the same fact with its own writer, and
this campaign measured three that had gone stale — including one that claimed a
drift row already closed. The map is the source; the count is a view of it. @status:impl/done

### 7.2 State projections (dashboard food) {#state}

- @fact:STATE-FILES `campaign.json` (wave, stage-of-campaign, gates, counters, `updated_at`),
  `corpus.json` (per-file rollups and counts), `findings.json` (the stitching
  obligation ledger), `tasks.json` (both task corpora with statuses),
  `docdebt.json` (harvest cards, doc-coverage). @status:impl/done
- @fact:DASHBOARD-READS-ONLY The dashboard reads **only**
  these; it computes nothing and parses no Markdown ever. @status:impl/done

### 7.3 Baseline (inter-campaign contract) {#baseline}

- @fact:BASELINE-RECORD `baseline.json` — per unit: URI#anchor, unit content-hash at verdict time,
  verdict, evidence refs, date, named crates, marker snapshot. **Shipped:**
  `baseline.rs`'s `BaselineUnit` carries exactly these fields, with
  `Baseline::load`, `Baseline::store` (`baseline/project.rs`), the
  `##CMD-BASELINE` writer and the `rescan` CLI all live. `store` was claimed
  here before it existed and was built to match on 2026-07-26 (F-065); the
  round trip — write the baseline, rescan against it on an unchanged tree —
  is what pins the two halves together. @status:impl/done
- @fact:BASELINE-INVALIDATION Invalidation:
  unit hash changed ⇒ suspect; named crate has commits after the verdict date
  ⇒ suspect; marker diverged from snapshot without a campaign ⇒ flagged;
  otherwise carry-forward (plus a small random control sample, because
  code-side invalidation is deliberately coarse). @status:spec/done

### 7.4 The campaign zone {#campaign-zone}

- @fact:ZONE-LAYOUT `campaigns/<id>/` at the repository root: `baseline.json`, `deferrals.md`,
  `harvest/`, `tasks/`, and the ephemeral `run/` (journal.jsonl, state/,
  RESUME.md, mirror/). @status:impl/done
- @fact:ZONE-EXCLUDED Excluded from markup scope, from packaging, and from
  registries — always. @status:impl/done
- @fact:ZONE-LIFETIMES `run/` is disposable after close-out; the other four
  survive between campaigns. @status:impl/done
- @fact:PROCESS-LAW-ELSEWHERE Process law (journal step protocol, recovery
  rules, RESUME contract) lives in the campaign plan, not here. @status:impl/done

### 7.5 The erasure law {#erasure}

- @fact:ERASURE-LAW Delete every derived artifact — cache, state, journal, mirror, weave — and no
  *fact* is lost: the markup in the sources carries all knowledge. @status:impl/done
- @fact:BASELINE-ACCELERATION The one
  artifact worth keeping anyway is `baseline.json`: not knowledge but
  **acceleration** — its loss returns the next run's cost from O(delta) to
  O(corpus). @status:spec/done

## 8. Parsing rules {#parsing}

- @fact:PARSE-MARKDOWN-FIRST Markdown frontend first: the scanner operates on the document tree
  (headings → units per the PROP-035 §5 body-span rule), then recognizes
  `<status>` elements and shorthand in text nodes only — never inside fenced
  code, inline code, or link targets. @status:impl/done
- @fact:PARSE-COUNTABLE **Countable units** *(fact amendment, 2026-07-24)*: inside a text block
  the scanner recognizes list items (`-` / `*` / `+` and `N.` / `N)`
  lines, with their indented continuation lines, at every nesting level)
  and table rows (`|`-delimited); the units the exhaustive counter walks
  are: plain paragraphs, the lead lines of a block before its first list
  item, each list item, and each non-empty body cell of a table. Header
  and delimiter rows of a table are structure. A marker counts for the
  unit whose text carries it (first/last token of that unit, or a
  fragment wrapper inside it). @status:impl/done
- @fact:PARSE-FACT-ANCHORS **Fact anchors** *(fact amendment, 2026-07-24)*: a `@fact:<ID>` first token
  of a paragraph or list item is that unit's anchor, recorded alongside
  the heading anchors; the scanner enforces the anchored-when-marked law
  (§3.8) — a marked unit with no anchor, and a duplicate id, are `check`
  errors. An opener inside code spans/fences is opaque, as all markup is. @status:impl/done
- @fact:PARSE-XML-GRAMMAR The element grammar is XML: attributes quoted, point markers self-closed.
  A future XML storage frontend consumes the same attribute schema natively;
  the markup language does not change. @status:impl/done
- @fact:PARSE-FOREIGN-OPAQUE Foreign inline grammars (`@spec://`, `#use`, `#embed`, `#source`,
  `<!-- REVIEW: … -->`) are opaque text to this scanner (§2). @status:impl/done

## 9. Genre semantics {#genres}

- @fact:EVERY-GENRE-IN-SCOPE **Every genre is in scope** — contracts, design docs, research, plans, manual
  tests: a design decision the code ignores is first-class drift. @status:impl/done
- @fact:GENRE-TERMINALS Genre only
  changes what the terminal looks like: a contract unit ends at `freeze/done`
  with evidence; a research doc's "implementation" is its downstream deltas; a
  campaign plan's own status line converts to a document marker mechanically. @status:impl/done

## 10. Maintenance discipline {#maintenance}

@fact:maintenance-lead After the first campaign: @status:spec/done

- @fact:EDIT-UPDATES-MARKER **Edit a unit ⇒ update its marker in the same commit.** `vibe progress
  check` sits in the gate panel and yellows on divergence. @status:spec/done
- @fact:TASK-LOOP Task pipelines close the loop: an IMPL task cites markers on entry and
  updates them on exit (`impl/work → impl/done`, then `test/plan`). @status:spec/done
- @fact:FREEZE-NEEDS-EVIDENCE `freeze/done` requires green evidence where a provider exists (§6). @status:spec/done
- @fact:DOC-COVERAGE-RATCHET Doc-coverage (units lacking `documents` edges / doc-view closure) ratchets
  like specmap orphans. @status:spec/done
- @fact:PERIODIC-REVERIFICATION Periodic re-verification runs as a recurring campaign
  (O(delta) via §7.3) and as a health-audit category between runs. @status:spec/done

### 10.1 The life of a fact under an active campaign {#fact-lifecycle}

@fact:LIFECYCLE-WHY **Editing the corpus while a campaign judges it is the normal
case, not an exception** — the campaign exists precisely because the corpus is
being reworked. Three things can happen to a fact, they are not the same thing,
and only one of them announces itself. @status:impl/done

- @fact:LIFECYCLE-EDITED **A judged fact whose text moves comes due for
  re-judgement, and the tooling names it.** The freshness reader compares the
  text a fact was judged against with the text on disk and lists every fact that
  moved, by anchor. This is the case the machinery was built for. @status:impl/done
- @fact:LIFECYCLE-ADDED **A fact added to an already-judged file is unjudged, and
  NOTHING says so.** It does not enter the verdict total, it does not appear in
  any percentage, and no gate fires. It is discovered only by comparing the
  file's addressable anchors against its verdict map — which no shipped command
  prints today. @status:impl/done
- @fact:LIFECYCLE-DELETED **A fact removed from a document leaves its verdict
  behind, and the verdict keeps counting.** The cache is keyed by anchor and
  nothing prunes a key whose anchor is gone. @status:impl/done
- @fact:STALE-IS-NOT-REJUDGE **«The file moved» and «a judged fact moved» are
  different questions, and conflating them wastes the whole point.** A file goes
  stale the moment its bytes change — including when the change only ADDS facts,
  leaving every judged fact untouched. A corpus can carry five stale files and
  zero facts owed re-judgement. Read the per-fact answer, never the per-file
  one. @status:impl/done
- @fact:SEAL-IS-A-WHOLE-FILE-ASSERTION **Sealing refuses a file carrying any
  unjudged marker, and this is correct rather than inconvenient.** Sealing
  asserts that *every* verdict in the file is valid for its current text, so a
  partially-judged file may be left flagged but not vouched for. That refusal is
  the only mechanism today that makes an added fact visible at all. @status:impl/done

### 10.2 Incremental debt clearance {#debt-clearance}

@fact:DEBT-IS-A-LIST-NOT-A-RATIO **The debt is enumerable, so it is paid item by
item and never by re-judging the corpus.** Three enumerable kinds: facts with no
verdict, facts whose text moved, verdicts whose anchor is gone. Each has names
and addresses; none is a percentage to be attacked wholesale. Re-judging
everything would redo work that nothing invalidated. @status:impl/done

@fact:DEBT-UNIT-IS-THE-FILE **The unit of clearance is one file**, because sealing
is a whole-file assertion (`##SEAL-IS-A-WHOLE-FILE-ASSERTION`) — a file is
either clear or flagged, and there is no half-sealed state to leave behind. @status:impl/done

@fact:DEBT-CHEAPEST-IS-THE-FILE-YOU-OPENED **The cheapest debt is in the file you
were going to read anyway.** Judging N facts in one document costs far less than
N facts in N documents, because the reading is shared; a session already editing
a document pays almost nothing to clear that document's backlog in the same
pass. @status:impl/done

@fact:DEBT-PROCEDURE **The procedure, run on demand and never automatically:** @status:impl/done

1. @fact:DEBT-STEP-MEASURE **Measure.** Print the three kinds with the files behind
   them, worst first. @status:spec/plan
2. @fact:DEBT-STEP-PICK **Pick one file** — either the heaviest, or the one this
   session is about to touch anyway. @status:impl/done
3. @fact:DEBT-STEP-JUDGE **Judge only its unjudged facts**, to the ordinary standard
   and clause by clause. A prescriptive fact is judged on coherence and on every
   referent resolving; a descriptive one is checked against the tree. Freshly
   authored text is not exempt from either. @status:impl/done
4. @fact:DEBT-STEP-SEAL **Merge and seal.** A refusal to seal means something in the
   file was missed — that refusal is the check, not an obstacle. @status:impl/done
5. @fact:DEBT-STEP-REPORT **Report how much was cleared**, so the number moves
   visibly rather than silently. @status:impl/done

@fact:DEBT-CLOSING-INCLUDES-JUDGING **Content moved into a specification is judged
in the same pass that moves it.** An unjudged statement in a spec is the same
kind of tail as a dangling citation: the move is not finished until the corpus
knows about what arrived. Without this the standing ruling «significant content
moves into the specifications on closure» manufactures debt at every closure. @status:spec/plan

@fact:DEBT-ASK-AT-SESSION-START **A session reports the debt when it restores
context** (owner ruling 2026-08-06) — one line in the resume report, beside the
gate state and the blockers. Reporting is not paying: the session says what the
debt is and waits, because clearing it is work like any other and its priority
is the owner's. @status:spec/plan

@fact:DEBT-MUST-BE-ASKABLE **The debt is a question the tool answers, not a query
somebody reconstructs** (owner ruling 2026-08-06). «How much debt is there for
the periodic clearance» must be answerable by asking `vibe progress`, in the
same breath as the confirmed/drift figures — three counts and the files behind
them. A number that exists only in a hand-written query is a number nobody
looks at, and this whole subsection describes work that is invisible until it is
printed. **The campaign-side script is a stopgap; the durable home is the
shipped verb.** @status:spec/plan

@fact:DEBT-DO-NOT-JUDGE-BLIND **What must not happen: clearing the count by
judging without evidence.** A verdict written to move a number is the defect
this whole apparatus exists to remove, and it is cheapest to commit exactly when
someone is paying down a backlog. @status:impl/done

## 11. Out of scope / future {#future}

- @fact:FUT-XML-STORAGE XML document storage (arrives with the PROP-035 XML frontend — this markup is
  already native to it); @status:spec/done
- @fact:FUT-SECOND-WAVE second-wave corpora (`packages/org.vibevm.world`,
  `org.vibevm.ai-native`, ~230–250 authored files) and the fractality specspace
  (explicitly excluded from wave 1 by owner decision); @status:spec/done
- @fact:FUT-EXTRACTION extraction into a
  standalone distributable product (the separability law keeps it cheap); @status:spec/done
- @fact:FUT-DASHBOARD dashboard evolution beyond the minimal read-only page. @status:spec/done
- @fact:DASHBOARD-TERM (Terminology note:
  this surface is always called the **dashboard** — never "storefront", a
  term already taken by the vibevm store surface.) @status:impl/done
