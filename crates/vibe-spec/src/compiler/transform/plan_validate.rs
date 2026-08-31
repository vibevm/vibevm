//! The transform-plan refusal law (R4-TRANSFORM-PLAN-ABI §2.1) and its
//! typed, bounded diagnostics.
//!
//! Validation is deterministic and precedence-frozen: the checked entry
//! count first, then each seed in input order — key scalar, duplicate key,
//! provider version/host scalar and content-hash recheck, implementation
//! name/epoch, selector/stage. The first violation wins, so an exact error
//! identifies the exact fault.
//!
//! Diagnostics never echo a payload: a key, version, host name, builtin
//! name or content hash can be attacker-sized, so every error carries at
//! most a fixed-size preview plus the true length, and the full value never
//! enters the error value at all.

use std::collections::HashMap;

use vibe_core::ContentHash;
use vibe_extension_registry::{CompiledSelector, HostIdentity};

use crate::compiler::ir::BackendId;

use super::plan::{
    ImplementationComponents, ProviderComponents, TransformImplementation, TransformProvider,
    TransformSeed, TransformStage,
};

/// The maximum characters a diagnostic preview ever shows.
const PREVIEW_CHARS: usize = 8;

/// Check that a seed count is a dense-orderable `u32`.
///
/// The `> u32::MAX` refusal is exercised by unit-testing this helper with a
/// raw `usize` — a vector of 2^32 seeds is never allocated to prove a law
/// about its length.
pub(super) fn checked_entry_count(len: usize) -> Result<u32, TransformPlanError> {
    u32::try_from(len).map_err(|_| TransformPlanError::TooManyEntries { count: len })
}

/// Validate every seed under the frozen precedence; the first fault wins.
pub(super) fn validate_seeds(seeds: &[TransformSeed]) -> Result<(), TransformPlanError> {
    let mut first_by_key: HashMap<&str, usize> = HashMap::with_capacity(seeds.len());
    for (index, seed) in seeds.iter().enumerate() {
        let key = seed.key().as_str();
        check_scalar(key).map_err(|fault| TransformPlanError::Scalar {
            seed: index,
            field: "key",
            fault,
        })?;
        if let Some(first) = first_by_key.insert(key, index) {
            return Err(TransformPlanError::DuplicateKey {
                preview: bounded(key),
                first,
                second: index,
            });
        }
        validate_provider(index, seed.provider())?;
        validate_implementation(index, seed.implementation())?;
        validate_selector_stage(index, seed.stage(), seed.selector())?;
    }
    Ok(())
}

/// Provider law: exact version scalar (both variants), ungrouped-host name
/// scalar, and a borrowed content-hash grammar recheck of every required or
/// present hash — the type's public `from_validated` can wrap a value that
/// never parsed, so an invalid Rust-constructed hash must still refuse here
/// (checked borrowed, without `parse`'s error allocation).
fn validate_provider(index: usize, provider: &TransformProvider) -> Result<(), TransformPlanError> {
    match provider.components() {
        ProviderComponents::Dependency {
            version,
            content_hash,
            ..
        } => {
            check_scalar(version).map_err(|fault| TransformPlanError::Scalar {
                seed: index,
                field: "provider.version",
                fault,
            })?;
            recheck_hash(index, "provider.content_hash", content_hash)?;
        }
        ProviderComponents::Host {
            identity,
            version,
            content_hash,
            ..
        } => {
            if let HostIdentity::UngroupedProject(name) = identity {
                check_scalar(name).map_err(|fault| TransformPlanError::Scalar {
                    seed: index,
                    field: "host project name",
                    fault,
                })?;
            }
            check_scalar(version).map_err(|fault| TransformPlanError::Scalar {
                seed: index,
                field: "provider.version",
                fault,
            })?;
            if let Some(content_hash) = content_hash {
                recheck_hash(index, "provider.content_hash", content_hash)?;
            }
        }
    }
    Ok(())
}

/// Implementation law: the builtin candidate name obeys the compiler's one
/// frozen backend-id grammar `[a-z0-9][a-z0-9._-]{0,63}` — checked through
/// that single validator's borrowed predicate, never by drifting a second
/// grammar and never by cloning an attacker-sized candidate into an owned
/// id or error — and the behavior epoch is nonzero.
fn validate_implementation(
    index: usize,
    implementation: &TransformImplementation,
) -> Result<(), TransformPlanError> {
    if let ImplementationComponents::Builtin { name, epoch } = implementation.components() {
        if !BackendId::is_valid_spelling(name) {
            return Err(TransformPlanError::ImplementationName {
                seed: index,
                preview: bounded(name),
            });
        }
        if epoch == 0 {
            return Err(TransformPlanError::ImplementationEpoch { seed: index });
        }
    }
    Ok(())
}

/// Stage law: lane/emitted refuse any supplied selector — manifest presence
/// itself is illegal there, even for a behaviorally unscoped one. Absence is
/// legal at every stage; source/document also accept a present selector.
fn validate_selector_stage(
    index: usize,
    stage: &TransformStage,
    selector: Option<&CompiledSelector>,
) -> Result<(), TransformPlanError> {
    if selector.is_some() && matches!(stage, TransformStage::Lane | TransformStage::Emitted) {
        return Err(TransformPlanError::SelectorStage {
            seed: index,
            stage: stage.clone(),
        });
    }
    Ok(())
}

/// The scalar law shared by keys, exact versions and ungrouped host names:
/// nonempty, no ASCII control byte. Accepted bytes are preserved verbatim —
/// this is not a SemVer parser and it never trims or normalizes.
fn check_scalar(value: &str) -> Result<(), ScalarFault> {
    if value.is_empty() {
        return Err(ScalarFault::Empty);
    }
    if let Some(position) = value.bytes().position(|byte| byte.is_ascii_control()) {
        return Err(ScalarFault::ControlByte { position });
    }
    Ok(())
}

/// Re-run the content-hash grammar over one required/present hash spelling,
/// borrowed: `ContentHash::is_valid_spelling` is the same law `parse`
/// enforces, without the error clone `parse` builds for a refusal — the
/// spelling can be attacker-sized, so the refusal path never allocates it.
fn recheck_hash(
    index: usize,
    field: &'static str,
    hash: &ContentHash,
) -> Result<(), TransformPlanError> {
    if !ContentHash::is_valid_spelling(hash.as_str()) {
        return Err(TransformPlanError::ContentHash {
            seed: index,
            field,
            preview: bounded(hash.as_str()),
        });
    }
    Ok(())
}

/// A fixed-size window over one possibly attacker-sized scalar: the first
/// [`PREVIEW_CHARS`] characters (escaped when rendered) and the true byte
/// length. The whole value is never retained.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("(head {head:?}, total length {total} bytes)")]
pub(crate) struct BoundedPreview {
    head: String,
    total: usize,
}

/// Compute the bounded window of one scalar.
///
/// Inspects only the first `PREVIEW_CHARS + 1` characters: the head is the
/// capped prefix and truncation is decided by whether one further character
/// exists, so a validated attacker-sized scalar is never rescanned just to
/// decorate its preview. The true byte length is `len()` — O(1) by
/// definition of the string type.
pub(super) fn bounded(value: &str) -> BoundedPreview {
    let mut characters = value.chars();
    let mut head: String = characters.by_ref().take(PREVIEW_CHARS).collect();
    if characters.next().is_some() {
        head.push('…');
    }
    BoundedPreview {
        head,
        total: value.len(),
    }
}

/// How a scalar violated the scalar law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ScalarFault {
    #[error("is empty; the law is nonempty and no ASCII control byte")]
    Empty,
    #[error("contains an ASCII control byte at byte position {position}")]
    ControlByte { position: usize },
}

/// One typed transform-plan refusal, actionable by field and seed index,
/// never echoing a payload.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TransformPlanError {
    #[error("refusing {count} transform seeds: a count above u32::MAX has no dense order")]
    TooManyEntries { count: usize },
    #[error("transform seed {seed}: field `{field}` {fault}")]
    Scalar {
        seed: usize,
        field: &'static str,
        fault: ScalarFault,
    },
    #[error("duplicate transform key {preview}: first at seed {first}, again at seed {second}")]
    DuplicateKey {
        preview: BoundedPreview,
        first: usize,
        second: usize,
    },
    #[error("transform seed {seed}: field `{field}` does not spell a content hash {preview}")]
    ContentHash {
        seed: usize,
        field: &'static str,
        preview: BoundedPreview,
    },
    #[error(
        "transform seed {seed}: builtin implementation name violates the frozen backend-id grammar [a-z0-9][a-z0-9._-]{{0,63}} {preview}"
    )]
    ImplementationName {
        seed: usize,
        preview: BoundedPreview,
    },
    #[error("transform seed {seed}: builtin implementation epoch 0 is not a behavior epoch")]
    ImplementationEpoch { seed: usize },
    #[error(
        "transform seed {seed}: stage {stage:?} refuses a selector; selectors are legal at source/document only"
    )]
    SelectorStage { seed: usize, stage: TransformStage },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_entry_count_covers_the_u32_boundary_without_allocating_seeds() {
        assert_eq!(checked_entry_count(0), Ok(0));
        assert_eq!(checked_entry_count(1), Ok(1));
        assert_eq!(checked_entry_count(u32::MAX as usize), Ok(u32::MAX));
        assert_eq!(
            checked_entry_count(u32::MAX as usize + 1),
            Err(TransformPlanError::TooManyEntries {
                count: u32::MAX as usize + 1
            })
        );
        // The law's own ceiling: one past the boundary on any wider usize.
        assert_eq!(
            checked_entry_count(0x1_0000_0000usize),
            Err(TransformPlanError::TooManyEntries {
                count: 0x1_0000_0000usize
            })
        );
    }

    #[test]
    fn the_scalar_law_is_nonempty_and_control_free_without_trimming() {
        assert_eq!(check_scalar("org.demo/tools#x"), Ok(()));
        // Whitespace is preserved, not trimmed: accepted bytes stay.
        assert_eq!(check_scalar(" padded "), Ok(()));
        assert_eq!(check_scalar(""), Err(ScalarFault::Empty));
        assert_eq!(
            check_scalar("a\nb"),
            Err(ScalarFault::ControlByte { position: 1 })
        );
        assert_eq!(
            check_scalar("\u{7f}"),
            Err(ScalarFault::ControlByte { position: 0 })
        );
    }

    #[test]
    fn previews_are_bounded_and_never_carry_the_payload() {
        let hostile = format!("{}{}", "a".repeat(3 * 1024 * 1024), "\n");
        let preview = bounded(&hostile);
        let rendered = preview.to_string();
        assert!(rendered.len() <= 64, "preview rendered {rendered}");
        // An echo would be megabytes; the head preview contributes at most
        // eight characters, so a 16-character run is impossible without one.
        assert!(!rendered.contains(&"a".repeat(16)));
        assert_eq!(preview.total, hostile.len());
        // The empty and short values preview themselves without decoration.
        assert_eq!(bounded("").to_string(), "(head \"\", total length 0 bytes)");
        assert_eq!(
            bounded("sha256:aa").to_string(),
            "(head \"sha256:a…\", total length 9 bytes)"
        );
    }
}
