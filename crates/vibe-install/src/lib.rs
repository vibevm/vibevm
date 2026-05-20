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
use vibe_core::manifest::i18n::resolve_localised;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
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

    #[error(
        "content drift on `{package}@{version}`: lockfile pins `{expected}` but the source served `{actual}` from `{source_url}`. \
         Refusing to install — content_hash is the identity per PROP-002 §2.1. \
         Likely cause: an upstream tag was force-pushed, or a mirror is serving different bytes than canonical. \
         Investigate before proceeding; `--trust-mirror` is reserved for the deliberate-divergence case (M1.6)."
    )]
    ContentDrift {
        package: String,
        version: String,
        expected: String,
        actual: String,
        source_url: String,
    },

    #[error("package `{package}` is not installed")]
    NotInstalled { package: String },

    #[error(
        "package `{package}@{from_version}` is already at the resolved version (content_hash matches lockfile pin) — nothing to update"
    )]
    AlreadyUpToDate {
        package: String,
        from_version: String,
    },

    #[error(
        "package `{package}` has user-edited file `{path}` (bytes differ from the install-time cache); refusing to overwrite. \
         Back up your edits, then run `vibe uninstall {package}` followed by `vibe install {package}` to apply the new version cleanly."
    )]
    UserEditedFile {
        package: String,
        path: PathBuf,
    },

    #[error(
        "package `{package}@{from_version}` was installed without an old cache directory at `{old_cache_dir}` — \
         can't verify whether the project files are pristine. Run `vibe registry sync` first to repopulate the cache, or `vibe uninstall {package} && vibe install {package}` to start fresh."
    )]
    OldCacheMissing {
        package: String,
        from_version: String,
        old_cache_dir: PathBuf,
    },

    #[error(
        "package `{package}@{from_version}` → `{to_version}` would change the transitive dependency set ({reason}); \
         `vibe update` does not yet handle dep-graph evolution. Run `vibe uninstall {package}` followed by `vibe install {package}` to apply the new graph."
    )]
    DependencyShapeChanged {
        package: String,
        from_version: String,
        to_version: String,
        reason: String,
    },

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

    #[error(
        "subskill `{subskill_path}` could not be read: {detail}"
    )]
    SubskillReadFailed {
        subskill_path: String,
        detail: String,
    },

    #[error(
        "subskill `{subskill_path}` writes target `{}` which collides with another planned write in this install",
        target.display()
    )]
    SubskillFileCollision {
        subskill_path: String,
        target: PathBuf,
    },

    #[error(
        "subskill conflict: `{a}` and `{b}` cannot be active together (declared in `[conflicts].subskills`)"
    )]
    SubskillConflict { a: String, b: String },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteKind {
    /// `[writes]` entry from the package's main manifest.
    Regular,
    /// Top-level `[boot_snippet]` from the package's main manifest.
    BootSnippet,
    /// File belonging to an active subskill. The string is the subskill
    /// path (`stack/rust`, etc.) so install reports can label
    /// subskill-driven writes distinctly from the package's main
    /// content.
    SubskillContent { subskill_path: String },
    /// Boot snippet from a subskill — same as `BootSnippet` but
    /// authored on a subskill rather than the package itself. For now
    /// vibe doesn't have a separate subskill boot mechanism, so this
    /// is reserved.
    SubskillBootSnippet { subskill_path: String },
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

/// Outcome of evaluating a single subskill's activation rules at plan
/// time. Recorded on `InstallPlan` so `register_installed` can emit
/// the lockfile `subskills_active` entry without re-walking the
/// activation logic.
#[derive(Debug, Clone)]
pub struct ActiveSubskill {
    /// Canonical addressable name within the parent package, e.g.
    /// `stack/rust`.
    pub path: String,
    /// Resolved delivery mode at plan time. `lazy-push` / `lazy-pull`
    /// degrade to `eager` until `vibe-mcp` (M1.7) lands; the manifest
    /// value is recorded here regardless so the lockfile remains
    /// truthful and a future resolver upgrade can light up the runtime
    /// behaviour without lockfile churn.
    pub delivery: vibe_core::manifest::DeliveryMode,
    /// Subskill's own `describes` PURL, if any. Forwarded into the
    /// lockfile entry.
    pub describes: Option<String>,
    /// Channels that fired during evaluation (for diagnostic output).
    pub channels_matched: Vec<&'static str>,
    /// Project-relative files this subskill specifically contributed
    /// at plan time. For `eager` / `lazy-push` modes, these are the
    /// target paths under the project root; for `lazy-pull`, this
    /// stays empty because the content never materialises into the
    /// project tree.
    pub files_written: Vec<PathBuf>,
    /// For `lazy-pull` subskills: relative paths inside the package
    /// cache (`<cache>/subskills/<path>/...`). The MCP server
    /// reads these on-demand via `read_subskill`. For `eager` /
    /// `lazy-push` modes, this is also populated so the MCP server
    /// has a precise per-subskill index regardless of delivery
    /// mode.
    pub cache_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub cached: CachedPackage,
    pub writes: Vec<PlannedWrite>,
    pub boot_snippet_filename: Option<String>,
    /// Subskills active for this install. Includes both manual
    /// (feature-driven) and context-based activations.
    pub active_subskills: Vec<ActiveSubskill>,
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

/// Knobs threaded into `plan_install`. Empty default = legacy behaviour
/// (no language fallback, English-only canonical paths, no features
/// activated, no subskills materialised). PROP-003 r2 surfaces every
/// new flag here so the function signature stays stable as further
/// slices land.
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Resolved language preference chain in priority order. First entry
    /// is primary; subsequent entries are fallback. The canonical
    /// (no-suffix) variant is consulted last. Empty = no localisation,
    /// all source files used verbatim.
    pub language_chain: Vec<String>,

    /// Features active for *this* package. Caller (typically
    /// `vibe-cli`) collects from CLI flags, project manifest, and
    /// transitive defaults; passes the resolved set in for the install
    /// plan to materialise feature-driven content (subskill paths from
    /// `[features]` activation, optional-dep flags, …). Empty = no
    /// non-default features active.
    pub feature_expansion: vibe_resolver::FeatureExpansion,

    /// Snapshot of project / machine state used to evaluate subskill
    /// `[activation]` probes. Empty default = no context-based
    /// activation; only manual (via parent feature) channels fire.
    pub activation_context: vibe_resolver::ActivationContext,

    /// Manifest-declared `describes` PURL for this package
    /// (forwarded from the package's `[package].describes`). Lockfile
    /// records this verbatim as a string so PURL parsing differences
    /// across versions don't change the lockfile bytes.
    pub describes: Option<String>,
}

/// Build an [`InstallPlan`] without touching disk beyond reads. Legacy
/// signature — language preference defaults to empty (no localisation).
pub fn plan_install(
    project_root: &Path,
    lockfile: &Lockfile,
    cached: CachedPackage,
) -> Result<InstallPlan, InstallError> {
    plan_install_with_options(project_root, lockfile, cached, &InstallOptions::default())
}

/// Build an [`InstallPlan`] with PROP-003 r2 options (i18n preference,
/// per-package features pre-applied at the call site, etc.). Today this
/// only acts on `language_chain`; future slices wire in feature- and
/// subskill-aware planning.
pub fn plan_install_with_options(
    project_root: &Path,
    lockfile: &Lockfile,
    cached: CachedPackage,
    options: &InstallOptions,
) -> Result<InstallPlan, InstallError> {
    // 1. Lockfile-vs-fetched integrity check first, *before* the
    // already-installed guard. If the lockfile pins a content_hash
    // for this (kind, name) and the freshly-fetched content_hash
    // disagrees, that's a content_drift event — one of:
    //   - upstream force-pushed the version tag,
    //   - a mirror is serving different bytes than canonical,
    //   - `[[override]]` was added pointing somewhere different.
    // Per PROP-002 §2.1 identity = (kind, name, version, content_hash);
    // mismatched content_hash means identity changed, and silent
    // "already installed" would mask the drift. Refuse loud.
    if let Some(existing) = lockfile.find(cached.resolved.kind, &cached.resolved.name) {
        if existing.content_hash != cached.content_hash {
            return Err(InstallError::ContentDrift {
                package: format!("{}:{}", existing.kind, existing.name),
                version: existing.version.to_string(),
                expected: existing.content_hash.clone(),
                actual: cached.content_hash.clone(),
                source_url: cached.source_uri.clone(),
            });
        }
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
    //
    // PROP-003 r2 i18n: source path is resolved through the language
    // fallback chain. `<file>.<lang>.<ext>` is preferred over the
    // canonical form when the consumer's preferred language is set and
    // the localised variant exists. Target path is always canonical —
    // operators want `spec/flows/wal/WAL-PROTOCOL.md`, not
    // `WAL-PROTOCOL.ru.md`, in their tree.
    for file in &manifest.writes.files {
        validate_target_rel(file)?;
        let source_abs = if options.language_chain.is_empty() {
            let abs = cached.cache_dir.join(file);
            if !abs.is_file() {
                return Err(InstallError::MissingSourceFile { path: file.clone() });
            }
            abs
        } else {
            resolve_localised(&cached.cache_dir, file, &options.language_chain)
                .ok_or_else(|| InstallError::MissingSourceFile { path: file.clone() })?
        };
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
        let source_abs = if options.language_chain.is_empty() {
            let abs = cached.cache_dir.join(&snippet.source);
            if !abs.is_file() {
                return Err(InstallError::MissingSourceFile {
                    path: snippet.source.clone(),
                });
            }
            abs
        } else {
            resolve_localised(&cached.cache_dir, &snippet.source, &options.language_chain)
                .ok_or_else(|| InstallError::MissingSourceFile {
                    path: snippet.source.clone(),
                })?
        };
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

    // 4. Subskill discovery + activation + materialisation
    // (PROP-003 §2.5).
    //
    // Walk `<cache>/subskills/<...>/vibe-subskill.toml`, evaluate each
    // subskill's activation rules against the manual channel
    // (`feature_expansion.active_subskills`) and the context probes,
    // and add eager-mode writes to the plan. lazy-push / lazy-pull
    // delivery modes degrade to eager with a `tracing::warn!` until
    // M1.7 (`vibe-mcp`) lands; the manifest mode is preserved on the
    // ActiveSubskill record so the lockfile stays truthful.
    let manifest_describes_type = cached
        .package_meta()
        .describes
        .as_ref()
        .map(|p| p.purl_type.clone());
    let active_subskills = collect_active_subskills(
        &cached.cache_dir,
        &options.feature_expansion,
        &options.activation_context,
        manifest_describes_type.as_deref(),
    )?;

    for sub in &active_subskills {
        let sub_root = cached.cache_dir.join("subskills").join(&sub.path);
        let manifest_path = sub_root.join(SubskillManifestFile::FILENAME);
        let sub_manifest = SubskillManifestFile::read(&manifest_path)
            .map_err(|e| InstallError::SubskillReadFailed {
                subskill_path: sub.path.clone(),
                detail: e.to_string(),
            })?;
        match sub.delivery {
            DeliveryMode::Eager => {}
            DeliveryMode::LazyPush => {
                tracing::warn!(
                    target: "vibe_install",
                    subskill = %sub.path,
                    delivery = "lazy-push",
                    package = %format!("{}:{}", cached.resolved.kind, cached.resolved.name),
                    "subskill delivery `lazy-push` not yet runtime-supported (M2.8); degrading to `eager` for materialisation. Manifest mode preserved in lockfile."
                );
            }
            DeliveryMode::LazyPull => {
                // Genuinely lazy: never materialise into the project
                // tree at install time. The MCP `read_subskill` tool
                // (M1.7 slice 3) reads from the package cache on
                // demand. Lockfile records the cache-file index so
                // the server can locate them without re-walking the
                // subskill manifest at each call.
                tracing::debug!(
                    target: "vibe_install",
                    subskill = %sub.path,
                    package = %format!("{}:{}", cached.resolved.kind, cached.resolved.name),
                    "lazy-pull subskill — skipping materialisation; MCP read_subskill will surface content on demand"
                );
                continue;
            }
        }
        for file in &sub_manifest.content.files_written {
            validate_target_rel(file)?;
            let source_abs = if options.language_chain.is_empty() {
                let abs = sub_root.join(file);
                if !abs.is_file() {
                    return Err(InstallError::MissingSourceFile { path: file.clone() });
                }
                abs
            } else {
                resolve_localised(&sub_root, file, &options.language_chain)
                    .ok_or_else(|| InstallError::MissingSourceFile { path: file.clone() })?
            };
            let target_abs = project_root.join(file);
            reject_existing_target(&target_abs)?;
            let target_rel = normalize_rel(file);
            if !seen_targets.insert(target_rel.clone()) {
                // Same-package collision between base writes and a subskill
                // (or between two active subskills) is an authoring bug —
                // refuse loud.
                return Err(InstallError::SubskillFileCollision {
                    subskill_path: sub.path.clone(),
                    target: target_rel,
                });
            }
            // Boot-prefix conflict guard fires for any file landing
            // under `spec/boot/`. Subskills frequently ship snippets;
            // pin the exact same NN-prefix uniqueness invariant.
            if let Some(boot_filename) = boot_filename_from(&target_rel) {
                validate_boot_filename(&boot_filename)?;
                reject_boot_snippet_conflict(
                    project_root,
                    lockfile,
                    &boot_filename,
                    &cached.resolved.kind,
                    &cached.resolved.name,
                )?;
            }
            writes.push(PlannedWrite {
                kind: WriteKind::SubskillContent {
                    subskill_path: sub.path.clone(),
                },
                source_abs,
                target_rel,
                target_abs,
            });
        }
    }

    Ok(InstallPlan {
        cached,
        writes,
        boot_snippet_filename,
        active_subskills,
    })
}

// ----------------------------------------------------------------------
// Subskill discovery + activation
// ----------------------------------------------------------------------

use vibe_core::manifest::DeliveryMode;
use vibe_core::manifest::SubskillManifest as SubskillManifestFile;

/// Walk `<cache_dir>/subskills/` non-recursively past depth 1
/// (PROP-003 §2.5.5 caps recursion at 3; recursive subskill resolution
/// is queued for a follow-up slice). Returns the active set in
/// canonical-path-sorted order so install plans are deterministic.
fn collect_active_subskills(
    cache_dir: &Path,
    feat_exp: &vibe_resolver::FeatureExpansion,
    ctx: &vibe_resolver::ActivationContext,
    package_describes_type: Option<&str>,
) -> Result<Vec<ActiveSubskill>, InstallError> {
    let subskills_dir = cache_dir.join("subskills");
    if !subskills_dir.is_dir() {
        return Ok(Vec::new());
    }

    // Collect every subskill manifest plus the relative path inside
    // `subskills/`. Walk to a generous depth (8) so deeply-nested layouts
    // still surface; the depth cap from PROP-003 §2.5.5 is enforced via
    // a separate findings pass in `vibe-check`, not here.
    let mut found: Vec<(String, SubskillManifestFile)> = Vec::new();
    for entry in walkdir::WalkDir::new(&subskills_dir)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != SubskillManifestFile::FILENAME {
            continue;
        }
        let manifest_path = entry.path();
        let parent = manifest_path
            .parent()
            .ok_or_else(|| InstallError::SubskillReadFailed {
                subskill_path: manifest_path.display().to_string(),
                detail: "subskill manifest has no parent dir".into(),
            })?;
        let rel = parent
            .strip_prefix(&subskills_dir)
            .map_err(|e| InstallError::SubskillReadFailed {
                subskill_path: parent.display().to_string(),
                detail: e.to_string(),
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let manifest = SubskillManifestFile::read(manifest_path).map_err(|e| {
            InstallError::SubskillReadFailed {
                subskill_path: rel.clone(),
                detail: e.to_string(),
            }
        })?;
        // The `path` field in the manifest must match the directory path —
        // catches authoring drift loudly.
        if manifest.subskill.path != rel {
            return Err(InstallError::SubskillReadFailed {
                subskill_path: rel.clone(),
                detail: format!(
                    "subskill manifest declares path `{}` but lives at `subskills/{}`",
                    manifest.subskill.path, rel
                ),
            });
        }
        found.push((rel, manifest));
    }
    // Sort for deterministic order.
    found.sort_by(|a, b| a.0.cmp(&b.0));

    // Evaluate activation per subskill.
    let mut active: Vec<ActiveSubskill> = Vec::new();
    for (path, manifest) in &found {
        let manual_match = feat_exp.active_subskills.contains(path);
        let sub_describes_type = manifest
            .subskill
            .describes
            .as_ref()
            .map(|p| p.purl_type.as_str())
            .or(package_describes_type);
        let probe_outcome =
            vibe_resolver::activation::evaluate(&manifest.activation, ctx, sub_describes_type);
        if !manual_match && !probe_outcome.active {
            continue;
        }
        let mut channels: Vec<&'static str> = Vec::new();
        if manual_match {
            channels.push("manual");
        }
        channels.extend(probe_outcome.channels_matched);
        let files_written: Vec<PathBuf> = match manifest.subskill.delivery {
            // `lazy-pull` keeps the project tree clean — files stay
            // in the package cache and surface only via the MCP
            // `read_subskill` tool.
            DeliveryMode::LazyPull => Vec::new(),
            // `eager` and `lazy-push` (lazy-push degrades to eager
            // until M2.8) materialise files into the project; their
            // declared `files_written` are the target paths.
            _ => manifest
                .content
                .files_written
                .iter()
                .map(|p| normalize_rel(p))
                .collect(),
        };
        let cache_files: Vec<PathBuf> = manifest
            .content
            .files_written
            .iter()
            .map(|p| normalize_rel(p))
            .collect();
        active.push(ActiveSubskill {
            path: path.clone(),
            delivery: manifest.subskill.delivery,
            describes: manifest.subskill.describes.as_ref().map(|p| p.to_string()),
            channels_matched: channels,
            files_written,
            cache_files,
        });
    }

    // Conflict enforcement: every active subskill's
    // `[conflicts].subskills` list must not contain another active
    // subskill from the same package.
    let active_paths: std::collections::HashSet<String> =
        active.iter().map(|a| a.path.clone()).collect();
    for (path, manifest) in &found {
        if !active_paths.contains(path) {
            continue;
        }
        for conflict in &manifest.conflicts.subskills {
            if active_paths.contains(conflict) {
                return Err(InstallError::SubskillConflict {
                    a: path.clone(),
                    b: conflict.clone(),
                });
            }
        }
    }

    Ok(active)
}

/// Extract the boot-snippet filename from a `spec/boot/<NN-name>.<ext>`
/// path. Returns `None` for any other shape.
fn boot_filename_from(target_rel: &Path) -> Option<String> {
    let s = target_rel.to_string_lossy().replace('\\', "/");
    let stripped = s.strip_prefix("spec/boot/")?;
    if stripped.contains('/') {
        return None;
    }
    Some(stripped.to_string())
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
///
/// `dependencies` carries the exact-pinned transitive deps of the
/// installed package as decided by the depsolver. Empty `Vec` is the
/// pre-resolver shape — pre-PROP-002 callers and the legacy single-pkg
/// install path.
pub fn register_installed(
    lockfile: &mut Lockfile,
    plan: &InstallPlan,
    files_written: Vec<PathBuf>,
    generated_at: String,
    dependencies: Vec<PackageRef>,
) {
    register_installed_with_metadata(
        lockfile,
        plan,
        files_written,
        generated_at,
        dependencies,
        RegisterMetadata::default(),
    );
}

/// PROP-003 r2 lockfile-v3 fields. Empty/None defaults preserve the
/// legacy v2-shape lockfile bytes for installs that don't activate
/// features / subskills / language.
#[derive(Debug, Default, Clone)]
pub struct RegisterMetadata {
    pub features: Vec<String>,
    pub describes: Option<String>,
    pub language: Option<String>,
}

/// Register an install with the full PROP-003 r2 v3 metadata. The
/// `features`, `describes`, and `language` fields land directly in
/// the lockfile entry; `subskills_active` is sourced from
/// `plan.active_subskills`.
pub fn register_installed_with_metadata(
    lockfile: &mut Lockfile,
    plan: &InstallPlan,
    files_written: Vec<PathBuf>,
    generated_at: String,
    dependencies: Vec<PackageRef>,
    metadata: RegisterMetadata,
) {
    let subskills_active: Vec<vibe_core::manifest::LockedSubskill> = plan
        .active_subskills
        .iter()
        .map(|s| vibe_core::manifest::LockedSubskill {
            path: s.path.clone(),
            delivery: s.delivery.as_str().to_string(),
            describes: s.describes.clone(),
            files_written: s.files_written.clone(),
            cache_files: s.cache_files.clone(),
        })
        .collect();
    let source_kind = if plan.cached.overridden {
        Some(vibe_core::manifest::SourceKind::Override)
    } else if plan.cached.is_path_source {
        // Path-source (PROP-007 §2.5) — a workspace-local package.
        // `source_url` (`cached.source_uri`) already carries the
        // workspace-relative path; no special-casing needed here.
        Some(vibe_core::manifest::SourceKind::Path)
    } else if plan.cached.is_git_source {
        Some(vibe_core::manifest::SourceKind::Git)
    } else {
        Some(vibe_core::manifest::SourceKind::Registry)
    };
    let entry = LockedPackage {
        kind: plan.cached.resolved.kind,
        name: plan.cached.resolved.name.clone(),
        version: plan.cached.resolved.version.clone(),
        registry: plan.cached.registry_name.clone(),
        source_url: plan.cached.source_uri.clone(),
        source_ref: plan.cached.source_ref.clone(),
        resolved_commit: plan.cached.resolved_commit.clone(),
        content_hash: plan.cached.content_hash.clone(),
        boot_snippet: plan.boot_snippet_filename.clone(),
        files_written,
        dependencies,
        overridden: plan.cached.overridden,
        source_kind,
        via_redirect: plan.cached.via_redirect.clone(),
        features: metadata.features,
        subskills_active,
        describes: metadata.describes,
        language: metadata.language,
    };
    lockfile.packages.push(entry);
    lockfile.meta.generated_at = generated_at;
}

// ==== update ===============================================================
//
// `vibe update` re-fetches a currently-installed package against its
// original root constraint, computes a project-file diff (added /
// removed / modified / identical / user-edited), and applies the diff
// after user confirmation.
//
// Spec: VIBEVM-SPEC.md §16 (M1 acceptance), ROADMAP §M1.2,
//       PROP-002 §2.7 (lockfile v2 — `dependencies` shape change is
//       refused at this layer per the spec's "narrow update" v0).

/// Per-file outcome the update would apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateChange {
    /// File is in the new package's writes but not in the old install.
    /// `apply_update` will write it.
    Added {
        target_rel: PathBuf,
        target_abs: PathBuf,
        source_abs: PathBuf,
    },
    /// File is in the old install but not in the new package's writes.
    /// `apply_update` will delete it.
    Removed {
        target_rel: PathBuf,
        target_abs: PathBuf,
    },
    /// File is in both old and new; bytes differ between old cache and
    /// new cache, project file matches the old cache (pristine).
    /// `apply_update` will overwrite from the new cache.
    Modified {
        target_rel: PathBuf,
        target_abs: PathBuf,
        source_abs: PathBuf,
    },
    /// File is in both old and new with byte-identical content. No-op
    /// at apply time; surfaced in `--json` output for completeness.
    Identical { target_rel: PathBuf },
}

impl UpdateChange {
    pub fn target_rel(&self) -> &Path {
        match self {
            UpdateChange::Added { target_rel, .. }
            | UpdateChange::Removed { target_rel, .. }
            | UpdateChange::Modified { target_rel, .. }
            | UpdateChange::Identical { target_rel } => target_rel,
        }
    }
}

/// Plan for updating one already-installed package to a new version
/// (or fresh content_hash at the same version, e.g. a deliberate
/// repackage).
#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub kind: PackageKind,
    pub name: String,
    pub from_version: semver::Version,
    pub to_version: semver::Version,
    pub from_content_hash: String,
    pub to_content_hash: String,
    /// New `CachedPackage` populated by the resolver. Carries the
    /// fresh manifest (which `register_updated` writes back into the
    /// lockfile) and the new cache_dir paths used for `apply_update`.
    pub new_cached: CachedPackage,
    /// Per-file changes, in stable order: removed, added, modified,
    /// identical. Lets the CLI render a deterministic diff.
    pub changes: Vec<UpdateChange>,
    /// New boot snippet filename, if the new manifest declares one.
    /// May differ from the old install's boot snippet (rename), in
    /// which case the old name appears in `changes` as `Removed` and
    /// the new name as `Added`.
    pub new_boot_snippet_filename: Option<String>,
}

impl UpdatePlan {
    pub fn package_label(&self) -> String {
        format!("{}:{}", self.kind, self.name)
    }
    /// `true` iff the plan would change at least one byte on disk.
    /// Pure-Identical plans are no-ops at apply time.
    pub fn has_changes(&self) -> bool {
        self.changes
            .iter()
            .any(|c| !matches!(c, UpdateChange::Identical { .. }))
    }
}

/// Build an [`UpdatePlan`] from the lockfile entry for `(kind, name)`
/// and a freshly fetched [`CachedPackage`] (returned by the resolver
/// for the same root constraint).
///
/// Refuses to plan when:
/// - the package is not installed (`NotInstalled`);
/// - the new manifest's `[requires]` differs from the locked
///   transitive dep set (`DependencyShapeChanged`) — narrow v0 of
///   `vibe update` does not cascade graph changes;
/// - the install-time cache for the old version is missing
///   (`OldCacheMissing`) — required for the user-edit verification.
///
/// User-edit detection: for every file in the union of old `files_written`
/// and new `manifest.writes`, compares the project's on-disk bytes to
/// the **old** cache's bytes for the same source. Mismatch is `UserEditedFile`
/// — refused loudly with a 3-way diff hint per ROADMAP §M1.2.
pub fn plan_update(
    project_root: &Path,
    lockfile: &Lockfile,
    new_cached: CachedPackage,
    old_cache_dir: &Path,
) -> Result<UpdatePlan, InstallError> {
    let kind = new_cached.resolved.kind;
    let name = new_cached.resolved.name.clone();
    let pkg_label = format!("{kind}:{name}");
    let existing = lockfile
        .find(kind, &name)
        .ok_or_else(|| InstallError::NotInstalled {
            package: pkg_label.clone(),
        })?;

    if !old_cache_dir.is_dir() {
        return Err(InstallError::OldCacheMissing {
            package: pkg_label,
            from_version: existing.version.to_string(),
            old_cache_dir: old_cache_dir.to_path_buf(),
        });
    }

    // --- Refuse on dep-shape change (narrow v0). -----------------
    //
    // The lockfile's `dependencies` array is the resolved transitive
    // pinning at install time. The new manifest declares its own
    // `[requires]` (and friends). For the pure version-bump v0, we
    // want the bumped version's resolved deps to match what's already
    // locked — i.e. no new transitive packages, no removed transitive
    // packages, no version bumps in transitives. That keeps `vibe
    // update` honest about its scope: project on-disk files change,
    // dep graph does not.
    //
    // We compare the *manifest declarations* (as `(kind, name)` set,
    // ignoring version) because doing a full re-resolve here would
    // require the resolver — `plan_update` is purely lockfile-aware.
    // `(kind, name)` keyed via display strings — `PackageKind` is not
    // `Ord`, and we don't need it to be: we just need a stable bag for
    // set-difference reporting.
    let old_dep_keys: std::collections::BTreeSet<String> = existing
        .dependencies
        .iter()
        .map(|p| format!("{}:{}", p.kind, p.name))
        .collect();
    let new_dep_keys: std::collections::BTreeSet<String> = new_cached
        .manifest
        .requires
        .packages
        .iter()
        .map(|spec| format!("{}:{}", spec.kind, spec.name))
        .collect();
    if old_dep_keys != new_dep_keys {
        let added: Vec<String> = new_dep_keys
            .difference(&old_dep_keys)
            .map(|s| format!("+{s}"))
            .collect();
        let removed: Vec<String> = old_dep_keys
            .difference(&new_dep_keys)
            .map(|s| format!("-{s}"))
            .collect();
        let mut parts: Vec<String> = added;
        parts.extend(removed);
        return Err(InstallError::DependencyShapeChanged {
            package: pkg_label,
            from_version: existing.version.to_string(),
            to_version: new_cached.resolved.version.to_string(),
            reason: parts.join(" "),
        });
    }

    // --- Build the new install plan as if we were doing a fresh write,
    // then convert it into a per-file diff against the old install.
    //
    // We can't call `plan_install` directly here because that path
    // refuses on AlreadyInstalled. The shape we want — list of
    // (project_rel, source_abs) for the new package — is a reduced
    // version of plan_install's body, so duplicate it locally.
    let mut new_targets: Vec<(PathBuf, PathBuf)> = Vec::new(); // (target_rel, source_abs)
    let new_manifest = &new_cached.manifest;
    for file in &new_manifest.writes.files {
        validate_target_rel(file)?;
        let source_abs = new_cached.cache_dir.join(file);
        if !source_abs.is_file() {
            return Err(InstallError::MissingSourceFile { path: file.clone() });
        }
        new_targets.push((normalize_rel(file), source_abs));
    }
    let mut new_boot_snippet_filename = None;
    if let Some(snippet) = &new_manifest.boot_snippet {
        validate_boot_filename(&snippet.filename)?;
        let source_abs = new_cached.cache_dir.join(&snippet.source);
        if !source_abs.is_file() {
            return Err(InstallError::MissingSourceFile {
                path: snippet.source.clone(),
            });
        }
        let target_rel = normalize_rel(Path::new(&format!("spec/boot/{}", snippet.filename)));
        new_targets.push((target_rel, source_abs));
        new_boot_snippet_filename = Some(snippet.filename.clone());
    }

    let new_paths: std::collections::BTreeSet<PathBuf> =
        new_targets.iter().map(|(t, _)| t.clone()).collect();
    let new_source_lookup: std::collections::HashMap<PathBuf, PathBuf> = new_targets
        .iter()
        .cloned()
        .collect();
    let old_paths: std::collections::BTreeSet<PathBuf> = existing
        .files_written
        .iter()
        .map(|p| normalize_rel(p))
        .collect();

    // --- For pristine-vs-edited verification we need a cache→project
    // mapping for the **old** install. Re-derive it from the old cache's
    // manifest.
    let old_manifest_path = old_cache_dir.join(Manifest::FILENAME);
    let old_manifest = Manifest::read(&old_manifest_path)?;
    let mut old_source_lookup: std::collections::HashMap<PathBuf, PathBuf> =
        std::collections::HashMap::new();
    for file in &old_manifest.writes.files {
        old_source_lookup.insert(normalize_rel(file), old_cache_dir.join(file));
    }
    if let Some(snippet) = &old_manifest.boot_snippet {
        let target_rel = normalize_rel(Path::new(&format!("spec/boot/{}", snippet.filename)));
        old_source_lookup.insert(target_rel, old_cache_dir.join(&snippet.source));
    }

    let mut changes: Vec<UpdateChange> = Vec::new();

    // Removed files — present in the old install, absent in the new.
    // Refuse if user has edited the file (would silently destroy work).
    for old in &old_paths {
        if new_paths.contains(old) {
            continue;
        }
        let target_abs = project_root.join(old);
        check_user_edit(&pkg_label, old, &target_abs, &old_source_lookup)?;
        changes.push(UpdateChange::Removed {
            target_rel: old.clone(),
            target_abs,
        });
    }

    // Both-side files: classify Identical / Modified / UserEdited.
    for path in old_paths.intersection(&new_paths) {
        let target_abs = project_root.join(path);
        check_user_edit(&pkg_label, path, &target_abs, &old_source_lookup)?;
        // Pristine. Compare old vs new cache bytes.
        let old_src = old_source_lookup
            .get(path)
            .cloned()
            .ok_or_else(|| InstallError::Io {
                path: path.clone(),
                source: std::io::Error::other(format!(
                    "no old-cache mapping for `{}` while planning update of `{}`",
                    path.display(),
                    pkg_label
                )),
            })?;
        let new_src = new_source_lookup
            .get(path)
            .cloned()
            .expect("new path came from new_targets and is recorded");
        let old_bytes = fs::read(&old_src).map_err(|e| InstallError::Io {
            path: old_src.clone(),
            source: e,
        })?;
        let new_bytes = fs::read(&new_src).map_err(|e| InstallError::Io {
            path: new_src.clone(),
            source: e,
        })?;
        if old_bytes == new_bytes {
            changes.push(UpdateChange::Identical {
                target_rel: path.clone(),
            });
        } else {
            changes.push(UpdateChange::Modified {
                target_rel: path.clone(),
                target_abs,
                source_abs: new_src,
            });
        }
    }

    // Added — in the new install, not in the old.
    for new_path in &new_paths {
        if old_paths.contains(new_path) {
            continue;
        }
        let target_abs = project_root.join(new_path);
        // Any pre-existing file at the target is a hard conflict —
        // either a stray manual edit or another package's write.
        if target_abs.exists() {
            return Err(InstallError::TargetFileExists {
                path: target_abs,
            });
        }
        let source_abs = new_source_lookup
            .get(new_path)
            .cloned()
            .expect("new path has a recorded source");
        changes.push(UpdateChange::Added {
            target_rel: new_path.clone(),
            target_abs,
            source_abs,
        });
    }

    // Sort changes for deterministic CLI output: Removed, Added, Modified, Identical.
    changes.sort_by_key(|c| {
        let order = match c {
            UpdateChange::Removed { .. } => 0,
            UpdateChange::Added { .. } => 1,
            UpdateChange::Modified { .. } => 2,
            UpdateChange::Identical { .. } => 3,
        };
        (order, c.target_rel().to_path_buf())
    });

    Ok(UpdatePlan {
        kind,
        name,
        from_version: existing.version.clone(),
        to_version: new_cached.resolved.version.clone(),
        from_content_hash: existing.content_hash.clone(),
        to_content_hash: new_cached.content_hash.clone(),
        new_cached,
        changes,
        new_boot_snippet_filename,
    })
}

fn check_user_edit(
    pkg_label: &str,
    rel: &Path,
    abs: &Path,
    old_source_lookup: &std::collections::HashMap<PathBuf, PathBuf>,
) -> Result<(), InstallError> {
    if !abs.exists() {
        // Project file already missing on disk — user removed it. We
        // treat that as an implicit consent to delete-or-add-back; no
        // refusal needed. The new write (if any) just lands; the
        // removal is a no-op.
        return Ok(());
    }
    let Some(old_src) = old_source_lookup.get(rel) else {
        // Path isn't in the old manifest's writes — must be a stray.
        // Only reachable for `Added` paths; we already handle that
        // case in plan_update above.
        return Ok(());
    };
    let on_disk = fs::read(abs).map_err(|e| InstallError::Io {
        path: abs.to_path_buf(),
        source: e,
    })?;
    let cache = fs::read(old_src).map_err(|e| InstallError::Io {
        path: old_src.clone(),
        source: e,
    })?;
    if on_disk != cache {
        return Err(InstallError::UserEditedFile {
            package: pkg_label.to_string(),
            path: rel.to_path_buf(),
        });
    }
    Ok(())
}

/// Apply an [`UpdatePlan`] — delete `Removed`, write `Added` and
/// `Modified`. `Identical` entries are no-ops. Returns the project-
/// relative paths the lockfile should record under `files_written`
/// (everything in the new install, dropping the removed paths).
///
/// Best-effort rollback on failure: anything written or deleted in
/// this call is restored if a later step errors. Backed by a snapshot
/// taken at the start.
pub fn apply_update(plan: &UpdatePlan) -> Result<Vec<PathBuf>, InstallError> {
    // Snapshot the bytes of every Removed/Modified target so we can
    // roll back. Added targets are absent today; rollback removes
    // them on failure.
    let mut snapshots: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for change in &plan.changes {
        match change {
            UpdateChange::Removed { target_abs, .. } | UpdateChange::Modified { target_abs, .. } => {
                let bytes = if target_abs.exists() {
                    Some(fs::read(target_abs).map_err(|e| InstallError::Io {
                        path: target_abs.clone(),
                        source: e,
                    })?)
                } else {
                    None
                };
                snapshots.push((target_abs.clone(), bytes));
            }
            UpdateChange::Added { target_abs, .. } => {
                snapshots.push((target_abs.clone(), None));
            }
            UpdateChange::Identical { .. } => {}
        }
    }

    let mut written: Vec<PathBuf> = Vec::new();
    let mut deleted: Vec<PathBuf> = Vec::new();
    let mut new_files_written: Vec<PathBuf> = Vec::new();

    let result = (|| -> Result<(), InstallError> {
        for change in &plan.changes {
            match change {
                UpdateChange::Removed { target_abs, target_rel } => {
                    if target_abs.exists() {
                        fs::remove_file(target_abs).map_err(|e| InstallError::Io {
                            path: target_abs.clone(),
                            source: e,
                        })?;
                        deleted.push(target_abs.clone());
                    }
                    let _ = target_rel; // recorded only via files_written omission
                }
                UpdateChange::Added { target_abs, target_rel, source_abs }
                | UpdateChange::Modified { target_abs, target_rel, source_abs } => {
                    if let Some(parent) = target_abs.parent() {
                        fs::create_dir_all(parent).map_err(|e| InstallError::Io {
                            path: parent.to_path_buf(),
                            source: e,
                        })?;
                    }
                    fs::copy(source_abs, target_abs).map_err(|e| InstallError::Io {
                        path: target_abs.clone(),
                        source: e,
                    })?;
                    written.push(target_abs.clone());
                    new_files_written.push(target_rel.clone());
                }
                UpdateChange::Identical { target_rel } => {
                    new_files_written.push(target_rel.clone());
                }
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        // Roll back. Restore every snapshot.
        for (abs, prior) in snapshots.iter().rev() {
            match prior {
                Some(bytes) => {
                    if let Some(parent) = abs.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(abs, bytes);
                }
                None => {
                    let _ = fs::remove_file(abs);
                }
            }
        }
        let _ = (written, deleted);
        return Err(e);
    }

    // Prune empty parent dirs whose only previous content was a
    // removed file. Same heuristic as `apply_uninstall`.
    for change in &plan.changes {
        if let UpdateChange::Removed { target_abs, .. } = change {
            let mut p = target_abs.clone();
            while let Some(parent) = p.parent() {
                if parent.as_os_str().is_empty() {
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
    }

    // Stable order for the lockfile: sort by target_rel so a follow-up
    // `vibe list` / `vibe.lock` diff is deterministic.
    new_files_written.sort();
    Ok(new_files_written)
}

/// Update the lockfile entry for `(kind, name)` to reflect the
/// applied [`UpdatePlan`]. Replaces the version, content_hash,
/// source_url, source_ref, resolved_commit, boot_snippet,
/// files_written; preserves the existing `dependencies` (refused on
/// shape change at plan time) and `overridden` flag.
pub fn register_updated(
    lockfile: &mut Lockfile,
    plan: &UpdatePlan,
    files_written: Vec<PathBuf>,
    generated_at: String,
) -> Result<(), InstallError> {
    let pkg_label = plan.package_label();
    let entry = lockfile
        .find_mut(plan.kind, &plan.name)
        .ok_or(InstallError::NotInstalled {
            package: pkg_label,
        })?;
    entry.version = plan.to_version.clone();
    entry.content_hash = plan.to_content_hash.clone();
    entry.source_url = plan.new_cached.source_uri.clone();
    entry.source_ref = plan.new_cached.source_ref.clone();
    entry.resolved_commit = plan.new_cached.resolved_commit.clone();
    entry.registry = plan.new_cached.registry_name.clone();
    entry.boot_snippet = plan.new_boot_snippet_filename.clone();
    entry.files_written = files_written;
    // `dependencies` and `overridden` are preserved — narrow v0
    // refuses dep-shape changes, so the locked transitive set is
    // still valid; an override-resolved entry stays an override.
    lockfile.meta.generated_at = generated_at;
    Ok(())
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
    // A user-typed root recorded in `[meta].root_dependencies` is the
    // mirror of what they passed to `vibe install`. Uninstalling that
    // package drops it from the root list — symmetric with the merge
    // on the install side. Transitives never appear here, so this is
    // a no-op for solver-pulled deps. See manual-tests/M1.5-gate-v2-
    // per-package-smoke.md step 8 for the contract.
    lockfile
        .meta
        .root_dependencies
        .retain(|r| !(r.kind == pkgref.kind && r.name == pkgref.name));
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
        fs::write(pkg_dir.join("vibe.toml"), manifest_toml).unwrap();
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

        register_installed(&mut lockfile, &plan, written, "t1".into(), Vec::new());
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
        register_installed(&mut lockfile, &plan, written, "t1".into(), Vec::new());

        // Plan again: should error — same content_hash, so AlreadyInstalled
        // not ContentDrift.
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        assert!(matches!(err, InstallError::AlreadyInstalled { .. }));
    }

    #[test]
    fn content_drift_errors_when_lockfile_hash_mismatches_fetched() {
        // Prove PROP-002 §2.1: a re-resolve that produces a different
        // content_hash for the same (kind, name, version) refuses to
        // proceed — masks neither as AlreadyInstalled nor as success.
        let reg_dir = tempdir().unwrap();
        seed_registry(reg_dir.path(), FIXTURE_MANIFEST, &fixture_content());
        let cache_dir = tempdir().unwrap();
        let cached = cached_for_test(reg_dir.path(), cache_dir.path());

        // Pre-populate the lockfile with an entry that pins a *different*
        // content_hash than what `cached` carries — simulates the case
        // where the registry's `flow:wal@0.3.0` tag was force-pushed
        // since the lockfile was written, OR a mirror is serving
        // different bytes than canonical.
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        lockfile.packages.push(LockedPackage {
            kind: cached.resolved.kind,
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            registry: Some("vibespecs".into()),
            source_url: "git@gitverse.ru:vibespecs/flow-wal.git".into(),
            source_ref: Some("v0.3.0".into()),
            resolved_commit: None,
            content_hash: "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
                .into(),
            boot_snippet: Some("10-flow-wal.md".into()),
            files_written: vec![],
            dependencies: Vec::new(),
            overridden: false,
            source_kind: Some(vibe_core::manifest::SourceKind::Registry),
            via_redirect: None,
            features: Vec::new(),
            subskills_active: Vec::new(),
            describes: None,
            language: None,
        });

        let project = empty_project();
        let err = plan_install(project.path(), &lockfile, cached).unwrap_err();
        match err {
            InstallError::ContentDrift {
                package,
                version,
                expected,
                actual,
                ..
            } => {
                assert_eq!(package, "flow:wal");
                assert_eq!(version, "0.3.0");
                assert_eq!(expected, "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
                assert!(actual.starts_with("sha256:"));
                assert_ne!(expected, actual);
            }
            other => panic!("expected ContentDrift, got: {other:?}"),
        }
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
        fs::write(pkg_dir.join("vibe.toml"), manifest).unwrap();
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
        register_installed(&mut lockfile, &plan, written, "t1".into(), Vec::new());

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
    fn unregister_drops_root_dependency_entry() {
        // Roots are recorded by `register_installed` (or by the install
        // CLI's `merge_root_dependencies`); unregistering must remove
        // them — the contract `manual-tests/M1.5-gate-v2-per-package-
        // smoke.md` step 8 depends on.
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        let pkgref = PackageRef::parse("flow:wal").unwrap();
        let other = PackageRef::parse("flow:atomic-commits").unwrap();
        lockfile.meta.root_dependencies =
            vec![pkgref.clone(), other.clone()];
        lockfile.packages.push(LockedPackage {
            kind: PackageKind::Flow,
            name: "wal".into(),
            version: semver::Version::parse("0.1.0").unwrap(),
            registry: None,
            source_url: "file:///fake".into(),
            source_ref: None,
            resolved_commit: None,
            content_hash: "sha256:whatever".into(),
            boot_snippet: None,
            files_written: Vec::new(),
            dependencies: Vec::new(),
            overridden: false,
            source_kind: Some(vibe_core::manifest::SourceKind::Registry),
            via_redirect: None,
            features: Vec::new(),
            subskills_active: Vec::new(),
            describes: None,
            language: None,
        });

        let _ = unregister_installed(&mut lockfile, &pkgref, "t1".into()).unwrap();

        // `flow:wal` is gone from roots; the unrelated root survives.
        assert_eq!(lockfile.meta.root_dependencies.len(), 1);
        assert_eq!(lockfile.meta.root_dependencies[0].name, "atomic-commits");
    }

    // ===================== update tests =====================

    /// Seed `<reg_root>/<kind>/<name>/v<version>/` with the supplied
    /// manifest + content. Caller controls the version, so the same
    /// helper writes both the "old" and the "new" install of a
    /// package.
    fn seed_version(
        reg_root: &Path,
        kind: &str,
        name: &str,
        version: &str,
        manifest_toml: &str,
        content: &[(&str, &str)],
    ) {
        let pkg_dir = reg_root
            .join(kind)
            .join(name)
            .join(format!("v{version}"));
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(Manifest::FILENAME), manifest_toml).unwrap();
        for (rel, body) in content {
            let path = pkg_dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
        }
    }

    fn cached_at(
        registry_root: &Path,
        cache_root: &Path,
        kind: &str,
        name: &str,
        version: &str,
    ) -> CachedPackage {
        let reg = LocalRegistry::new(registry_root).unwrap();
        let pkgref = PackageRef::parse(&format!("{kind}:{name}@{version}")).unwrap();
        let resolved: ResolvedPackage = reg.resolve(&pkgref).unwrap();
        reg.fetch(&resolved, cache_root).unwrap()
    }

    fn install_v1(
        reg_dir: &tempfile::TempDir,
        cache_dir: &tempfile::TempDir,
        project: &tempfile::TempDir,
    ) -> Lockfile {
        let manifest_v1 = r#"
[package]
name = "wal"
kind = "flow"
version = "0.1.0"

[writes]
files = [
    "spec/flows/wal/A.md",
    "spec/flows/wal/B.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"
"#;
        seed_version(
            reg_dir.path(),
            "flow",
            "wal",
            "0.1.0",
            manifest_v1,
            &[
                ("spec/flows/wal/A.md", "v1 A\n"),
                ("spec/flows/wal/B.md", "v1 B\n"),
                ("boot/10-flow-wal.md", "v1 boot\n"),
            ],
        );
        let cached = cached_at(reg_dir.path(), cache_dir.path(), "flow", "wal", "0.1.0");
        let mut lockfile = Lockfile::empty("vibe-test", "t0");
        let plan = plan_install(project.path(), &lockfile, cached).unwrap();
        let written = apply_install(&plan).unwrap();
        register_installed(&mut lockfile, &plan, written, "t0".into(), Vec::new());
        lockfile
    }

    fn fetch_v2(
        reg_dir: &tempfile::TempDir,
        cache_dir: &tempfile::TempDir,
    ) -> CachedPackage {
        // v0.2.0: A.md changed bytes, B.md removed, C.md added, boot
        // snippet unchanged. Tests Added / Removed / Modified /
        // Identical classification in one shot.
        let manifest_v2 = r#"
[package]
name = "wal"
kind = "flow"
version = "0.2.0"

[writes]
files = [
    "spec/flows/wal/A.md",
    "spec/flows/wal/C.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"
"#;
        seed_version(
            reg_dir.path(),
            "flow",
            "wal",
            "0.2.0",
            manifest_v2,
            &[
                ("spec/flows/wal/A.md", "v2 A — changed!\n"),
                ("spec/flows/wal/C.md", "v2 C\n"),
                ("boot/10-flow-wal.md", "v1 boot\n"), // unchanged → Identical
            ],
        );
        cached_at(reg_dir.path(), cache_dir.path(), "flow", "wal", "0.2.0")
    }

    #[test]
    fn plan_update_classifies_added_removed_modified_identical() {
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let lockfile = install_v1(&reg_dir, &cache_dir, &project);
        let new_cached = fetch_v2(&reg_dir, &cache_dir);

        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        let plan = plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap();

        assert_eq!(plan.from_version.to_string(), "0.1.0");
        assert_eq!(plan.to_version.to_string(), "0.2.0");
        assert!(plan.has_changes());

        // 1 Removed (B), 1 Added (C), 1 Modified (A), 1 Identical (boot).
        let count = |pred: fn(&UpdateChange) -> bool| {
            plan.changes.iter().filter(|c| pred(c)).count()
        };
        assert_eq!(count(|c| matches!(c, UpdateChange::Removed { .. })), 1);
        assert_eq!(count(|c| matches!(c, UpdateChange::Added { .. })), 1);
        assert_eq!(count(|c| matches!(c, UpdateChange::Modified { .. })), 1);
        assert_eq!(count(|c| matches!(c, UpdateChange::Identical { .. })), 1);

        // Spot-check the actual paths.
        let removed_paths: Vec<&Path> = plan
            .changes
            .iter()
            .filter_map(|c| match c {
                UpdateChange::Removed { target_rel, .. } => Some(target_rel.as_path()),
                _ => None,
            })
            .collect();
        assert_eq!(removed_paths, vec![Path::new("spec/flows/wal/B.md")]);

        let added_paths: Vec<&Path> = plan
            .changes
            .iter()
            .filter_map(|c| match c {
                UpdateChange::Added { target_rel, .. } => Some(target_rel.as_path()),
                _ => None,
            })
            .collect();
        assert_eq!(added_paths, vec![Path::new("spec/flows/wal/C.md")]);
    }

    #[test]
    fn plan_update_refuses_user_edited_file() {
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let lockfile = install_v1(&reg_dir, &cache_dir, &project);

        // User edits A.md after install. plan_update would overwrite
        // the edit with v2 bytes — refuse.
        fs::write(
            project.path().join("spec/flows/wal/A.md"),
            "user-edited locally\n",
        )
        .unwrap();

        let new_cached = fetch_v2(&reg_dir, &cache_dir);
        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        let err = plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap_err();
        match err {
            InstallError::UserEditedFile { package, path } => {
                assert_eq!(package, "flow:wal");
                assert_eq!(path, PathBuf::from("spec/flows/wal/A.md"));
            }
            other => panic!("expected UserEditedFile, got: {other:?}"),
        }
    }

    #[test]
    fn plan_update_refuses_old_cache_missing() {
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let lockfile = install_v1(&reg_dir, &cache_dir, &project);
        let new_cached = fetch_v2(&reg_dir, &cache_dir);

        // Wipe the v0.1.0 cache so plan_update can't verify pristineness.
        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        fs::remove_dir_all(&old_cache_dir).unwrap();

        let err = plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap_err();
        assert!(matches!(err, InstallError::OldCacheMissing { .. }));
    }

    #[test]
    fn plan_update_refuses_dependency_shape_change() {
        // v1 has empty deps; v2 declares a new transitive. Refuse.
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let lockfile = install_v1(&reg_dir, &cache_dir, &project);

        let manifest_v2 = r#"
[package]
name = "wal"
kind = "flow"
version = "0.2.0"

[writes]
files = ["spec/flows/wal/A.md"]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"

[requires.packages]
"flow:atomic-commits" = "^0.1"
"#;
        seed_version(
            reg_dir.path(),
            "flow",
            "wal",
            "0.2.0",
            manifest_v2,
            &[
                ("spec/flows/wal/A.md", "v2\n"),
                ("boot/10-flow-wal.md", "v1 boot\n"),
            ],
        );
        let new_cached = cached_at(reg_dir.path(), cache_dir.path(), "flow", "wal", "0.2.0");

        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        let err = plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap_err();
        match err {
            InstallError::DependencyShapeChanged {
                package,
                from_version,
                to_version,
                reason,
            } => {
                assert_eq!(package, "flow:wal");
                assert_eq!(from_version, "0.1.0");
                assert_eq!(to_version, "0.2.0");
                assert!(reason.contains("+flow:atomic-commits"));
            }
            other => panic!("expected DependencyShapeChanged, got: {other:?}"),
        }
    }

    #[test]
    fn plan_update_refuses_unknown_package() {
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let _ = install_v1(&reg_dir, &cache_dir, &project);
        let lockfile = Lockfile::empty("vibe-test", "t0");
        // Lockfile is fresh — flow:wal is "not installed" from this lockfile's perspective.
        let new_cached = fetch_v2(&reg_dir, &cache_dir);
        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        let err =
            plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap_err();
        assert!(matches!(err, InstallError::NotInstalled { .. }));
    }

    #[test]
    fn apply_update_writes_added_modified_and_removes_removed() {
        let reg_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        let project = empty_project();
        let mut lockfile = install_v1(&reg_dir, &cache_dir, &project);

        // Pre-conditions on the project tree.
        assert!(project.path().join("spec/flows/wal/A.md").exists());
        assert!(project.path().join("spec/flows/wal/B.md").exists());
        assert!(!project.path().join("spec/flows/wal/C.md").exists());

        let new_cached = fetch_v2(&reg_dir, &cache_dir);
        let old_cache_dir = cache_dir.path().join("flow/wal/v0.1.0");
        let plan = plan_update(project.path(), &lockfile, new_cached, &old_cache_dir).unwrap();
        let written = apply_update(&plan).unwrap();

        // B is gone; A overwritten with v2 bytes; C now exists.
        assert!(!project.path().join("spec/flows/wal/B.md").exists());
        assert!(project.path().join("spec/flows/wal/C.md").exists());
        assert_eq!(
            fs::read_to_string(project.path().join("spec/flows/wal/A.md")).unwrap(),
            "v2 A — changed!\n"
        );
        // boot snippet untouched (Identical).
        assert_eq!(
            fs::read_to_string(project.path().join("spec/boot/10-flow-wal.md")).unwrap(),
            "v1 boot\n"
        );
        // files_written records the new shape (sorted).
        assert!(written.contains(&PathBuf::from("spec/flows/wal/A.md")));
        assert!(written.contains(&PathBuf::from("spec/flows/wal/C.md")));
        assert!(written.contains(&PathBuf::from("spec/boot/10-flow-wal.md")));
        assert!(!written.contains(&PathBuf::from("spec/flows/wal/B.md")));

        // register_updated bumps the lockfile entry.
        register_updated(&mut lockfile, &plan, written, "t1".into()).unwrap();
        let entry = lockfile.find(PackageKind::Flow, "wal").unwrap();
        assert_eq!(entry.version.to_string(), "0.2.0");
        assert!(entry.content_hash.starts_with("sha256:"));
        // content_hash MUST have changed — the v2 payload is byte-different from v1.
        assert_ne!(entry.content_hash, "sha256:placeholder");
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
            source_kind: Some(vibe_core::manifest::SourceKind::Registry),
            via_redirect: None,
            features: Vec::new(),
            subskills_active: Vec::new(),
            describes: None,
            language: None,
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
