//! The §5.0.3 message-shape laws, over RECORDED Cargo output.
//!
//! Every fixture below is a real line captured from
//! `cargo build --message-format=json-render-diagnostics` and
//! `cargo check …` on a dependency-free crate, with only the workspace
//! root shortened. They are kept whole — `manifest_path`, `crate_types`,
//! `src_path`, `edition`, `doc`, `doctest`, `test`, `profile`,
//! `features`, `filenames` and all — because "unknown fields are ignored
//! BY DESIGN" is a law about THESE fields, and a trimmed fixture would
//! quietly stop testing it.
//!
//! The mixed slashes in the recorded paths are also real: on Windows
//! Cargo echoes the `--target-dir` we passed verbatim and appends its own
//! components with backslashes. A reader that assumed one separator would
//! have looked correct against a hand-written fixture.

use specmark::verifies;

use super::super::config::OutputSelect;
use super::*;

/// A `bin` artifact with an executable, freshly compiled.
const ARTIFACT_BIN: &str = r#"{"reason":"compiler-artifact","package_id":"path+file:///C:/w/fx#vibe-r8-fixture@0.1.0","manifest_path":"C:\\w\\fx\\Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"vibe-r8-fixture","src_path":"C:\\w\\fx\\src\\main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["C:/w/fx/target\\debug\\vibe-r8-fixture.exe","C:/w/fx/target\\debug\\vibe_r8_fixture.pdb"],"executable":"C:/w/fx/target\\debug\\vibe-r8-fixture.exe","fresh":false}"#;

/// The same artifact on a rebuild: Cargo's own freshness verdict is true.
const ARTIFACT_BIN_FRESH: &str = r#"{"reason":"compiler-artifact","package_id":"path+file:///C:/w/fx#vibe-r8-fixture@0.1.0","manifest_path":"C:\\w\\fx\\Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"vibe-r8-fixture","src_path":"C:\\w\\fx\\src\\main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["C:/w/fx/target\\debug\\vibe-r8-fixture.exe","C:/w/fx/target\\debug\\vibe_r8_fixture.pdb"],"executable":"C:/w/fx/target\\debug\\vibe-r8-fixture.exe","fresh":true}"#;

/// A second `bin` artifact of a second package — the ambiguity fixture.
const ARTIFACT_SECOND_BIN: &str = r#"{"reason":"compiler-artifact","package_id":"path+file:///C:/w/fx/other#vibe-r8-other@0.1.0","manifest_path":"C:\\w\\fx\\other\\Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"vibe-r8-other","src_path":"C:\\w\\fx\\other\\src\\main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["C:/w/fx/target\\debug\\vibe-r8-other.exe"],"executable":"C:/w/fx/target\\debug\\vibe-r8-other.exe","fresh":false}"#;

/// A `bin`-kind artifact that carried NO executable — exactly what
/// `cargo check` emits, recorded rather than imagined.
const ARTIFACT_NO_EXECUTABLE: &str = r#"{"reason":"compiler-artifact","package_id":"path+file:///C:/w/fx#vibe-r8-fixture@0.1.0","manifest_path":"C:\\w\\fx\\Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"vibe-r8-fixture","src_path":"C:\\w\\fx\\src\\main.rs","edition":"2021","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["C:/w/fx/target-check\\debug\\deps\\libvibe_r8_fixture-893ed8eceb64efc3.rmeta"],"executable":null,"fresh":false}"#;

/// A library artifact — a message the executable law must not consider.
const ARTIFACT_LIB: &str = r#"{"reason":"compiler-artifact","package_id":"path+file:///C:/w/fx#vibe-r8-fixture@0.1.0","manifest_path":"C:\\w\\fx\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"vibe_r8_fixture","src_path":"C:\\w\\fx\\src\\lib.rs","edition":"2021","doc":true,"doctest":true,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["C:/w/fx/target\\debug\\libvibe_r8_fixture.rlib"],"executable":null,"fresh":false}"#;

const BUILD_FINISHED: &str = r#"{"reason":"build-finished","success":true}"#;

/// The recorded `cargo metadata --format-version 1 --no-deps` document,
/// trimmed only of the members after `targets`.
const METADATA: &str = r#"{"packages":[{"name":"vibe-r8-fixture","version":"0.1.0","id":"path+file:///C:/w/fx#vibe-r8-fixture@0.1.0","license":null,"source":null,"dependencies":[],"targets":[{"kind":["bin"],"crate_types":["bin"],"name":"vibe-r8-fixture","src_path":"C:\\w\\fx\\src\\main.rs","edition":"2021","doc":true,"doctest":false,"test":true}],"features":{},"manifest_path":"C:\\w\\fx\\Cargo.toml"}],"workspace_members":["path+file:///C:/w/fx#vibe-r8-fixture@0.1.0"],"target_directory":"C:\\w\\fx\\target","version":1,"workspace_root":"C:\\w\\fx","metadata":null}"#;

fn stream(lines: &[&str]) -> Vec<CargoMessage> {
    match parse_stream("fixture", &lines.join("\n")) {
        Ok(messages) => messages,
        Err(error) => panic!("the recorded stream reads: {error}"),
    }
}

fn select(package: Option<&str>, bin: Option<&str>) -> OutputSelect {
    OutputSelect {
        package: package.map(str::to_owned),
        bin: bin.map(str::to_owned),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn unknown_fields_are_ignored_by_design() {
    let messages = stream(&[ARTIFACT_BIN, BUILD_FINISHED]);

    assert_eq!(messages.len(), 2);
    let artifact = &messages[0];
    assert_eq!(artifact.reason, "compiler-artifact");
    assert_eq!(
        artifact.package_id.as_deref(),
        Some("path+file:///C:/w/fx#vibe-r8-fixture@0.1.0")
    );
    let target = artifact.target.as_ref().expect("the artifact has a target");
    assert_eq!(target.name, "vibe-r8-fixture");
    assert_eq!(target.kind, vec!["bin".to_owned()]);
    assert_eq!(
        artifact.executable.as_deref(),
        Some(r"C:/w/fx/target\debug\vibe-r8-fixture.exe")
    );
    assert_eq!(artifact.fresh, Some(false));
    // The second line carries none of the artifact members at all.
    assert_eq!(messages[1].reason, "build-finished");
    assert_eq!(messages[1].executable, None);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_executable_comes_only_from_the_message() {
    let messages = stream(&[ARTIFACT_LIB, ARTIFACT_BIN, BUILD_FINISHED]);

    let chosen = match select_message("t", "o", &select(None, Some("vibe-r8-fixture")), &messages) {
        Ok(chosen) => chosen,
        Err(error) => panic!("the bin artifact is selected: {error}"),
    };

    assert_eq!(
        chosen.executable.as_deref(),
        Some(r"C:/w/fx/target\debug\vibe-r8-fixture.exe")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn cargos_own_fresh_verdict_passes_through() {
    let messages = stream(&[ARTIFACT_BIN_FRESH, BUILD_FINISHED]);

    let chosen = match select_message("t", "o", &select(None, None), &messages) {
        Ok(chosen) => chosen,
        Err(error) => panic!("the only artifact is selected: {error}"),
    };

    assert_eq!(chosen.fresh, Some(true));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_ambiguous_selection_refuses_instead_of_taking_the_first() {
    let messages = stream(&[ARTIFACT_BIN, ARTIFACT_SECOND_BIN, BUILD_FINISHED]);

    let refusal = select_message("t", "helper.exe", &select(None, None), &messages)
        .expect_err("two bin artifacts cannot answer one output");

    match &refusal {
        MechanismError::AmbiguousArtifact { matched, names, .. } => {
            assert_eq!(*matched, 2);
            assert!(names.contains("vibe-r8-fixture"), "{names}");
            assert!(names.contains("vibe-r8-other"), "{names}");
        }
        other => panic!("expected an ambiguity refusal, got {other}"),
    }
    assert!(refusal.to_string().contains("never resolved by taking the"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_null_executable_refuses_instead_of_guessing_a_target_path() {
    let messages = stream(&[ARTIFACT_NO_EXECUTABLE, BUILD_FINISHED]);

    let refusal = select_message("t", "helper.exe", &select(None, None), &messages)
        .expect_err("an artifact without an executable produces none");

    match &refusal {
        MechanismError::NoExecutable { bin, .. } => assert_eq!(bin, "vibe-r8-fixture"),
        other => panic!("expected a missing-executable refusal, got {other}"),
    }
    assert!(refusal.to_string().contains("will not guess"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn no_match_refuses_and_says_how_many_it_read() {
    let messages = stream(&[ARTIFACT_LIB, ARTIFACT_BIN, BUILD_FINISHED]);

    let refusal = select_message("t", "helper.exe", &select(None, Some("absent")), &messages)
        .expect_err("nothing answers `bin = absent`");

    match &refusal {
        MechanismError::NoArtifact {
            considered,
            predicate,
            ..
        } => {
            // The library artifact is not a candidate at all.
            assert_eq!(*considered, 1);
            assert_eq!(predicate, "bin `absent`");
        }
        other => panic!("expected a no-artifact refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_package_predicate_narrows_a_two_artifact_build() {
    let messages = stream(&[ARTIFACT_BIN, ARTIFACT_SECOND_BIN, BUILD_FINISHED]);

    let chosen = match select_message("t", "o", &select(Some("vibe-r8-other"), None), &messages) {
        Ok(chosen) => chosen,
        Err(error) => panic!("the second package's artifact is selected: {error}"),
    };

    assert_eq!(
        chosen.executable.as_deref(),
        Some(r"C:/w/fx/target\debug\vibe-r8-other.exe")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_line_that_is_not_a_cargo_message_refuses_with_its_line_number() {
    let refusal = parse_stream("t", "{\"reason\":\"build-finished\"}\nnot json\n")
        .expect_err("a non-message line is the signal the format moved");

    match &refusal {
        MechanismError::MessageDecode { line, value, .. } => {
            assert_eq!(*line, 2);
            assert_eq!(value, "not json");
        }
        other => panic!("expected a decode refusal, got {other}"),
    }
    assert!(
        refusal
            .to_string()
            .contains("never a guessed artifact path")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn blank_lines_are_skipped_and_do_not_shift_the_line_numbers() {
    let refusal = parse_stream("t", "\n\n{\"reason\":\"x\"}\n\n[]\n")
        .expect_err("a JSON array is not a Cargo message");

    match refusal {
        MechanismError::MessageDecode { line, .. } => assert_eq!(line, 5),
        other => panic!("expected a decode refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn package_id_answers_all_three_spellings_without_substring_luck() {
    // Modern spec with a `name@version` fragment.
    assert!(package_id_names(
        "path+file:///C:/w/fx#vibe-r8-fixture@0.1.0",
        "vibe-r8-fixture"
    ));
    // Modern spec whose fragment is a bare version.
    assert!(package_id_names("registry+https://x/serde#1.0.0", "serde"));
    // Legacy triple.
    assert!(package_id_names(
        "serde 1.0.0 (registry+https://x)",
        "serde"
    ));
    // A fragment that names the package outright.
    assert!(package_id_names("path+file:///C:/w#serde", "serde"));
    // And the near-misses a substring test would have accepted.
    assert!(!package_id_names(
        "registry+https://x/serde_json#1.0.0",
        "serde"
    ));
    assert!(!package_id_names(
        "serde_json 1.0.0 (registry+https)",
        "serde"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn metadata_confirms_a_declared_package_and_bin() {
    let metadata = match parse_metadata("t", METADATA) {
        Ok(metadata) => metadata,
        Err(error) => panic!("the recorded metadata reads: {error}"),
    };

    assert!(
        confirm_against_metadata(
            "t",
            "o",
            &select(Some("vibe-r8-fixture"), Some("vibe-r8-fixture")),
            &metadata,
        )
        .is_ok()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn metadata_refuses_an_unknown_package_and_an_unknown_bin() {
    let metadata = match parse_metadata("t", METADATA) {
        Ok(metadata) => metadata,
        Err(error) => panic!("the recorded metadata reads: {error}"),
    };

    let unknown_package =
        confirm_against_metadata("t", "o", &select(Some("absent"), None), &metadata)
            .expect_err("no such package");
    match &unknown_package {
        MechanismError::UnknownPackage { candidates, .. } => {
            assert_eq!(candidates, "vibe-r8-fixture");
        }
        other => panic!("expected an unknown-package refusal, got {other}"),
    }

    let unknown_bin = confirm_against_metadata("t", "o", &select(None, Some("absent")), &metadata)
        .expect_err("no such bin target");
    match &unknown_bin {
        MechanismError::UnknownBin { candidates, .. } => {
            assert_eq!(candidates, "vibe-r8-fixture");
        }
        other => panic!("expected an unknown-bin refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn malformed_metadata_refuses() {
    let refusal = parse_metadata("t", "{\"packages\": 7}").expect_err("packages is an array");

    assert!(matches!(refusal, MechanismError::MetadataDecode { .. }));
}
