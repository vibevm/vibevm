//! The two end-to-end packaging runs — declared source in, distributable
//! and A2 record out.
//!
//! They exist to prove the parts really compose: config, frontmatter,
//! includes, containment, the engine-owned output root, the record writer
//! and the A2 reader that will consume the record later. Everything they
//! assert is recomputed here from the produced bytes, so a record that
//! restated a plan value rather than digesting what was written would fail.

use specmark::verifies;
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, DigestAlgorithm, RelativeRoot,
    VerificationStatus,
};

use super::support::*;
use super::*;

/// Read back and validate the record one produced artifact left.
fn record(root: &Path, produced: &PackagedArtifact) -> ArtifactRecord {
    let bytes = match std::fs::read(root.join(&produced.record)) {
        Ok(bytes) => bytes,
        Err(error) => panic!("the artifact record reads: {error}"),
    };
    let record: ArtifactRecord = match serde_json::from_slice(&bytes) {
        Ok(record) => record,
        Err(error) => panic!("the artifact record parses: {error}"),
    };
    if let Err(error) = validate(&record) {
        panic!("the written record satisfies the A2 laws: {error}");
    }
    record
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_static_skill_becomes_one_framed_document_with_recorded_digests() {
    let root = temp();
    write_demo_skill(
        root.path(),
        "\nIntro.\n\n<!-- vibe:include reference.md -->\n\nOutro.\n",
    );
    write(root.path(), "skills/demo/reference.md", "Reference body.\n");
    let targets = vec![skill_target(
        "demo",
        "skills/demo",
        &["skills/demo/reference.md"],
    )];

    let outcomes = match run_default(root.path(), &targets) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the skill packages: {error}"),
    };

    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.target, "demo");
    assert_eq!(outcome.mechanism, "package:static-skill");
    assert_eq!(outcome.provider, "org.vibevm/vibe#static-skill");
    assert_eq!(outcome.displaced_default, None);
    assert_eq!(outcome.produced.len(), 1);
    let produced = &outcome.produced[0];
    assert_eq!(produced.id, "demo.md");
    assert_eq!(produced.path_relative, "target/vibe-package/demo/SKILL.md");

    // The document, and the digest of the bytes that are really there.
    let document = match std::fs::read(root.path().join(&produced.path_relative)) {
        Ok(bytes) => bytes,
        Err(error) => panic!("the distributable reads: {error}"),
    };
    assert_eq!(produced.bytes, document.len() as u64);
    let recomputed = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&document))
    };
    assert_eq!(produced.digest, recomputed);
    let text = String::from_utf8(document).unwrap_or_default();
    assert!(text.contains("Intro."), "{text}");
    assert!(text.contains("Reference body."), "{text}");
    assert!(text.contains("Outro."), "{text}");
    assert!(text.contains("vibe:included"), "{text}");

    // The record half.
    let record = record(root.path(), produced);
    assert_eq!(record.id, "demo.md");
    assert_eq!(record.kind, RecordKind::File);
    assert_eq!(record.shape, ArtifactShape::File);
    assert_eq!(record.digest.algorithm, DigestAlgorithm::Sha256);
    assert_eq!(record.digest.value, produced.digest);
    assert_eq!(record.path_relative.root, RelativeRoot::Project);
    assert_eq!(record.producer.mechanism, "package:static-skill");
    assert_eq!(record.producer.provider.key, "org.vibevm/vibe#static-skill");
    assert_eq!(record.verification.status, VerificationStatus::Verified);
    assert_eq!(record.media_type.as_deref(), Some("text/markdown"));
    assert_eq!(record.platform, None);

    // Engine-fresh, and the record says so by PRESENCE: the input census
    // is complete and hashable (§4.1), and no toolchain took part.
    assert_eq!(record.freshness.inputs.as_ref().map(String::len), Some(64));
    assert_eq!(record.freshness.config.as_ref().map(String::len), Some(64));
    assert_eq!(record.freshness.toolchain, None);
    let evidence = record
        .verification
        .evidence
        .as_deref()
        .unwrap_or("<no evidence>");
    assert!(
        evidence.contains("org.vibevm/vibe#static-skill"),
        "{evidence}"
    );
    assert!(
        evidence.contains("engine-fresh over 2 declared input(s)"),
        "{evidence}"
    );
    assert!(evidence.contains("workspace-path=1"), "{evidence}");
    assert!(evidence.contains("network=never"), "{evidence}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn an_agent_plugin_becomes_a_directory_with_a_canonical_tree_digest() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/mcp.json",
        "{ \"mcpServers\": { \"demo\": { \"command\": \"${PLUGIN_ROOT}/bin/demo\" } } }\n",
    );
    let targets = vec![plugin_target("demo-plugin", "plugin", Vec::new(), &[])];

    let outcomes = match run_default(root.path(), &targets) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the plugin packages: {error}"),
    };

    assert_eq!(outcomes.len(), 1);
    let produced = &outcomes[0].produced[0];
    assert_eq!(produced.id, "demo-plugin.dir");
    assert_eq!(produced.path_relative, "target/vibe-package/demo-plugin");
    assert_eq!(produced.files, 3);

    let record = record(root.path(), produced);
    assert_eq!(record.kind, RecordKind::Directory);
    assert_eq!(record.shape, ArtifactShape::Directory);
    assert_eq!(
        record.digest.algorithm,
        DigestAlgorithm::Sha256Tree,
        "a directory carries the canonical tree digest, never a file's SHA-256",
    );
    assert_eq!(record.digest.value, produced.digest);
    assert_eq!(record.producer.mechanism, "package:agent-plugin");
    assert_eq!(record.producer.provider.key, "org.vibevm/vibe#agent-plugin");
    assert_eq!(
        record.media_type, None,
        "a directory declares no media type"
    );
    assert_eq!(record.freshness.inputs.as_ref().map(String::len), Some(64));
    assert_eq!(record.freshness.toolchain, None);
    let evidence = record
        .verification
        .evidence
        .as_deref()
        .unwrap_or("<no evidence>");
    assert!(evidence.contains("3 file(s)"), "{evidence}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn one_package_target_consumes_another_package_target_s_output_through_its_record() {
    // The phase-forward law inside one family: the second target reads the
    // FIRST target's A2 record, which the same executor wrote a moment
    // earlier — one door for a consumed artifact, whoever produced it.
    let root = temp();
    write_demo_skill(root.path(), "\nBody.\n");
    write_demo_plugin(root.path());
    let targets = vec![
        plugin_target(
            "bundle",
            "plugin",
            vec![vibe_core::manifest::ArtifactInput::Artifact {
                artifact: "demo.md".to_owned(),
            }],
            &[("demo.md", "com.example.tools/demo-skill.md")],
        ),
        skill_target("demo", "skills/demo", &[]),
    ];

    let outcomes = match run_default(root.path(), &targets) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the chained package graph executes: {error}"),
    };

    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].target, "demo", "the producer ran first");
    assert_eq!(outcomes[1].target, "bundle");
    let staged = root
        .path()
        .join("target/vibe-package/bundle/com.example.tools/demo-skill.md");
    let placed = match std::fs::read_to_string(&staged) {
        Ok(text) => text,
        Err(error) => panic!("the placed skill reads: {error}"),
    };
    assert!(placed.starts_with("---\nname: demo\n"), "{placed}");
}
