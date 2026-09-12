# Fixtures

Two files, both **copies**, both taken from the Rust pipeline's own
goldens. They are here so the site can be built and its page count
checked before any real documentation package is on the machine.

| File | Copied from | Refreshed by |
| --- | --- | --- |
| `island.html` | `crates/vibe-doc/tests/golden/guide-every-block.numbered.html` | `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test island`, then copy |
| `manifest.json` | `formats/corpora/doc-manifest/e1/manual.json` | `VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test doc_manifest_wire`, then copy |

They are inputs, not output: nothing in the site build writes them, and
nothing here edits them. The island is one page carrying every block of
the documentation genre once — which is what makes it the right subject
for the prose stylesheet, since a rule the pipeline can emit and the
stylesheet has never seen would show up here first.

When the two drift apart, the golden is right. Copy it over and look at
what changed: a page that renders differently from the pipeline's own
snapshot is the one failure this whole arrangement exists to catch.
