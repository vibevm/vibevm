//! Publishing into a directory-layout registry (PROP-002 `#registry-type`):
//! a `monorepo` (one Git repository, pushed with plain `git`, no host API),
//! a `localdir` (a directory written straight on the filesystem) or a Git
//! working copy of a monorepo (committed into, never pushed). All of them
//! place a package at `<group>/<name>/v<version>/`.
//!
//! Publication is one library operation over a batch of
//! `(group, name, version, file tree, provenance)` items
//! ([`publish_batch`]): `vibe registry publish` sends one item read from a
//! package directory ([`publish`]), `vibe registry mirror` sends every
//! version it copies. Each batch also refreshes the registry's own index
//! (`_index/`, PROP-002 `#registry-index`) through
//! [`vibe_index::layout::refresh`] — in a monorepo inside the same commit as
//! the versions, so one push carries both.
//!
//! A version directory is immutable by default: re-publishing identical
//! content is a no-op, different content is refused unless the item allows
//! `replace`. Identity is the package's content hash, so build output and
//! `.git` never make two copies differ.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type");
use std::path::{Path, PathBuf};
use vibe_core::PackageKind;
use vibe_core::cancellation::CancellationToken;
use vibe_core::manifest::Manifest;
use vibe_index::journal::Event;

use crate::PublishError;
use index_plan::{IndexPlan, plan_index, write_index};
mod git_tree;
mod index_plan;
mod localdir;
mod monorepo;
mod working_copy;

/// Where a layout publish writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutTarget {
    /// One Git repository; packages on `branch`.
    Monorepo { url: String, branch: String },
    /// A directory (possibly a network share).
    Localdir { root: PathBuf },
    /// A Git working copy of a monorepo: the batch is committed onto its
    /// checked-out branch and never pushed. A tree with uncommitted changes
    /// is refused.
    WorkingCopy { path: PathBuf },
}

/// The registry's identity as the publisher configured it; the registry's
/// journal keeps the identity of its first publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutRegistry {
    /// `[[registry]].name`.
    pub name: String,
    /// `[[registry]].url` — what an entry projected from the tree points at
    /// (`<url>#<group>/<name>/v<version>`).
    pub url: String,
}

/// Where a version's bytes came from, when not from this registry itself
/// (a mirrored version): recorded as the entry's `source_url`,
/// `source_ref` and `resolved_commit` in the registry's journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub source_url: String,
    pub source_ref: String,
    pub resolved_commit: Option<String>,
}

/// A Git tree object a [`PublishItem`] is taken from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTree {
    /// A repository (bare or not) holding the tree's objects.
    pub repo: PathBuf,
    /// A ref in `repo` whose history contains the tree.
    pub reference: String,
    /// The tree object id.
    pub tree: String,
}

/// One version to publish.
#[derive(Debug, Clone)]
pub struct PublishItem {
    pub group: String,
    pub name: String,
    pub version: semver::Version,
    /// The package's file tree (`.git` is never copied).
    pub tree: PathBuf,
    pub provenance: Option<Provenance>,
    /// The version is a Git tree (a mirrored tag): it is inserted by tree
    /// object, so every file, mode and symlink is kept and the version
    /// directory's tree hash equals the source commit's tree hash — checked
    /// before anything is committed (PROP-002 `#registry-mirror`). `tree`
    /// is then the same tree checked out, used for hashing.
    pub git: Option<GitTree>,
    /// Overwrite an existing version directory whose content differs.
    pub replace: bool,
}

/// Inputs to one batch.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub target: LayoutTarget,
    pub registry: LayoutRegistry,
    pub items: Vec<PublishItem>,
    /// Further journal facts to record (a mirror's yanks and tombstones).
    /// Facts the registry already reflects are dropped, so a rerun records
    /// nothing twice.
    pub facts: Vec<Event>,
    /// Files written under `_index/` with the batch (registry-relative
    /// path, source file) — a mirror's record and history bundles. Only
    /// those whose bytes differ are written.
    pub files: Vec<(String, PathBuf)>,
    /// The commit message (monorepo, working copy); a default names the
    /// published versions.
    pub message: Option<String>,
    /// Decide and report, write nothing.
    pub dry_run: bool,
}

/// Inputs to one single-package publish (`vibe registry publish`).
#[derive(Debug, Clone)]
pub struct LayoutPublishConfig {
    pub source_dir: PathBuf,
    pub target: LayoutTarget,
    pub registry: LayoutRegistry,
    /// Overwrite an existing version directory whose content differs.
    pub replace: bool,
    /// Decide and report, write nothing.
    pub dry_run: bool,
}

/// What happened to the version directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAction {
    /// The version directory did not exist and was written.
    Created,
    /// It existed with different content and `replace` overwrote it.
    Replaced,
    /// It already held identical content; nothing was written.
    Unchanged,
}

impl LayoutAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            LayoutAction::Created => "created",
            LayoutAction::Replaced => "replaced",
            LayoutAction::Unchanged => "unchanged",
        }
    }
}

/// The outcome of one item.
#[derive(Debug, Clone)]
pub struct ItemOutcome {
    pub group: String,
    pub name: String,
    pub version: semver::Version,
    /// `<group>/<name>/v<version>` inside the registry.
    pub package_path: String,
    pub action: LayoutAction,
    /// `sha256:` content hash of the published tree.
    pub content_hash: String,
    /// The Git tree hash of the written version directory, when the item
    /// came as a [`GitTree`] and was written: verified equal to it.
    pub tree: Option<String>,
}

/// The outcome of a batch.
#[derive(Debug, Clone)]
pub struct BatchOutcome {
    pub items: Vec<ItemOutcome>,
    /// Journal facts recorded beside the versions (after dropping those
    /// the registry already reflected).
    pub facts_recorded: usize,
    /// The registry index was rewritten (or, on a dry run, would be).
    pub index_refreshed: bool,
    /// The commit made (monorepo: pushed; working copy: local).
    pub commit: Option<String>,
    /// A non-fast-forward rejection was absorbed by one refetch.
    pub retried: bool,
    pub dry_run: bool,
}

impl BatchOutcome {
    /// Whether the batch changed (or would change) anything.
    pub fn changed(&self) -> bool {
        self.index_refreshed
            || self.facts_recorded > 0
            || self
                .items
                .iter()
                .any(|item| item.action != LayoutAction::Unchanged)
    }
}

/// The outcome the CLI renders for a single-package publish.
#[derive(Debug, Clone)]
pub struct LayoutPublishOutcome {
    pub kind: PackageKind,
    pub group: String,
    pub name: String,
    pub version: semver::Version,
    /// `<group>/<name>/v<version>` inside the registry.
    pub package_path: String,
    pub action: LayoutAction,
    /// The pushed commit (monorepo, when something was pushed).
    pub commit: Option<String>,
    /// A non-fast-forward rejection was absorbed by one refetch.
    pub retried: bool,
    /// `sha256:` content hash of the published tree.
    pub content_hash: String,
    pub dry_run: bool,
}

/// Publish `config.source_dir` into its layout registry.
///
/// ```no_run
/// use std::path::PathBuf;
/// use vibe_publish::layout_publish::{
///     LayoutPublishConfig, LayoutRegistry, LayoutTarget, publish,
/// };
///
/// let outcome = publish(
///     &LayoutPublishConfig {
///         source_dir: PathBuf::from("my-package"),
///         target: LayoutTarget::Localdir { root: PathBuf::from("/srv/vibe") },
///         registry: LayoutRegistry { name: "corp".into(), url: "file:///srv/vibe".into() },
///         replace: false,
///         dry_run: true,
///     },
///     &Default::default(),
/// )
/// .unwrap();
/// println!("{} {}", outcome.package_path, outcome.action.as_str());
/// ```
pub fn publish(
    config: &LayoutPublishConfig,
    cancellation: &CancellationToken,
) -> Result<LayoutPublishOutcome, PublishError> {
    let package = Package::read(&config.source_dir)?;
    let batch = publish_batch(
        &BatchConfig {
            target: config.target.clone(),
            registry: config.registry.clone(),
            items: vec![PublishItem {
                group: package.group.clone(),
                name: package.name.clone(),
                version: package.version.clone(),
                tree: config.source_dir.clone(),
                provenance: None,
                git: None,
                replace: config.replace,
            }],
            facts: Vec::new(),
            files: Vec::new(),
            message: None,
            dry_run: config.dry_run,
        },
        cancellation,
    )?;
    let item = batch
        .items
        .into_iter()
        .next()
        .ok_or_else(|| PublishError::Git("the publish batch reported no item".into()))?;
    Ok(LayoutPublishOutcome {
        kind: package.kind,
        group: item.group,
        name: item.name,
        version: item.version,
        package_path: item.package_path,
        action: item.action,
        commit: batch.commit,
        retried: batch.retried,
        content_hash: item.content_hash,
        dry_run: batch.dry_run,
    })
}

/// Publish a batch of versions (and journal facts) into one layout
/// registry: one commit and one push for a monorepo, one index refresh for
/// a localdir.
pub fn publish_batch(
    config: &BatchConfig,
    cancellation: &CancellationToken,
) -> Result<BatchOutcome, PublishError> {
    // Stage every payload once: the content hash compared against an
    // existing version directory and the bytes written are the same tree.
    let staging = tempfile::TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating staging dir: {e}"),
    })?;
    let mut staged = Vec::with_capacity(config.items.len());
    for (index, item) in config.items.iter().enumerate() {
        checkpoint(cancellation)?;
        // A Git-tree item is already a private checkout of its tree.
        let payload = if item.git.is_some() {
            item.tree.clone()
        } else {
            let payload = staging.path().join(format!("payload-{index}"));
            crate::git_publish::copy_dir_controlled(&item.tree, &payload, cancellation)?;
            payload
        };
        staged.push(Staged {
            content_hash: content_hash(&payload)?,
            payload,
            item: item.clone(),
        });
    }
    match &config.target {
        LayoutTarget::Localdir { root } => localdir::publish(config, root, &staged, cancellation),
        LayoutTarget::Monorepo { url, branch } => {
            monorepo::publish(config, url, branch, &staged, cancellation)
        }
        LayoutTarget::WorkingCopy { path } => {
            working_copy::publish(config, path, &staged, cancellation)
        }
    }
}

/// A layout registry's current tree, for a caller that plans a batch
/// against it (`vibe registry mirror` deciding what is new). A monorepo is
/// fetched (its branch, shallow) into a temporary directory held here.
#[derive(Debug)]
pub struct TargetTree {
    root: PathBuf,
    _fetched: Option<tempfile::TempDir>,
}

impl TargetTree {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Read the tree a batch into `target` would start from.
pub fn read_target(target: &LayoutTarget) -> Result<TargetTree, PublishError> {
    match target {
        LayoutTarget::Localdir { root } => Ok(TargetTree {
            root: root.clone(),
            _fetched: None,
        }),
        LayoutTarget::WorkingCopy { path } => Ok(TargetTree {
            // Refused before any planning: the batch would refuse it anyway.
            root: working_copy::clean_toplevel(path)?,
            _fetched: None,
        }),
        LayoutTarget::Monorepo { url, branch } => {
            let work = tempfile::TempDir::new().map_err(|e| PublishError::Io {
                path: std::env::temp_dir(),
                message: format!("creating a working tree: {e}"),
            })?;
            crate::git_publish::run_git_in(
                work.path(),
                &["init", "-q", &format!("--initial-branch={branch}")],
            )?;
            monorepo::fetch(work.path(), url, branch)?;
            Ok(TargetTree {
                root: work.path().to_path_buf(),
                _fetched: Some(work),
            })
        }
    }
}

struct Package {
    kind: PackageKind,
    group: String,
    name: String,
    version: semver::Version,
}

impl Package {
    fn read(source_dir: &Path) -> Result<Self, PublishError> {
        let invalid = |reason: String| PublishError::SourceInvalid {
            path: source_dir.to_path_buf(),
            reason,
        };
        let manifest = Manifest::read(source_dir.join(Manifest::FILENAME))
            .map_err(|e| invalid(format!("cannot read vibe.toml: {e}")))?;
        let meta = manifest
            .package
            .ok_or_else(|| invalid("vibe.toml has no [package] table".into()))?;
        Ok(Package {
            kind: meta.kind,
            group: meta.group.as_str().to_string(),
            name: meta.name.to_string(),
            version: meta.version,
        })
    }
}

struct Staged {
    item: PublishItem,
    payload: PathBuf,
    content_hash: String,
}

impl Staged {
    fn path(&self) -> String {
        format!(
            "{}/{}/v{}",
            self.item.group, self.item.name, self.item.version
        )
    }

    fn outcome(&self, action: LayoutAction) -> ItemOutcome {
        ItemOutcome {
            group: self.item.group.clone(),
            name: self.item.name.clone(),
            version: self.item.version.clone(),
            package_path: self.path(),
            action,
            content_hash: self.content_hash.clone(),
            tree: match (&self.item.git, action) {
                (Some(git), LayoutAction::Created | LayoutAction::Replaced) => {
                    Some(git.tree.clone())
                }
                _ => None,
            },
        }
    }

    /// Compare against an existing version directory: `Created` when
    /// absent, `Unchanged` when identical, `Replaced` when different and
    /// the item allows it, the refusal otherwise.
    fn decide(&self, existing: &Path, location: &str) -> Result<LayoutAction, PublishError> {
        if !existing.exists() {
            return Ok(LayoutAction::Created);
        }
        if content_hash(existing)? == self.content_hash {
            return Ok(LayoutAction::Unchanged);
        }
        if self.item.replace {
            return Ok(LayoutAction::Replaced);
        }
        Err(PublishError::VersionExists {
            package: format!(
                "{}/{}@{}",
                self.item.group, self.item.name, self.item.version
            ),
            location: location.to_string(),
        })
    }
}

/// The decisions for every staged item against the tree at `root`.
fn decide_all(
    root: &Path,
    staged: &[Staged],
    location: impl Fn(&str) -> String,
) -> Result<Vec<LayoutAction>, PublishError> {
    staged
        .iter()
        .map(|s| s.decide(&root.join(s.path()), &location(&s.path())))
        .collect()
}

/// The commit message of a batch.
fn commit_message(config: &BatchConfig, staged: &[Staged], actions: &[LayoutAction]) -> String {
    if let Some(message) = &config.message {
        return message.clone();
    }
    let changed: Vec<(&Staged, &LayoutAction)> = staged
        .iter()
        .zip(actions)
        .filter(|(_, action)| **action != LayoutAction::Unchanged)
        .collect();
    match changed.as_slice() {
        [] => "index: refresh".to_string(),
        [(stage, action)] => format!(
            "{}: {}/{}@{}",
            if **action == LayoutAction::Replaced {
                "replace"
            } else {
                "publish"
            },
            stage.item.group,
            stage.item.name,
            stage.item.version
        ),
        many => format!("publish: {} versions", many.len()),
    }
}

fn outcomes(staged: &[Staged], actions: &[LayoutAction]) -> Vec<ItemOutcome> {
    staged
        .iter()
        .zip(actions)
        .map(|(stage, action)| stage.outcome(*action))
        .collect()
}

fn content_hash(dir: &Path) -> Result<String, PublishError> {
    vibe_registry::compute_content_hash(dir).map_err(|e| PublishError::Io {
        path: dir.to_path_buf(),
        message: format!("hashing package tree: {e}"),
    })
}

fn checkpoint(cancellation: &CancellationToken) -> Result<(), PublishError> {
    cancellation
        .checkpoint()
        .map_err(|error| PublishError::Git(error.to_string()))
}

fn io(path: &Path, what: &str, error: std::io::Error) -> PublishError {
    PublishError::Io {
        path: path.to_path_buf(),
        message: format!("{what}: {error}"),
    }
}

#[cfg(test)]
#[path = "layout_publish/tests.rs"]
mod tests;
