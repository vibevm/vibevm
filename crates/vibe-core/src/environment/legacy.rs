//! Pure interpretation of old records. This module neither rewrites a lock
//! nor authenticates a source or a physical target. Its output is a proposal
//! with explicit weaker evidence, not a new exact environment identity.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LEGACY-RECORD-MIGRATION"
);

use crate::ContentHash;
use crate::manifest::LockedPackage;

use super::{AbsoluteTargetPath, CaseComparison, PrefixAssessment, PrefixCompatibility, Reference};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyHashRecipe {
    LegacyTree0,
    PortableTree1,
}

fn recipe(hash: &ContentHash) -> Option<LegacyHashRecipe> {
    let text = hash.as_str();
    if text.starts_with("sha256-tree/1:") {
        Some(LegacyHashRecipe::PortableTree1)
    } else if text.starts_with("sha256:") {
        Some(LegacyHashRecipe::LegacyTree0)
    } else {
        None
    }
}

fn full_digest(hash: &ContentHash) -> bool {
    let text = hash.as_str();
    let tail = text
        .strip_prefix("sha256-tree/1:")
        .or_else(|| text.strip_prefix("sha256:"));
    tail.is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

/// The entire incumbent lock row is retained so no old provenance field is
/// silently lost as its schema grows. The source locator is provenance; it
/// is never treated as the package's content identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyPackageEvidence {
    pub record: LockedPackage,
    pub recipe: LegacyHashRecipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingLegacyInput {
    SourceCapture,
    RecipeOneCapture,
    ConfigurationBytes,
    TargetPlatform,
    DependencyRole,
    RecursiveClosure,
    PayloadPrefix,
    PhysicalTarget,
    CurrentOutputBytes,
    OwnerVerification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyClassification {
    /// Readable as the old owner's claim, with its weaker meaning intact.
    Legacy {
        missing_for_exact: Vec<MissingLegacyInput>,
    },
    /// A requested stronger claim needs named owner evidence.
    NeedsInput { missing: Vec<MissingLegacyInput> },
    /// The supplied record cannot even be interpreted at this boundary.
    Unavailable { reason: &'static str },
}

pub struct ExactPromotionEvidence<'a> {
    pub source_capture: Option<&'a Reference>,
    pub configuration_bytes: Option<&'a [u8]>,
    pub target_platform: Option<&'a Reference>,
    pub dependency_role: Option<&'a Reference>,
    pub recursive_closure: Option<&'a Reference>,
    pub payload_prefix: Option<&'a Reference>,
    pub physical_target: Option<&'a Reference>,
}

fn absent(value: Option<&Reference>) -> bool {
    value.is_none_or(|value| value.as_str().trim().is_empty())
}

/// Preserve a `vibe.lock` package row exactly as legacy evidence. A recipe-0
/// hash stays recipe 0; an empty dependency list proves no closure either way.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LEGACY-RECORD-MIGRATION",
    r = 1
)]
pub fn legacy_package(row: &LockedPackage) -> Result<LegacyPackageEvidence, LegacyClassification> {
    let recipe = recipe(&row.content_hash).ok_or(LegacyClassification::Unavailable {
        reason: "unrecognized package content-hash recipe",
    })?;
    Ok(LegacyPackageEvidence {
        record: row.clone(),
        recipe,
    })
}

/// The old artifact owner supplies this view. `vibe-core` must not import its
/// wire type: that would invert the dependency graph. A production adapter
/// belongs beside the lifecycle artifact-record reader in a later atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyArtifactView {
    pub schema: u32,
    pub output_id: String,
    pub producer_target: String,
    pub producer_provider: String,
    pub digest: Option<String>,
    pub verification_at_production: Option<LegacyArtifactVerification>,
    pub path_relative: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyArtifactVerification {
    VerifiedAtProduction,
    UnverifiedAtProduction,
}

pub fn classify_legacy_artifact(view: &LegacyArtifactView) -> LegacyClassification {
    if view.schema != 1 || view.output_id.is_empty() || view.producer_target.is_empty() {
        return LegacyClassification::Unavailable {
            reason: "unsupported or incomplete artifact record",
        };
    }
    if view.digest.is_none()
        || view.verification_at_production != Some(LegacyArtifactVerification::VerifiedAtProduction)
    {
        return LegacyClassification::NeedsInput {
            missing: vec![
                MissingLegacyInput::CurrentOutputBytes,
                MissingLegacyInput::OwnerVerification,
            ],
        };
    }
    LegacyClassification::Legacy {
        missing_for_exact: vec![
            MissingLegacyInput::TargetPlatform,
            MissingLegacyInput::DependencyRole,
            MissingLegacyInput::RecursiveClosure,
            MissingLegacyInput::PayloadPrefix,
            MissingLegacyInput::PhysicalTarget,
            MissingLegacyInput::CurrentOutputBytes,
        ],
    }
}

/// Judge whether old package evidence can be *proposed* for a stronger
/// environment claim. Even a fully populated proposal remains legacy until
/// the source/storage owners validate its exact before-state and bytes.
pub fn classify_exact_promotion(
    row: &LegacyPackageEvidence,
    evidence: &ExactPromotionEvidence<'_>,
) -> LegacyClassification {
    let mut missing = Vec::new();
    if absent(evidence.source_capture) {
        missing.push(MissingLegacyInput::SourceCapture);
    }
    if evidence.configuration_bytes.is_none() {
        missing.push(MissingLegacyInput::ConfigurationBytes);
    }
    if absent(evidence.target_platform) {
        missing.push(MissingLegacyInput::TargetPlatform);
    }
    if absent(evidence.dependency_role) {
        missing.push(MissingLegacyInput::DependencyRole);
    }
    // `dependencies = []` is not closure proof; all old rows require an
    // explicit recursive proof regardless of their recorded list length.
    if absent(evidence.recursive_closure) {
        missing.push(MissingLegacyInput::RecursiveClosure);
    }
    if absent(evidence.payload_prefix) {
        missing.push(MissingLegacyInput::PayloadPrefix);
    }
    if absent(evidence.physical_target) {
        missing.push(MissingLegacyInput::PhysicalTarget);
    }
    if !missing.is_empty() {
        return LegacyClassification::NeedsInput { missing };
    }
    // A recipe-0 row remains a valid historical pin but cannot be renamed
    // recipe 1. A separate capture/re-hash must produce a new exact source.
    if row.recipe == LegacyHashRecipe::LegacyTree0 {
        return LegacyClassification::NeedsInput {
            missing: vec![MissingLegacyInput::RecipeOneCapture],
        };
    }
    LegacyClassification::Legacy {
        missing_for_exact: vec![MissingLegacyInput::OwnerVerification],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{TargetAnchor, TargetPathPolicy};

    fn r(value: &str) -> Reference {
        Reference::new(value).unwrap()
    }
    fn old_package() -> LockedPackage {
        toml::from_str(
            r#"
            kind = "flow"
            name = "wal"
            group = "org.example"
            version = "1.1.0"
            source_url = "https://origin.example/wal.git"
            source_ref = "v1.1.0"
            resolved_commit = "abc123"
            content_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            source_kind = "registry"
        "#,
        )
        .unwrap()
    }

    #[test]
    fn old_recipe_and_empty_dependency_list_keep_legacy_meaning() {
        let row = old_package();
        let legacy = legacy_package(&row).unwrap();
        assert_eq!(legacy.recipe, LegacyHashRecipe::LegacyTree0);
        assert_eq!(
            legacy.record.content_hash.as_str(),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(legacy.record.source_ref.as_deref(), Some("v1.1.0"));
        assert_eq!(legacy.record.resolved_commit.as_deref(), Some("abc123"));
        assert!(legacy.record.dependencies.is_empty());
        let proposal = classify_exact_promotion(
            &legacy,
            &ExactPromotionEvidence {
                source_capture: None,
                configuration_bytes: None,
                target_platform: None,
                dependency_role: None,
                recursive_closure: None,
                payload_prefix: None,
                physical_target: None,
            },
        );
        let LegacyClassification::NeedsInput { missing } = proposal else {
            panic!("missing inputs must be named")
        };
        assert!(missing.contains(&MissingLegacyInput::RecursiveClosure));
        assert!(missing.contains(&MissingLegacyInput::ConfigurationBytes));
        let complete_supplied = classify_exact_promotion(
            &legacy,
            &ExactPromotionEvidence {
                source_capture: Some(&r("old-capture")),
                configuration_bytes: Some(b""),
                target_platform: Some(&r("linux")),
                dependency_role: Some(&r("runtime")),
                recursive_closure: Some(&r("closure")),
                payload_prefix: Some(&r("prefix")),
                physical_target: Some(&r("target")),
            },
        );
        assert_eq!(
            complete_supplied,
            LegacyClassification::NeedsInput {
                missing: vec![MissingLegacyInput::RecipeOneCapture],
            }
        );
    }

    #[test]
    fn existence_only_artifact_never_becomes_verified() {
        let mut view = LegacyArtifactView {
            schema: 1,
            output_id: "tool".into(),
            producer_target: "build-tool".into(),
            producer_provider: "cargo".into(),
            digest: None,
            verification_at_production: None,
            path_relative: Some("bin/tool".into()),
        };
        assert!(matches!(
            classify_legacy_artifact(&view),
            LegacyClassification::NeedsInput { .. }
        ));
        view.digest = Some("sha256:abc".into());
        view.verification_at_production = Some(LegacyArtifactVerification::VerifiedAtProduction);
        assert!(matches!(
            classify_legacy_artifact(&view),
            LegacyClassification::Legacy { .. }
        ));
    }

    #[test]
    fn same_content_mirror_retains_both_locators_and_original_recipe() {
        let legacy = legacy_package(&old_package()).unwrap();
        assert!(matches!(
            assess_source_substitution(&legacy, "https://mirror.example/wal.git", None),
            SourceSubstitution::NeedsInput(_)
        ));
        assert!(matches!(
            assess_source_substitution(&legacy, "https://mirror.example/wal.git\n", None),
            SourceSubstitution::Unavailable(_)
        ));
        let owner = r("source-verifier");
        let evidence = r("source-proof");
        let proof = SameContentProof {
            owner: &owner,
            evidence: &evidence,
            observed_hash: &legacy.record.content_hash,
        };
        assert_eq!(
            assess_source_substitution(&legacy, "https://mirror.example/wal.git", Some(&proof)),
            SourceSubstitution::SameContent {
                recipe: LegacyHashRecipe::LegacyTree0,
                original_source: "https://origin.example/wal.git".into(),
                alternate_source: "https://mirror.example/wal.git".into(),
                verifier: owner.clone(),
                evidence: evidence.clone(),
            }
        );
        let malformed_proof = SameContentProof {
            owner: &owner,
            evidence: &r("proof\nline"),
            observed_hash: &legacy.record.content_hash,
        };
        assert!(matches!(
            assess_source_substitution(
                &legacy,
                "https://mirror.example/wal.git",
                Some(&malformed_proof),
            ),
            SourceSubstitution::NeedsInput(_)
        ));
        let changed = ContentHash::parse(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        let other = SameContentProof {
            owner: &r("source-verifier"),
            evidence: &r("proof"),
            observed_hash: &changed,
        };
        assert!(matches!(
            assess_source_substitution(&legacy, "https://mirror.example/wal.git", Some(&other)),
            SourceSubstitution::Unavailable(_)
        ));
        let mut abbreviated = old_package();
        abbreviated.content_hash = ContentHash::parse("sha256:abc").unwrap();
        let abbreviated = legacy_package(&abbreviated).unwrap();
        let short = SameContentProof {
            owner: &r("source-verifier"),
            evidence: &r("proof"),
            observed_hash: &abbreviated.record.content_hash,
        };
        assert!(matches!(
            assess_source_substitution(
                &abbreviated,
                "https://mirror.example/wal.git",
                Some(&short)
            ),
            SourceSubstitution::NeedsInput(_)
        ));
    }

    #[test]
    fn prefix_bound_change_refuses_and_relocatable_requires_owner_proof() {
        let policy = TargetPathPolicy::LinuxBytes;
        let old = AbsoluteTargetPath::new(
            policy,
            TargetAnchor::PosixRoot,
            vec![b"opt".to_vec(), b"a".to_vec()],
        )
        .unwrap();
        let new = AbsoluteTargetPath::new(
            policy,
            TargetAnchor::PosixRoot,
            vec![b"opt".to_vec(), b"b".to_vec()],
        )
        .unwrap();
        let bound = PrefixCompatibility::PrefixBound {
            exact_target_prefix: old.clone(),
        };
        assert!(matches!(
            assess_prefix_substitution(
                &bound,
                &old,
                &new,
                CaseComparison::ByteSensitive,
                &[],
                None
            ),
            LegacyClassification::Unavailable { .. }
        ));
        let producer = r("payload-producer");
        let evidence = r("relocation-proof");
        let portable = PrefixCompatibility::DeclaredRelocatable {
            producer: producer.clone(),
            evidence: Some(evidence.clone()),
            conditions: vec![],
        };
        assert!(matches!(
            assess_prefix_substitution(
                &portable,
                &old,
                &new,
                CaseComparison::ByteSensitive,
                &[],
                None
            ),
            LegacyClassification::NeedsInput { .. }
        ));
        let proof = PrefixOwnerProof {
            producer: &producer,
            evidence: &evidence,
        };
        assert!(matches!(
            assess_prefix_substitution(
                &portable,
                &old,
                &new,
                CaseComparison::ByteSensitive,
                &[],
                Some(&proof)
            ),
            LegacyClassification::Legacy { .. }
        ));
    }
}

/// Supplied source-owner proof, not an authorization minted by this pure
/// function. A matching URL or version never substitutes for this proof.
pub struct SameContentProof<'a> {
    pub owner: &'a Reference,
    pub evidence: &'a Reference,
    pub observed_hash: &'a ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSubstitution {
    SameContent {
        recipe: LegacyHashRecipe,
        original_source: String,
        alternate_source: String,
        verifier: Reference,
        evidence: Reference,
    },
    NeedsInput(&'static str),
    Unavailable(&'static str),
}

/// Compare a mirror under the selected row's *same named recipe*, retaining
/// both locators. This does not fetch bytes or grant a cache/storage write.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-SOURCE-SUBSTITUTION",
    r = 1
)]
pub fn assess_source_substitution(
    selected: &LegacyPackageEvidence,
    alternate_source: &str,
    proof: Option<&SameContentProof<'_>>,
) -> SourceSubstitution {
    if alternate_source.is_empty()
        || alternate_source.trim() != alternate_source
        || alternate_source.chars().any(char::is_control)
    {
        return SourceSubstitution::Unavailable("alternate source locator is malformed");
    }
    let Some(proof) = proof else {
        return SourceSubstitution::NeedsInput("source owner must prove alternate content");
    };
    if [proof.owner.as_str(), proof.evidence.as_str()]
        .iter()
        .any(|value| {
            value.trim().is_empty() || value.trim() != *value || value.chars().any(char::is_control)
        })
    {
        return SourceSubstitution::NeedsInput("source verifier identity is absent");
    }
    if !full_digest(&selected.record.content_hash) || !full_digest(proof.observed_hash) {
        return SourceSubstitution::NeedsInput(
            "a full content digest is required for substitution",
        );
    }
    if recipe(proof.observed_hash) != Some(selected.recipe)
        || proof.observed_hash != &selected.record.content_hash
    {
        return SourceSubstitution::Unavailable("alternate content or hash recipe differs");
    }
    SourceSubstitution::SameContent {
        recipe: selected.recipe,
        original_source: selected.record.source_url.as_str().to_owned(),
        alternate_source: alternate_source.to_owned(),
        verifier: proof.owner.clone(),
        evidence: proof.evidence.clone(),
    }
}

pub struct PrefixOwnerProof<'a> {
    pub producer: &'a Reference,
    pub evidence: &'a Reference,
}

/// Preserve the existing declarative prefix ruling and demand separately
/// supplied producer evidence for a relocatable claim. Prefix-bound changes
/// remain a refusal; relocation recipes belong to a later owner.
pub fn assess_prefix_substitution(
    policy: &PrefixCompatibility,
    source: &AbsoluteTargetPath,
    destination: &AbsoluteTargetPath,
    comparison: CaseComparison,
    conditions: &[Reference],
    proof: Option<&PrefixOwnerProof<'_>>,
) -> LegacyClassification {
    match policy.assess(source, destination, comparison, conditions) {
        PrefixAssessment::PrefixMismatch => LegacyClassification::Unavailable {
            reason: "prefix-bound payload changed prefix",
        },
        PrefixAssessment::Unsupported(_) => LegacyClassification::Unavailable {
            reason: "payload relocation is unsupported",
        },
        PrefixAssessment::UnresolvedComparison => LegacyClassification::NeedsInput {
            missing: vec![MissingLegacyInput::PayloadPrefix],
        },
        PrefixAssessment::NeedsEvidence | PrefixAssessment::NeedsConditions(_) => {
            LegacyClassification::NeedsInput {
                missing: vec![MissingLegacyInput::PayloadPrefix],
            }
        }
        PrefixAssessment::DeclarativeMatch => match policy {
            PrefixCompatibility::DeclaredRelocatable {
                producer,
                evidence: Some(expected),
                ..
            } if proof
                .is_some_and(|proof| proof.producer == producer && proof.evidence == expected) =>
            {
                LegacyClassification::Legacy {
                    missing_for_exact: vec![MissingLegacyInput::PhysicalTarget],
                }
            }
            PrefixCompatibility::DeclaredRelocatable { .. } => LegacyClassification::NeedsInput {
                missing: vec![MissingLegacyInput::PayloadPrefix],
            },
            PrefixCompatibility::PrefixBound { .. } => LegacyClassification::Legacy {
                missing_for_exact: vec![MissingLegacyInput::PhysicalTarget],
            },
            PrefixCompatibility::Unsupported { .. } => unreachable!("assess refused above"),
        },
    }
}
