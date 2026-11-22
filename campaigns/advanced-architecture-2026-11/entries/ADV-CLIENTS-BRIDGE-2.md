# ADV-CLIENTS-BRIDGE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `DeclaredSkillProjection::unresolved_references` returns, once per skill, every relative path that a projected Markdown file names and that neither the projected skill tree nor the project root contains.

**Write.** `crates/vibe-agent-projection/src/pkgskill/projection/closure.rs`, `crates/vibe-agent-projection/src/pkgskill/projection/closure_tests.rs`, `crates/vibe-agent-projection/src/pkgskill/projection.rs`, `crates/vibe-agent-projection/src/pkgskill.rs`

**Check.** `cargo test -p vibe-agent-projection --lib pkgskill::projection::closure::tests 2>&1 | grep -F 'test result: ok. 5 passed'`

**Test.** crates/vibe-agent-projection/src/pkgskill/projection/closure_tests.rs (module path pkgskill::projection::closure::tests), five tests: `one_dangling_script_reference_warns_once`, `frontmatter_script_and_project_root_resolution`, `fenced_urls_absolute_and_placeholder_tokens_are_ignored`, `non_markdown_files_are_not_scanned`, `projection_reports_each_skill_once_across_agents`. None exists today, so the filter runs 0 tests and the grep fails.

**Read first.**
- crates/vibe-agent-projection/src/pkgskill/projection.rs: struct DeclaredSkillProjection (private fields `project_root: PathBuf`, `tasks: Vec<ProjectionTask>`), struct ProjectionTask (fields provider: String, agent: Agent, scope: Scope, name: String, source: PathBuf, include: Vec<String>), fn prepare_declared_skill_plan, the `#[path = "projection/binding.rs"] mod binding;` block
- crates/vibe-agent-projection/src/pkgskill/snapshot.rs: fn snapshot_source (`pub(super) fn snapshot_source(source: &Path, include: &[String]) -> Result<BTreeMap<String, Vec<u8>>, PackageSkillError>`)
- crates/vibe-agent-projection/src/pkgskill.rs: the `pub use projection::{...}` list
- crates/vibe-agent-projection/src/pkgskill/projection/operations.rs: impl DeclaredSkillProjection fn install (style of a method in a child module; it calls `crate::pkgskill::snapshot_source(&task.source, &task.include)?`)
- crates/vibe-agent-projection/src/pkgskill/projection/tests.rs: fn project_with_skill (fixture pattern)

**Steps.**
1. Create crates/vibe-agent-projection/src/pkgskill/projection/closure.rs with a `//!` doc line, `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-INSTALL");`, and imports `std::collections::{BTreeMap, BTreeSet}`, `std::path::Path`, `anyhow::Result`, `serde::Serialize`, `specmark::spec`, `super::DeclaredSkillProjection`. Add the public type with its compiled doctest (the crate is in conform.toml gated_pub_doctest):
```
/// One relative path a projected Markdown file names that neither the
/// projected skill tree nor the project root contains.
///
/// ```
/// use vibe_agent_projection::pkgskill::UnresolvedReference;
/// let r = UnresolvedReference {
///     skill: "demo".into(),
///     file: "SKILL.md".into(),
///     reference: "scripts/missing.sh".into(),
/// };
/// assert_eq!(r.reference, "scripts/missing.sh");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-INSTALL")]
pub struct UnresolvedReference {
    pub skill: String,
    pub file: String,
    pub reference: String,
}
```
2. In closure.rs add `pub(super) fn scan_unresolved(skill: &str, files: &BTreeMap<String, Vec<u8>>, project_root: &Path) -> Vec<UnresolvedReference>`, implementing exactly these rules. (1) Scan only keys ending in `.md`; skip values that are not UTF-8. (2) Frontmatter: when the first line, after trim_end, equals `---`, find the first later line that, after trim_end, equals `---`. The lines strictly between those two are frontmatter, and the lines after the closing line are body. If no later line equals `---` after trim_end, the file has no frontmatter and every line (including the first) is body. Each whitespace-separated word of a frontmatter line is a candidate after trimming the chars `"'(` from its start and the chars `"'),;:.` from its end. (3) Body: a line whose trim_start starts with three backticks toggles a fenced block; fence lines and lines inside a fence are skipped. On every other body line: split the line on the backtick char; each odd-index segment that is not the last segment is a code span, and its first whitespace-separated word is a candidate. Also, for every occurrence of `](`, the text up to the next `)` is a link target (skip it when there is no `)`); its first whitespace-separated word, cut at the first `#`, is a candidate. Only frontmatter words are trimmed: code-span first words and link-target first words are used as they are (link targets are only cut at `#`). (4) A candidate is a reference only if: it does not start with `/` or `~`; it does not contain `://`; after removing at most one leading `./` or `../`, the rest splits on `/` into at least two non-empty segments made only of ASCII letters, digits, `_`, `.`, `-`; and the last segment, split at its last `.`, has a non-empty stem and a non-empty all-ASCII-alphanumeric extension. (5) A reference resolves when any of these holds: (a) joining it to the directory of the referencing key (the part before the key's last `/`, or empty) and normalising `.` and `..` lexically stays inside the tree and names a key in `files`; (b) it does not start with `../` and, with one leading `./` removed, names a key in `files`; (c) it does not start with `../` and `project_root.join(<it with one leading ./ removed>).is_file()`. (6) Every unresolved reference yields `UnresolvedReference { skill: skill.to_string(), file: <key>, reference: <the candidate exactly as it was after step (2) or (3)> }`; sort and dedup the rows before returning.
3. In closure.rs add the method:
```
impl DeclaredSkillProjection {
    /// Unresolved relative references of every selected skill, one row per
    /// (skill, file, reference), skills deduplicated across agents and scopes.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-INSTALL")]
    pub fn unresolved_references(&self) -> Result<Vec<UnresolvedReference>> {
```
Its body iterates `self.tasks`, skips a task whose `(provider, name)` pair was already seen (a BTreeSet of `(String, String)`), skips a task whose `source` does not exist, otherwise runs `let files = crate::pkgskill::snapshot_source(&task.source, &task.include)?;` and extends the output with `scan_unresolved(&task.name, &files, &self.project_root)`; then sort, dedup, return. End the file with exactly:
```
#[cfg(test)]
#[path = "closure_tests.rs"]
mod tests;
```
4. In crates/vibe-agent-projection/src/pkgskill/projection.rs, directly below the `pub use binding::{...};` statement that follows `mod binding;`, add:
```
#[path = "projection/closure.rs"]
mod closure;
pub use closure::UnresolvedReference;
```
In crates/vibe-agent-projection/src/pkgskill.rs add `UnresolvedReference` to the `pub use projection::{...}` list (cargo fmt sorts it).
5. Create closure_tests.rs starting with `use super::*; use std::collections::BTreeMap; use std::fs;` and a helper `fn files(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>>` that maps each pair to (key, bytes). Tests 1 to 4 call `scan_unresolved("demo", &files(&[...]), root.path())` where `root = tempfile::tempdir().unwrap()`. Test `one_dangling_script_reference_warns_once`: entries SKILL.md = `"---\nname: demo\n---\nRun `scripts/run.sh --json`, then `scripts/missing.sh`.\nSee [docs](https://example.com/a/b.md) and [guide](references/guide.md).\n"`, scripts/run.sh = `"#!/bin/sh\n"`, references/guide.md = `"Back to [skill](../SKILL.md) and [run](../scripts/run.sh).\n"`; root is an empty tempdir; assert the result equals `vec![UnresolvedReference { skill: "demo".into(), file: "SKILL.md".into(), reference: "scripts/missing.sh".into() }]`. Test `frontmatter_script_and_project_root_resolution`: entries references/cmd/plan.md = `"---\nscripts:\n  sh: scripts/bash/check.sh --json\n---\nRead `.specify/memory/constitution.md`.\n"`; root is a tempdir containing the file .specify/memory/constitution.md (any bytes); assert the result equals `vec![UnresolvedReference { skill: "demo".into(), file: "references/cmd/plan.md".into(), reference: "scripts/bash/check.sh".into() }]`. Test `fenced_urls_absolute_and_placeholder_tokens_are_ignored`: entries SKILL.md with exactly the bytes in the literal block below; root is an empty tempdir; assert the result is empty.
```
```sh
scripts/in-fence.sh
```
Use `/usr/bin/env.sh`, `~/x/y.sh`, `specs/<feature>/plan.md`, `$ROOT/a/b.sh` and `https://h/x/y.sh`.
```
Test `non_markdown_files_are_not_scanned`: entries only scripts/run.sh = "see `lib/missing.sh`"; root is an empty tempdir; assert the result is empty. Test `projection_reports_each_skill_once_across_agents`: a tempdir project with vibe.toml of exactly:
```
[package]
group = "org.example"
name = "demo"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "UPL-1.0"
description = "fixture"
keywords = ["fixture"]

[[skill]]
name = "demo"
path = "skills/demo"
agents = ["claude", "opencode"]
```
and skills/demo/SKILL.md = "Run `scripts/missing.sh`.\n"; call `crate::pkgskill::prepare_declared_skill_projection(project.path(), &crate::pkgskill::DeclaredSkillFilter::all(), crate::agents::Scope::Project).unwrap().unresolved_references().unwrap()` and assert it equals `vec![UnresolvedReference { skill: "demo".into(), file: "SKILL.md".into(), reference: "scripts/missing.sh".into() }]`.
6. Run `cargo fmt -p vibe-agent-projection`, `cargo clippy -p vibe-agent-projection --all-targets -- -D warnings`, `cargo test -p vibe-agent-projection --doc UnresolvedReference 2>&1 | grep -F 'test result: ok. 1 passed'`, then the check. Confirm with `wc -l` that projection.rs (547 before), pkgskill.rs (487 before) and closure.rs are each at or below 600.

**Decisions already made.**
- Severity is data only here: the method returns rows; it never fails because of a dangling reference and never changes install results.
- Scanned files: only keys ending in `.md`. Candidates come from YAML frontmatter words, inline code spans (first word) and Markdown link targets (first word, fragment cut). Fenced code blocks are skipped. Frontmatter is included because Spec Kit command templates name their scripts there (`scripts:` / `sh: scripts/bash/<name>.sh --json`).
- Frontmatter needs both an opening and a closing `---` line, each compared after trim_end; an unclosed opening line means the file has no frontmatter.
- Only frontmatter words are trimmed of surrounding punctuation; code-span and link-target words are used as they are, so a span like `scripts/x.sh,` fails the grammar and is not reported.
- Resolution order: the referencing file's directory inside the skill tree, then the skill root, then the project root (Spec Kit's `.specify/...` paths resolve against the project root).
- Placeholders (`<`, `{`, `$`), URLs, absolute paths and `~` paths are not references, because they fail the grammar or the explicit prefix rules.
- Rows are deduplicated per (skill, file, reference); skills are deduplicated by (provider, name) across agents and scopes; output is sorted.
- The scan reads the same source snapshot the projection writes (after hydration and include filtering), not the projected agent directories.
- Ordering: runs after ADV-CLIENTS-BRIDGE-1 in the same worktree, because both build, lint and test vibe-agent-projection and a half-done sibling would break this entry's clippy and check. The entries share no file.

**Not in this entry.** Not in this entry: printing the warning or adding JSON output in vibe-cli (ADV-CLIENTS-BRIDGE-3), scanning non-Markdown files, wiring the scan into vibe-mcp tools, editing the external org.speckit/speckit package, regenerating the root specmap.json.

**Depends on.** ADV-CLIENTS-BRIDGE-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
