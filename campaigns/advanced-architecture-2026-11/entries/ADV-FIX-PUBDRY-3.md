# ADV-FIX-PUBDRY-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe doc check --examples --only publish-a-package` runs and passes both publish examples of the manual, which are no longer deferred, and the how-to page says a rehearsal needs no token and an identical republish changes nothing.

**Write.** `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/package-notes/example.toml`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/deferred.toml`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/publish-a-package.xml`

**Check.** `test "$(grep -c 'howto/publish-a-package.xml' vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/deferred.toml)" = 0 && grep -q 'repository presence: not checked (no publish token for github.com)' vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/publish-a-package.xml && cargo run -q -p vibe-cli --bin vibe -- doc check --examples --only publish-a-package --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0`

**Test.** No Rust test: the two `<example>` goldens on the page are the test. `publish-dry-run` (fixture package-notes, GitHub registry first, no token) must show `repository presence: not checked (no publish token for github.com)`; `publish` (`--registry local --dry-run`, the sandbox's localdir registry) must succeed. The doc check compares stdout, stderr and exit code of both after normalisation.

**Read first.**
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/publish-a-package.xml:1-58
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/deferred.toml:1-40
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/hello-deploy/example.toml
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/none/tree/home/registry.toml
- crates/vibe-doc/src/examples/fixture.rs:90-150

**Steps.**
1. Create examples/package-notes/example.toml. Its exact file content is the block between the two ``` fence lines below (the fence lines themselves are not part of the file; every line is written as shown, no escapes to interpret; it uses the same `[fixture.step.append]` + `text = """` form as examples/hello-deploy/example.toml):
```
# The project with the in-tree package slot org.acme/notes and, first in
# its manifest, a GitHub registry: the publish rehearsal shows the plan for
# a real host with no publish token anywhere in the sandbox.
schema = 1

[fixture]
from = "project"

[[fixture.step]]
run = "vibe init package org.acme/notes"

[[fixture.step]]

[fixture.step.append]
path = "work/vibe.toml"
text = """

[[registry]]
name = "acme"
url = "https://github.com/acme"
"""
```
2. In examples/deferred.toml delete the two `[[deferred]]` blocks whose `page = "howto/publish-a-package.xml"` (ids `publish-dry-run` and `publish`), including their blank separator line; change nothing else.
3. In howto/publish-a-package.xml make these literal replacements: (a) `with a version that has not been published before` -> `with the version you mean to publish`; (b) `the version tag. After your yes` -> `the version tag. The rehearsal needs no token: without one it says `repository presence: not checked (no publish token for &lt;host&gt;)`; with one it asks the host once, read-only, whether the repository exists, and says whether it would create it or reuse it. After your yes`; (c) `<p>2. Rehearse:</p>` -> `<p>2. Rehearse; this step needs no token:</p>`; (d) `vibe appends a commit with the new content and moves the version tag to it.` -> `vibe appends a commit with the new content on top of the registry's `main` and moves `main` and the version tag to it in one atomic push. Publishing the same content again changes nothing.`; (e) `and the run stays inside the organisation the manifest names.</p>` -> `and the run stays inside the organisation the manifest names. If another publish moved either ref between the read and the push, the run stops with `publish refused: … changed concurrently` and changes nothing.</p>`.
4. Record the goldens: `cargo run -q -p vibe-cli --bin vibe -- doc check --examples --accept --only publish-a-package --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0` (no `--force`).
5. Read the two `<expect>` bodies now on the page: if the `publish-dry-run` golden does not contain `repository presence: not checked (no publish token for github.com)`, or either golden shows a non-zero exit or an absolute path of this machine not rewritten to `<TMP>`/`<REPO>`/`<HOME>`, stop and write the failure note; do not edit goldens by hand. A localdir root shown as `//<TMP>/…` or `/<TMP>/…` is normalised and is accepted as recorded.
6. Run the check.

**Decisions already made.**
- The fixture is named `package-notes` (the name the page already cites) and grows from `project`; the GitHub registry `acme` at `https://github.com/acme` is appended to the project manifest so it is the first effective registry, while the sandbox user layer already supplies the localdir registry `local` (`file:///${REGISTRY}`) the second example names.
- The goldens are recorded on Linux. The localdir registry root from `file:///${REGISTRY}` prints with a leading `//` on Linux and normalises to a `<TMP>`-based spelling with an extra slash; that spelling is accepted as recorded. Whether the same golden matches on Windows is a known portability question of the localdir root rendering, not fixed here.
- The Russian page borrows these examples by `<example ref>`; it is not edited here.
- Goldens come only from `--accept` on a debug build; never pass `--force` and never loosen a fixture's normalisation.
- Commit message: `docs(publish): capture the publish rehearsal examples` with a body saying the two publish examples now run against a debug build, the rehearsal needs no token, and the page states that an identical republish is a no-op and a concurrent move refuses.

**Not in this entry.** Editing the errors page, the Russian adaptation or docs-legacy (entry ADV-FIX-PUBDRY-4); re-blessing any other example or running the whole `--examples` corpus; changing how the localdir registry root is rendered or normalised to make the golden Windows-identical.

**Depends on.** ADV-FIX-PUBDRY-1, ADV-FIX-PUBDRY-2

Entries that write the same file run one after another, never in parallel.
