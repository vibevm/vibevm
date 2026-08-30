//! §7.0.8's acceptance, end to end: "Determinism IS the acceptance: two
//! runs, one digest."
//!
//! The e2e packages one FILE artifact and one DIRECTORY artifact through
//! the real package executor — the same routing, the same input
//! resolution and the same record cell every other provider goes through
//! — and runs it twice.

use specmark::verifies;
use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, MechanismRoutes,
};
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::package::support::{
    config, empty_world, execution, key, registry, run_default, temp, write,
};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
use crate::mechanism::{execute_package_targets, package::PackageOutcome};

/// One `package:windows-zip` target over the given inputs.
fn zip_target(id: &str, inputs: Vec<ArtifactInput>, table: &str) -> ArtifactPackageTarget {
    ArtifactPackageTarget {
        id: id.to_owned(),
        mechanism: key("package:windows-zip"),
        provider: None,
        inputs: Some(inputs),
        outputs: vec![ArtifactOutput {
            id: format!("{id}.zip"),
            kind: ArtifactKind::Archive,
            select: None,
        }],
        config: (!table.is_empty()).then(|| config(table)),
    }
}

/// Record one already-produced artifact so the input resolver can find
/// it — the engine-owned state every consumed artifact is read through.
fn record_artifact(
    root: &std::path::Path,
    id: &str,
    relative: &str,
    shape: ArtifactShape,
    digest: &str,
    kind: ArtifactKind,
) {
    let absolute = crate::mechanism::contain::forward_slashed(&root.join(relative));
    let record = build_record(&RecordInputs {
        target: "producer",
        mechanism: &key("build:cargo"),
        provider_key: "org.vibevm/vibe#cargo",
        provider_version: None,
        provider_hash: None,
        output_id: id,
        kind,
        shape,
        digest,
        path_absolute: &absolute,
        path_relative: relative,
        freshness: RecordFreshness::default(),
        platform: None,
        media_type: None,
        created_at: "2026-08-30T00:00:00Z",
        evidence: "fixture artifact".to_owned(),
    })
    .expect("the fixture record builds");
    write_record(root, &record).expect("the fixture record writes");
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

/// Write the two source artifacts and their records: one file and one
/// directory.
fn seed(root: &std::path::Path) {
    write(root, "target/debug/helper.exe", "helper");
    record_artifact(
        root,
        "helper.exe",
        "target/debug/helper.exe",
        ArtifactShape::File,
        &sha256(b"helper"),
        ArtifactKind::Executable,
    );
    write(root, "target/vibe-package/plugin/plugin.json", "{}\n");
    write(root, "target/vibe-package/plugin/skills/a/SKILL.md", "a\n");
    let tree = crate::mechanism::contain::tree_digest(&root.join("target/vibe-package/plugin"))
        .expect("the fixture tree digests");
    record_artifact(
        root,
        "plugin.dir",
        "target/vibe-package/plugin",
        ArtifactShape::Directory,
        &tree.digest,
        ArtifactKind::AgentPlugin,
    );
}

fn digest_of(outcome: &PackageOutcome) -> String {
    outcome
        .produced
        .first()
        .expect("one distributable")
        .digest
        .clone()
}

/// §7.0.8's acceptance: two runs over one file artifact and one directory
/// artifact produce one digest.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn two_runs_over_a_file_and_a_directory_artifact_produce_one_digest() {
    let project = temp();
    seed(project.path());
    let targets = [zip_target(
        "bundle",
        vec![
            ArtifactInput::Artifact {
                artifact: "helper.exe".to_owned(),
            },
            ArtifactInput::Artifact {
                artifact: "plugin.dir".to_owned(),
            },
        ],
        "layout = \"distribution/windows\"",
    )];

    let first = run_default(project.path(), &targets).expect("the first run packages");
    let first_digest = digest_of(&first[0]);
    let second = run_default(project.path(), &targets).expect("the second run packages");
    let second_digest = digest_of(&second[0]);

    assert_eq!(
        first_digest, second_digest,
        "two runs, one digest — the whole acceptance",
    );
    let archive = project.path().join("target/vibe-package/bundle/bundle.zip");
    assert!(
        archive.is_file(),
        "the distributable is at the engine's own path"
    );
    let bytes = std::fs::read(&archive).expect("the archive reads");
    assert_eq!(sha256(&bytes), first_digest, "the record digests the bytes");
    // The archived names: the file artifact under its own id, the
    // directory artifact by its canonical walk, both under `layout`.
    let names = archived_names(&bytes);
    assert_eq!(
        names,
        [
            "distribution/windows/helper.exe",
            "distribution/windows/plugin.dir/plugin.json",
            "distribution/windows/plugin.dir/skills/a/SKILL.md",
        ],
        "sorted archived names, forward-slashed, under the layout prefix",
    );
    // And the record is the ordinary A2 one, beside every other provider's.
    assert!(
        project
            .path()
            .join(".vibe/state/artifacts/bundle.zip.json")
            .is_file(),
    );
}

/// The archived names of one rendered archive, read back out of the
/// local headers — the archive's own witness, not the census we passed in.
fn archived_names(bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0_usize;
    while at + 30 <= bytes.len() && bytes[at..at + 4] == [0x50, 0x4b, 0x03, 0x04] {
        let size = u32::from_le_bytes([
            bytes[at + 18],
            bytes[at + 19],
            bytes[at + 20],
            bytes[at + 21],
        ]) as usize;
        let name_length = u16::from_le_bytes([bytes[at + 26], bytes[at + 27]]) as usize;
        let start = at + 30;
        names.push(String::from_utf8_lossy(&bytes[start..start + name_length]).into_owned());
        at = start + name_length + size;
    }
    names
}

/// Without a `layout` the entries sit at the archive root — the member is
/// optional and adds a prefix, it does not rename anything.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_layout_prefix_is_optional() {
    let project = temp();
    seed(project.path());
    let targets = [zip_target(
        "bare",
        vec![ArtifactInput::Artifact {
            artifact: "helper.exe".to_owned(),
        }],
        "",
    )];

    run_default(project.path(), &targets).expect("the run packages");

    let bytes = std::fs::read(project.path().join("target/vibe-package/bare/bare.zip"))
        .expect("the archive reads");
    assert_eq!(archived_names(&bytes), ["helper.exe"]);
}

/// A `layout` change is a freshness change: the engine-fresh fingerprint
/// is taken over ARCHIVED names, so the record cannot claim two different
/// archives were the same work.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn the_layout_enters_the_engine_fresh_fingerprint() {
    let project = temp();
    seed(project.path());
    let inputs = || {
        vec![ArtifactInput::Artifact {
            artifact: "helper.exe".to_owned(),
        }]
    };
    let bare = [zip_target("same", inputs(), "")];
    let laid_out = [zip_target("same", inputs(), "layout = \"dist\"")];

    let first = run_default(project.path(), &bare).expect("the bare run packages");
    let second = run_default(project.path(), &laid_out).expect("the laid-out run packages");

    assert_ne!(
        digest_of(&first[0]),
        digest_of(&second[0]),
        "a layout change really is a different archive",
    );
}

/// The config is strict: an unknown member and each engine-owned member
/// refuse by name.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_config_is_strict_and_the_engine_owned_members_refuse_by_name() {
    let project = temp();
    seed(project.path());
    for (table, needle) in [
        ("compress = true", "unknown member"),
        ("timestamp = \"now\"", "fixed timestamp constant"),
        (
            "compression = \"deflate\"",
            "compression parameters are fixed",
        ),
        ("output = \"x.zip\"", "engine-owned"),
        ("layout = \"../escape\"", "escapes the root"),
    ] {
        let targets = [zip_target(
            "strict",
            vec![ArtifactInput::Artifact {
                artifact: "helper.exe".to_owned(),
            }],
            table,
        )];
        let error = run_default(project.path(), &targets)
            .unwrap_err()
            .to_string();
        assert!(error.contains(needle), "`{table}` refused with: {error}");
    }
}

/// Two inputs that would archive to one name refuse: an archive never has
/// two claimants for one entry.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn two_inputs_claiming_one_archived_name_refuse() {
    let project = temp();
    seed(project.path());
    write(project.path(), "docs/helper.exe", "a decoy\n");
    let targets = [zip_target(
        "clash",
        vec![
            ArtifactInput::Artifact {
                artifact: "helper.exe".to_owned(),
            },
            ArtifactInput::Path {
                path: std::path::PathBuf::from("helper.exe"),
            },
        ],
        "",
    )];
    write(project.path(), "helper.exe", "a decoy at the root\n");

    let error = run_default(project.path(), &targets)
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("two claimants for one entry name"),
        "{error}"
    );
}

/// Routing is real for this provider too: a host that routes
/// `package:windows-zip` away gets the transport refusal and NOT an
/// archive.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_routed_away_zip_target_refuses_by_the_unlanded_transport() {
    use crate::mechanism::package::support::{pin, world_with_plugin};
    let project = temp();
    seed(project.path());
    let mut world = world_with_plugin();
    for source in &mut world.installed {
        for declaration in &mut source.mechanisms {
            declaration.name = "windows-zip".into();
        }
    }
    let plane = registry(&world);
    let mut routes = MechanismRoutes::default();
    routes.insert(
        key("package:windows-zip"),
        pin(crate::mechanism::package::support::PLUGIN_PIN),
    );
    let targets = [zip_target(
        "routed",
        vec![ArtifactInput::Artifact {
            artifact: "helper.exe".to_owned(),
        }],
        "",
    )];

    let error = execute_package_targets(&execution(project.path(), &targets, &plane, &routes))
        .expect_err("the transport is a later atom");

    assert!(error.to_string().contains("not yet landed"), "{error}");
    assert!(
        !project
            .path()
            .join("target/vibe-package/routed/routed.zip")
            .exists(),
        "and the builtin demonstrably did not run",
    );
    let _ = empty_world();
}

/// The archived ORDER is a function of the archived names, never of the
/// declaration order — §7.0.8's "entries sorted by archived name".
///
/// Declaring the inputs in the opposite order to the one they archive in
/// is what makes this a law rather than a coincidence: a census that
/// merely inherited the declaration order would pass every test whose
/// fixture happened to be written alphabetically.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_archived_order_follows_the_names_not_the_declaration() {
    let project = temp();
    seed(project.path());
    let forward = [zip_target(
        "ordered",
        vec![
            ArtifactInput::Artifact {
                artifact: "helper.exe".to_owned(),
            },
            ArtifactInput::Artifact {
                artifact: "plugin.dir".to_owned(),
            },
        ],
        "",
    )];
    // The SAME set, declared in the opposite order.
    let reversed = [zip_target(
        "ordered",
        vec![
            ArtifactInput::Artifact {
                artifact: "plugin.dir".to_owned(),
            },
            ArtifactInput::Artifact {
                artifact: "helper.exe".to_owned(),
            },
        ],
        "",
    )];

    let first = run_default(project.path(), &forward).expect("the forward run packages");
    let first_digest = digest_of(&first[0]);
    let first_names = archived_names(
        &std::fs::read(
            project
                .path()
                .join("target/vibe-package/ordered/ordered.zip"),
        )
        .expect("the archive reads"),
    );
    let second = run_default(project.path(), &reversed).expect("the reversed run packages");
    let second_digest = digest_of(&second[0]);
    let second_names = archived_names(
        &std::fs::read(
            project
                .path()
                .join("target/vibe-package/ordered/ordered.zip"),
        )
        .expect("the archive reads"),
    );

    assert_eq!(
        first_names,
        [
            "helper.exe",
            "plugin.dir/plugin.json",
            "plugin.dir/skills/a/SKILL.md",
        ],
    );
    assert_eq!(
        first_names, second_names,
        "the archived order is the sorted names, whichever order they were declared in",
    );
    assert_eq!(
        first_digest, second_digest,
        "so the two declarations produce one archive",
    );
}
