# ADV-FIX-BOOTSIZE-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Plain `vibe tree` prints `<lane file>: unreadable: <error>` for an unreadable static lane (and `INDEX.md: unreadable: <error>` for an unreadable index), and the TUI status line shows `unreadable` in place of the byte and line counts.

**Write.** `crates/vibe-cli/src/commands/tree/plain.rs`, `crates/vibe-cli/src/commands/tree/tui/render.rs`, `crates/vibe-cli/src/commands/tree/tui/snapshot.rs`, `vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe unreadable_static_lane && bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe a_markdown_lane_header_is_byte_identical_to_the_legacy_text && bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe commands::tree::tui::snapshot`

**Test.** (1) crates/vibe-cli/src/commands/tree/plain.rs tests module, new fn `unreadable_static_lane_and_index_render_with_their_errors`: builds `tree(vec![pkg("g/a", LoadType::Static, false, true, &[])], &["g/a"])`, sets `static_lane_name` to `vibe_core::layout::STATIC_XML.to_string()`, sets `boot.static_md = Some(StaticLane { present: false, path: vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_xml()), bytes: 0, lines: 0, contributions: Vec::new(), state: LaneState::Unreadable, error: Some("Permission denied (os error 13)".into()) })`, sets `boot.index_md.state = LaneState::Unreadable` and `boot.index_md.error = Some("stream did not contain valid UTF-8".into())`; asserts `header_of(&render(&t))` == "project: /tmp/x\nSTATIC.xml: unreadable: Permission denied (os error 13)\nINDEX.md: unreadable: stream did not contain valid UTF-8\npackages: 1   roots: 1\ncolumns: load  T=transitive  C=condition  S=in STATIC.xml\n\n". (2) crates/vibe-cli/src/commands/tree/tui/snapshot.rs tests module, new fn `the_status_line_reports_an_unreadable_static_lane`: `fixture_tree()` with `boot.static_md` set to the same unreadable StaticLane, renders `snapshot_headless(tree, 74, 22, "", false)`, takes the first line, asserts it contains "unreadable" and does not contain "0b / 0L". Both names contain `unreadable_static_lane`, so the first command runs exactly these two; before the work it runs zero tests, and with the tests added but the renderers unchanged both fail because the renderers print `0 bytes, 0 lines` / `0b / 0L`. The second and third commands are the existing plain legacy-header test and the committed TUI golden tests, which must stay green.

**Read first.**
- crates/vibe-cli/src/commands/tree/plain.rs:172-193
- crates/vibe-cli/src/commands/tree/plain.rs:355-433
- crates/vibe-cli/src/commands/tree/tui/render.rs:72-100
- crates/vibe-cli/src/commands/tree/tui/snapshot.rs:160-221
- crates/vibe-cli/src/commands/tree/model.rs:188-243
- vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml:115-124

**Steps.**
1. plain.rs `header`: when `tree.boot.static_md` is `Some(lane)` with `lane.state == LaneState::Unreadable`, push `format!("{lane_name}: unreadable: {}\n", lane.error.as_deref().unwrap_or(""))` instead of the size line; the present and `(none)` branches stay byte-identical.
2. plain.rs `header`: directly after the static-lane line, when `tree.boot.index_md.state == LaneState::Unreadable`, push `format!("INDEX.md: unreadable: {}\n", tree.boot.index_md.error.as_deref().unwrap_or(""))`; print nothing extra for present or absent index lanes.
3. tui/render.rs `render_status`: compute the value text as `"unreadable".to_string()` when `boot.static_md` is `Some` with state `LaneState::Unreadable`, else the existing `format!("{bytes}b / {lines}L")`; label span unchanged.
4. Add the plain test `unreadable_static_lane_and_index_render_with_their_errors` and the snapshot test `the_status_line_reports_an_unreadable_static_lane` as described in the test field.
5. In vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml insert the unit given in spec_edit directly after the closing `</static-size-purpose>` tag (line 123 today).
6. Run the check (all three commands, as one chained line).

**Decisions already made.**
- Plain wording is exactly `<static lane file name>: unreadable: <error>` and `INDEX.md: unreadable: <error>`, with the error text verbatim from the model.
- The index line appears only when the index lane is unreadable, so all existing plain output and TUI goldens stay byte-identical.
- The TUI shows only the static lane's state on the status line; an unreadable index is not shown in the TUI.
- Commit: `feat(tree): show unreadable boot lanes in plain and TUI output` with a body saying an unreadable lane used to look like an empty one.

**Not in this entry.** Not in this entry: a TUI detail panel or modal for lane errors, colouring the status value, or adding an absent-index line to the plain header.

**Depends on.** ADV-FIX-BOOTSIZE-2

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml: inside `<static-size title="2.6 The STATIC.md size indicator">` → `<facts ordered="false">`, directly after the closing `</static-size-purpose>` tag, insert:
        <UNREADABLE-LANE-SHOWN fact="true" status="spec/done">When a committed lane exists but cannot be read, the plain header prints `&lt;lane file name&gt;: unreadable: &lt;error&gt;` in place of the static lane's size line, adds `INDEX.md: unreadable: &lt;error&gt;` after it for an unreadable index, and the TUI status line shows `unreadable` in place of `&lt;bytes&gt;b / &lt;lines&gt;L`.</UNREADABLE-LANE-SHOWN>

Entries that write the same file run one after another, never in parallel.
