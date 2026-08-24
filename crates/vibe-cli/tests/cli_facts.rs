//! End-to-end contract for the W1 `vibe facts` command family.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#cli");

mod common;

use std::fs;

use common::UserScratch;

#[test]
fn facts_crud_filters_and_spec_to_registry_sync() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("tempdir");
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::create_dir_all(project.path().join(common::spec_rel("common"))).expect("spec dir");
    let spec = project.path().join(common::spec_rel("common/RULE.md"));
    let spec_bytes = "# Rule {#root}\n\n@fact:RULE The rule. @status:impl/done\n";
    fs::write(&spec, spec_bytes).expect("spec");

    let host = "spec://org.example/demo/common/RULE#RULE";
    let package = "spec://org.external/flow/flows/main#PACKAGE-RULE";

    user.vibe()
        .current_dir(project.path())
        .args([
            "facts",
            "set",
            host,
            "spec/work",
            "--comment",
            "adopted locally",
        ])
        .assert()
        .success();

    let get = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "get", host])
        .output()
        .expect("get");
    assert!(get.status.success());
    let stdout = String::from_utf8_lossy(&get.stdout);
    assert!(stdout.contains(host));
    assert!(stdout.contains("spec/work"));

    let list = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "list", "--status", "spec/work"])
        .output()
        .expect("list");
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains(host));

    let deferred = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "set", package, "impl/done"])
        .output()
        .expect("set uninstalled package fact");
    assert!(deferred.status.success());
    assert!(
        String::from_utf8_lossy(&deferred.stdout).contains("not installed; will apply at install")
    );
    assert!(
        project
            .path()
            .join(common::facts_rel("org.external.flow.toml"))
            .is_file()
    );
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "rm", package])
        .assert()
        .success();
    assert!(
        !project
            .path()
            .join(common::facts_rel("org.external.flow.toml"))
            .exists()
    );

    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync"])
        .assert()
        .failure();
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync", "--write"])
        .assert()
        .success();
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "sync"])
        .assert()
        .success();

    let get = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "get", host])
        .output()
        .expect("get after sync");
    assert!(String::from_utf8_lossy(&get.stdout).contains("impl/done"));
    assert_eq!(
        fs::read_to_string(spec).expect("spec unchanged"),
        spec_bytes
    );
}

#[test]
fn facts_clean_names_orphans_preserves_spec_and_honours_dry_run() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("project");
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\ngroup = \"org.consumer\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::write(
        project.path().join("vibe.lock"),
        "[meta]\ngenerated_by = \"test\"\ngenerated_at = \"2026-08-22T00:00:00Z\"\nschema_version = 6\n",
    )
    .expect("empty lockfile");
    fs::create_dir_all(project.path().join(common::facts_root())).expect("facts home");
    fs::write(
        project.path().join(common::facts_rel("spec.toml")),
        "schema = 1\n\n[[fact]]\naddress = \"spec://org.consumer/demo/common/RULE#HOST\"\norigin = \"spec\"\nstatus = \"impl/done\"\n",
    )
    .expect("spec facts");
    let orphan = project
        .path()
        .join(common::facts_rel("org.vanished.pkg.toml"));
    fs::write(
        &orphan,
        "schema = 1\n\n[[fact]]\naddress = \"spec://org.vanished/pkg/RULE#ONE\"\norigin = \"package\"\npackage = \"org.vanished/pkg\"\nstatus = \"impl/done\"\n",
    )
    .expect("orphan facts");

    let report = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "report", "--package", "org.vanished/pkg"])
        .output()
        .expect("orphan report");
    assert!(report.status.success());
    let stdout = String::from_utf8_lossy(&report.stdout);
    assert!(stdout.contains("org.vanished/pkg  1/? (indeterminate 0)"));
    assert!(stdout.contains("no materialised slot"));

    let dry_run = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "clean", "--dry-run"])
        .output()
        .expect("clean dry-run");
    assert!(dry_run.status.success());
    let stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(stdout.contains(&common::facts_rel("org.vanished.pkg.toml")));
    assert!(stdout.contains("1 entries"));
    assert!(stdout.contains("removed=0 kept=1 orphaned=1"));
    assert!(orphan.is_file());
    assert!(
        project
            .path()
            .join(common::facts_rel("spec.toml"))
            .is_file()
    );

    let clean = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "clean"])
        .output()
        .expect("clean");
    assert!(clean.status.success());
    let stdout = String::from_utf8_lossy(&clean.stdout);
    assert!(stdout.contains(&common::facts_rel("org.vanished.pkg.toml")));
    assert!(stdout.contains("removed=1 kept=0"));
    assert!(!orphan.exists());
    assert!(
        project
            .path()
            .join(common::facts_rel("spec.toml"))
            .is_file()
    );
}

#[test]
fn package_set_rederives_and_adopt_fills_only_absent_statuses() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("project");
    let registry = tempfile::tempdir().expect("registry");
    let package = registry
        .path()
        .join("org.example")
        .join("facts-pkg")
        .join("v1.0.0");
    fs::create_dir_all(package.join(vibe_core::layout::current_specs_root()))
        .expect("package spec dir");
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"facts-pkg\"\nkind = \"flow\"\nversion = \"1.0.0\"\nepoch = 1\n",
    )
    .expect("package manifest");
    fs::write(
        package.join(common::spec_rel("RULE.md")),
        "# Rules\n\n@fact:FIRST First. <status stage=\"spec\" state=\"work\" comment=\"author extra\"/>\n\n@fact:SECOND Second. @status:impl/done\n",
    )
    .expect("package spec");
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\ngroup = \"org.consumer\"\nname = \"demo\"\nversion = \"0.1.0\"\nspec_format = \"markdown\"\n",
    )
    .expect("project manifest");

    user.vibe()
        .args(["install", "org.example/facts-pkg@=1.0.0", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let first = "spec://org.example/facts-pkg/RULE#FIRST";
    let second = "spec://org.example/facts-pkg/RULE#SECOND";
    user.vibe()
        .current_dir(project.path())
        .args(["facts", "set", first, "impl/done"])
        .assert()
        .success();

    let slot = project
        .path()
        .join(common::slot_dir("org.example.facts-pkg", "1.0.0"));
    let materialised =
        fs::read_to_string(slot.join(common::spec_rel("RULE.md"))).expect("slot spec");
    assert!(materialised.contains("stage=\"impl\" state=\"done\" comment=\"author extra\""));
    let manifest = vibe_workspace::vibedeps::read_derived_manifest(&slot).expect("manifest");
    assert!(manifest.overlay_hash.is_some());
    fs::create_dir_all(slot.join(common::spec_rel("boot"))).expect("generated boot dir");
    fs::write(
        slot.join(common::spec_rel("boot/STATIC.xml")),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"GENERATED\" status=\"impl/done\">projection only</fact></p></spec>",
    )
    .expect("generated XML lane");

    let adopt = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "adopt", "--package", "org.example/facts-pkg"])
        .output()
        .expect("adopt");
    assert!(
        adopt.status.success(),
        "{}",
        String::from_utf8_lossy(&adopt.stderr)
    );
    let stdout = String::from_utf8_lossy(&adopt.stdout);
    assert!(stdout.contains("added=1 kept=1"), "{stdout}");
    let facts = fs::read_to_string(
        project
            .path()
            .join(common::facts_rel("org.example.facts-pkg.toml")),
    )
    .expect("package facts");
    assert!(facts.contains(first));
    assert!(facts.contains(second));
    assert!(!facts.contains("GENERATED"));

    let report = user
        .vibe()
        .current_dir(project.path())
        .args(["facts", "report", "--package", "org.example/facts-pkg"])
        .output()
        .expect("adoption report");
    assert!(report.status.success());
    assert!(
        String::from_utf8_lossy(&report.stdout)
            .contains("org.example/facts-pkg  2/2 (indeterminate 0)")
    );

    user.vibe()
        .current_dir(project.path())
        .args(["facts", "rm", first])
        .assert()
        .success();
    let restored =
        fs::read_to_string(slot.join(common::spec_rel("RULE.md"))).expect("restored slot spec");
    assert!(restored.contains("stage=\"spec\" state=\"work\" comment=\"author extra\""));

    user.vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .arg("--quiet")
        .assert()
        .success();

    let uninstall = user
        .vibe()
        .current_dir(project.path())
        .args(["uninstall", "org.example/facts-pkg", "--assume-yes"])
        .output()
        .expect("uninstall");
    assert!(
        uninstall.status.success(),
        "{}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    let stdout = String::from_utf8_lossy(&uninstall.stdout);
    assert!(stdout.contains("kept"));
    assert!(stdout.contains("run `vibe facts clean`"));
    assert!(
        project
            .path()
            .join(common::facts_rel("org.example.facts-pkg.toml"))
            .is_file()
    );
}

const MARKUP_LINT_MOVED_NOTE: &str = "note: the markup lint moved — prefer 'vibe facts check'";

fn markup_fixture(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root.join(common::specs_root())).expect("spec dir");
    fs::write(root.join(common::spec_rel("RULE.md")), body).expect("spec");
    fs::write(
        root.join("progress.toml"),
        format!("include = [\"{}/**/*.md\"]\n", common::specs_str()),
    )
    .expect("progress config");
}

fn assert_check_spellings_match(
    user: &UserScratch,
    root: &std::path::Path,
    expected_success: bool,
) {
    let facts = user
        .vibe()
        .args(["facts", "check", "--exhaustive", "--path"])
        .arg(root)
        .output()
        .expect("vibe facts check");
    let alias = user
        .vibe()
        .args(["progress", "check", "--exhaustive", "--path"])
        .arg(root)
        .output()
        .expect("vibe progress check");

    assert_eq!(facts.status.success(), expected_success, "{facts:?}");
    assert_eq!(alias.status.code(), facts.status.code(), "exit code");
    assert_eq!(alias.stdout, facts.stdout, "stdout");
    let expected_stderr = format!(
        "{MARKUP_LINT_MOVED_NOTE}\n{}",
        String::from_utf8_lossy(&facts.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&alias.stderr),
        expected_stderr,
        "the alias adds exactly one leading stderr line"
    );
}

#[test]
fn facts_check_and_progress_alias_match_on_valid_and_broken_markup() {
    let user = UserScratch::new();
    let valid = tempfile::tempdir().expect("valid tree");
    markup_fixture(
        valid.path(),
        "# Rule {#root}\n\n@fact:RULE Valid. @status:impl/done\n",
    );
    assert_check_spellings_match(&user, valid.path(), true);

    let broken = tempfile::tempdir().expect("broken tree");
    markup_fixture(
        broken.path(),
        "# Rule {#root}\n\n@fact:RULE Broken. @status:not-a-stage/done\n",
    );
    assert_check_spellings_match(&user, broken.path(), false);
}

#[test]
fn progress_check_json_suppresses_the_transition_note() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("project");
    markup_fixture(
        project.path(),
        "# Rule {#root}\n\n@fact:RULE Valid. @status:impl/done\n",
    );

    let output = user
        .vibe()
        .args(["--json", "progress", "check", "--path"])
        .arg(project.path())
        .output()
        .expect("JSON progress check");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "machine mode has no transition note"
    );
}

#[test]
fn bare_campaign_id_resolves_under_campaigns_and_missing_id_creates_nothing() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().expect("project");
    markup_fixture(
        project.path(),
        "# Rule {#root}\n\n@fact:RULE Valid. @status:impl/done\n",
    );
    let campaigns = project.path().join("campaigns");
    fs::create_dir_all(campaigns.join("demo")).expect("demo campaign");

    let existing = user
        .vibe()
        .args(["--quiet", "progress", "scan", "--path"])
        .arg(project.path())
        .args(["--campaign", "demo"])
        .output()
        .expect("scan existing campaign");
    assert!(existing.status.success(), "{existing:?}");
    assert!(
        campaigns.join("demo/run/state/campaign.json").is_file(),
        "the bare id writes inside campaigns/demo"
    );
    assert!(
        !project.path().join("demo").exists(),
        "the bare id never resolves from cwd"
    );

    let missing = user
        .vibe()
        .args(["--quiet", "progress", "scan", "--path"])
        .arg(project.path())
        .args(["--campaign", "missing"])
        .output()
        .expect("scan missing campaign");
    assert!(!missing.status.success(), "{missing:?}");
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(
        stderr.contains("campaign `missing` does not exist"),
        "{stderr}"
    );
    assert!(stderr.contains("existing campaigns: demo"), "{stderr}");
    assert!(
        stderr.contains("fix: pass an existing campaign id"),
        "{stderr}"
    );
    assert!(
        !campaigns.join("missing").exists(),
        "no nested garbage zone"
    );
    assert!(
        !project.path().join("missing").exists(),
        "no cwd garbage zone"
    );
}
