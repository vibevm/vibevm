//! What three projections LEAVE BEHIND: the records, the evidence and the
//! epoch-1 fingerprint.
//!
//! This cell carries the fingerprint mutations. Drop the client or the
//! adapter epoch from the digest and
//! [`the_three_clients_fingerprint_one_source_differently`] or
//! [`the_fingerprint_is_exactly_the_algorithm_this_adapter_specifies`]
//! parts from its independently recomputed oracle — the second one catches
//! a removal the first cannot, because two clients can still differ from
//! each other while the epoch has quietly stopped being input.

use sha2::{Digest, Sha256};
use specmark::verifies;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactShape, DigestAlgorithm, VerificationStatus,
};

use super::ADAPTER_EPOCH;
use super::support::*;
use crate::mechanism::package::support::temp;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_canonical_plugin_records_the_agent_plugin_kind_and_a_directory_shape() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcomes = match run(root, Vec::new()) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the canonical plugin packages: {error}"),
    };

    let produced = &outcome(&outcomes, CANONICAL).produced[0];
    let record = record(root, produced);
    assert_eq!(
        record.kind,
        RecordKind::AgentPlugin,
        "§6.2's own contract: the record's kind is `agent-plugin`",
    );
    assert_eq!(
        record.shape,
        ArtifactShape::Directory,
        "the PHYSICAL shape is still the directory Agent Plugins 1.0 defines",
    );
    assert_eq!(record.digest.algorithm, DigestAlgorithm::Sha256Tree);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn all_three_projections_execute_through_normal_package_dispatch() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcomes = all_three(root);

    assert_eq!(outcomes.len(), 4, "the canonical target and three clients");
    assert_eq!(
        outcomes[0].target, CANONICAL,
        "dependency order runs the producer first",
    );
    for (id, mechanism, client) in CLIENTS {
        let outcome = outcome(&outcomes, id);
        assert_eq!(outcome.mechanism, mechanism);
        assert_eq!(
            outcome.provider,
            format!("org.vibevm/vibe#{client}-plugin-projection"),
            "§6.3.0.2: a provider id is not its logical name",
        );
        assert_eq!(outcome.via, "the shipped builtin default");
        assert_eq!(outcome.displaced_default, None);
        let produced = &outcome.produced[0];
        let record = record(root, produced);
        assert_eq!(record.id, format!("{id}.dir"));
        assert_eq!(
            record.kind,
            RecordKind::Directory,
            "a projection is a client-native tree, never a canonical plugin",
        );
        assert_eq!(record.shape, ArtifactShape::Directory);
        assert_eq!(record.digest.algorithm, DigestAlgorithm::Sha256Tree);
        assert_eq!(record.digest.value, produced.digest);
        assert_eq!(record.verification.status, VerificationStatus::Verified);
        assert_eq!(record.media_type, None);
        assert_eq!(record.platform, None);
        assert_eq!(record.freshness.inputs.as_ref().map(String::len), Some(64));
        assert_eq!(record.freshness.config.as_ref().map(String::len), Some(64));
        assert_eq!(record.freshness.toolchain, None, "no toolchain took part");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn every_projection_states_its_client_epoch_identity_components_and_census() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcomes = all_three(root);

    for (id, _, client) in CLIENTS {
        let evidence = record(root, &outcome(&outcomes, id).produced[0])
            .verification
            .evidence
            .unwrap_or_else(|| "<no evidence>".to_owned());
        for expected in [
            client,
            &format!("adapter epoch {ADAPTER_EPOCH}"),
            "demo-plugin",
            "1.4.2",
            "components [skills, mcp]",
            "withheld by contract",
            "network=never",
            "effect=workspace",
            "engine-fresh over 1 declared input(s)",
            "artifact-record=1 workspace-path=0",
        ] {
            assert!(evidence.contains(expected), "`{expected}` in `{evidence}`");
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_three_clients_fingerprint_one_source_differently() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcomes = all_three(root);

    let digests: Vec<String> = CLIENTS
        .iter()
        .map(|(id, _, _)| {
            record(root, &outcome(&outcomes, id).produced[0])
                .freshness
                .inputs
                .unwrap_or_default()
        })
        .collect();
    assert_ne!(digests[0], digests[1], "claude and codex differ");
    assert_ne!(digests[1], digests[2], "codex and opencode differ");
    assert_ne!(digests[0], digests[2], "claude and opencode differ");
}

/// The DIFFERENTIAL ORACLE for the epoch-1 projection fingerprint.
///
/// The algorithm is specified in prose on the provider's own `fingerprint`;
/// this recomputes it here from values read out of the finished run,
/// independently of the production code. Remove the client, remove the
/// epoch, reorder the component set or drop the canonical digest, and the
/// two answers part — which is what makes §6.3.0.4's "records adapter epoch
/// 1 in its fingerprint" runnable capital rather than a sentence.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_fingerprint_is_exactly_the_algorithm_this_adapter_specifies() {
    const DOMAIN: &str = "client-plugin-projection/1";

    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcomes = all_three(root);
    let canonical = outcome(&outcomes, CANONICAL).produced[0].digest.clone();

    for (id, _, client) in CLIENTS {
        let mut hash = Sha256::new();
        for (field, value) in [
            ("client", client.to_owned()),
            ("adapter-epoch", ADAPTER_EPOCH.to_string()),
            ("plugin", canonical.clone()),
            ("name", "demo-plugin".to_owned()),
            ("version", "1.4.2".to_owned()),
        ] {
            hash.update(DOMAIN.as_bytes());
            hash.update(b"\x00");
            hash.update(field.as_bytes());
            hash.update(b"\x00");
            hash.update(value.as_bytes());
            hash.update(b"\x00");
        }
        for component in ["skills", "mcp"] {
            hash.update(DOMAIN.as_bytes());
            hash.update(b"\x00component\x00");
            hash.update(component.as_bytes());
            hash.update(b"\x00");
        }
        let expected = format!("{:x}", hash.finalize());
        let recorded = record(root, &outcome(&outcomes, id).produced[0])
            .freshness
            .inputs
            .unwrap_or_default();
        assert_eq!(recorded, expected, "the recorded fingerprint for {client}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_authored_component_order_cannot_change_the_projection_or_its_fingerprint() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let forward = project(root, "order-a", "package:claude-plugin", &["skills", "mcp"]);
    let reversed = project(root, "order-b", "package:claude-plugin", &["mcp", "skills"]);

    assert_eq!(
        forward.produced[0].digest, reversed.produced[0].digest,
        "one requested SET is one projection",
    );
    let left = record(root, &forward.produced[0]).freshness.inputs;
    let right = record(root, &reversed.produced[0]).freshness.inputs;
    assert_eq!(
        left, right,
        "`components` is a set, so its order is not input"
    );
}
