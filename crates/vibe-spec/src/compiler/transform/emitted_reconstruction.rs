//! The T9 manager-side emitted reconstruction cell
//! (R4-TRANSFORM-PLAN-ABI §6.5): the single writer of a post-backend
//! [`EmittedArtifact`], and the only place `EmissionProvenance.
//! emitted_transforms` is ever appended to.
//!
//! Four properties of this cell are load-bearing and easy to lose.
//!
//! **The two arms are ONE law, so they live in one cell.** The wrapper hands
//! this cell the original artifact and the behavior's returned bytes and takes
//! back whatever it answers; it never compares the two itself. If the compare
//! lived in the wrapper, "byte-equal returns the ORIGINAL untouched" and
//! "changed bytes rebuild everything" would be two rules in two places, free
//! to drift — and the first one is exactly the rule a careless recompute
//! silently breaks, because a recomputed digest of identical bytes still
//! compares equal while the artifact is no longer the same value.
//!
//! **The digest is recomputed through the ONE existing digest cell.** There is
//! deliberately no second SHA-256 or framing here: `emitted_bytes_digest` is
//! the same function `emit` used to author the digest this cell supersedes, so
//! `EmittedArtifact::output_fingerprint`, the compile trace and fresh-skip all
//! observe the post-transform truth in one spelling rather than two.
//!
//! **Reconstruction is a whole-value build, never a mutation.** §6 bans
//! `bytes_mut`/`provenance_mut`, and no such accessor exists on any type. The
//! old artifact is CONSUMED here and its provenance destructured member by
//! member, so the copied set is stated rather than implied: a member added to
//! [`EmissionProvenance`] later makes this cell fail to compile until its
//! author decides whether a transform preserves it or the manager rewrites it.
//!
//! **No witness gate guards this position, and that is a decision, not an
//! omission.** A lane behavior receives the whole carrier — including
//! `frame.renames`, which flows onward into `EmissionProvenance.renames` — so
//! `lane_admission` must prove the returned lane did not forge provenance. An
//! emitted behavior receives bytes and returns bytes: there is no channel
//! through which it could reach provenance at all, so this cell is the single
//! writer by construction and has nothing to authenticate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use crate::compiler::emit::emitted_bytes_digest;
use crate::compiler::ir::{EmissionProvenance, EmittedArtifact};
use crate::compiler::pass::PassName;

/// Answer one emitted behavior's output: the ORIGINAL artifact when the bytes
/// did not move, a wholly rebuilt one when they did.
///
/// `pass` is the entry's exact schedule pass name, handed down as a value the
/// wrapper already owns. Nothing here renders, parses or re-derives a name:
/// the identity is recorded, never reconstructed from a rendered spelling.
///
/// There is no error arm. A reconstruction has no failure mode of its own —
/// byte-equal and changed output are both lawful, and the behavior's own
/// refusal was already projected onto the entry's identity by the wrapper
/// before this cell is reached.
pub(crate) fn reconstruct(
    original: EmittedArtifact,
    bytes: Vec<u8>,
    pass: &PassName,
) -> EmittedArtifact {
    if bytes.as_slice() == original.bytes() {
        // Byte-equal output is not a rewrite: no recompute, no append, and the
        // artifact returned is `Eq` to the untransformed compile's — including
        // an `emitted_transforms` still empty.
        return original;
    }
    let EmittedArtifact {
        provenance,
        bytes: _superseded_bytes,
    } = original;
    let EmissionProvenance {
        context,
        backend,
        producer,
        source_lane_digest,
        renames,
        contributions,
        mut emitted_transforms,
        bytes_digest: _superseded_digest,
    } = provenance;
    emitted_transforms.push(pass.clone());
    EmittedArtifact {
        provenance: EmissionProvenance {
            // Copied unchanged: each of these describes what the closure, link,
            // assemble and emit stages did, which no post-backend rewrite of
            // the bytes can restate.
            context,
            backend,
            producer,
            source_lane_digest,
            renames,
            contributions,
            // The only member this cell authors, plus the digest below.
            emitted_transforms,
            bytes_digest: emitted_bytes_digest(&bytes),
        },
        bytes,
    }
}

/// Rebuild one artifact after manager-owned framing changed, preserving every
/// provenance member and appending no executed transform. This is distinct
/// from [`reconstruct`]: pending-header finalization records evidence about a
/// transform that did not run, so only bytes and their digest may move.
pub(super) fn reframe(original: EmittedArtifact, bytes: Vec<u8>) -> EmittedArtifact {
    let EmittedArtifact {
        provenance,
        bytes: _superseded_bytes,
    } = original;
    let EmissionProvenance {
        context,
        backend,
        producer,
        source_lane_digest,
        renames,
        contributions,
        emitted_transforms,
        bytes_digest: _superseded_digest,
    } = provenance;
    EmittedArtifact {
        provenance: EmissionProvenance {
            context,
            backend,
            producer,
            source_lane_digest,
            renames,
            contributions,
            emitted_transforms,
            bytes_digest: emitted_bytes_digest(&bytes),
        },
        bytes,
    }
}
