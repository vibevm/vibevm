# ADV-SDD-BOOT-DELTA-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** In human mode, `vibe install <pkgs> --plan-only --boot-delta` prints one `boot: <lane> <before> -> <after> (<signed delta> B)` line per changed lane under the package list, or `boot: unchanged` when no lane changes.

**Write.** `crates/vibe-cli/src/commands/install/boot_delta.rs`, `crates/vibe-cli/src/commands/install/preview.rs`

**Check.** `cargo test -p vibe-cli --bin vibe commands::install::boot_delta::tests 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-cli/src/commands/install/boot_delta.rs :: tests::render_names_each_changed_lane_with_signed_delta and tests::render_unchanged_is_one_line (new)

**Read first.**
- crates/vibe-cli/src/commands/install/boot_delta.rs (as left by ADV-SDD-BOOT-DELTA-3)
- crates/vibe-cli/src/commands/install/preview.rs fn run_with_selected (the human branch: ctx.heading and the per-package ctx.step loop)
- crates/vibe-cli/src/commands/install/closure_diff.rs fn render and fn size_text (the precedent)

**Steps.**
1. In boot_delta.rs, add `pub(super) fn render(rows: &[BootDeltaRow]) -> Vec<String>` as decided.
2. At the end of boot_delta.rs, add `#[cfg(test)] mod tests { use super::*; ... }` with the two tests given in decisions.
3. In preview.rs fn run_with_selected, directly after the `for row in report.packages { ... }` loop and before `Ok(())`, add the block given in decisions. (report.packages was moved by the loop; reading report.boot_delta afterwards is allowed.)
4. Run `cargo fmt --all`; reflowed lines are expected. Run `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix only warnings in this entry's files. Run `wc -l` on both files: each must stay at or under 600 lines.
5. Run the check.

**Decisions already made.**
- Order: this entry runs after ADV-SDD-BOOT-DELTA-3; both write boot_delta.rs and preview.rs.
- preview.rs block:
```
    if let Some(rows) = &report.boot_delta {
        for line in super::boot_delta::render(rows) {
            ctx.step(&line);
        }
    }
```
- render: an empty slice gives `vec!["boot: unchanged".to_string()]`. Otherwise each row becomes `format!("boot: {} {} -> {} ({}{} B)", lane, size(before), size(after), sign, magnitude)`, where size(Some(n)) = `format!("{n} B")` and size(None) = `"absent"`. The delta is `i128::from(after.unwrap_or(0)) - i128::from(before.unwrap_or(0))`; sign is "+" when the delta is >= 0 and "-" otherwise; magnitude = `delta.unsigned_abs()`.
- Test render_names_each_changed_lane_with_signed_delta. Rows (BootDeltaRow { lane, before, after }): ("vibevm/vibespecs/boot/INDEX.md", Some(814), Some(963)), ("vibevm/vibespecs/boot/STATIC.xml", None, Some(1976)), ("vibevm/vibespecs/boot/STATIC.md", Some(500), None). Expected lines, in order:
```
boot: vibevm/vibespecs/boot/INDEX.md 814 B -> 963 B (+149 B)
boot: vibevm/vibespecs/boot/STATIC.xml absent -> 1976 B (+1976 B)
boot: vibevm/vibespecs/boot/STATIC.md 500 B -> absent (-500 B)
```
- Test render_unchanged_is_one_line: `render(&[])` equals `vec!["boot: unchanged".to_string()]`.
- The JSON path is untouched: run_with_selected returns through ctx.emit_json before the human branch.

**Not in this entry.** Not in this entry: KB/MB unit formatting, colours, changing the closure_diff renderer, or touching the JSON output.

**Depends on.** ADV-SDD-BOOT-DELTA-3

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
