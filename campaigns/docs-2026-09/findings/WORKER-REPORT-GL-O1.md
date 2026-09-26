# WORKER-REPORT-GL-O1 — the glossary as an entity, and the card a term shows

Packet: `campaigns/docs-2026-09/findings/PACKET-GL-O1.md`. Tree
`C:\Users\olegc\git\v\vibevm`, branch `main`, HEAD `5ec1611ed`. Nothing
staged, nothing committed; git used read-only (`status`, `diff`, `show`,
`archive`, `ls-files`).

---

## 1. Files

### Rust — new

| File | What it is |
| --- | --- |
| `crates/vibe-core/src/manifest/document/validation/glossary.rs` | the `[glossary]` grammar, a sibling of `validation/navigation.rs` |
| `crates/vibe-core/src/manifest/document/tests_glossary.rs` | its tests (a file of its own: `tests_documentation.rs` stood at 595 lines) |
| `crates/vibe-doc/src/glossary.rs` + `glossary/tests.rs` | the glossary ENTITY: the declaration, the entries, and what a defect of one is |
| `crates/vibe-doc/src/html/gloss.rs` + `gloss/tests.rs` | the island's hidden definitions block and the walk that picks the entries a page names |
| `crates/vibe-doc/tests/fixture/manual/vibevm/vibespecs/reference/addresses.xml` | the fixture manual's glossary page — three entries |

### Rust — changed

`crates/vibe-core/src/manifest/package/documentation.rs` (`GlossaryDecl`),
`manifest/document.rs` (`Manifest::glossary`), `manifest/package/visibility.rs`
(`ManifestWire` + both conversions), `manifest/package.rs`, `manifest/mod.rs`
(re-exports), `manifest/document/validation.rs` (the call).

`crates/vibe-check/src/checks/doc_package_contract.rs` (+ `tests.rs`) —
`check_glossary`.

`crates/vibe-doc/src/`: `content.rs` (`Content::glossary`), `build.rs`
(`content()` fills it), `agent.rs` (explicitly `None`), `html.rs` (the `prose`
lens helper, the defs block, `GLOSS_DEF_ID`), `html/inline.rs` (`open_anchor`),
`html/links.rs` (`Links::glossing`, `Links::gloss_of`), `html/rule.rs` (+
`rule/tests.rs`) — step 0, `style.rs`, `style/glossary.rs`, `style/terms.rs` and
their tests, `translations.rs` + `tests.rs`, `manifest/tests.rs` (the fixture has
two pages now), `lib.rs`.

`crates/vibe-doc-server/src/routes.rs` — the local reader fills the glossary
from the page set it already read.

Fixtures and generated baselines, all through the pipeline:
`crates/vibe-doc/tests/fixture/manual/vibe.toml` (declares `[glossary]`),
`…/guide/every-block.xml` (one existing paragraph gained two term links — no new
block, so no number moved), `crates/vibe-doc/tests/island.rs`,
`tests/projections.rs`, the four goldens under `tests/golden/`, and
`formats/corpora/doc-manifest/e1/manual.json` (`VIBE_DOC_BLESS=1`; the fixture
gained a page, so the wire corpus gained a page row — the schema did not move).

### Site — new

`design/src/components/glossary-card/{index.tsx,styles.css}`,
`site/src/reader/glossary-card.ts`, `site/src/lib/gloss-place.ts` +
`gloss-place.test.ts`, `site/tests/glossary-card.spec.ts`.

### Site — changed

`design/src/index.ts` (export), `site/src/reader/mount.ts` (start it),
`site/src/routes/doc/[...path]/index.tsx` and
`site/src/components/served/index.tsx` (render the empty card),
`site/src/fixtures/README.md`, the six fixture files of `doc-build/` +
`island.html`, `specmap.json` (regenerated; 3 edges added).

### Not mine

Six files under `vibevm/vibepacks/org.vibevm.core/vibevm-docs{,-ru}/` are
modified in this tree and I did not touch them — a parallel session is
documenting `[glossary]` on the manual pages (`reference/manifest.xml`,
`authoring/write-documentation.xml`,
`howto/read-documentation-locally.xml`). The manual packages are outside my
perimeter; I left them alone. `?? cache/`, `?? target-progress-wire/`,
`?? vibevm/vibedeps/org.vibevm.doc.web/` and two other packets' `PACKET-*.md`
were already there.

---

## 2. Step 0 — the descriptions of folded quotations

`crate::html::rule::tidy` runs on the rule's words BEFORE the description is
cut, and takes off exactly two marks of a HEADING:

* a leading section ordinal — digits with dots and the whitespace after them.
  Only a DOTTED run counts: `6.2 ` and `8.1 ` go, `72 hours after…` stays, on
  the honest reading that `6.2` is an address and `72` is a quantity;
* every `{#…}` token, taken together with the single space in front of it, so
  `REQ {#provenance-edit}. From…` reads `REQ. From…` and not `REQ . From…`.

Before the cut, not after: an ordinal stripped afterwards would already have
eaten one of the line's six words. Both come from one place — an anchor may
name a SECTION, and a section resolves to its heading
(`crate::citations::anchor`), so a citation of a numbered heading of a Markdown
specification arrives with its address in front of it.

Unit tests: `a_headings_section_number_is_not_part_of_the_description` (both
corpus cases plus the `72 hours` counter-example) and
`a_named_anchor_is_not_part_of_the_description` (the `REQ {#…}.` case and both
marks at once).

**On the manual, before and after.** Measured over a real `vibe doc build
--format html` of both editions from one scratch copy, rendered twice: once
with a `vibe.exe` built from `git archive HEAD` (the pre-change tree, in
`…/scratchpad/gl-o1/before-tree`), once with the current one.

| | EN before | EN after | RU before | RU after |
| --- | --- | --- | --- | --- |
| quotations with a description | 756 | **754** | 756 | **754** |
| quotations with the generic line | 52 | **54** | 52 | **54** |

16 distinct descriptions changed and none now carries a section number or an
anchor (checked: 0). Examples: `4.1 The format registry` → `The format
registry`; `6.2 vibe.toml is the most expensive…` → `vibe.toml is the most
expensive format…`; `REQ {#provenance-edit}. From the provenance view…` → `REQ.
From the provenance view…`; `REQ {#gitignore-autogen} (Δ-06, imperative 6).
vibe…` → `REQ (Δ-06, imperative 6). vibe init…`.

The two that moved to the generic line are both citations of
`spec://org.vibevm.core/vibevm/common/PROP-054#why-c-abi`, whose text is the
heading `8.1 The ABI is C + JSON, never the Rust ABI`. Without `8.1` that is
nine words, so the ten-word floor of `##READER-RULE-FOLDED` applies — and it
now applies to the rule's own words rather than to an address standing in front
of them, which is the correct reading of the rule and not a regression.

---

## 3. The grammar and the checks

**`vibe-core`** — `[glossary]` is a top-level table with one field, `page`: a
document path without its extension, spelled as a pin and a chapter row spell
one. `deny_unknown_fields`; legal only in a `doc`-kind package, for the reason
`[navigation]` is (it names a page, and only documentation has a page tree).
The grammar checks the FORM and nothing else — whether the page exists and
whether its sections are entries are questions about a directory and about
prose, and a grammar reads neither.

**`vibe check`** (`##GLOSSARY-CHECKED`) — `doc_package_contract::check_glossary`
runs only when the manifest declares a glossary and reports, each as its own
error: the page is not there; the page does not parse; an entry carries no
`title`; an entry does not open with a paragraph. The judgment lives in
`vibe_doc::glossary::check` rather than in the cell, because the reader shows
those same entries in its cards and the style linter measures those same terms
— three surfaces, one reading of what a glossary is.

**`vibe doc check --translations`** (`##GLOSSARY-TRANSLATION`) — a new
`Problem::Glossary { source, translation }`, folded in from the two manifests
beside the chapters, reported as a defect of the MANIFEST (`Problem::page()`
answers `vibe.toml`, as it does for a chapter). Three shapes: the source
declares one and the translation none (`GLOSSARY MISSING <page>`); the two name
different pages; the translation invents one the source does not have — the same
rule read backwards, because the path is the source's to decide.

**The style linter** — `style::glossary::terms` now takes
`Option<&glossary::Glossary>` instead of a `PageSet`, and
`style::terms::check` takes the glossary so it knows which page not to judge
against its own entries and which target counts as an introduction. The
constant `GLOSSARY_PAGE = "glossary/index.xml"` is **deleted**; it had no other
user.

Measured on the manual (EN, the declaring scratch copy vs a copy with no
`[glossary]`, same binary):

```
with    [glossary]: 49 of 49 page(s) clean, 0 error(s), 130 warning(s)  — 52 term findings
without [glossary]: 49 of 49 page(s) clean, 0 error(s),  78 warning(s)  —  0 term findings
```

And the declared glossary gives the manual exactly what the constant used to:
`doc check --style` output from the pre-change binary and from the current one
is byte-identical on both editions apart from the two timing lines.

---

## 4. The island

`data-gloss` and `aria-describedby` are added by `html::inline::open_anchor`,
which asks `Links::gloss_of(target)`. The lens is set once per island, in
`html::prose(content, page)`, and threaded to every block — a card that
appeared in a paragraph and not in a list item would be a reader wondering
which words have definitions. `href` is untouched: selecting a term still
opens the glossary at the entry.

`Links::glossing(page, glossary)` needs both halves and refuses the glossary
page itself. A target is an entry only when `links::target_document(page,
target)` — the module's own reading of what a relative address means — resolves
to the declared document AND the fragment names an entry the glossary has. The
attribute carries the ENTRY's id rather than the fragment as written, so the
attribute and the definition below it cannot disagree about spelling.

The island then ends, inside `<article class="doc-page">`, with

```html
<aside class="gloss-defs" hidden="" data-gloss-defs="">
  <div class="gloss-def" id="gloss-island" data-gloss="island">
    <p class="gloss-def__term">island</p>
    <p class="gloss-def__text">A page's content as finished HTML…</p>
  </div>
</aside>
```

— the entries THIS page links, each once, in the order it first links them,
read with `html::inline::hrefs` + `target_document` (the pair `chapters.rs`
measures forward links with) so a link inside a code span is not counted. The
block takes **no** number: `data-p` is unchanged by its presence
(`the_definitions_take_no_block_number`). A page that links no term carries no
block at all rather than an empty one.

A definition's own links are resolved against the CURRENT page, as the packet
prescribes and as everything else in that island is, and the defs block is
rendered with **no** glossary lens of its own — a definition that opened a
second card would show a reader the same paragraph twice. One consequence worth
recording: a RELATIVE link written beside the glossary's source file gains one
`../` from the current page's directory, so it can miss when the glossary lives
in another folder. The real manual's glossary carries no Markdown links at all
(0 of them; it cross-references in italics), so nothing in the corpus is
affected; the fixture entry uses an absolute site address, which is
position-independent, and the static build's link check followed it (1154
links, 0 broken).

`.md`, `.xml` and the `llms` files are untouched by design (only `html.rs` and
`html/inline.rs` learned the glossary) and empirically — see §7.

`agent.rs` passes `glossary: None` deliberately: that surface answers for ONE
page by address and the MCP tool refuses HTML outright ("the island is for a
reader with a browser"), while the `.md` and `.xml` an agent does read carry the
glossary page whole beside every other page.

---

## 5. The manifest wire — decision

**`[glossary]` is NOT added to `schemas/doc_manifest.jtd.json`.** The site needs
nothing from it: the card is driven entirely by the island's own bytes —
`a[data-gloss]` and `#gloss-<id>` — and the glossary page shows no card because
the PIPELINE emits no attributes there, not because the shell knows which page
it is. Putting the path on the wire would mean a schema epoch, a codegen run and
a generated TypeScript member that nothing reads, which is a second statement of
one fact for no reader. `##PIPE-SHELL-PARSES-NOTHING` is satisfied the other way
round: the shell parses nothing because the definitions arrive finished. If a
later feature needs the path in the browser — a search that groups terms, say —
the member can be added then, with a user.

---

## 6. The card

`site/src/reader/glossary-card.ts`, started from `mount.ts` after
`startRuleTransclusion`; the empty markup is the design-system component
`GlossaryCard`, rendered once per page in both shells (the public route and the
local reader's `served`).

**Desktop test, taken at the moment of the hover and not once at load**:
`link.closest(".has-sidebar")` (the class `reader/toc.ts` puts on `.doc-view`
when the window is ≥1100px and the reader's column has not taken the sidebar's
room) AND `window.matchMedia("(hover: hover) and (pointer: fine)")`. Both move
under the reader — a resize, a widened column, a keyboard attached to a tablet —
so a decision taken at load would be a decision taken against a page that no
longer exists. The test is asked again after the pause, because 350 ms is long
enough for a window to be resized.

**Timings**: `OPEN_AFTER_MS = 350` on a hover, 0 on keyboard focus (a reader who
tabbed to a term asked for it), `GRACE_MS = 140` on leaving, so the pointer can
cross the gap to the card.

**Positioning**: the pure function `lib/gloss-place.ts` — below the link when
`below + height + 12 ≤ innerHeight`, otherwise above when there is room,
otherwise below anyway (a card clamped into the middle of the text would be a
card over the term). Sideways it is the link's left edge, pulled back to
`innerWidth - 12 - width` and never before `12`, so a narrow column cannot gain
a horizontal scrollbar. The answer is in document coordinates. Seven unit tests
in `gloss-place.test.ts`, including "a card never covers the term it explains"
over six vertical positions.

**Closing**: pointer leaves the link and the card (after the grace), Escape,
scroll, focus out. One card per page, reused; `aria-hidden="true"` on it,
because the reader already hears the definition through the link's
`aria-describedby` on every device. The term and the definition are cloned, so
the links inside a definition stay clickable and the card keeps itself open
while the pointer is on it.

**Styles**: `design/src/components/glossary-card/styles.css`, tokens only
(`--bg-raise`, `--line`, `--radius-md`, `--shadow-card`, `--text`, `--text-2`,
`--font-mono`, `--accent`, `--speed`), `max-width: min(24rem, 100vw - 2rem)`,
0.85rem against the column's text, the term at weight 600. The opacity
transition is refused under `prefers-reduced-motion` (the base stylesheet takes
durations away, which is a jump rather than no movement). Contrast audit green,
46 gated pairs, 0 below threshold.

**Decided on the spot: the dotted underline stays.** `a[data-gloss]` inside
`.has-sidebar .prose` gets `text-decoration-style: dotted` with a small offset,
and goes back to solid under `(hover: none), (pointer: coarse)`. The reason is
the norm's own asymmetry: the card exists only in the desktop layout on a fine
pointer, so the only honest place to advertise it is exactly there — a hint in
the narrow layout would promise something that layout does not do. It is a
decoration change and no colour, so nothing new enters the contrast audit; e2e
asserts `dotted` at 1440 and `solid` at 834.

---

## 7. Fixtures

The fixture manual (`crates/vibe-doc/tests/fixture/manual`, published to the
site as `doc-build/com.example.docs/fixture-manual/0.1.0`) declares
`[glossary] page = "reference/addresses"`. The reference page IS the glossary:
three top-level sections, each a term in the heading and a definition in the
first paragraph. `guide/every-block.xml` links two of them from a paragraph it
already had, so no block number moved anywhere. No new SITE page, so the CSP
ceiling is untouched.

Generated by the README's own recipe: the guide page's three projections are
copies of the re-blessed goldens (`VIBE_DOC_BLESS=1 cargo test -p vibe-doc
--test island --test projections`), and the glossary page's three are `vibe doc
build --format html|md|xml` over the Rust fixture, copied in — which makes
`reference/addresses/index.html` a real pipeline render instead of the
hand-written stand-in it was. `island.html` stays a byte copy of
`guide-every-block.numbered.html`.

**The rollback package is kept.** `doc-build-pair*/` (from
`tests/fixture/translations/{source,adaptation}`) declare no glossary at all and
are byte-unchanged, so the case that must not change — a documentation rendered
exactly as it was before a glossary could be declared — still has a package of
its own in the site's own build, beside the changed one, in
`libraries.spec.ts`.

**Deviation from the packet, stated.** The packet says «перевод объявляет то
же». The declaring fixture is the fixture MANUAL, whose site-side translation is
`manifest-ru.json` — a page-manifest fixture, and `[glossary]` is deliberately
not on that wire (§5), so there is nothing for it to declare. Making the
`doc-build-pair*` pair the declaring one instead would have meant regenerating
two whole fixture trees (whose `manifest.json` carries a render clock) and
giving up the documented rollback package, for a case only the minutes-long
two-library e2e build can see. So the translation rule is covered where it can
be exercised: the `vibe-core` grammar (a translation may declare the table), the
`unmirrored_glossary` unit tests (all four combinations, both renderings), and
an end-to-end run on real packages — see §8, where a scratch EN edition and its
RU adaptation are checked both red and green.

The fixtures README records all of it, and its stale CSP figure is corrected
from 3607 to the measured 3824 of 4000 bytes (the reader has grown since that
paragraph was written; the conclusion — the library cannot grow by a page — is
unchanged).

---

## 8. Self-check — verbatim

```
$ cargo fmt --all -- --check
EXIT=0

$ cargo test -p vibe-core -p vibe-check -p vibe-doc
test result: ok. 98 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 518 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 622 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 204 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
EXIT=0

$ cargo clippy -p vibe-core -p vibe-check -p vibe-doc --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.68s
EXIT=0

$ cargo build -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.82s
EXIT=0
```

On copies of both editions of the manual in scratch, each with
`[glossary] page = "glossary/index"` added to its manifest and nothing else
changed (`…/scratchpad/gl-o1/docs-en`, `…/docs-ru`):

```
$ target/debug/vibe.exe check --path <scratch-en>
vibe check: clean — every check passed against `…\gl-o1\docs-en`
EXIT=0

$ target/debug/vibe.exe check --path <scratch-ru>
vibe check: clean — every check passed against `…\gl-o1\docs-ru`
EXIT=0

$ target/debug/vibe.exe doc check --style --translations --path <scratch-ru>
  GLOSSARY glossary/index (`org.vibevm.core/vibevm-docs` declares none)
    a translation declares the SAME glossary as the documentation it adapts, and two pages are two vocabularies for one manual
    (spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-TRANSLATION)
translations: adapting org.vibevm.core/vibevm-docs found in the project's own in-tree registry, 49 page(s), 1 problem(s), 0 unreadable page(s)
error: a translation does not mirror the documentation it adapts (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR; fix: repair the translation against its source — never the other way round)
EXIT=1
```

That red is the rule working, not a defect: the scratch RU copy declares a
glossary and the SOURCE the four sources reach is the repository's own EN
package, which (correctly, it is outside my perimeter) declares none. With both
editions declaring the same — a scratch project whose in-tree registry holds the
declaring EN copy — the same command is green:

```
$ cd <scratch-proj> && vibe.exe doc check --style --translations --path <scratch-proj>/ru-edition
translations: adapting org.vibevm.core/vibevm-docs found in the project's own in-tree registry, 49 page(s), 0 problem(s), 0 unreadable page(s)
  terms-per-sentence 0 error(s), 7 warning(s)
  readability (ARI)        median 12.9, hardest architecture/what-the-lifecycle-epic-delivered.xml at 17.4
style: 49 of 49 page(s) clean, 100% (threshold 100%), 0 error(s), 35 warning(s), 0 unreadable page(s) [ru]
EXIT=0
```

And `##GLOSSARY-CHECKED` on the real manual's shape — one entry of the scratch
EN copy edited to open with a list instead of a paragraph:

```
$ target/debug/vibe.exe check --path <scratch-en-broken>
  [E]  [doc_package_contract] vibe.toml — the glossary entry for `anchor` on `glossary/index` does not open with a paragraph — the first paragraph is the definition, and it is what a reader is shown in place (violates spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-CHECKED; fix: correct the page or the entry, or drop [glossary] if this documentation defines no terms)
1 error, 0 warnings, 0 info
EXIT=1
```

**The `.md` of the manual, before and after.** Both editions built with
`--format md` by the pre-change binary and by the current one, from the same
scratch copy:

```
$ diff -r -x manifest.json <before-md-en> <after-md-en>
EXIT=0
$ diff -r -x manifest.json <before-md-ru> <after-md-ru>
EXIT=0
```

57 files each — every `.md` page and all four `llms` tiers byte-identical.
`manifest.json` is excluded because it carries the render clock and nothing
else: its only difference is `rendered_at`.

**The HTML of the manual, before and after** (49 pages per edition):

| | EN before | EN after | RU before | RU after |
| --- | --- | --- | --- | --- |
| links with `data-gloss` | 0 | **283** | 0 | **287** |
| `gloss-defs` blocks | 0 | **47** | 0 | **47** |

47 of 49 pages carry a block; the two that do not are the glossary page itself
(by law) and `reference/commands`, which links no term. 48 distinct entries are
carried across each edition.

The local reader agrees, which is the point of the island being one set of
bytes — `vibe doc serve` over the same copy, page `model/two-trees`:

```
$ curl …/doc/org.vibevm.core/vibevm-docs/1.0.0/model/two-trees/ | grep -o 'data-gloss="…"'
      2 data-gloss="lock-file"      2 data-gloss="manifest"      2 data-gloss="registry"   … (link + definition each)
   1 gloss-defs block;  aria-describedby="gloss-specification" …
$ curl …/glossary/index/ | grep -c 'data-gloss\|gloss-defs'
0
```

Site, from `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0`:

```
$ TYPESCRIPT_AI_NATIVE=…/target/debug/typescript-ai-native.exe node tools/floor.mjs --keep-going
=== prettier --check (floor perimeter: design/src, site/src) ===
=== tsc --noEmit ===
=== tests (node --test) ===
=== eslint (floor perimeter: design/src, site/src) ===
=== typescript-ai-native-conform check ===
=== typescript-ai-native-specmap --check ===
=== test-gate (xfail-strict) ===
test-gate: 160 results parsed (0 failed, 0 skipped), baseline entries: 0
floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0

$ node design/audit/contrast.mjs
=== pairs: gated=46, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0

$ node tools/build.mjs static
build (static): csp.txt — 68 inline script hash(es) over 177 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
links: green — 1154 checked, 0 broken.
build (static): ok
EXIT=0

$ node node_modules/@playwright/test/cli.js test -c site/tests/playwright.config.ts
  2 skipped
  248 passed (1.2m)
EXIT=0

$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): no trace of the previous framework — 4 mark(s) looked for, none found
build (embedded): ok
EXIT=0
```

The eleven new e2e tests, each green in the full run:

```
  ok  [chromium] › glossary-card.spec.ts › the definition travels in the page, and the link says which one
  ok  [chromium] › glossary-card.spec.ts › pointing at a term opens a card with its term and definition
  ok  [chromium] › glossary-card.spec.ts › the card waits for the pointer to rest before it opens
  ok  [chromium] › glossary-card.spec.ts › the card stands beside the term and covers neither it nor the window
  ok  [chromium] › glossary-card.spec.ts › leaving the term closes the card, and Escape closes it too
  ok  [chromium] › glossary-card.spec.ts › keyboard focus on a term opens the card
  ok  [chromium] › glossary-card.spec.ts › the glossary page itself shows no card
  ok  [chromium] › glossary-card.spec.ts › the narrow layout shows no card, and a term is still a link
  ok  [chromium] › glossary-card.spec.ts › a touch screen shows no card, and a tap opens the glossary
  ok  [chromium] › glossary-card.spec.ts › the card does not animate under reduced motion
  ok  [chromium] › glossary-card.spec.ts › a term is underlined dotted in the desktop layout and plainly below it
```

The 2 skips are pre-existing and environmental:
`local-reader.spec.ts` skips its two navigation tests because this `vibe.exe`
carries the bare shell (built without `--features
vibe-doc-shell/embedded-shell`), which has no chrome to read a manifest into.
They skip identically at HEAD.

The CSP policy line measures 3824 of the 4000-byte ceiling — inside it, and the
generator wrote the real `map` rather than its refusal fallback. The headroom is
176 bytes, which is the ceiling X-044 already records; my change adds no site
page.

---

## 9. Deviations

1. **The translation fixture** — the packet's «перевод объявляет то же» is
   honoured by the grammar, the `--translations` rule with its unit tests and an
   end-to-end red/green run on real packages, rather than by making the
   `doc-build-pair*` fixture trees the declaring ones. The reasoning, and what
   it would have cost, is in §7.
2. **Two baselines re-blessed** beyond the packet's named perimeter, both
   generated artefacts of the fixture I was told to change and both refreshed
   with their own blessing switch: the four `crates/vibe-doc/tests/golden/*`
   files, and `formats/corpora/doc-manifest/e1/manual.json` (the fixture manual
   gained a page, so the corpus gained a page row; the schema and the epoch did
   not move). Neither is a ratchet baseline.
3. **One stale number corrected** in `site/src/fixtures/README.md` (the CSP
   figure, 3607 → the measured 3824). It is in a file the packet put in
   perimeter, and leaving a false measurement in a paragraph a future decision
   rests on would have been worse than touching it.

Nothing else. `build.rs` and `content.rs` — the files the packet warned a
parallel session might also be editing — were touched minimally: one field on
`Content` and one line filling it in `build::content`, both beside the existing
`examples`/`lang` fields and neither near `translations::borrowed`.
