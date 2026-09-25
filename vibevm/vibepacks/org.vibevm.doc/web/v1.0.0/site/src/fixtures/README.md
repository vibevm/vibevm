# Fixtures

Three files and three trees. Some of them are **copies** taken from the
Rust pipeline's own goldens or its output; the rest are the site's own
library, written here. They exist so the site can be built, its page
count checked, its language machinery exercised and its links followed
before any real documentation package is on the machine.

| File | Where it comes from | Refreshed by |
| --- | --- | --- |
| `island.html` | copy of `crates/vibe-doc/tests/golden/guide-every-block.numbered.html` | `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test island`, then copy |
| `manifest.json` | the site's own: the pipeline's `formats/corpora/doc-manifest/e1/manual.json` with a **second page** added | edited here, kept parseable by `manifest.test.ts` |
| `manifest-ru.json` | the site's own: an **official adaptation** of `manifest.json` that carries only one of its two pages | edited here |
| `doc-build/` | a `vibe doc build` output in miniature: the agent surfaces of `manifest.json`'s library | see below |
| `doc-build-pair/` | a `vibe doc build` output over `crates/vibe-doc/tests/fixture/translations/source` — a **second source documentation** | `vibe doc build --format html\|md\|xml`, merged into one directory |
| `doc-build-pair-ru/` | the same over `…/tests/fixture/translations/adaptation` — the **official adaptation** of the one above | as above |

## The second library

A site carries one library per source documentation, and a fixture set
with one documentation in it cannot show that. `doc-build-pair/` and
`doc-build-pair-ru/` are the second: a package that adapts nothing of
`fixture-manual`'s, and an adaptation that names `com.example.docs/pair`
in `[translates]` and must therefore be attributed to it and to nothing
else. They are not copied from goldens — they are what `vibe doc build`
wrote over the crate's own translation fixture, unedited, media
placeholders included, so the card addresses a page names in `og:image`
are real files a link check can follow.

A build renders them only when a deployment names them in
`VIBE_DOC_OUT`. `pnpm build:static` with nothing named still renders the
one-library fixture pair below, exactly as it did before they existed.

## `doc-build/` — the tree the build copies from

`tools/build.mjs` copies the projections, the `llms` tiers and the card
images of every documentation into the site's output rather than
producing them: they are the pipeline's, block numbers and resolved
citations included (R-26). A deployment names the real trees in
`VIBE_DOC_OUT`; this one is the default, so a build with nothing
installed still writes every address its pages link to — which is what
makes `tools/lint-links.mjs` mean something in a plain `pnpm
build:static`.

| File | Where it comes from |
| --- | --- |
| `manifest.json` | byte copy of `manifest.json` beside it — the card and page list the tree is the rendering of |
| `com.example.docs/fixture-manual/0.1.0/guide/every-block.md` | copy of `crates/vibe-doc/tests/golden/guide-every-block.md` |
| `com.example.docs/fixture-manual/0.1.0/guide/every-block.xml` | copy of `crates/vibe-doc/tests/golden/guide-every-block.xml` |
| `com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html` | copy of `crates/vibe-doc/tests/golden/guide-every-block.numbered.html` — the same bytes as `island.html` |
| `com.example.docs/fixture-manual/0.1.0/reference/addresses.{md,xml}` | the site's own, for the second page `manifest.json` declares |
| `com.example.docs/fixture-manual/0.1.0/reference/addresses/index.html` | the site's own, in the shape the pipeline renders that page's `.xml` into |
| `llms.txt`, `llms-full.txt` | the site's own, in the shape `vibe doc build` writes them |

A tree is a whole edition and not only its agent surfaces. Its
`manifest.json` is the card, the language and the page list the site
builds its addresses, shelves and navigation from, and each page's
`<document>/index.html` is the island the site publishes at that
address — the same bytes `vibe doc serve` glues into the same marker
per request. A deployment names its real trees in `VIBE_DOC_OUT` and
this one is never read then; a build told nothing renders the two
manifests beside this file and shows `island.html` on every page, which
is what it did before the trees carried a library at all.

The two budgeted tiers — `llms-small.txt` and `llms-medium.txt` — are not
here. Nothing on the site links them, the real trees carry them, and a
fixture of a token budget over two short pages would be a file with
nothing in it to be wrong about. There is no `media/` either, so the
fixture build's pages name the site's own `og.png` as their card, which
is the same fallback a real package without a preview takes.

The island stays a byte copy, and when it and the golden drift apart the
golden is right: a page that renders differently from the pipeline's own
snapshot is the one failure that arrangement exists to catch.

The two manifests are not copies, and the reason is in what they have to
show. A wire corpus is written to exercise the wire — one package, one
page, no translation — while the shell has to show a language selector, a
shelf, a fallback and a page count that is more than one. A second page
gives the navigation an order to be wrong about; an adaptation that is
missing that page is what a translation fallback is built and measured
on (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`). Neither invents a field: both
are the same schema the generator emits, and `manifest.test.ts` parses
them with the same parser the site uses at build time.

What each of them declares is chosen the same way — to be the case worth
measuring. The source says a model wrote its prose and the adaptation
says both hands did (`##CARD-AUTHORSHIP`), so the shelf's authorship
filter is exercised on the interesting pair: «human-authored» is measured
over a document that also stands in the other group, which is the rule a
reader is most likely to be surprised by. The two libraries in
`doc-build-pair*/` declare nothing, so the third state — a documentation
that says nothing about who wrote it, and therefore stands in neither
named group — has a document of its own to be measured on.

Both of them also say what their list of pages should look like
(`##NAV-PINNED`): the source pins its guide page and names the folder its
other pages live in, and the adaptation names that same folder in its own
words. Without those two tables every page the package builds shows the
fallback — no pinned page, and a heading made from a directory name — so
the rule that a documentation decides its own navigation would be true in
the code and invisible in every build.

And the source declares a **learning path** (`##NAV-CHAPTERS`), which is
the second order a manual has: two chapters, one page each, the second of
them an appendix. Two pages are as much of a path as this library can
carry — see the note on its length below — and they carry what a build
has to draw: a numbered chapter, an appendix that takes no number, a
chapter whose `id` is not a folder of the page tree, the two ends of the
path, and a step that crosses from one chapter into the other. The
adaptation names the first chapter and leaves the appendix unnamed,
because a chapter a translation has not named keeps the source's words
(`##NAV-CHAPTERS-TRANSLATION`) and that rule is invisible in a build where
every chapter is named. The two libraries in `doc-build-pair*/` declare no
path at all, so the case that must not change — a documentation shown
exactly as it was before a path could be declared — has a package of its
own to be measured on.

**Why this library is not larger.** A third page was written for it and
taken out again: the site's own Content-Security-Policy names one hash per
distinct inline script, the serving configuration cannot carry a value of
4096 bytes (`site/src/seo/csp.ts`, `CSP_CONF_LIMIT`), and the fixture
build stands at 3607 of the 4000 bytes that leaves. One more page is four
more addresses and eight more hashes — 4039 bytes, over the ceiling, and
`csp.test.ts` goes red because the generator then writes no policy at all.
So the fixture library cannot grow by a page until the deployment's answer
to that ceiling is decided (X-044), and what needs a path of three pages —
a page with a neighbour on both sides — is measured in
`site/src/lib/contents.test.ts`, over a library written inside the test.

The source is also the library's bridge. It declares the two authorships
a bridge keeps apart — who maintains the wrapper, who wrote the bytes it
wraps, and the licence of those bytes (PROP-023
`##AUTHORSHIP-SEPARATION`) — because a card and the head of a package's
page both have to show them, and this is the only library the package
builds by itself. The adaptation beside it declares no bridge, so the
unchanged case is on the same shelf as the changed one: one card with
two names under it, one with the publisher and nothing else.

The addresses are the source's. A translation is served under the source
package's coordinate with a language segment in front of it, never under
its own name (D-06) — which is exactly what lets the language selector
lead to the same page with the same fragment. The adaptation's own
coordinate is what the selector prints as its publisher.
