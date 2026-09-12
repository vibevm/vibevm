# Releasing the manual {#root}

The manual is a `doc` package: it is warmed into the store, never installed, and the site renders every published version from its own bytes. A release is a publication of this package; nothing in it gates a product release (`spec://org.vibevm.core/vibevm/common/PROP-058#GATE-NO-LOCK`).

1. **Green on the inside.** From the package root:

   ```bash
   vibe doc check --examples --citations --derived --coverage --media --translations --style --min 100 --path .
   vibe check --path .
   ```

   Every example matches or is listed in `examples/deferred.toml` with a reason; every citation resolves; every `derived` block is current; coverage is one hundred per cent; every page is clean under the style law.

2. **The version.** Bump `version` in `vibe.toml` and, when the product version changed, the `[[documents]] version` the manual documents (`PROP-058#LOOP-VERSION`). Patch for small edits, minor for a month of work, major only when the manual's own shape changes.

3. **The changelog.** Add the version to `CHANGELOG.md` in plain text written from the journal: what a reader will find new, changed or gone. Never paste the output of `vibe doc diff`.

4. **The journal.** The last entry of the version names the release and the numbers of the queue on that day.

5. **Publish.** `vibe publish` from the package root — the owner's word, every time. After publication: the site rebuilds on its next pass and shows the version as `latest`; check with `curl` that `/doc/sitemap.xml` and `/doc/llms.txt` answer and that the root `robots.txt` and `llms.txt` of the domain still point at them.

6. **Snapshot.** At a version change or at the end of a full reconciliation, record the surface snapshot: `vibe doc surface --record <version> --path .`, and commit `maintenance/surface/<version>.json`.
