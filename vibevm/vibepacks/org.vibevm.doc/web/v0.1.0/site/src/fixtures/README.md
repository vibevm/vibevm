# Fixtures

Three files. One is a **copy** taken from the Rust pipeline's own golden;
two are the site's own library, written here. They exist so the site can
be built, its page count checked and its language machinery exercised
before any real documentation package is on the machine.

| File | Where it comes from | Refreshed by |
| --- | --- | --- |
| `island.html` | copy of `crates/vibe-doc/tests/golden/guide-every-block.numbered.html` | `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test island`, then copy |
| `manifest.json` | the site's own: the pipeline's `formats/corpora/doc-manifest/e1/manual.json` with a **second page** added | edited here, kept parseable by `manifest.test.ts` |
| `manifest-ru.json` | the site's own: an **official adaptation** of `manifest.json` that carries only one of its two pages | edited here |

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
