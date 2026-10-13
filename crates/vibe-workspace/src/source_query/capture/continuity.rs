//! Current-world checks for one prepared point query. Only the selected
//! membership chain and the node's retained dependency closure are observed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SELECTED-WORLD-CONTINUITY");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, LockedPackage, Lockfile, Manifest};
use vibe_core::visibility::NodeId;
use vibe_safefs::{Project, StableFileSnapshot};

use crate::Workspace;
use crate::extension_world::ExtensionWorldEpoch;
use crate::vibedeps::{
    SLOT_RECORD_FILENAME, in_place_slot_abs_path, read_slot_record_bytes, slot_abs_path,
};

use super::{BoundSourceInputs, CaptureOwner, PreparedOwnerCapture, validate_snapshot};
use crate::source_query::SourceQueryUnavailable;

mod extract;
mod membership;
mod retained;
mod service;
pub use extract::{
    CommonMarkExtraction, ExtractedSource, ExtractionError, ExtractionProvenance,
    RepresentationWitness, SelectedDescriptorWitness, SourceEncoding, SpanBasis, SpecExtraction,
    SpecOutline, StructuredView,
};
pub use membership::{
    CanonicalNodeIdentity, CapturedSourceFile, MaterializedVibeMedia, RecordedOutput,
    SourceMembershipIndex, SourceStrength, canonical_node_of_output, native_xml_range_of_output,
};
pub use retained::{
    RetainedOriginCapture, RetainedOriginDocument, RetainedOriginFact, RetainedSourceInventory,
};
pub use service::{PreparedDocInstance, PreparedDocument, SourceQueryResponse, SourceQueryService};

const MANIFEST_CAP: usize = 1024 * 1024;
const LOCK_CAP: usize = 8 * 1024 * 1024;
const RECORD_CAP: usize = 8 * 1024 * 1024;
const OUTPUT_CAP: usize = 64 * 1024 * 1024;

/// Snapshots supplied by the trusted preparation owner. Keys are root-relative
/// node identities (`.` for the workspace root) and selected package IDs.
/// Supplying these values does not read disk or grant a source query.
#[derive(Debug, Clone)]
pub struct PreparedWorldWitness {
    pub lock: Option<StableFileSnapshot>,
    pub membership: BTreeMap<String, StableFileSnapshot>,
    pub slot_records: BTreeMap<(String, String), StableFileSnapshot>,
}

impl PreparedWorldWitness {
    pub fn new(
        lock: Option<StableFileSnapshot>,
        membership: BTreeMap<String, StableFileSnapshot>,
        slot_records: BTreeMap<(String, String), StableFileSnapshot>,
    ) -> Self {
        Self {
            lock,
            membership,
            slot_records,
        }
    }
}

/// A B1 selected source binding plus the exact witnesses needed to refuse a
/// mixed current world. The inner binding remains inert until this check runs.
#[derive(Debug)]
pub struct WitnessedSourceWorld {
    prepared: BoundSourceInputs,
    workspace_root: PathBuf,
    membership: Vec<(String, PathBuf, StableFileSnapshot)>,
    lock_rows: BTreeMap<(String, String), LockedPackage>,
    closure_manifests: BTreeMap<(String, String), (PathBuf, StableFileSnapshot)>,
    slot_records: BTreeMap<(String, String), StableFileSnapshot>,
    has_lock: bool,
}

impl WitnessedSourceWorld {
    /// Root of the retained consumer workspace, for a selected output's
    /// root-relative display path. It is not a new filesystem read grant.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Join an already-selected node and epoch to caller-prepared snapshots.
    /// No provider, workspace discovery or filesystem read occurs here.
    pub fn from_prepared_node(
        workspace: &Workspace,
        node_rel: &str,
        epoch: &ExtensionWorldEpoch,
        requested_visible: &BTreeSet<NodeId>,
        captures: Vec<PreparedOwnerCapture>,
        witnesses: PreparedWorldWitness,
    ) -> Result<Self, SourceQueryUnavailable> {
        let chain = membership_chain(workspace, node_rel)?;
        if witnesses.membership.len() != chain.len() {
            return Err(SourceQueryUnavailable::WorldUnavailable(
                "membership witness",
            ));
        }
        let mut membership = Vec::with_capacity(chain.len());
        for (rel, root, manifest) in chain {
            let snapshot =
                witnesses
                    .membership
                    .get(&rel)
                    .ok_or(SourceQueryUnavailable::WorldUnavailable(
                        "membership witness",
                    ))?;
            check_manifest_snapshot(snapshot, manifest, "membership witness")?;
            membership.push((rel, root, snapshot.clone()));
        }
        let (node_root, node_manifest) = selected_node(workspace, node_rel)?;
        let owner = epoch.node_owner_view(&node_root, node_manifest)?;
        let lock_rows =
            lock_rows_for_owner(witnesses.lock.as_ref(), &owner, epoch, &workspace.root)?;
        let mut closure_manifests = BTreeMap::new();
        for source in &owner.installed {
            let key = coordinate(
                source.provider.id.group().as_str(),
                source.provider.id.name().as_str(),
            );
            let capture = captures
                .iter()
                .find(|capture| capture.coordinate() == Some((key.0.as_str(), key.1.as_str())))
                .ok_or_else(|| {
                    SourceQueryUnavailable::MissingPreparedCapture(format!(
                        "{}/{} manifest",
                        key.0, key.1
                    ))
                })?;
            let expected = epoch.package_manifest(&source.provider.id)?;
            check_manifest_snapshot(&capture.manifest, expected, "selected manifest")?;
            closure_manifests.insert(key, (capture.root.clone(), capture.manifest.clone()));
        }
        let prepared = BoundSourceInputs::from_prepared_node(
            workspace,
            node_rel,
            epoch,
            requested_visible,
            captures,
        )?;
        let mut retained_records = BTreeMap::new();
        for (key, capture) in &prepared.captured {
            let CaptureOwner::Dependency { group, name } = key else {
                continue;
            };
            if let Some(map) = &capture.descriptor {
                let coordinate = coordinate(group, name);
                let record = witnesses.slot_records.get(&coordinate).ok_or(
                    SourceQueryUnavailable::WorldUnavailable("selected descriptor record"),
                )?;
                validate_record(
                    record,
                    &capture.root,
                    &capture.manifest,
                    &capture.outputs,
                    map,
                )?;
                retained_records.insert(coordinate, record.clone());
            }
        }
        Ok(Self {
            prepared,
            workspace_root: workspace.root.clone(),
            membership,
            lock_rows,
            closure_manifests,
            slot_records: retained_records,
            has_lock: witnesses.lock.is_some(),
        })
    }

    /// Compare current no-follow observations to this prepared world. The
    /// answer exposes captured bytes only after all selected checks agree.
    /// This is detection-bound; a later namespace change remains possible.
    pub fn revalidate_current(&self) -> Result<&BoundSourceInputs, SourceQueryUnavailable> {
        self.revalidate_with_hooks(|| {}, || {})
    }

    fn revalidate_with(
        &self,
        after_first_pass: impl FnOnce(),
    ) -> Result<&BoundSourceInputs, SourceQueryUnavailable> {
        self.revalidate_with_hooks(after_first_pass, || {})
    }

    fn revalidate_with_hooks(
        &self,
        after_first_pass: impl FnOnce(),
        after_final_outputs: impl FnOnce(),
    ) -> Result<&BoundSourceInputs, SourceQueryUnavailable> {
        self.check_governing_world()?;
        self.check_governing_files()?;
        self.check_outputs()?;
        after_first_pass();
        self.check_governing_world()?;
        self.check_governing_files()?;
        self.check_outputs()?;
        after_final_outputs();
        // A descriptor or member changed during the final output reads must
        // not be certified by a lock-only tail check. Recheck finite governing
        // witnesses after those reads; no third output pass is needed.
        self.check_governing_files()?;
        self.check_governing_world()?;
        Ok(&self.prepared)
    }

    fn check_governing_files(&self) -> Result<(), SourceQueryUnavailable> {
        for (_, root, prepared) in &self.membership {
            let current = snapshot(
                root,
                Manifest::FILENAME,
                MANIFEST_CAP,
                "membership manifest",
            )?;
            if &current != prepared {
                return Err(SourceQueryUnavailable::StaleWorld("membership manifest"));
            }
        }
        for (key, (root, prepared)) in &self.closure_manifests {
            let current = snapshot(root, Manifest::FILENAME, MANIFEST_CAP, "selected manifest")?;
            if &current != prepared {
                return Err(SourceQueryUnavailable::StaleWorld("selected manifest"));
            }
            if let Some(record) = self.slot_records.get(key) {
                let current = snapshot(
                    root,
                    SLOT_RECORD_FILENAME,
                    RECORD_CAP,
                    "selected descriptor record",
                )?;
                if &current != record {
                    return Err(SourceQueryUnavailable::StaleWorld(
                        "selected descriptor record",
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_outputs(&self) -> Result<(), SourceQueryUnavailable> {
        for capture in self.prepared.captured.values() {
            for (relative, prepared) in &capture.outputs {
                let current = snapshot(&capture.root, relative, OUTPUT_CAP, "selected output")?;
                if &current != prepared {
                    return Err(SourceQueryUnavailable::StaleWorld("selected output"));
                }
            }
        }
        Ok(())
    }

    fn check_governing_world(&self) -> Result<(), SourceQueryUnavailable> {
        let current = Project::open(&self.workspace_root)
            .map_err(|_| SourceQueryUnavailable::WorldUnavailable("workspace root"))?
            .read_file_snapshot_bounded(Lockfile::FILENAME, LOCK_CAP)
            .map_err(|_| SourceQueryUnavailable::WorldUnavailable("selected lock"))?;
        if !self.has_lock && current.is_some() {
            return Err(SourceQueryUnavailable::StaleWorld("selected lock"));
        }
        if self.has_lock {
            let current =
                current.ok_or(SourceQueryUnavailable::WorldUnavailable("selected lock"))?;
            let lock = parse_lock(&current)?;
            let rows = selected_lock_rows(&lock, self.lock_rows.keys())?;
            if rows != self.lock_rows {
                return Err(SourceQueryUnavailable::StaleWorld("selected lock"));
            }
        }
        Ok(())
    }
}

fn coordinate(group: &str, name: &str) -> (String, String) {
    (group.to_owned(), name.to_owned())
}

fn selected_node<'a>(
    workspace: &'a Workspace,
    node_rel: &str,
) -> Result<(PathBuf, &'a Manifest), SourceQueryUnavailable> {
    if node_rel == "." {
        Ok((workspace.root.clone(), &workspace.root_manifest))
    } else {
        let member = workspace
            .member_by_rel_path(node_rel)
            .ok_or_else(|| SourceQueryUnavailable::UnknownNode(node_rel.to_owned()))?;
        Ok((workspace.member_abs_path(member), &member.manifest))
    }
}

fn membership_chain<'a>(
    workspace: &'a Workspace,
    node_rel: &str,
) -> Result<Vec<(String, PathBuf, &'a Manifest)>, SourceQueryUnavailable> {
    let mut chain = vec![(
        ".".to_owned(),
        workspace.root.clone(),
        &workspace.root_manifest,
    )];
    if node_rel == "." {
        return Ok(chain);
    }
    let mut members = Vec::new();
    let mut seen = BTreeSet::new();
    let mut cursor = Some(node_rel.to_owned());
    while let Some(rel) = cursor {
        if !seen.insert(rel.clone()) {
            return Err(SourceQueryUnavailable::WorldUnavailable("membership cycle"));
        }
        let member = workspace
            .member_by_rel_path(&rel)
            .ok_or_else(|| SourceQueryUnavailable::UnknownNode(rel.clone()))?;
        cursor = member
            .parent
            .as_ref()
            .map(|parent| parent.as_str().to_owned());
        members.push((rel, workspace.member_abs_path(member), &member.manifest));
    }
    chain.extend(members.into_iter().rev());
    Ok(chain)
}

fn check_manifest_snapshot(
    snapshot: &StableFileSnapshot,
    expected: &Manifest,
    boundary: &'static str,
) -> Result<(), SourceQueryUnavailable> {
    validate_snapshot(snapshot, boundary)?;
    let text = std::str::from_utf8(&snapshot.bytes)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?;
    let parsed = Manifest::parse_str(text)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?;
    let parsed = serde_json::to_vec(&parsed)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?;
    let expected = serde_json::to_vec(expected)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?;
    if parsed != expected {
        return Err(SourceQueryUnavailable::StaleWorld(boundary));
    }
    Ok(())
}

fn parse_lock(snapshot: &StableFileSnapshot) -> Result<Lockfile, SourceQueryUnavailable> {
    validate_snapshot(snapshot, "selected lock")?;
    let text = std::str::from_utf8(&snapshot.bytes)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable("selected lock"))?;
    let lock: Lockfile = toml::from_str(text)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable("selected lock"))?;
    if lock.meta.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(SourceQueryUnavailable::WorldUnavailable(
            "selected lock schema",
        ));
    }
    Ok(lock)
}

fn lock_rows_for_owner(
    snapshot: Option<&StableFileSnapshot>,
    owner: &vibe_extension_registry::ExtensionWorld,
    epoch: &ExtensionWorldEpoch,
    workspace_root: &Path,
) -> Result<BTreeMap<(String, String), LockedPackage>, SourceQueryUnavailable> {
    let Some(snapshot) = snapshot else {
        if owner.installed.is_empty() {
            return Ok(BTreeMap::new());
        }
        return Err(SourceQueryUnavailable::WorldUnavailable("selected lock"));
    };
    let lock = parse_lock(snapshot)?;
    let keys = owner
        .installed
        .iter()
        .map(|source| {
            coordinate(
                source.provider.id.group().as_str(),
                source.provider.id.name().as_str(),
            )
        })
        .collect::<Vec<_>>();
    let rows = selected_lock_rows(&lock, keys.iter())?;
    for source in &owner.installed {
        let key = coordinate(
            source.provider.id.group().as_str(),
            source.provider.id.name().as_str(),
        );
        let row = rows
            .get(&key)
            .ok_or(SourceQueryUnavailable::WorldUnavailable(
                "selected lock row",
            ))?;
        let manifest = epoch.package_manifest(&source.provider.id)?;
        let declared =
            manifest
                .package
                .as_ref()
                .ok_or(SourceQueryUnavailable::WorldUnavailable(
                    "selected package manifest",
                ))?;
        let lock_root = if row.materialization.is_in_place() {
            in_place_slot_abs_path(workspace_root, &row.group, row.name.as_str())
        } else {
            slot_abs_path(workspace_root, &row.group, row.name.as_str(), &row.version)
        };
        if row.kind != source.provider.kind
            || row.version.to_string() != source.provider.version
            || row.content_hash != source.provider.content_hash
            || row.materialization != declared.materialization
            || lock_root != source.provider.root
            || row.dependencies.len() != epoch.package_edges(&source.provider.id)?.len()
        {
            return Err(SourceQueryUnavailable::StaleWorld("selected lock row"));
        }
        for (edge, expected) in row
            .dependencies
            .iter()
            .zip(epoch.package_edges(&source.provider.id)?)
        {
            let selected = rows
                .get(&coordinate(
                    expected.group().as_str(),
                    expected.name().as_str(),
                ))
                .ok_or(SourceQueryUnavailable::WorldUnavailable(
                    "selected lock edge",
                ))?;
            let pinned = selected
                .as_package_ref()
                .map_err(|_| SourceQueryUnavailable::WorldUnavailable("selected lock edge"))?;
            if *edge != pinned {
                return Err(SourceQueryUnavailable::StaleWorld("selected lock row"));
            }
        }
    }
    Ok(rows)
}

fn selected_lock_rows<'a>(
    lock: &Lockfile,
    keys: impl IntoIterator<Item = &'a (String, String)>,
) -> Result<BTreeMap<(String, String), LockedPackage>, SourceQueryUnavailable> {
    let mut rows = BTreeMap::new();
    for key in keys {
        let mut matches = lock
            .packages
            .iter()
            .filter(|row| row.group.as_str() == key.0 && row.name.as_str() == key.1);
        let row = matches
            .next()
            .ok_or(SourceQueryUnavailable::WorldUnavailable(
                "selected lock row",
            ))?;
        if matches.next().is_some() {
            return Err(SourceQueryUnavailable::WorldUnavailable(
                "duplicate selected lock row",
            ));
        }
        rows.insert(key.clone(), row.clone());
    }
    Ok(rows)
}

fn validate_record(
    snapshot: &StableFileSnapshot,
    root: &Path,
    manifest: &StableFileSnapshot,
    outputs: &BTreeMap<String, StableFileSnapshot>,
    map: &vibe_spec::ValidatedDescriptorMap,
) -> Result<(), SourceQueryUnavailable> {
    validate_snapshot(snapshot, "selected descriptor record")?;
    let record = read_slot_record_bytes(&root.join(SLOT_RECORD_FILENAME), &snapshot.bytes)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable("selected descriptor record"))?;
    if record.source_hash.as_str() != map.source_witness()
        || record
            .files
            .iter()
            .find(|row| row.path == Manifest::FILENAME)
            .map(|row| row.sha256.as_str())
            != Some(manifest.sha256.as_str())
    {
        return Err(SourceQueryUnavailable::StaleWorld(
            "selected descriptor record",
        ));
    }
    for (path, capture) in outputs {
        let row = record.files.iter().find(|row| row.path == *path).ok_or(
            SourceQueryUnavailable::StaleWorld("selected descriptor record"),
        )?;
        let mapped = map
            .output_row(path)
            .ok_or(SourceQueryUnavailable::StaleWorld(
                "selected descriptor record",
            ))?;
        let source = if record.spec_format == vibe_core::manifest::SpecFormat::Mixed {
            row.path.as_str()
        } else {
            row.source
                .as_deref()
                .ok_or(SourceQueryUnavailable::WorldUnavailable(
                    "selected descriptor record",
                ))?
        };
        if row.sha256 != capture.sha256 || mapped.source != source {
            return Err(SourceQueryUnavailable::StaleWorld(
                "selected descriptor record",
            ));
        }
    }
    Ok(())
}

fn snapshot(
    root: &Path,
    relative: &str,
    cap: usize,
    boundary: &'static str,
) -> Result<StableFileSnapshot, SourceQueryUnavailable> {
    Project::open(root)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?
        .read_file_snapshot_bounded(relative, cap)
        .map_err(|_| SourceQueryUnavailable::WorldUnavailable(boundary))?
        .ok_or(SourceQueryUnavailable::WorldUnavailable(boundary))
}

#[cfg(test)]
#[path = "m_01_b_2_cases.rs"]
mod m_01_b_2_cases;

#[cfg(test)]
#[path = "m_01_b_2_final_window.rs"]
mod m_01_b_2_final_window;
