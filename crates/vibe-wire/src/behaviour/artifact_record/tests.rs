//! RED arms for every scalar law of the artifact record, plus the
//! positives that keep them honest. Each law has at least one arm; the
//! arms are minimal mutations of one legal base value, so a refusal
//! names the law, not a fixture's accident.

use chrono::TimeZone;

use crate::behaviour::artifact_record::{
    AbsolutePathUnsafety, ArtifactRecordError, MechanismDefect, RECORD_EPOCH, validate,
};
use crate::behaviour::scalars::RelativePathDefect;
use crate::generated::artifact_record::{
    ArtifactKind, ArtifactRecord, ArtifactShape, ContentDigest, DigestAlgorithm,
    FreshnessFingerprints, ProducerIdentity, ProviderIdentity, RelativeIdentity, RelativeRoot,
    Rfc3339Timestamp, VerificationState, VerificationStatus,
};

const HEX64: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn ts() -> Rfc3339Timestamp {
    chrono::Utc
        .with_ymd_and_hms(2026, 8, 29, 12, 0, 0)
        .single()
        .expect("one instant")
}

/// One legal minimal record: no optional member carried, the provider
/// reduced to its key — everything the wire demands and nothing else.
fn base() -> ArtifactRecord {
    ArtifactRecord {
        schema: RECORD_EPOCH,
        id: "vibe-helper.exe".to_string(),
        kind: ArtifactKind::Executable,
        shape: ArtifactShape::File,
        path_absolute: "C:/work/demo/target/release/vibe-helper.exe".to_string(),
        path_relative: RelativeIdentity {
            root: RelativeRoot::Project,
            path: "target/release/vibe-helper.exe".to_string(),
        },
        digest: ContentDigest {
            algorithm: DigestAlgorithm::Sha256,
            value: HEX64.to_string(),
        },
        producer: ProducerIdentity {
            target: "vibe-helper".to_string(),
            mechanism: "build:cargo".to_string(),
            provider: ProviderIdentity {
                key: "org.vibevm/vibe#cargo".to_string(),
                version: None,
                content_hash: None,
            },
        },
        freshness: FreshnessFingerprints {
            inputs: None,
            config: None,
            toolchain: None,
        },
        created_at: ts(),
        verification: VerificationState {
            status: VerificationStatus::Unverified,
            evidence: None,
        },
        media_type: None,
        platform: None,
    }
}

#[test]
fn the_minimal_record_validates() {
    validate(&base()).unwrap();
}

#[test]
fn the_full_record_validates() {
    let mut record = base();
    record.kind = ArtifactKind::Directory;
    record.shape = ArtifactShape::Directory;
    record.digest.algorithm = DigestAlgorithm::Sha256Tree;
    record.path_relative = RelativeIdentity {
        root: RelativeRoot::Store,
        path: "org.vibevm/vibe/1.2.3/plugin".to_string(),
    };
    record.producer.mechanism = "package:agent-plugin".to_string();
    record.producer.provider.version = Some("1.2.3".to_string());
    record.producer.provider.content_hash = Some(format!("sha256:{HEX64}"));
    record.freshness.inputs = Some(HEX64.to_string());
    record.freshness.config = Some(HEX64.to_string());
    record.freshness.toolchain = Some(HEX64.to_string());
    record.verification.status = VerificationStatus::Verified;
    record.verification.evidence = Some("cargo json artifact digest re-hashed".to_string());
    record.media_type = Some("application/zip".to_string());
    record.platform = Some("x86_64-pc-windows-msvc".to_string());
    validate(&record).unwrap();
}

#[test]
fn a_newer_epoch_refuses() {
    let mut record = base();
    record.schema = 2;
    assert_eq!(
        validate(&record),
        Err(ArtifactRecordError::SchemaEpoch { found: 2 })
    );
}

#[test]
fn an_id_outside_the_frozen_grammar_refuses() {
    let mut record = base();
    record.id = "Vibe-Helper.exe".to_string();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::IdNotPortableToken { .. })
    ));
}

#[test]
fn every_absolute_path_unsafety_is_reachable() {
    for (path, reason) in [
        (
            "C:\\work\\demo\\vibe-helper.exe",
            AbsolutePathUnsafety::Backslash,
        ),
        (
            "C:/work/demo\n/vibe-helper.exe",
            AbsolutePathUnsafety::ControlByte,
        ),
        (
            "target/release/vibe-helper.exe",
            AbsolutePathUnsafety::NotAbsolute,
        ),
    ] {
        let mut record = base();
        record.path_absolute = path.to_string();
        assert_eq!(
            validate(&record),
            Err(ArtifactRecordError::UnsafeAbsolutePath {
                path: crate::behaviour::compiler_trace_index::ScalarPreview::of(path),
                reason,
            }),
            "{path:?} must refuse as {reason:?}"
        );
    }
}

#[test]
fn a_relative_identity_that_leaves_its_root_refuses() {
    let mut record = base();
    record.path_relative.path = "../outside/vibe-helper.exe".to_string();
    assert_eq!(
        validate(&record),
        Err(ArtifactRecordError::UnsafeRelativePath {
            path: crate::behaviour::compiler_trace_index::ScalarPreview::of(
                "../outside/vibe-helper.exe"
            ),
            defect: RelativePathDefect::ParentSegment,
        })
    );
}

#[test]
fn a_short_digest_value_refuses() {
    let mut record = base();
    record.digest.value = HEX64[..63].to_string();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::DigestValueNotHex { .. })
    ));
    // The same length in UPPERCASE is equally wrong — the law is
    // lowercase hex, not just sixty-four characters.
    record.digest.value = HEX64.to_uppercase();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::DigestValueNotHex { .. })
    ));
}

#[test]
fn a_producer_outside_the_grammar_refuses() {
    let mut record = base();
    record.producer.target = "Vibe Helper".to_string();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::ProducerTargetNotPortableToken { .. })
    ));
}

#[test]
fn every_mechanism_defect_is_reachable() {
    for (mechanism, reason) in [
        ("cargo", MechanismDefect::MissingRolePrefix),
        ("deploy:vibe-bin", MechanismDefect::UnknownRole),
        ("build:Cargo Build", MechanismDefect::BadTail),
    ] {
        let mut record = base();
        record.producer.mechanism = mechanism.to_string();
        assert_eq!(
            validate(&record),
            Err(ArtifactRecordError::BadMechanismKey {
                mechanism: crate::behaviour::compiler_trace_index::ScalarPreview::of(mechanism),
                reason,
            }),
            "{mechanism:?} must refuse as {reason:?}"
        );
    }
}

#[test]
fn a_provider_key_outside_the_extension_shape_refuses() {
    let mut record = base();
    record.producer.provider.key = "org.vibevm/vibe-cargo".to_string();
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::BadProviderKey { .. })
    ));
}

#[test]
fn a_content_hash_outside_the_identity_spelling_refuses() {
    let mut record = base();
    // Bare hex is the record-family digest spelling, but content_hash
    // is an IDENTITY digest — it carries the scheme every lockfile row
    // carries, and bare hex here would be a second spelling of one
    // thing.
    record.producer.provider.content_hash = Some(HEX64.to_string());
    assert!(matches!(
        validate(&record),
        Err(ArtifactRecordError::BadContentHash { .. })
    ));
}

#[test]
fn every_freshness_member_refuses_a_short_digest() {
    for member in [
        "freshness.inputs",
        "freshness.config",
        "freshness.toolchain",
    ] {
        let mut record = base();
        let short = HEX64[..63].to_string();
        if member == "freshness.inputs" {
            record.freshness.inputs = Some(short.clone());
        } else if member == "freshness.config" {
            record.freshness.config = Some(short.clone());
        } else {
            record.freshness.toolchain = Some(short);
        }
        let error = validate(&record).expect_err("a 63-hex digest must refuse");
        assert!(
            matches!(
                &error,
                ArtifactRecordError::BadFreshnessDigest { member: m, .. } if *m == member
            ),
            "{member}: wrong refusal {error:?}"
        );
    }
}

#[test]
fn every_free_text_member_refuses_blank_and_control_bytes() {
    for (field, value) in [
        ("media_type", "  "),
        ("media_type", "application/zip\n"),
        ("platform", "\r"),
        ("verification.evidence", " "),
    ] {
        let mut record = base();
        if field == "media_type" {
            record.media_type = Some(value.to_string());
        } else if field == "platform" {
            record.platform = Some(value.to_string());
        } else {
            record.verification.evidence = Some(value.to_string());
        }
        let error = validate(&record).expect_err("blank free text must refuse");
        assert!(
            matches!(
                &error,
                ArtifactRecordError::UnsafeScalar { field: f, .. } if *f == field
            ),
            "{field} = {value:?}: wrong refusal {error:?}"
        );
    }
    // Absence stays absent: the gate rules only on a present value.
    let mut minimal = base();
    minimal.media_type = None;
    minimal.platform = None;
    minimal.verification.evidence = None;
    validate(&minimal).unwrap();
}
