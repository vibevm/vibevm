//! The canonical transform-plan digests (R4-TRANSFORM-PLAN-ABI §4.1): the
//! exact frame schedule over the one `StableDigest` primitive.
//!
//! Framing is frozen: the domain is the first length-framed field; tags and
//! presence are single bytes; counts and orders are u32/u64 little-endian;
//! every byte field is `u64_le(len) || bytes`; a child SHA-256 is always a
//! length-framed 32-byte field, never raw. Providers frame their typed
//! components — group, name, version, kind, hash — and the raw authored
//! spelling of an ungrouped host; a rendered owner coordinate, a resolved
//! root, the wall clock and the registry's all-view never enter. Selector
//! dimensions are canonical OR-sets (byte-sorted, deduplicated, counted
//! after dedup) while entry order stays semantic and is never sorted.

use vibe_extension_registry::{CompiledSelector, HostIdentity};

use crate::compiler::digest::StableDigest;

use super::plan::{
    ProviderComponents, TransformEntry, TransformImplementation, TransformProvider, TransformStage,
};

/// The canonical digest domain of one implementation identity (epoch 1).
const IMPLEMENTATION_DIGEST_DOMAIN: &[u8] = b"vibe-transform-implementation-v1\0epoch=1\0";
/// The canonical digest domain of one whole plan (epoch 1).
const PLAN_DIGEST_DOMAIN: &[u8] = b"vibe-transform-plan-v1\0epoch=1\0";

/// The frozen implementation-kind tag: builtin. Byte 1 stays reserved for
/// the R5 native implementation identity.
const TAG_IMPLEMENTATION_BUILTIN: u8 = 0;
/// The frozen provider tags: installed dependency, selected host.
const TAG_PROVIDER_DEPENDENCY: u8 = 0;
const TAG_PROVIDER_HOST: u8 = 1;
/// The frozen host-identity tags: ungrouped project, coordinate, virtual.
const TAG_HOST_UNGROUPED: u8 = 0;
const TAG_HOST_COORDINATE: u8 = 1;
const TAG_HOST_VIRTUAL_WORKSPACE: u8 = 2;

/// The 32-byte canonical digest of one implementation identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplementationDigest([u8; 32]);

impl ImplementationDigest {
    /// The raw digest bytes.
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// The 32-byte canonical digest of one whole (nonempty) plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlanDigest([u8; 32]);

impl PlanDigest {
    /// The raw digest bytes.
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The stable external projection: `sha256:` plus 64 lowercase hex
    /// characters. Computed on demand — the rendered identity is an output
    /// projection and is never stored, and it never enters another digest.
    pub(crate) fn sha256_hex(&self) -> String {
        let mut text = String::with_capacity("sha256:".len() + 2 * self.0.len());
        text.push_str("sha256:");
        text.push_str(&self.lowercase_hex());
        text
    }

    /// The 64 lowercase hex characters alone, with no algorithm prefix.
    ///
    /// The ONE hex rendering of this digest; [`Self::sha256_hex`] is that
    /// string behind its algorithm label, so the two projections cannot drift
    /// into two spellings of one value. T10C's boot-graph fingerprint frame
    /// takes this form because the frame's own label already says what the
    /// bytes are (`transforms:<hex>`), and repeating `sha256:` inside it would
    /// name the algorithm twice.
    pub(crate) fn lowercase_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut text = String::with_capacity(2 * self.0.len());
        for byte in self.0 {
            text.push(HEX[(byte >> 4) as usize] as char);
            text.push(HEX[(byte & 0x0f) as usize] as char);
        }
        text
    }
}

/// Digest one implementation identity: domain, builtin tag, name field,
/// epoch.
pub(super) fn implementation_digest(
    implementation: &TransformImplementation,
) -> ImplementationDigest {
    let mut digest = StableDigest::new(IMPLEMENTATION_DIGEST_DOMAIN);
    digest.byte(TAG_IMPLEMENTATION_BUILTIN);
    digest.field(implementation.builtin_name().as_bytes());
    digest.u32(implementation.builtin_epoch());
    ImplementationDigest(digest.finish())
}

/// Digest one whole plan: domain, entry count, then every entry in
/// effective order — key field, stage byte, dense order, provider frame,
/// implementation-digest field, config presence/digest field, selector
/// presence and canonical dimensions.
pub(super) fn plan_digest(entries: &[TransformEntry]) -> PlanDigest {
    let mut digest = StableDigest::new(PLAN_DIGEST_DOMAIN);
    digest.usize(entries.len());
    for entry in entries {
        digest.field(entry.seed().key().as_str().as_bytes());
        digest.byte(stage_byte(entry.seed().stage()));
        digest.u32(entry.order());
        frame_provider(&mut digest, entry.seed().provider());
        digest.field(entry.implementation_digest().as_bytes());
        match entry.config_digest() {
            Some(config) => {
                digest.byte(1);
                digest.field(config.as_bytes());
            }
            None => digest.byte(0),
        }
        frame_selector(&mut digest, entry.seed().selector());
    }
    PlanDigest(digest.finish())
}

/// The frozen stage ordinals: 0=source, 1=document, 2=lane, 3=emitted.
fn stage_byte(stage: &TransformStage) -> u8 {
    match stage {
        TransformStage::Source => 0,
        TransformStage::Document => 1,
        TransformStage::Lane => 2,
        TransformStage::Emitted => 3,
    }
}

/// Frame one provider's typed components.
fn frame_provider(digest: &mut StableDigest, provider: &TransformProvider) {
    match provider.components() {
        ProviderComponents::Dependency {
            id,
            version,
            kind,
            content_hash,
        } => {
            digest.byte(TAG_PROVIDER_DEPENDENCY);
            digest.field(id.group().as_str().as_bytes());
            digest.field(id.name().as_str().as_bytes());
            digest.field(version.as_bytes());
            digest.field(kind.as_str().as_bytes());
            digest.field(content_hash.as_str().as_bytes());
        }
        ProviderComponents::Host {
            identity,
            version,
            kind,
            content_hash,
        } => {
            digest.byte(TAG_PROVIDER_HOST);
            match identity {
                HostIdentity::UngroupedProject(name) => {
                    digest.byte(TAG_HOST_UNGROUPED);
                    digest.field(name.as_bytes());
                }
                HostIdentity::Coordinate(id) => {
                    digest.byte(TAG_HOST_COORDINATE);
                    digest.field(id.group().as_str().as_bytes());
                    digest.field(id.name().as_str().as_bytes());
                }
                HostIdentity::VirtualWorkspace => {
                    digest.byte(TAG_HOST_VIRTUAL_WORKSPACE);
                }
            }
            digest.field(version.as_bytes());
            match kind {
                Some(kind) => {
                    digest.byte(1);
                    digest.field(kind.as_str().as_bytes());
                }
                None => digest.byte(0),
            }
            match content_hash {
                Some(content_hash) => {
                    digest.byte(1);
                    digest.field(content_hash.as_str().as_bytes());
                }
                None => digest.byte(0),
            }
        }
    }
}

/// Frame selector presence and both dimensions. Each present dimension is a
/// canonical OR-set: byte-sorted, deduplicated members, the count taken
/// after deduplication. `None` writes only presence byte 0; a present empty
/// dimension writes presence byte 1 plus a zero count — absence never
/// equals present-empty.
fn frame_selector(digest: &mut StableDigest, selector: Option<&CompiledSelector>) {
    match selector {
        None => digest.byte(0),
        Some(selector) => {
            digest.byte(1);
            frame_dimension(digest, selector.package_patterns());
            frame_dimension(digest, selector.path_patterns());
        }
    }
}

/// Frame one selector dimension as a canonical OR-set.
fn frame_dimension(digest: &mut StableDigest, dimension: Option<&[String]>) {
    match dimension {
        None => digest.byte(0),
        Some(patterns) => {
            digest.byte(1);
            let mut canonical: Vec<&str> = patterns.iter().map(String::as_str).collect();
            canonical.sort_unstable();
            canonical.dedup();
            digest.usize(canonical.len());
            for pattern in canonical {
                digest.field(pattern.as_bytes());
            }
        }
    }
}
