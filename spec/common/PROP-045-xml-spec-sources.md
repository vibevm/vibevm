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
contract). **Resolved by the XML-MEASURE map (2026-08-21): there is no
single Markdown frontend to widen — FOUR independent families read
different MD subsets today (progress-core's scanner, the vendored
specmap engine's mdspec, the boot/tree directive readers, vibe-check's
point scanners), with real dialect drift already between them (fence
grammar run-matching vs prefix-toggling). The pivot is therefore a NEW
shared crate — `vibe-specdoc` — owning the document IR and both
frontends/backends; host consumers converge on it. The vendored specmap
engine is engine-workspace territory (sync-engines law): its XML
frontend is built in the AUTHORED engine workspace and写-throughs as its
own slice (S4b), never patched in the vendored copy.* @status:spec/work

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

@fact:NAMED-SECTION-ELEMENTS **Decision (ADR-part; owner ruling 2026-08-22,
verbatim): «Гораздо логичней `<three-bands title=\"…\">`. … Вся суть XML
нотации в том, что у тебя названия тэгов несут названия сущностей, это
упрощает работу нейросети».** A section serialises with its ANCHOR as the
element name — `<three-bands title="1. The three bands">` — because the
dialect's first reader is an agent, and a tag that names its entity is
self-describing where an endless `<section>` river is not. The generic
form `<section id="…" title="…">` remains in the dialect as the REQUIRED
fallback for the two cases XML itself forbids or the grammar reserves: an
anchor that is not a valid XML name (leading digit) and an anchor
colliding with the structural vocabulary (`spec`,`title`,`status`,
`section`,`p`,`fact`,`list`,`item`,`table`,`tr`,`td`,`fence`,`quote») —
measured over the live corpus, that tail is 2 anchors of 1393; the
emitter writes the named form everywhere else, the readers accept both.
The converter recipe bumps (`specdoc/1` → `specdoc/2`), so every
transformed slot re-materialises by the derived-manifest law rather than
lingering in the old shape. The owner's next call arrived the same
day — facts follow, see `##NAMED-FACT-ELEMENTS`. @status:spec/work

@fact:NAMED-FACT-ELEMENTS **Decision (ADR-part; owner ruling 2026-08-22,
verbatim): «сконвертируй и факты тоже. Предлагаю такой формат
`<fact-name fact="true" ...>`. Таким образом кастомный XML-парсер всегда
может найти соответствующие элементы».** A fact serialises with its ID
as the element name, carrying the DISCRIMINATOR attribute —
`<THE-LAW fact="true" status="impl/done">body</THE-LAW>` — so a reader
that knows nothing of the vocabulary still finds every fact by one
attribute test. The recognition law: an element IS a fact iff its name
is `fact` (the generic form, which stays in the dialect) or it carries
`fact="true"`. The named form is emitted whenever the id passes the
same elementability predicate sections use (fact-id grammar already
forbids leading digits, so the fallback tail is vocabulary collisions
only); the typed-fact fence binding stays by id and does not change.
The owner's second clause binds the scanners: the progress machinery
must work when a fact's SOURCE — not a materialised copy — is authored
XML; the host lane holds by construction (XML sources enter progress
through the canonical MD projection) and is PINNED by explicit tests
(an observed .xml source scans unit-for-unit equal to its MD twin),
while the specmap engine's native reader learns the named form
mirror-wise. The converter recipe bumps again (`specdoc/2` →
`specdoc/3`); the host re-materialises once, after both shapes land.
The owner's third clause (2026-08-22, same sitting) binds the boot
lanes: «статические и динамические лоадеры должны хорошо работать с
новым синтаксисом фактов» — pinned at the transition's landing by (a)
the static-splice determinism test running over a NAMED-shape snippet
whose projected facts survive into STATIC, (b) the vibe-spec
normal-closure byte-equality test running over BOTH serialisations
(generic and named) of one dependency, and (c) the polygon re-run at
specdoc/3, whose control package auto-adopts the named shape through
to_xml — INDEX targets, STATIC splice and every machine loader then
exercise the final syntax end-to-end; the agent half of the dynamic
router is §5a's measurement, deliberately run AFTER this transition so
it measures the shape that ships. @status:spec/work

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

@fact:XML-PARSER-DEPTH **The heavyweight question, answered on the record
(owner's challenge, 2026-08-21 — the Java instinct «сразу брать мощный
навороченный парсер»).** The Java-world case for a Xerces-class engine is
the SOAP-era grammar surface: DTD, XSD, XInclude, catalogs. This dialect
OUTLAWS that surface by construction — a closed ~13-element vocabulary
where a foreign construct is a loud error, no DTD, no external entities —
so a heavyweight would buy capability this contract forbids, while its
Rust incarnation (libxml2 FFI bindings) would pay a C build dependency
and libxml2's CVE record inside a security-adjacent input path. Three
recorded consequences: **(a)** the XXE attack class dies by construction
(quick-xml does not process DTD — here a feature); **(b)** validation is
OUR closed-vocabulary walk with contract-citing errors, not a schema
engine's; **(c)** conformance is proven on OUR documents — the golden
corpus and round-trip property tests over redbook — not assumed from the
parser's reputation. **Escape hatch, explicit:** the frontend is one
module behind the pivot seam; if the reader shows conformance holes, it
swaps to `roxmltree` (the ecosystem's conformance-strongest read-only
DOM) while the quick-xml Writer stays — a bounded change that alters no
consumer. quick-xml itself is the ecosystem's de-facto standard (the
most-used XML crate on crates.io by a wide margin; calamine, the RSS
stack and docx readers ride it), not a first-hit pick. @status:spec/work

## 3. The materialisation setting and the three targets {#materialisation}

@fact:SETTING **The setting and its home (revised by the measure —
reproducibility rules).** The materialisation format is a REPRODUCIBLE
project property, so its canonical home is the project manifest:
`vibe.toml [project] spec_format = "mixed" | "markdown" | "xml"`, with
the effective value recorded where derived state lives (the derived
manifest below) so two machines materialise identically. The user-config
family supplies only the operator DEFAULT for projects that do not pin
one (`[install] spec_format`, beside `slot_integrity`), per the standing
precedence CLI > env > project > user > built-in; vibe-settings is
barred from this key by PROP-040's own boundary (app prefs never extend
vibe.toml). **`mixed` is the built-in default** — for an all-MD world
byte-for-byte today's behaviour; the flip of the default to XML is the
owner's word, never silent. @status:spec/work

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

@fact:HASH-LAW **The hash law under transformation (ADR-part, upgraded by
the measure).** Source identity is untouched: the lockfile
`content_hash` and the machine store keep hashing the SOURCE form
(PROP-010's verbatim store and collision law stand on source bytes). A
transformed slot is a DERIVED artifact with its own recorded identity:
materialisation under `markdown`/`xml` runs a VERSIONED deterministic
converter and writes a **derived manifest** into the slot —
`(source_hash, output_format, converter_recipe, derived_hash)` — where
`derived_hash` is the standard content-hash recipe over the transformed
tree. The P011V spot-check then stays MEANINGFUL for every format:
`mixed` slots verify against the lock source hash as built; transformed
slots verify against their derived manifest's `derived_hash`, and a
missing/mismatched derived manifest (or a converter_recipe the binary no
longer carries) is `Diverged` → honest re-materialise. The slot's
freshness decision includes the output format: changing `spec_format`
re-materialises even though `(kind, name, version)` did not move.
Semantic equivalence of source and derivative is the converter's own
proof through the shared IR (the golden corpus), never the hash's job.
@status:spec/work

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

@fact:PROJECTION-READ **Decision (ADR-part): scanners read XML through its
canonical Markdown projection — one dispatch layer above the parser, no
dependency cycle.** `vibe-specdoc` depends on `progress-core` (its MD
frontend is the adapter), so progress-core cannot itself call specdoc.
The consumers dispatch instead: a `.xml` spec entering any scanner
(progress, check, specmap-host, show) is first projected
`from_xml → to_markdown` — deterministic and canonical by S1's emitter —
and the projection feeds the existing MD machinery; units, facts,
anchors, hashes and verdict staleness all work unchanged, and a source
edit moves the projection exactly when it moves meaning. Alternatives
weighed: a native XML unit-walker in every scanner (a fifth and sixth
parser family — the disease the measure named), and inverting the crate
dependency (progress-core consuming specdoc — a cycle). RECORDED
DEGRADATION, honest: a diagnostic for an XML source cites
projection-relative line numbers, and v1 marks such diagnostics with
the projection notice rather than pretending; native source positions
are follow-up work riding the specmap-engine slice (S4b).
@status:spec/work

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

### 5b. Questions the build surfaced — dispositions {#surfaced}

@fact:REVIEW-COMMENTS-LAW **REVIEW markers and comments (S4 finding,
disposed).** The dialect legally SKIPS XML comments as layout and the
pivot deliberately drops them, so a projection loses `<!-- REVIEW: -->`
markers. The law therefore is: comment-consuming readers (review aging,
managed-block scanners) read RAW SOURCE TEXT of both forms — the comment
syntax is shared by MD and XML — and never the projection; S4 built
review aging exactly so, with source-relative lines. A comment-carrying
pivot was weighed and refused: comments are the one construct whose
whole point is to be invisible to the document model. @status:impl/done

@fact:NORMAL-FORMAT-RESIDUE **The `normal` boot format over XML slots is
named residue (S4 finding, open).** `compile_normal_entry` reads its
closure through vibe-spec's own MD section source, which the projection
cannot feed without touching that crate; `simple` snippets and authored
boot files carry XML materialisation fully today. The residue rides the
S4b family (the engine/vibe-spec lane), recorded here so scenario №0's
polygon states honestly which snippet formats its packages use.
@status:spec/work

@fact:BOOT-ORIGIN-LITERAL-MATCH **`show effective`'s boot-origin match is
logical-document keyed (S4 finding, closed by S5).** The literal-name
match silently degraded a transformed snippet's origin to `user»; the
S5 landing keys the match on the logical stem (`10-flow-wal.md` and
`10-flow-wal.xml` are one contribution), pinned by the polygon's
origin test — a snippet materialised into `.xml` reports its package.
@status:impl/done

@fact:GENERATED-ARTIFACTS-OUTSIDE-DERIVED **Generated boot artifacts are
outside the derived identity (S5 polygon finding, law).** Boot
regeneration writes a child `spec/boot/STATIC.md` / `INDEX.md` INTO a
dependency slot after materialisation, and by `##BOOT-LANE-SCOPE` those
projections are Markdown regardless of `spec_format` — so the derived
hash excludes them (the same exclusion genre as the derived manifest
itself) and the format-purity claim never counts them: a transformed
slot's «no foreign-form spec files» is asserted over SOURCES, not over
projections the machine regenerates at will. The polygon caught both
failure shapes live before this law existed: a stale derived hash the
moment boot regenerated, and a fake purity violation on the one slot
whose snippet compiles to a child STATIC. @status:impl/done

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
