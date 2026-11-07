//! Host-checked installed data; the workspace remains the sole slot-record decoder.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW");
use super::*;
use crate::source_policy::{PackagePolicyCheckpoint, PackageSourceMode, observe_package_policy};
use std::sync::Arc;
use vibe_core::manifest::{SourceInterpretation, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_safefs::{Pinned, Project};

/// Independent selection supplied by the host, never inferred from a slot record.
#[derive(Debug, Clone)]
pub struct InstalledSourceExpectation {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: String,
    pub source_hash: ContentHash,
    pub materialized_format: SpecFormat,
}

/// One checked original/output relationship. Raw resources have no native address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedOutputRow {
    pub source_path: String,
    pub output_path: String,
    pub interpretation: Option<SourceInterpretation>,
    pub media: String,
    pub sha256: String,
    pub native_address: Option<String>,
}

#[derive(Debug)]
struct PreparedData {
    slot_relative: String,
    expectation: InstalledSourceExpectation,
    policy: PackagePolicyCheckpoint,
    record_relative: String,
    record_bytes: Arc<[u8]>,
    rows: Vec<PreparedOutputRow>,
}

/// Finite host-prepared lookup data, not authentication or a filesystem grant.
#[derive(Debug, Clone)]
pub struct PreparedInstalledSource(Arc<PreparedData>);

impl PreparedInstalledSource {
    pub fn from_host_checked(
        slot_relative: String,
        expectation: InstalledSourceExpectation,
        policy: PackagePolicyCheckpoint,
        record_relative: String,
        record_bytes: Arc<[u8]>,
        rows: Vec<PreparedOutputRow>,
    ) -> Result<Self, ResolveError> {
        validate::prepared(
            &slot_relative,
            &expectation,
            &policy,
            &record_relative,
            &rows,
        )?;
        Ok(Self(Arc::new(PreparedData {
            slot_relative,
            expectation,
            policy,
            record_relative,
            record_bytes,
            rows,
        })))
    }
    pub fn expectation(&self) -> &InstalledSourceExpectation {
        &self.0.expectation
    }
    pub fn slot_relative(&self) -> &str {
        &self.0.slot_relative
    }
    pub fn outputs(&self) -> &[PreparedOutputRow] {
        &self.0.rows
    }
    /// Captured manifest metadata; callers still recheck through the bound reader.
    pub fn manifest(&self) -> Option<&vibe_core::manifest::Manifest> {
        self.0.policy.manifest()
    }
    pub fn output_for_source(&self, source: &str) -> Option<&PreparedOutputRow> {
        self.0.rows.iter().find(|row| row.source_path == source)
    }
    fn slot(&self, project: &Project) -> Result<Pinned, ResolveError> {
        project
            .dir(&self.0.slot_relative.split('/').collect::<Vec<_>>(), false)
            .map_err(|error| fail(&self.0.slot_relative, error))
    }
    fn recheck(&self, project: &Project) -> Result<Pinned, ResolveError> {
        let slot = self.slot(project)?;
        self.0
            .policy
            .recheck_in(project, &slot)
            .map_err(|error| fail(&self.0.slot_relative, error))?;
        let record = project
            .read_source_bytes_in(&slot, &self.0.record_relative)
            .map_err(|error| fail(&self.0.slot_relative, error))?;
        if record.as_deref() != Some(self.0.record_bytes.as_ref()) {
            return Err(fail(&self.0.slot_relative, "captured slot record changed"));
        }
        self.0
            .policy
            .recheck_in(project, &slot)
            .map_err(|error| fail(&self.0.slot_relative, error))?;
        Ok(slot)
    }
}

#[derive(Debug)]
pub(super) struct BoundInstalledSources {
    project: Arc<Project>,
    workspace_policy: PackagePolicyCheckpoint,
    bindings: BTreeMap<(String, String), PreparedInstalledSource>,
    installed_self: Option<(String, String)>,
}

impl BoundInstalledSources {
    fn workspace(&self) -> Result<(), ResolveError> {
        let root = self
            .project
            .root_dir()
            .map_err(|error| fail("workspace", error))?;
        self.workspace_policy
            .recheck_in(&self.project, &root)
            .map_err(|error| fail("workspace", error))
    }
    fn binding(&self, group: &str, name: &str) -> Result<&PreparedInstalledSource, ResolveError> {
        self.bindings
            .get(&(group.into(), name.into()))
            .ok_or_else(|| {
                fail(
                    &format!("{group}/{name}"),
                    "missing installed source binding",
                )
            })
    }
    pub(super) fn recheck_all(&self) -> Result<(), ResolveError> {
        self.workspace()?;
        for binding in self.bindings.values() {
            binding.recheck(&self.project)?;
        }
        Ok(())
    }
}

impl FileResolver {
    /// Attach data to an existing complete selected world, preserving legacy peers.
    pub fn with_installed_source_bindings(
        mut self,
        workspace: Arc<Project>,
        bindings: Vec<PreparedInstalledSource>,
        installed_self: Option<(String, String)>,
    ) -> Result<Self, ResolveError> {
        let workspace_policy = observe_package_policy(&self.ws_root)
            .map_err(|e| fail("workspace", e))?
            .into_checkpoint();
        workspace_policy
            .recheck_in(
                &workspace,
                &workspace.root_dir().map_err(|e| fail("workspace", e))?,
            )
            .map_err(|e| fail("workspace", e))?;
        let selected = self.selected.as_ref().ok_or_else(|| {
            fail(
                "workspace",
                "installed bindings require the complete independently selected world",
            )
        })?;
        let mut by_coordinate = BTreeMap::new();
        for binding in bindings {
            let e = binding.expectation();
            let key = (e.group.to_string(), e.name.clone());
            let chosen = selected.get(&key).ok_or_else(|| {
                fail(
                    &binding.0.slot_relative,
                    "binding is absent from selected world",
                )
            })?;
            let physical = workspace.root_path().join(&binding.0.slot_relative);
            let logical = self.ws_root.join(&binding.0.slot_relative);
            if chosen.version != e.version || (chosen.root != physical && chosen.root != logical) {
                return Err(fail(
                    &binding.0.slot_relative,
                    "binding differs from selected package root/version",
                ));
            }
            binding.recheck(&workspace)?;
            if by_coordinate.insert(key, binding).is_some() {
                return Err(fail("workspace", "duplicate installed binding"));
            }
        }
        if let Some((group, name)) = &installed_self {
            if !self.is_self(group, name) {
                return Err(fail(
                    "workspace",
                    "installed self differs from self coordinate",
                ));
            }
            let binding = by_coordinate
                .get(&(group.clone(), name.clone()))
                .ok_or_else(|| fail("workspace", "installed self has no binding"))?;
            if self.self_root != workspace.root_path().join(binding.slot_relative())
                && self.self_root != self.ws_root.join(binding.slot_relative())
            {
                return Err(fail(
                    "workspace",
                    "installed self root differs from selected slot",
                ));
            }
        }
        self.installed_sources = Some(Arc::new(BoundInstalledSources {
            project: workspace,
            workspace_policy,
            bindings: by_coordinate,
            installed_self,
        }));
        Ok(self)
    }
    /// Borrow the binding attached to this resolver, without claiming a fresh
    /// byte observation. Actual reads still recheck its metadata and digests.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-QUERY-READ",
        r = 1
    )]
    pub fn installed_source_binding(
        &self,
        group: &str,
        name: &str,
    ) -> Result<&PreparedInstalledSource, ResolveError> {
        self.installed_sources
            .as_ref()
            .ok_or_else(|| fail("workspace", "installed bindings unavailable"))?
            .binding(group, name)
    }
    pub fn recheck_installed_package(&self, group: &str, name: &str) -> Result<(), ResolveError> {
        let bound = self
            .installed_sources
            .as_ref()
            .ok_or_else(|| fail("workspace", "installed bindings unavailable"))?;
        bound.workspace()?;
        bound.binding(group, name)?.recheck(&bound.project)?;
        bound.workspace()
    }
    pub fn read_installed_source(
        &self,
        group: &str,
        name: &str,
        original: &str,
        overlay: Option<&[u8]>,
    ) -> Result<Vec<u8>, ResolveError> {
        let bound = self
            .installed_sources
            .as_ref()
            .ok_or_else(|| fail(original, "installed bindings unavailable"))?;
        let binding = bound.binding(group, name)?.clone();
        let row = binding
            .output_for_source(original)
            .ok_or_else(|| fail(original, "original resource is not recorded"))?
            .clone();
        DeclaredRead {
            bound: bound.clone(),
            binding,
            row,
        }
        .read(overlay)
        .map(|bytes| bytes.into_owned())
    }
    pub(super) fn declared_location(
        &self,
        address: &SpecAddress,
        _formats: &[String],
    ) -> Result<Option<super::observation::SourceLocation>, ResolveError> {
        let Some(bound) = &self.installed_sources else {
            return Ok(None);
        };
        let Authority::Package {
            group,
            name,
            version,
        } = &address.authority
        else {
            return Ok(None);
        };
        let Some(binding) = bound.bindings.get(&(group.clone(), name.clone())) else {
            return Ok(None);
        };
        if self.is_self(group, name) {
            if let Some(version) = version {
                return Err(ResolveError::SelfCoordinateVersioned {
                    self_group: group.clone(),
                    self_name: name.clone(),
                    version: version.clone(),
                });
            }
            if bound.installed_self.as_ref() != Some(&(group.clone(), name.clone())) {
                return Err(fail(
                    &address.raw,
                    "installed self was not explicitly bound",
                ));
            }
        } else if let Some(version) = version
            && version.trim_start_matches('=') != binding.expectation().version
        {
            return Err(ResolveError::SelectedVersionMismatch {
                given: format!("{group}/{name}"),
                requested: version.clone(),
                selected: binding.expectation().version.clone(),
            });
        }
        bound.workspace()?;
        let slot = binding.recheck(&bound.project)?;
        let row = binding
            .outputs()
            .iter()
            .find(|row| row.native_address.as_deref() == Some(&address.doc_path))
            .ok_or_else(|| ResolveError::DocNotFound {
                doc_path: address.doc_path.clone(),
                base: slot.path().display().to_string(),
            })?
            .clone();
        // Preserve the exact selected root spelling as the lexical overlay key.
        let path = if self.is_self(group, name) {
            self.self_root.clone()
        } else {
            self.selected
                .as_ref()
                .and_then(|selected| selected.get(&(group.clone(), name.clone())))
                .ok_or_else(|| fail(&address.raw, "declared coordinate lost its selected root"))?
                .root
                .clone()
        }
        .join(&row.output_path);
        let physical_stem = Path::new(&row.output_path)
            .file_stem()
            .and_then(|p| p.to_str())
            .ok_or_else(|| fail(&row.output_path, "invalid output stem"))?
            .to_owned();
        let guard = DeclaredRead {
            bound: bound.clone(),
            binding: binding.clone(),
            row,
        };
        guard.check_present()?;
        Ok(Some(super::observation::SourceLocation::declared(
            ResolvedSourceFile {
                path,
                format: "markdown".into(),
                physical_stem,
            },
            guard,
        )))
    }
}

pub(super) struct DeclaredRead {
    bound: Arc<BoundInstalledSources>,
    binding: PreparedInstalledSource,
    row: PreparedOutputRow,
}

impl DeclaredRead {
    pub(super) fn media(&self) -> (SourceInterpretation, &str) {
        (
            self.row.interpretation.expect("native row validated"),
            &self.row.media,
        )
    }
    fn check_present(&self) -> Result<(), ResolveError> {
        self.bound.workspace()?;
        let slot = self.binding.recheck(&self.bound.project)?;
        let mut parts = self.row.output_path.split('/').peekable();
        let mut parent = slot;
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                parent = parent
                    .open_child(part)
                    .map_err(|e| fail(&self.row.output_path, e))?;
            } else if !self
                .bound
                .project
                .source_file_present_in(&parent, part)
                .map_err(|e| fail(&self.row.output_path, e))?
            {
                return Err(fail(
                    &self.row.output_path,
                    "recorded native output is absent",
                ));
            }
        }
        self.binding.recheck(&self.bound.project)?;
        self.bound.workspace()
    }
    pub(super) fn read<'a>(
        &self,
        overlay: Option<&'a [u8]>,
    ) -> Result<std::borrow::Cow<'a, [u8]>, ResolveError> {
        use sha2::{Digest, Sha256};
        self.bound.workspace()?;
        let slot = self.binding.recheck(&self.bound.project)?;
        let bytes = match overlay {
            Some(bytes) => std::borrow::Cow::Borrowed(bytes),
            None => std::borrow::Cow::Owned(
                self.bound
                    .project
                    .read_source_bytes_in(&slot, &self.row.output_path)
                    .map_err(|e| fail(&self.row.output_path, e))?
                    .ok_or_else(|| fail(&self.row.output_path, "recorded output disappeared"))?,
            ),
        };
        if format!("{:x}", Sha256::digest(bytes.as_ref())) != self.row.sha256 {
            return Err(fail(
                &self.row.output_path,
                "output digest differs from recorded bytes",
            ));
        }
        self.binding.recheck(&self.bound.project)?;
        self.bound.workspace()?;
        Ok(bytes)
    }
}

/// Pure native projection by declared materialized media, independent of suffix.
pub fn project_declared_native(
    bytes: &[u8],
    media: &str,
    interpretation: SourceInterpretation,
) -> Result<String, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| projection_error(format_args!("declared native bytes are not UTF-8: {e}")))?;
    match (interpretation, media) {
        (SourceInterpretation::VibeSpec, "text/markdown") => Ok(text.to_owned()),
        (SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc, "application/xml") => {
            let vocabulary = if interpretation == SourceInterpretation::VibeDoc {
                vibe_specdoc::Vocabulary::Doc
            } else {
                vibe_specdoc::Vocabulary::Spec
            };
            vibe_specdoc::from_xml_with(text, vocabulary)
                .map(|doc| vibe_specdoc::to_markdown(&doc))
                .map_err(projection_error)
        }
        _ => Err(projection_error(
            "declared source is not a supported native media/interpretation pair",
        )),
    }
}

fn projection_error(reason: impl std::fmt::Display) -> String {
    format!(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW: {reason}; fix surface: captured native media, interpretation and bytes"
    )
}

pub(super) fn fail(path: &str, reason: impl std::fmt::Display) -> ResolveError {
    ResolveError::SourceObservation {
        path: path.into(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW: {reason}; fix surface: prepare selected installed source bindings again"
        ),
    }
}
#[cfg(test)]
mod tests;
mod validate;
