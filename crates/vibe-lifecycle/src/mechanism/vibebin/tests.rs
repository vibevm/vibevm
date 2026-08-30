//! The vibe-bin provider's PURE laws — everything decided before a byte is
//! written: the launcher template, the strict config, the plan's owned set
//! and §7.1's collision law.
//!
//! Its reconciling half is [`super::apply_tests`], and the world both share
//! is [`super::support`]. The split is the file budget's, along the seam the
//! provider itself has: `plan` opens no destination for writing, while
//! `apply`/`remove`/`recover` are the verbs that do.

use super::launcher::{
    LauncherFlavour, MARKER_PREFIX, Occupant, PROJECT_SHIM_MARKER, VIBE_BIN_MARKER,
};
use super::support::{World, apply, launcher_name, plan_of, refusal, request, target};
use super::{VibeBinProvider, launcher};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::{BUILTIN_VIBE_BIN_PIN, DeployProvider};
use specmark::verifies;
use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

/// §7.1.0 ruling 3: "its body is a fixed marked template embedding ONLY the
/// command name, the genre/owner marker and the pointer indirection — never
/// a version, never a digest, never a copied binary."
///
/// Proven on BOTH flavours, on either host: a body only one machine can
/// render is a body only one machine can review.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_launcher_body_carries_the_marker_the_command_and_no_version_at_all() {
    for flavour in [LauncherFlavour::WindowsCmd, LauncherFlavour::PosixSh] {
        let body = launcher::render(flavour, "vibe-helper");
        let text = String::from_utf8(body).expect("a launcher body is text");
        assert!(text.contains(VIBE_BIN_MARKER), "{flavour:?}: {text}");
        assert!(text.contains("vibe-helper.current"), "{flavour:?}: {text}");
        // The pointer indirection resolves from the launcher's OWN
        // location, so no absolute machine path can be inside it.
        let anchor = match flavour {
            LauncherFlavour::WindowsCmd => "%~dp0",
            LauncherFlavour::PosixSh => "dirname",
        };
        assert!(text.contains(anchor), "{flavour:?}: {text}");
        // The marker line is the one place a `spec://` URI belongs, so the
        // absolute-path law is read over the EXECUTABLE body beneath it.
        let executable: String = text
            .lines()
            .filter(|line| !line.contains(VIBE_BIN_MARKER))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !executable.contains(":\\") && !executable.contains(":/"),
            "{flavour:?} embeds an absolute path: {executable}",
        );
        // And nothing that looks like a digest: no 64-hex run anywhere.
        assert!(
            !executable
                .split(|byte: char| !byte.is_ascii_hexdigit())
                .any(|run| run.len() >= 64),
            "{flavour:?} embeds a digest: {executable}",
        );
    }
}

/// The body is a function of the command alone — the same command renders
/// the same bytes whatever generation is being installed.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn one_command_renders_one_launcher_body_for_every_generation() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let first = world.artifact("helper.exe", "ONE", ArtifactKind::Executable);
    let second = world.artifact("helper.exe", "TWO", ArtifactKind::Executable);

    let one = plan_of(&world, &row, &first);
    let two = plan_of(&world, &row, &second);

    assert_eq!(
        one.resources[0], two.resources[0],
        "two artifacts, one launcher — the desired launcher digest cannot move",
    );
    assert_ne!(
        one.resources[1].desired_digest, two.resources[1].desired_digest,
        "and the POINTER is what a new generation moves",
    );
    assert_eq!(one.config_digest, two.config_digest);
}

/// §7.1.0 ruling 4: "Owned resources are the launcher and the pointer — NOT
/// the payload."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_plan_owns_the_launcher_and_the_pointer_and_never_the_payload() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);

    let plan = plan_of(&world, &row, &artifact);

    let owned: Vec<&str> = plan
        .resources
        .iter()
        .map(|resource| resource.resource.as_str())
        .collect();
    assert_eq!(
        owned,
        [
            launcher_name("vibe-helper").as_str(),
            "bin/vibe-helper.current",
        ],
    );
    assert!(
        !owned.iter().any(|resource| resource.starts_with("store/")),
        "a receipt that owned a shared payload would make undeploy erase another generation's",
    );
    assert!(plan.reversible, "the pointer is what restoration needs");
    assert!(plan.summary.contains("vibe-helper"));
}

/// §7.1's collision law against an UNMARKED user file: refuse, name both
/// genres by their exact markers, and ask for another alias.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_unmarked_file_at_the_launcher_name_is_a_hard_collision() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let occupied = world.at(&launcher_name("vibe-helper"));
    std::fs::create_dir_all(occupied.parent().expect("bin has a parent")).expect("bin creates");
    std::fs::write(&occupied, "my own script\n").expect("the user's file writes");

    let error = VibeBinProvider
        .plan(&request(&world, &row, Some(&artifact), false))
        .expect_err("an unmarked file is never overwritten");

    let DeployProviderError::LauncherCollision {
        resource,
        observed,
        ours,
        shim,
        ..
    } = refusal(&error)
    else {
        panic!("expected the collision refusal, got: {error}");
    };
    assert_eq!(*resource, launcher_name("vibe-helper"));
    assert!(observed.contains("no VibeVM launcher marker"), "{observed}");
    assert_eq!(*ours, VIBE_BIN_MARKER);
    assert_eq!(*shim, PROJECT_SHIM_MARKER);
    let rendered = error.to_string();
    assert!(rendered.contains(VIBE_BIN_MARKER), "{rendered}");
    assert!(rendered.contains(PROJECT_SHIM_MARKER), "{rendered}");
    assert!(rendered.contains("another `command` alias"), "{rendered}");
    assert_eq!(
        std::fs::read_to_string(&occupied).expect("the user's file survives"),
        "my own script\n",
        "and the refusal touched nothing",
    );
}

/// The other genre by ITS marker — the PROP-025 project-pinned shim.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_prop_025_shim_genre_is_a_hard_collision_named_by_its_own_marker() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let occupied = world.at(&launcher_name("vibe-helper"));
    std::fs::create_dir_all(occupied.parent().expect("bin has a parent")).expect("bin creates");
    std::fs::write(
        &occupied,
        format!("# {PROJECT_SHIM_MARKER}\nvibe bin exec x\n"),
    )
    .expect("the shim writes");

    let error = VibeBinProvider
        .plan(&request(&world, &row, Some(&artifact), false))
        .expect_err("the other genre is never overwritten");

    let DeployProviderError::LauncherCollision { observed, .. } = refusal(&error) else {
        panic!("expected the collision refusal, got: {error}");
    };
    assert!(observed.contains(PROJECT_SHIM_MARKER), "{observed}");
    assert!(observed.contains("PROP-025"), "{observed}");
}

/// A third VibeVM genre is refused too — the prefix is what says "some
/// VibeVM launcher", and neither known marker is what says which.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_third_vibevm_genre_is_refused_as_neither_of_the_two() {
    let world = World::new();
    let occupied = world.at(&launcher_name("vibe-helper"));
    std::fs::create_dir_all(occupied.parent().expect("bin has a parent")).expect("bin creates");
    std::fs::write(
        &occupied,
        format!("# {MARKER_PREFIX} genre=something-else\n"),
    )
    .expect("the foreign launcher writes");

    assert_eq!(
        launcher::classify(&occupied).expect("the occupant classifies"),
        Occupant::ForeignGenre,
    );
}

/// §7.1.0 ruling 5's other half: "Same-genre is an update, not a collision."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn our_own_genre_at_the_same_name_is_an_ordinary_update() {
    let world = World::new();
    let row = target(
        "local-helper",
        "helper.exe",
        Some("command = \"vibe-helper\""),
    );
    let first = world.artifact("helper.exe", "ONE", ArtifactKind::Executable);
    apply(&world, &row, &first);

    let second = world.artifact("helper.exe", "TWO", ArtifactKind::Executable);
    let report = apply(&world, &row, &second);

    assert_eq!(
        report.prior_state_handle,
        Some(format!("pointer:{}", first.digest)),
        "an update keeps what restoration needs",
    );
    let pointer =
        std::fs::read(world.at("bin/vibe-helper.current")).expect("the pointer reads back");
    assert_eq!(
        launcher::pointer_digest(&pointer),
        Some(second.digest.clone()),
        "and the POINTER is what moved",
    );
}

/// §7.1.0 ruling 7: "every other kind refuses by name."
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_non_executable_artifact_refuses_by_its_own_kind() {
    let world = World::new();
    let row = target(
        "local-helper",
        "notes.md",
        Some("command = \"vibe-helper\""),
    );
    let artifact = world.artifact("notes.md", "not a program", ArtifactKind::File);

    let error = VibeBinProvider
        .plan(&request(&world, &row, Some(&artifact), false))
        .expect_err("only an explicit executable may use this provider");

    let DeployProviderError::ArtifactKind {
        kind,
        supported,
        provider,
        ..
    } = refusal(&error)
    else {
        panic!("expected the kind refusal, got: {error}");
    };
    assert_eq!(*kind, "file");
    assert_eq!(*supported, "executable");
    assert_eq!(*provider, BUILTIN_VIBE_BIN_PIN);
    assert!(error.to_string().contains("MSI, dpkg, Homebrew"));
}

/// An executable-kind DIRECTORY has no single payload to resolve.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_directory_shaped_artifact_refuses_by_its_shape() {
    let world = World::new();
    let row = target("local-helper", "tree", Some("command = \"vibe-helper\""));
    let mut artifact = world.artifact("tree", "payload", ArtifactKind::Executable);
    artifact.shape = ArtifactShape::Directory;

    let error = VibeBinProvider
        .plan(&request(&world, &row, Some(&artifact), false))
        .expect_err("a tree is not a payload");

    assert!(
        matches!(refusal(&error), DeployProviderError::ArtifactShape { .. }),
        "expected the shape refusal, got: {error}",
    );
}

/// The config table is strict, and the engine-owned members refuse BY NAME
/// rather than as "unknown".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_config_table_is_strict_and_names_what_it_will_not_take() {
    let world = World::new();
    let artifact = world.artifact("helper.exe", "payload", ArtifactKind::Executable);
    let cases = [
        (
            "command = \"vibe-helper\"\nlayout = \"x\"",
            "layout",
            "unknown member",
        ),
        (
            "command = \"vibe-helper\"\nversion = \"1.0\"",
            "version",
            "version-free by construction",
        ),
        (
            "command = \"vibe-helper\"\nbin_dir = \"x\"",
            "bin_dir",
            "engine-owned",
        ),
        (
            "command = \"vibe-helper\"\npath = \"x\"",
            "path",
            "never modifies PATH",
        ),
        (
            "command = \"Vibe-Helper\"",
            "command",
            "portable single-segment",
        ),
        ("command = \"tool.current\"", "command", "reserved suffix"),
        ("command = \"nul\"", "command", "reserved Windows device"),
        ("command = 7", "command", "expected a string"),
    ];
    for (text, member, needle) in cases {
        let row = target("local-helper", "helper.exe", Some(text));
        let error = VibeBinProvider
            .plan(&request(&world, &row, Some(&artifact), false))
            .unwrap_err();
        let DeployProviderError::Config {
            member: named,
            reason,
            ..
        } = refusal(&error)
        else {
            panic!("expected a config refusal for `{text}`, got: {error}");
        };
        assert_eq!(named, member, "for `{text}`");
        assert!(reason.contains(needle), "for `{text}`: {reason}");
    }
}
