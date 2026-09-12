# Fixtures

Three files and one tree. Three of them are **copies** taken from the
Rust pipeline's own goldens; the rest are the site's own library, written
here. They exist so the site can be built, its page count checked, its
language machinery exercised and its links followed before any real
documentation package is on the machine.

| File | Where it comes from | Refreshed by |
| --- | --- | --- |
| `island.html` | copy of `crates/vibe-doc/tests/golden/guide-every-block.numbered.html` | `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test island`, then copy |
| `manifest.json` | the site's own: the pipeline's `formats/corpora/doc-manifest/e1/manual.json` with a **second page** added | edited here, kept parseable by `manifest.test.ts` |
| `manifest-ru.json` | the site's own: an **official adaptation** of `manifest.json` that carries only one of its two pages | edited here |
| `doc-build/` | a `vibe doc build` output in miniature: the agent surfaces of `manifest.json`'s library | see below |

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
| `com.example.docs/fixture-manual/0.1.0/guide/every-block.md` | copy of `crates/vibe-doc/tests/golden/guide-every-block.md` |
| `com.example.docs/fixture-manual/0.1.0/guide/every-block.xml` | copy of `crates/vibe-doc/tests/golden/guide-every-block.xml` |
| `com.example.docs/fixture-manual/0.1.0/reference/addresses.{md,xml}` | the site's own, for the second page `manifest.json` declares |
| `llms.txt`, `llms-full.txt` | the site's own, in the shape `vibe doc build` writes them |

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

The addresses are the source's. A translation is served under the source
package's coordinate with a language segment in front of it, never under
its own name (D-06) — which is exactly what lets the language selector
lead to the same page with the same fragment. The adaptation's own
coordinate is what the selector prints as its publisher.
