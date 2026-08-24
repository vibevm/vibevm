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
fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[cfg(test)]
fn workspace_root(name: &str, members: &[&str]) -> String {
    let list = members
        .iter()
        .map(|m| format!("\"{}\"", crate::layout_paths::packages(m)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n\n[workspace]\nmembers = [{list}]\n"
    )
}

#[cfg(test)]
fn package_rel(tail: impl AsRef<Path>) -> String {
    crate::layout_paths::packages(tail)
}

#[cfg(test)]
fn package_path(tail: impl AsRef<Path>) -> PathBuf {
    crate::layout_paths::packages_path(tail)
}

#[cfg(test)]
fn stage_package(root: &Path, name: &str, info: &OriginInfo) -> StagedNode {
    let rel = package_rel(name);
    stage_node(&root.join(&rel), &rel, info).unwrap()
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
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn selection_includes_default_publish_and_skips_never() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["a", "b"]),
    );
    // a: default posture (publish = true). b: publish = false.
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
        &package_publish("b", "flow", "false"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, package_rel("a"));
    assert_eq!(sel.skipped.len(), 1);
    assert_eq!(sel.skipped[0].rel_path, package_rel("b"));
    assert!(sel.skipped[0].reason.contains("publish = false"));
}

#[test]
fn selection_honours_registry_list_form() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["a", "b"]),
    );
    // a: publish only to "vibespecs". b: publish only to "corp".
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package_publish("a", "flow", "[\"vibespecs\"]"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
        &package_publish("b", "flow", "[\"corp\"]"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, package_rel("a"));
    // b is reported skipped — its list excludes "vibespecs".
    assert_eq!(sel.skipped.len(), 1);
    assert!(sel.skipped[0].reason.contains("excludes registry"));
}

#[test]
fn selection_skips_non_package_nodes_without_reporting() {
    let tmp = TempDir::new().unwrap();
    // Root is a plain [project] — not a package; not reported.
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["a"]));
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
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
            "{}\n[workspace]\nmembers = [\"{}\"]\n",
            package("umbrella", "stack"),
            package_rel("a")
        ),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
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
        &workspace_root("mono", &["a", "b"]),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
        &package("b", "flow"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let member = package_rel("b");
    let sel = select_publishable_nodes(&ws, "vibespecs", Some(&member)).unwrap();
    assert_eq!(sel.publishable.len(), 1);
    assert_eq!(sel.publishable[0].rel_path, member);
}

#[test]
fn selection_member_filter_reports_excluded_target() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["a"]));
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package_publish("a", "flow", "false"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    // --member names a real node, but its posture excludes it.
    let member = package_rel("a");
    let sel = select_publishable_nodes(&ws, "vibespecs", Some(&member)).unwrap();
    assert!(sel.publishable.is_empty());
    assert_eq!(sel.skipped.len(), 1);
    assert!(sel.skipped[0].reason.contains("publish = false"));
}

#[test]
fn selection_member_filter_rejects_unknown_node() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["a"]));
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let ghost = package_rel("ghost");
    let err = select_publishable_nodes(&ws, "vibespecs", Some(&ghost)).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::MemberNotFound { .. }),
        "{err}"
    );
}

// ----- topological order -----

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#selective-publish",
    r = 1
)]
fn topo_order_is_dependency_first() {
    // b depends on a via a path dep — a must publish before b.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["a", "b"]),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    let expected = [package_rel("a"), package_rel("b")];
    assert_eq!(
        rels,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn topo_order_stable_without_edges() {
    // No inter-member deps — stable rel_path order.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["z", "a", "m"]),
    );
    write(
        tmp.path(),
        package_path("z/vibe.toml"),
        &package("z", "flow"),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("m/vibe.toml"),
        &package("m", "flow"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    let expected = [package_rel("a"), package_rel("m"), package_rel("z")];
    assert_eq!(
        rels,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn topo_order_chain_of_three() {
    // c → b → a. Publish order must be a, b, c.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["a", "b", "c"]),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
            package("b", "flow")
        ),
    );
    write(
        tmp.path(),
        package_path("c/vibe.toml"),
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
            package("c", "flow")
        ),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
    let ordered = topo_order(&ws, &sel.publishable).unwrap();
    let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
    let expected = [package_rel("a"), package_rel("b"), package_rel("c")];
    assert_eq!(
        rels,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn topo_order_detects_cycle() {
    // a depends on b, b depends on a — a hard error.
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["a", "b"]),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &format!(
            "{}\n[requires.packages]\n\"org.vibevm/b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
            package("a", "flow")
        ),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
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
        &workspace_root("mono", &["a", "b"]),
    );
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("b/vibe.toml"),
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
    let expected = [package_rel("a"), package_rel("b")];
    assert_eq!(
        rels,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

mod tests_staging;
