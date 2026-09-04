//! The engine's own mechanism SOURCE — what vibe ships, in what order, and
//! under which descriptor.
//!
//! Split from the collection walk next door because they are two
//! responsibilities: that cell proves how three sources become one
//! registry, and this one proves what the first of those sources contains.
//! The table is a frozen list read out of the architecture, so its pins are
//! long and exact, and they belong where a reader looking for "what does
//! vibe ship" will find them.

use specmark::verifies;
use vibe_core::manifest::{ExtensionsControl, MechanismFreshness};

use crate::{MechanismProvider, builtin_mechanism_source, collect_mechanisms};

use super::support::{host, mechanism_key, provider_pin, world};

fn pins(registry: &crate::MechanismRegistry) -> Vec<String> {
    registry
        .rows()
        .iter()
        .map(|row| row.pin().to_string())
        .collect()
}

/// The engine's reserved identity is a real, parseable provider identity and
/// every shipped row is keyed under it — the machinery behind spelling the
/// reserved owner as a constant instead of a runtime-parsed typed value.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_reserved_identity_is_spellable_and_stable() {
    let source = builtin_mechanism_source();
    assert_eq!(source.owner(), "org.vibevm/vibe");

    let registry = collect_mechanisms(&world(
        Vec::new(),
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("an empty world still collects the engine's own source");
    assert_eq!(
        pins(&registry),
        [
            // The five historical rows, in their historical positions.
            "org.vibevm/vibe#cargo",
            "org.vibevm/vibe#static-skill",
            "org.vibevm/vibe#agent-plugin",
            "org.vibevm/vibe#vibe-bin",
            "org.vibevm/vibe#windows-zip",
            // §6.3.0.2's nine, appended and never interleaved.
            "org.vibevm/vibe#claude-plugin-projection",
            "org.vibevm/vibe#codex-plugin-projection",
            "org.vibevm/vibe#opencode-plugin-projection",
            "org.vibevm/vibe#claude-skill",
            "org.vibevm/vibe#codex-skill",
            "org.vibevm/vibe#opencode-skill",
            "org.vibevm/vibe#claude-plugin",
            "org.vibevm/vibe#codex-plugin",
            "org.vibevm/vibe#opencode-plugin",
            // §13.1's two rows, appended after all fourteen incumbents.
            "org.vibevm/vibe#static-file",
            "org.vibevm/vibe#vibe-opt-launcher",
        ],
        "the sixteen shipped rows, in the order the collector appends them"
    );
    assert_eq!(
        registry
            .rows()
            .iter()
            .map(|row| row.key().to_string())
            .collect::<Vec<_>>(),
        [
            "build:cargo",
            "package:static-skill",
            "package:agent-plugin",
            "deploy:vibe-bin",
            "package:windows-zip",
            "package:claude-plugin",
            "package:codex-plugin",
            "package:opencode-plugin",
            "deploy:claude-skill",
            "deploy:codex-skill",
            "deploy:opencode-skill",
            "deploy:claude-plugin",
            "deploy:codex-plugin",
            "deploy:opencode-plugin",
            "package:static-file",
            "deploy:vibe-opt-launcher",
        ],
    );
    for row in registry.rows() {
        assert!(row.is_builtin());
        assert_eq!(row.provider(), &MechanismProvider::Builtin);
        assert_eq!(row.handler().kind(), "builtin");
        assert_eq!(row.protocol(), 1);
        assert!(row.provider_ordinal().is_none());
        assert!(row.is_enabled());
    }
}

/// Every shipped row's DESCRIPTOR is read out of the architecture, not
/// chosen at the table: §4.1 rules Cargo provider-fresh, §§6.1–6.2 rule
/// the two §6 packaging rows engine-fresh because their input sets are
/// closed and hashable, §7.0.8 rules the archive row engine-fresh for the
/// same reason, and a deploy target reconciles state no engine census can
/// hash. §6.3.0.2 rules the nine client rows in the same words —
/// "Projection rows are engine-fresh; destination rows are provider-fresh."
/// §13.1 applies that split to static-file and vibe-opt-launcher.
/// The config-schema spellings are engine-owned identities under
/// `schemas/mechanism/`, in one snake_case shape keyed by the PROVIDER id.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn every_shipped_row_declares_the_freshness_and_schema_the_architecture_gives_it() {
    let source = builtin_mechanism_source();
    let rows: Vec<(&str, MechanismFreshness, String)> = source
        .declarations()
        .iter()
        .map(|declaration| {
            (
                declaration.id.as_str(),
                declaration.freshness,
                declaration
                    .config_schema
                    .display()
                    .to_string()
                    .replace('\\', "/"),
            )
        })
        .collect();

    assert_eq!(
        rows,
        vec![
            (
                "cargo",
                MechanismFreshness::Provider,
                "schemas/mechanism/build_cargo.jtd.json".to_string(),
            ),
            (
                "static-skill",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_static_skill.jtd.json".to_string(),
            ),
            (
                "agent-plugin",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_agent_plugin.jtd.json".to_string(),
            ),
            (
                "vibe-bin",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_vibe_bin.jtd.json".to_string(),
            ),
            (
                "windows-zip",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_windows_zip.jtd.json".to_string(),
            ),
            (
                "claude-plugin-projection",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_claude_plugin_projection.jtd.json".to_string(),
            ),
            (
                "codex-plugin-projection",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_codex_plugin_projection.jtd.json".to_string(),
            ),
            (
                "opencode-plugin-projection",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_opencode_plugin_projection.jtd.json".to_string(),
            ),
            (
                "claude-skill",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_claude_skill.jtd.json".to_string(),
            ),
            (
                "codex-skill",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_codex_skill.jtd.json".to_string(),
            ),
            (
                "opencode-skill",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_opencode_skill.jtd.json".to_string(),
            ),
            (
                "claude-plugin",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_claude_plugin.jtd.json".to_string(),
            ),
            (
                "codex-plugin",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_codex_plugin.jtd.json".to_string(),
            ),
            (
                "opencode-plugin",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_opencode_plugin.jtd.json".to_string(),
            ),
            (
                "static-file",
                MechanismFreshness::Engine,
                "schemas/mechanism/package_static_file.jtd.json".to_string(),
            ),
            (
                "vibe-opt-launcher",
                MechanismFreshness::Provider,
                "schemas/mechanism/deploy_vibe_opt_launcher.jtd.json".to_string(),
            ),
        ],
    );
}

/// §6.3.0.2's own sentence, as a test: "The first three rows deliberately
/// prove that provider id and logical name are separate fields."
///
/// The projection row is keyed `org.vibevm/vibe#claude-plugin-projection`
/// and services the logical capability `package:claude-plugin`; the DEPLOY
/// row keyed `org.vibevm/vibe#claude-plugin` services
/// `deploy:claude-plugin`. Two rows, one reserved owner, two distinct `#id`
/// spellings — which is exactly why the projection could not be named after
/// the capability it defaults for.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_client_projection_row_proves_provider_id_and_logical_name_are_separate() {
    let registry = collect_mechanisms(&world(
        Vec::new(),
        host(Vec::new(), ExtensionsControl::default()),
        None,
    ))
    .expect("the engine's own source collects");

    let projection = registry
        .find(&provider_pin("org.vibevm/vibe#claude-plugin-projection"))
        .expect("the projection row is keyed by its own provider id");
    assert_eq!(projection.declaration().id, "claude-plugin-projection");
    assert_eq!(projection.logical_name(), "claude-plugin");
    assert_ne!(
        projection.declaration().id.as_str(),
        projection.logical_name(),
        "a projection row's id is NOT its logical name",
    );
    assert_eq!(projection.key(), &mechanism_key("package:claude-plugin"));
    assert_eq!(projection.handler().kind(), "builtin");

    // The same logical spelling under the OTHER role is a different row,
    // and the builtin default of each key is the row that declares it.
    let destination = registry
        .find(&provider_pin("org.vibevm/vibe#claude-plugin"))
        .expect("the destination row keeps the bare id");
    assert_eq!(destination.key(), &mechanism_key("deploy:claude-plugin"));
    assert_eq!(destination.logical_name(), "claude-plugin");
    assert_eq!(
        registry
            .builtin_default(&mechanism_key("package:claude-plugin"))
            .map(|row| row.pin().to_string()),
        Some("org.vibevm/vibe#claude-plugin-projection".to_owned()),
    );
    assert_eq!(
        registry
            .builtin_default(&mechanism_key("deploy:claude-plugin"))
            .map(|row| row.pin().to_string()),
        Some("org.vibevm/vibe#claude-plugin".to_owned()),
    );
}
