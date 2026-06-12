//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use super::*;
use specmark::verifies;
use std::fs;
use tempfile::TempDir;

#[cfg(test)]
fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[cfg(test)]
fn workspace_root(name: &str, members: &[&str]) -> String {
    let list = members
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n\n[workspace]\nmembers = [{list}]\n"
    )
}

#[cfg(test)]
fn package(name: &str, kind: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n"
    )
}

#[cfg(test)]
fn package_publish(name: &str, kind: &str, publish: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n\
         publish = {publish}\n"
    )
}

#[cfg(test)]
fn origin_info() -> OriginInfo {
    OriginInfo {
        upstream: "https://github.com/you/monorepo".to_string(),
        commit: Some("abc123def456".to_string()),
        generated_by: "vibe 0.1.0".to_string(),
        generated_at: "2026-05-21T00:00:00Z".to_string(),
    }
}

// ----- selection -----

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn selection_includes_default_publish_and_skips_never() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    // a: default posture (publish = true). b: publish = false.
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &package_publish("b", "flow", "false"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, "packages/a");
    assert_eq!(sel.skipped.len(), 1);
    assert_eq!(sel.skipped[0].rel_path, "packages/b");
    assert!(sel.skipped[0].reason.contains("publish = false"));
}

#[test]
fn selection_honours_registry_list_form() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    // a: publish only to "vibespecs". b: publish only to "corp".
    write(
        tmp.path(),
        "packages/a/vibe.toml",
        &package_publish("a", "flow", "[\"vibespecs\"]"),
    );
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &package_publish("b", "flow", "[\"corp\"]"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, "packages/a");
    // b is reported skipped — its list excludes "vibespecs".
    assert_eq!(sel.skipped.len(), 1);
    assert!(sel.skipped[0].reason.contains("excludes registry"));
}

#[test]
fn selection_skips_non_package_nodes_without_reporting() {
    let tmp = TempDir::new().unwrap();
    // Root is a plain [project] — not a package; not reported.
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    // The [project] root is not in `skipped` — it is not a package.
    assert!(sel.skipped.is_empty());
}

#[test]
fn selection_includes_root_when_it_is_a_package() {
    // cargo-style: root carries [package] + [workspace]. PROP-007 §2.9.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &format!(
            "{}\n[workspace]\nmembers = [\"packages/a\"]\n",
            package("umbrella", "stack")
        ),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 2);
    assert!(sel.publishable.iter().any(|n| n.rel_path == "."));
}

#[test]
fn selection_member_filter_narrows_to_one() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/b/vibe.toml", &package("b", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", Some("packages/b")).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, "packages/b");
}

#[test]
fn selection_member_filter_reports_excluded_target() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a"]),
    );
    write(
        tmp.path(),
        "packages/a/vibe.toml",
        &package_publish("a", "flow", "false"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    // --member names a real node, but its posture excludes it.
    let sel = select_publishable_nodes(&ws, "vibespecs", Some("packages/a")).unwrap();
    assert!(sel.publishable.is_empty());
    assert_eq!(sel.skipped.len(), 1);
    assert!(sel.skipped[0].reason.contains("publish = false"));
}

#[test]
fn selection_member_filter_rejects_unknown_node() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let err = select_publishable_nodes(&ws, "vibespecs", Some("packages/ghost")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::MemberNotFound { .. }),
        "{err}"
    );
}

// ----- topological order -----

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn topo_order_is_dependency_first() {
    // b depends on a via a path dep — a must publish before b.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    assert_eq!(rels, vec!["packages/a", "packages/b"]);
}

#[test]
fn topo_order_stable_without_edges() {
    // No inter-member deps — stable rel_path order.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/z", "packages/a", "packages/m"]),
    );
    write(tmp.path(), "packages/z/vibe.toml", &package("z", "flow"));
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/m/vibe.toml", &package("m", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    assert_eq!(rels, vec!["packages/a", "packages/m", "packages/z"]);
}

#[test]
fn topo_order_chain_of_three() {
    // c → b → a. Publish order must be a, b, c.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b", "packages/c"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    write(
        tmp.path(),
        "packages/c/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
            package("c", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    assert_eq!(rels, vec!["packages/a", "packages/b", "packages/c"]);
}

#[test]
fn topo_order_detects_cycle() {
    // a depends on b, b depends on a — a hard error.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    write(
        tmp.path(),
        "packages/a/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
            package("a", "flow")
        ),
    );
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let err = topo_order(&ws, &sel.publishable).unwrap_err();
    assert!(matches!(err, WorkspaceError::NestingCycle { .. }), "{err}");
}

#[test]
fn topo_order_path_dep_outside_selection_imposes_no_edge() {
    // b path-deps an external dir that is not a selected node. That
    // imposes no ordering — both still publish, rel_path order.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/a", "packages/b"]),
    );
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(
        tmp.path(),
        "packages/b/vibe.toml",
        &format!(
            "{}\n[requires.packages]\n\
             \"org.vibevm/ext\" = {{ path = \"../../external\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    assert_eq!(rels, vec!["packages/a", "packages/b"]);
}

// ----- staging -----

#[test]
fn stage_node_writes_origin_section() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/a/spec/X.md", "spec content");
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    let origin = manifest.origin.as_ref().expect("origin written");
    assert_eq!(origin.upstream, "https://github.com/you/monorepo");
    assert_eq!(origin.path, "packages/a");
    assert_eq!(origin.commit.as_deref(), Some("abc123def456"));
    assert_eq!(origin.generated_by, "vibe 0.1.0");
    assert_eq!(origin.generated_at, "2026-05-21T00:00:00Z");
    // Spec content travelled.
    assert!(staged.staging.path().join("spec/X.md").is_file());
}

#[test]
fn stage_node_excludes_git_and_vibe_dirs() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/a/.git/HEAD", "ref: refs/heads/main");
    write(tmp.path(), "packages/a/.git/objects/x", "obj");
    write(tmp.path(), "packages/a/.vibe/cache.bin", "cache");
    write(tmp.path(), "packages/a/keep.md", "keep me");
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    assert!(!staged.staging.path().join(".git").exists());
    assert!(!staged.staging.path().join(".vibe").exists());
    assert!(staged.staging.path().join("keep.md").is_file());
}

#[test]
fn stage_node_prepends_readme_banner() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/a/README.md", "# Original readme\n");
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    let readme = fs::read_to_string(staged.staging.path().join("README.md")).unwrap();
    assert!(readme.contains("Generated copy — do not contribute here"));
    assert!(readme.contains("https://github.com/you/monorepo"));
    // Original content preserved below the banner.
    assert!(readme.contains("# Original readme"));
    // Banner comes first.
    assert!(readme.starts_with("<!-- vibevm:generated-copy -->"));
}

#[test]
fn stage_node_creates_readme_when_absent() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    let readme_path = staged.staging.path().join("README.md");
    assert!(readme_path.is_file());
    let readme = fs::read_to_string(&readme_path).unwrap();
    assert!(readme.contains("Generated copy — do not contribute here"));
}

#[test]
fn stage_node_writes_pr_template() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    let pr_template = fs::read_to_string(
        staged
            .staging
            .path()
            .join(".github/PULL_REQUEST_TEMPLATE.md"),
    )
    .unwrap();
    assert!(pr_template.contains("does not accept pull requests"));
    assert!(pr_template.contains("https://github.com/you/monorepo"));
    assert!(pr_template.contains("org.vibevm/a"));
}

#[test]
fn stage_node_sets_generated_copy_description() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    let desc = manifest
        .package
        .as_ref()
        .and_then(|p| p.description.clone())
        .expect("description set");
    assert!(desc.contains("Generated copy of `org.vibevm/a`"));
    assert!(desc.contains("https://github.com/you/monorepo"));
}

#[test]
fn stage_node_omits_commit_when_none() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    let mut info = origin_info();
    info.commit = None;
    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &info).unwrap();
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    assert!(manifest.origin.as_ref().unwrap().commit.is_none());
}

#[test]
fn stage_node_regenerates_boot_for_the_published_shape() {
    // PROP-009 §2.11: the dev tree's boot artifacts reference the
    // workspace `vibedeps/` slots, which do not exist in a standalone
    // published copy. `stage_node` regenerates them from the staged
    // node's own authored boot so nothing dangles.
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
    write(tmp.path(), "packages/a/spec/boot/00-core.md", "# core");
    // A stale dev-tree INDEX.md pointing at a workspace `vibedeps/`
    // slot — exactly what must not be published verbatim.
    write(
        tmp.path(),
        "packages/a/spec/boot/INDEX.md",
        "schema = 1\n\n[[entry]]\n\
         path = \"vibedeps/flow-dep/1.0.0/boot/dep.md\"\nkind = \"static\"\n",
    );
    // A stale INLINE.md left over from a dev-tree inline dependency.
    write(
        tmp.path(),
        "packages/a/spec/boot/INLINE.md",
        "stale inline lane",
    );
    write(tmp.path(), "packages/a/CLAUDE.md", "stale dev redirect");

    let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();

    // The dangling `vibedeps/` reference is gone; the node's own
    // authored foundation boot is named instead.
    let index = fs::read_to_string(staged.staging.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        !index.contains("vibedeps/"),
        "the published INDEX.md must not dangle on a workspace vibedeps/ slot:\n{index}"
    );
    assert!(
        index.contains("spec/boot/00-core.md"),
        "the published INDEX.md must name the node's own authored boot:\n{index}"
    );
    // No inline dependencies in the published shape — the stale
    // INLINE.md is removed.
    assert!(
        !staged.staging.path().join("spec/boot/INLINE.md").exists(),
        "a stale INLINE.md must be cleared in the published copy"
    );
    // The redirect is regenerated as a thin generated pointer.
    let claude = fs::read_to_string(staged.staging.path().join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("Generated by vibe") && claude.contains("spec/boot/INDEX.md"),
        "CLAUDE.md must be a regenerated redirect:\n{claude}"
    );
}
