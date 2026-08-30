//! §6.3.0.4's "the three projection shapes are exact", proved as bytes.
//!
//! Every assertion here compares a STAGED file against the canonical source
//! it came from, or against a shape this engine fixes. An adapter that
//! re-encoded a manifest, moved a file one directory, or emitted a
//! component nobody requested parts from the canonical bytes and turns one
//! of these red — which is what makes "exact" a measurement rather than an
//! adjective.

use specmark::verifies;

use super::client::{
    CLAUDE_MANIFEST_DIR, CODEX_MANIFEST_DIR, DOT_MCP_MANIFEST, OPENCODE_CONFIG, PLUGIN_MANIFEST,
};
use super::support::*;
use crate::mechanism::package::support::{temp, write};

/// The engine-owned package directory of one target.
fn out(target: &str) -> String {
    format!("target/vibe-package/{target}")
}

/// The canonical source bytes of one tree-relative file.
fn authored(root: &std::path::Path, relative: &str) -> String {
    staged(root, &format!("{SOURCE}/{relative}"))
}

/// One path the projection must NOT hold.
fn absent(root: &std::path::Path, relative: &str) {
    assert!(
        !root.join(relative).exists(),
        "`{relative}` is absent from the projection by contract",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_claude_projection_is_the_exact_shape_6_3_freezes() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcome = project(
        root,
        "demo-claude",
        "package:claude-plugin",
        &["skills", "mcp"],
    );

    let out = out("demo-claude");
    assert_eq!(
        staged(
            root,
            &format!("{out}/{CLAUDE_MANIFEST_DIR}/{PLUGIN_MANIFEST}")
        ),
        authored(root, PLUGIN_MANIFEST),
        "the FULL canonical manifest bytes move to the hidden directory",
    );
    assert_eq!(
        staged(root, &format!("{out}/{DOT_MCP_MANIFEST}")),
        authored(root, "mcp.json"),
        "the canonical MCP declaration is copied byte-for-byte to `.mcp.json`",
    );
    assert_eq!(
        staged(root, &format!("{out}/skills/demo/SKILL.md")),
        authored(root, "skills/demo/SKILL.md"),
    );
    assert_eq!(
        staged(root, &format!("{out}/skills/demo/reference.md")),
        authored(root, "skills/demo/reference.md"),
        "a skill's whole tree is retained, not only its entry document",
    );
    absent(root, &format!("{out}/{PLUGIN_MANIFEST}"));
    absent(root, &format!("{out}/mcp.json"));
    absent(root, &format!("{out}/com.example.tools/extension.json"));
    assert_eq!(outcome.produced[0].files, 4);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_codex_projection_differs_from_claude_only_in_its_manifest_directory() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcome = project(
        root,
        "demo-codex",
        "package:codex-plugin",
        &["skills", "mcp"],
    );

    let out = out("demo-codex");
    assert_eq!(
        staged(
            root,
            &format!("{out}/{CODEX_MANIFEST_DIR}/{PLUGIN_MANIFEST}")
        ),
        authored(root, PLUGIN_MANIFEST),
    );
    assert_eq!(
        staged(root, &format!("{out}/{DOT_MCP_MANIFEST}")),
        authored(root, "mcp.json"),
    );
    absent(
        root,
        &format!("{out}/{CLAUDE_MANIFEST_DIR}/{PLUGIN_MANIFEST}"),
    );
    absent(root, &format!("{out}/mcp.json"));
    assert_eq!(outcome.produced[0].files, 4);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_opencode_projection_emits_skills_and_one_config_and_no_manifest() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcome = project(
        root,
        "demo-opencode",
        "package:opencode-plugin",
        &["skills", "mcp"],
    );

    let out = out("demo-opencode");
    assert_eq!(
        staged(root, &format!("{out}/skills/demo/SKILL.md")),
        authored(root, "skills/demo/SKILL.md"),
    );
    let config = staged(root, &format!("{out}/{OPENCODE_CONFIG}"));
    assert!(config.contains("\"mcp\""), "{config}");
    // §6.3.0.4: OpenCode "does not emit/call the unrelated npm/TypeScript
    // plugin API", and §6.3's frozen shape gives it no metadata file.
    absent(root, &format!("{out}/{PLUGIN_MANIFEST}"));
    absent(
        root,
        &format!("{out}/{CLAUDE_MANIFEST_DIR}/{PLUGIN_MANIFEST}"),
    );
    absent(
        root,
        &format!("{out}/{CODEX_MANIFEST_DIR}/{PLUGIN_MANIFEST}"),
    );
    absent(root, &format!("{out}/{DOT_MCP_MANIFEST}"));
    absent(root, &format!("{out}/mcp.json"));
    absent(root, &format!("{out}/com.example.tools/extension.json"));
    assert_eq!(
        outcome.produced[0].files, 3,
        "two skill files and one configuration fragment",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unselected_component_is_absent_from_every_client() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    for (id, mechanism, directory) in [
        (
            "skills-claude",
            "package:claude-plugin",
            CLAUDE_MANIFEST_DIR,
        ),
        ("skills-codex", "package:codex-plugin", CODEX_MANIFEST_DIR),
    ] {
        let outcome = project(root, id, mechanism, &["skills"]);
        let out = out(id);
        assert!(
            root.join(format!("{out}/{directory}/{PLUGIN_MANIFEST}"))
                .is_file(),
            "the manifest is the plugin's identity, not a portable component",
        );
        absent(root, &format!("{out}/{DOT_MCP_MANIFEST}"));
        assert_eq!(outcome.produced[0].files, 3);
    }

    let outcome = project(
        root,
        "skills-opencode",
        "package:opencode-plugin",
        &["skills"],
    );
    absent(
        root,
        &format!("{}/{OPENCODE_CONFIG}", out("skills-opencode")),
    );
    assert_eq!(outcome.produced[0].files, 2);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_projection_of_only_mcp_carries_no_skill() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    let outcome = project(root, "mcp-only", "package:opencode-plugin", &["mcp"]);

    let out = out("mcp-only");
    assert!(root.join(format!("{out}/{OPENCODE_CONFIG}")).is_file());
    absent(root, &format!("{out}/skills/demo/SKILL.md"));
    assert_eq!(outcome.produced[0].files, 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_reverse_domain_extension_directory_is_legal_and_never_projected() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);
    write(root, "plugin/org.example.other/data.json", "{}\n");

    let outcome = project(
        root,
        "extensions",
        "package:claude-plugin",
        &["skills", "mcp"],
    );

    let out = out("extensions");
    absent(root, &format!("{out}/com.example.tools/extension.json"));
    absent(root, &format!("{out}/org.example.other/data.json"));
    assert_eq!(
        outcome.produced[0].files, 4,
        "the two extension files are withheld by contract, not by accident",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_two_defined_placeholders_survive_projection_byte_for_value() {
    let home = temp();
    let root = home.path();
    write_full_plugin(root);

    project(
        root,
        "ph-claude",
        "package:claude-plugin",
        &["skills", "mcp"],
    );
    project(
        root,
        "ph-opencode",
        "package:opencode-plugin",
        &["skills", "mcp"],
    );

    let copied = staged(root, &format!("{}/{DOT_MCP_MANIFEST}", out("ph-claude")));
    assert!(copied.contains("${PLUGIN_ROOT}/bin/demo"), "{copied}");
    assert!(copied.contains("${PLUGIN_DATA}"), "{copied}");
    let translated = staged(root, &format!("{}/{OPENCODE_CONFIG}", out("ph-opencode")));
    assert!(
        translated.contains("${PLUGIN_ROOT}/bin/demo"),
        "runtime substitution belongs to the deploy adapter: {translated}",
    );
    assert!(translated.contains("${PLUGIN_DATA}"), "{translated}");
}
