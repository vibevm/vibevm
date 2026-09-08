//! Truthful finalization of one provisional compiler-native pending artifact.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER");

use std::fmt;

use crate::compiler::emit::framing::{
    CommentSyntax, static_header, static_header_block, static_header_with_pending,
};
use crate::compiler::ir::{ArtifactFrame, EmittedArtifact, StaticCompileMode};

use super::emitted_reconstruction;
use super::header::{transforms_header_payload, transforms_header_payload_excluding};
use super::native_policy::session::validate_pending_set;
use super::native_policy::{
    CompilerNativePolicyError, CompilerPendingArtifact, CompilerPendingSet,
};
use super::plan::TransformPlan;

const PENDING_HEADER_PREFIX: &str = "vibe:transforms-pending";
const RESERVED_TRANSFORM_OPEN: &[u8] = b"<!-- vibe:transforms";

/// Publishable artifact plus the owned, non-reusable replay expectation that
/// produced its pending evidence.
pub struct CompilerFinalizedPendingArtifact {
    artifact: EmittedArtifact,
    pending: CompilerPendingSet,
}

impl CompilerFinalizedPendingArtifact {
    #[must_use]
    pub const fn artifact(&self) -> &EmittedArtifact {
        &self.artifact
    }

    #[must_use]
    pub const fn pending(&self) -> &CompilerPendingSet {
        &self.pending
    }

    #[must_use]
    pub fn into_parts(self) -> (EmittedArtifact, CompilerPendingSet) {
        (self.artifact, self.pending)
    }
}

impl fmt::Debug for CompilerFinalizedPendingArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerFinalizedPendingArtifact")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

/// Render the one pending-transform payload spelling from a genuine compiler
/// pending set and the workspace-owned raw fingerprint.
pub fn compiler_pending_header_payload(
    pending: &CompilerPendingSet,
    fingerprint: &[u8; 32],
) -> Result<String, CompilerPendingFinalizeError> {
    validate_shape(pending)?;
    let mut payload = format!("{PENDING_HEADER_PREFIX} sha256:{}", lower_hex(fingerprint));
    for reference in pending.iter() {
        payload.push(' ');
        payload.push_str(&reference.order().to_string());
        payload.push('=');
        payload.push_str(&vibe_specdoc::encode_generated_xml_comment(
            reference.key().as_str(),
        ));
    }
    if payload.contains(['\r', '\n']) || payload.contains("--") || payload.ends_with('-') {
        return Err(error(FinalizeFault::UnsafeHeader));
    }
    Ok(payload)
}

/// Consume one provisional pending artifact and rebuild only its opening
/// transform framing and output digest.
pub fn finalize_compiler_pending_artifact(
    pending: CompilerPendingArtifact,
    plan: &TransformPlan,
    fingerprint: &[u8; 32],
) -> Result<CompilerFinalizedPendingArtifact, CompilerPendingFinalizeError> {
    validate_pending_set(plan, pending.pending()).map_err(policy_error)?;
    let pending_header = compiler_pending_header_payload(pending.pending(), fingerprint)?;
    let full_active = transforms_header_payload(plan)
        .ok_or_else(|| error(FinalizeFault::PlanHasNoActiveHeader))?;
    let filtered_active =
        transforms_header_payload_excluding(plan, pending.pending()).map_err(policy_error)?;

    let (artifact, pending) = pending.into_parts();
    let context = artifact.provenance().context();
    let target = context.target();
    let ArtifactFrame::StaticLane { generated_path, .. } = context.frame() else {
        return Err(error(FinalizeFault::UnsupportedArtifact));
    };
    let syntax = if target.is_static_markdown()
        && context.artifact().as_str() == "static-md"
        && context.mode() == StaticCompileMode::QualifyPerNode
        && generated_path.ends_with(".md")
    {
        CommentSyntax::Markdown
    } else if target.is_static_xml()
        && context.artifact().as_str() == "static-xml"
        && context.mode() == StaticCompileMode::QualifyPerNode
        && generated_path.ends_with(".xml")
    {
        CommentSyntax::Xml
    } else {
        return Err(error(FinalizeFault::UnsupportedArtifact));
    };
    let (actual_active, pass_suffix) =
        active_payload(artifact.bytes(), syntax, generated_path, &full_active)?;
    let filtered_active = preserve_pass_suffix(filtered_active, &pass_suffix);
    let expected = static_header(syntax, generated_path, Some(&actual_active));
    let replacement = static_header_with_pending(
        syntax,
        generated_path,
        filtered_active.as_deref(),
        &pending_header,
    );
    let Some(body) = artifact.bytes().strip_prefix(expected.as_bytes()) else {
        return Err(error(FinalizeFault::OriginalFraming));
    };
    if body
        .windows(RESERVED_TRANSFORM_OPEN.len())
        .any(|window| window == RESERVED_TRANSFORM_OPEN)
    {
        return Err(error(FinalizeFault::DuplicateReservedFraming));
    }
    let mut bytes = Vec::with_capacity(replacement.len() + body.len());
    bytes.extend_from_slice(replacement.as_bytes());
    bytes.extend_from_slice(body);
    let artifact = emitted_reconstruction::reframe(artifact, bytes);
    Ok(CompilerFinalizedPendingArtifact { artifact, pending })
}

fn preserve_pass_suffix(transforms: Option<String>, suffix: &str) -> Option<String> {
    match (transforms, suffix.strip_prefix(' ')) {
        (Some(transforms), _) => Some(format!("{transforms}{suffix}")),
        (None, Some(passes)) if !passes.is_empty() => Some(passes.to_owned()),
        (None, _) => None,
    }
}

fn active_payload(
    bytes: &[u8],
    syntax: CommentSyntax,
    generated_path: &str,
    transforms: &str,
) -> Result<(String, String), CompilerPendingFinalizeError> {
    let fixed = static_header_block(syntax, generated_path);
    let rest = bytes
        .strip_prefix(fixed.as_bytes())
        .ok_or_else(|| error(FinalizeFault::OriginalFraming))?;
    let end = rest
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| error(FinalizeFault::OriginalFraming))?;
    let line =
        std::str::from_utf8(&rest[..end]).map_err(|_| error(FinalizeFault::OriginalFraming))?;
    let payload = line
        .strip_prefix("<!-- ")
        .and_then(|value| value.strip_suffix(" -->"))
        .ok_or_else(|| error(FinalizeFault::OriginalFraming))?;
    let suffix = payload
        .strip_prefix(transforms)
        .filter(|suffix| suffix.is_empty() || suffix.starts_with(" vibe:passes sha256:"))
        .ok_or_else(|| error(FinalizeFault::OriginalFraming))?;
    Ok((payload.to_owned(), suffix.to_owned()))
}

fn validate_shape(pending: &CompilerPendingSet) -> Result<(), CompilerPendingFinalizeError> {
    let mut references = pending.iter();
    let Some(first) = references.next() else {
        return Err(error(FinalizeFault::EmptyPending));
    };
    let plan_digest = first.plan_digest_bytes();
    if pending.retained_plan_digest() != Some(plan_digest) {
        return Err(error(FinalizeFault::InvalidPendingShape));
    }
    let mut prior = first.order();
    for reference in references {
        if reference.plan_digest_bytes() != plan_digest || reference.order() <= prior {
            return Err(error(FinalizeFault::InvalidPendingShape));
        }
        prior = reference.order();
    }
    Ok(())
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

/// Opaque bounded pending-finalization refusal.
pub struct CompilerPendingFinalizeError {
    fault: FinalizeFault,
}

impl fmt::Debug for CompilerPendingFinalizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompilerPendingFinalizeError(..)")
    }
}

impl fmt::Display for CompilerPendingFinalizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fault.fmt(formatter)
    }
}

impl std::error::Error for CompilerPendingFinalizeError {}

#[derive(Debug, thiserror::Error)]
enum FinalizeFault {
    #[error("a pending header requires at least one pending compiler-native transform")]
    EmptyPending,
    #[error("pending compiler references do not share one strictly ordered plan identity")]
    InvalidPendingShape,
    #[error("pending artifact does not match the retained transform plan: {0}")]
    Policy(CompilerNativePolicyError),
    #[error("the retained transform plan has no active header to finalize")]
    PlanHasNoActiveHeader,
    #[error("pending finalization supports only Markdown/XML static-lane artifacts")]
    UnsupportedArtifact,
    #[error("the provisional artifact does not begin with its exact full active opening")]
    OriginalFraming,
    #[error("the provisional artifact body contains duplicate reserved transform framing")]
    DuplicateReservedFraming,
    #[error("the generated pending header is not safe generated-comment payload")]
    UnsafeHeader,
}

fn policy_error(source: CompilerNativePolicyError) -> CompilerPendingFinalizeError {
    error(FinalizeFault::Policy(source))
}

fn error(fault: FinalizeFault) -> CompilerPendingFinalizeError {
    CompilerPendingFinalizeError { fault }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_filter_preserves_the_exact_pass_suffix() {
        let path = "vibevm/vibespecs/boot/STATIC.md";
        let transforms = "vibe:transforms org.demo/tools#native";
        let suffix = " vibe:passes sha256:abc 1:org.demo/tools#pass:after=emit";
        let bytes = format!(
            "{}<!-- {transforms}{suffix} -->\n\nbody",
            static_header_block(CommentSyntax::Markdown, path)
        );
        let (active, observed) =
            active_payload(bytes.as_bytes(), CommentSyntax::Markdown, path, transforms).unwrap();
        assert_eq!(active, format!("{transforms}{suffix}"));
        assert_eq!(observed, suffix);
        assert_eq!(
            preserve_pass_suffix(None, &observed).as_deref(),
            Some(suffix.trim_start())
        );
        assert_eq!(
            preserve_pass_suffix(Some("vibe:transforms retained".into()), &observed).unwrap(),
            format!("vibe:transforms retained{suffix}")
        );
    }
}
