# The weekly loop {#root}

Thirty to sixty minutes, once a week. The rule is `spec://org.vibevm.core/vibevm/common/PROP-058#LOOP-WEEKLY`; the page for people is `architecture/how-this-manual-is-maintained`.

1. **Report.** A cheap model runs, from the package root of the manual:

   ```bash
   vibe doc todo --format md --path .
   vibe doc check --citations --derived --coverage --media --style --path .
   ```

   The two outputs are the report of the week. The maintainer reads the report, not the raw trees.

2. **Sort the queue.** For every row: fixed in five minutes → fix now, one commit `docs(vibevm-docs): …` per edit, at most five edits this loop; bigger → a `docs:` line in the host's `BACKLOG.md` with a severity and the address; disputable → one line to the owner.

3. **Signals of the week.** Questions from people and agents, adaptations that fell behind, pages with odd reader behaviour once analytics exist. Each becomes an edit, a debt line, or «observation without action: reason» in the journal.

4. **Page of the week.** The next page along `maintenance/reviews.toml` (a page with no row comes first). Read it aloud as the reader of `AUTHORING.md` §1 would. Every stumble is an edit or a debt line. Write the reading date and your name or role into `reviews.toml`.

5. **Journal.** One entry in `JOURNAL.md`: what was done, what was deferred, what surprised, how long the loop took.

6. **Ship or hold.** Small edits ship as a patch version weekly or accumulate to the monthly release — the owner's open choice (`PROP-058#SELF-OPEN-CADENCE`); until his word they accumulate.
