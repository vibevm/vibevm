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
use vibe_core::user_config::SlotIntegrity;

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
    /// `vibedeps/` slot paths freshly materialised this run — a new or
    /// version-bumped dependency whose content was copied.
    pub materialised: Vec<String>,
    /// `vibedeps/` slot paths skipped — already present for the resolved
    /// version, trusted and not re-copied (PROP-011 §2.3). Empty when
    /// `slot_integrity` is `Verify`.
    pub skipped: Vec<String>,
    /// `vibedeps/` slot paths pruned — present before, absent from this
    /// resolution (a version bump, or a dropped dependency).
    pub pruned: Vec<String>,
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
///
/// `slot_integrity` governs the PROP-011 §2.3 materialise-diff skip: with
/// [`SlotIntegrity::TrustPresence`] a slot already on disk for the
/// resolved version is trusted and not re-copied; with
/// [`SlotIntegrity::Verify`] every slot is re-materialised. `vibe install`
/// passes the user-config value; `vibe reinstall --force` passes `Verify`,
/// since its whole purpose is to overwrite slots from a fresh fetch.
pub fn apply_resolution(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
) -> Result<InstallOutcome, WorkspaceError> {
    // 0. Validate every node's `<vibevm>` instruction-file block before
    //    any mutation — a malformed block aborts here, not mid-install
    //    (PROP-012 §2.4).
    validate_redirect_blocks(workspace)?;

    // 1. Materialise the resolution into `vibedeps/`. PROP-011 §2.3 — a
    //    slot already present for the resolved (immutable) version is
    //    trusted and skipped; only a new or version-bumped dependency
    //    pays the recursive copy. `SlotIntegrity::Verify` opts out, so a
    //    hand-edited slot is overwritten.
    let mut materialised = Vec::new();
    let mut skipped = Vec::new();
    for dep in resolution {
        let slot = vibedeps::slot_rel_path(dep.kind, &dep.name, &dep.version);
        let present = vibedeps::is_materialised(&workspace.root, dep.kind, &dep.name, &dep.version);
        if present && slot_integrity == SlotIntegrity::TrustPresence {
            skipped.push(slot);
        } else {
            vibedeps::materialise(
                &workspace.root,
                dep.kind,
                &dep.name,
                &dep.version,
                &dep.content_dir,
            )?;
            materialised.push(slot);
        }
    }

    // 2. Prune any `vibedeps/` slot no longer in the resolution — a
    //    version bump or a dropped dependency must leave no orphan. Both
    //    the freshly-materialised and the skipped slots belong to the
    //    current resolution and are kept.
    let kept: Vec<String> = materialised.iter().chain(&skipped).cloned().collect();
    let pruned = prune_stale_slots(&workspace.root, &kept)?;

    // 3. Regenerate every node's boot artifacts from the resolution.
    let nodes_regenerated = regenerate_boot_from(workspace, resolution)?;

    Ok(InstallOutcome {
        materialised,
        skipped,
        pruned,
        nodes_regenerated,
    })
}

/// Remove every `vibedeps/` slot whose path is not in `kept`, returning
/// the removed slot paths (sorted). A `<kind>-<name>` directory left with
/// no surviving version is removed too, so `vibedeps/` holds exactly the
/// current resolution and no empty husks.
fn prune_stale_slots(
    workspace_root: &Path,
    kept: &[String],
) -> Result<Vec<String>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibedeps::VIBEDEPS_DIR);
    if !vibedeps_dir.is_dir() {
        return Ok(Vec::new());
    }
    let keep: HashSet<&str> = kept.iter().map(String::as_str).collect();
    let mut pruned = Vec::new();
    for kind_name in fs::read_dir(&vibedeps_dir).map_err(|e| io_err(&vibedeps_dir, e))? {
        let kind_name = kind_name.map_err(|e| io_err(&vibedeps_dir, e))?;
        let kind_name_dir = kind_name.path();
        if !kind_name_dir.is_dir() {
            continue;
        }
        let kn = kind_name.file_name().to_string_lossy().into_owned();
        let mut any_kept = false;
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let version_dir = version.path();
            if !version_dir.is_dir() {
                continue;
            }
            let ver = version.file_name().to_string_lossy().into_owned();
            let rel = format!("{}/{kn}/{ver}", vibedeps::VIBEDEPS_DIR);
            if keep.contains(rel.as_str()) {
                any_kept = true;
            } else {
                fs::remove_dir_all(&version_dir).map_err(|e| io_err(&version_dir, e))?;
                pruned.push(rel);
            }
        }
        if !any_kept {
            let _ = fs::remove_dir(&kind_name_dir);
        }
    }
    pruned.sort();
    Ok(pruned)
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
    // PROP-012 §2.4 — reject a malformed instruction-file block before
    // any boot-artifact write.
    validate_redirect_blocks(workspace)?;
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
///
/// `pub(crate)` so [`crate::publish`] can reuse it to regenerate a
/// staged copy's boot artifacts for the published shape (PROP-009 §2.11).
pub(crate) fn node_own_boot(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
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
            let boot_path = snippet
                .map(|bs| format!("{slot}/{}", bs.source.to_string_lossy().replace('\\', "/")));
            DependencyBoot {
                kind: dep.kind,
                name: dep.name.clone(),
                boot_path,
                category: snippet.and_then(|bs| bs.category),
                // Only a direct requirement carries a consumer-declared
                // `link`; a transitive dependency reads back as `None`.
                declared_link: node_manifest.requires.declared_link(dep.kind, &dep.name),
                suggested_link: snippet.and_then(|bs| bs.link),
                // The package's `[boot_snippet].when` OS gate, if any — it
                // forces the entry `dynamic` (PROP-009 §2.4).
                when: snippet.and_then(|bs| bs.when),
                requires: dep.requires.clone(),
            }
        })
        .collect()
}

/// Validate every node's agent instruction files before any mutation
/// (PROP-012 §2.4): a malformed `<vibevm>` block aborts the operation
/// here — ahead of materialisation or any boot-artifact write — so an
/// install never half-applies. A missing instruction file is fine; it is
/// created on write.
fn validate_redirect_blocks(workspace: &Workspace) -> Result<(), WorkspaceError> {
    for (rel, _) in workspace.iter_nodes() {
        let node_dir = workspace.node_abs_path(rel);
        for name in boot_artifacts::REDIRECT_FILES {
            let path = node_dir.join(name);
            let content = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_err(&path, e)),
            };
            if let boot_artifacts::BlockLocation::Malformed(reason) =
                boot_artifacts::locate_block(&content)
            {
                return Err(WorkspaceError::MalformedRedirectBlock { path, reason });
            }
        }
    }
    Ok(())
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
        let outcome = apply_resolution(
            &ws,
            std::slice::from_ref(&dep),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();

        assert_eq!(outcome.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
        assert_eq!(outcome.nodes_regenerated, vec!["."]);
        // The package tree is materialised verbatim into its slot.
        assert!(
            ws_dir
                .path()
                .join("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md")
                .is_file()
        );
        assert!(
            ws_dir
                .path()
                .join("vibedeps/flow-wal/0.3.0/vibe.toml")
                .is_file()
        );
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
        let outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence).unwrap();
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
        apply_resolution(
            &ws,
            std::slice::from_ref(&dep),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();

        // The consumer declared `link = "inline"`, so the dependency's
        // boot is concatenated into INLINE.md.
        let inline = fs::read_to_string(ws_dir.path().join("spec/boot/INLINE.md")).unwrap();
        assert!(inline.contains("# critical discipline"), "{inline}");
    }

    #[test]
    fn apply_resolution_renders_when_from_a_boot_snippet() {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"flow:win\" = \"^1.0\"\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");

        let (dep, _pkg) = dep_with_boot(
            "win",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/win.md\"\nwhen = \"os:windows\"\n",
            "boot/win.md",
            "# windows-only guidance",
        );

        let ws = Workspace::load(ws_dir.path()).unwrap();
        apply_resolution(
            &ws,
            std::slice::from_ref(&dep),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();

        // The `[boot_snippet].when` rides into INDEX.md, and the entry is
        // dynamic — a condition forces the dynamic INCLUDE form even with
        // no `link` declared anywhere.
        let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
        assert!(
            index.contains("vibedeps/flow-win/1.0.0/boot/win.md"),
            "{index}"
        );
        assert!(index.contains("kind = \"dynamic\""), "{index}");
        assert!(index.contains("when = \"os:windows\""), "{index}");
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
        apply_resolution(&ws, &[wal, extra], SlotIntegrity::TrustPresence).unwrap();

        let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
        assert!(
            index.contains("vibedeps/flow-wal/0.3.0/boot/wal.md"),
            "{index}"
        );
        // `flow:extra` is materialised but not in the boot index.
        assert!(
            ws_dir
                .path()
                .join("vibedeps/flow-extra/0.1.0/boot/extra.md")
                .is_file()
        );
        assert!(!index.contains("flow-extra"), "{index}");
    }

    #[test]
    fn apply_resolution_prunes_a_stale_slot_on_version_bump() {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"flow:wal\" = \"^0\"\n",
        );
        write(ws_dir.path(), "spec/boot/00-core.md", "# core");
        let ws = Workspace::load(ws_dir.path()).unwrap();

        let (wal_v1, _v1) = dep_with_boot(
            "wal",
            "0.1.0",
            "[boot_snippet]\nsource = \"boot/wal.md\"\n",
            "boot/wal.md",
            "# v1",
        );
        apply_resolution(
            &ws,
            std::slice::from_ref(&wal_v1),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();
        assert!(ws_dir.path().join("vibedeps/flow-wal/0.1.0").is_dir());

        // Re-apply with wal bumped to 0.2.0 — the 0.1.0 slot is now stale.
        let (wal_v2, _v2) = dep_with_boot(
            "wal",
            "0.2.0",
            "[boot_snippet]\nsource = \"boot/wal.md\"\n",
            "boot/wal.md",
            "# v2",
        );
        let outcome = apply_resolution(
            &ws,
            std::slice::from_ref(&wal_v2),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();
        assert!(ws_dir.path().join("vibedeps/flow-wal/0.2.0").is_dir());
        assert!(
            !ws_dir.path().join("vibedeps/flow-wal/0.1.0").exists(),
            "the stale 0.1.0 slot must be pruned"
        );
        assert_eq!(outcome.pruned, vec!["vibedeps/flow-wal/0.1.0"]);
    }

    // --- PROP-011 §2.3 — materialise only the diff -----------------------

    #[test]
    fn apply_resolution_skips_a_present_slot_under_trust_presence() {
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
            "[boot_snippet]\nsource = \"boot/wal.md\"\n",
            "boot/wal.md",
            "# wal",
        );
        let ws = Workspace::load(ws_dir.path()).unwrap();

        // First apply — the slot is absent, so it is materialised.
        let first = apply_resolution(
            &ws,
            std::slice::from_ref(&dep),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();
        assert_eq!(first.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
        assert!(first.skipped.is_empty());

        // A sentinel inside the slot — a file the source never had. If
        // the second apply re-copies the slot, `materialise` clears it
        // first and the sentinel vanishes; if it skips, the sentinel
        // survives. It is the proof the skip actually skipped.
        let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
        fs::write(&sentinel, "untouched").unwrap();

        let second = apply_resolution(
            &ws,
            std::slice::from_ref(&dep),
            SlotIntegrity::TrustPresence,
        )
        .unwrap();
        assert!(
            second.materialised.is_empty(),
            "a present slot must not be re-copied"
        );
        assert_eq!(second.skipped, vec!["vibedeps/flow-wal/0.3.0"]);
        assert!(
            sentinel.is_file(),
            "TrustPresence must leave the slot untouched"
        );
    }

    #[test]
    fn apply_resolution_rematerialises_a_present_slot_under_verify() {
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
            "[boot_snippet]\nsource = \"boot/wal.md\"\n",
            "boot/wal.md",
            "# wal",
        );
        let ws = Workspace::load(ws_dir.path()).unwrap();

        apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify).unwrap();
        let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
        fs::write(&sentinel, "doomed").unwrap();

        // Second apply under Verify — the present slot is re-materialised,
        // so the sentinel is cleared along with it.
        let second =
            apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify).unwrap();
        assert_eq!(second.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
        assert!(second.skipped.is_empty(), "Verify must re-copy, never skip");
        assert!(!sentinel.exists(), "Verify must re-materialise the slot");
    }
}
