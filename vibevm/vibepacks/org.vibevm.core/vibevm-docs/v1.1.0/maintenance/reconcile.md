# The full reconciliation {#root}

A day or two, once a quarter and before a major milestone. A promise the team makes itself, never a release condition (`spec://org.vibevm.core/vibevm/common/PROP-058#LOOP-RECONCILE`). Written from the first one, 2026-09-12, which took one working day for forty-nine pages.

1. **Start green.** `bash tools/self-check.sh` at the phase border, or the targeted gate the owner allows: `cargo xtask check-codegen`, `cargo xtask specmap --check`, `cargo xtask wire-diff`, `cargo xtask conform check`, `vibe facts check --exhaustive`. Drift of the manual is invisible on a red panel (`PROP-058#LOOP-RECONCILE-STARTS-GREEN`).

2. **Build the release binary.** `cargo build --release -p vibe-cli`. The manual is checked against the product a user installs, not against a debug build; the two print the same help, and the release is the one the version number names.

3. **Run everything.** A cheap model, from the package root of the manual:

   ```bash
   vibe doc check --examples --citations --derived --coverage --media --translations --style --min 100 --binary <release vibe> --sandbox <short path>
   vibe doc todo --format md --path .
   ```

   Every example matches or sits in `examples/deferred.toml` with a reason; every citation resolves; `derived` is current or re-recorded with `--accept` after the prose around it was read; coverage is one hundred per cent. Prompts run through an agent with `--prompts` when one is available; without one, the asserts are listed and the run is recorded as skipped. Nothing writes into `~/.vibe` or the package tree while the examples run.

4. **Reread every page against the product.** In the order of `maintenance/reviews.toml`, least recently read first; a page with no row first of all. For each page: the page, the example on it, `vibe <command> --help` and the specification open beside it. Read the prose against the example on the same page first — the first reconciliation found its largest defect there, prose promising a file the example on the same page showed absent. A stumble is an edit with a `rule` beside it, or a debt line. Write the reading date and the reader into `reviews.toml` as you go, not at the end.

5. **Describe the product as it is.** Where the specification promises what the product does not yet do, the page says what happens today and cites the rule as the promise; the gap goes to `BACKLOG.md` as a product item, never into the prose as a wish.

6. **Reread adaptations against their sources**, block for block; `vibe doc check --translations` must report no structural difference, and the meaning is a human's call.

7. **Record the surface.** `vibe doc surface --record <version> --path .` with the release binary, so the next version change has something to diff against. Commit `maintenance/surface/<version>.json`.

8. **Close.** Set `reconciled = "<date>"` in `maintenance/reviews.toml`; one journal entry with the gaps before and after, the pages reread and rewritten, and the time; the package ships a version compatible with the current product release — the owner's word — and after publication the root links of the domain to `/doc/sitemap.xml` and `/doc/llms.txt` are checked with `curl`.

The gate is the reconciliation's own: zero gaps in the queue and no reading date older than the reconciliation. It gates nothing else.
