//! Golden captures of the surfaces PROP-067 `#coverage` brought into the
//! display language after the first captures: `vibe self ls`, `vibe check`
//! with one finding, the unattended `vibe init`, the `consent_required`
//! refusal of PROP-066 and a project `vibe uninstall` with its trailer.
//! Each is run on a redirected stream and compared as plain text after the
//! same normalisation as `cli_display_golden.rs`.

#![cfg(target_os = "linux")]

mod common;
mod display_support;
#[path = "cli_uninstall_transitives/support.rs"]
#[allow(
    dead_code,
    reason = "the uninstall capture uses one world and its root"
)]
mod support;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::UserScratch;
use display_support::{golden, normalise};

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Stderr then stdout, the order a terminal shows a run whose prose ends
/// before its document begins.
fn screen(output: &Output) -> String {
    format!("{}{}", text(&output.stderr), text(&output.stdout))
}

/// A version store with two installed instances, the second one active,
/// written as `vibe self install` leaves it — without building anything.
fn two_installed_versions(base: &Path) {
    let store = base.join("opt").join("vibevm");
    for instance in ["branch/main/1", "tag/1.0.7/2"] {
        fs::create_dir_all(store.join("versions").join(instance)).unwrap();
    }
    fs::write(
        store.join("state.toml"),
        "next_instance = 3\n\n\
         [[install]]\nkind = \"branch\"\nid = \"main\"\ninstance = 1\n\
         commit = \"4f2a91c0de5b7e8a\"\ntoolchain = \"rustc 1.93.0\"\nprofile = \"debug\"\n\
         installed_at = \"2026-11-01T00:00:00Z\"\norigin = \"external\"\n\
         source_path = \"/src/vibevm\"\n\n\
         [[install]]\nkind = \"tag\"\nid = \"1.0.7\"\ninstance = 2\n\
         commit = \"9c1d0e77aa0b1c2d\"\ntoolchain = \"prebuilt\"\nprofile = \"release\"\n\
         installed_at = \"2026-11-01T00:00:00Z\"\norigin = \"binary\"\n\
         payload_sha256 = \"feedface\"\n",
    )
    .unwrap();
    fs::write(
        store.join("current"),
        format!("{}\n", store.join("versions/tag/1.0.7/2").display()),
    )
    .unwrap();
}

#[test]
fn the_self_listing_reads_as_its_golden() {
    let user = UserScratch::new();
    let base = tempfile::tempdir().unwrap();
    two_installed_versions(base.path());
    let listed = user
        .vibe()
        .args(["self", "ls"])
        .env("VIBEVM_INSTALL_ROOT", base.path())
        .env_remove("VIBEVM_HOME")
        .output()
        .unwrap();
    assert!(listed.status.success(), "{}", screen(&listed));
    // A test binary built inside a checkout also lists itself, running from
    // that source tree; where the build lives decides whether it does, so
    // the capture keeps the installed instances alone.
    let stdout = text(&listed.stdout);
    let installed = stdout
        .split("── running from a source tree")
        .next()
        .unwrap_or_default();
    golden("self-list", &normalise(installed, &[]));
}

#[test]
fn a_check_with_one_finding_reads_as_its_golden() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    // An opening marker with no closing one: the redirect-block check's
    // one finding, and nothing else in a fresh project.
    fs::write(
        project.path().join("CLAUDE.md"),
        "# notes\n\n<vibevm>\nunfinished\n",
    )
    .unwrap();
    let checked = user
        .vibe()
        .args(["check", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        text(&checked.stderr).contains("vibe check: 1 error"),
        "an error finding ends in the refusal: {}",
        screen(&checked)
    );
    golden(
        "check",
        &normalise(&text(&checked.stdout), &[(project.path(), "<project>")]),
    );
}

#[test]
fn the_unattended_init_reads_as_its_golden() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let initialised = user
        .vibe()
        .current_dir(root.path())
        .args(["init", "demo", "--unattended", "--plain", "--name", "demo"])
        .output()
        .unwrap();
    assert!(initialised.status.success(), "{}", screen(&initialised));
    golden("init-wizard", &normalise(&screen(&initialised), &[]));
}

#[test]
fn a_project_uninstall_ends_with_its_trailer() {
    let world = support::World::new(&["a"], &[("a", &["b"]), ("b", &[])]);
    let removed = world
        .user
        .vibe()
        .args([
            "uninstall",
            "org.test/a",
            "--assume-yes",
            "--plain",
            "--path",
        ])
        .arg(world.root())
        .output()
        .unwrap();
    assert!(removed.status.success(), "{}", screen(&removed));
    golden(
        "uninstall",
        &normalise(&text(&removed.stdout), &[(world.root(), "<project>")]),
    );
}

/// A registry holding an application package whose one input is fetched
/// from `url`, and the remap package `org.corp/intranet` offering a row
/// for that input.
fn consent_registry(root: &Path, url: &str) -> std::path::PathBuf {
    let registry = root.join("registry");
    let app = registry.join("org.example/remapped/v1.0.0");
    fs::create_dir_all(&app).unwrap();
    let digest = "0".repeat(64);
    fs::write(
        app.join("vibe.toml"),
        format!(
            "[package]\nname = \"remapped\"\ngroup = \"org.example\"\nkind = \"app\"\n\
             version = \"1.0.0\"\n\n[application]\nid = \"remapped\"\n\n\
             [[installer_options]]\nid = \"src\"\nname = \"Build from source\"\n\
             platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
             [installer_options.installation]\ntype = \"source_build\"\n\
             [[installer_options.installation.sources]]\nname = \"payload\"\n\
             url = \"{url}\"\nsha256 = \"{digest}\"\n\
             [installer_options.installation.recipe]\ninterpreter = \"posix_sh\"\n\
             script = \"build.sh\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    fs::write(app.join("build.sh"), "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n").unwrap();
    let remap = registry.join("org.corp/intranet/v1.0.0");
    fs::create_dir_all(&remap).unwrap();
    fs::write(
        remap.join("vibe.toml"),
        format!(
            "[package]\nname = \"intranet\"\ngroup = \"org.corp\"\nkind = \"remap\"\n\
             version = \"1.0.0\"\n\n[remap]\ndescription = \"the corp network\"\n\n\
             [[remap.rows]]\nsha256 = \"{digest}\"\npath = \"{}\"\n\
             reason = \"the air-gapped artifact store\"\n",
            root.join("store").display()
        ),
    )
    .unwrap();
    registry
}

#[test]
fn the_consent_refusal_reads_as_its_golden() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    // Nothing listens there: the refusal comes before any fetch.
    let registry = consent_registry(root.path(), "http://127.0.0.1:9/dist/payload.txt");
    let install = |package: &str| {
        user.vibe()
            .current_dir(root.path())
            .args(["install", "-g", package, "--unattended", "--registry"])
            .arg(&registry)
            .output()
            .unwrap()
    };
    let offered = install("org.corp/intranet");
    assert!(offered.status.success(), "{}", screen(&offered));
    let refused = install("org.example/remapped");
    assert_eq!(refused.status.code(), Some(2), "{}", screen(&refused));
    golden(
        "consent",
        &normalise(
            &text(&refused.stderr),
            &[(root.path(), "<root>"), (&user.settings, "<settings>")],
        ),
    );
}
