//! Apply a resolution to the workspace — the install half of the loading
//! model (PROP-009 §2.7).
//!
//! [`apply_resolution`] takes a discovered [`Workspace`] and a resolved,
//! fetched dependency set, and:
//!
//! 1. materialises each resolved package into its `vibedeps/` slot
//!    ([`crate::vibedeps`]);
//! 2. computes every node's effective boot ([`crate::boot`]) and writes
//!    its boot artifacts ([`crate::boot_artifacts`]).
//!
//! It is decoupled from the depsolver and the registry: the caller —
//! workspace-aware `vibe install` — runs `Workspace::discover` and the
//! unified resolution, then hands the result here as [`ResolvedDep`]s.
//! This keeps the orchestration unit-testable without the registry stack,
//! the same decoupling [`crate::boot`] uses.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::PackageKind;
use vibe_core::manifest::{BootCategory, Manifest};

use crate::boot::{self, AuthoredBoot, DependencyBoot, NodeBootInputs};
use crate::{Workspace, WorkspaceError, boot_artifacts, vibedeps};

/// A resolved, fetched dependency ready to materialise — the minimum the
/// install orchestrator needs, decoupled from the registry's richer
/// `CachedPackage`.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    /// On-disk directory holding the package's fetched content tree — the
    /// source `vibedeps` materialisation copies verbatim.
    pub content_dir: PathBuf,
    /// The package's parsed manifest (its `vibe.toml`) — read for the
    /// `[boot_snippet]` contribution.
    pub manifest: Manifest,
    /// `(kind, name)` of every package this one directly requires — the
    /// edges of the dependency-boot topological order.
    pub requires: Vec<(PackageKind, String)>,
}

/// What [`apply_resolution`] did — for the caller to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    /// `vibedeps/` slot paths materialised, in resolution order.
    pub materialised: Vec<String>,
    /// `rel_path` of every node whose boot artifacts were regenerated.
    pub nodes_regenerated: Vec<String>,
}

/// Materialise a resolution into the workspace and regenerate every node's
/// boot artifacts (PROP-009 §2.7).
///
/// Materialisation is workspace-wide — one `vibedeps/` slot per resolved
/// package at the absolute root. Boot artifacts are computed per node: the
/// root from the whole resolution, a member from its own `[requires]`
/// closure, with the absolute root's foundation boot inherited downward.
pub fn apply_resolution(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<InstallOutcome, WorkspaceError> {
    // 1. Materialise every resolved package into its `vibedeps/` slot.
    let mut materialised = Vec::with_capacity(resolution.len());
    for dep in resolution {
        vibedeps::materialise(
            &workspace.root,
            dep.kind,
            &dep.name,
            &dep.version,
            &dep.content_dir,
        )?;
        materialised.push(vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version));
    }

    // 2. Regenerate every node's boot artifacts from the resolution.
    let nodes_regenerated = regenerate_boot_from(workspace, resolution)?;

    Ok(InstallOutcome {
        materialised,
        nodes_regenerated,
    })
}

/// Regenerate every node's boot artifacts from a given `resolution` — the
/// boot half of [`apply_resolution`], without materialising. Returns the
/// `rel_path` of every node whose artifacts were written.
pub fn regenerate_boot_from(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
) -> Result<Vec<String>, WorkspaceError> {
    // The absolute root's foundation boot — inherited by every member
    // (PROP-009 §2.2: inherited foundation flows down).
    let root_foundation: Vec<AuthoredBoot> = node_own_boot(&workspace.root, ".")?
        .into_iter()
        .filter(|b| b.category == Some(BootCategory::Foundation))
        .collect();

    let mut nodes_regenerated = Vec::new();
    for (rel, manifest) in workspace.iter_nodes() {
        let node_dir = workspace.node_abs_path(rel);
        let own = node_own_boot(&node_dir, rel)?;
        let inherited: Vec<AuthoredBoot> = if rel == "." {
            Vec::new()
        } else {
            root_foundation.clone()
        };
        let deps = node_dependency_boot(manifest, resolution);
        let effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &deps,
            default_link: manifest.boot.default_link,
        })?;
        boot_artifacts::write_boot_artifacts(&node_dir, &workspace.root, &effective)?;
        nodes_regenerated.push(rel.to_string());
    }
    Ok(nodes_regenerated)
}

/// Regenerate every node's boot artifacts from the materialised `vibedeps/`
/// state already on disk — no fresh resolution, no re-materialisation.
///
/// Used by `vibe uninstall` (after a slot is removed) and, later, by
/// `vibe reinstall`. The resolution is reconstructed by reading each
/// `vibedeps/` slot's own manifest.
pub fn regenerate_boot(workspace: &Workspace) -> Result<Vec<String>, WorkspaceError> {
    let resolution = read_materialised(&workspace.root)?;
    regenerate_boot_from(workspace, &resolution)
}

/// Reconstruct the resolution by reading every `vibedeps/` slot's manifest.
/// A slot whose `vibe.toml` is missing or carries no `[package]` table is
/// skipped — it is not a materialised package.
fn read_materialised(workspace_root: &Path) -> Result<Vec<ResolvedDep>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibedeps::VIBEDEPS_DIR);
    if !vibedeps_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for kind_name in fs::read_dir(&vibedeps_dir).map_err(|e| io_err(&vibedeps_dir, e))? {
        let kind_name = kind_name.map_err(|e| io_err(&vibedeps_dir, e))?;
        let kind_name_dir = kind_name.path();
        if !kind_name_dir.is_dir() {
            continue;
        }
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let slot = version.path();
            let manifest_path = slot.join("vibe.toml");
            if !slot.is_dir() || !manifest_path.is_file() {
                continue;
            }
            let manifest =
                Manifest::read(&manifest_path).map_err(|e| WorkspaceError::Manifest {
                    path: manifest_path.clone(),
                    source: Box::new(e),
                })?;
            let Some(pkg) = &manifest.package else {
                continue;
            };
            let requires: Vec<(PackageKind, String)> = manifest
                .requires
                .iter_pkgrefs()
                .map(|(k, n)| (k, n.to_string()))
                .collect();
            out.push(ResolvedDep {
                kind: pkg.kind,
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                content_dir: slot.clone(),
                manifest: manifest.clone(),
                requires,
            });
        }
    }
    Ok(out)
}

/// Discover a node's own authored boot files — every `*.md` in its
/// `spec/boot/`, minus the generated `INLINE.md` / `INDEX.md`. The
/// user-owned `00-core.md` / `90-user.md` are `Foundation` / `UserOverride`
/// by name convention; any other authored file is mid-band (`None`).
fn node_own_boot(node_dir: &Path, node_rel: &str) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let boot_dir = node_dir.join("spec").join("boot");
    if !boot_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&boot_dir).map_err(|e| io_err(&boot_dir, e))? {
        let entry = entry.map_err(|e| io_err(&boot_dir, e))?;
        let path = entry.path();
        if !entry.file_type().map_err(|e| io_err(&path, e))?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".md") {
            continue;
        }
        // The generated artifacts are not authored boot.
        if name == boot_artifacts::INLINE_FILE || name == boot_artifacts::INDEX_FILE {
            continue;
        }
        let category = match name.as_str() {
            "00-core.md" => Some(BootCategory::Foundation),
            "90-user.md" => Some(BootCategory::UserOverride),
            _ => None,
        };
        let rel_path = if node_rel == "." {
            format!("spec/boot/{name}")
        } else {
            format!("{node_rel}/spec/boot/{name}")
        };
        files.push(AuthoredBoot {
            path: rel_path,
            category,
            origin: node_rel.to_string(),
        });
    }
    // Deterministic order — the engine keeps a band's collection order.
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Build the dependency-boot inputs for one node: the transitive closure
/// of its `[requires]` within `resolution`, each turned into a
/// [`DependencyBoot`].
fn node_dependency_boot(
    node_manifest: &Manifest,
    resolution: &[ResolvedDep],
) -> Vec<DependencyBoot> {
    let index: HashMap<(PackageKind, &str), &ResolvedDep> = resolution
        .iter()
        .map(|d| ((d.kind, d.name.as_str()), d))
        .collect();

    // Breadth-first transitive closure from the node's direct requires.
    let mut visited: HashSet<(PackageKind, String)> = HashSet::new();
    let mut queue: VecDeque<(PackageKind, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .map(|(k, n)| (k, n.to_string()))
        .collect();
    let mut closure: Vec<&ResolvedDep> = Vec::new();
    while let Some((kind, name)) = queue.pop_front() {
        if !visited.insert((kind, name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(kind, name.as_str())) {
            closure.push(dep);
            for (rk, rn) in &dep.requires {
                queue.push_back((*rk, rn.clone()));
            }
        }
    }

    closure
        .iter()
        .map(|dep| {
            let slot = vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version);
            let snippet = dep.manifest.boot_snippet.as_ref();
            let boot_path = snippet.map(|bs| {
                format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/"))
            });
            DependencyBoot {
                kind: dep.kind,
                name: dep.name.clone(),
                boot_path,
                category: snippet.and_then(|bs| bs.category),
                // Only a direct requirement carries a consumer-declared
                // `link`; a transitive dependency reads back as `None`.
                declared_link: node_manifest.requires.declared_link(dep.kind, &dep.name),
                suggested_link: snippet.and_then(|bs| bs.link),
                requires: dep.requires.clone(),
            }
        })
        .collect()
}

/// Build a [`WorkspaceError::Io`] from a `std::io::Error` and its path.
fn io_err(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    fn ver(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    /// A `ResolvedDep` with a content tree written into a fresh temp dir.
    /// The `TempDir` is returned so the caller keeps it alive.
    fn dep_with_boot(
        name: &str,
        version: &str,
        snippet_toml: &str,
        boot_rel: &str,
        boot_body: &str,
    ) -> (ResolvedDep, TempDir) {
        let pkg = TempDir::new().unwrap();
        write(
            pkg.path(),
            "vibe.toml",
            &format!(
                "[package]\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n\n{snippet_toml}"
            ),
        );
        write(pkg.path(), boot_rel, boot_body);
        let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
        let dep = ResolvedDep {
            kind: PackageKind::Flow,
            name: name.to_string(),
            version: ver(version),
            content_dir: pkg.path().to_path_buf(),
            manifest,
            requires: vec![],
        };
        (dep, pkg)
    }

    #[test]
    fn apply_resolution_materialises_and_regenerates_a_standalone_project() {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"flow:wal\" = \"^0.3\"\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");

        let (dep, _pkg) = dep_with_boot(
            "wal",
            "0.3.0",
            "[boot_snippet]\nsource = \"boot/10-flow-wal.md\"\ncategory = \"flow\"\n",
            "boot/10-flow-wal.md",
            "# wal boot",
        );

        let ws = Workspace::load(ws_dir.path()).unwrap();
        let outcome = apply_resolution(&ws, std::slice::from_ref(&dep)).unwrap();

        assert_eq!(outcome.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
        assert_eq!(outcome.nodes_regenerated, vec!["."]);
        // The package tree is materialised verbatim into its slot.
        assert!(
            ws_dir
                .path()
                .join("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md")
                .is_file()
        );
        assert!(ws_dir.path().join("vibedeps/flow-wal/0.3.0/vibe.toml").is_file());
        // INDEX.md names the node's own foundation boot and the dependency.
        let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
        assert!(index.contains("spec/boot/00-core.md"), "{index}");
        assert!(
            index.contains("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md"),
            "{index}"
        );
        // The redirect lands at the node root.
        assert!(ws_dir.path().join("CLAUDE.md").is_file());
    }

    #[test]
    fn apply_resolution_with_no_dependencies_still_writes_index() {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");
        let ws = Workspace::load(ws_dir.path()).unwrap();
        let outcome = apply_resolution(&ws, &[]).unwrap();
        assert!(outcome.materialised.is_empty());
        assert_eq!(outcome.nodes_regenerated, vec!["."]);
        assert!(ws_dir.path().join("spec/boot/INDEX.md").is_file());
    }

    #[test]
    fn apply_resolution_inline_dependency_produces_inline_md() {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"flow:crit\" = { version = \"^1.0\", link = \"inline\" }\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");

        let (dep, _pkg) = dep_with_boot(
            "crit",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/crit.md\"\n",
            "boot/crit.md",
            "# critical discipline",
        );

        let ws = Workspace::load(ws_dir.path()).unwrap();
        apply_resolution(&ws, std::slice::from_ref(&dep)).unwrap();

        // The consumer declared `link = "inline"`, so the dependency's
        // boot is concatenated into INLINE.md.
        let inline = fs::read_to_string(ws_dir.path().join("spec/boot/INLINE.md")).unwrap();
        assert!(inline.contains("# critical discipline"), "{inline}");
    }

    #[test]
    fn apply_resolution_skips_a_dependency_outside_the_node_requires() {
        // The resolution carries `flow:extra`, but the project does not
        // require it — it is materialised, but contributes no boot entry.
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"flow:wal\" = \"^0.3\"\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");

        let (wal, _w) = dep_with_boot(
            "wal",
            "0.3.0",
            "[boot_snippet]\nsource = \"boot/wal.md\"\n",
            "boot/wal.md",
            "# wal",
        );
        let (extra, _e) = dep_with_boot(
            "extra",
            "0.1.0",
            "[boot_snippet]\nsource = \"boot/extra.md\"\n",
            "boot/extra.md",
            "# extra",
        );

        let ws = Workspace::load(ws_dir.path()).unwrap();
        apply_resolution(&ws, &[wal, extra]).unwrap();

        let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
        assert!(index.contains("vibedeps/flow-wal/0.3.0/boot/wal.md"), "{index}");
        // `flow:extra` is materialised but not in the boot index.
        assert!(
            ws_dir.path().join("vibedeps/flow-extra/0.1.0/boot/extra.md").is_file()
        );
        assert!(!index.contains("flow-extra"), "{index}");
    }
}
