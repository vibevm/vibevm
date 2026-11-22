# ADV-FIX-RELEASE-NOTES-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `cargo xtask dist gitverse-publish` creates the GitVerse release with the GitHub release body (the changelog section) under the existing 'mirrored from the GitHub release' sentence, and keeps today's fixed body when the source has no body.

**Write.** `xtask/src/dist/release/gitverse.rs`, `xtask/src/dist/release/gitverse/source.rs`, `xtask/src/dist/release/gitverse/tests.rs`, `xtask/src/dist/release/gitverse/tests/integration.rs`

**Check.** `bash tools/cargo-test-ran.sh -p xtask gitverse_notes_`

**Test.** xtask/src/dist/release/gitverse/tests.rs: `gitverse_notes_follow_the_mirrored_sentence` takes fixture(), sets notes = Some("- Changed X.\n\nMutable vibevm 1.0.0 distribution. Verify downloads through `DISTRIBUTIONS.json`.".to_string()), and asserts release_request(&source).body == "vibevm 1.0.0 distribution, mirrored from the GitHub release.\n\n" + that notes text; `gitverse_notes_absent_keep_the_fixed_body` asserts release_request(&fixture()).body == "vibevm 1.0.0 distribution, mirrored from the GitHub release. Verify downloads through `DISTRIBUTIONS.json`.". xtask/src/dist/release/gitverse/tests/integration.rs: `gitverse_notes_come_from_the_github_release_body` copies the fake-server setup of `source_mirrors_the_published_github_release` but the by-tag reply JSON adds "body": "- Changed X." and asserts from_github(&github, &version()).unwrap().notes == Some("- Changed X.".to_string()). The existing `source_mirrors_the_published_github_release` keeps passing (its reply has no body, so notes is None and equals fixture()).

**Read first.**
- xtask/src/dist/release/gitverse.rs:144-158
- xtask/src/dist/release/gitverse/source.rs:40-48
- xtask/src/dist/release/gitverse/source.rs:104-203
- xtask/src/dist/release/gitverse/tests.rs:1-64
- xtask/src/dist/release/gitverse/tests/integration.rs:1-8
- xtask/src/dist/release/gitverse/tests/integration.rs:248-293

**Steps.**
1. In source.rs add the field `pub(crate) notes: Option<String>` as the last field of SourceRelease with doc comment `/// The GitHub release body, when the source is the published GitHub release.`; the only struct literal (in `verify`, near line 197) sets `notes: None`.
2. In source.rs `from_github`, right after the draft check, add `let notes = release.body.clone().filter(|body| !body.trim().is_empty());` and replace the final `verify(version, files)` with `verify(version, files).map(|mut source| { source.notes = notes; source })`.
3. In gitverse.rs `release_request`, set body to `format!("vibevm {} distribution, mirrored from the GitHub release.\n\n{notes}", source.version)` when `source.notes` is Some(notes), and keep today's exact format! string when it is None.
4. Add the two unit tests to gitverse/tests.rs and the integration test to gitverse/tests/integration.rs as described in `test`; run `cargo fmt -p xtask`.

**Decisions already made.**
- New field name `notes: Option<String>` on SourceRelease; `--from-dir` sources always have notes None (no changelog lookup there).
- An empty or whitespace-only GitHub body counts as no notes.
- Notes are inserted verbatim (no trimming, no cap) after "vibevm <version> distribution, mirrored from the GitHub release.\n\n".
- Only the create request changes; an already existing GitVerse release is reused as today and its body is not updated.
- All four touched files are under 450 lines today and stay under 600 (conform max_file_lines).
- Test name prefix `gitverse_notes_`.

**Not in this entry.** Not in this entry: updating the body of an existing GitVerse release, reading CHANGELOG.md for `--from-dir`, or touching the dry-run output format beyond what release_request already prints.

Entries that write the same file run one after another, never in parallel.
