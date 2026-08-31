//! Pure portable evidence for compiler-native transforms pending a build.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::collections::BTreeMap;
use std::fmt;

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{ExtensionKey, MechanismKey, SpecFormat};
use vibe_spec::{CompilerPendingRef, CompilerPendingSet, compiler_pending_header_payload};

use super::OwnerRuntimeId;

const DOMAIN: &[u8] = b"vibe-transform-pending-v1\0epoch=1\0";
const MAX_NODE_REL_BYTES: usize = 512;

macro_rules! witness_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $name([u8; 32]);

        impl $name {
            #[must_use]
            pub const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            #[must_use]
            pub fn hex(&self) -> String {
                lower_hex(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&format_args!("sha256:{}", self.hex()))
                    .finish()
            }
        }
    };
}

witness_type!(PendingSourceWitness);
witness_type!(PendingHandlerConfigWitness);
witness_type!(PendingBuildProviderDigest);

/// Closed portable current-platform key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PendingPlatformKey(PendingPlatform);

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingPlatform {
    WindowsX8664,
    LinuxX8664,
    MacosAarch64,
}

impl PendingPlatformKey {
    pub fn new(value: impl Into<String>) -> Result<Self, PendingEvidenceError> {
        let value = value.into();
        let platform = match value.as_str() {
            "windows-x86_64" => PendingPlatform::WindowsX8664,
            "linux-x86_64" => PendingPlatform::LinuxX8664,
            "macos-aarch64" => PendingPlatform::MacosAarch64,
            _ => return Err(fault(PendingEvidenceFault::InvalidPlatform)),
        };
        Ok(Self(platform))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self.0 {
            PendingPlatform::WindowsX8664 => "windows-x86_64",
            PendingPlatform::LinuxX8664 => "linux-x86_64",
            PendingPlatform::MacosAarch64 => "macos-aarch64",
        }
    }
}

impl fmt::Debug for PendingPlatformKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PendingPlatformKey")
            .field(&self.as_str())
            .finish()
    }
}

/// Closed artifact target identity. Physical paths are deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingArtifactTarget {
    BootStatic,
}

#[derive(Clone, PartialEq, Eq)]
struct PendingFactRef {
    plan_digest: [u8; 32],
    order: u32,
    key: ExtensionKey,
}

/// Build facts associated with one exact compiler pending reference.
#[derive(PartialEq, Eq)]
pub struct PendingBuildFact {
    reference: PendingFactRef,
    platform: PendingPlatformKey,
    source: PendingSourceWitness,
    handler_config: PendingHandlerConfigWitness,
    route: MechanismKey,
    provider: PendingBuildProviderDigest,
}

impl fmt::Debug for PendingBuildFact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingBuildFact")
            .field("order", &self.reference.order)
            .field("key", &self.reference.key.as_str())
            .field("platform", &self.platform)
            .field("source", &self.source)
            .field("handler_config", &self.handler_config)
            .field("route", &self.route.to_string())
            .field("provider", &self.provider)
            .finish()
    }
}

impl PendingBuildFact {
    pub fn from_pending(
        reference: &CompilerPendingRef,
        platform: PendingPlatformKey,
        source: PendingSourceWitness,
        handler_config: PendingHandlerConfigWitness,
        route: MechanismKey,
        provider: PendingBuildProviderDigest,
    ) -> Result<Self, PendingEvidenceError> {
        if route.to_string() != "build:cargo" {
            return Err(fault(PendingEvidenceFault::InvalidRoute));
        }
        Ok(Self {
            reference: PendingFactRef {
                plan_digest: *reference.plan_digest_bytes(),
                order: reference.order(),
                key: reference.key().clone(),
            },
            platform,
            source,
            handler_config,
            route,
            provider,
        })
    }
}

/// Raw SHA-256 identity of one pending artifact fact set.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PendingFingerprint([u8; 32]);

impl PendingFingerprint {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn hex(&self) -> String {
        lower_hex(&self.0)
    }

    #[must_use]
    pub fn sha256(&self) -> String {
        format!("sha256:{}", self.hex())
    }
}

impl fmt::Debug for PendingFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PendingFingerprint")
            .field(&self.sha256())
            .finish()
    }
}

/// Fingerprint plus the generated-comment logical payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingArtifactEvidence {
    fingerprint: PendingFingerprint,
    header_payload: String,
}

impl PendingArtifactEvidence {
    #[must_use]
    pub const fn fingerprint(&self) -> &PendingFingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn header_payload(&self) -> &str {
        &self.header_payload
    }
}

/// Build the exact portable evidence for one pending boot-static artifact.
pub fn build_pending_artifact_evidence(
    pending: &CompilerPendingSet,
    owner: OwnerRuntimeId,
    target: PendingArtifactTarget,
    format: SpecFormat,
    facts: Vec<PendingBuildFact>,
) -> Result<Option<PendingArtifactEvidence>, PendingEvidenceError> {
    if let OwnerRuntimeId::Node { rel } = &owner
        && !valid_node_rel(rel)
    {
        return Err(fault(PendingEvidenceFault::InvalidOwner));
    }
    if pending.is_empty() {
        return if facts.is_empty() {
            Ok(None)
        } else {
            Err(fault(PendingEvidenceFault::ExtraFact))
        };
    }

    let references = pending.iter().collect::<Vec<_>>();
    let plan_digest = *references[0].plan_digest_bytes();
    let mut prior_order = None;
    for reference in &references {
        if reference.plan_digest_bytes() != &plan_digest {
            return Err(fault(PendingEvidenceFault::PendingPlanMismatch));
        }
        if prior_order.is_some_and(|prior| prior >= reference.order()) {
            return Err(fault(PendingEvidenceFault::PendingOrder));
        }
        prior_order = Some(reference.order());
    }

    let mut by_order = BTreeMap::new();
    for fact in facts {
        let order = fact.reference.order;
        if let Some(prior) = by_order.insert(order, fact) {
            let next = &by_order[&order];
            let kind = if prior.eq(next) {
                PendingEvidenceFault::DuplicateFact
            } else {
                PendingEvidenceFault::ConflictingFact
            };
            return Err(fault(kind));
        }
    }

    let mut ordered = Vec::with_capacity(references.len());
    for reference in &references {
        let Some(fact) = by_order.remove(&reference.order()) else {
            if by_order.values().any(|fact| {
                fact.reference.plan_digest == plan_digest && fact.reference.key == *reference.key()
            }) {
                return Err(fault(PendingEvidenceFault::FactOrderMismatch));
            }
            return Err(fault(PendingEvidenceFault::MissingFact));
        };
        if fact.reference.plan_digest != plan_digest {
            return Err(fault(PendingEvidenceFault::FactPlanMismatch));
        }
        if fact.reference.key != *reference.key() {
            return Err(fault(PendingEvidenceFault::FactKeyMismatch));
        }
        ordered.push((*reference, fact));
    }
    if !by_order.is_empty() {
        return Err(fault(PendingEvidenceFault::ExtraFact));
    }

    let fingerprint = PendingFingerprint(pending_digest(
        &owner,
        target,
        format,
        &plan_digest,
        &ordered,
    )?);
    let header_payload = compiler_pending_header_payload(pending, fingerprint.as_bytes())
        .map_err(|_| fault(PendingEvidenceFault::PendingHeader))?;
    Ok(Some(PendingArtifactEvidence {
        fingerprint,
        header_payload,
    }))
}

fn pending_digest(
    owner: &OwnerRuntimeId,
    target: PendingArtifactTarget,
    format: SpecFormat,
    plan_digest: &[u8; 32],
    ordered: &[(&CompilerPendingRef, PendingBuildFact)],
) -> Result<[u8; 32], PendingEvidenceError> {
    let mut digest = Sha256::new();
    frame(&mut digest, DOMAIN)?;
    match owner {
        OwnerRuntimeId::Node { rel } => {
            digest.update([0]);
            frame(&mut digest, rel.as_bytes())?;
        }
        OwnerRuntimeId::Unit { provider } => {
            digest.update([1]);
            frame(&mut digest, provider.group().as_str().as_bytes())?;
            frame(&mut digest, provider.name().as_str().as_bytes())?;
        }
    }
    digest.update([match target {
        PendingArtifactTarget::BootStatic => 0,
    }]);
    digest.update([match format {
        SpecFormat::Mixed => 0,
        SpecFormat::Markdown => 1,
        SpecFormat::Xml => 2,
    }]);
    frame(&mut digest, plan_digest)?;
    digest.update(
        u64::try_from(ordered.len())
            .map_err(|_| fault(PendingEvidenceFault::LengthOverflow))?
            .to_le_bytes(),
    );
    for (reference, fact) in ordered {
        digest.update(reference.order().to_le_bytes());
        frame(&mut digest, reference.key().as_str().as_bytes())?;
        frame(&mut digest, fact.platform.as_str().as_bytes())?;
        frame(&mut digest, &fact.source.0)?;
        frame(&mut digest, &fact.handler_config.0)?;
        let route = fact.route.to_string();
        frame(&mut digest, route.as_bytes())?;
        frame(&mut digest, &fact.provider.0)?;
    }
    Ok(digest.finalize().into())
}

fn frame(digest: &mut Sha256, bytes: &[u8]) -> Result<(), PendingEvidenceError> {
    let len =
        u64::try_from(bytes.len()).map_err(|_| fault(PendingEvidenceFault::LengthOverflow))?;
    digest.update(len.to_le_bytes());
    digest.update(bytes);
    Ok(())
}

fn valid_node_rel(rel: &str) -> bool {
    if rel == "." {
        return true;
    }
    !rel.is_empty()
        && rel.len() <= MAX_NODE_REL_BYTES
        && !rel.starts_with('/')
        && !rel
            .chars()
            .any(|character| matches!(character, '\\' | ':' | '\0'))
        && rel
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// Opaque bounded pending-evidence refusal.
pub struct PendingEvidenceError {
    fault: PendingEvidenceFault,
}

impl fmt::Debug for PendingEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PendingEvidenceError(..)")
    }
}

impl fmt::Display for PendingEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fault.fmt(formatter)
    }
}

impl std::error::Error for PendingEvidenceError {}

#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
enum PendingEvidenceFault {
    #[error(
        "pending node owner is not one bounded portable relative path \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: pass `.` or one bounded forward workspace-relative node path)"
    )]
    InvalidOwner,
    #[error(
        "pending platform key is not one supported closed platform value \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: pass exactly `windows-x86_64`, `linux-x86_64`, or `macos-aarch64`)"
    )]
    InvalidPlatform,
    #[error(
        "pending build route is not the exact `build:cargo` mechanism key \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: bind each pending build fact to the selected `build:cargo` provider)"
    )]
    InvalidRoute,
    #[error(
        "pending compiler references do not share one transform plan \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: rebuild the pending set from one exact retained owner runtime)"
    )]
    PendingPlanMismatch,
    #[error(
        "pending compiler references are not in strict dense-order order \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: retain compiler pending references in their exact increasing plan order)"
    )]
    PendingOrder,
    #[error(
        "pending build facts contain one duplicate reference \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: supply exactly one build fact for each pending reference)"
    )]
    DuplicateFact,
    #[error(
        "pending build facts conflict at one dense order \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: keep only the fact carrying the exact pending reference at that order)"
    )]
    ConflictingFact,
    #[error(
        "pending build fact is missing \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: supply one fact for every pending reference)"
    )]
    MissingFact,
    #[error(
        "pending build facts contain an extra reference \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: remove facts not named by this pending set)"
    )]
    ExtraFact,
    #[error(
        "pending build fact carries a different dense order \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW; \
         fix: bind the fact to the pending reference at its exact order)"
    )]
    FactOrderMismatch,
    #[error(
        "pending build fact carries a different qualified key \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: bind the fact to the exact qualified pending key)"
    )]
    FactKeyMismatch,
    #[error(
        "pending build fact carries a different transform plan \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: derive the fact from this pending set)"
    )]
    FactPlanMismatch,
    #[error(
        "pending evidence length does not fit the frozen u64 frame \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY; \
         fix: reduce the bounded pending evidence input)"
    )]
    LengthOverflow,
    #[error(
        "compiler pending-header construction refused validated pending evidence \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER; \
         fix: rebuild the pending set from one exact retained compiler result)"
    )]
    PendingHeader,
}

fn fault(fault: PendingEvidenceFault) -> PendingEvidenceError {
    PendingEvidenceError { fault }
}

fn lower_hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}
