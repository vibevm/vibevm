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
fn project(name: &str) -> String {
    format!("[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n")
}

#[cfg(test)]
fn package(name: &str, kind: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n"
    )
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

#[test]
fn standalone_project_is_its_own_root() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "vibe.toml", &project("solo"));
    let ws = Workspace::discover(tmp.path()).unwrap();
    assert!(ws.is_standalone());
    assert!(ws.members.is_empty());
    assert_eq!(ws.root_manifest.require_project().unwrap().name, "solo");
}

#[test]
fn explicit_members_load() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/flow-wal", "packages/feat-auth"]),
    );
    write(
        tmp.path(),
        "packages/flow-wal/vibe.toml",
        &package("wal", "flow"),
    );
    write(
        tmp.path(),
        "packages/feat-auth/vibe.toml",
        &package("auth", "feat"),
    );

    let ws = Workspace::load(tmp.path()).unwrap();
    assert!(!ws.is_standalone());
    assert_eq!(ws.members.len(), 2);
    // Sorted by rel_path: feat-auth before flow-wal.
    assert_eq!(ws.members[0].rel_path, "packages/feat-auth");
    assert_eq!(ws.members[1].rel_path, "packages/flow-wal");
    assert_eq!(ws.members[0].depth, 0);
    assert_eq!(
        ws.member_by_rel_path("packages/flow-wal")
            .unwrap()
            .manifest
            .require_package()
            .unwrap()
            .name,
        "wal"
    );
}

#[test]
fn glob_members_expand_and_skip_non_packages() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/*"]),
    );
    write(
        tmp.path(),
        "packages/flow-a/vibe.toml",
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        "packages/flow-b/vibe.toml",
        &package("b", "flow"),
    );
    // A directory under packages/ with no manifest — a glob match must
    // silently skip it.
    fs::create_dir_all(tmp.path().join("packages/scratch")).unwrap();
    write(tmp.path(), "packages/scratch/notes.txt", "ignore me");

    let ws = Workspace::load(tmp.path()).unwrap();
    assert_eq!(ws.members.len(), 2);
    assert_eq!(ws.members[0].rel_path, "packages/flow-a");
    assert_eq!(ws.members[1].rel_path, "packages/flow-b");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#nesting", r = 1)]
fn nested_workspace_recurses_with_depth() {
    let tmp = TempDir::new().unwrap();
    // Root lists a sub-workspace as a member.
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
    // The sub node is itself a [workspace] AND a package.
    write(
        tmp.path(),
        "sub/vibe.toml",
        &format!(
            "{}\n[workspace]\nmembers = [\"leaf\"]\n",
            package("sub", "stack")
        ),
    );
    write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

    let ws = Workspace::load(tmp.path()).unwrap();
    assert_eq!(ws.members.len(), 2);
    let sub = ws.member_by_rel_path("sub").unwrap();
    assert_eq!(sub.depth, 0);
    let leaf = ws.member_by_rel_path("sub/leaf").unwrap();
    assert_eq!(leaf.depth, 1);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#nesting", r = 1)]
fn discover_from_member_finds_absolute_root() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
    write(
        tmp.path(),
        "sub/vibe.toml",
        &format!(
            "{}\n[workspace]\nmembers = [\"leaf\"]\n",
            package("sub", "stack")
        ),
    );
    write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

    // Discovery from the deepest leaf must bubble up to the absolute root.
    let ws = Workspace::discover(tmp.path().join("sub/leaf")).unwrap();
    assert_eq!(ws.root, canonical(tmp.path()).unwrap());
    assert_eq!(ws.members.len(), 2);
    assert!(!ws.is_standalone());
}

#[test]
fn discover_from_member_of_unrelated_workspace_picks_the_enclosing_one() {
    let tmp = TempDir::new().unwrap();
    // The outer [workspace] does NOT list `sub` — it lists `other`.
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("outer", &["other"]),
    );
    write(tmp.path(), "other/vibe.toml", &package("other", "flow"));
    // `sub` is its own [workspace], not reachable from `outer`.
    write(
        tmp.path(),
        "sub/vibe.toml",
        &workspace_root("sub-ws", &["leaf"]),
    );
    write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

    let ws = Workspace::discover(tmp.path().join("sub/leaf")).unwrap();
    // The enclosing workspace is `sub`, not the unrelated `outer`.
    assert_eq!(ws.root, canonical(&tmp.path().join("sub")).unwrap());
    assert_eq!(ws.members.len(), 1);
    assert_eq!(ws.members[0].rel_path, "leaf");
}

#[test]
fn missing_explicit_member_errors() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &workspace_root("mono", &["packages/ghost"]),
    );
    let err = Workspace::load(tmp.path()).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::MemberNotFound { .. }),
        "{err}"
    );
}

#[test]
fn nesting_cycle_is_detected() {
    let tmp = TempDir::new().unwrap();
    // Root lists `sub`; `sub` lists `..` back to the root directory.
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
    write(
        tmp.path(),
        "sub/vibe.toml",
        &format!(
            "{}\n[workspace]\nmembers = [\"..\"]\n",
            package("sub", "flow")
        ),
    );
    let err = Workspace::load(tmp.path()).unwrap_err();
    assert!(matches!(err, WorkspaceError::NestingCycle { .. }), "{err}");
}

#[test]
fn iter_nodes_yields_root_then_members() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "vibe.toml", &workspace_root("mono", &["pkg"]));
    write(tmp.path(), "pkg/vibe.toml", &package("pkg", "flow"));
    let ws = Workspace::load(tmp.path()).unwrap();
    let nodes: Vec<&str> = ws.iter_nodes().map(|(p, _)| p).collect();
    assert_eq!(nodes, vec![".", "pkg"]);
    assert_eq!(ws.lockfile_path(), ws.root.join("vibe.lock"));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#versions", r = 1)]
fn version_var_resolves_from_root_workspace() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
         [workspace]\nmembers = [\"pkg\"]\n\n\
         [workspace.versions]\ncore = \"^0.2\"\n",
    );
    write(
        tmp.path(),
        "pkg/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"core\" }\n",
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let pkg = ws.member_by_rel_path("pkg").unwrap();
    // The placeholder is resolved: var_packages drained into packages.
    assert!(pkg.manifest.requires.var_packages.is_empty());
    assert_eq!(pkg.manifest.requires.packages.len(), 1);
    assert_eq!(
        pkg.manifest.requires.packages[0].to_string(),
        "org.vibevm/wal@^0.2"
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#versions", r = 1)]
fn version_var_matryoshka_nearest_wins() {
    let tmp = TempDir::new().unwrap();
    // Root defines core = ^0.1; a nested workspace overrides it to ^0.9.
    write(
        tmp.path(),
        "vibe.toml",
        "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
         [workspace]\nmembers = [\"sub\"]\n\n\
         [workspace.versions]\ncore = \"^0.1\"\n",
    );
    write(
        tmp.path(),
        "sub/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"sub\"\nkind = \"stack\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"leaf\"]\n\n\
         [workspace.versions]\ncore = \"^0.9\"\n",
    );
    write(
        tmp.path(),
        "sub/leaf/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"leaf\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"core\" }\n",
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    let leaf = ws.member_by_rel_path("sub/leaf").unwrap();
    // The nearest enclosing [workspace.versions] — sub's — wins.
    assert_eq!(
        leaf.manifest.requires.packages[0].to_string(),
        "org.vibevm/wal@^0.9"
    );
}

#[test]
fn unknown_version_var_errors() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
         [workspace]\nmembers = [\"pkg\"]\n",
    );
    write(
        tmp.path(),
        "pkg/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"ghost\" }\n",
    );
    let err = Workspace::load(tmp.path()).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::UnknownVersionVar { .. }),
        "{err}"
    );
}
