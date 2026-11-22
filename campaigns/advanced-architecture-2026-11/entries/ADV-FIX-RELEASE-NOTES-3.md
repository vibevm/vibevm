# ADV-FIX-RELEASE-NOTES-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `cargo xtask wire-diff` fails, under every epoch regime, when a freshly added formats/breaks/NNN.md is not cited in CHANGELOG.md's [Unreleased] section, naming the note and the exact upgrade-note ending to add, and passes once it is cited.

**Write.** `xtask/src/wire_diff.rs`, `xtask/src/wire_diff/citations.rs`

**Check.** `bash tools/cargo-test-ran.sh -p xtask break_note_citation_`

**Test.** xtask/src/wire_diff/citations.rs (#[cfg(test)] mod tests): `break_note_citation_uncited_fresh_note_is_red` calls check_break_note_citations(&["formats/breaks/014.md".to_string()], "## [Unreleased]\n\n- Changed X.\n") and asserts the error string equals "wire-diff: formats/breaks/014.md is new but CHANGELOG.md [Unreleased] does not cite it; add an upgrade note ending 'Break note: `formats/breaks/014.md`.'"; `break_note_citation_in_unreleased_is_green` asserts Ok for a changelog "## [Unreleased]\r\n\r\n- Changed X. Break note: `formats/breaks/014.md`.\r\n" (CRLF on purpose); `break_note_citation_in_a_dated_section_does_not_count` asserts Err when the only citation sits under "## [1.0.11] — 2026-11-03" and [Unreleased] lacks it; `break_note_citation_without_fresh_notes_is_green` asserts Ok for an empty note list and an empty changelog.

**Read first.**
- xtask/src/wire_diff.rs:53-60
- xtask/src/wire_diff.rs:221-266
- xtask/src/wire_diff.rs:268-312
- xtask/src/dist/changelog.rs

**Steps.**
1. Create xtask/src/wire_diff/citations.rs with module doc `//! Fresh break notes must reach the release notes: [Unreleased] cites each one.` and `use anyhow::{Result, bail};`.
2. In citations.rs add `pub(super) fn check_break_note_citations(notes: &[String], changelog: &str) -> Result<()>` with a one-paragraph doc comment (a fresh break note must reach the user's release notes, so [Unreleased] must cite its path): unreleased = crate::dist::changelog::section(changelog, "Unreleased").unwrap_or(""); uncited = notes whose path string is not a substring of unreleased; if uncited is empty return Ok(()); otherwise bail! with one line per uncited note joined by "\n", each exactly "wire-diff: <note> is new but CHANGELOG.md [Unreleased] does not cite it; add an upgrade note ending 'Break note: `<note>`.'".
3. In citations.rs add the #[cfg(test)] mod tests with the four tests named in `test`.
4. In xtask/src/wire_diff.rs add `mod citations;` on its own line directly after `use crate::rebuild;`.
5. In run_wire_diff, directly after `let notes = fresh_break_notes(&root)?;` and before the verdict match, add: if !notes.is_empty() { let changelog = std::fs::read_to_string(root.join("CHANGELOG.md")).context("wire-diff: reading CHANGELOG.md to check break-note citations")?; citations::check_break_note_citations(&notes, &changelog)?; }. Run `cargo fmt -p xtask` and confirm wire_diff.rs stays at or under 600 lines.

**Decisions already made.**
- The function and its tests live in the new file xtask/src/wire_diff/citations.rs (child module of wire_diff) because wire_diff.rs is at 562 lines and conform max_file_lines = 600 applies to xtask; wire_diff.rs gains only about 8 lines.
- The check lives inside `cargo xtask wire-diff` (no new self-check step) and runs before the verdict, so it applies under Quiet, Reporting, Declared and the red verdicts alike.
- CHANGELOG.md is read from the working tree (the break note is uncommitted by definition) and only when there is at least one fresh note.
- Citation = the literal note path (as git status prints it, e.g. formats/breaks/014.md) appearing anywhere inside the [Unreleased] section; backticks around it are allowed because the match is a substring match; CRLF files work because section() trims and the path never spans a line break.
- A missing [Unreleased] section counts as empty, so every fresh note is reported uncited.
- Reuse crate::dist::changelog::section from ADV-FIX-RELEASE-NOTES-1; no second extractor.
- Test name prefix `break_note_citation_`.

**Not in this entry.** Not in this entry: changing the five-row verdict table or its messages, requiring citations for old (tracked) notes, or editing CHANGELOG.md / formats/breaks/ themselves.

**Depends on.** ADV-FIX-RELEASE-NOTES-1

Entries that write the same file run one after another, never in parallel.
