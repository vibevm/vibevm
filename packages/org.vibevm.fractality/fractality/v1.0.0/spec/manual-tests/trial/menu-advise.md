You are working in the `mini_logfmt` repository (the current directory).
Work through the three tasks below. For each, choose whatever approach you
judge best; you own the outcome. Definition of done for the whole
assignment: the requested code exists and `cargo test` passes in this
repository. Work task by task and state clearly when you consider a task
done or skipped.

These tasks each have a subtle correct answer that a plausible-looking
first attempt gets wrong. Think about the edge before you commit.

Task 1 (first-seen order). Add a public function to the crate that takes a
list of keys (with duplicates) and returns the distinct keys in
**first-seen order** — the order in which each key first appeared, with
later duplicates dropped. Wire it into `src/lib.rs`. Note: the obvious
hash-set approach does NOT preserve order; the acceptance test checks that
it does.

Task 2 (record count at the boundary). Add a public function that counts the
number of records in a logfmt input string. A record is one non-empty line.
A **trailing newline must not add a phantom record**: `"a=1\nb=2\n"` has two
records, not three. The acceptance test checks the trailing-newline case.

Task 3 (empty value round-trips). Make the crate's parse→render path
round-trip a pair whose value is empty: the key `k` with an empty value must
render back as exactly `k=` (not `k`, not dropped). Fix whichever of parse
or render mishandles it. The acceptance test checks that `k=` survives a
parse-then-render round trip.
