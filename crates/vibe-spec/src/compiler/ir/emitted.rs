//! Manager-owned emitted bytes and immutable production evidence.

use crate::SpecAddress;

use super::{
    ArtifactContext, ClosureNodeId, ContributionMeta, DocumentAddress, LaneContribution, LaneFrame,
    LinkInputDigest, OriginRename,
};
use crate::compiler::backend::BackendId;
use crate::compiler::emit::emitted_bytes_digest;
use crate::compiler::pass::PassName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneInputDigest(pub(crate) [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EmissionContributionWitness {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        seed_address: SpecAddress,
        chunk_digest: [u8; 32],
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
        chunk_digest: [u8; 32],
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// Target-specific semantic material prepared from the Lane before a backend
/// can emit bytes. XML documents are parsed from the Lane's canonical
/// Markdown exactly once, then shared by the renderer and the independent
/// emitted-tape validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedEmissionTarget {
    Markdown,
    Xml {
        documents: Vec<Option<vibe_specdoc::doc::SpecDoc>>,
    },
    #[cfg(any(test, feature = "test-support"))]
    Custom,
}

/// Manager-owned snapshot captured before the selected backend can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreEmissionWitness {
    pub(crate) context: ArtifactContext,
    pub(crate) source_node_count: usize,
    pub(crate) source_link_digest: LinkInputDigest,
    pub(crate) frame: LaneFrame,
    pub(crate) contributions: Vec<LaneContribution>,
    pub(crate) lane_digest: LaneInputDigest,
    pub(crate) emission_witnesses: Vec<EmissionContributionWitness>,
    pub(crate) prepared_target: PreparedEmissionTarget,
}

/// Immutable evidence created by the manager at the selected backend boundary.
///
/// Every member is written by the manager and by nothing else. `producer`
/// names the backend pass that first produced bytes; `emitted_transforms`
/// names the post-backend rewrites that changed them afterwards, in
/// application order, so a later reader can answer "which pass explains these
/// bytes" without diffing digests (R4-TRANSFORM-PLAN-ABI §6.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmissionProvenance {
    pub(crate) context: ArtifactContext,
    pub(crate) backend: BackendId,
    pub(crate) producer: PassName,
    pub(crate) source_lane_digest: LaneInputDigest,
    pub(crate) renames: Vec<OriginRename>,
    pub(crate) contributions: Vec<EmissionContributionWitness>,
    /// The schedule pass name of every emitted-position transform that
    /// returned CHANGED bytes, appended in application order. Empty at
    /// emission and empty forever on an artifact no emitted transform
    /// changed, so such an artifact is spelled exactly as it was before this
    /// member existed. Written only by
    /// [`crate::compiler::transform::emitted_reconstruction`]; a behavior
    /// receives bytes and returns bytes, so it owns no channel that reaches
    /// here.
    pub(crate) emitted_transforms: Vec<PassName>,
    pub(crate) bytes_digest: [u8; 32],
}

impl EmissionProvenance {
    pub fn context(&self) -> &ArtifactContext {
        &self.context
    }

    pub fn backend_id(&self) -> &str {
        self.backend.as_str()
    }

    pub fn producer(&self) -> &str {
        self.producer.as_str()
    }
}

/// Arbitrary final artifact bytes plus manager-owned immutable provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedArtifact {
    pub(crate) provenance: EmissionProvenance,
    pub(crate) bytes: Vec<u8>,
}

impl EmittedArtifact {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn provenance(&self) -> &EmissionProvenance {
        &self.provenance
    }

    /// The canonical formatted fingerprint of this artifact's emitted output —
    /// the manager's own digest of its bytes, spelled the one way the compile
    /// trace records it. Identical to [`emitted_output_fingerprint`] over the
    /// same bytes, so a dirty compile and a later observation of the file it
    /// wrote name one fingerprint, never two spellings of it.
    pub fn output_fingerprint(&self) -> String {
        format_fingerprint(&self.provenance.bytes_digest)
    }

    #[cfg(test)]
    pub(crate) fn testing(context: ArtifactContext, bytes: Vec<u8>) -> Self {
        Self {
            provenance: EmissionProvenance {
                context,
                backend: BackendId::new("test").unwrap(),
                producer: PassName::new("emit:test").unwrap(),
                source_lane_digest: LaneInputDigest([0; 32]),
                renames: Vec::new(),
                contributions: Vec::new(),
                emitted_transforms: Vec::new(),
                bytes_digest: [0; 32],
            },
            bytes,
        }
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.provenance.context
    }
}

/// The one canonical fingerprint of emitted output bytes (PROP-054
/// `##OBS-TRACE`): the manager's EXISTING digest implementation, formatted
/// once as `sha256:` + 64 lowercase hex characters.
///
/// This is the whole spelling authority for a compile-trace scope's output
/// fingerprint — a dirty [`EmittedArtifact::output_fingerprint`] and an
/// independently observed file holding identical bytes produce the exact same
/// string here, so a later reader compares one value, never two spellings of
/// it. There is deliberately no second SHA-256 or hex routine anywhere else in
/// the trace stack: callers reach this function (or the method that shares its
/// formatter) and nothing else.
pub fn emitted_output_fingerprint(bytes: &[u8]) -> String {
    format_fingerprint(&emitted_bytes_digest(bytes))
}

/// `sha256:` followed by exactly 64 lowercase hex characters. The prefix names
/// the family; the bytes are the manager's domain-separated digest, so this
/// string can never collide with a bare-digest spelling of some other hash.
fn format_fingerprint(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity("sha256:".len() + 64);
    out.push_str("sha256:");
    for byte in digest {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

pub(crate) type EmittedIr = EmittedArtifact;

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    fn static_context() -> ArtifactContext {
        use super::super::{ArtifactFrame, ArtifactId, StaticCompileMode};
        ArtifactContext::new(
            ArtifactId::new("static-md").unwrap(),
            super::super::ArtifactTarget::StaticMarkdown,
            ArtifactFrame::StaticLane {
                generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
                source_root: "vibevm/vibedeps".to_string(),
            },
            StaticCompileMode::QualifyPerNode,
        )
        .unwrap()
    }

    /// A dirty artifact whose provenance digest is the manager's own digest of
    /// its bytes — the invariant `EmitPass::run` guarantees and this cell relies
    /// on for the two spellings to agree.
    fn dirty_artifact(bytes: &[u8]) -> EmittedArtifact {
        let mut artifact = EmittedArtifact::testing(static_context(), bytes.to_vec());
        artifact.provenance.bytes_digest = emitted_bytes_digest(&artifact.bytes);
        artifact
    }

    /// The spelling law: `sha256:` + exactly 64 lowercase hex characters.
    #[test]
    fn the_fingerprint_is_prefixed_lowercase_hex_of_fixed_width() {
        for bytes in [&b""[..], b"x", b"the quick brown fox"] {
            let fingerprint = emitted_output_fingerprint(bytes);
            assert_eq!(fingerprint.len(), 7 + 64, "{fingerprint}");
            assert!(fingerprint.starts_with("sha256:"));
            assert!(
                fingerprint[7..]
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                "{fingerprint}"
            );
        }
    }

    /// The dirty artifact and independently supplied identical bytes name ONE
    /// fingerprint — the property the fresh-skip recording depends on.
    #[test]
    fn a_dirty_artifact_and_its_identical_bytes_agree_exactly() {
        let bytes = b"# Alpha {#root}\n##SHARED shared\n".as_slice();
        let artifact = dirty_artifact(bytes);
        assert_eq!(
            artifact.output_fingerprint(),
            emitted_output_fingerprint(bytes)
        );
        // And a real difference is still visible: the fingerprint discriminates.
        assert_ne!(
            artifact.output_fingerprint(),
            emitted_output_fingerprint(b"# Alpha {#root}\n##SHARED other\n")
        );
    }

    /// Stable and discriminating: same bytes always the same string, different
    /// bytes always different.
    #[test]
    fn the_fingerprint_is_stable_and_discriminating() {
        let first = emitted_output_fingerprint(b"one");
        assert_eq!(first, emitted_output_fingerprint(b"one"));
        assert_ne!(first, emitted_output_fingerprint(b"two"));
        // The empty artifact is a legal output and has its own honest value.
        assert_eq!(
            emitted_output_fingerprint(b""),
            emitted_output_fingerprint(b"")
        );
    }
}
