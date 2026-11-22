# ADV-RELEASE-REPLACE-GUARD-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The `Build vibevm distributions` workflow has a boolean input `replace_published` (default false), and its prepare step passes `--replace-published` to `xtask dist prepare` exactly when that input is true.

**Write.** `.github/workflows/release-distributions.yml`, `xtask/src/dist/release/tests.rs`, `xtask/src/dist/release/tests/workflow.rs`

**Check.** `cargo test -p xtask dist::release::tests::workflow:: 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** New xtask/src/dist/release/tests/workflow.rs with one test, `release_workflow_forwards_replace_published_to_prepare` (module path dist::release::tests::workflow). The test reads `<CARGO_MANIFEST_DIR>/../.github/workflows/release-distributions.yml`, normalises CRLF to LF, and asserts the exact input block and prepare-step lines given in the decisions. Before the work the filter matches 0 tests, so the grep fails.

**Read first.**
- .github/workflows/release-distributions.yml: `on.workflow_dispatch.inputs` (input `self_check`), job `prepare`, step `Prepare or replace the mutable-version draft`
- xtask/src/dist/release/tests.rs: the `#[path = "tests/..."]` / `mod ...;` declaration pairs
- xtask/src/dist/mod.rs: DistCommand::Prepare (field replace_published, added by entry 1)

**Steps.**
1. Confirm entry 1 has landed: `mod replace_guard;` is present in xtask/src/dist/release/tests.rs. If not, stop and report.
2. In .github/workflows/release-distributions.yml, insert the input block from the decisions directly after the `self_check` input's `default: false` line, at the same indentation as the other inputs.
3. In the step `Prepare or replace the mutable-version draft`, add the env line `          VIBEVM_DIST_REPLACE_PUBLISHED: ${{ inputs.replace_published }}` directly after the `          VIBEVM_DIST_VERSION: ${{ inputs.version }}` line of that step. Replace its single-line `run:` with the exact multi-line run block from the decisions. Change nothing else in the file.
4. Create xtask/src/dist/release/tests/workflow.rs with the exact content from the decisions.
5. In xtask/src/dist/release/tests.rs, add `#[path = "tests/workflow.rs"]` and `mod workflow;` after the last existing `#[path = "tests/..."]` / `mod ...;` pair (after entry 1 that is `mod replace_guard;`), separated by one blank line.
6. Run `cargo fmt -p xtask` and `cargo clippy -p xtask --all-targets -- -D warnings`. Run `wc -l xtask/src/dist/release/tests.rs`; it must be at most 600. Then run the check.

**Decisions already made.**
- Input block (exact, 6-space indent for the key):
```
      replace_published:
        description: Let prepare delete and replace an already published release of this version
        required: true
        type: boolean
        default: false
```
- New run block for the prepare step (exact, replacing the line `        run: target/debug/xtask dist prepare --version "$VIBEVM_DIST_VERSION"`):
```
        run: |
          set -- --version "$VIBEVM_DIST_VERSION"
          if [ "$VIBEVM_DIST_REPLACE_PUBLISHED" = true ]; then set -- "$@" --replace-published; fi
          target/debug/xtask dist prepare "$@"
```
The input reaches the shell through an env variable, never through `${{ }}` inside the script text, matching how VIBEVM_DIST_VERSION is passed.
- Content of xtask/src/dist/release/tests/workflow.rs (exact):
```
use std::path::Path;

#[test]
fn release_workflow_forwards_replace_published_to_prepare() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../.github/workflows/release-distributions.yml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .replace("\r\n", "\n");
    let input = concat!(
        "      replace_published:\n",
        "        description: Let prepare delete and replace an already published release of this version\n",
        "        required: true\n",
        "        type: boolean\n",
        "        default: false\n",
    );
    assert!(text.contains(input), "replace_published input is missing");
    assert!(text.contains("          VIBEVM_DIST_REPLACE_PUBLISHED: ${{ inputs.replace_published }}\n"));
    assert!(text.contains(
        "          if [ \"$VIBEVM_DIST_REPLACE_PUBLISHED\" = true ]; then set -- \"$@\" --replace-published; fi\n"
    ));
    assert!(text.contains("          target/debug/xtask dist prepare \"$@\"\n"));
}
```
The file needs no `use super::*;`. Reformatting by cargo fmt is fine.
- This CI edit is its own commit, separate from entry 1 (owner decision 4 in PLAN.md approves the published-release guard as part of the CI batch, as separate commits). Commit message: `ci(release): forward a replace_published input to dist prepare`.
- xtask/src/dist/release/tests.rs is shared with ADV-RELEASE-REPLACE-GUARD-1; the two entries run one after another, entry 1 first. The workflow file may also be edited by other ADV-RELEASE entries (smoke, aarch64); this entry runs one after another with any of them, never in parallel on that file.

**Not in this entry.** Not in this entry: changing the `upload` input's wording or default, new inputs for build/finalize, pinning or bumping actions, editing docker-images.yml, or adding a YAML parser dependency to xtask.

**Depends on.** ADV-RELEASE-REPLACE-GUARD-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
