//! One-shot, all-memory preparation of the exact Collect replay closure.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::{ExtensionKey, LinkType, SpecFormat};
use vibe_spec::{
    CompilerNativePolicy, CompilerPendingSet, FileResolver, FsSectionSource, SelfCoordinate,
};

use crate::boot::hybrid::{UnitId, UnitInput};
use crate::boot::{BootProvenance, EffectiveBoot};
use crate::extension_world::{
    CompilerNativeReplayFactory, OwnerRuntimeEpoch, OwnerRuntimeEpochToken, OwnerRuntimeId,
    PendingArtifactEvidence,
};
use crate::{WorkspaceError, boot_artifacts};

#[path = "replay_prepare/prepared.rs"]
mod prepared;
pub(crate) use prepared::PreparedBootReplay;
#[path = "replay_prepare/validate.rs"]
mod validate;
use validate::validate_ready;

pub(super) struct UnitReplayCandidate {
    owner: OwnerRuntimeId,
    id: UnitId,
    effective: EffectiveBoot,
    spec_format: SpecFormat,
    base_fingerprint: String,
    boot_dir: PathBuf,
    index_path: PathBuf,
    static_path: PathBuf,
    stale_path: PathBuf,
    dependencies: Box<[UnitId]>,
    native: bool,
}

pub(super) struct NodeReplayCandidate {
    owner: OwnerRuntimeId,
    rel: String,
    effective: EffectiveBoot,
    spec_format: SpecFormat,
    node_dir: PathBuf,
    index_path: PathBuf,
    static_path: PathBuf,
    stale_path: PathBuf,
    dependencies: Box<[UnitId]>,
    native: bool,
}

pub(super) struct BootReplayCandidates {
    workspace_root: PathBuf,
    self_coord: SelfCoordinate,
    units: Vec<UnitReplayCandidate>,
    nodes: Vec<NodeReplayCandidate>,
}

impl BootReplayCandidates {
    pub(super) fn new(workspace_root: PathBuf, self_coord: SelfCoordinate) -> Self {
        Self {
            workspace_root,
            self_coord,
            units: Vec::new(),
            nodes: Vec::new(),
        }
    }

    pub(super) fn push_unit(&mut self, candidate: UnitReplayCandidate) {
        self.units.push(candidate);
    }

    pub(super) fn push_node(&mut self, candidate: NodeReplayCandidate) {
        self.nodes.push(candidate);
    }
}

impl UnitReplayCandidate {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        workspace_root: &Path,
        slot: &str,
        owner: OwnerRuntimeId,
        id: UnitId,
        effective: EffectiveBoot,
        spec_format: SpecFormat,
        base_fingerprint: String,
        dependencies: Box<[UnitId]>,
        native: bool,
    ) -> Self {
        let boot_dir = workspace_root
            .join(slot)
            .join(vibe_core::layout::current_boot_dir());
        let index_path = boot_dir.join(boot_artifacts::INDEX_FILE);
        let static_path = boot_dir.join(boot_artifacts::static_file(spec_format));
        let stale_path = boot_dir.join(boot_artifacts::publication::stale_static_file(spec_format));
        Self {
            owner,
            id,
            effective,
            spec_format,
            base_fingerprint,
            boot_dir,
            index_path,
            static_path,
            stale_path,
            dependencies,
            native,
        }
    }
}

impl NodeReplayCandidate {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        node_dir: PathBuf,
        rel: String,
        effective: EffectiveBoot,
        spec_format: SpecFormat,
        dependencies: Box<[UnitId]>,
        native: bool,
    ) -> Self {
        let boot_dir = node_dir.join(vibe_core::layout::current_boot_dir());
        Self {
            owner: OwnerRuntimeId::Node { rel: rel.clone() },
            rel,
            effective,
            spec_format,
            node_dir,
            index_path: boot_dir.join(boot_artifacts::INDEX_FILE),
            static_path: boot_dir.join(boot_artifacts::static_file(spec_format)),
            stale_path: boot_dir.join(boot_artifacts::publication::stale_static_file(spec_format)),
            dependencies,
            native,
        }
    }
}

pub(crate) enum BootReplaySet {
    Empty,
    Sealed(SealedBootReplay),
}

pub(crate) struct SealedBootReplay {
    token: OwnerRuntimeEpochToken,
    workspace_root: PathBuf,
    self_coord: SelfCoordinate,
    lanes: Vec<ReplayLane>,
}

enum ReplayCandidate {
    Unit(UnitReplayCandidate),
    Node(NodeReplayCandidate),
}

struct ReplayLane {
    candidate: ReplayCandidate,
    direct: Option<DirectPending>,
}

struct DirectPending {
    evidence: Option<PendingArtifactEvidence>,
    pending: Option<CompilerPendingSet>,
    expected: Box<[ReceiptIdentity]>,
}

#[derive(PartialEq, Eq)]
struct ReceiptIdentity {
    plan_digest: [u8; 32],
    order: u32,
    key: ExtensionKey,
}

pub(super) fn static_dependencies(
    id: &UnitId,
    table: &HashMap<UnitId, UnitInput>,
    emitted: &HashSet<UnitId>,
) -> Box<[UnitId]> {
    let mut dependencies = table
        .get(id)
        .into_iter()
        .flat_map(|unit| &unit.edges)
        .filter(|edge| {
            emitted.contains(&edge.target)
                && matches!(
                    edge.link,
                    LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
                )
        })
        .map(|edge| edge.target.clone())
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    dependencies.into_boxed_slice()
}

pub(super) fn node_generated_dependencies(
    effective: &EffectiveBoot,
    emitted: &HashSet<UnitId>,
) -> Box<[UnitId]> {
    effective
        .static_entries()
        .filter(|entry| entry.unit_substituted)
        .filter_map(|entry| match &entry.provenance {
            BootProvenance::Dependency { group, name } => {
                let id = (group.clone(), name.clone());
                emitted.contains(&id).then_some(id)
            }
            BootProvenance::Node => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

pub(super) fn seal_replay_set(
    candidates: BootReplayCandidates,
    continuations: BTreeMap<OwnerRuntimeId, boot_artifacts::OwnerNativeCompileContinuation>,
    epoch: &OwnerRuntimeEpoch,
) -> Result<BootReplaySet, WorkspaceError> {
    let mut direct = BTreeMap::new();
    for (owner, continuation) in continuations {
        if let boot_artifacts::OwnerNativeCompileContinuation::Pending { evidence, pending } =
            continuation
        {
            let expected_header = vibe_spec::compiler_pending_header_payload(
                &pending,
                evidence.fingerprint().as_bytes(),
            )
            .map_err(|_| replay_error(&owner.to_string(), "pending evidence header refused"))?;
            if expected_header != evidence.header_payload() {
                return Err(replay_error(
                    &owner.to_string(),
                    "pending evidence does not bind its exact pending set",
                ));
            }
            let expected = pending
                .iter()
                .map(|reference| ReceiptIdentity {
                    plan_digest: *reference.plan_digest_bytes(),
                    order: reference.order(),
                    key: reference.key().clone(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            direct.insert(
                owner,
                DirectPending {
                    evidence: Some(evidence),
                    pending: Some(pending),
                    expected,
                },
            );
        }
    }
    if direct.is_empty() {
        return Ok(BootReplaySet::Empty);
    }

    let mut affected_units = direct
        .keys()
        .filter_map(|owner| match owner {
            OwnerRuntimeId::Unit { provider } => {
                Some((provider.group().clone(), provider.name().to_string()))
            }
            OwnerRuntimeId::Node { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    loop {
        let parents = candidates
            .units
            .iter()
            .filter(|candidate| {
                !affected_units.contains(&candidate.id)
                    && candidate
                        .dependencies
                        .iter()
                        .any(|child| affected_units.contains(child))
            })
            .map(|candidate| candidate.id.clone())
            .collect::<Vec<_>>();
        if parents.is_empty() {
            break;
        }
        affected_units.extend(parents);
    }

    let expected_units = affected_units.len();
    let mut lanes = Vec::new();
    let mut seen_owners = BTreeSet::new();
    let mut visited_units = BTreeSet::new();
    for candidate in candidates.units {
        if affected_units.contains(&candidate.id) {
            let owner = candidate.owner.clone();
            if !seen_owners.insert(owner.clone()) {
                return Err(replay_error(&owner.to_string(), "duplicate replay recipe"));
            }
            if !visited_units.insert(candidate.id.clone()) {
                return Err(replay_error(&owner.to_string(), "duplicate affected unit"));
            }
            if candidate.dependencies.iter().any(|dependency| {
                affected_units.contains(dependency) && !visited_units.contains(dependency)
            }) {
                return Err(replay_error(
                    &owner.to_string(),
                    "affected unit order is not dependency-first",
                ));
            }
            let direct_lane = direct.remove(&owner);
            if direct_lane.is_some() && !candidate.native {
                return Err(replay_error(
                    &owner.to_string(),
                    "direct Pending unit has no native intersection",
                ));
            }
            lanes.push(ReplayLane {
                candidate: ReplayCandidate::Unit(candidate),
                direct: direct_lane,
            });
        }
    }
    if lanes.len() != expected_units {
        return Err(replay_error(
            "<units>",
            "affected unit closure is incomplete or cyclic",
        ));
    }

    let mut nodes = candidates.nodes;
    nodes.sort_by(|left, right| left.rel.cmp(&right.rel));
    for candidate in nodes {
        let owner = candidate.owner.clone();
        if direct.contains_key(&owner)
            || candidate
                .dependencies
                .iter()
                .any(|dependency| affected_units.contains(dependency))
        {
            if !seen_owners.insert(owner.clone()) {
                return Err(replay_error(&owner.to_string(), "duplicate replay recipe"));
            }
            let direct_lane = direct.remove(&owner);
            if direct_lane.is_some() && !candidate.native {
                return Err(replay_error(
                    &owner.to_string(),
                    "direct Pending node has no native intersection",
                ));
            }
            lanes.push(ReplayLane {
                candidate: ReplayCandidate::Node(candidate),
                direct: direct_lane,
            });
        }
    }
    if !direct.is_empty() {
        return Err(replay_error(
            "<closure>",
            "a direct Pending owner has no retained replay recipe",
        ));
    }
    Ok(BootReplaySet::Sealed(SealedBootReplay {
        token: epoch.replay_token(),
        workspace_root: candidates.workspace_root,
        self_coord: candidates.self_coord,
        lanes,
    }))
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "remove when R5.4-INSTALL invokes replay preparation"
    )
)]
pub(crate) fn prepare_boot_replay<F: CompilerNativeReplayFactory>(
    replay: BootReplaySet,
    epoch: &OwnerRuntimeEpoch,
    factory: &mut F,
) -> Result<PreparedBootReplay, WorkspaceError> {
    let BootReplaySet::Sealed(mut replay) = replay else {
        return Ok(PreparedBootReplay {
            publications: Box::new([]),
        });
    };
    if !epoch.matches_replay_token(&replay.token) {
        return Err(replay_error(
            "<epoch>",
            "replay belongs to a different runtime epoch",
        ));
    }

    let mut prepared_indexes = Vec::with_capacity(replay.lanes.len());
    for lane in &replay.lanes {
        let candidate = lane.candidate();
        boot_artifacts::publication::preflight_artifact_targets(
            candidate.index_path(),
            candidate.static_path(),
            candidate.stale_path(),
        )?;
        prepared_indexes.push(Some(boot_artifacts::publication::prepare_index(
            candidate.effective(),
            candidate.spec_format(),
        )?));
    }

    let mut policies = BTreeMap::new();
    for lane in &mut replay.lanes {
        if !lane.candidate.native() {
            continue;
        }
        let policy = match lane.direct.as_mut() {
            Some(direct) => {
                CompilerNativePolicy::resolve(direct.pending.take().ok_or_else(|| {
                    replay_error(
                        &lane.candidate.owner().to_string(),
                        "pending set visited twice",
                    )
                })?)
            }
            None => CompilerNativePolicy::fail(),
        };
        let owner = lane.candidate.owner().clone();
        if policies.insert(owner.clone(), policy).is_some() {
            return Err(replay_error(&owner.to_string(), "duplicate replay policy"));
        }
    }
    let mut provider = factory.create(policies)?;
    let mut overlay = BTreeMap::new();
    let mut publications = Vec::with_capacity(replay.lanes.len());
    let mut failure = None;

    for (position, mut lane) in replay.lanes.into_iter().enumerate() {
        let prepared_index = prepared_indexes[position].take().ok_or_else(|| {
            replay_error(&lane.candidate.owner().to_string(), "lane visited twice")
        })?;
        let source = FsSectionSource::with_overlay(
            FileResolver::new(&replay.workspace_root, replay.self_coord.clone()),
            overlay.clone(),
        );
        let owner = match lane.candidate.owner() {
            OwnerRuntimeId::Unit { provider } => epoch.unit(provider),
            OwnerRuntimeId::Node { rel } => epoch.node(rel),
        }?;
        let compiled = boot_artifacts::native_managed::compile_static_owner_managed_with_source(
            lane.candidate.effective(),
            &replay.workspace_root,
            &replay.self_coord,
            lane.candidate.spec_format(),
            owner,
            &source,
            &overlay,
            lane.candidate.native().then_some(&mut provider),
        );
        let compiled = match compiled {
            Ok(Some(compiled)) => compiled,
            Ok(None) => {
                failure = Some(replay_error(
                    &lane.candidate.owner().to_string(),
                    "affected lane produced no static artifact",
                ));
                break;
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        };
        let (artifact, continuation, _) = compiled.into_parts();
        if let Err(error) = validate_ready(&mut lane, continuation.as_ref()) {
            failure = Some(error);
            break;
        }
        if lane.direct.is_some() {
            failure = Some(replay_error(
                &lane.candidate.owner().to_string(),
                "direct Pending state survived replay receipt validation",
            ));
            break;
        }
        overlay.insert(
            lane.candidate.static_path().to_path_buf(),
            Arc::<[u8]>::from(artifact.bytes().to_vec()),
        );
        let index = boot_artifacts::publication::finish_index(
            prepared_index,
            lane.candidate.base_fingerprint(),
            None,
        )
        .into_bytes()
        .into_boxed_slice();
        publications
            .push(lane.into_publication(index, Some(artifact.into_bytes().into_boxed_slice())));
    }

    let terminal = factory.finish(provider);
    if let Some(error) = failure {
        return Err(error);
    }
    terminal?;
    if publications.len() != prepared_indexes.len() {
        return Err(replay_error(
            "<order>",
            "not every sealed replay lane was prepared",
        ));
    }
    Ok(PreparedBootReplay {
        publications: publications.into_boxed_slice(),
    })
}

impl ReplayLane {
    fn candidate(&self) -> &ReplayCandidate {
        &self.candidate
    }
}

impl ReplayCandidate {
    fn owner(&self) -> &OwnerRuntimeId {
        match self {
            Self::Unit(candidate) => &candidate.owner,
            Self::Node(candidate) => &candidate.owner,
        }
    }

    fn effective(&self) -> &EffectiveBoot {
        match self {
            Self::Unit(candidate) => &candidate.effective,
            Self::Node(candidate) => &candidate.effective,
        }
    }

    fn spec_format(&self) -> SpecFormat {
        match self {
            Self::Unit(candidate) => candidate.spec_format,
            Self::Node(candidate) => candidate.spec_format,
        }
    }

    fn native(&self) -> bool {
        match self {
            Self::Unit(candidate) => candidate.native,
            Self::Node(candidate) => candidate.native,
        }
    }

    fn index_path(&self) -> &Path {
        match self {
            Self::Unit(candidate) => &candidate.index_path,
            Self::Node(candidate) => &candidate.index_path,
        }
    }

    fn static_path(&self) -> &Path {
        match self {
            Self::Unit(candidate) => &candidate.static_path,
            Self::Node(candidate) => &candidate.static_path,
        }
    }

    fn stale_path(&self) -> &Path {
        match self {
            Self::Unit(candidate) => &candidate.stale_path,
            Self::Node(candidate) => &candidate.stale_path,
        }
    }

    fn base_fingerprint(&self) -> Option<&str> {
        match self {
            Self::Unit(candidate) => Some(&candidate.base_fingerprint),
            Self::Node(_) => None,
        }
    }
}

pub(super) fn replay_error(owner: &str, reason: impl Into<String>) -> WorkspaceError {
    WorkspaceError::NativeCompileProvider {
        owner: owner.to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "replay_prepare/tests.rs"]
mod tests;
