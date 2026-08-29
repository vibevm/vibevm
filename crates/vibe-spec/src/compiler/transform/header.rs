//! The active-transforms header payload (R4 architecture §7.1): the ONE
//! comment line a nonempty active plan contributes to its artifact.
//!
//! **What it says, and what it is not.** One token per plan entry, in the
//! plan's dense effective order, each token the entry's canonical
//! `ExtensionKey` spelling. It records the ACTIVE list and nothing beyond it:
//! identity attribution stays in provenance/IR, and NOTHING ever parses this
//! payload back to recover a plan. That law is why the payload needs no
//! separator escape and no length frame — it is a record for a reader and a
//! byte-identity input for the artifact, never a wire.
//!
//! **One codec, one spelling.** Tokens are spelled by the shared generated-
//! comment codec [`vibe_specdoc::encode_generated_xml_comment`] — the same
//! cell the generated XML lane already uses for its qualified anchors. A
//! second, local percent table would be two spellings of one identity, which
//! §7.1 rejects by name; this cell therefore contains no escape literal at
//! all, and its fence asserts that mechanically.
//!
//! **Why the comment is XML-safe unconditionally.** The codec guarantees an
//! encoded payload contains no `--` and never ends in `-`. Tokens are joined
//! by single spaces and carried inside `<!-- vibe:transforms … -->`, so no
//! `--` can form across a join and the last token cannot touch the closing
//! `-->`. A key containing `--` (legal in a package name) is therefore
//! encoded, never raw — the corner §7.1 names as the reason the codec is
//! mandatory here. A single interior `-` is left readable by the codec and is
//! lawful comment content; that is the codec's canonical spelling, not a
//! second one.
//!
//! **Whitespace.** A declaration `id` carrying a literal space would put that
//! space inside a token, and the codec does not escape it (spaces are lawful
//! XML comment content). The payload's grammar is therefore per
//! whitespace-separated RUN, not per key — which costs nothing, because no
//! consumer recovers keys from here. Inventing an escape for it would be the
//! second percent spelling this cell exists to prevent.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use super::plan::TransformPlan;

/// The reserved payload prefix of the active-transforms header. The framing
/// around it (`<!--` / `-->`) belongs to the emit cell, which owns how a
/// generated comment is spelled in each lane.
pub(crate) const TRANSFORMS_HEADER_PREFIX: &str = "vibe:transforms";

/// The header payload one owner-scoped plan contributes, or `None`.
///
/// `None` for the empty plan is the whole of the active-only law: an owner
/// that activates nothing contributes no line, so every committed artifact
/// stays byte-identical.
pub(crate) fn transforms_header_payload(plan: &TransformPlan) -> Option<String> {
    if plan.is_empty() {
        return None;
    }
    let mut payload = String::from(TRANSFORMS_HEADER_PREFIX);
    // Dense effective order, exactly as `TransformPlan::build` assigned it —
    // never sorted, never re-tiered: the authored order IS the record.
    for entry in plan.entries() {
        payload.push(' ');
        payload.push_str(&vibe_specdoc::encode_generated_xml_comment(
            entry.seed().key().as_str(),
        ));
    }
    Some(payload)
}

/// The encoded tokens of one OBSERVED header payload, in emitted order —
/// `None` when the payload does not open the reserved header at all.
///
/// The tape validators read a payload off the artifact and must judge its
/// grammar before comparing it to the engine's own; this is the split half of
/// that judgment, so the prefix and the separator are spelled once, here,
/// rather than once per lane.
pub(crate) fn observed_header_tokens(payload: &str) -> Option<impl Iterator<Item = &str>> {
    let rest = payload.strip_prefix(TRANSFORMS_HEADER_PREFIX)?;
    // The prefix is a WHOLE token: `vibe:transformsX` is some other payload,
    // not this header with a mangled first entry.
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.split_whitespace())
}
