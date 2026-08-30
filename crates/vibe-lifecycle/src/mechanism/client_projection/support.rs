//! Shared fixtures for the client-projection suites.
//!
//! They build on the package phase's own fixture home rather than beside
//! it: a projection consumes what `package:agent-plugin` produced, so the
//! only honest fixture is the real chain — canonical target first, its A2
//! record written by the real executor, projection target second. A hand
//! written record would prove the adapter reads A record, not that it reads
//! THE record the canonical provider writes.

use sha2::{Digest, Sha256};
use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, MechanismRoutes,
};
use vibe_wire::behaviour::artifact_record::validate;
use vibe_wire::generated::artifact_record::{
    ArtifactKind as RecordKind, ArtifactRecord, ArtifactShape, ContentDigest, DigestAlgorithm,
    FreshnessFingerprints, ProducerIdentity, ProviderIdentity, RelativeIdentity, RelativeRoot,
    VerificationState, VerificationStatus,
};

use crate::PackageError;
use crate::mechanism::MechanismError;
use crate::mechanism::client_projection::ClientProjectionError;
use crate::mechanism::package::support::{config, key, plugin_target, write};
use crate::mechanism::package::{PackageOutcome, PackagedArtifact, execute_package_targets};

/// The three client rows, as a projection graph declares them: target id,
/// logical mechanism key, and the client's own word.
pub(crate) const CLIENTS: [(&str, &str, &str); 3] = [
    ("p-claude", "package:claude-plugin", "claude"),
    ("p-codex", "package:codex-plugin", "codex"),
    ("p-opencode", "package:opencode-plugin", "opencode"),
];

/// The canonical target's id, and therefore its artifact id `<id>.dir`.
pub(crate) const CANONICAL: &str = "demo-plugin";

/// The artifact id the canonical target produces.
pub(crate) const CANONICAL_ARTIFACT: &str = "demo-plugin.dir";

/// The canonical source tree's project-relative root.
pub(crate) const SOURCE: &str = "plugin";

/// One canonical `package:agent-plugin` target over the fixture tree.
pub(crate) fn canonical_target() -> ArtifactPackageTarget {
    plugin_target(CANONICAL, SOURCE, Vec::new(), &[])
}

/// One client-projection target consuming the canonical plugin.
pub(crate) fn projection_target(
    id: &str,
    mechanism: &str,
    components: &[&str],
) -> ArtifactPackageTarget {
    let rendered = components
        .iter()
        .map(|component| format!("\"{component}\""))
        .collect::<Vec<_>>()
        .join(", ");
    ArtifactPackageTarget {
        id: id.to_owned(),
        mechanism: key(mechanism),
        provider: None,
        inputs: Some(vec![ArtifactInput::Artifact {
            artifact: CANONICAL_ARTIFACT.to_owned(),
        }]),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.dir"),
            kind: ArtifactKind::Directory,
            select: None,
        }],
        config: Some(config(&format!("components = [{rendered}]\n"))),
    }
}

/// The canonical fixture plugin: a manifest, one skill, one local and one
/// remote MCP server, and a reverse-domain client-extension directory that
/// no projection may emit.
pub(crate) fn write_full_plugin(root: &std::path::Path) {
    write(
        root,
        "plugin/plugin.json",
        "{\n  \"name\": \"demo-plugin\",\n  \"version\": \"1.4.2\",\n  \
         \"description\": \"A demonstration plugin.\"\n}\n",
    );
    write(
        root,
        "plugin/skills/demo/SKILL.md",
        "---\nname: demo\ndescription: A packaged skill.\n---\n\nBody.\n",
    );
    write(root, "plugin/skills/demo/reference.md", "Reference.\n");
    write(root, "plugin/mcp.json", FULL_MCP);
    write(
        root,
        "plugin/com.example.tools/extension.json",
        "{ \"client\": \"only\" }\n",
    );
}

/// The fixture MCP declaration: one local server with `args` and `env`, one
/// remote server with `headers`, both carrying the two defined placeholders
/// so their preservation is observable.
pub(crate) const FULL_MCP: &str = concat!(
    "{\n  \"mcpServers\": {\n",
    "    \"zeta\": {\n      \"url\": \"https://example.test/mcp\",\n",
    "      \"headers\": { \"X-Trace\": \"on\", \"Authorization\": \"Bearer ${PLUGIN_DATA}\" }\n",
    "    },\n",
    "    \"alpha\": {\n      \"command\": \"${PLUGIN_ROOT}/bin/demo\",\n",
    "      \"args\": [\"--data\", \"${PLUGIN_DATA}\"],\n",
    "      \"env\": { \"DEMO_MODE\": \"on\", \"AAA\": \"1\" }\n",
    "    }\n  }\n}\n",
);

/// A plugin with a manifest and nothing else — neither skills nor MCP.
pub(crate) fn write_bare_plugin(root: &std::path::Path) {
    write(
        root,
        "plugin/plugin.json",
        "{\n  \"name\": \"bare-plugin\",\n  \"version\": \"0.2.0\"\n}\n",
    );
}

/// Run the canonical target and every named projection, in one graph.
pub(crate) fn run(
    root: &std::path::Path,
    projections: Vec<ArtifactPackageTarget>,
) -> Result<Vec<PackageOutcome>, PackageError> {
    let world = crate::mechanism::package::support::empty_world();
    let registry = crate::mechanism::package::support::registry(&world);
    let routes = MechanismRoutes::default();
    let mut targets = vec![canonical_target()];
    targets.extend(projections);
    execute_package_targets(&crate::mechanism::package::support::execution(
        root, &targets, &registry, &routes,
    ))
}

/// Run the canonical target plus one projection and return that
/// projection's outcome.
pub(crate) fn project(
    root: &std::path::Path,
    id: &str,
    mechanism: &str,
    components: &[&str],
) -> PackageOutcome {
    let targets = vec![projection_target(id, mechanism, components)];
    match run(root, targets) {
        Ok(mut outcomes) if outcomes.len() == 2 => outcomes.swap_remove(1),
        Ok(other) => panic!("expected the canonical target and one projection, got {other:?}"),
        Err(error) => panic!("the projection runs: {error}"),
    }
}

/// The projection refusal one run produced.
pub(crate) fn capability(error: PackageError) -> ClientProjectionError {
    match error {
        PackageError::Provider(MechanismError::Projection(inner)) => inner,
        other => panic!("expected a client-projection refusal, got {other}"),
    }
}

/// The provider refusal one run produced.
pub(crate) fn refusal(error: PackageError) -> MechanismError {
    match error {
        PackageError::Provider(inner) => inner,
        other => panic!("expected a provider refusal, got {other}"),
    }
}

/// Read one staged projection file as text.
pub(crate) fn staged(root: &std::path::Path, relative: &str) -> String {
    match std::fs::read_to_string(root.join(relative)) {
        Ok(text) => text,
        Err(error) => panic!("`{relative}` reads: {error}"),
    }
}

/// Run the canonical plugin and all three client projections in one graph.
pub(crate) fn all_three(root: &std::path::Path) -> Vec<PackageOutcome> {
    let targets = CLIENTS
        .iter()
        .map(|(id, mechanism, _)| projection_target(id, mechanism, &["skills", "mcp"]))
        .collect();
    match run(root, targets) {
        Ok(outcomes) => outcomes,
        Err(error) => panic!("the three projections run: {error}"),
    }
}

/// One outcome of a multi-target run, by target id.
pub(crate) fn outcome<'a>(outcomes: &'a [PackageOutcome], target: &str) -> &'a PackageOutcome {
    match outcomes.iter().find(|outcome| outcome.target == target) {
        Some(found) => found,
        None => panic!("`{target}` ran: {outcomes:?}"),
    }
}

/// Read back and validate the record one produced artifact left.
pub(crate) fn record(root: &std::path::Path, produced: &PackagedArtifact) -> ArtifactRecord {
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

/// One hand-written record claiming an `agent-plugin` that is a FILE — the
/// state only a broken or hand-made record can reach, and the reason the
/// shape half of the provenance gate is a refusal rather than an assertion.
pub(crate) fn file_shaped_plugin_record(root: &std::path::Path, id: &str, relative: &str) {
    let bytes = match std::fs::read(root.join(relative)) {
        Ok(bytes) => bytes,
        Err(error) => panic!("the fixture file reads: {error}"),
    };
    let record = ArtifactRecord {
        schema: 1,
        id: id.to_owned(),
        kind: RecordKind::AgentPlugin,
        shape: ArtifactShape::File,
        path_absolute: crate::mechanism::contain::forward_slashed(&root.join(relative)),
        path_relative: RelativeIdentity {
            root: RelativeRoot::Project,
            path: relative.to_owned(),
        },
        digest: ContentDigest {
            algorithm: DigestAlgorithm::Sha256,
            value: format!("{:x}", Sha256::digest(&bytes)),
        },
        producer: ProducerIdentity {
            target: "hand-made".to_owned(),
            mechanism: "package:agent-plugin".to_owned(),
            provider: ProviderIdentity {
                key: "org.vibevm/vibe#agent-plugin".to_owned(),
                version: None,
                content_hash: None,
            },
        },
        freshness: FreshnessFingerprints {
            inputs: None,
            config: None,
            toolchain: None,
        },
        created_at: match "2026-08-30T00:00:00Z".parse() {
            Ok(stamp) => stamp,
            Err(error) => panic!("the fixture clock parses: {error}"),
        },
        verification: VerificationState {
            status: VerificationStatus::Verified,
            evidence: None,
        },
        media_type: None,
        platform: None,
    };
    if let Err(error) = crate::mechanism::record::write_record(root, &record) {
        panic!("the fixture record publishes: {error}");
    }
}
