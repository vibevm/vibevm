# PROP-045: XML spec sources and materialisation targets {#root}

<status stage="spec" state="work" comment="commissioned by the owner 2026-08-21 (mandate quoted in §1 verbatim); DRAFT skeleton authored the same sitting, ahead of the XML-MEASURE integration map — refinement points are named inline and close as the measure and the build land"/>

## 1. Mandate {#mandate}

@fact:MANDATE-VERBATIM The owner's mandate, verbatim (2026-08-21): «XML как
источник спецификаций (вместо Markdown). Везде где пользователь использует
Markdown можно использовать XML. … Можно смешивать XML и Markdown в одном
проекте. При материализации, XML превращается либо в Markdown (с минимальной
деградацией качества — использовать вложенные секции/заголовки, все что
невыразимо в такой форме — не поддерживается), либо в XML рядом с Markdown,
либо в XML. … Формат материализации — настройка пользователя. … Лоадеры
должны корректно обрабатывать все три вида материализации: XML, Markdown,
Mixed (XML + Markdown). … XML как целевой формат материализации должен быть
целевым для разных видов исходников. Всё, даже Markdown, при материализации
превращается в XML. … Markdown материализация все ещё должна поддерживаться,
это важно. … мы целимся в то, что Mixed Input (XML + Markdown) должен
нормально транслироваться в XML (не в mixed!), и это станет в будущем
основным форматом материализации». Acceptance named in the same mandate: a
small test project importing `org.vibevm.world/redbook`, exercised in all
three materialisation modes (XML+MD→XML, XML+MD→Markdown, XML+MD→Mixed),
all well-tested — «это большое изменение». @status:spec/done

@fact:FOUNDATION-ALREADY-RECORDED The direction was pre-recorded before the
mandate: PROP-043 §8 `##PARSE-XML-GRAMMAR» holds that the markup element
grammar is XML and that «a future XML storage frontend consumes the same
attribute schema natively; the markup language does not change». This PROP
builds that frontend and widens it from elements to whole documents.
@status:spec/done

## 2. The shape — one pivot, two frontends, two backends {#shape}

@fact:PIVOT-MODEL **Decision (ADR-part).** There is ONE internal document
model — the pivot — and every format is a frontend (parse into it) or a
backend (emit from it). The pivot is the progress-markup semantic tree the
tree already owns: document → nested sections (the heading hierarchy with
anchors) → blocks (paragraph / list / table / fence / quote) → facts
(anchored units with status and body spans), plus the `<status>` document
element and fragment wrappers. Conversion between formats is always
parse → pivot → emit; there is no direct MD↔XML text rewriting.
Alternatives weighed: per-pair converters (N² growth, drift between pairs)
and a lossless-CST pivot preserving all whitespace (cost without a
consumer; the degradation law below makes semantic-level fidelity the
contract). *Уточни после замера: живёт ли pivot в `progress-core` (его
parse уже строит это дерево) или в новом крейте, который progress-core и
specmap потребляют — реши по фактическим зависимостям обоих движков.*
@status:spec/work

@fact:XML-DIALECT-IS-THE-MD-SUBSET **Decision (ADR-part).** The XML dialect
is deliberately ISOMORPHIC to the Markdown-expressible structure — exactly
the constructs the markup contract names, in XML syntax, and nothing more.
A schema-foreign element or attribute is a loud parse error, never a
silent skip (the same closed-vocabulary law the typed-fact grammar took).
This is what makes the owner's degradation law hold by construction:
XML→MD loses nothing semantic because the dialect cannot express what MD
cannot; «всё невыразимое — не поддерживается» is enforced by the schema,
not by a lossy converter. @status:spec/work

@fact:DIALECT-SKETCH The dialect, first cut (*построй и уточни: точные
имена элементов/атрибутов финализируются с первым золотым корпусом*):
@status:spec/work

```xml
<spec xmlns="https://vibevm.org/spec/1">
  <title>PROP-NNN: …</title>                      <!-- the H1 -->
  <status stage="spec" state="work" comment="…"/> <!-- the existing element, verbatim -->
  <section id="anchor" title="2. The laws">       <!-- Hn nesting = section nesting; {#x} = id -->
    <p>plain prose; inline Markdown conventions ride as literal text</p>
    <p><fact id="NAME" status="impl/done">the fact body, one unit</fact></p>
    <list ordered="false"><item><fact id="N2" status="spec/done">…</fact></item></list>
    <table><tr><td>cells are countable units, as in MD</td></tr></table>
    <fence lang="rust" fact="N3">code; fact= is the @fact/code binding</fence>
    <quote>blockquote unit</quote>
  </section>
</spec>
```

@fact:INLINE-STAYS-MARKDOWN **Decision (ADR-part).** Inline content —
emphasis, inline code, links, `##NAME` citations, `spec://` addresses —
rides INSIDE text nodes as literal Markdown conventions, in both
directions. The pivot does not model inline grammar. Why: the markup
contract already treats inline code as opaque; round-tripping stays
byte-stable at the text level; XML authorship needs no inline vocabulary;
and every consumer that reads fact bodies today keeps reading the same
strings. Alternative weighed — a full inline element vocabulary
(`<code>`, `<a>`, `<b>`) — rejected as cost without a consumer and a
fresh drift surface between two inline grammars. @status:spec/work

@fact:XML-PARSER-DEPENDENCY **Decision (ADR-part): the XML machinery rides
`quick-xml`, one new workspace dependency, pinned.** Measured 2026-08-21:
the tree carries NO xml crate anywhere (Cargo.lock grep across quick-xml /
roxmltree / xml-rs / xmlparser — zero hits), so the frontend/backend need
one. Alternatives weighed: `roxmltree` (a clean read-only DOM — but this
PROP needs an EMITTER as much as a parser, and roxmltree has no writer),
`xml-rs` (both directions but dated and slow), hand-rolling (a parser for
a security-adjacent input format is exactly what one does not hand-roll).
`quick-xml` carries both an event reader and a Writer with correct
escaping, is maintained and widely fuzzed, and its event model fits the
pivot walk. The version is pinned at the S1 landing with the workspace's
usual `workspace = true` discipline. @status:spec/work

## 3. The materialisation setting and the three targets {#materialisation}

@fact:SETTING **The user setting** (*построй: дом и имя по образцу
`[install] slot_integrity` — user-config семья, слои L1/L2*):
`[install] spec_format = "mixed" | "markdown" | "xml"`.
**`mixed` is the introduction default** — every file materialises in its
authored format, which for an all-MD world is byte-for-byte today's
behaviour: the feature lands with zero change for existing projects. The
owner's aim is recorded with it: XML is the intended FUTURE primary, and
the default flips only by the owner's word, never silently.
@status:spec/work

@fact:TARGET-MD **`markdown` target:** XML sources emit as Markdown through
the pivot (nested sections → heading levels, facts → anchored units,
fences/tables/quotes → their MD forms); MD sources copy verbatim. This
target exists for tooling that cannot read XML or mixed trees — named in
the mandate as load-bearing, and it stays supported for as long as this
PROP stands. @status:spec/work

@fact:TARGET-XML **`xml` target:** every source — including Markdown —
emits as dialect XML through the pivot. Mixed input translating into
CLEAN XML (never mixed output) is this target's acceptance bar, because
it is the future primary. @status:spec/work

@fact:TARGET-MIXED **`mixed` target:** copy-through; each file keeps its
authored format. @status:spec/work

@fact:HASH-LAW **The hash law under transformation (ADR-part).** The
lockfile `content_hash` and the machine store keep hashing the SOURCE
form — fetch, store, and identity are untouched by this PROP (PROP-010's
verbatim store stands). A vibedeps slot materialised under `markdown` or
`xml` is a DERIVED artifact: presence-trust still keys on the lockfile
version; the `slot_integrity = verify` spot-check (PROP-011 §2.3, the
P011V seam) applies as built ONLY to `mixed` slots (source-identical
bytes); a transformed slot is honestly `Unverifiable` by source-hash and
takes the pre-spot-check re-materialise discipline. *Уточни после замера:
где именно verify узнаёт формат слота — кандидат: материализация пишет
формат в свой существующий след; если следа нет — минимальный маркер,
описанный здесь же.* @status:spec/work

@fact:BOOT-LANE-SCOPE **Boot-lane law under the setting (revised by the
owner's scenario ruling, 2026-08-21).** Boot artifacts are generated
projections; v1 keeps the COMPILED static artifact (`spec/boot/STATIC.md`)
Markdown regardless of `spec_format` — but both lanes must be fully ready
for a transformed vibedeps tree: the static compiler consumes snippets in
WHATEVER format the slots materialised (an XML-materialised snippet
converts at splice time — pivot conversion, both directions), and the
dynamic lane's entries point at the files as materialised — under
`spec_format = "xml"` a dynamic INCLUDE target IS an `.xml` file, and
`INDEX.md` carries that path honestly. The dynamic ROUTER is the reading
agent itself (PROP-009: boot is pure file-reading; a dynamic entry is an
INCLUDE resolved by the reader), which is why §5a measures agents, not
code. A fully-XML compiled static artifact remains named follow-up work.
@status:spec/work

@fact:SCENARIO-ZERO **The owner's named scenario — acceptance case №0
(ruling 2026-08-21, verbatim: «все спеки в spec написаны в Markdown,
внутренние пакеты в packages — в Markdown, а формат материализации —
XML. Лоадеры и все остальное должны быть к этому готовы. и статические,
и динамические, все»).** Sources are 100 % Markdown (the host `spec/`
tree and every `packages/` member); `spec_format = "xml"`; after
install/materialisation: every vibedeps spec file is dialect XML, the
static lane compiles clean from those XML snippets, the dynamic lane's
INCLUDE targets are `.xml` and resolve, and every reader — `vibe
progress check`, specmap, `vibe check`, `vibe tree», the boot readers —
is green over the result. This is the polygon's primary run; the three
mixed-input runs of §5 ride beside it. @status:spec/work

## 4. Loaders and scanners read all three {#loaders}

@fact:LOADER-LAW Every consumer of spec sources — the progress scanner and
`vibe progress check», the specmap unit parser, `vibe check`, the tree/
boot readers, mirror views — accepts `.md` and `.xml` spec files and a
tree mixing both. XML goes through the XML frontend into the same pivot
the MD frontend feeds, so every downstream mechanism (facts, verdicts,
staleness hashes, unit counts, anchors, specmap units) works unchanged
on either source. *Построй по замеру: точный список интеграционных точек
(`*.md`-фильтры) даёт XML-MEASURE; каждая точка получает парную `.xml»
ветку, ни одна — молча.* @status:spec/work

@fact:ADDRESSING-UNCHANGED `spec://` addressing is format-blind: anchors
are `id` attributes in XML and `{#…}`/first-token anchors in MD, minted
into the same address space; a document's address does not change when
its serialisation does. @status:spec/work

## 5. The redbook polygon — acceptance {#polygon}

@fact:POLYGON A dedicated test project imports `org.vibevm.world/redbook`
(the largest real corpus of house-style markup) plus locally-authored
XML and MD specs, and the suite drives all three targets end-to-end:
**(a)** XML+MD → `xml`: every materialised spec file is dialect XML,
goldens pinned, and the round-trip MD→XML→MD over redbook files is
semantically stable (units, facts, anchors, statuses, tables, fences —
counted equal; the degradation measure); **(b)** XML+MD → `markdown`:
every file is MD, XML-authored sources render with nested headings per
the degradation law; **(c)** XML+MD → `mixed`: byte-identical
copy-through. In all three, the loaders prove themselves: `vibe progress
check` clean, specmap builds, boot regenerates. @status:spec/work

### 5a. The dynamic-router measurement — external agents {#agent-routing}

@fact:AGENT-ROUTER-MEASURE **The hardest part, named by the owner
(2026-08-21): measuring the dynamic routers in EXTERNAL agents.** The
dynamic lane has no code router — the reading agent resolves dynamic
INCLUDEs itself — so readiness for XML targets is an empirical property
of live agent harnesses, not of this repository's code, and it is
MEASURED, not assumed. The instrument is the worker lanes this project
already runs: claudez (a Claude-family agent) and codexrunner (a
GPT-family agent) are external agents by construction. Protocol: the
polygon project (scenario №0 state — XML-materialised tree, honest
`INDEX.md` with static and dynamic entries, at least one `when`-guarded
conditional entry) plus a probe packet that orders a cold worker to
perform the standard boot (read STATIC in full, then every INDEX entry,
resolving dynamic INCLUDEs) and then answer control questions whose
answers exist ONLY inside dynamically-included XML files (one per
dynamic entry, plus one inside a `when`-inactive entry that must NOT be
answered — the negative control). Scoring is by artifact: each answer
cites the file it came from; the measure is answered/missed per lane,
per agent family. The run is repeated over the `markdown» and `mixed»
materialisations of the same tree as the baseline — the DELTA between
XML and MD scores is the finding, not the absolute number. Results are
recorded in the polygon's report and this section's facts flip to
`impl/…` with the measured numbers cited. *Проверь при постройке: probe
не должен подсказывать формат — пакет говорит «выполни бут по
CLAUDE.md/INDEX», не «прочитай XML».* @status:spec/work

@fact:AGENT-ROUTER-MODEL-TIERS **Model tiers are a measurement dimension —
the simpler models go first (owner's advice, verbatim, 2026-08-21: «при
измерении воркеров через Codex я бы советовал попользоваться не моделью
gpt-5.6-sol, а в первую очередь более простыми моделями, чтобы проверить
насколько они вообще справляются с новыми режимами»).** A strong model
masks format friction; the weak model is the sensitive instrument. The
Codex-lane probes therefore run the SIMPLER available tiers first
(`CODEXRUNNER_MODEL`/`CODEXRUNNER_EFFORT` are the launcher's overrides;
the measurement slice enumerates which tiers the installed codex
actually serves and records the list), with the pinned strong tier
(gpt-5.6-sol) run last as the ceiling reference — the per-tier score
table, not one number, is the deliverable. The symmetric extension on
the Claude-family lane (a small slot exists there too) is the builder's
own addition, applied with the same first-simple ordering unless the
owner says otherwise. The lane DEFAULT for work tasks is untouched: this
tiering is the measurement protocol's, not the launcher's.
@status:spec/work

## 6. Build order {#build-order}

@fact:SLICES S1 pivot + XML frontend/backend + golden round-trips over the
redbook corpus → S2 MD backend (XML→MD) + degradation tests → S3 the
setting + transforming materialisation + the hash law → S4 the scanner/
checker/specmap/boot-compiler integration points (the measured `*.md`
list; the static splice learns XML input, INDEX carries materialised
paths) → S5 the redbook polygon E2E: scenario №0 first, then the three
mixed-input targets → S5a the external-agent router measurement (§5a,
claudez + codexrunner over the polygon) → S6 docs, ALPHA-NOTES, judging.
Each slice lands with its own gates; the polygon plus the agent
measurement are the wave's exit. @status:spec/work
