//! `[recommends]` on the project plane, and `--no-recommends` as the decline
//! (PROP-077 `#PROJECT-RECOMMENDS`, `#NO-RECOMMENDS-FLAG`; PROP-050
//! `#WEAK-EDGES-ARE-ORDINARY-EDGES`).
//!
//! The owner's decision of 2026-11-10 — «Вариант 1 делай» — is that a package
//! recommended by a project dependency is installed and visible to its
//! recommender as if it were that package's ordinary dependency, with three
//! differences and no more: it is best-effort (an unsatisfiable or conflicting
//! recommendation is a warning and a report line, never a solve failure), it
//! is never promoted to a root of the project, and `--no-recommends` skips the
//! expansion entirely. These tests measure all three ends to end — the lock
//! row and its `weak-chain` provenance, the materialised slot, `vibe why`, the
//! report rows, and the replay.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use predicates::prelude::*;

/// A directory registry holding one package that recommends another, both
/// the smallest manifest a `flow` package can be — plus the two
/// best-effort cases: a recommendation nothing can satisfy, and one that
/// conflicts with the package recommending it.
fn registry(root: &Path) -> PathBuf {
    let registry = root.join("weak-registry");
    write(
        &registry,
        "recommender",
        "\n[recommends]\npackages = [\"org.vibevm.fixture/companion\"]\n\
         \n[suggests]\npackages = [\"org.vibevm.fixture/hint\"]\n",
    );
    write(&registry, "companion", "");
    write(
        &registry,
        "unsatisfiable",
        "\n[recommends]\npackages = [\"org.vibevm.fixture/companion@^9.0.0\"]\n",
    );
    write(
        &registry,
        "rival",
        "\n[recommends]\npackages = [\"org.vibevm.fixture/enemy\"]\n",
    );
    write(
        &registry,
        "enemy",
        "\n[conflicts]\npackages = [\"org.vibevm.fixture/rival\"]\n",
    );
    registry
}

fn write(registry: &Path, name: &str, extra: &str) {
    let dir = registry
        .join("org.vibevm.fixture")
        .join(name)
        .join("v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "{name}"
group = "org.vibevm.fixture"
kind = "flow"
version = "1.0.0"
authors = ["vibevm test fixtures"]
license = "UPL-1.0"
description = "PROP-077 weak-dependency fixture"
{extra}"#
        ),
    )
    .unwrap();
}

fn lock(project: &Path) -> String {
    fs::read_to_string(project.join("vibe.lock")).unwrap()
}

fn manifest(project: &Path) -> String {
    fs::read_to_string(project.join("vibe.toml")).unwrap()
}

/// The locked row for one package, from its `[[package]]` header to the next.
fn lock_row(project: &Path, name: &str) -> String {
    let text = lock(project);
    let needle = format!("name = \"{name}\"");
    let start = text
        .find(&needle)
        .unwrap_or_else(|| panic!("`{name}` is absent from vibe.lock:\n{text}"));
    let head = text[..start].rfind("[[package]]").unwrap_or(0);
    let tail = text[start..]
        .find("\n[[package]]")
        .map_or(text.len(), |offset| start + offset);
    text[head..tail].to_string()
}

/// `vibe install <pkg>` with and without the flag; both must succeed. The
/// returned text is stdout and stderr together: the report lines are stdout
/// and a skipped recommendation is a warning, which belongs on stderr.
fn install(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
    package: Option<&str>,
    decline: bool,
) -> String {
    let mut command = user.vibe();
    command.arg("install");
    if decline {
        command.arg("--no-recommends");
    }
    if let Some(package) = package {
        command.arg(package);
    }
    let output = command
        .arg("--path")
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .assert()
        .success()
        .get_output()
        .clone();
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A dependency's recommendation is installed, visible through its
/// recommender, recorded weak, and never a root of the project.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS")]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-050#WEAK-EDGES-ARE-ORDINARY-EDGES")]
fn a_recommended_package_is_installed_visible_to_its_recommender_and_never_a_root() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());

    let stdout = install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        false,
    );

    // It is installed: a lock row of its own, a materialised slot, and the
    // human report says what the recommendation brought in.
    let row = lock_row(project.path(), "companion");
    assert!(
        row.contains("admitted_by = \"weak-chain\""),
        "the lock records the weak edge as the provenance of this member:\n{row}"
    );
    assert!(
        project
            .path()
            .join("vibevm/vibedeps/org.vibevm.fixture.companion/1.0.0/vibe.toml")
            .is_file()
            || project
                .path()
                .join("vibedeps/org.vibevm.fixture.companion/1.0.0/vibe.toml")
                .is_file(),
        "the recommended package is materialised like any other member"
    );
    assert!(
        stdout.contains("Recommended and installed"),
        "the install report names what the recommendation installed:\n{stdout}"
    );
    assert!(
        stdout.contains("org.vibevm.fixture/hint"),
        "and reports the suggestion it installs nowhere:\n{stdout}"
    );

    // It is NOT promoted to the project root: the manifest the user owns is
    // untouched, and the lock's root set names only what the user asked for.
    assert!(
        !manifest(project.path()).contains("companion"),
        "a recommendation is never written into the user's `[requires]`"
    );
    let meta = lock(project.path());
    let roots = meta
        .split("[[package]]")
        .next()
        .expect("the lock has a [meta] head");
    assert!(
        roots.contains("recommender") && !roots.contains("companion"),
        "meta.root_dependencies carries the asked-for root and not the recommendation:\n{roots}"
    );

    // And `vibe why` explains it through its recommender, in the lock's own
    // vocabulary.
    user.vibe()
        .arg("why")
        .arg("org.vibevm.fixture/companion")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("weak-chain"))
        .stdout(predicate::str::contains(
            "org.vibevm.fixture/recommender -> org.vibevm.fixture/companion",
        ));
}

/// A recommendation nothing satisfies, and one that conflicts with its own
/// recommender: a warning and a report row each, and an install that succeeds
/// (PROP-003 `#CAP-WEAK-DEPS-WARNING`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS")]
fn an_unsatisfiable_or_conflicting_recommendation_warns_and_is_skipped() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());

    for (root, companion) in [("unsatisfiable", "companion"), ("rival", "enemy")] {
        let project = tempfile::tempdir().unwrap();
        common::init_project(project.path());
        let package = format!("org.vibevm.fixture/{root}");
        let stdout = install(&user, project.path(), &registry, Some(&package), false);

        assert!(
            stdout.contains(&format!("recommends `org.vibevm.fixture/{companion}`")),
            "the skip is a named warning, not silence:\n{stdout}"
        );
        assert!(
            stdout.contains("Recommended and skipped: 1"),
            "and the report tallies it:\n{stdout}"
        );
        let text = lock(project.path());
        assert!(
            text.contains(root),
            "the install the user asked for still happened:\n{text}"
        );
        assert!(
            !text.contains(&format!("name = \"{companion}\"")),
            "and nothing it could not install is in the lock:\n{text}"
        );
    }
}

/// `--no-recommends`: the greedy weak expansion is not run at all, so the
/// companion is not installed and the decline is reported rather than silent
/// (`PROP-077#NO-RECOMMENDS-FLAG`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn the_flag_declines_the_whole_expansion_and_says_so() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());

    let stdout = install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        true,
    );
    let declined = lock(project.path());
    assert!(declined.contains("recommender"));
    assert!(
        !declined.contains("name = \"companion\""),
        "the declined recommendation reaches neither the lock nor the tree:\n{declined}"
    );
    assert!(
        stdout.contains("--no-recommends"),
        "the decline names the flag that caused it:\n{stdout}"
    );
    assert!(
        stdout.contains("org.vibevm.fixture/hint"),
        "`[suggests]` is not silenced by the flag — it installs nothing anyway:\n{stdout}"
    );
}

/// The flag changes a solve, never a replay: a locked weak member is what was
/// installed, so a bare `vibe install` materialises it again whichever way the
/// flag is passed, and `vibe reinstall` replays the same lock. A deliberate
/// re-solve is where the flag decides, and `vibe update --no-recommends`
/// therefore drops the member it declines.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS")]
fn a_locked_weak_member_is_replayed_and_only_a_re_solve_reconsiders_it() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());

    install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        false,
    );
    let installed = lock(project.path());
    assert!(installed.contains("name = \"companion\""));

    // A replay writes the same bytes whichever way the flag is passed: the
    // lock is the record of what was installed.
    install(&user, project.path(), &registry, None, false);
    assert_eq!(installed, lock(project.path()));
    install(&user, project.path(), &registry, None, true);
    assert_eq!(
        installed,
        lock(project.path()),
        "a replay is not a solve — the flag has nothing to decide here"
    );

    // `vibe reinstall` replays the lock too, and admits the flag.
    user.vibe()
        .arg("reinstall")
        .arg("--no-recommends")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(lock(project.path()).contains("name = \"companion\""));

    // A bare `vibe update` is a SCOPED update: it moves the roots it
    // re-resolves and removes no member outside them, so it admits the flag
    // without reconsidering a locked weak member.
    user.vibe()
        .arg("update")
        .arg("--no-recommends")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(&registry)
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(lock(project.path()).contains("name = \"companion\""));

    // The deliberate re-solve is where the flag decides: an explicit
    // `vibe install <pkg> --no-recommends` recomputes the whole effective set,
    // so the member it declines leaves the lock and its slot is pruned.
    install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        true,
    );
    let declined = lock(project.path());
    assert!(
        !declined.contains("name = \"companion\""),
        "a re-solve applies the flag as it is passed then:\n{declined}"
    );

    // And the same re-solve with recommendations on brings it back, weak again.
    install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        false,
    );
    assert!(lock_row(project.path(), "companion").contains("admitted_by = \"weak-chain\""));
}

/// The `--json` document carries the same account as the terminal, as rows
/// rather than prose (`AV-FMT-105`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#PROJECT-RECOMMENDS")]
fn the_json_report_carries_a_row_per_recommendation_and_suggestion() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());

    let output = user
        .vibe()
        .arg("install")
        .arg("org.vibevm.fixture/recommender")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(&registry)
        .arg("--assume-yes")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // The command emits a stream of documents; the install report is the one
    // whose `command` is exactly `install`.
    let document: serde_json::Value = serde_json::Deserializer::from_str(&stdout)
        .into_iter::<serde_json::Value>()
        .filter_map(Result::ok)
        .find(|value| value.get("command").and_then(|c| c.as_str()) == Some("install"))
        .unwrap_or_else(|| panic!("no install report in:\n{stdout}"));

    let recommended = document["recommended"]
        .as_array()
        .unwrap_or_else(|| panic!("no `recommended` member in:\n{document}"));
    assert_eq!(recommended.len(), 1);
    assert_eq!(recommended[0]["package"], "org.vibevm.fixture/companion");
    assert_eq!(recommended[0]["by"], "org.vibevm.fixture/recommender");
    assert_eq!(recommended[0]["outcome"], "installed");
    assert!(recommended[0].get("reason").is_none());

    let suggested = document["suggested"]
        .as_array()
        .unwrap_or_else(|| panic!("no `suggested` member in:\n{document}"));
    assert_eq!(suggested.len(), 1);
    assert_eq!(suggested[0]["package"], "org.vibevm.fixture/hint");
    assert_eq!(suggested[0]["by"], "org.vibevm.fixture/recommender");
}

/// A flag silently ignored is worse than a flag refused
/// (`PROP-077#NO-RECOMMENDS-FLAG`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn a_command_that_neither_solves_nor_installs_refuses_the_flag_by_name() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());
    user.vibe()
        .arg("list")
        .arg("--no-recommends")
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--no-recommends"));
}
