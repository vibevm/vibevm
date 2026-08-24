//! The 2026-08-24 install-world rulings, end to end: an explicit-pkgref
//! install must never shrink the world (PROP-011 ##EXPLICIT-PKGREF-FULL-SOLVE),
//! zero dependencies is a normal state (##EMPTY-REQUIRES-IS-A-NO-OP), and the
//! `vibe clean` verb with its `clean install` chain (PROP-053).
//!
//! The stand is the hermetic `fixtures/registry` directory registry over
//! `file:///` — two flow packages (`integration-alpha`, `integration-beta`)
//! installed side by side, then the verbs under test poke at the world.

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, fixture_registry};
use specmark::verifies;

/// A project wired to the hermetic fixture registry over `file:///`.
fn project_with_fixture_registry(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let url = format!(
        "file:///{}",
        fixture_registry().display().to_string().replace('\\', "/")
    );
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(&format!(
        "\n[[registry]]\nname = \"fixture\"\nurl = \"{url}\"\n"
    ));
    fs::write(&manifest_path, manifest).unwrap();
    project
}

fn install(user: &UserScratch, project: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = user.vibe();
    cmd.arg("install");
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--path").arg(project).arg("--assume-yes").assert()
}

fn slot(project: &Path, slot: &str, version: &str) -> std::path::PathBuf {
    project.join(common::slot_dir(slot, version))
}

// ---- ##EXPLICIT-PKGREF-FULL-SOLVE -----------------------------------------

/// The owner's bug (2026-08-24): `vibe install <pkgref>` resolved ONLY the
/// named package and the apply-phase prune then erased every other slot.
/// A pkgref install must leave the rest of the world standing.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#EXPLICIT-PKGREF-FULL-SOLVE"
)]
fn an_explicit_pkgref_install_keeps_the_rest_of_the_world() {
    let user = UserScratch::new();
    let project = project_with_fixture_registry(&user);
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    let alpha = slot(project.path(), "org.vibevm.integration-alpha", "0.1.0");
    assert!(alpha.is_dir(), "alpha slot after its install");
    install(&user, project.path(), &["flow:org.vibevm/integration-beta"]).success();
    let beta = slot(project.path(), "org.vibevm.integration-beta", "0.1.0");
    assert!(beta.is_dir(), "beta slot after its install");
    assert!(
        alpha.is_dir(),
        "the alpha slot must survive the beta install —          a pkgref install never shrinks the world"
    );

    // Re-install ONE of them by name: the other slot must survive, and the
    // lock must keep both packages.
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    assert!(
        beta.is_dir(),
        "the beta slot must survive an explicit alpha install — \
         a pkgref install never shrinks the world"
    );
    let lock = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    assert!(
        lock.contains("integration-beta"),
        "the lock must keep the untouched package:\n{lock}"
    );
}

/// The same shape offline: with every package already fetched into the
/// machine cache, `vibe install <pkgref> --offline` refreshes the named one
/// and leaves the rest standing (the owner's exact reproduction).
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#EXPLICIT-PKGREF-FULL-SOLVE"
)]
fn an_offline_pkgref_install_keeps_the_rest_of_the_world() {
    let user = UserScratch::new();
    let project = project_with_fixture_registry(&user);
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    install(&user, project.path(), &["flow:org.vibevm/integration-beta"]).success();

    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha", "--offline"],
    )
    .success();
    assert!(
        slot(project.path(), "org.vibevm.integration-beta", "0.1.0").is_dir(),
        "the beta slot must survive an offline alpha install"
    );
}

// ---- ##EMPTY-REQUIRES-IS-A-NO-OP ------------------------------------------

/// `vibe init && vibe install` must work out of the box: a project with zero
/// dependencies is a fresh project, not an error.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#EMPTY-REQUIRES-IS-A-NO-OP"
)]
fn a_bare_install_over_zero_dependencies_is_a_noop() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    install(&user, project.path(), &[]).success();
}

// ---- PROP-053: vibe clean --------------------------------------------------

fn clean(user: &UserScratch, project: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = user.vibe();
    cmd.arg("clean");
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--path").arg(project).arg("--assume-yes").assert()
}

/// `vibe clean` removes exactly the derived prompt state — the vibedeps root
/// and the generated boot artifacts — and never the authored surface, the
/// lock, or the machine cache.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-REMOVES-DERIVED")]
fn clean_removes_the_derived_state_and_keeps_the_authored_surface() {
    let user = UserScratch::new();
    let project = project_with_fixture_registry(&user);
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    let deps_root = project.path().join(common::deps_root());
    let index = project.path().join(common::index_rel());
    assert!(deps_root.is_dir() && index.is_file(), "installed stand");

    clean(&user, project.path(), &[]).success();

    assert!(!deps_root.exists(), "clean removes the vibedeps root");
    assert!(!index.exists(), "clean removes the generated INDEX");
    assert!(
        !project.path().join(common::static_md_rel()).exists()
            && !project.path().join(common::static_xml_rel()).exists(),
        "clean removes the generated STATIC lane"
    );
    assert!(
        project
            .path()
            .join(common::boot_rel("00-core.md"))
            .is_file()
            || project
                .path()
                .join(common::boot_rel("00-core.xml"))
                .is_file(),
        "the authored boot snippet survives clean"
    );
    assert!(
        project.path().join("vibe.lock").is_file(),
        "the lock survives clean — it is the resolution, not derived state"
    );

    // Clean over an already-clean tree is a quiet success.
    clean(&user, project.path(), &[]).success();
}

/// `vibe clean install` — the Maven phase chain: wipe, then the full world
/// reinstalls from the kept lock.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CHAIN-GRAMMAR")]
fn clean_install_rebuilds_the_whole_world() {
    let user = UserScratch::new();
    let project = project_with_fixture_registry(&user);
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    install(&user, project.path(), &["flow:org.vibevm/integration-beta"]).success();

    clean(&user, project.path(), &["install"]).success();

    assert!(
        slot(project.path(), "org.vibevm.integration-alpha", "0.1.0").is_dir()
            && slot(project.path(), "org.vibevm.integration-beta", "0.1.0").is_dir(),
        "clean install rebuilds every slot"
    );
    assert!(
        project.path().join(common::index_rel()).is_file(),
        "clean install regenerates the boot artifacts"
    );
}

/// `vibe clean install <pkgref>` — wipe, reinstall everything, then refresh
/// the named package; the world stays whole.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CHAIN-PKGREF-SEMANTICS")]
fn clean_install_with_a_pkgref_refreshes_the_named_and_keeps_the_world() {
    let user = UserScratch::new();
    let project = project_with_fixture_registry(&user);
    install(
        &user,
        project.path(),
        &["flow:org.vibevm/integration-alpha"],
    )
    .success();
    install(&user, project.path(), &["flow:org.vibevm/integration-beta"]).success();

    clean(
        &user,
        project.path(),
        &["install", "flow:org.vibevm/integration-alpha"],
    )
    .success();

    assert!(
        slot(project.path(), "org.vibevm.integration-alpha", "0.1.0").is_dir()
            && slot(project.path(), "org.vibevm.integration-beta", "0.1.0").is_dir(),
        "the chained pkgref refresh keeps the whole world"
    );
}
