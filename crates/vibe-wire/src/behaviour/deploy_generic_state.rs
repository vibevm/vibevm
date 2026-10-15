//! Closed, payload-free generic deployment sidecar validation.
//! Historical grant subjects are audit data; this reader issues no authority.
use crate::generated::deploy_generic_state::*;
use sha2::{Digest, Sha256};

pub const RESOURCE_LIMIT: usize = 4096;
pub const BLOB_LIMIT: u32 = 4 * 1024 * 1024;
pub const RETENTION_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid generic deployment state: {0}")]
pub struct StateError(pub &'static str);

fn check(ok: bool, reason: &'static str) -> Result<(), StateError> {
    if ok { Ok(()) } else { Err(StateError(reason)) }
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn path(s: &str) -> bool {
    text(s)
        && !s.contains(['\\', ':'])
        && s.split('/').all(|x| !x.is_empty() && x != "." && x != "..")
}
fn blob(b: &Blob, total: &mut u64) -> Result<(), StateError> {
    check(
        hash(&b.digest) && b.bytes <= BLOB_LIMIT && b.mode <= 0o777,
        "blob",
    )?;
    *total = total
        .checked_add(u64::from(b.bytes))
        .ok_or(StateError("retention overflow"))?;
    check(*total <= RETENTION_LIMIT, "retention budget")
}
/// Canonical semantic identity excludes progress, phase and historical grant.
/// Tuple framing and generated field order remain stable across processes.
pub fn semantics_digest(b: &Binding) -> Result<String, StateError> {
    let resources = b
        .resources
        .iter()
        .map(|r| {
            (
                &r.scope,
                &r.destination,
                &r.key,
                &r.operation,
                &r.desired,
                &r.before,
            )
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(
        "deploy-generic-semantics/1",
        &b.semantics,
        &b.pins,
        resources,
        b.transfer.as_ref().map(|t| {
            (
                &t.family,
                &t.donor_id,
                &t.donor_root_identity,
                &t.receipt_digest,
                &t.hold_id,
                &t.resources,
            )
        }),
    ))
    .map_err(|_| StateError("semantic encoding"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn validate(state: &DeployGenericState) -> Result<(), StateError> {
    check(state.schema == 1, "epoch")?;
    check(
        state.committed.is_some() || state.pending.is_some(),
        "empty state",
    )?;
    let mut total = 0;
    for b in [state.committed.as_ref(), state.pending.as_ref()]
        .into_iter()
        .flatten()
    {
        check(
            hash(&b.deployment) && hash(&b.plan_hash) && hash(&b.grant_subject),
            "join identity",
        )?;
        check(text(&b.coordination_identity), "coordination identity")?;
        check(
            b.semantics_digest == semantics_digest(b)?,
            "semantics digest",
        )?;
        let p = &b.pins;
        check(
            [
                &p.provider,
                &p.provider_version,
                &p.declaration,
                &p.descriptor,
                &p.source,
            ]
            .into_iter()
            .all(|s| text(s)),
            "source identity",
        )?;
        check(
            [
                &p.provider_digest,
                &p.declaration_digest,
                &p.descriptor_digest,
                &p.source_digest,
            ]
            .into_iter()
            .all(|s| hash(s)),
            "source digest",
        )?;
        check(b.resources.len() <= RESOURCE_LIMIT, "resource bound")?;
        if let Some(d) = &b.receipt_digest {
            check(hash(d), "receipt digest")?;
        }
        if matches!(b.phase, BindingPhase::Committed | BindingPhase::Receipt) {
            check(b.receipt_digest.is_some(), "missing receipt join")?;
            check(
                b.resources
                    .iter()
                    .all(|r| matches!(r.progress, ResourceProgress::Completed)),
                "incomplete published receipt",
            )?;
        }
        if b.revoked {
            check(
                !matches!(
                    b.operation,
                    BindingOperation::Apply
                        | BindingOperation::Upgrade
                        | BindingOperation::Recovery
                        | BindingOperation::Adoption
                ),
                "revoked forward operation",
            )?;
        }
        for (i, r) in b.resources.iter().enumerate() {
            check(path(&r.destination), "relative destination")?;
            check(
                r.key.len() <= 64 && r.key.iter().all(|s| text(s)),
                "literal key",
            )?;
            check(
                matches!(r.operation, ResourceOperation::File) == r.key.is_empty(),
                "operation/key shape",
            )?;
            for old in &b.resources[..i] {
                if old.scope != r.scope {
                    continue;
                }
                let same = old.destination == r.destination;
                let ancestor = old.destination.starts_with(&(r.destination.clone() + "/"))
                    || r.destination.starts_with(&(old.destination.clone() + "/"));
                check(
                    !ancestor
                        && !(same && (old.key.starts_with(&r.key) || r.key.starts_with(&old.key))),
                    "overlapping resource",
                )?;
                if same {
                    check(old.operation == r.operation, "mixed document semantics")?;
                }
            }
            if let Value::Present(v) = &r.desired {
                blob(&v.blob, &mut total)?;
            }
            if let Before::Retained(v) = &r.before {
                blob(&v.blob, &mut total)?;
            }
        }
        if let Some(t) = &b.transfer {
            check(
                text(&t.donor_id)
                    && text(&t.donor_root_identity)
                    && hash(&t.receipt_digest)
                    && hash(&t.hold_id),
                "transfer identity",
            )?;
            check(
                t.receipt_blob.digest == t.receipt_digest,
                "donor receipt join",
            )?;
            blob(&t.receipt_blob, &mut total)?;
            let mut donor = b.clone();
            donor.transfer = None;
            donor.resources = t.resources.clone();
            donor.phase = BindingPhase::Prepared;
            donor.receipt_digest = None;
            donor.revoked = false;
            donor.semantics_digest = semantics_digest(&donor)?;
            validate(&DeployGenericState {
                schema: 1,
                committed: None,
                pending: Some(donor),
            })?;
            check(t.resources.len() <= RESOURCE_LIMIT, "donor resource bound")?;
            for r in &t.resources {
                check(
                    path(&r.destination) && r.key.iter().all(|s| text(s)),
                    "donor resource path/key",
                )?;
                match &r.desired {
                    Value::Present(v) => blob(&v.blob, &mut total)?,
                    Value::Absent(_) => return Err(StateError("absent donor ownership")),
                }
            }

            check(
                matches!(
                    b.operation,
                    BindingOperation::Adoption
                        | BindingOperation::Inverse
                        | BindingOperation::Recovery
                        | BindingOperation::Revocation
                        | BindingOperation::Cleanup
                        | BindingOperation::Upgrade
                ),
                "transfer operation",
            )?;
        }
    }
    if let Some(c) = &state.committed {
        check(
            matches!(c.phase, BindingPhase::Committed),
            "committed phase",
        )?;
    }
    if let Some(p) = &state.pending {
        check(!matches!(p.phase, BindingPhase::Committed), "pending phase")?;
        if let Some(c) = &state.committed {
            check(
                c.deployment == p.deployment
                    && c.coordination_identity == p.coordination_identity
                    && c.generation.checked_add(1) == Some(p.generation),
                "changed join",
            )?;
        }
    }
    Ok(())
}

/// The caller supplies the independently read receipt identity. A valid shape
/// alone cannot join a sidecar to a different deployment or receipt.
pub fn validate_receipt_join(
    state: &DeployGenericState,
    deployment: &str,
    generation: u32,
    receipt_digest: &str,
    plan_hash: &str,
) -> Result<(), StateError> {
    validate(state)?;
    let b = state
        .committed
        .as_ref()
        .ok_or(StateError("no committed state"))?;
    check(
        b.deployment == deployment
            && b.generation == generation
            && b.receipt_digest.as_deref() == Some(receipt_digest)
            && b.plan_hash == plan_hash,
        "receipt join",
    )
}

#[cfg(test)]
mod m_08_c_3_cases {
    use super::*;
    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../formats/corpora/deploy-generic-state/e1")
                .join(name),
        )
        .unwrap()
    }
    fn committed() -> DeployGenericState {
        serde_json::from_str(&fixture("valid/committed.json")).unwrap()
    }
    fn rehash(s: &mut DeployGenericState) {
        for b in [s.committed.as_mut(), s.pending.as_mut()]
            .into_iter()
            .flatten()
        {
            b.semantics_digest = semantics_digest(b).unwrap();
        }
    }
    #[test]
    fn strict_corpus_roundtrip() {
        for name in [
            "valid/committed.json",
            "valid/pending-removal.json",
            "valid/transfer.json",
        ] {
            let s: DeployGenericState = serde_json::from_str(&fixture(name)).unwrap();
            validate(&s).unwrap();
            let encoded = serde_json::to_vec(&s).unwrap();
            let read: DeployGenericState = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(s, read);
            let mut v: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
            v["unexpected"] = true.into();
            assert!(serde_json::from_value::<DeployGenericState>(v).is_err());
        }
        let mut nested: serde_json::Value =
            serde_json::from_str(&fixture("valid/committed.json")).unwrap();
        nested["committed"]["resources"][0]["before"]["payload"] = "must-not-be-inline".into();
        assert!(serde_json::from_value::<DeployGenericState>(nested).is_err());
        let duplicate = fixture("valid/committed.json").replacen(
            "\"schema\": 1",
            "\"schema\": 1, \"schema\": 1",
            1,
        );
        assert!(serde_json::from_str::<DeployGenericState>(&duplicate).is_err());
        assert!(
            serde_json::from_str::<DeployGenericState>(&fixture("invalid/unknown-semantics.json"))
                .is_err()
        );
        let s: DeployGenericState =
            serde_json::from_str(&fixture("invalid/changed-join.json")).unwrap();
        assert!(validate(&s).is_err());
    }
    #[test]
    fn transfer_root_is_required_and_bound_to_semantics() {
        let mut raw: serde_json::Value =
            serde_json::from_str(&fixture("valid/transfer.json")).unwrap();
        raw["pending"]["transfer"]
            .as_object_mut()
            .unwrap()
            .remove("donor_root_identity");
        assert!(serde_json::from_value::<DeployGenericState>(raw).is_err());
        let mut state: DeployGenericState =
            serde_json::from_str(&fixture("valid/transfer.json")).unwrap();
        state
            .pending
            .as_mut()
            .unwrap()
            .transfer
            .as_mut()
            .unwrap()
            .donor_root_identity = "different-root".into();
        assert!(validate(&state).is_err());
        state
            .pending
            .as_mut()
            .unwrap()
            .transfer
            .as_mut()
            .unwrap()
            .donor_root_identity
            .clear();
        rehash(&mut state);
        assert!(validate(&state).is_err());
    }
    #[test]
    fn semantic_phase_and_join_refusals() {
        let mut s = committed();
        let b = s.committed.as_ref().unwrap();
        validate_receipt_join(
            &s,
            &b.deployment,
            b.generation,
            b.receipt_digest.as_ref().unwrap(),
            &b.plan_hash,
        )
        .unwrap();
        assert!(
            validate_receipt_join(
                &s,
                &"f".repeat(64),
                b.generation,
                b.receipt_digest.as_ref().unwrap(),
                &b.plan_hash
            )
            .is_err()
        );
        s.committed.as_mut().unwrap().resources[0].progress = ResourceProgress::Uncertain;
        assert!(validate(&s).is_err());
        s = committed();
        s.committed.as_mut().unwrap().revoked = true;
        assert!(validate(&s).is_err());
        s = committed();
        s.committed.as_mut().unwrap().pins.source = "changed".into();
        assert!(validate(&s).is_err());
    }
    #[test]
    fn retention_bounds_and_literal_keys() {
        let mut s = committed();
        let b = s.committed.as_mut().unwrap();
        b.resources[0].operation = ResourceOperation::JsonMember;
        b.resources[0].key = vec!["a.b/slash".into()];
        rehash(&mut s);
        validate(&s).unwrap();
        s.committed.as_mut().unwrap().resources[0].destination = "../escape".into();
        rehash(&mut s);
        assert!(validate(&s).is_err());
        s = committed();
        if let Value::Present(v) = &mut s.committed.as_mut().unwrap().resources[0].desired {
            v.blob.bytes = BLOB_LIMIT + 1;
        }
        rehash(&mut s);
        assert!(validate(&s).is_err());
        s = committed();
        let mut row = s.committed.as_ref().unwrap().resources[0].clone();
        if let Value::Present(v) = &mut row.desired {
            v.blob.bytes = BLOB_LIMIT;
        }
        s.committed.as_mut().unwrap().resources = (0..9)
            .map(|i| {
                let mut r = row.clone();
                r.destination = format!("files/{i}");
                r
            })
            .collect();
        rehash(&mut s);
        assert!(validate(&s).is_err());
        s = committed();
        let row = s.committed.as_ref().unwrap().resources[0].clone();
        s.committed.as_mut().unwrap().resources.push(row);
        rehash(&mut s);
        assert!(validate(&s).is_err());
    }
    #[test]
    fn legacy_reader_refuses_new_state() {
        assert!(
            serde_json::from_str::<crate::generated::deploy_receipt::DeployReceipt>(&fixture(
                "valid/committed.json"
            ))
            .is_err()
        );
        let mut s = committed();
        s.committed.as_mut().unwrap().resources[0].before =
            Before::Unavailable(Box::new(BeforeUnavailable {
                reason: BeforeUnavailableReason::LegacyNoBeforeState,
            }));
        rehash(&mut s);
        validate(&s).unwrap();
        assert!(matches!(
            s.committed.unwrap().resources[0].before,
            Before::Unavailable(_)
        ));
    }
}
