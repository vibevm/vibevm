//! The index side of a batch (PROP-002 `#registry-index`): which journal
//! facts and `_index/` files it adds, and the refresh that writes them.
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use vibe_core::layout_registry::IndexState;
use vibe_index::journal::Event;
use vibe_index::layout::LayoutIdentity;
use vibe_index::scanner::tree_entry::{EntrySource, entry_from_tree};

use super::{BatchConfig, LayoutAction, Staged, io};
use crate::PublishError;

/// What the registry index needs from this batch: the journal facts to
/// append and whether the index must be rewritten at all.
pub(super) struct IndexPlan {
    pub(super) facts: Vec<Event>,
    /// Batch files whose bytes differ from the tree's.
    pub(super) files: Vec<(String, PathBuf)>,
    pub(super) refresh: bool,
}

/// Plan the index change for the tree at `root` (before any version of
/// this batch is written, so `Published` entries are built from the staged
/// payloads).
pub(super) fn plan_index(
    root: &Path,
    config: &BatchConfig,
    identity: &LayoutIdentity,
    staged: &[Staged],
    actions: &[LayoutAction],
    at: DateTime<Utc>,
) -> Result<IndexPlan, PublishError> {
    let index_error = |message: String| PublishError::LayoutIndex {
        path: vibe_core::layout_registry::index_dir(root),
        message,
    };
    let (current, state) = vibe_index::layout::load_or_project(root, identity)
        .map_err(|e| index_error(e.to_string()))?;
    let mut facts = Vec::new();
    for (stage, action) in staged.iter().zip(actions) {
        if *action == LayoutAction::Unchanged {
            continue;
        }
        let (source_url, source_ref, resolved_commit) = match &stage.item.provenance {
            Some(p) => (
                p.source_url.clone(),
                p.source_ref.clone(),
                p.resolved_commit.clone(),
            ),
            None => (
                format!("{}#{}", config.registry.url, stage.path()),
                identity
                    .branch
                    .clone()
                    .unwrap_or_else(|| format!("v{}", stage.item.version)),
                None,
            ),
        };
        let entry = entry_from_tree(
            &stage.payload,
            EntrySource {
                source_url,
                source_ref,
                resolved_commit,
                registry: config.registry.name.clone(),
                indexed_at: at,
                indexed_by: format!("vibe {}", env!("CARGO_PKG_VERSION")),
            },
        )
        .map_err(|e| index_error(format!("{}: {e}", stage.path())))?;
        facts.push(Event::Published {
            entry: Box::new(entry),
        });
    }
    let publishing = |group: &vibe_core::Group, name: &str, version: &semver::Version| {
        staged.iter().any(|s| {
            s.item.group == group.as_str() && s.item.name == name && &s.item.version == version
        })
    };
    for fact in &config.facts {
        let new = match fact {
            Event::Yanked {
                group,
                name,
                version,
                ..
            } => match current
                .get(group, name)
                .and_then(|pkg| pkg.versions.iter().find(|v| &v.version == version))
            {
                Some(entry) => !entry.yanked,
                None => publishing(group, name, version),
            },
            Event::Buried { name, .. } => !current.tombstones.contains_key(name),
            _ => true,
        };
        if new {
            facts.push(fact.clone());
        }
    }
    let mut files = Vec::new();
    for (relative, source) in &config.files {
        if !relative.starts_with(&format!("{}/", vibe_core::layout_registry::INDEX_DIR))
            || relative
                .split('/')
                .any(|part| part == ".." || part.is_empty())
        {
            return Err(index_error(format!(
                "batch file `{relative}` is not a path inside `{}/`",
                vibe_core::layout_registry::INDEX_DIR
            )));
        }
        let wanted = std::fs::read(source).map_err(|e| io(source, "read", e))?;
        if std::fs::read(root.join(relative)).ok().as_deref() != Some(wanted.as_slice()) {
            files.push((relative.clone(), source.clone()));
        }
    }
    Ok(IndexPlan {
        refresh: !facts.is_empty() || !files.is_empty() || state != IndexState::Current,
        facts,
        files,
    })
}

/// Append the planned facts and rewrite the index of the tree at `root`.
pub(super) fn write_index(
    root: &Path,
    identity: &LayoutIdentity,
    plan: IndexPlan,
    at: DateTime<Utc>,
) -> Result<(), PublishError> {
    for (relative, source) in &plan.files {
        let target = root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io(parent, "create_dir_all", e))?;
        }
        std::fs::copy(source, &target).map_err(|e| io(&target, "copy", e))?;
    }
    vibe_index::layout::refresh(root, identity, plan.facts, at)
        .map(|_| ())
        .map_err(|e| PublishError::LayoutIndex {
            path: vibe_core::layout_registry::index_dir(root),
            message: e.to_string(),
        })
}
