//! Prepared original authored inputs shared by admission and generation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);
use super::*;
use std::path::PathBuf;
use vibe_spec::source_policy::{CapturedAuthoredBoot, PackagePolicyObservation};
#[derive(Debug)]
pub(crate) struct PreparedNodeBoot {
    pub(crate) entries: Vec<AuthoredBoot>,
    root: PathBuf,
    relative: String,
    policy: PackagePolicyObservation,
    captured: Option<CapturedAuthoredBoot>,
}
impl PreparedNodeBoot {
    pub(crate) fn prepare(
        node_dir: &Path,
        node_rel: &str,
        expected: &Manifest,
    ) -> Result<Self, WorkspaceError> {
        let policy = vibe_spec::source_policy::observe_package_policy(node_dir)
            .map_err(|e| error(node_dir, e))?;
        if policy.manifest() != Some(expected) {
            return Err(error(
                node_dir,
                "authored node manifest differs from admitted workspace",
            ));
        }
        let captured = policy
            .source_rows()
            .map(|_| policy.capture_authored_boot())
            .transpose()
            .map_err(|e| error(node_dir, e))?;
        let entries = match &captured {
            Some(captured) => {
                let mut entries = Vec::new();
                for resource in captured.resources() {
                    vibe_spec::project_declared_native(
                        resource.bytes(),
                        resource.media(),
                        vibe_core::manifest::SourceInterpretation::VibeSpec,
                    )
                    .map_err(|e| error(node_dir, e))?;
                    let name = Path::new(resource.relative())
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| error(node_dir, "invalid boot filename"))?;
                    entries.push(AuthoredBoot {
                        path: if node_rel == "." {
                            resource.relative().into()
                        } else {
                            format!("{node_rel}/{}", resource.relative())
                        },
                        category: match name {
                            "00-core.md" => Some(BootCategory::Foundation),
                            "90-user.md" => Some(BootCategory::UserOverride),
                            _ => None,
                        },
                        origin: node_rel.into(),
                    });
                }
                entries
            }
            None => legacy_entries(node_dir, node_rel)?,
        };
        let result = Self {
            entries,
            root: node_dir.into(),
            relative: node_rel.into(),
            policy,
            captured,
        };
        result.recheck()?;
        Ok(result)
    }
    pub(crate) fn recheck(&self) -> Result<(), WorkspaceError> {
        match &self.captured {
            Some(captured) => self
                .policy
                .recheck_authored_boot(captured)
                .map_err(|e| error(&self.root, e)),
            None => {
                self.policy
                    .recheck_unchanged()
                    .map_err(|e| error(&self.root, e))?;
                Ok(())
            }
        }
    }
    pub(crate) fn simple(
        &self,
        entry: &crate::boot::BootEntry,
        overlay: Option<&[u8]>,
    ) -> Result<Option<String>, WorkspaceError> {
        let Some(captured) = &self.captured else {
            return Ok(None);
        };
        let original = if self.relative == "." {
            entry.path.as_str()
        } else {
            entry
                .path
                .strip_prefix(&format!("{}/", self.relative))
                .unwrap_or("")
        };
        let Some(resource) = captured
            .resources()
            .iter()
            .find(|resource| resource.relative() == original)
        else {
            return Ok(None);
        };
        self.recheck()?;
        if overlay.is_some_and(|bytes| bytes != resource.bytes()) {
            return Err(error(
                &self.root,
                "node overlay differs from captured original",
            ));
        }
        let projected = vibe_spec::project_declared_native(
            resource.bytes(),
            resource.media(),
            vibe_core::manifest::SourceInterpretation::VibeSpec,
        )
        .map_err(|e| error(&self.root, e))?;
        self.recheck()?;
        Ok(Some(projected))
    }
}
fn error(path: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.into(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW: {reason}"
        ),
    }
}
// Path-only publishing callers have no retained node input at publication.
// Keep their existing declared-policy refusal until they supply that carrier.
pub(super) fn unbound_entries(
    node_dir: &Path,
    node_rel: &str,
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let policy = vibe_spec::source_policy::observe_package_policy(node_dir)
        .map_err(|e| error(node_dir, e))?;
    let entries = match policy.source_rows() {
        Some(rows) if !rows.is_empty() => {
            return Err(error(
                node_dir,
                policy.unsupported("unbound node-authored boot generation"),
            ));
        }
        Some(_) => Vec::new(),
        None => legacy_entries(node_dir, node_rel)?,
    };
    policy.recheck_unchanged().map_err(|e| error(node_dir, e))?;
    Ok(entries)
}
fn legacy_entries(node_dir: &Path, node_rel: &str) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let boot_dir = node_dir.join(layout::current_boot_dir());
    if !boot_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let mut spec_paths = Vec::new();
    for entry in fs::read_dir(&boot_dir).map_err(|e| io_err(&boot_dir, e))? {
        let entry = entry.map_err(|e| io_err(&boot_dir, e))?;
        let path = entry.path();
        if !entry.file_type().map_err(|e| io_err(&path, e))?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !vibe_specdoc::is_spec_source(&path) {
            continue;
        }
        // The generated artifacts are not authored boot.
        if name == boot_artifacts::static_file(SpecFormat::Mixed)
            || name == boot_artifacts::static_file(SpecFormat::Xml)
            || name == boot_artifacts::INDEX_FILE
        {
            continue;
        }
        let category = match name.as_str() {
            "00-core.md" => Some(BootCategory::Foundation),
            "90-user.md" => Some(BootCategory::UserOverride),
            _ => None,
        };
        let rel_path = if node_rel == "." {
            layout_paths::boot(&name)
        } else {
            path_to_slash(
                &Path::new(node_rel)
                    .join(layout::current_boot_dir())
                    .join(&name),
            )
        };
        spec_paths.push(path.clone());
        files.push(AuthoredBoot {
            path: rel_path,
            category,
            origin: node_rel.to_string(),
        });
    }
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&spec_paths).first() {
        let rel = collision
            .markdown
            .strip_prefix(node_dir)
            .unwrap_or(&collision.markdown)
            .to_path_buf();
        return Err(WorkspaceError::Io {
            path: rel,
            reason: collision.message(),
        });
    }
    // Deterministic order — the engine keeps a band's collection order.
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
