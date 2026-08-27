//! Unit tests for the pkgskill cell, out-of-line per the file-length
//! budget. Included via cfg(test) #[path] mod tests; the module-tree
//! position is unchanged, so use super::* resolves as in the inline form.
//!
//! Non-`#[test]` helpers carry `#[cfg(test)]` so the file-grain conform
//! frontend scopes their `unwrap`s as test code.

use super::*;
use specmark::verifies;

/// Build a package skill body on disk: `<dir>/skills/<name>/SKILL.md`
/// plus an asset, returning the skill-body dir.
#[cfg(test)]
fn make_skill_body(root: &Path, body: &str) -> std::path::PathBuf {
    let dir = root.join("skills").join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), body).unwrap();
    fs::write(dir.join("ref.md"), "asset").unwrap();
    dir
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill", r = 3)]
fn projects_dir_skill_and_is_idempotent() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let body = make_skill_body(pkg.path(), "the skill");

    let r = install_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        false,
    )
    .unwrap();
    assert_eq!(r.status, "created");
    let landed = proj
        .path()
        .join(".claude")
        .join("skills")
        .join("demo")
        .join("SKILL.md");
    assert!(landed.is_file());
    assert_eq!(fs::read_to_string(&landed).unwrap(), "the skill");
    assert!(proj.path().join(".claude/skills/demo/ref.md").is_file());

    // Second run with identical bytes → unchanged.
    let r2 = install_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        false,
    )
    .unwrap();
    assert_eq!(r2.status, "unchanged");
}

#[test]
fn updates_when_body_diverges_and_drops_stale_files() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let body = make_skill_body(pkg.path(), "v1");
    install_package_skill(
        Agent::OpenCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        false,
    )
    .unwrap();

    // Drop the asset and change the body → the projection must follow.
    fs::remove_file(body.join("ref.md")).unwrap();
    fs::write(body.join("SKILL.md"), "v2").unwrap();
    let r = install_package_skill(
        Agent::OpenCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        false,
    )
    .unwrap();
    assert_eq!(r.status, "updated");
    let base = proj.path().join(".opencode").join("skills").join("demo");
    assert_eq!(fs::read_to_string(base.join("SKILL.md")).unwrap(), "v2");
    assert!(!base.join("ref.md").exists(), "stale file must be dropped");
}

#[test]
fn single_file_source_lands_under_skill_dir() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let file = pkg.path().join("SKILL.md");
    fs::write(&file, "single").unwrap();
    let r = install_package_skill(
        Agent::Codex,
        Scope::Project,
        Some(proj.path()),
        "solo",
        &file,
        false,
    )
    .unwrap();
    assert_eq!(r.status, "created");
    assert_eq!(
        fs::read_to_string(proj.path().join(".agents/skills/solo/SKILL.md")).unwrap(),
        "single"
    );
}

#[test]
fn skipped_for_skill_unsupported_agent() {
    let proj = tempfile::tempdir().unwrap();
    let file = proj.path().join("SKILL.md");
    fs::write(&file, "x").unwrap();
    // Cursor is JSON-config-only — no filesystem skill loader.
    let r = install_package_skill(
        Agent::Cursor,
        Scope::Project,
        Some(proj.path()),
        "k",
        &file,
        false,
    )
    .unwrap();
    assert_eq!(r.status, "skipped");
    assert!(r.path.is_none());
}

#[test]
fn dry_run_writes_nothing() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let body = make_skill_body(pkg.path(), "x");
    let r = install_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        true,
    )
    .unwrap();
    assert_eq!(r.status, "would-create");
    assert!(!proj.path().join(".claude").exists());
}

#[test]
fn uninstall_removes_then_reports_absent() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let body = make_skill_body(pkg.path(), "x");
    install_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &body,
        false,
    )
    .unwrap();
    let r = uninstall_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        false,
    )
    .unwrap();
    assert_eq!(r.status, "removed");
    assert!(!proj.path().join(".claude/skills/demo").exists());
    let r2 = uninstall_package_skill(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        false,
    )
    .unwrap();
    assert_eq!(r2.status, "absent");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#skill-include",
    r = 1
)]
fn include_projects_only_matching_files() {
    let pkg = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    // A noisy upstream subtree: the wanted SKILL.md + a wanted ref,
    // plus unrelated build junk and a top-level noise file.
    let dir = pkg.path().join("upstream");
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::create_dir_all(dir.join("build")).unwrap();
    fs::write(dir.join("SKILL.md"), "skill").unwrap();
    fs::write(dir.join("references").join("a.md"), "ref").unwrap();
    fs::write(dir.join("build").join("junk.o"), "junk").unwrap();
    fs::write(dir.join("README.txt"), "noise").unwrap();

    let r = install_package_skill_selecting(
        Agent::ClaudeCode,
        Scope::Project,
        Some(proj.path()),
        "demo",
        &dir,
        &["SKILL.md".to_string(), "references/**/*.md".to_string()],
        false,
    )
    .unwrap();
    assert_eq!(r.status, "created");
    let base = proj.path().join(".claude").join("skills").join("demo");
    assert!(base.join("SKILL.md").is_file());
    assert!(base.join("references/a.md").is_file());
    assert!(!base.join("build/junk.o").exists(), "junk must be excluded");
    assert!(!base.join("README.txt").exists(), "noise must be excluded");
}

#[test]
fn glob_match_semantics() {
    assert!(glob_match("SKILL.md", "SKILL.md"));
    assert!(!glob_match("SKILL.md", "references/SKILL.md"));
    assert!(glob_match("*.md", "a.md"));
    assert!(!glob_match("*.md", "a/b.md")); // single * stays in-segment
    assert!(glob_match("references/**/*.md", "references/a.md"));
    assert!(glob_match("references/**/*.md", "references/x/y.md"));
    assert!(!glob_match("references/**/*.md", "references/a.txt"));
    assert!(glob_match("docs/", "docs/x/y.md")); // trailing / = subtree
    assert!(glob_match("docs/", "docs"));
    assert!(!glob_match("docs/", "docsx"));
    assert!(glob_match("a?c.md", "abc.md"));
    assert!(!glob_match("a?c.md", "a/c.md"));
}
