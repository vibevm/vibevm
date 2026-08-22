# PROP-043 — The facts markup: the inline status grammar and its IR {#root}

<status stage="impl" state="done" action="continue" actionstage="impl" comment="contract ratified 2026-07-24; SPLIT 2026-08-22 by the owner's facts/progress boundary ruling — this document is now the universal facts layer (grammar, placement, parsing, genres); the campaign toolchain moved verbatim to PROP-047 (modules/vibe-progress)"/>

@fact:self-uri `spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043` @status:spec/done

@fact:status-line **Status:** RATIFIED 2026-07-24 (owner, in session — «ратифицирую PROP-043»)
and **IMPLEMENTED**: the markup language below is binding. The tool surface,
the data contracts and the campaign consumer moved verbatim to PROP-047 at the
2026-08-22 boundary split (`##BOUNDARY-SPLIT`); their statuses live there. @status:impl/done

@fact:related **Depends on / relates to** (all as *external companions* — see the
separability law, §2): the `addressable-specs` flow (anchors, `spec://` URIs),
[PROP-035](../vibe-workspace/PROP-035-spec-compiler.md) (the document IR whose
Markdown/XML dual-frontend model this markup targets),
[PROP-014 specmap](../../../vibedeps/org.vibevm.ai-native.core-ai-native/0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md)
(spec↔code traceability — consumed by the progress layer through the
evidence-provider seam, PROP-047 §4),
[PROP-029](../../common/PROP-029-fully-qualified-addresses.md) (`spec://`
grammar), and [PROP-047](../vibe-progress/PROP-047-progress-campaigns.md) —
the campaign toolchain built on this grammar. The first consumer was the
spec-actualization campaign
(`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`). @status:spec/done

---

## 1. Motivation {#motivation}

- @fact:memory-accumulated The spec tree is the project's memory, and it has accumulated every stage of
  thought — shipped contracts, half-executed plans, parked ideas, and prose that
  no longer matches the code. @status:spec/done
- @fact:status-lines-freeform Document-level `**Status:**` lines exist (~55 of
  them) but are free-form: fifteen vocabularies, unparseable, and silent below
  document granularity. @status:spec/done

@fact:needs-lead The project needs (the tool and campaign-substrate needs live
in PROP-047 §1 since the boundary split): @status:spec/done

1. @fact:NEED-MARKUP a **machine-readable markup** that records, inline in the sources, where
   every document / section / paragraph / text fragment stands; @status:impl/done

@fact:KNOWLEDGE-IN-SOURCES The durable knowledge lives **in the markup, in the sources**. Every other
artifact this PROP defines (cache, state projections, baselines) is derived
acceleration, erasable without loss of any fact (PROP-047 §5.5, the erasure law). @status:impl/done

## 2. The separability law {#separability}

@fact:SEPARABILITY-LAW The facts machinery is designed as a **standalone layer hosted inside
vibevm**, extractable at any moment: @status:impl/done

- @fact:SEP-CORE The **core** (parser, model, rollup, renderers, cache) is its own crate with
  no dependency on vibe-core, vibe-spec, specmap, or any vibevm subsystem. Its
  input is a file tree plus a config; its output is the parsed model this
  document defines (the campaign data contracts consuming it are PROP-047 §5). @status:impl/done
- @fact:SEP-NON-COLLISION Non-collision with neighbouring inline grammars (`@spec://`, `#use`,
  `#embed`, `#source`) is a **convention held by tests**, not shared code: the
  core ships fixtures containing those foreign directives and asserts zero
  false matches. @status:impl/done
- @fact:SEP-SELF-CONTAINED-SPECS The specs of this layer are self-contained: they cite neighbouring vibevm
  systems as external companions and never fold their content in (the campaign
  templates and the owner guide belong to the progress layer beside PROP-047). @status:impl/done

@fact:SEP-NEVER **Never** couple the core to a vibevm crate; never let a foreign subsystem
reach into the core's parsing; never split ONE layer's normative text across
documents. *Owner-revised 2026-08-22 (`##BOUNDARY-SPLIT`): the original
one-document form of this clause predates the boundary — the system is now TWO
layers with exactly one home each (the facts grammar here, the campaign
toolchain in PROP-047), and the no-split law binds within each home.* @status:impl/done

@fact:BOUNDARY-SPLIT **The facts/progress boundary (owner ruling 2026-08-22, chat,
near-verbatim): «факты — то, поверх чего можно построить совершенно разные
процессы рефакторингов; синтаксис, IR и операции над фактами — в модуль facts,
и это можно показывать сразу; в progress остаются наши инструменты для
рефакторингов vibevm, доделываемые до зрелости».** The law: this document owns
the universal lower layer — grammar (§3), parsing (§4), genre semantics (§5) —
plus the adoption registry beside it (PROP-046); PROP-047 owns the upper layer
— tool, config, evidence, campaign data contracts, maintenance discipline. The
dependency points strictly upward (progress knows facts; facts never knows
progress), and every unit of both documents kept its anchor through the split —
only the doc-paths changed, with the old path carrying a tombstone pointer. @status:spec/done

@fact:BOUNDARY-CLI **The CLI follows the boundary.** The markup lint is a facts
operation: `vibe facts check [--exhaustive]` becomes its durable home, with
`vibe progress check` kept as a transitional alias (printing the new spelling);
the gate panel switches to the facts spelling. Campaign verbs
(`scan`/`mirror`/`seal`/`gate`/`baseline`/`rescan`/`resume`/`weave`/`report`)
stay under `vibe progress`. The same wave repairs B-100: a bare `--campaign
<id>` resolves against `campaigns/<id>` instead of silently minting a
cwd-relative state zone. @status:spec/work

@fact:BOUNDARY-CRATES **Crate names lag the boundary deliberately.** `progress-core`
remains the crate name of the facts core for now — renaming it is a mechanical
ripple (engine copies, sync-engines) with no consumer until the progress
layer's own maturation wave, which is when it happens. The spec boundary and
the CLI boundary are the ones the public sees. @status:spec/done

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
   - @fact:FENCE-IS-AN-EXAMPLE-UNTIL-MARKED **A fenced block is an example, not an assertion** *(owner,
     2026-08-06)*. By default nobody is asked to believe what a fence says
     and no agent is asked to run it. Marking the fact `@fact/code:<ID>`
     makes the fence **part of that fact's body**: the fact then has an
     address, a verdict, and it comes due for re-judgement when the block's
     text moves. @status:impl/done
   - @fact:WHY-A-FENCE-NEEDED-THIS **Why the type exists.** A fence carries no anchor of its own and
     cannot be given one — it is a payload, copied out and pasted elsewhere,
     and an anchor written inside would travel with the copy. Measured over
     this corpus: **372 fenced blocks carry zero facts** while all 7255 text
     blocks carry theirs, so a claim inside a fence belonged to nobody, could
     not be judged, and could not be made stale. Two false statements
     survived exactly that way in one week. @status:impl/done
   - @fact:ONE-OBJECT-TYPE-IS-IMPLEMENTED **The known type set is `code`, and that is a measurement.**
     Fences are the only block kind falling outside fact bodies: the corpus
     holds no images at all, and 891 of 908 table rows and 84 of 96 block
     quotes already sit inside a fact. A type naming a block kind that is
     already covered would address nothing. @status:impl/done
   - @fact:UNKNOWN-OBJECT-TYPE-IS-AN-ERROR **An unknown type is a `check` error**, as are a typed anchor
     that is not its block's last fact and one with no matching block below
     it. Ignoring an unimplemented type would let the grammar promise what it
     cannot do, and the author would learn years later that nothing read it. @status:impl/done
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

## 4. Parsing rules {#parsing}

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
- @fact:PARSE-TYPED-FACT-CODE **Typed fact — the fence joins the body** *(owner ruling variant D
  2026-08-06; built 2026-08-20, B-068)*: `@fact/code:<ID>` is a fact
  definition in the same position and id namespace as `@fact:<ID>`, whose
  body is its own unit **plus the first fenced block after it** (blank
  lines between are fine; any other block breaks the adjacency and is a
  parse error, as is a typed fact that is not the last fact of its text
  block). The attached fence enters the fact's content hash — editing the
  fence stales the fact — while staying opaque to marker/anchor scans, as
  all fences are. `code` is the one implemented type: an unknown type
  (`@fact/<t>:`) is a parse error naming `<t>`, never a silent skip — a
  grammar must not promise what it cannot check. By default a fence
  remains an example belonging to no fact; the typed form is the opt-in
  that turns it into a judgeable assertion. @status:impl/done
- @fact:PARSE-XML-GRAMMAR The element grammar is XML: attributes quoted, point markers self-closed.
  A future XML storage frontend consumes the same attribute schema natively;
  the markup language does not change. @status:impl/done
- @fact:PARSE-FOREIGN-OPAQUE Foreign inline grammars (`@spec://`, `#use`, `#embed`, `#source`,
  `<!-- REVIEW: … -->`) are opaque text to this scanner (§2). @status:impl/done

## 5. Genre semantics {#genres}

- @fact:EVERY-GENRE-IN-SCOPE **Every genre is in scope** — contracts, design docs, research, plans, manual
  tests: a design decision the code ignores is first-class drift. @status:impl/done
- @fact:GENRE-TERMINALS Genre only
  changes what the terminal looks like: a contract unit ends at `freeze/done`
  with evidence; a research doc's "implementation" is its downstream deltas; a
  campaign plan's own status line converts to a document marker mechanically. @status:impl/done

## 6. Out of scope / future {#future}

- @fact:FUT-XML-STORAGE XML document storage (arrives with the PROP-035 XML frontend — this markup is
  already native to it); @status:spec/done
- @fact:FUT-EXTRACTION extraction into a
  standalone distributable product (the separability law keeps it cheap); @status:spec/done