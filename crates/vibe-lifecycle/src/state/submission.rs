//! Durable idempotency index under the existing workspace mutation lease.
//!
//! This file is admission metadata only. Its rows point at domain run or
//! journal identities; they never store execution status or a result. A
//! caller may dispatch an effect only after `Created` has been durably
//! returned, while `Existing` and `Retired` are observation/recovery paths.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::LifecycleLease;

const FILE: &str = ".vibe/machine-submissions.json";
const CAP: usize = 1024 * 1024;
const MAX_FIELD: usize = 256;
const MAX_TTL_MS: u64 = 24 * 60 * 60 * 1000;

fn now_ms() -> Result<u64, SubmissionError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SubmissionError::InvalidInput("host clock predates Unix epoch"))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|_| SubmissionError::InvalidInput("host clock is outside supported range"))
}

/// Trusted, surface-supplied identities. These identify the context, target
/// and principal; client request bodies must never construct this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionScope {
    pub context: String,
    pub target: String,
    pub principal: String,
}

/// An owner-issued, finite admission namespace. Its opaque ID is generated
/// by the lifecycle owner, never accepted as a caller-selected value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionNamespace {
    id: String,
}

impl SubmissionNamespace {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Reconstitute a token supplied by a previously trusted host session.
    /// This is only a lookup key: `reserve` also checks trusted scope.
    pub fn from_host_token(id: String) -> Result<Self, SubmissionError> {
        if !valid_namespace_id(&id) {
            return Err(SubmissionError::InvalidInput("namespace token"));
        }
        Ok(Self { id })
    }
}

/// Only `Created` grants permission to start a domain effect. In both other
/// cases the caller must inspect the named existing domain journal/run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationDecision {
    Created { domain_run_id: String },
    Existing { domain_run_id: String },
    Retired { domain_run_id: Option<String> },
}

/// The admission handshake is separate from the domain's execution status.
/// `Ready` means the caller proved a matching durable journal before effects;
/// it never means the effect itself completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalAdmission {
    Pending,
    Ready,
}

/// Read-only answer for a previously submitted request. It is correlation
/// evidence only: neither variant grants effect dispatch or recreates a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExistingSubmissionInspection {
    Absent,
    RetiredUnreserved,
    Reserved {
        domain_run_id: String,
        journal_admission: JournalAdmission,
        reservation_ref: Option<String>,
        retired: bool,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SubmissionError {
    #[error("invalid submission {0}")]
    InvalidInput(&'static str),
    #[error("unknown submission namespace")]
    UnknownNamespace,
    #[error("submission namespace belongs to another context, target or principal")]
    ForeignScope,
    #[error("submission key already names different request or plan bytes")]
    KeyConflict,
    #[error("submission reservation does not match its namespace, key or domain run")]
    ReservationMismatch,
    #[error("submission reservation has no proven durable domain journal")]
    JournalNotReady,
    #[error("domain run `{domain_run_id}` is already reserved by another submission")]
    DomainRunConflict { domain_run_id: String },
    #[error("submission index is malformed or over its size bound: {0}")]
    InvalidIndex(String),
    #[error("submission index I/O refused: {0}")]
    Io(String),
    #[error(
        "submission publication may have succeeded; no effect may start until the index is reread: {0}"
    )]
    UncertainPublication(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: u32,
    next_epoch: u64,
    namespaces: BTreeMap<String, NamespaceRow>,
}

impl Default for Index {
    fn default() -> Self {
        Self {
            schema: 1,
            next_epoch: 1,
            namespaces: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamespaceRow {
    epoch: u64,
    context: String,
    target: String,
    principal: String,
    issued_at_ms: u64,
    observed_at_ms: u64,
    expires_at_ms: u64,
    retired: bool,
    keys: BTreeMap<String, ReservationRow>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReservationRow {
    request_sha256: String,
    plan_sha256: String,
    domain_run_id: String,
    #[serde(default)]
    journal_ready: bool,
}

fn valid_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FIELD
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_namespace_id(id: &str) -> bool {
    let Some((epoch, random)) = id.split_once('-') else {
        return false;
    };
    epoch.len() == 16
        && random.len() == 32
        && epoch.bytes().all(|byte| byte.is_ascii_hexdigit())
        && random.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(lease: &LifecycleLease) -> Result<Index, SubmissionError> {
    let bytes = lease
        .project()
        .read_file_bounded(FILE, CAP)
        .map_err(|error| SubmissionError::Io(format!("{error:#}")))?;
    let Some(bytes) = bytes else {
        return Ok(Index::default());
    };
    let index: Index = serde_json::from_slice(&bytes)
        .map_err(|error| SubmissionError::InvalidIndex(error.to_string()))?;
    if index.schema != 1 || index.next_epoch == 0 {
        return Err(SubmissionError::InvalidIndex(format!(
            "unsupported schema or epoch {} / {}",
            index.schema, index.next_epoch
        )));
    }
    // Never overwrite a syntactically valid but semantically unsafe index.
    let mut domain_run_ids = BTreeSet::new();
    for (id, namespace) in &index.namespaces {
        if !valid_namespace_id(id)
            || u64::from_str_radix(&id[..16], 16).ok() != Some(namespace.epoch)
            || namespace.epoch >= index.next_epoch
            || !valid_field(&namespace.context)
            || !valid_field(&namespace.target)
            || !valid_field(&namespace.principal)
            || namespace.expires_at_ms <= namespace.issued_at_ms
            || namespace.observed_at_ms < namespace.issued_at_ms
            || namespace.keys.iter().any(|(key, row)| {
                !valid_field(key)
                    || !valid_field(&row.domain_run_id)
                    || row.request_sha256.len() != 64
                    || row.plan_sha256.len() != 64
                    || !row
                        .request_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                    || !row.plan_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(SubmissionError::InvalidIndex(
                "invalid namespace or reservation row".into(),
            ));
        }
        for reservation in namespace.keys.values() {
            if !domain_run_ids.insert(reservation.domain_run_id.clone()) {
                return Err(SubmissionError::InvalidIndex(
                    "one domain run ID is reserved by more than one submission".into(),
                ));
            }
        }
    }
    Ok(index)
}

fn publish(lease: &LifecycleLease, index: &Index) -> Result<(), SubmissionError> {
    let bytes = serde_json::to_vec(index)
        .map_err(|error| SubmissionError::InvalidIndex(error.to_string()))?;
    if bytes.len() > CAP {
        return Err(SubmissionError::InvalidIndex(format!(
            "candidate {} bytes exceeds {CAP}",
            bytes.len()
        )));
    }
    lease
        .project()
        .write_atomic(FILE, &bytes)
        .map(|_| ())
        .map_err(|error| {
            let possibly = error.stage == vibe_safefs::PublishStage::PossiblyPublished;
            let reason = format!("{:#}", error.into_report());
            if possibly {
                SubmissionError::UncertainPublication(reason)
            } else {
                SubmissionError::Io(reason)
            }
        })
}

/// Admission index operations require the same live outer lease as the domain
/// lifecycle. No method opens another lock or stores a competing result.
pub struct SubmissionReservations;

impl SubmissionReservations {
    /// Find an existing reservation by its original canonical request before
    /// inspecting current product files. The stored plan digest is used only
    /// inside the reservation ref; current post-effect files cannot become a
    /// replacement plan. This never mutates the index or admits an effect.
    pub fn inspect_existing_request(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        canonical_request: &[u8],
    ) -> Result<ExistingSubmissionInspection, SubmissionError> {
        Self::inspect_existing_request_at(
            lease,
            namespace,
            scope,
            now_ms()?,
            key,
            canonical_request,
        )
    }

    fn inspect_existing_request_at(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        now_ms: u64,
        key: &str,
        canonical_request: &[u8],
    ) -> Result<ExistingSubmissionInspection, SubmissionError> {
        if !valid_field(key) || canonical_request.len() > CAP {
            return Err(SubmissionError::InvalidInput(
                "key or request payload bound",
            ));
        }
        let index = read(lease)?;
        let owner = index
            .namespaces
            .get(namespace.id())
            .ok_or(SubmissionError::UnknownNamespace)?;
        if owner.context != scope.context
            || owner.target != scope.target
            || owner.principal != scope.principal
        {
            return Err(SubmissionError::ForeignScope);
        }
        if now_ms < owner.observed_at_ms {
            return Err(SubmissionError::InvalidInput("host clock rolled back"));
        }
        let retired = owner.retired || now_ms >= owner.expires_at_ms;
        let Some(row) = owner.keys.get(key) else {
            return Ok(if retired {
                ExistingSubmissionInspection::RetiredUnreserved
            } else {
                ExistingSubmissionInspection::Absent
            });
        };
        if row.request_sha256 != digest(canonical_request) {
            return Err(SubmissionError::KeyConflict);
        }
        Ok(ExistingSubmissionInspection::Reserved {
            domain_run_id: row.domain_run_id.clone(),
            journal_admission: if row.journal_ready {
                JournalAdmission::Ready
            } else {
                JournalAdmission::Pending
            },
            reservation_ref: row
                .journal_ready
                .then(|| reservation_ref_of(namespace.id(), owner, key, row)),
            retired,
        })
    }

    /// Issue a finite namespace after a host has authenticated its scope.
    /// The lifecycle owner reads host wall time and caps the requested TTL.
    /// A wall-clock jump between unobserved calls cannot be proven away;
    /// observed rollback is detected from the durable watermark.
    pub fn issue_namespace(
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        ttl_ms: u64,
    ) -> Result<SubmissionNamespace, SubmissionError> {
        Self::issue_at(lease, scope, now_ms()?, ttl_ms)
    }

    fn issue_at(
        lease: &LifecycleLease,
        scope: &SubmissionScope,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<SubmissionNamespace, SubmissionError> {
        if !valid_field(&scope.context)
            || !valid_field(&scope.target)
            || !valid_field(&scope.principal)
            || ttl_ms == 0
            || ttl_ms > MAX_TTL_MS
        {
            return Err(SubmissionError::InvalidInput("scope or expiry"));
        }
        let expires_at_ms = now_ms
            .checked_add(ttl_ms)
            .ok_or(SubmissionError::InvalidInput("expiry overflow"))?;
        let mut index = read(lease)?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| SubmissionError::Io(error.to_string()))?;
        let epoch = index.next_epoch;
        index.next_epoch = epoch.checked_add(1).ok_or(SubmissionError::InvalidIndex(
            "namespace epochs exhausted".into(),
        ))?;
        let id = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let id = format!("{epoch:016x}-{id}");
        if index.namespaces.contains_key(&id) {
            return Err(SubmissionError::InvalidIndex(
                "namespace ID collision".into(),
            ));
        }
        index.namespaces.insert(
            id.clone(),
            NamespaceRow {
                epoch,
                context: scope.context.clone(),
                target: scope.target.clone(),
                principal: scope.principal.clone(),
                issued_at_ms: now_ms,
                observed_at_ms: now_ms,
                expires_at_ms,
                retired: false,
                keys: BTreeMap::new(),
            },
        );
        publish(lease, &index)?;
        Ok(SubmissionNamespace { id })
    }

    /// Reserve canonical request and plan *bytes* before effects. The domain
    /// run ID is already selected by its owner and is only indexed here.
    pub fn reserve(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        request: &[u8],
        plan: &[u8],
        domain_run_id: &str,
    ) -> Result<ReservationDecision, SubmissionError> {
        Self::reserve_at(
            lease,
            namespace,
            scope,
            now_ms()?,
            key,
            request,
            plan,
            domain_run_id,
        )
    }

    fn reserve_at(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        now_ms: u64,
        key: &str,
        request: &[u8],
        plan: &[u8],
        domain_run_id: &str,
    ) -> Result<ReservationDecision, SubmissionError> {
        if !valid_field(key)
            || !valid_field(domain_run_id)
            || request.len() > CAP
            || plan.len() > CAP
        {
            return Err(SubmissionError::InvalidInput(
                "key, domain identity or payload bound",
            ));
        }
        let mut index = read(lease)?;
        let run_id_reserved = index.namespaces.values().any(|namespace| {
            namespace
                .keys
                .values()
                .any(|reservation| reservation.domain_run_id == domain_run_id)
        });
        let row = index
            .namespaces
            .get_mut(namespace.id())
            .ok_or(SubmissionError::UnknownNamespace)?;
        if row.context != scope.context
            || row.target != scope.target
            || row.principal != scope.principal
        {
            return Err(SubmissionError::ForeignScope);
        }
        if now_ms < row.observed_at_ms {
            return Err(SubmissionError::InvalidInput("host clock rolled back"));
        }
        let must_publish =
            now_ms > row.observed_at_ms || (now_ms >= row.expires_at_ms && !row.retired);
        row.observed_at_ms = now_ms;
        if now_ms >= row.expires_at_ms {
            row.retired = true;
        }
        let request_sha256 = digest(request);
        let plan_sha256 = digest(plan);
        if let Some(prior) = row.keys.get(key) {
            if prior.request_sha256 != request_sha256 || prior.plan_sha256 != plan_sha256 {
                // A conflict cannot erase the observed expiry. Persist the
                // watermark/tombstone before returning its typed refusal.
                if must_publish {
                    publish(lease, &index)?;
                }
                return Err(SubmissionError::KeyConflict);
            }
            let decision = if row.retired {
                ReservationDecision::Retired {
                    domain_run_id: Some(prior.domain_run_id.clone()),
                }
            } else {
                ReservationDecision::Existing {
                    domain_run_id: prior.domain_run_id.clone(),
                }
            };
            if must_publish {
                publish(lease, &index)?;
            }
            return Ok(decision);
        }
        if row.retired {
            if must_publish {
                publish(lease, &index)?;
            }
            return Ok(ReservationDecision::Retired {
                domain_run_id: None,
            });
        }
        if run_id_reserved {
            // The lease serializes this check with the insertion. Preserve
            // any observed clock watermark before refusing a second owner.
            if must_publish {
                publish(lease, &index)?;
            }
            return Err(SubmissionError::DomainRunConflict {
                domain_run_id: domain_run_id.to_owned(),
            });
        }
        row.keys.insert(
            key.to_owned(),
            ReservationRow {
                request_sha256,
                plan_sha256,
                domain_run_id: domain_run_id.to_owned(),
                journal_ready: false,
            },
        );
        publish(lease, &index)?;
        Ok(ReservationDecision::Created {
            domain_run_id: domain_run_id.to_owned(),
        })
    }

    /// Inspect the pre-effect journal handshake for an already validated
    /// reservation. The owning service must first call `reserve` with the
    /// canonical payload, so a client cannot use this lookup as admission.
    pub fn journal_admission(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        domain_run_id: &str,
    ) -> Result<JournalAdmission, SubmissionError> {
        let index = read(lease)?;
        let row = reservation(&index, namespace, scope, key, domain_run_id)?;
        Ok(if row.journal_ready {
            JournalAdmission::Ready
        } else {
            JournalAdmission::Pending
        })
    }

    /// Read the original reserved plan digest for a matched domain run.
    /// This is correlation evidence for a domain-owned intent, never an
    /// effect grant or a substitute for inspecting that owner's journal.
    pub fn inspect_reserved_plan_sha256(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        domain_run_id: &str,
    ) -> Result<String, SubmissionError> {
        let index = read(lease)?;
        let row = reservation(&index, namespace, scope, key, domain_run_id)?;
        Ok(format!("sha256:{}", row.plan_sha256))
    }

    /// Stable correlation with one exact, journal-ready reservation. This
    /// value carries no effect permission: the domain owner must still prove
    /// its own journal and outcome. Retired rows remain readable so a lost
    /// response cannot be relabeled after namespace expiry or collection.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SUBMISSION-RESERVATION",
        r = 6
    )]
    pub fn reservation_ref(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        domain_run_id: &str,
    ) -> Result<String, SubmissionError> {
        let index = read(lease)?;
        let reserved = reservation(&index, namespace, scope, key, domain_run_id)?;
        if !reserved.journal_ready {
            return Err(SubmissionError::JournalNotReady);
        }
        let owner = index
            .namespaces
            .get(namespace.id())
            .ok_or(SubmissionError::UnknownNamespace)?;
        Ok(reservation_ref_of(namespace.id(), owner, key, reserved))
    }

    /// Record that the owner has re-read and proven the exact durable domain
    /// journal. Publication uncertainty refuses effect dispatch; a retry
    /// reconciles the index and journal under the same outer lease.
    pub fn mark_journal_ready(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
        key: &str,
        domain_run_id: &str,
    ) -> Result<(), SubmissionError> {
        let mut index = read(lease)?;
        let prior = reservation(&index, namespace, scope, key, domain_run_id)?;
        if prior.journal_ready {
            return Ok(());
        }
        index
            .namespaces
            .get_mut(namespace.id())
            .unwrap()
            .keys
            .get_mut(key)
            .unwrap()
            .journal_ready = true;
        publish(lease, &index)
    }

    /// Retire without deleting any compact reservation identity. A retired
    /// token can never be renewed or reused for a new dispatch.
    pub fn retire_namespace(
        lease: &LifecycleLease,
        namespace: &SubmissionNamespace,
        scope: &SubmissionScope,
    ) -> Result<(), SubmissionError> {
        let mut index = read(lease)?;
        let row = index
            .namespaces
            .get_mut(namespace.id())
            .ok_or(SubmissionError::UnknownNamespace)?;
        if row.context != scope.context
            || row.target != scope.target
            || row.principal != scope.principal
        {
            return Err(SubmissionError::ForeignScope);
        }
        if !row.retired {
            row.retired = true;
            publish(lease, &index)?;
        }
        Ok(())
    }
}

fn reservation_ref_of(
    namespace_id: &str,
    owner: &NamespaceRow,
    key: &str,
    reserved: &ReservationRow,
) -> String {
    let mut hash = Sha256::new();
    for (tag, value) in [
        ("domain", "vibevm/submission-reservation-ref/e1"),
        ("namespace", namespace_id),
        ("context", owner.context.as_str()),
        ("target", owner.target.as_str()),
        ("principal", owner.principal.as_str()),
        ("key", key),
        ("request_sha256", reserved.request_sha256.as_str()),
        ("plan_sha256", reserved.plan_sha256.as_str()),
        ("domain_run_id", reserved.domain_run_id.as_str()),
    ] {
        hash.update((tag.len() as u16).to_be_bytes());
        hash.update(tag.as_bytes());
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("sha256:{:x}", hash.finalize())
}

fn reservation<'a>(
    index: &'a Index,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    domain_run_id: &str,
) -> Result<&'a ReservationRow, SubmissionError> {
    let namespace = index
        .namespaces
        .get(namespace.id())
        .ok_or(SubmissionError::UnknownNamespace)?;
    if namespace.context != scope.context
        || namespace.target != scope.target
        || namespace.principal != scope.principal
    {
        return Err(SubmissionError::ForeignScope);
    }
    let row = namespace
        .keys
        .get(key)
        .ok_or(SubmissionError::ReservationMismatch)?;
    if row.domain_run_id != domain_run_id {
        return Err(SubmissionError::ReservationMismatch);
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> SubmissionScope {
        SubmissionScope {
            context: "ctx".into(),
            target: "target".into(),
            principal: "principal".into(),
        }
    }

    #[test]
    fn existing_request_inspection_binds_request_and_never_mutates_index() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        assert_eq!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &scope(),
                11,
                "key",
                b"request"
            )
            .unwrap(),
            ExistingSubmissionInspection::Absent
        );
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"original plan",
            "run-a",
        )
        .unwrap();
        let before = std::fs::read(root.path().join(FILE)).unwrap();
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease, &namespace, &scope(), 12, "key", b"request"
            )
            .unwrap(),
            ExistingSubmissionInspection::Reserved {
                domain_run_id,
                journal_admission: JournalAdmission::Pending,
                reservation_ref: None,
                retired: false,
            } if domain_run_id == "run-a"
        ));
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &scope(),
                12,
                "key",
                b"changed"
            ),
            Err(SubmissionError::KeyConflict)
        ));
        let foreign = SubmissionScope {
            principal: "other".into(),
            ..scope()
        };
        assert!(matches!(
            SubmissionReservations::inspect_existing_request_at(
                &lease, &namespace, &foreign, 12, "key", b"request"
            ),
            Err(SubmissionError::ForeignScope)
        ));
        assert_eq!(std::fs::read(root.path().join(FILE)).unwrap(), before);

        SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope(), "key", "run-a")
            .unwrap();
        let ready_before = std::fs::read(root.path().join(FILE)).unwrap();
        let expected_ref =
            SubmissionReservations::reservation_ref(&lease, &namespace, &scope(), "key", "run-a")
                .unwrap();
        assert_eq!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &scope(),
                21,
                "key",
                b"request"
            )
            .unwrap(),
            ExistingSubmissionInspection::Reserved {
                domain_run_id: "run-a".into(),
                journal_admission: JournalAdmission::Ready,
                reservation_ref: Some(expected_ref),
                retired: true,
            }
        );
        assert_eq!(
            SubmissionReservations::inspect_existing_request_at(
                &lease,
                &namespace,
                &scope(),
                21,
                "new-key",
                b"request"
            )
            .unwrap(),
            ExistingSubmissionInspection::RetiredUnreserved
        );
        assert_eq!(std::fs::read(root.path().join(FILE)).unwrap(), ready_before);
    }

    #[test]
    fn retry_after_response_loss_attaches_to_durable_run() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        let first = SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        assert_eq!(
            first,
            ReservationDecision::Created {
                domain_run_id: "run-a".into()
            }
        );
        drop(lease);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionNamespace::from_host_token(namespace.id().into()).unwrap();
        let retry = SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            12,
            "key",
            b"request",
            b"plan",
            "run-b",
        )
        .unwrap();
        assert_eq!(
            retry,
            ReservationDecision::Existing {
                domain_run_id: "run-a".into()
            }
        );
    }

    #[test]
    fn same_key_with_different_payload_or_plan_refuses() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        for (request, plan) in [
            (b"other".as_slice(), b"plan".as_slice()),
            (b"request", b"other"),
        ] {
            assert!(matches!(
                SubmissionReservations::reserve_at(
                    &lease,
                    &namespace,
                    &scope(),
                    12,
                    "key",
                    request,
                    plan,
                    "run-b"
                ),
                Err(SubmissionError::KeyConflict)
            ));
        }
    }

    #[test]
    fn different_key_cannot_reuse_reserved_run_before_journal_exists() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                11,
                "key-a",
                b"request-a",
                b"plan-a",
                "run-a",
            ),
            Ok(ReservationDecision::Created { .. })
        ));
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease, &namespace, &scope(), 12, "key-b", b"request-b", b"plan-b", "run-a",
            ),
            Err(SubmissionError::DomainRunConflict { domain_run_id }) if domain_run_id == "run-a"
        ));
        let index = read(&lease).unwrap();
        assert_eq!(index.namespaces[namespace.id()].keys.len(), 1);
        assert!(!index.namespaces[namespace.id()].keys["key-a"].journal_ready);
    }

    #[test]
    fn post_publish_reservation_fault_never_returns_created_and_retry_keeps_run() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        vibe_safefs::fail_after_publish(Some(FILE));
        let first = SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        );
        vibe_safefs::fail_after_publish(None);
        assert!(matches!(
            first,
            Err(SubmissionError::UncertainPublication(_))
        ));
        let index = read(&lease).unwrap();
        assert_eq!(
            index.namespaces[namespace.id()].keys["key"].domain_run_id,
            "run-a"
        );
        assert!(!index.namespaces[namespace.id()].keys["key"].journal_ready);
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                12,
                "key",
                b"request",
                b"plan",
                "run-b",
            )
            .unwrap(),
            ReservationDecision::Existing {
                domain_run_id: "run-a".into()
            }
        );
    }

    #[test]
    fn post_publish_ready_fault_never_claims_success_and_retry_reconciles_marker() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        vibe_safefs::fail_after_publish(Some(FILE));
        let ready = SubmissionReservations::mark_journal_ready(
            &lease,
            &namespace,
            &scope(),
            "key",
            "run-a",
        );
        vibe_safefs::fail_after_publish(None);
        assert!(matches!(
            ready,
            Err(SubmissionError::UncertainPublication(_))
        ));
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                12,
                "key",
                b"request",
                b"plan",
                "run-b",
            )
            .unwrap(),
            ReservationDecision::Existing {
                domain_run_id: "run-a".into()
            }
        );
        assert_eq!(
            SubmissionReservations::journal_admission(&lease, &namespace, &scope(), "key", "run-a")
                .unwrap(),
            JournalAdmission::Ready
        );
        SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope(), "key", "run-a")
            .unwrap();
        assert_eq!(
            read(&lease).unwrap().namespaces[namespace.id()].keys.len(),
            1
        );
    }

    #[test]
    fn duplicate_persisted_run_id_refuses_index_read() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let first = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        let second = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &first,
            &scope(),
            11,
            "key-a",
            b"request-a",
            b"plan-a",
            "run-a",
        )
        .unwrap();
        let mut index = read(&lease).unwrap();
        index.namespaces.get_mut(second.id()).unwrap().keys.insert(
            "key-b".into(),
            ReservationRow {
                request_sha256: digest(b"request-b"),
                plan_sha256: digest(b"plan-b"),
                domain_run_id: "run-a".into(),
                journal_ready: false,
            },
        );
        publish(&lease, &index).unwrap();
        assert!(matches!(
            read(&lease),
            Err(SubmissionError::InvalidIndex(reason)) if reason.contains("more than one submission")
        ));
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &first,
                &scope(),
                12,
                "key-a",
                b"request-a",
                b"plan-a",
                "run-b",
            ),
            Err(SubmissionError::InvalidIndex(_))
        ));
    }

    #[test]
    fn valid_looking_unissued_token_cannot_reserve_or_renew_namespace() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let forged = SubmissionNamespace::from_host_token(
            "0000000000000001-00000000000000000000000000000000".into(),
        )
        .unwrap();
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &forged,
                &scope(),
                11,
                "key",
                b"request",
                b"plan",
                "run-a",
            ),
            Err(SubmissionError::UnknownNamespace)
        ));
        let issued = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        assert_eq!(&issued.id()[..16], "0000000000000001");
        assert_ne!(issued.id(), forged.id());
    }

    #[test]
    fn expiry_and_retirement_preserve_tombstone_without_dispatch() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                20,
                "key",
                b"request",
                b"plan",
                "run-b"
            )
            .unwrap(),
            ReservationDecision::Retired {
                domain_run_id: Some("run-a".into())
            }
        );
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                12,
                "new",
                b"request",
                b"plan",
                "run-b"
            )
            .unwrap_err()
            .to_string(),
            SubmissionError::InvalidInput("host clock rolled back").to_string()
        );
        // The earlier observed expiry was written, so rewinding the injected
        // clock does not permit a fresh key to dispatch.
        SubmissionReservations::retire_namespace(&lease, &namespace, &scope()).unwrap();
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                21,
                "new",
                b"request",
                b"plan",
                "run-b"
            )
            .unwrap(),
            ReservationDecision::Retired {
                domain_run_id: None
            }
        );
    }

    #[test]
    fn expired_conflicting_key_persists_retirement_before_refusal() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                20,
                "key",
                b"other",
                b"plan",
                "run-b",
            ),
            Err(SubmissionError::KeyConflict)
        ));
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                12,
                "new",
                b"request",
                b"plan",
                "run-b",
            ),
            Err(SubmissionError::InvalidInput("host clock rolled back"))
        ));
        assert_eq!(
            SubmissionReservations::reserve_at(
                &lease,
                &namespace,
                &scope(),
                20,
                "new",
                b"request",
                b"plan",
                "run-b",
            )
            .unwrap(),
            ReservationDecision::Retired {
                domain_run_id: None
            }
        );
    }

    #[test]
    fn owner_assigns_distinct_epochs_and_rejects_excessive_ttl() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        assert!(matches!(
            SubmissionReservations::issue_at(&lease, &scope(), 10, MAX_TTL_MS + 1),
            Err(SubmissionError::InvalidInput(_))
        ));
        let first = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        let second = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        assert_eq!(&first.id()[..16], "0000000000000001");
        assert_eq!(&second.id()[..16], "0000000000000002");
        let other_scope = SubmissionScope {
            principal: "other".into(),
            ..scope()
        };
        assert!(matches!(
            SubmissionReservations::reserve_at(
                &lease,
                &first,
                &other_scope,
                11,
                "key",
                b"r",
                b"p",
                "run"
            ),
            Err(SubmissionError::ForeignScope)
        ));
    }

    #[test]
    fn journal_ready_marker_requires_matching_reserved_run_and_scope() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        let foreign = SubmissionScope {
            principal: "other".into(),
            ..scope()
        };
        assert!(matches!(
            SubmissionReservations::mark_journal_ready(
                &lease,
                &namespace,
                &scope(),
                "key",
                "run-b",
            ),
            Err(SubmissionError::ReservationMismatch)
        ));
        assert!(matches!(
            SubmissionReservations::mark_journal_ready(
                &lease, &namespace, &foreign, "key", "run-a",
            ),
            Err(SubmissionError::ForeignScope)
        ));
        assert_eq!(
            SubmissionReservations::journal_admission(
                &lease, &namespace, &scope(), "key", "run-a",
            )
            .unwrap(),
            JournalAdmission::Pending
        );
    }

    #[test]
    fn reservation_ref_is_stable_after_retry_and_retirement_without_index_write() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope(), "key", "run-a")
            .unwrap();
        SubmissionReservations::retire_namespace(&lease, &namespace, &scope()).unwrap();
        let before = lease
            .project()
            .read_file_bounded(FILE, CAP)
            .unwrap()
            .unwrap();
        let first =
            SubmissionReservations::reservation_ref(&lease, &namespace, &scope(), "key", "run-a")
                .unwrap();
        assert_eq!(first.len(), 71);
        assert!(
            first
                .strip_prefix("sha256:")
                .unwrap()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        );
        assert_eq!(
            lease
                .project()
                .read_file_bounded(FILE, CAP)
                .unwrap()
                .unwrap(),
            before
        );
        drop(lease);
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let restored = SubmissionNamespace::from_host_token(namespace.id().into()).unwrap();
        let retry =
            SubmissionReservations::reservation_ref(&lease, &restored, &scope(), "key", "run-a")
                .unwrap();
        assert_eq!(retry, first);
        assert_eq!(
            lease
                .project()
                .read_file_bounded(FILE, CAP)
                .unwrap()
                .unwrap(),
            before
        );
    }

    #[test]
    fn reservation_ref_refuses_unready_and_foreign_coordinates_without_index_write() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        let other_namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        SubmissionReservations::reserve_at(
            &lease,
            &namespace,
            &scope(),
            11,
            "key",
            b"request",
            b"plan",
            "run-a",
        )
        .unwrap();
        let before = lease
            .project()
            .read_file_bounded(FILE, CAP)
            .unwrap()
            .unwrap();
        assert!(matches!(
            SubmissionReservations::reservation_ref(&lease, &namespace, &scope(), "key", "run-a"),
            Err(SubmissionError::JournalNotReady)
        ));
        SubmissionReservations::mark_journal_ready(&lease, &namespace, &scope(), "key", "run-a")
            .unwrap();
        let ready_bytes = lease
            .project()
            .read_file_bounded(FILE, CAP)
            .unwrap()
            .unwrap();
        let foreign = SubmissionScope {
            principal: "foreign".into(),
            ..scope()
        };
        assert!(matches!(
            SubmissionReservations::reservation_ref(&lease, &namespace, &foreign, "key", "run-a"),
            Err(SubmissionError::ForeignScope)
        ));
        for (token, key, run) in [
            (&namespace, "foreign-key", "run-a"),
            (&namespace, "key", "run-b"),
            (&other_namespace, "key", "run-a"),
        ] {
            assert!(matches!(
                SubmissionReservations::reservation_ref(&lease, token, &scope(), key, run),
                Err(SubmissionError::ReservationMismatch)
            ));
        }
        let unissued = SubmissionNamespace::from_host_token(
            "0000000000000003-00000000000000000000000000000000".into(),
        )
        .unwrap();
        assert!(matches!(
            SubmissionReservations::reservation_ref(&lease, &unissued, &scope(), "key", "run-a"),
            Err(SubmissionError::UnknownNamespace)
        ));
        assert_ne!(
            before, ready_bytes,
            "only mark_journal_ready writes the index"
        );
        assert_eq!(
            lease
                .project()
                .read_file_bounded(FILE, CAP)
                .unwrap()
                .unwrap(),
            ready_bytes
        );
    }

    #[test]
    fn simultaneous_same_key_has_one_created_decision() {
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_at(&lease, &scope(), 10, 10).unwrap();
        drop(lease);
        let created = std::sync::atomic::AtomicUsize::new(0);
        // The real cross-process lease permits one mutation epoch at a time.
        // Contending peers retry Busy and then observe the durable pointer.
        std::thread::scope(|threads| {
            for i in 0..8 {
                let namespace = namespace.clone();
                let created = &created;
                let path = root.path().to_path_buf();
                threads.spawn(move || {
                    loop {
                        match LifecycleLease::acquire(&path) {
                            Ok(lease) => {
                                let result = SubmissionReservations::reserve_at(
                                    &lease,
                                    &namespace,
                                    &scope(),
                                    11,
                                    "key",
                                    b"request",
                                    b"plan",
                                    &format!("run-{i}"),
                                )
                                .unwrap();
                                if matches!(result, ReservationDecision::Created { .. }) {
                                    created.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                }
                                break;
                            }
                            Err(crate::LifecycleLeaseError::Busy { .. }) => {
                                std::thread::yield_now()
                            }
                            Err(error) => panic!("unexpected lease error: {error}"),
                        }
                    }
                });
            }
        });
        assert_eq!(created.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
#[path = "submission/lifetime_tests.rs"]
mod lifetime_tests;
