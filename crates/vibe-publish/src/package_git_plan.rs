//! Pure protected package-Git publication identity and recovery comparison.
//! No C.1 reservation, Git command, RepoCreator call, network, or effect grant.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DOMAIN: &[u8] = b"vibevm/package-git-publication-plan/e2\0";
const EPOCH: u32 = 2;

mod objects;
pub(crate) use objects::{CommitMetadata, GitPerson, TagMetadata};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageCoordinate {
    pub kind: String,
    pub group: String,
    pub name: String,
    pub version: String,
    /// This selected version may be replaced without a SemVer increment.
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostedRepository {
    pub host: String,
    pub organization: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreationOptions {
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub default_branch: String,
    pub auto_init: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RepositoryTarget {
    HostedCreate {
        repository: HostedRepository,
        creation: CreationOptions,
    },
    /// Digest of a host-selected, credential-free canonical remote identity.
    /// It is not a raw URL, and no numeric provider ID is invented.
    DirectGitExisting { remote_identity_sha256: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RepositoryIdentity {
    Hosted {
        repository: HostedRepository,
        id: u64,
    },
    Direct {
        remote_identity_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RepositoryObservation {
    Absent,
    Present(RepositoryIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RawRef {
    Absent,
    Present { oid: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum TagRef {
    Absent,
    Present {
        raw_oid: String,
        peeled_commit_oid: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RefSnapshot {
    pub main: RawRef,
    pub version_tag: TagRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum StagedCommit {
    /// New commit whose one parent is the exact observed main, or no parent
    /// when the repository was absent/empty.
    New {
        oid: String,
        parent_oid: Option<String>,
    },
    /// Exact pre-existing main commit reused for an unchanged payload.
    ReusedMain { oid: String },
}

impl StagedCommit {
    fn oid(&self) -> &str {
        match self {
            Self::New { oid, .. } | Self::ReusedMain { oid } => oid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StagedObjects {
    /// Claimed identities only. A later owner must retain and verify the
    /// actual object bytes/types before this plan can authorize an effect.
    pub git_tree_oid: String,
    pub commit: StagedCommit,
    /// None only for a pre-effect no-op that reuses the observed peeled tag;
    /// it avoids manufacturing a new annotated object merely to decide.
    pub annotated_tag_oid: Option<String>,
    pub tag_target_commit_oid: String,
    pub commit_metadata: Option<CommitMetadata>,
    pub tag_metadata: Option<TagMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageGitPlan {
    pub epoch: u32,
    pub package: PackageCoordinate,
    /// Exact selected mutable version ref; never inferred by a transport.
    pub version_tag_ref: String,
    pub target: RepositoryTarget,
    /// Exact held flattened payload bytes and separately retained provenance.
    /// Claimed held-byte witness; this pure module cannot prove custody.
    pub payload_sha256: String,
    pub flattened_submodules_sha256: String,
    /// Git object algorithm is selected before staging; all OIDs must agree.
    pub oid_hex_len: usize,
    pub before_repo: RepositoryObservation,
    pub before_refs: RefSnapshot,
    pub staged: StagedObjects,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PlanError {
    #[error("protected package Git plan is invalid: {0}")]
    Invalid(&'static str),
}

fn invalid(reason: &'static str) -> PlanError {
    PlanError::Invalid(reason)
}
fn oid(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| oid(hex, 64))
}
fn text(value: &str, cap: usize) -> bool {
    !value.is_empty()
        && value.len() <= cap
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
fn source_token(value: &str) -> bool {
    text(value, 256)
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
fn repo_token(value: &str) -> bool {
    text(value, 100)
        && value.is_ascii()
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && !value.contains("..")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn host(value: &str) -> bool {
    text(value, 253)
        && value.is_ascii()
        && value == value.to_ascii_lowercase()
        && !value.contains("..")
        && value.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn raw_oid(value: &RawRef) -> Option<&str> {
    match value {
        RawRef::Absent => None,
        RawRef::Present { oid } => Some(oid),
    }
}
fn tag_raw(value: &TagRef) -> Option<&str> {
    match value {
        TagRef::Absent => None,
        TagRef::Present { raw_oid, .. } => Some(raw_oid),
    }
}

impl PackageGitPlan {
    pub(crate) fn validate(&self) -> Result<(), PlanError> {
        if self.epoch != EPOCH {
            return Err(invalid("package Git plan epoch must be e2"));
        }
        if !source_token(&self.package.kind)
            || !source_token(&self.package.group)
            || !source_token(&self.package.name)
            || !self.package.mutable
            || semver::Version::parse(&self.package.version)
                .ok()
                .is_none_or(|v| v.to_string() != self.package.version)
        {
            return Err(invalid("package coordinate or mutable SemVer"));
        }
        let Some(tag_name) = self.version_tag_ref.strip_prefix("refs/tags/") else {
            return Err(invalid("selected version tag ref"));
        };
        let Some(prefix) = tag_name.strip_suffix(&self.package.version) else {
            return Err(invalid("selected version tag differs from package version"));
        };
        if prefix.is_empty()
            || prefix.len() > 32
            || !prefix.is_ascii()
            || !prefix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            || tag_name.contains("..")
        {
            return Err(invalid("selected version tag prefix"));
        }
        match (&self.target, &self.before_repo) {
            (
                RepositoryTarget::HostedCreate {
                    repository,
                    creation,
                },
                before,
            ) => {
                if !host(&repository.host)
                    || !repo_token(&repository.organization)
                    || !repo_token(&repository.name)
                    || creation.default_branch != "main"
                    || creation.auto_init
                    || creation
                        .description
                        .as_deref()
                        .is_some_and(|v| !text(v, 1024))
                    || creation.homepage.as_deref().is_some_and(|v| {
                        !text(v, 2048)
                            || !v.starts_with("https://")
                            || v.contains(['@', '?', '#'])
                            || v[8..].split('/').next().is_none_or(|name| !host(name))
                    })
                {
                    return Err(invalid("hosted repository scope or creation options"));
                }
                if let RepositoryObservation::Present(RepositoryIdentity::Hosted {
                    repository: observed,
                    id,
                }) = before
                    && (observed != repository || *id == 0)
                {
                    return Err(invalid("observed hosted repository identity"));
                }
                if matches!(
                    before,
                    RepositoryObservation::Present(RepositoryIdentity::Direct { .. })
                ) {
                    return Err(invalid("direct identity cannot satisfy hosted target"));
                }
            }
            (
                RepositoryTarget::DirectGitExisting {
                    remote_identity_sha256,
                },
                RepositoryObservation::Present(RepositoryIdentity::Direct {
                    remote_identity_sha256: observed,
                }),
            ) if digest(remote_identity_sha256) && observed == remote_identity_sha256 => {}
            (RepositoryTarget::DirectGitExisting { .. }, _) => {
                return Err(invalid(
                    "direct Git target needs the same present credential-free identity",
                ));
            }
        }
        if !digest(&self.payload_sha256) || !digest(&self.flattened_submodules_sha256) {
            return Err(invalid("held payload or submodule provenance digest"));
        }
        if self.oid_hex_len != 40 && self.oid_hex_len != 64 {
            return Err(invalid(
                "Git object algorithm must use 40 or 64 lowercase hex",
            ));
        }
        let len = self.oid_hex_len;
        let present_main = raw_oid(&self.before_refs.main);
        if present_main.is_some_and(|v| !oid(v, len))
            || tag_raw(&self.before_refs.version_tag).is_some_and(|v| !oid(v, len))
            || matches!(&self.before_refs.version_tag, TagRef::Present { peeled_commit_oid, .. } if !oid(peeled_commit_oid, len))
            || !oid(&self.staged.git_tree_oid, len)
            || !oid(self.staged.commit.oid(), len)
            || self
                .staged
                .annotated_tag_oid
                .as_deref()
                .is_some_and(|v| !oid(v, len))
            || !oid(&self.staged.tag_target_commit_oid, len)
        {
            return Err(invalid(
                "Git object OID is malformed or uses a foreign algorithm",
            ));
        }
        if matches!(self.before_repo, RepositoryObservation::Absent)
            && (present_main.is_some() || !matches!(self.before_refs.version_tag, TagRef::Absent))
        {
            return Err(invalid("absent repository cannot have refs"));
        }
        if self.staged.tag_target_commit_oid != self.staged.commit.oid() {
            return Err(invalid("annotated tag does not target staged commit"));
        }
        match &self.staged.commit {
            StagedCommit::New {
                oid: desired,
                parent_oid,
            } => {
                if parent_oid.as_deref() != present_main || present_main == Some(desired) {
                    return Err(invalid("new commit parent is not the exact observed main"));
                }
            }
            StagedCommit::ReusedMain { oid: desired } if present_main != Some(desired) => {
                return Err(invalid("reused commit is not observed main"));
            }
            _ => {}
        }
        if self.staged.annotated_tag_oid.is_none()
            && !matches!((&self.staged.commit, &self.before_refs.version_tag),
                (StagedCommit::ReusedMain { oid }, TagRef::Present { peeled_commit_oid, .. })
                    if oid == peeled_commit_oid)
        {
            return Err(invalid(
                "absent staged tag object requires an exact peeled pre-effect no-op",
            ));
        }
        self.validate_object_bytes()?;
        Ok(())
    }

    pub(crate) fn digest(&self) -> Result<String, PlanError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| invalid("plan serialization"))?;
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
        Ok(format!("sha256:{:x}", hash.finalize()))
    }

    fn desired_refs(&self) -> Option<RefSnapshot> {
        Some(RefSnapshot {
            main: RawRef::Present {
                oid: self.staged.commit.oid().into(),
            },
            version_tag: TagRef::Present {
                raw_oid: self.staged.annotated_tag_oid.clone()?,
                peeled_commit_oid: self.staged.tag_target_commit_oid.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoReady {
    identity: RepositoryIdentity,
}
impl RepoReady {
    pub(crate) fn identity(&self) -> &RepositoryIdentity {
        &self.identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateDecision {
    NeedCreate,
    AlreadyPresent(RepoReady),
    Landed(RepoReady),
    Uncertain,
    RecoveryRequired,
}

fn identity_matches_target(target: &RepositoryTarget, identity: &RepositoryIdentity) -> bool {
    match (target, identity) {
        (
            RepositoryTarget::HostedCreate { repository, .. },
            RepositoryIdentity::Hosted {
                repository: observed,
                id,
            },
        ) => repository == observed && *id > 0,
        (
            RepositoryTarget::DirectGitExisting {
                remote_identity_sha256,
            },
            RepositoryIdentity::Direct {
                remote_identity_sha256: observed,
            },
        ) => remote_identity_sha256 == observed,
        _ => false,
    }
}

/// The optional response must be retained evidence from an actual successful
/// hosted CreateRepo callback. This pure comparison cannot authenticate it.
/// A lost response cannot be promoted by a same-name GET.
pub(crate) fn classify_create(
    plan: &PackageGitPlan,
    dispatched: bool,
    response: Option<&RepositoryIdentity>,
    observed: &RepositoryObservation,
) -> Result<CreateDecision, PlanError> {
    plan.validate()?;
    if !dispatched && response.is_some() {
        return Err(invalid("repository callback without a dispatched call"));
    }
    if matches!(observed, RepositoryObservation::Present(identity)
        if !identity_matches_target(&plan.target, identity))
        || response.is_some_and(|identity| !identity_matches_target(&plan.target, identity))
    {
        return Ok(CreateDecision::RecoveryRequired);
    }
    match (
        &plan.target,
        &plan.before_repo,
        dispatched,
        response,
        observed,
    ) {
        (
            RepositoryTarget::DirectGitExisting { .. },
            RepositoryObservation::Present(expected),
            false,
            None,
            RepositoryObservation::Present(now),
        ) if expected == now => Ok(CreateDecision::AlreadyPresent(RepoReady {
            identity: now.clone(),
        })),
        (RepositoryTarget::DirectGitExisting { .. }, _, _, _, _) => {
            Ok(CreateDecision::RecoveryRequired)
        }
        (
            RepositoryTarget::HostedCreate { .. },
            RepositoryObservation::Present(expected),
            false,
            None,
            RepositoryObservation::Present(now),
        ) if expected == now => Ok(CreateDecision::AlreadyPresent(RepoReady {
            identity: now.clone(),
        })),
        (
            RepositoryTarget::HostedCreate { .. },
            RepositoryObservation::Absent,
            false,
            None,
            RepositoryObservation::Absent,
        ) => Ok(CreateDecision::NeedCreate),
        (
            RepositoryTarget::HostedCreate { repository, .. },
            RepositoryObservation::Absent,
            true,
            Some(RepositoryIdentity::Hosted {
                repository: response_repo,
                id,
            }),
            RepositoryObservation::Present(now),
        ) if *id > 0
            && response_repo == repository
            && now
                == &RepositoryIdentity::Hosted {
                    repository: repository.clone(),
                    id: *id,
                } =>
        {
            Ok(CreateDecision::Landed(RepoReady {
                identity: now.clone(),
            }))
        }
        (RepositoryTarget::HostedCreate { .. }, RepositoryObservation::Absent, true, None, _) => {
            Ok(CreateDecision::Uncertain)
        }
        (
            RepositoryTarget::HostedCreate { .. },
            RepositoryObservation::Absent,
            true,
            Some(_),
            RepositoryObservation::Absent,
        ) => Ok(CreateDecision::Uncertain),
        _ => Ok(CreateDecision::RecoveryRequired),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PushCallback {
    NotDispatched,
    ExitZero,
    LostOrFailed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PushDecision {
    NeedPush,
    PreEffectNoOp,
    Landed,
    Uncertain,
    RecoveryRequired,
}

pub(crate) fn classify_push(
    plan: &PackageGitPlan,
    ready: &RepoReady,
    observed_repo: &RepositoryObservation,
    callback: PushCallback,
    observed_refs: &RefSnapshot,
) -> Result<PushDecision, PlanError> {
    plan.validate()?;
    let RepositoryObservation::Present(identity) = observed_repo else {
        return Ok(PushDecision::RecoveryRequired);
    };
    if identity != ready.identity() || !identity_matches_target(&plan.target, identity) {
        return Ok(PushDecision::RecoveryRequired);
    }
    let baseline = &plan.before_refs;
    let is_baseline = observed_refs == baseline;
    let desired = plan.desired_refs();
    let is_desired = desired.as_ref().is_some_and(|refs| observed_refs == refs);
    let unchanged_payload = matches!(&plan.staged.commit,
        StagedCommit::ReusedMain { oid } if raw_oid(&baseline.main) == Some(oid.as_str()));
    let peeled_already_desired = matches!(&baseline.version_tag,
        TagRef::Present { peeled_commit_oid, .. } if peeled_commit_oid == plan.staged.commit.oid());
    if !is_desired && !is_baseline {
        return Ok(PushDecision::RecoveryRequired);
    }
    if desired.is_none() {
        return Ok(
            if callback == PushCallback::NotDispatched
                && is_baseline
                && unchanged_payload
                && peeled_already_desired
            {
                PushDecision::PreEffectNoOp
            } else {
                PushDecision::RecoveryRequired
            },
        );
    }
    Ok(match callback {
        PushCallback::NotDispatched
            if is_baseline && unchanged_payload && peeled_already_desired =>
        {
            PushDecision::PreEffectNoOp
        }
        PushCallback::NotDispatched if is_baseline => PushDecision::NeedPush,
        PushCallback::NotDispatched => PushDecision::RecoveryRequired,
        PushCallback::ExitZero if is_desired => PushDecision::Landed,
        PushCallback::ExitZero => PushDecision::RecoveryRequired,
        PushCallback::LostOrFailed => PushDecision::Uncertain,
    })
}

#[cfg(test)]
#[path = "package_git_plan/tests.rs"]
mod tests;
