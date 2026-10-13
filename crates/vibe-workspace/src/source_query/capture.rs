//! Caller-prepared bytes joined to one selected owner view. No file/provider
//! operations occur in this constructor; preparation is a separate owner.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#PREPARED-VISIBLE-SOURCE-AUTHORITY");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::manifest::{Manifest, SelectedSource};
use vibe_core::visibility::NodeId;
use vibe_extension_registry::HostExtensionSource;
use vibe_safefs::{StableFileSnapshot, split_relative};
use vibe_spec::{SelectedPackage, ValidatedDescriptorMap};

use crate::Workspace;
use crate::extension_world::{ExtensionWorldEpoch, OrderedResolutionIdentity};

use super::{SourceQueryUnavailable, select_owner_roots};

mod continuity;
pub use continuity::{
    CanonicalNodeIdentity, CapturedSourceFile, CommonMarkExtraction, ExtractedSource,
    ExtractionError, ExtractionProvenance, MaterializedVibeMedia, PreparedDocInstance,
    PreparedDocument, PreparedWorldWitness, RecordedOutput, RepresentationWitness,
    RetainedOriginCapture, RetainedOriginDocument, RetainedOriginFact, SelectedDescriptorWitness,
    SourceEncoding, SourceMembershipIndex, SourceQueryResponse, SourceQueryService, SourceStrength,
    SpanBasis, SpecExtraction, SpecOutline, StructuredView, WitnessedSourceWorld,
    canonical_node_of_output, native_xml_range_of_output,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CaptureOwner {
    SelfNode,
    Dependency { group: String, name: String },
}

impl CaptureOwner {
    fn label(&self) -> String {
        match self {
            Self::SelfNode => "self".to_owned(),
            Self::Dependency { group, name } => format!("{group}/{name}"),
        }
    }
}

/// One separately captured owner. The snapshots contain bytes and held-file
/// identities; the path and owner binding are checked when the selected world
/// is sealed. This input is not itself a public query grant.
#[derive(Debug, Clone)]
pub struct PreparedOwnerCapture {
    owner: CaptureOwner,
    pub(super) root: PathBuf,
    pub(super) manifest: StableFileSnapshot,
    pub(super) outputs: BTreeMap<String, StableFileSnapshot>,
    pub(super) descriptor: Option<ValidatedDescriptorMap>,
}

impl PreparedOwnerCapture {
    pub fn self_node(
        root: PathBuf,
        manifest: StableFileSnapshot,
        outputs: BTreeMap<String, StableFileSnapshot>,
        descriptor: Option<ValidatedDescriptorMap>,
    ) -> Self {
        Self {
            owner: CaptureOwner::SelfNode,
            root,
            manifest,
            outputs,
            descriptor,
        }
    }

    pub fn dependency(
        group: impl Into<String>,
        name: impl Into<String>,
        root: PathBuf,
        manifest: StableFileSnapshot,
        outputs: BTreeMap<String, StableFileSnapshot>,
        descriptor: Option<ValidatedDescriptorMap>,
    ) -> Self {
        Self {
            owner: CaptureOwner::Dependency {
                group: group.into(),
                name: name.into(),
            },
            root,
            manifest,
            outputs,
            descriptor,
        }
    }

    /// Pure parsed-manifest access for the install visibility projection.
    /// The projection receives these captured bytes, never a provider oracle.
    pub fn captured_manifest(&self) -> Result<Manifest, SourceQueryUnavailable> {
        let label = self.owner.label();
        validate_snapshot(&self.manifest, &label)?;
        let text =
            std::str::from_utf8(&self.manifest.bytes).map_err(|error| invalid(&label, error))?;
        Manifest::parse_str(text).map_err(|error| invalid(&label, error))
    }

    pub fn coordinate(&self) -> Option<(&str, &str)> {
        match &self.owner {
            CaptureOwner::SelfNode => None,
            CaptureOwner::Dependency { group, name } => Some((group, name)),
        }
    }
}

/// Validate every supplied package manifest against the same retained epoch
/// before the visibility owner analyzes even hidden edges. This is a pure
/// constructor-time join, not a later filesystem continuity check.
pub fn captured_manifest_closure(
    epoch: &ExtensionWorldEpoch,
    captures: &[PreparedOwnerCapture],
) -> Result<BTreeMap<(String, String), Manifest>, SourceQueryUnavailable> {
    let mut parsed = BTreeMap::new();
    for capture in captures {
        let Some((group, name)) = capture.coordinate() else {
            continue;
        };
        let label = format!("{group}/{name}");
        let installed = epoch
            .installed()
            .find(|row| {
                row.provider.id.group().as_str() == group && row.provider.id.name().as_str() == name
            })
            .ok_or_else(|| invalid(&label, "capture is outside selected resolution"))?;
        if capture.root != installed.provider.root {
            return Err(invalid(
                &label,
                "capture root differs from selected resolution",
            ));
        }
        let manifest = capture.captured_manifest()?;
        let expected = epoch.package_manifest(&installed.provider.id)?;
        if serde_json::to_vec(&manifest).map_err(|error| invalid(&label, error))?
            != serde_json::to_vec(expected).map_err(|error| invalid(&label, error))?
        {
            return Err(invalid(
                &label,
                "captured manifest differs from selected resolution",
            ));
        }
        if parsed
            .insert((group.to_owned(), name.to_owned()), manifest)
            .is_some()
        {
            return Err(invalid(&label, "duplicate prepared manifest coordinate"));
        }
    }
    Ok(parsed)
}

/// Internal binding of selected roots and prepared bytes. This is input to
/// install's visibility-sealed authority, not itself a query grant. Its
/// public constructor is a composition seam; arbitrary visible IDs here
/// cannot authorize a query.
#[derive(Debug)]
pub struct BoundSourceInputs {
    pub(super) host: HostExtensionSource,
    pub(super) node_root: PathBuf,
    pub(super) resolution_identity: OrderedResolutionIdentity,
    pub(super) visible: BTreeSet<NodeId>,
    selected: BTreeMap<(String, String), SelectedPackage>,
    captured: BTreeMap<CaptureOwner, PreparedOwnerCapture>,
}

impl BoundSourceInputs {
    /// Number of retained owners, including self; hidden captures are absent.
    pub fn owner_count(&self) -> usize {
        self.captured.len()
    }

    /// Trusted workspace composition entry: select one exact root/member
    /// first. An arbitrary directory spelling cannot become the self owner.
    pub fn from_prepared_node(
        workspace: &Workspace,
        node_rel: &str,
        epoch: &ExtensionWorldEpoch,
        requested_visible: &BTreeSet<NodeId>,
        captures: Vec<PreparedOwnerCapture>,
    ) -> Result<Self, SourceQueryUnavailable> {
        let (root, manifest) = if node_rel == "." {
            (workspace.root.clone(), &workspace.root_manifest)
        } else {
            let member = workspace
                .member_by_rel_path(node_rel)
                .ok_or_else(|| SourceQueryUnavailable::UnknownNode(node_rel.to_owned()))?;
            (workspace.member_abs_path(member), &member.manifest)
        };
        let analysis = super::visibility::owner_visibility(epoch, &root, manifest, &captures)?;
        let visible = analysis.effective.keys().cloned().collect::<BTreeSet<_>>();
        if !requested_visible.is_subset(&visible) {
            return Err(SourceQueryUnavailable::Visibility(
                "requested source is not visible to the selected node".into(),
            ));
        }
        Self::from_prepared(epoch, &root, manifest, &visible, captures)
    }

    pub(crate) fn from_prepared(
        epoch: &ExtensionWorldEpoch,
        node_root: &Path,
        node_manifest: &Manifest,
        visible: &BTreeSet<NodeId>,
        captures: Vec<PreparedOwnerCapture>,
    ) -> Result<Self, SourceQueryUnavailable> {
        let roots = select_owner_roots(epoch, node_root, node_manifest, visible)?;
        if roots.host.provider.root != node_root {
            return Err(invalid("self", "host root differs from selected node"));
        }
        let mut supplied = BTreeMap::new();
        for capture in captures {
            let key = capture.owner.clone();
            if supplied.insert(key.clone(), capture).is_some() {
                return Err(invalid(&key.label(), "duplicate prepared capture"));
            }
        }
        let mut captured = BTreeMap::new();
        let self_capture = supplied
            .remove(&CaptureOwner::SelfNode)
            .ok_or_else(|| SourceQueryUnavailable::MissingPreparedCapture("self".into()))?;
        validate_owner_capture(&self_capture, node_root, node_manifest, None)?;
        captured.insert(CaptureOwner::SelfNode, self_capture);
        for ((group, name), selected) in &roots.selected {
            let key = CaptureOwner::Dependency {
                group: group.clone(),
                name: name.clone(),
            };
            let capture = supplied
                .remove(&key)
                .ok_or_else(|| SourceQueryUnavailable::MissingPreparedCapture(key.label()))?;
            let installed = epoch
                .installed()
                .find(|row| {
                    row.provider.id.group().as_str() == group
                        && row.provider.id.name().as_str() == name
                })
                .ok_or_else(|| invalid(&key.label(), "not in ordered resolution"))?;
            if installed.provider.root != selected.root
                || installed.provider.version != selected.version
            {
                return Err(invalid(
                    &key.label(),
                    "selected root/version differs from resolution",
                ));
            }
            let expected = epoch.package_manifest(&installed.provider.id)?;
            validate_owner_capture(
                &capture,
                &selected.root,
                expected,
                Some(installed.provider.content_hash.as_str()),
            )?;
            captured.insert(key, capture);
        }
        // Nonvisible captures were needed only as prepared visibility input;
        // discard them rather than publishing a hidden package read grant.
        Ok(Self {
            host: roots.host,
            node_root: node_root.to_path_buf(),
            resolution_identity: epoch.resolution_identity().clone(),
            visible: visible.clone(),
            selected: roots.selected,
            captured,
        })
    }

    pub(crate) fn selected_root(&self, group: &str, name: &str) -> Option<&Path> {
        self.selected
            .get(&(group.to_owned(), name.to_owned()))
            .map(|row| row.root.as_path())
    }

    pub(crate) fn captured_output(
        &self,
        group: Option<(&str, &str)>,
        relative: &str,
    ) -> Result<&StableFileSnapshot, SourceQueryUnavailable> {
        let key = group.map_or(CaptureOwner::SelfNode, |(group, name)| {
            CaptureOwner::Dependency {
                group: group.to_owned(),
                name: name.to_owned(),
            }
        });
        let label = key.label();
        self.captured
            .get(&key)
            .and_then(|capture| capture.outputs.get(relative))
            .ok_or(SourceQueryUnavailable::MissingPreparedCapture(format!(
                "{label}/{relative}"
            )))
    }
}

fn validate_owner_capture(
    capture: &PreparedOwnerCapture,
    root: &Path,
    expected_manifest: &Manifest,
    source_witness: Option<&str>,
) -> Result<(), SourceQueryUnavailable> {
    let label = capture.owner.label();
    if capture.root != root {
        return Err(invalid(&label, "capture root differs from selected root"));
    }
    let manifest = capture.captured_manifest()?;
    let actual = serde_json::to_vec(&manifest).map_err(|error| invalid(&label, error))?;
    let expected = serde_json::to_vec(expected_manifest).map_err(|error| invalid(&label, error))?;
    if actual != expected {
        return Err(invalid(
            &label,
            "captured manifest differs from selected manifest",
        ));
    }
    if capture.outputs.is_empty() {
        return Err(SourceQueryUnavailable::MissingPreparedCapture(label));
    }
    if let Some(map) = &capture.descriptor {
        if !map.spec_root().starts_with(root)
            || map.manifest_sha256() != capture.manifest.sha256
            || source_witness.is_some_and(|witness| map.source_witness() != witness)
        {
            return Err(invalid(
                &label,
                "descriptor identity differs from prepared owner",
            ));
        }
    }
    for (relative, snapshot) in &capture.outputs {
        split_relative(relative).map_err(|error| invalid(&label, error))?;
        validate_snapshot(snapshot, &label)?;
        if let Some(map) = &capture.descriptor {
            let row = map
                .output_row(relative)
                .ok_or_else(|| invalid(&label, "captured output has no selected descriptor row"))?;
            if row.output_sha256 != snapshot.sha256 {
                return Err(invalid(
                    &label,
                    "captured output differs from descriptor digest",
                ));
            }
            let selection = if let Some(package) = &manifest.package {
                package.select_source(&row.source)
            } else if let Some(project) = &manifest.project {
                let coordinate = project
                    .group
                    .as_ref()
                    .map(|group| format!("{group}/{}", project.name))
                    .unwrap_or_else(|| "self".into());
                vibe_core::manifest::select_source_rows(&coordinate, &project.sources, &row.source)
            } else {
                return Err(invalid(
                    &label,
                    "descriptor owner has no authored node manifest",
                ));
            }
            .map_err(|error| invalid(&label, format!("source selection: {error:?}")))?;
            let interpretation = match selection {
                SelectedSource::Unclassified { .. } => None,
                SelectedSource::Descriptor { descriptor, .. } => Some(descriptor.interpretation()),
                SelectedSource::NeedsInput { .. } => {
                    return Err(invalid(&label, "source declaration needs input selection"));
                }
            };
            if interpretation != row.interpretation {
                return Err(invalid(
                    &label,
                    "descriptor interpretation differs from captured manifest",
                ));
            }
        }
    }
    Ok(())
}

fn validate_snapshot(
    snapshot: &StableFileSnapshot,
    owner: &str,
) -> Result<(), SourceQueryUnavailable> {
    let digest = format!("{:x}", Sha256::digest(&snapshot.bytes));
    if snapshot.size != snapshot.bytes.len() as u64 || snapshot.sha256 != digest {
        return Err(invalid(
            owner,
            "prepared bytes differ from their stated digest/size",
        ));
    }
    Ok(())
}

fn invalid(owner: &str, reason: impl std::fmt::Display) -> SourceQueryUnavailable {
    SourceQueryUnavailable::InvalidCapture {
        owner: owner.to_owned(),
        reason: reason.to_string(),
    }
}
