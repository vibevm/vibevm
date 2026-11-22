# ADV-FIX-BOOTSIZE-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe tree --json` reports each STATIC contribution's `bytes` as the exact byte length of its region in the lane file, so a CRLF lane with no final newline gives contribution bytes 22, 26 and 20 for the hermetic fixture instead of 19, 23 and 20.

**Write.** `crates/vibe-cli/src/commands/tree/artifacts.rs`, `crates/vibe-cli/tests/tree_fixture.rs`, `vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test tree_fixture tree_json_counts_crlf_contribution_bytes_exactly && bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe multiline_c1_provenance_keeps_fields_and_body_line_accounting`

**Test.** crates/vibe-cli/tests/tree_fixture.rs, new fn `tree_json_counts_crlf_contribution_bytes_exactly`: calls `write_fixture(dir.path())`, then overwrites `root.join(common::static_md_rel())` with `text`, where `text` = `static_md().replace('\n', "\r\n")` with its final "\r\n" removed via `strip_suffix("\r\n")`. Runs `run_tree_json`. Asserts `boot.static_md.contributions[*].bytes` == [22, 26, 20] in order (crit, umbrella, member). Then asserts the sum of contribution bytes == `text.len()` minus, for every line of `text.lines()` (which strips the CRLF) that starts with "<!-- vibe:static", `line.len() + 2` (the two bytes of its "\r\n"). Before the work the test does not exist (zero tests run); once added it fails against today's code with [19, 23, 20]; after the fix it passes. The second command re-runs the existing unit test `multiline_c1_provenance_keeps_fields_and_body_line_accounting` (LF body "BODY-A\nBODY-B\n"), which must keep passing.

**Read first.**
- crates/vibe-cli/src/commands/tree/artifacts.rs:74-118
- crates/vibe-cli/src/commands/tree/artifacts.rs:467-481
- crates/vibe-cli/tests/tree_fixture.rs:40-60
- crates/vibe-cli/tests/tree_fixture.rs:178-232
- vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml:142-150

**Steps.**
1. In `decompile_static` (artifacts.rs), after `let lines: Vec<&str> = text.lines().collect();`, add `let line_starts: Vec<usize> = std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).take(lines.len()).collect();` and a local closure `let offset = |line: usize| line_starts.get(line).copied().unwrap_or(text.len());`.
2. Replace `let bytes: u64 = body.iter().map(|l| l.len() as u64 + 1).sum();` with `let bytes = (offset(body_end.min(lines.len())) - offset(body_start.min(lines.len()))) as u64;`. Leave `lines`, `body` and the `embeds::scan` call unchanged.
3. Update the doc comment of `decompile_static` to say `bytes` is the exact byte span of the body in the raw text (CRLF counted as two bytes, a missing final newline counted as none).
4. Add the test `tree_json_counts_crlf_contribution_bytes_exactly` to crates/vibe-cli/tests/tree_fixture.rs exactly as described in the test field, reusing `write_fixture`, `static_md`, `run_tree_json` and `common::static_md_rel`; take marker lines from `text.lines()`.
5. In vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml insert the unit given in spec_edit directly after the closing `</STATIC-DECOMPILE>` tag (line 149 today).
6. Run the check (both commands, as one chained line).

**Decisions already made.**
- Byte span = offset of the first byte of line `body_start` to the offset of the first byte of line `body_end` (the next marker's first line), or end of text when the index is at or past the last line.
- Line indices are the same 0-based indices produced by `text.lines()` and by `markers::collect` (which counts '\n'); no change to marker parsing.
- `lines` keeps counting logical lines exactly as today; only `bytes` changes.
- The lane-level `boot.static_md.bytes` (build.rs, `t.len()`) is already exact and is not touched.
- No new type, no change to `StaticContribution` fields, no JSON schema change.
- The test splits lines with `text.lines()` (CR stripped), so each marker line costs `line.len() + 2` bytes; never adjust production code to make a `split('\n')`-based count match.
- This entry and ADV-FIX-BOOTSIZE-2/3 all edit PROP-036 at different anchors; run them one after another, never in parallel.
- Commit: `fix(tree): count static contribution bytes exactly` with a body saying the old sum of line length plus one miscounted CRLF lanes and lanes without a final newline.

**Not in this entry.** Not in this entry: changing how missing or unreadable lane files are read (build.rs `read_opt`), or reworking `embeds::scan` to take byte ranges.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml: inside `<static-decompile>` → `<facts ordered="false">`, directly after the closing `</STATIC-DECOMPILE>` tag, insert:
        <CONTRIBUTION-EXACT-BYTES fact="true" status="spec/done">A contribution's `bytes` is the exact byte length of its region body in the lane file: from the first byte of the line after its marker to the first byte of the next marker line, or to end of file. CRLF line endings count two bytes each and a missing final newline counts none; `lines` counts logical lines.</CONTRIBUTION-EXACT-BYTES>

Entries that write the same file run one after another, never in parallel.
