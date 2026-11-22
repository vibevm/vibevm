# ADV-RELEASE-REPLACE-GUARD-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `cargo xtask dist prepare` refuses to delete an already published same-version release, and deletes nothing, unless `--replace-published` is given; a same-version draft is still replaced without the flag.

**Write.** `xtask/src/dist/mod.rs`, `xtask/src/dist/release.rs`, `xtask/src/dist/release/prepare_draft.rs`, `xtask/src/dist/release/tests.rs`, `xtask/src/dist/release/tests/replace_guard.rs`, `vibevm/vibespecs/common/PROP-019-version-manager.xml`, `specmap.json`

**Check.** `cargo test -p xtask dist::release::tests::replace_guard:: 2>&1 | grep -F "test result: ok. 7 passed"`

**Test.** New module xtask/src/dist/release/tests/replace_guard.rs (module path dist::release::tests::replace_guard). It holds three new tests, `prepare_refuses_to_replace_a_published_release_without_the_flag`, `prepare_replaces_a_draft_without_the_flag` and `dist_prepare_parses_the_replace_published_flag`, plus the four existing `prepare_*` tests moved there from tests.rs: prepare_validates_the_prepositioned_tag_before_replacing_the_release, prepare_without_a_tag_creates_it_before_the_draft, prepare_refuses_a_tag_update_response_with_wrong_provenance and prepare_refuses_a_moved_tag_before_deleting_the_existing_release. That makes 7 tests under the filter. Before the work the filter matches 0 tests (today `cargo test -p xtask dist::release::tests` runs 16 tests, none in a `replace_guard` module), so the grep fails. The clap test covers the field name and `#[arg(long)]`; the run_dist arm that forwards the field is checked by review only (it needs a live GitHub client to run).

**Read first.**
- xtask/src/dist/release.rs: fn prepare, fn prepare_with, trait ReleaseHost, fn validate_tag_ref, the `mod companions; pub(crate) mod gitverse; mod local; mod status;` block and the `use local::...` lines after it
- xtask/src/dist/mod.rs: enum DistCommand variant Prepare, fn run_dist
- xtask/src/dist/release/tests.rs: struct MockHost (fields release, operations, moved_to), MockHost::draft, fn release, consts COMMIT and TREE, the four `prepare_*` tests, the `#[path = "tests/concurrency.rs"] mod concurrency;` declaration
- xtask/src/dist/release/tests/concurrency.rs (pattern for a child test module that starts with `use super::*;`)
- vibevm/vibespecs/common/PROP-019-version-manager.xml: element RELEASE-MUTABLE
- specmap.json: the spec-unit object whose `anchor` is `RELEASE-MUTABLE`

**Steps.**
1. Precondition, before any edit: run `cargo xtask specmap --check` at the repository root. It must pass. If it fails for any reason (for example the specification authority file `.vibe/specmap-authority.json` is missing, or the committed specmap.json is already out of date), make no change at all: stop and report the exact error to the lead. Do not run `vibe install` or any other command that writes outside the write list.
2. Create xtask/src/dist/release/prepare_draft.rs and move `fn prepare_with` there from release.rs, deleting it from release.rs. Use the header and imports given in the decisions. Its signature becomes `pub(super) fn prepare_with(host: &dyn ReleaseHost, version: &Version, identity: &GitIdentity, replace_published: bool) -> Result<GithubRelease>`. Keep the body byte for byte except the final `if let Some(existing) = host.find_release(&request.tag_name)? { ... }` block, which becomes the guard block given in the decisions. In release.rs, add `mod prepare_draft;` on its own line right after the line `mod local;`, and add `use prepare_draft::prepare_with;` on its own line right after `use local::{read_local_platform, upload_with};`.
3. In release.rs, change `pub(crate) fn prepare(repo_root: &Path, raw_version: &str) -> Result<()>` to `pub(crate) fn prepare(repo_root: &Path, raw_version: &str, replace_published: bool) -> Result<()>`. Change the call to `prepare_with(&client, &version, &identity, replace_published)?`. Replace its println with the exact code in the decisions.
4. In xtask/src/dist/mod.rs, replace the `Prepare` variant (with its doc comment `/// Delete any same-version release and prepare a fresh mutable GitHub draft.`) by the exact block in the decisions. Change the run_dist arm `DistCommand::Prepare { version } => release::prepare(repo_root, &version),` to `DistCommand::Prepare { version, replace_published } => release::prepare(repo_root, &version, replace_published),`.
5. Create xtask/src/dist/release/tests/replace_guard.rs. It starts with `use super::*;`, then holds the four `prepare_*` tests cut verbatim from tests.rs, then the three new tests given in the decisions. In the moved tests, add a fourth argument to each `prepare_with` call: `true` in prepare_validates_the_prepositioned_tag_before_replacing_the_release (its fixture is a published release, draft = false), and `false` in the other three. In tests.rs, delete the four moved tests (with the blank line after each) and add the two lines `#[path = "tests/replace_guard.rs"]` and `mod replace_guard;` after `mod concurrency;`, separated from it by one blank line as the existing declarations are.
6. In vibevm/vibespecs/common/PROP-019-version-manager.xml, inside RELEASE-MUTABLE, replace the one line given in the decisions (the line count stays the same). Then run `cargo xtask specmap` to regenerate specmap.json. Run `git diff --numstat -- specmap.json`: it must print exactly `2	2	specmap.json` (only the `content_hash` and `heading` lines of the RELEASE-MUTABLE spec-unit object change; no `line` value moves, because no #[spec] marker sits in the touched xtask files and the spec line count is unchanged). If any other line of specmap.json changes, stop and report instead of committing. Then run `cargo xtask specmap --check`; it must pass.
7. Run `cargo fmt -p xtask` and `cargo clippy -p xtask --all-targets -- -D warnings`. Run `wc -l` on xtask/src/dist/release.rs, xtask/src/dist/release/prepare_draft.rs, xtask/src/dist/release/tests.rs and xtask/src/dist/release/tests/replace_guard.rs; each must be at most 600. Run `git status --short`: apart from the paths that were already modified or untracked before this entry started, it must list only paths in the write list. Then run the check.

**Decisions already made.**
- Flag: `--replace-published` on `cargo xtask dist prepare` (clap field `replace_published: bool`, `#[arg(long)]`). The guard sits after both validate_tag_ref calls and before delete_release. Nothing else in prepare_with changes order: a moved tag is still refused before any release lookup.
- Exact replacement for the Prepare variant in mod.rs:
```
    /// Replace a same-version draft with a fresh mutable GitHub draft. A published
    /// same-version release is replaced only with --replace-published.
    Prepare {
        #[arg(long)]
        version: String,

        /// Delete and replace a same-version release that is already published.
        #[arg(long)]
        replace_published: bool,
    },
```
- Header and imports of prepare_draft.rs (exact):
```
//! `dist prepare`: replace a same-version draft with a fresh one; a published
//! release is replaced only when the caller passes `--replace-published`.

use anyhow::{Result, bail};
use semver::Version;
use vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME;
use vibe_publish::{CreateGithubRelease, GithubRelease};

use super::{GitIdentity, ReleaseHost, validate_tag_ref};
```
GitIdentity (a private `use super::snapshot::...` import of release.rs), the private trait ReleaseHost and the private fn validate_tag_ref are all nameable from a child module through `super::`. release.rs keeps all its existing imports: CreateGithubRelease is still used by trait ReleaseHost and its client impl, DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME by finalize_with, Version and bail by other functions. If clippy still reports an unused import in release.rs, delete just that name.
- Guard block in prepare_with (exact):
```
    if let Some(existing) = host.find_release(&request.tag_name)? {
        if !existing.draft && !replace_published {
            let tag = &request.tag_name;
            bail!(
                "dist prepare: release `{tag}` is already published; replacing it deletes the live \
                 release and installers fail until `dist finalize --publish` runs. Pass \
                 --replace-published to replace it deliberately."
            );
        }
        host.delete_release(existing.id)?;
    }
```
- Success output in fn prepare (exact, replacing the current println):
```
    let opt_in = if replace_published { "; --replace-published given" } else { "" };
    println!(
        "dist prepare: fresh draft `{}` at {} (release id {}, prior release replaced{opt_in})",
        release.tag_name, identity.commit, release.id
    );
```
- New test 1 (exact):
```
#[test]
fn prepare_refuses_to_replace_a_published_release_without_the_flag() {
    let host = MockHost::draft();
    host.release.borrow_mut().as_mut().unwrap().draft = false;
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let error = prepare_with(&host, &Version::parse("1.0.0").unwrap(), &identity, false)
        .expect_err("a published release must not be replaced without the flag")
        .to_string();
    assert!(error.contains("release `v1.0.0` is already published"));
    assert!(error.contains("Pass --replace-published"));
    assert_eq!(
        host.release.borrow().as_ref().map(|release| (release.id, release.draft)),
        Some((7, false))
    );
    assert_eq!(host.operations.borrow().as_slice(), ["tag-find", "find"]);
}
```
- New test 2 (exact):
```
#[test]
fn prepare_replaces_a_draft_without_the_flag() {
    let host = MockHost::draft();
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let result = prepare_with(&host, &Version::parse("1.0.0").unwrap(), &identity, false).unwrap();
    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["tag-find", "find", "delete:7", "create"]
    );
}
```
- New test 3, the clap wiring (exact; the wrapper struct sits in replace_guard.rs above the test):
```
#[derive(clap::Parser)]
struct DistCli {
    #[command(subcommand)]
    command: crate::dist::DistCommand,
}

#[test]
fn dist_prepare_parses_the_replace_published_flag() {
    use clap::Parser;
    let with_flag =
        DistCli::try_parse_from(["xtask", "prepare", "--version", "1.0.0", "--replace-published"])
            .unwrap();
    assert!(matches!(
        with_flag.command,
        crate::dist::DistCommand::Prepare { ref version, replace_published: true } if version == "1.0.0"
    ));
    let without_flag = DistCli::try_parse_from(["xtask", "prepare", "--version", "1.0.0"]).unwrap();
    assert!(matches!(
        without_flag.command,
        crate::dist::DistCommand::Prepare { replace_published: false, .. }
    ));
}
```
DistCommand is `pub(crate)` and derives `Debug, Subcommand`; xtask already depends on clap with derive (main.rs derives Parser). Reformatting by cargo fmt is fine.
- Line budget: tests.rs is 599 lines and release.rs is 592 lines today; conform max_file_lines is 600. Moving the four prepare tests (about 81 lines) out of tests.rs and prepare_with (about 27 lines) out of release.rs keeps both well under the limit. The moved tests keep every assertion unchanged.
- Spec line in PROP-019 RELEASE-MUTABLE. Old line (exact):
```
  deletes any existing draft or published release for `V`, and creates one fresh draft. Four hosts may
```
New line (exact):
```
  deletes an existing draft for `V` (a published release only with `--replace-published`), and creates one fresh draft. Four hosts may
```
The status attribute stays `impl/done`, and no #[spec] marker is added: the existing RELEASE-MUTABLE markers in crates/vibe-publish/src/github_release.rs stay as they are.
- The only caller of `release::prepare` is run_dist in mod.rs. The only callers of `prepare_with` are fn prepare and the four moved tests (verified by grep).
- xtask/src/dist/release/tests.rs is shared with ADV-RELEASE-REPLACE-GUARD-2; the two entries run one after another, this entry first.
- Commit message: `feat(xtask): refuse to replace a published release in dist prepare without --replace-published`.

**Not in this entry.** Not in this entry: the workflow input (entry 2), a guard in `dist upload` or `dist finalize`, any change to the GitVerse mirror, a new PreparedRelease return type that would print whether the replaced release really was published, running `vibe install` or creating the specmap authority file, or reshuffling other tests out of tests.rs.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-019-version-manager.xml, element RELEASE-MUTABLE: replace the single line `  deletes any existing draft or published release for `V`, and creates one fresh draft. Four hosts may` with `  deletes an existing draft for `V` (a published release only with `--replace-published`), and creates one fresh draft. Four hosts may` (step 6), then regenerate specmap.json with `cargo xtask specmap` and confirm only the RELEASE-MUTABLE content_hash and heading lines changed.

Entries that write the same file run one after another, never in parallel.
