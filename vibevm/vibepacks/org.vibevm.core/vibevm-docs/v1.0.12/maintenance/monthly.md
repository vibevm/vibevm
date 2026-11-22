# The monthly loop {#root}

Half a day, once a month, with the owner. The rule is `spec://org.vibevm.core/vibevm/common/PROP-058#LOOP-MONTHLY`; until the first rehearsal it is a hypothesis, and this list changes with it.

1. **Metrics.** `vibe doc todo --format json --path .` gives the eight numbers (`PROP-058#METRICS-LEAD`); put them in a table next to last month's. The trend matters more than the value.

2. **Audit the corpus.** Every top-level command, manifest field and package kind has a page (compare the `derived` blocks of the reference pages with `vibe --help` and `vibe doc surface`); the glossary keeps one word to one meaning — search the pages for a term used in another sense and for two words used for one thing; a term is introduced where a reader first looks for it; duplicate pages, dead pages; the `llms` tiers within their budgets; ten prompts run through an agent with `vibe doc check --prompts --sample 10` — a red assert is an edit or a debt line.

3. **Analytics.** Once the site is deployed: pages with many exits and short reading time are rewrite candidates; searches without a result; the search console.

4. **Adaptations.** Structural divergences from `vibe doc check --translations`; a page with divergences goes into the adaptation queue.

5. **Debt.** Every `docs:` line of `BACKLOG.md` is closed, given an atom, or re-rated with a reason.

6. **Style.** Ticks that slipped past the linter during the month's readings go into `style/banned.*.txt` and the linter's rules; a false positive is a fix of the rule, never a workaround in the text.

7. **Journal → regulation.** Every entry of the month with an empty «→ регламент» gets its decision; a change of the regulation is a dated edit of PROP-058 citing the entries.

8. **Three pages aloud.** The owner reads one new page, the most visited page, and the oldest by `reviews.toml`; dates go into `reviews.toml`.

9. **Release.** The package ships a minor version with `CHANGELOG.md` written by hand from the month's journal; publication is the owner's word.
