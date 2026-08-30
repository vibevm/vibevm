//! The projections' ADMISSION laws: what may enter one, and what the
//! strict `components` table may say.
//!
//! This cell carries the atom's sharpest mutations. Weaken the provenance
//! gate — admit a workspace path, or drop the recorded-kind check — and
//! [`a_workspace_path_is_never_a_canonical_agent_plugin`] or
//! [`a_projection_can_never_be_fed_a_projection`] goes red, because both
//! inputs are physically directories that look exactly like a canonical
//! plugin. Loosen the component set and
//! [`the_component_set_is_strict_in_every_direction`] follows.

use specmark::verifies;
use vibe_core::manifest::{ArtifactInput, ArtifactKind};

use super::ClientProjectionProvider;
use super::client::ProjectionClient;
use super::error::ClientProjectionError;
use super::support::*;
use crate::mechanism::package::support::{config, temp, write};
use crate::mechanism::{MechanismError, PackageProvider};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_workspace_path_is_never_a_canonical_agent_plugin() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    let mut target = projection_target("wrong", "package:claude-plugin", &["skills"]);
    // A real file of the real plugin source — the friendliest possible
    // wrong answer, and still refused: nothing RECORDED it as a plugin.
    target.inputs = Some(vec![ArtifactInput::Path {
        path: std::path::PathBuf::from("plugin/plugin.json"),
    }]);

    let error = run(root, vec![target]).expect_err("a workspace path carries no recorded kind");

    match capability(error) {
        ClientProjectionError::InputNotAgentPlugin { found, input, .. } => {
            assert_eq!(input, "plugin/plugin.json");
            assert!(found.contains("no recorded kind"), "{found}");
        }
        other => panic!("expected the provenance refusal, got {other}"),
    }
    assert!(!root.join("target/vibe-package/wrong").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_projection_can_never_be_fed_a_projection() {
    // A projection's own output is a recorded `directory` that is
    // physically indistinguishable from a canonical plugin — and is not
    // one. This is the RECORDED half of the same provenance law.
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    let first = projection_target("first", "package:claude-plugin", &["skills", "mcp"]);
    let mut second = projection_target("second", "package:codex-plugin", &["skills"]);
    second.inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "first.dir".to_owned(),
    }]);

    let error = run(root, vec![first, second])
        .expect_err("a recorded plain directory is not a canonical Agent Plugin");

    match capability(error) {
        ClientProjectionError::InputNotAgentPlugin { found, input, .. } => {
            assert_eq!(input, "first.dir");
            assert_eq!(found, "a recorded `directory` artifact");
        }
        other => panic!("expected the provenance refusal, got {other}"),
    }
    assert!(!root.join("target/vibe-package/second").exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_record_claiming_an_agent_plugin_file_refuses_on_its_shape() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    write(root, "loose/plugin.json", "{}\n");
    file_shaped_plugin_record(root, "loose.plugin", "loose/plugin.json");
    let mut target = projection_target("shape", "package:claude-plugin", &["skills"]);
    target.inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "loose.plugin".to_owned(),
    }]);

    let error = run(root, vec![target]).expect_err("Agent Plugins 1.0 defines a directory");

    match capability(error) {
        ClientProjectionError::InputNotDirectory { input, shape, .. } => {
            assert_eq!(input, "loose.plugin");
            assert_eq!(shape, "file");
        }
        other => panic!("expected the shape refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_second_input_or_none_at_all_refuses_by_count() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    write(root, "assets/note.txt", "asset\n");

    for (id, inputs, expected) in [
        ("none", Some(Vec::new()), 0_usize),
        (
            "two",
            Some(vec![
                ArtifactInput::Artifact {
                    artifact: CANONICAL_ARTIFACT.to_owned(),
                },
                ArtifactInput::Path {
                    path: std::path::PathBuf::from("assets/note.txt"),
                },
            ]),
            2,
        ),
    ] {
        let mut target = projection_target(id, "package:claude-plugin", &["skills"]);
        target.inputs = inputs;

        let error = run(root, vec![target]).expect_err("a projection adapts exactly one plugin");

        match capability(error) {
            ClientProjectionError::InputCount {
                found, provider, ..
            } => {
                assert_eq!(found, expected);
                assert_eq!(provider, "org.vibevm/vibe#claude-plugin-projection");
            }
            other => panic!("expected the input-count refusal, got {other}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_requested_component_the_plugin_cannot_supply_refuses_by_name() {
    for (component, id) in [("skills", "no-skills"), ("mcp", "no-mcp")] {
        let home = temp();
        let root = home.path();
        write_bare_plugin(root);

        let error = run(
            root,
            vec![projection_target(
                id,
                "package:opencode-plugin",
                &[component],
            )],
        )
        .expect_err("no adapter silently drops a requested component");

        match capability(error) {
            ClientProjectionError::ComponentMissing {
                component: named,
                client,
                ..
            } => {
                assert_eq!(named, component);
                assert_eq!(client, "opencode");
            }
            other => panic!("expected the capability report, got {other}"),
        }
        assert!(
            !root.join(format!("target/vibe-package/{id}")).exists(),
            "the refusal precedes output creation",
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_component_set_is_strict_in_every_direction() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    for (id, table, member) in [
        ("empty", "components = []\n", "components"),
        (
            "duplicate",
            "components = [\"skills\", \"skills\"]\n",
            "components[1]",
        ),
        (
            "unknown-value",
            "components = [\"hooks\"]\n",
            "components[0]",
        ),
        ("scalar", "components = \"skills\"\n", "components"),
        ("unknown-member", "client = \"claude\"\n", "client"),
        ("engine-owned", "output_dir = \"x\"\n", "output_dir"),
        ("absent", "", "components"),
    ] {
        let mut target = projection_target(id, "package:claude-plugin", &["skills"]);
        target.config = if table.is_empty() {
            None
        } else {
            Some(config(table))
        };

        let error = run(root, vec![target]).expect_err("the projection config is strict");

        match refusal(error) {
            MechanismError::Config {
                member: named,
                reason,
                ..
            } => {
                assert_eq!(named, member, "for `{id}`");
                assert!(!reason.is_empty(), "for `{id}`");
            }
            other => panic!("expected a config refusal for `{id}`, got {other}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_output_kind_a_projection_does_not_produce_refuses() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    let mut target = projection_target("kind", "package:claude-plugin", &["skills"]);
    // Not `agent-plugin`: a projection is a client-native tree, and
    // recording one as a canonical plugin is what would let it be projected
    // again.
    target.outputs[0].kind = ArtifactKind::AgentPlugin;

    let error = run(root, vec![target]).expect_err("a projection is a plain directory");

    match refusal(error) {
        MechanismError::UnsupportedKind {
            kind, supported, ..
        } => {
            assert_eq!(kind, "agent-plugin");
            assert_eq!(supported, "directory");
        }
        other => panic!("expected the kind refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn every_projection_provider_declares_the_workspace_only_posture() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    all_three(root);

    for client in [
        ProjectionClient::Claude,
        ProjectionClient::Codex,
        ProjectionClient::OpenCode,
    ] {
        let posture = ClientProjectionProvider::new(client).descriptor().posture();
        assert!(posture.contains("effect=workspace"), "{posture}");
        assert!(posture.contains("network=never"), "{posture}");
        assert!(posture.contains("privilege=none"), "{posture}");
        assert!(posture.contains("reversibility=n/a"), "{posture}");
        assert!(
            posture.contains("ops=plan+fingerprint+apply+verify"),
            "{posture}",
        );
    }
    // §6.3.0.1 keeps every home, client and destination effect in the
    // DEPLOY lane. A package-phase run leaves no deploy state at all.
    assert!(!root.join(".vibe/state/deploy").exists());
    assert!(!root.join(".claude").exists());
    assert!(!root.join(".codex").exists());
    assert!(!root.join(".config").exists());
}

/// The crate's public re-export really names this adapter family's error,
/// so a caller can match a capability report without reaching inside.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_capability_report_is_a_public_value() {
    let refusal: crate::ClientProjectionError = ClientProjectionError::ComponentMissing {
        target: "demo".to_owned(),
        client: "codex",
        component: "skills",
        reason: "none declared".to_owned(),
    };

    let rendered = MechanismError::from(refusal).to_string();

    assert!(rendered.contains("capability"), "{rendered}");
    assert!(rendered.contains("PROP-054#ONE-MACHINE"), "{rendered}");
}
