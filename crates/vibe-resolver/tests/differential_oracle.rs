//! Differential oracle for the `DepProvider` cell pair (GUIDE-RUST §7,
//! R-040; PLAYBOOK Phase 3): the `local-registry` and `multi-registry`
//! provider cells must drive `NaiveDepSolver` to the **same resolved
//! graph** over equivalent registry content.
//!
//! Hermetic by construction: the multi side runs against real bare git
//! repositories under a tempdir (file-path remotes, no network) — also
//! the first brick of the AUDIT P1 "hermetic harness driving the git
//! registry against real `file://` repositories".
//!
//! Divergence list (documented, per the playbook's "documented-
//! divergence list otherwise"): none known today. A future divergence
//! is recorded here with its debt id before the assertion is relaxed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use specmark::verifies;
use vibe_core::PackageRef;
use vibe_registry::{LocalRegistry, MultiRegistryResolver, ShellGit};
use vibe_resolver::{
    DepSolver, LocalRegistryProvider, MultiRegistryProvider, NaiveDepSolver, ResolvedGraph,
    ResolvoDepSolver,
};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.email=oracle@vibevm.test",
            "-c",
            "user.name=Oracle",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawning git");
    assert!(
        out.status.success(),
        "git {args:?} failed in {}:\n{}",
        cwd.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// One package version: `(semver, vibe.toml body)`.
type VersionedManifest<'a> = (&'a str, String);

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n{extra}"
    )
}

/// Multi side: a bare repo `<orgdir>/org.vibevm.<name>.git` whose tags
/// `v<semver>` each carry that version's manifest.
fn seed_git_package(orgdir: &Path, scratch: &Path, name: &str, versions: &[VersionedManifest]) {
    let work = scratch.join(format!("work-{name}"));
    std::fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q"]);
    for (ver, body) in versions {
        std::fs::write(work.join("vibe.toml"), body).unwrap();
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "-m", &format!("v{ver}")]);
        git(&work, &["tag", &format!("v{ver}")]);
    }
    let bare: PathBuf = orgdir.join(format!("org.vibevm_{name}.git"));
    git(
        scratch,
        &[
            "clone",
            "--bare",
            "-q",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
}

/// Local side: `<root>/org.vibevm/<name>/v<semver>/vibe.toml`.
fn seed_local_package(root: &Path, name: &str, versions: &[VersionedManifest]) {
    for (ver, body) in versions {
        let dir = root.join("org.vibevm").join(name).join(format!("v{ver}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vibe.toml"), body).unwrap();
    }
}

/// Order- and provenance-insensitive view of a resolved graph.
fn normalize(graph: &ResolvedGraph) -> Vec<(String, String, String, bool, Vec<String>)> {
    let mut rows: Vec<_> = graph
        .iter()
        .map(|n| {
            let mut deps: Vec<String> = n.dependencies.iter().map(|d| d.to_string()).collect();
            deps.sort();
            (
                n.group.to_string(),
                n.name.clone(),
                n.version.to_string(),
                n.is_root,
                deps,
            )
        })
        .collect();
    rows.sort();
    rows
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-002#solver")]
fn provider_pair_agrees_on_solvable_inputs() {
    if !git_available() {
        eprintln!("differential oracle: `git` not on PATH — skipping (hermetic harness needs it)");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let orgdir = tmp.path().join("org");
    let localroot = tmp.path().join("local");
    let scratch = tmp.path().join("scratch");
    let cache = tmp.path().join("cache");
    for d in [&orgdir, &localroot, &scratch, &cache] {
        std::fs::create_dir_all(d).unwrap();
    }

    // Equivalent content on both sides: `a` requires b@^0.1 (two b
    // versions published — the pair must agree on picking 0.1.5), `c`
    // is a standalone second root.
    let b_versions: Vec<VersionedManifest> = vec![
        ("0.1.0", manifest("b", "0.1.0", "")),
        ("0.1.5", manifest("b", "0.1.5", "")),
    ];
    let a_versions: Vec<VersionedManifest> = vec![(
        "0.1.0",
        manifest(
            "a",
            "0.1.0",
            "\n[requires.packages]\n\"org.vibevm/b\" = \"^0.1\"\n",
        ),
    )];
    let c_versions: Vec<VersionedManifest> = vec![("0.2.0", manifest("c", "0.2.0", ""))];

    for (name, versions) in [("a", &a_versions), ("b", &b_versions), ("c", &c_versions)] {
        seed_git_package(&orgdir, &scratch, name, versions);
        seed_local_package(&localroot, name, versions);
    }

    let roots = [
        PackageRef::parse("org.vibevm/a").unwrap(),
        PackageRef::parse("org.vibevm/c").unwrap(),
    ];

    // Cell 1: local-registry provider.
    let local = LocalRegistry::new(&localroot).unwrap();
    let local_graph = NaiveDepSolver::new(LocalRegistryProvider::new(&local))
        .solve(&roots)
        .expect("local-registry arm must solve");

    // Cell 2: multi-registry provider over the bare git org.
    let org_url = orgdir.to_string_lossy().replace('\\', "/");
    let section: vibe_core::manifest::RegistrySection = toml::from_str(&format!(
        "name = \"oracle\"\nurl = \"{org_url}\"\nnaming = \"fqdn\"\n"
    ))
    .unwrap();
    let multi = MultiRegistryResolver::from_manifest(
        &[section],
        &[],
        &[],
        cache,
        Arc::new(ShellGit::new()),
        3600,
    )
    .expect("building the multi-registry resolver");
    let multi_graph = NaiveDepSolver::new(MultiRegistryProvider::new(&multi))
        .solve(&roots)
        .expect("multi-registry arm must solve");

    assert_eq!(
        normalize(&local_graph),
        normalize(&multi_graph),
        "the DepProvider cell pair diverged on equivalent registry content"
    );
    // And the agreed answer is the right one: b resolved to 0.1.5.
    let b = local_graph
        .find(&vibe_core::Group::parse("org.vibevm").unwrap(), "b")
        .expect("b in graph");
    assert_eq!(b.version.to_string(), "0.1.5");

    // Resolvo over the very same real providers (S1 / PROP-017): the
    // production `VersionEnumerator` path must drive resolvo to the same
    // graph naive reaches on this conflict-free world — proving
    // `MultiRegistryResolver::list_versions` and the provider impls feed
    // candidates correctly off real `file://` git repos and local disk.
    let local_resolvo = ResolvoDepSolver::new(LocalRegistryProvider::new(&local))
        .solve(&roots)
        .expect("local-registry resolvo arm must solve");
    let multi_resolvo = ResolvoDepSolver::new(MultiRegistryProvider::new(&multi))
        .solve(&roots)
        .expect("multi-registry resolvo arm must solve");
    assert_eq!(
        normalize(&local_graph),
        normalize(&local_resolvo),
        "resolvo diverged from naive over the local-registry provider"
    );
    assert_eq!(
        normalize(&multi_graph),
        normalize(&multi_resolvo),
        "resolvo diverged from naive over the multi-registry provider"
    );
}
