//! Turning manifests into declared skills, and the one kind that needs a
//! pass of its own (PROP-057 `##KIND-DOC-NOT-INSTALLED`).
//!
//! Two things live here because they are one job seen twice: lowering a
//! package's `[[skill]]` tables into the inventory, and finding the
//! packages a lock file can never name. Out of line from the
//! orchestration it serves, by the file-length budget.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;
use vibe_workspace::Workspace;

use vibe_core::manifest::LockedEmbeddedSource;
use vibe_core::{ContentHash, PackageName};

use super::{DeclaredSkill, DeclaredSkillProvider};

/// The skills of documentation packages this project authors in-tree.
///
/// Every other kind reaches an agent through the lock file, and a `doc`
/// package cannot: `vibe install` refuses it by law — documentation is
/// read, not installed (PROP-057 `##KIND-DOC-NOT-INSTALLED`) — so a
/// manual's skill would be invisible to `vibe skill` forever with the
/// two passes above alone. The project-local registry is where a project
/// authors its own packages, and a documentation package it authors is
/// documentation it holds.
///
/// A coordinate the lock file already provided is skipped: an in-tree
/// package that is ALSO installed is one package, and listing its skill
/// twice would offer the same projection under two origins.
///
/// What this does not reach is a manual warmed into the MACHINE store by
/// `vibe cache add`. That is the store's per-machine nature meeting a
/// per-project command, and `##KIND-DOC-KNOWN-LIMIT` rules a project-level
/// «consult this documentation» declaration out of this wave; until there
/// is one, a consumer reads a warmed manual through `vibe doc serve` and
/// the `read_doc` tool, which need no declaration.
pub(super) fn documentation_skills(ws: &Workspace, out: &mut Vec<DeclaredSkill>) -> Result<()> {
    let root = ws
        .node_abs_path(".")
        .join(vibe_core::layout::current_packages_root());
    let seen: Vec<String> = out.iter().map(|skill| skill.decl.name.clone()).collect();
    for slot in package_slots(&root) {
        let Ok(manifest) = Manifest::read(slot.join(Manifest::FILENAME)) else {
            continue;
        };
        let is_documentation = manifest
            .package
            .as_ref()
            .is_some_and(|package| package.kind == PackageKind::Doc);
        if !is_documentation || manifest.skills.is_empty() {
            continue;
        }
        let origin = match &manifest.package {
            Some(package) => format!("{}:{}", package.kind.as_str(), package.name),
            None => continue,
        };
        let mut found = Vec::new();
        lower_manifest_skills(&manifest, &slot, &origin, None, None, &mut found)?;
        out.extend(
            found
                .into_iter()
                .filter(|skill| !seen.contains(&skill.decl.name)),
        );
    }
    Ok(())
}

/// Every `<group>/<name>/<version>` directory under a package registry
/// root, in a stable order. An unreadable level is skipped rather than
/// fatal: a registry with one odd directory in it still holds packages.
fn package_slots(registry_root: &Path) -> Vec<PathBuf> {
    let mut slots = Vec::new();
    let mut level = vec![registry_root.to_path_buf()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            let mut found: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect();
            found.sort();
            next.extend(found);
        }
        level = next;
    }
    slots.append(&mut level);
    slots
}

pub(super) fn lower_manifest_skills(
    manifest: &Manifest,
    base: &Path,
    origin: &str,
    content_hash: Option<ContentHash>,
    locked_embedded_sources: Option<&[LockedEmbeddedSource]>,
    out: &mut Vec<DeclaredSkill>,
) -> Result<()> {
    if manifest.skills.is_empty() {
        return Ok(());
    }
    let package = manifest.package.as_ref().with_context(|| {
        format!(
            "manifest `{}` declares [[skill]] without package role",
            base.join(Manifest::FILENAME).display()
        )
    })?;
    let name = PackageName::parse(&package.name)?;
    let provider = match content_hash {
        Some(content_hash) => DeclaredSkillProvider::Installed {
            group: package.group.clone(),
            name,
            version: package.version.to_string(),
            kind: package.kind,
            root: base.to_path_buf(),
            content_hash,
            embedded_sources: locked_embedded_sources.unwrap_or_default().to_vec(),
            declared_sources: manifest.embedded_sources.clone(),
        },
        None => DeclaredSkillProvider::Authored {
            group: package.group.clone(),
            name,
            version: package.version.to_string(),
            kind: package.kind,
            root: base.to_path_buf(),
            embedded_sources: manifest.embedded_sources.clone(),
        },
    };
    for decl in &manifest.skills {
        out.push(DeclaredSkill {
            source: base.join(&decl.path),
            source_root: base.to_path_buf(),
            decl: decl.clone(),
            origin: origin.to_string(),
            provider: provider.clone(),
        });
    }
    Ok(())
}
