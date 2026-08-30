//! §6.2's laws, sentence by sentence.
//!
//! Two of them carry the atom's mutations. Make the canonical directory
//! digest depend on the walk — or let it skip a file — and
//! [`the_canonical_digest_does_not_depend_on_creation_order`] or
//! [`a_file_the_digest_would_skip_changes_both_the_digest_and_the_census`]
//! goes red. Drop the placement law and
//! [`a_declared_input_with_no_placement_refuses`] goes red.

use specmark::verifies;
use vibe_core::manifest::{ArtifactInput, ArtifactKind};

use crate::PackageError;
use crate::mechanism::MechanismError;
use crate::mechanism::package::support::*;

/// The provider refusal one run produced.
fn refusal(error: PackageError) -> MechanismError {
    match error {
        PackageError::Provider(inner) => inner,
        other => panic!("expected a provider refusal, got {other}"),
    }
}

/// Package one plugin fixture and return its one produced artifact.
fn package(root: &std::path::Path) -> crate::PackagedArtifact {
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];
    match run_default(root, &targets) {
        Ok(mut outcomes) if outcomes.len() == 1 && outcomes[0].produced.len() == 1 => {
            outcomes.swap_remove(0).produced.swap_remove(0)
        }
        Ok(other) => panic!("expected one produced directory, got {other:?}"),
        Err(error) => panic!("the plugin packages: {error}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_package_unit_is_a_directory_with_a_canonical_tree_digest() {
    let root = temp();
    write_demo_plugin(root.path());

    let produced = package(root.path());

    assert_eq!(produced.path_relative, "target/vibe-package/demo");
    assert_eq!(produced.digest.len(), 64);
    assert_eq!(produced.files, 2, "plugin.json and one SKILL.md");
    assert!(
        root.path()
            .join("target/vibe-package/demo/plugin.json")
            .is_file(),
        "the manifest is staged at the plugin root",
    );
    assert!(
        root.path()
            .join("target/vibe-package/demo/skills/demo/SKILL.md")
            .is_file(),
        "the skill keeps its fixed location",
    );
    assert!(
        !root.path().join("target/vibe-package/demo.zip").exists(),
        "a directory is a first-class artifact; it is not implicitly zipped",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_canonical_digest_does_not_depend_on_creation_order() {
    let first = temp();
    write(
        first.path(),
        "plugin/com.example.tools/b.txt",
        "second file\n",
    );
    write(
        first.path(),
        "plugin/com.example.tools/a.txt",
        "first file\n",
    );
    write_demo_plugin(first.path());

    let second = temp();
    write_demo_plugin(second.path());
    write(
        second.path(),
        "plugin/com.example.tools/a.txt",
        "first file\n",
    );
    write(
        second.path(),
        "plugin/com.example.tools/b.txt",
        "second file\n",
    );

    let left = package(first.path());
    let right = package(second.path());

    assert_eq!(left.digest, right.digest, "one content, one digest");
    assert_eq!(left.files, 4);
    assert_eq!(right.files, 4);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn a_file_the_digest_would_skip_changes_both_the_digest_and_the_census() {
    let root = temp();
    write_demo_plugin(root.path());
    let before = package(root.path());

    write(root.path(), "plugin/com.example.tools/extra.txt", "extra\n");
    let after = package(root.path());

    assert_ne!(before.digest, after.digest);
    assert_eq!(before.files + 1, after.files);
}

/// The DIFFERENTIAL ORACLE for `sha256-tree/1`.
///
/// The algorithm is specified in prose in [`super::digest`]; this recomputes
/// it here, independently of the production walk, over the tree that was
/// really staged. Any change to the ordering, the separator, the census or
/// the algorithm label parts the two answers — which is what makes the
/// specification runnable capital rather than a comment.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_canonical_digest_is_exactly_the_algorithm_the_cell_specifies() {
    use sha2::{Digest, Sha256};

    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "plugin/com.example.tools/z.txt", "zeta\n");
    write(root.path(), "plugin/com.example.tools/a.txt", "alpha\n");

    let produced = package(root.path());

    let staged = root.path().join("target/vibe-package/demo");
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut walk = vec![staged.clone()];
    while let Some(directory) = walk.pop() {
        let listing = match std::fs::read_dir(&directory) {
            Ok(listing) => listing,
            Err(error) => panic!("the staged tree lists: {error}"),
        };
        for entry in listing {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => panic!("the staged entry reads: {error}"),
            };
            if path.is_dir() {
                walk.push(path);
                continue;
            }
            let Ok(relative) = path.strip_prefix(&staged) else {
                panic!("every staged path is below the staged root");
            };
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) => panic!("the staged file reads: {error}"),
            };
            pairs.push((
                relative.to_string_lossy().replace('\\', "/"),
                format!("{:x}", Sha256::digest(&bytes)),
            ));
        }
    }
    pairs.sort();

    let mut hash = Sha256::new();
    hash.update(b"sha256-tree/1\x00");
    for (path, digest) in &pairs {
        hash.update(path.as_bytes());
        hash.update(b"\x00");
        hash.update(digest.as_bytes());
        hash.update(b"\x00");
    }

    assert_eq!(produced.files, pairs.len(), "the census covers every file");
    assert_eq!(
        produced.digest,
        format!("{:x}", hash.finalize()),
        "the recorded digest is the specified canonical one",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_loose_file_at_the_plugin_root_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/README.md",
        "not a portable component\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("the root admits `plugin.json` and `mcp.json` only");

    match refusal(error) {
        MechanismError::PluginShape { entry, .. } => assert_eq!(entry, "README.md"),
        other => panic!("expected the shape refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_client_extension_directory_that_is_not_reverse_domain_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "plugin/commands/thing.md", "invented\n");
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("commands/hooks/agents are client projections, not portable fields");

    match refusal(error) {
        MechanismError::PluginShape { entry, reason, .. } => {
            assert_eq!(entry, "commands");
            assert!(reason.contains("reverse-domain"), "{reason}");
        }
        other => panic!("expected the shape refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_skill_directory_without_its_fixed_entry_document_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "plugin/skills/other/NOTES.md", "no entry\n");
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("`skills/<name>/SKILL.md` is the fixed location");

    match refusal(error) {
        MechanismError::PluginShape { entry, .. } => {
            assert_eq!(entry, "skills/other/SKILL.md");
        }
        other => panic!("expected the shape refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_source_tree_with_no_plugin_manifest_refuses() {
    let root = temp();
    write(
        root.path(),
        "plugin/skills/demo/SKILL.md",
        "---\nname: demo\ndescription: A packaged skill.\n---\n\nBody.\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("an Agent Plugin 1.0 directory declares itself");

    match refusal(error) {
        MechanismError::PluginShape { entry, .. } => assert_eq!(entry, "plugin.json"),
        other => panic!("expected the shape refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_plugin_manifest_missing_a_required_member_refuses_naming_it() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/plugin.json",
        "{ \"name\": \"demo-plugin\" }\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets).expect_err("`version` is required");

    match refusal(error) {
        MechanismError::PluginManifest { file, member, .. } => {
            assert_eq!(file, "plugin.json");
            assert_eq!(member, "version");
        }
        other => panic!("expected the manifest refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_mcp_manifest_with_an_invented_portable_field_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/mcp.json",
        "{ \"mcpServers\": {}, \"hooks\": {} }\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("portable v1 components are skills and MCP servers only");

    match refusal(error) {
        MechanismError::PluginManifest { member, .. } => assert_eq!(member, "hooks"),
        other => panic!("expected the manifest refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_mcp_server_declaring_two_transports_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/mcp.json",
        "{ \"mcpServers\": { \"demo\": { \"command\": \"demo\", \"url\": \"https://x\" } } }\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets).expect_err("exactly one transport");

    match refusal(error) {
        MechanismError::PluginManifest { member, reason, .. } => {
            assert_eq!(member, "mcpServers.demo");
            assert!(reason.contains("exactly one"), "{reason}");
        }
        other => panic!("expected the manifest refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_placeholder_outside_the_two_defined_ones_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/mcp.json",
        "{ \"mcpServers\": { \"demo\": { \"command\": \"${HOME}/bin/demo\" } } }\n",
    );
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("only `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` have a defined meaning");

    match refusal(error) {
        MechanismError::PluginManifest { reason, .. } => {
            assert!(reason.contains("PLUGIN_ROOT"), "{reason}");
        }
        other => panic!("expected the manifest refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_two_defined_placeholders_are_admitted() {
    let root = temp();
    write_demo_plugin(root.path());
    write(
        root.path(),
        "plugin/mcp.json",
        "{ \"mcpServers\": { \"demo\": { \"command\": \"${PLUGIN_ROOT}/bin/demo\", \
         \"args\": [\"--data\", \"${PLUGIN_DATA}\"] } } }\n",
    );

    let produced = package(root.path());

    assert_eq!(produced.files, 3, "plugin.json, mcp.json and the SKILL.md");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_declared_input_with_no_placement_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "assets/note.txt", "asset\n");
    let targets = vec![plugin_target(
        "demo",
        "plugin",
        vec![ArtifactInput::Path {
            path: std::path::PathBuf::from("assets/note.txt"),
        }],
        &[],
    )];

    let error = run_default(root.path(), &targets)
        .expect_err("no adapter silently drops a declared component");

    match refusal(error) {
        MechanismError::Config { member, reason, .. } => {
            assert_eq!(member, "place");
            assert!(reason.contains("assets/note.txt"), "{reason}");
        }
        other => panic!("expected the placement refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_placement_outside_a_reverse_domain_directory_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "assets/note.txt", "asset\n");
    let targets = vec![plugin_target(
        "demo",
        "plugin",
        vec![ArtifactInput::Path {
            path: std::path::PathBuf::from("assets/note.txt"),
        }],
        &[("assets/note.txt", "skills/demo/note.txt")],
    )];

    let error = run_default(root.path(), &targets)
        .expect_err("§6.2 fixes the shape a placed file may land in");

    match refusal(error) {
        MechanismError::Config { member, reason, .. } => {
            assert_eq!(member, "place.assets/note.txt");
            assert!(reason.contains("reverse-domain"), "{reason}");
        }
        other => panic!("expected the placement refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_placed_input_lands_in_its_client_extension_directory() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "assets/note.txt", "asset\n");
    let targets = vec![plugin_target(
        "demo",
        "plugin",
        vec![ArtifactInput::Path {
            path: std::path::PathBuf::from("assets/note.txt"),
        }],
        &[("assets/note.txt", "com.example.tools/note.txt")],
    )];

    match run_default(root.path(), &targets) {
        Ok(outcomes) => assert_eq!(outcomes[0].produced[0].files, 3),
        Err(error) => panic!("the plugin packages: {error}"),
    }
    let staged = root
        .path()
        .join("target/vibe-package/demo/com.example.tools/note.txt");
    match std::fs::read_to_string(&staged) {
        Ok(text) => assert_eq!(text, "asset\n"),
        Err(error) => panic!("the placed input reads: {error}"),
    }
}

/// The WINDOWS half of the same law, with a real junction — the reparse
/// shape §6.2 names explicitly, on the one platform that has it.
///
/// The containment cell's claim ("symlinks, junctions and reparse points —
/// all three of which `symlink_metadata` reports as a symlink file type")
/// rests on std's name-surrogate reparse handling; this test is that claim
/// proven empirically rather than believed, because a junction needs no
/// privilege to create and this law is exactly what stands between a
/// packaged plugin and a directory that points outside the workspace.
#[cfg(windows)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_junction_entry_in_the_source_tree_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "outside/secret.txt", "not yours\n");
    let status = std::process::Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            &root
                .path()
                .join("plugin")
                .join("com.example.tools")
                .to_string_lossy(),
            &root.path().join("outside").to_string_lossy(),
        ])
        .status();
    match status {
        Ok(code) if code.success() => {}
        Ok(code) => panic!("mklink /J refused: {code}"),
        Err(error) => panic!("the fixture junction spawns: {error}"),
    }
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("containment holds across junctions, not only symlinks");

    match refusal(error) {
        MechanismError::PluginShape { entry, reason, .. } => {
            assert_eq!(entry, "com.example.tools");
            assert!(reason.contains("link"), "{reason}");
        }
        other => panic!("expected the containment refusal, got {other}"),
    }
}

#[cfg(unix)]
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_linked_entry_in_the_source_tree_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    write(root.path(), "outside/secret.txt", "not yours\n");
    if let Err(error) = std::os::unix::fs::symlink(
        root.path().join("outside"),
        root.path().join("plugin/com.example.tools"),
    ) {
        panic!("the fixture links: {error}");
    }
    let targets = vec![plugin_target("demo", "plugin", Vec::new(), &[])];

    let error = run_default(root.path(), &targets)
        .expect_err("containment holds across symlinks, junctions and reparse points");

    match refusal(error) {
        MechanismError::PluginShape { entry, reason, .. } => {
            assert_eq!(entry, "com.example.tools");
            assert!(reason.contains("link"), "{reason}");
        }
        other => panic!("expected the containment refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_output_kind_this_provider_does_not_produce_refuses() {
    let root = temp();
    write_demo_plugin(root.path());
    let mut target = plugin_target("demo", "plugin", Vec::new(), &[]);
    target.outputs[0].kind = ArtifactKind::Archive;

    let error = run_default(root.path(), &[target])
        .expect_err("Agent Plugins 1.0 defines a directory, not an archive");

    match refusal(error) {
        MechanismError::UnsupportedKind {
            kind, supported, ..
        } => {
            assert_eq!(kind, "archive");
            assert_eq!(supported, "directory");
        }
        other => panic!("expected the kind refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_stale_distributable_from_a_previous_run_never_survives_into_the_digest() {
    let root = temp();
    write_demo_plugin(root.path());
    let first = package(root.path());

    // A file the engine's own output directory holds from an earlier run.
    write(
        root.path(),
        "target/vibe-package/demo/com.example.tools/stale.txt",
        "left over\n",
    );
    let second = package(root.path());

    assert_eq!(
        first.digest, second.digest,
        "the engine empties its own root"
    );
    assert!(
        !root
            .path()
            .join("target/vibe-package/demo/com.example.tools/stale.txt")
            .exists(),
    );
}
