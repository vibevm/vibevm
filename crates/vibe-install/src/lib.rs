//! The `install` / `uninstall` / `update` workflows.
//!
//! Plan → user-confirm → apply → update-lockfile → report. Mutating nodes run
//! only after an `Approval` is produced.
//!
//! ## Package layout
//!
//! Packages use the **mirror layout** pinned in `VIBEVM-SPEC.md` §13.1: every
//! entry in `writes.files` is both the source path inside the package and the
//! target path inside the project. Boot snippets are the one exception — they
//! carry an explicit `source` field, and their target is always
//! `spec/boot/<filename>`. `plan_install` relies on this and computes each
//! write's `source_abs` as `cache_dir.join(file)`.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6 (install subgraph), §6 (boot dir), §11.1 (M0
//! scope), §13 (package model); [`spec://vibevm/common/PROP-000#package-layout`]
//! for the decision record.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vibe_core::manifest::{LockedPackage, Lockfile};
use vibe_core::{PackageKind, PackageRef};
use vibe_registry::CachedPackage;

/// Files the user owns exclusively. Packages may never declare writes to these
/// paths; `vibe init` creates them, and only the user edits them.
pub const USER_OWNED_PATHS: &[&str] = &["spec/boot/00-core.md", "spec/boot/90-user.md"];

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(
        "boot snippet filename `{filename}` is already claimed{}",
        match existing_owner {
            Some(o) => format!(" by `{o}`"),
            None => String::new(),
        }
    )]
    BootSnippetConflict {
        filename: String,
        existing_owner: Option<String>,
    },

    #[error(
        "boot snippet numeric prefix `{prefix}` is already taken by `{existing}`; pick a different NN-* number"
    )]
    BootSnippetNumericConflict {
        prefix: String,
        existing: String,
    },

    #[error("package `{package}` is already installed at version {version} — use `vibe update` instead")]
    AlreadyInstalled { package: String, version: String },

    #[error("package `{package}` is not installed")]
    NotInstalled { package: String },

    #[error("target file `{path}` already exists and is not owned by this package")]
    TargetFileExists { path: PathBuf },

    #[error("package declares a write to user-owned path `{path}` — packages must not touch these")]
    WritesToUserOwnedPath { path: PathBuf },

    #[error(
        "package declares a write to an absolute or escaping path `{path}` — writes must stay within the project"
    )]
    EscapingWritePath { path: PathBuf },

    #[error("package declares a source file `{path}` that does not exist in the package")]
    MissingSourceFile { path: PathBuf },

    #[error("user declined the install plan")]
    UserDeclined,

    #[error(transparent)]
    Registry(#[from] vibe_registry::RegistryError),

    #[error(transparent)]
    Core(#[from] vibe_core::Error),

    #[error("I/O error on `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl InstallError {
    pub fn exit_code(&self) -> u8 {
        match self {
            InstallError::BootSnippetConflict { .. }
            | InstallError::BootSnippetNumericConflict { .. } => 3,
            InstallError::UserDeclined => 5,
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    Regular,
    BootSnippet,
}

#[derive(Debug, Clone)]
pub struct PlannedWrite {
    pub kind: WriteKind,
    /// Path inside the cache — absolute.
    pub source_abs: PathBuf,
    /// Path inside the project — relative, always forward-slashed.
    pub target_rel: PathBuf,
    /// Absolute target path on disk.
    pub target_abs: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub cached: CachedPackage,
    pub writes: Vec<PlannedWrite>,
    pub boot_snippet_filename: Option<String>,
}

impl InstallPlan {
    pub fn package_label(&self) -> String {
        format!(
            "{}:{}@{}",
            self.cached.resolved.kind,
            self.cached.resolved.name,
            self.cached.resolved.version
        )
    }
}

/// Build an [`InstallPlan`] without touching disk beyond reads.
pub fn plan_install(
    project_root: &Path,
    lockfile: &Lockfile,
    cached: CachedPackage,
) -> Result<InstallPlan, InstallError> {
    // 1. Refuse if already installed.
    if let Some(existing) = lockfile.find(cached.resolved.kind, &cached.resolved.name) {
        return Err(InstallError::AlreadyInstalled {
            package: format!("{}:{}", existing.kind, existing.name),
            version: existing.version.to_string(),
        });
    }

    let manifest = &cached.manifest;
    let mut writes: Vec<PlannedWrite> = Vec::new();
    let mut seen_targets: HashSet<PathBuf> = HashSet::new();

    // 2. Regular writes: each entry is BOTH a source (relative to package)
    // and a target (relative to project root). See the module-level REVIEW.
    for file in &manifest.writes.files {
        validate_target_rel(file)?;
        let source_abs = cached.cache_dir.join(file);
        if !source_abs.is_file() {
            return Err(InstallError::MissingSourceFile { path: file.clone() });
        }
        let target_abs = project_root.join(file);
        reject_existing_target(&target_abs)?;
        let target_rel = normalize_rel(file);
        if !seen_targets.insert(target_rel.clone()) {
            // Duplicate write entries are an authoring bug.
            continue;
        }
        writes.push(PlannedWrite {
            kind: WriteKind::Regular,
            source_abs,
            target_rel,
            target_abs,
        });
    }

    // 3. Boot snippet, if any.
    let mut boot_snippet_filename = None;
    if let Some(snippet) = &manifest.boot_snippet {
        validate_boot_filename(&snippet.filename)?;
        let source_abs = cached.cache_dir.join(&snippet.source);
        if !source_abs.is_file() {
            return Err(InstallError::MissingSourceFile {
                path: snippet.source.clone(),
            });
        }
        let target_rel = normalize_rel(Path::new(&format!("spec/boot/{}", snippet.filename)));
        let target_abs = project_root.join(&target_rel);
        reject_boot_snippet_conflict(
            project_root,
            lockfile,
            &snippet.filename,
            &cached.resolved.kind,
            &cached.resolved.name,
        )?;
        writes.push(PlannedWrite {
            kind: WriteKind::BootSnippet,
            source_abs,
            target_rel,
            target_abs,
        });
        boot_snippet_filename = Some(snippet.filename.clone());
    }

    Ok(InstallPlan {
        cached,
        writes,
        boot_snippet_filename,
    })
}

/// Perform the writes declared by `plan`. Returns the relative paths (as
/// forward-slashed strings in `PathBuf`) that were actually written, suitable
/// for recording in the lockfile.
///
/// On failure, best-effort rollback: delete anything this call had created.
pub fn apply_install(plan: &InstallPlan) -> Result<Vec<PathBuf>, InstallError> {
    let mut written: Vec<PathBuf> = Vec::new();
    for w in &plan.writes {
        if let Some(parent) = w.target_abs.parent() {
            fs::create_dir_all(parent).map_err(|source| InstallError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        match fs::copy(&w.source_abs, &w.target_abs) {
            Ok(_) => written.push(w.target_rel.clone()),
            Err(e) => {
                // Roll back what we have written so far.
                for prev in &written {
                    let abs = plan
                        .writes
                        .iter()
                        .find(|x| x.target_rel == *prev)
                        .map(|x| x.target_abs.clone());
                    if let Some(p) = abs {
                        let _ = fs::remove_file(&p);
                    }
                }
                return Err(InstallError::Io {
                    path: w.target_abs.clone(),
                    source: e,
                });
            }
        }
    }
    Ok(written)
}

/// Update `lockfile` to reflect a successful install.
pub fn register_installed(
    lockfile: &mut Lockfile,
    plan: &InstallPlan,
    files_written: Vec<PathBuf>,
    generated_at: String,
) {
    // Phase A: the per-package-registry / resolver fields
    // (`registry`, `source_ref`, `resolved_commit`, `dependencies`,
    // `overridden`) will be populated by the `MultiRegistryResolver` +
    // `GitPackageRegistry` commits queued in TASKS.md. For now
    // `source_url` carries the legacy combined URI string and the rest
    // stays at its schema-v2 default. The lockfile on disk is already
    // in v2 shape — the fields are just unset until upstream fills them.
    let entry = LockedPackage {
        kind: plan.cached.resolved.kind,
        name: plan.cached.resolved.name.clone(),
        version: plan.cached.resolved.version.clone(),
        registry: None,
        source_url: plan.cached.source_uri.clone(),
        source_ref: None,
        resolved_commit: None,
        content_hash: plan.cached.content_hash.clone(),
        boot_snippet: plan.boot_snippet_filename.clone(),
        files_written,
        dependencies: Vec::new(),
        overridden: false,
    };
    lockfile.packages.push(entry);
    lockfile.meta.generated_at = generated_at;
}

// ==== uninstall ============================================================

#[derive(Debug, Clone)]
pub struct UninstallPlan {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    pub removed_paths: Vec<PathBuf>,
    pub project_root: PathBuf,
}

pub fn plan_uninstall(
    project_root: &Path,
    lockfile: &Lockfile,
    pkgref: &PackageRef,
) -> Result<UninstallPlan, InstallError> {
    let entry = lockfile
        .find(pkgref.kind, &pkgref.name)
        .ok_or_else(|| InstallError::NotInstalled {
            package: format!("{}:{}", pkgref.kind, pkgref.name),
        })?;

    let removed_paths: Vec<PathBuf> = entry
        .files_written
        .iter()
        .filter(|p| !is_user_owned(p))
        .cloned()
        .collect();

    Ok(UninstallPlan {
        kind: entry.kind,
        name: entry.name.clone(),
        version: entry.version.clone(),
        removed_paths,
        project_root: project_root.to_path_buf(),
    })
}

pub fn apply_uninstall(plan: &UninstallPlan) -> Result<Vec<PathBuf>, InstallError> {
    let mut removed: Vec<PathBuf> = Vec::new();
    for rel in &plan.removed_paths {
        let abs = plan.project_root.join(rel);
        if abs.exists() {
            fs::remove_file(&abs).map_err(|source| InstallError::Io {
                path: abs.clone(),
                source,
            })?;
            removed.push(rel.clone());
        }
    }
    // Prune empty parent directories, but never touch spec/boot itself (it's a
    // permanent part of the layout) or the project root.
    for rel in &plan.removed_paths {
        let mut p = plan.project_root.join(rel);
        while let Some(parent) = p.parent() {
            if parent == plan.project_root {
                break;
            }
            if is_structural_dir(parent, &plan.project_root) {
                break;
            }
            match fs::read_dir(parent) {
                Ok(mut it) => {
                    if it.next().is_some() {
                        break;
                    }
                }
                Err(_) => break,
            }
            if fs::remove_dir(parent).is_err() {
                break;
            }
            p = parent.to_path_buf();
        }
    }
    Ok(removed)
}

pub fn unregister_installed(
    lockfile: &mut Lockfile,
    pkgref: &PackageRef,
    generated_at: String,
) -> Result<LockedPackage, InstallError> {
    let removed = lockfile
        .remove(pkgref.kind, &pkgref.name)
        .ok_or_else(|| InstallError::NotInstalled {
            package: format!("{}:{}", pkgref.kind, pkgref.name),
        })?;
    lockfile.meta.generated_at = generated_at;
    Ok(removed)
}

// ==== helpers ==============================================================

fn validate_target_rel(path: &Path) -> Result<(), InstallError> {
    if path.is_absolute() {
        return Err(InstallError::EscapingWritePath {
            path: path.to_path_buf(),
        });
    }
    for comp in path.components() {
        use std::path::Component;
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(InstallError::EscapingWritePath {
                    path: path.to_path_buf(),
                });
            }
            _ => {}
        }
    }
    if is_user_owned(path) {
        return Err(InstallError::WritesToUserOwnedPath {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn reject_existing_target(target_abs: &Path) -> Result<(), InstallError> {
    if target_abs.exists() {
        return Err(InstallError::TargetFileExists {
            path: target_abs.to_path_buf(),
        });
    }
    Ok(())
}

fn reject_boot_snippet_conflict(
    project_root: &Path,
    lockfile: &Lockfile,
    filename: &str,
    kind: &PackageKind,
    name: &str,
) -> Result<(), InstallError> {
    let boot_dir = project_root.join("spec/boot");
    let target = boot_dir.join(filename);
    if target.exists() {
        // Is this an installed package's snippet, or a stray file?
        let existing_owner = lockfile
            .packages
            .iter()
            .find(|p| p.boot_snippet.as_deref() == Some(filename))
            .map(|p| format!("{}:{}", p.kind, p.name));
        return Err(InstallError::BootSnippetConflict {
            filename: filename.to_string(),
            existing_owner,
        });
    }
    if let Some(prefix) = numeric_prefix(filename) {
        if !boot_dir.is_dir() {
            return Ok(());
        }
        let entries = fs::read_dir(&boot_dir).map_err(|source| InstallError::Io {
            path: boot_dir.clone(),
            source,
        })?;
        for entry in entries.flatten() {
            let name_os = entry.file_name();
            let existing = name_os.to_string_lossy();
            if existing.as_ref() == filename {
                // Exact-match handled above.
                continue;
            }
            if numeric_prefix(&existing) == Some(prefix) {
                // Skip user-owned boot files.
                if existing == "00-core.md" || existing == "90-user.md" {
                    continue;
                }
                let _ = (kind, name); // unused but kept for future attribution
                return Err(InstallError::BootSnippetNumericConflict {
                    prefix: prefix.to_string(),
                    existing: existing.into_owned(),
                });
            }
        }
    }
    Ok(())
}

fn validate_boot_filename(filename: &str) -> Result<(), InstallError> {
    // Expect `NN-<slug>.md` per §6.2.
    if filename.len() < 4 {
        return Err(InstallError::BootSnippetConflict {
            filename: filename.to_string(),
            existing_owner: None,
        });
    }
    let (prefix, rest) = filename.split_at(2);
    if !prefix.chars().all(|c| c.is_ascii_digit()) || !rest.starts_with('-') {
        return Err(InstallError::BootSnippetConflict {
            filename: filename.to_string(),
            existing_owner: None,
        });
    }
    if !filename.ends_with(".md") {
        return Err(InstallError::BootSnippetConflict {
            filename: filename.to_string(),
            existing_owner: None,
        });
    }
    let n: u8 = prefix.parse().unwrap_or(100);
    // 10-89 is the package-writable range. 00-09 is reserved for user
    // foundations, 90-99 for user overrides.
    if !(10..90).contains(&n) {
        return Err(InstallError::BootSnippetConflict {
            filename: filename.to_string(),
            existing_owner: Some("reserved range (00-09 foundation / 90-99 user)".into()),
        });
    }
    Ok(())
}

fn numeric_prefix(filename: &str) -> Option<&str> {
    if filename.len() < 3 {
        return None;
    }
    let (prefix, rest) = filename.split_at(2);
    if prefix.chars().all(|c| c.is_ascii_digit()) && rest.starts_with('-') {
        Some(prefix)
    } else {
        None
    }
}

fn normalize_rel(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().replace('\\', "/");
    PathBuf::from(s)
}

fn is_user_owned(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    USER_OWNED_PATHS.iter().any(|u| *u == s)
}

fn is_structural_dir(path: &Path, root: &Path) -> bool {
    let rel = match path.strip_prefix(root) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        Err(_) => return true,
    };
    matches!(
        rel.as_str(),
        "spec" | "spec/boot" | "spec/flows" | "spec/feats" | "spec/stacks"
            | "spec/common" | "spec/modules" | ".vibe" | ".vibe/cache"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use vibe_registry::{LocalRegistry, ResolvedPackage};

    fn seed_registry(root: &Path, manifest_toml: &str, content: &[(&str, &str)]) {
        let pkg_dir = root.join("flow/wal/v0.3.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("vibe-package.toml"), manifest_toml).unwrap();
        fs::write(pkg_dir.join("README.md"), "# wal\n").unwrap();
        for (rel, body) in content {
            let path = pkg_dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
        }
    }

    fn cached_for_test(registry_root: &Path, cache_root: &Path) -> CachedPackage {
        let reg = LocalRegistry::new(registry_root).unwrap();
        let pkgref = PackageRef::parse("flow:wal@0.3.0").unwrap();
        let resolved: ResolvedPackage = reg.resolve(&pkgref).unwrap();
        reg.fetch(&resolved, cache_root).unwrap()
    }

    const FIXTURE_MANIFEST: &str = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"

[writes]
files = [
    "spec/flows/wal/WAL-PROTOCOL.md",
    "spec/flows/wal/session-end-hook.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"
"#;

    fn fixture_content() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "spec/flows/wal/WAL-PROTOCOL.md",
                "# WAL Protocol\n\nContent...\n",
            ),
            (
                "spec/flows/wal/session-end-hook.md",
                "# Session end hook\n",
            ),
            ("boot/10-flow-wal.md", "# Flow: WAL boot snippet\n"),
        ]
    }

    fn empty_project() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("spec/boot")).unwrap();
        dir
    }

    #[test]
    fn plan_install_works_on_fresh_project() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let lockfile = Lockfile::empty("vibe-test", "now");
        let plan = plan_install(project.path(), &lockfile, cached).unwrap();

        assert_eq!(plan.writes.len(), 3);
        assert_eq!(
            plan.writes
                .iter()
                .filter(|w| w.kind == WriteKind::Regular)
                .count(),
            2
        );
        assert_eq!(
            plan.writes
                .iter()
                .filter(|w| w.kind == WriteKind::BootSnippet)
                .count(),
            1
        );
        assert_eq!(plan.boot_snippet_filename.as_deref(), Some("10-flow-wal.md"));
    }

    #[test]
    fn apply_install_writes_files_and_register_updates_lockfile() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        let plan = plan_install(project.path(), &lockfile, cached).unwrap();

        let written = apply_install(&plan).unwrap();
        for rel in &written {
            assert!(project.path().join(rel).is_file(), "expected {rel:?} exists");
        }

        register_installed(&mut lockfile, &plan, written, "t1".into());
        assert_eq!(lockfile.packages.len(), 1);
        let entry = &lockfile.packages[0];
        assert_eq!(entry.kind, PackageKind::Flow);
        assert_eq!(entry.name, "wal");
        assert_eq!(entry.version.to_string(), "0.3.0");
        assert_eq!(entry.boot_snippet.as_deref(), Some("10-flow-wal.md"));
        assert!(!entry.content_hash.is_empty());
    }

    #[test]
    fn already_installed_errors() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        // Pre-install.
        let plan = plan_install(project.path(), &lockfile, cached.clone()).unwrap();
        let written = apply_install(&plan).unwrap();
        register_installed(&mut lockfile, &plan, written, "t1".into());

        // Plan again: should error.
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(err, InstallError::AlreadyInstalled { .. }));
    }

    #[test]
    fn boot_snippet_conflict_is_reported() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        // Plant an existing file at the same boot filename.
        fs::write(project.path().join("spec/boot/10-flow-wal.md"), "existing").unwrap();

        let lockfile = Lockfile::empty("vibe-test", "t0");
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(err, InstallError::BootSnippetConflict { .. }));
    }

    #[test]
    fn numeric_conflict_with_different_snippet_name_is_reported() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        fs::write(
            project.path().join("spec/boot/10-flow-other.md"),
            "different",
        )
        .unwrap();

        let lockfile = Lockfile::empty("vibe-test", "t0");
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(
            err,
            InstallError::BootSnippetNumericConflict { .. }
        ));
    }

    #[test]
    fn user_owned_00_core_is_ignored() {
        // 00-core.md exists in most projects; planning should still succeed.
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        fs::write(project.path().join("spec/boot/00-core.md"), "user content").unwrap();

        let lockfile = Lockfile::empty("vibe-test", "t0");
        let plan = plan_install(project.path(), &lockfile, cached).unwrap();
        assert_eq!(plan.writes.len(), 3);
    }

    #[test]
    fn package_declaring_write_to_user_owned_path_errors() {
        let manifest = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"

[writes]
files = [
    "spec/boot/00-core.md",
]
"#;
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), manifest, &[("spec/boot/00-core.md", "hi")]);
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let lockfile = Lockfile::empty("vibe-test", "t0");
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(err, InstallError::WritesToUserOwnedPath { .. }));
    }

    #[test]
    fn escaping_write_path_is_rejected() {
        let manifest = r#"
[package]
name = "wal"
kind = "flow"
version = "0.3.0"

[writes]
files = ["../escape.md"]
"#;
        let reg_dir = tempdir().unwrap();
        let pkg_dir = reg_dir.path().join("flow/wal/v0.3.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("vibe-package.toml"), manifest).unwrap();
        fs::write(pkg_dir.join("README.md"), "hi").unwrap();
        // We don't need the source file; plan_install rejects before file lookup.

        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let lockfile = Lockfile::empty("vibe-test", "t0");
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(err, InstallError::EscapingWritePath { .. }));
    }

    #[test]
    fn uninstall_removes_files_and_entry() {
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        let project = empty_project();
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        let plan = plan_install(project.path(), &lockfile, cached).unwrap();
        let written = apply_install(&plan).unwrap();
        register_installed(&mut lockfile, &plan, written, "t1".into());

        let pkgref = PackageRef::parse("flow:wal").unwrap();
        let uplan = plan_uninstall(project.path(), &lockfile, &pkgref).unwrap();
        assert_eq!(uplan.removed_paths.len(), 3);

        let removed = apply_uninstall(&uplan).unwrap();
        assert_eq!(removed.len(), 3);
        for rel in &removed {
            assert!(!project.path().join(rel).exists(), "{rel:?} still exists");
        }

        let gone = unregister_installed(&mut lockfile, &pkgref, "t2".into()).unwrap();
        assert_eq!(gone.name, "wal");
        assert!(lockfile.packages.is_empty());
    }

    #[test]
    fn uninstall_never_deletes_user_owned_file() {
        let reg_dir = tempdir().unwrap();
        // Construct a fake lockfile that claims files_written includes 00-core.
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        lockfile.packages.push(LockedPackage {
            kind: PackageKind::Flow,
            name: "wal".into(),
            version: semver::Version::parse("0.3.0").unwrap(),
            registry: None,
            source_url: "file:///fake".into(),
            source_ref: None,
            resolved_commit: None,
            content_hash: "sha256:whatever".into(),
            boot_snippet: Some("10-flow-wal.md".into()),
            files_written: vec![
                PathBuf::from("spec/boot/00-core.md"),
                PathBuf::from("spec/boot/10-flow-wal.md"),
            ],
            dependencies: Vec::new(),
            overridden: false,
        });

        let project = empty_project();
        fs::write(project.path().join("spec/boot/00-core.md"), "user data").unwrap();
        fs::write(project.path().join("spec/boot/10-flow-wal.md"), "snippet").unwrap();

        let pkgref = PackageRef::parse("flow:wal").unwrap();
        let uplan = plan_uninstall(project.path(), &lockfile, &pkgref).unwrap();
        // 00-core is filtered out of removed_paths.
        assert_eq!(uplan.removed_paths.len(), 1);

        let _ = reg_dir;
        apply_uninstall(&uplan).unwrap();

        assert!(project.path().join("spec/boot/00-core.md").exists());
        assert!(!project.path().join("spec/boot/10-flow-wal.md").exists());
    }
}
