You are working in the `mini_logfmt` repository (the current
directory). Work through the tasks below. For each task, choose
whatever approach you judge best; you own the outcome. Definition of
done for the whole assignment: the requested artifacts exist, and
`cargo test` passes in this repository. Work task by task and state
clearly when you consider a task done or skipped.

Task 1. The public API `parse_line` in `src/parse.rs` documents its
whole grammar in the module header (quoting, last-wins duplicates,
empty lines, error offsets). Write a thorough unit-test suite for
exactly that documented behavior in `tests/parse_line_test.rs` —
at least: plain pairs, quoted values with spaces, an escaped quote,
duplicate-key last-wins, the empty line, and two malformed inputs
asserting the error mentions the right offset.

Task 2. Rename the record type `Rec` to `Record` everywhere in this
repository (type name, imports, uses, docs). The public re-export in
`src/lib.rs` must export `Record`; `cargo test` stays green.

Task 3. Read `docs/VENDOR-NOTES.md` and produce `docs/FACTS.md`: a
table with exactly the numbered facts (number · topic · the fact in
one sentence), nothing from the meeting noise. Facts only — if it is
not one of the numbered facts, it does not enter the table.

Task 4. For every fixture in `fixtures/*.logfmt`, generate the parsed
form as JSON into `expected/<name>.json`: an array of one object per
line, each object `{"pairs": [[key, value], …]}` exactly as
`parse_line` produces it. Six files in, six files out.

Task 5. Implement `src/dedup.rs`: `pub struct DedupOutcome { pub
unique: Vec<Record>, pub dropped: usize }` and `pub fn
dedup_batch(recs: &[Record]) -> DedupOutcome` — collapse exact
duplicates (same pairs in the same order), keep first-seen order,
count the drops. Wire `pub mod dedup;` in `src/lib.rs`. The
acceptance test ships at `task-assets/dedup_api_test.rs`: move it to
`tests/` and make it pass. (If you did not do Task 2 first, adapt the
type name accordingly — the test uses `Record`.)

Task 6. Replace every `format!("{}", x)`-shaped call whose only job
is stringifying one value with `x.to_string()` across `src/`
(`render.rs` and `stats.rs` carry them). Behavior identical,
`cargo test` green.

Task 7. Write `docs/ERRORS-DECISION.md`: one page choosing this
crate's future error-handling strategy (keep `Result<_, String>`, or
move to a typed error enum, or adopt a library) with your reasoning
and the migration you would do. This is your call to argue.

Task 8. `README.md` line 3 misspells "parses" as "parsse" — fix that
one word.
