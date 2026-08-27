//! Red oracles for the capability-safe roll-forward transaction.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_core::manifest::SkillDecl;
use vibe_core::{Group, PackageKind, PackageName};

use super::nofollow::Project;
use super::stage::Stage;
use super::state::{canonicalize_receipt, read_receipt, receipt_binding, write_receipt};
use crate::pkgskill::{
    DeclaredSkillProvider, ProjectSkillProviderInput, lower_project_skill_bindings,
    probe_project_skill_binding, reconcile_project_skill_binding,
    reconcile_vanished_project_skill_bindings, recover_project_skill_bindings,
};

pub(super) fn provider(
    root: &Path,
    name: &str,
    skill: &str,
    agents: &[&str],
) -> ProjectSkillProviderInput {
    ProjectSkillProviderInput {
        provider: DeclaredSkillProvider::Authored {
            group: Group::parse("org.example").unwrap(),
            name: PackageName::parse(name).unwrap(),
            version: "0.1.0".into(),
            kind: PackageKind::Tool,
            root: root.to_path_buf(),
        },
        declarations: vec![SkillDecl {
            name: skill.into(),
            path: "skills/body".into(),
            description: None,
            agents: agents.iter().map(|agent| (*agent).into()).collect(),
            include: Vec::new(),
        }],
    }
}

pub(super) fn seed(root: &Path, name: &str) -> tempfile::TempDir {
    let package = tempfile::tempdir().unwrap();
    fs::create_dir_all(package.path().join("skills/body")).unwrap();
    fs::write(
        package.path().join("skills/body/SKILL.md"),
        format!("body-{name}"),
    )
    .unwrap();
    let _ = root;
    package
}

/// Stage and durably publish the intent for `binding`, then "die" (return
/// without executing) — exactly the crash window recovery must close.
pub(super) fn begin_interrupted(
    project_root: &Path,
    binding: &crate::pkgskill::ProjectSkillBinding,
) {
    let project = Project::open(project_root).unwrap();
    let _guard = project.lock(super::nofollow::LOCK_FILE).unwrap();
    let files = binding.selected_files.as_ref().unwrap();
    let stage = Stage::create(&project, files).unwrap();
    let mut receipt = read_receipt(&project)
        .unwrap()
        .unwrap_or_else(super::state::empty_receipt);
    // The committed before-state drops any prior row for this binding; the
    // applying intent carries the complete desired after-state.
    receipt.binding.retain(|row| row.key != binding.identity());
    let mut before = receipt.clone();
    canonicalize_receipt(&mut before);
    let after_row = receipt_binding(binding, files);
    receipt.binding.push(after_row);
    canonicalize_receipt(&mut receipt);
    let mut intent = before;
    intent.applying = Some(
        vibe_wire::generated::package_skill_receipt::PackageSkillApplying {
            binding: receipt.binding.clone(),
            key: binding.identity(),
            nonce: stage.nonce.clone(),
        },
    );
    write_receipt(&project, &intent).unwrap();
    let _ = stage; // stage stays durable on "death"
}

pub(super) fn lower(
    project_root: &Path,
    inputs: Vec<ProjectSkillProviderInput>,
) -> Vec<crate::pkgskill::ProjectSkillBinding> {
    lower_project_skill_bindings(project_root, inputs).unwrap()
}

#[test]
fn interrupted_later_binding_recovers_before_earlier_one() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let two = seed(project.path(), "two");
    let bindings = lower(
        project.path(),
        vec![
            provider(one.path(), "one", "alpha", &["claude"]),
            provider(two.path(), "two", "beta", &["claude"]),
        ],
    );
    let (alpha, beta) = (bindings[0].clone(), bindings[1].clone());
    reconcile_project_skill_binding(project.path(), &alpha).unwrap();
    // Crash inside beta: durable intent, no mutation yet.
    begin_interrupted(project.path(), &beta);
    let outside = Path::new(&beta.targets[0].path).join("SKILL.md");
    // The engine-owned recovery finishes beta first…
    let recovered = recover_project_skill_bindings(project.path()).unwrap();
    assert_eq!(recovered.len(), 1, "{recovered:?}");
    assert_eq!(fs::read_to_string(&outside).unwrap(), "body-two");
    // …and only then does the earlier ordinary binding run clean.
    let after = reconcile_project_skill_binding(project.path(), &alpha).unwrap();
    assert!(after.iter().all(|report| report.status == "unchanged"));
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_none());
}

#[test]
fn intent_alone_is_never_ownership_over_foreign_bytes() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    let target = Path::new(&alpha.targets[0].path);
    fs::create_dir_all(target).unwrap();
    fs::write(target.join("SKILL.md"), "someone-elses\n").unwrap();
    // Third-party bytes at an intended path: refuse and preserve.
    let error = recover_project_skill_bindings(project.path()).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("refusing unowned pre-existing file"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "someone-elses\n"
    );
    // The intent stays durable for a manual resolution path.
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_some());
    // Exact staged desired bytes recover forward.
    fs::write(target.join("SKILL.md"), "body-one").unwrap();
    recover_project_skill_bindings(project.path()).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
}

#[test]
fn changed_declaration_after_crash_recovers_stored_plan_then_reconciles() {
    let project = tempfile::tempdir().unwrap();
    let one = tempfile::tempdir().unwrap();
    fs::create_dir_all(one.path().join("skills/body")).unwrap();
    fs::write(one.path().join("skills/body/SKILL.md"), "v1").unwrap();
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    // The source and therefore the desired plan change after the crash.
    fs::write(one.path().join("skills/body/SKILL.md"), "v2").unwrap();
    let target = Path::new(&alpha.targets[0].path);
    // Recovery completes the *stored* plan without user history restoration…
    recover_project_skill_bindings(project.path()).unwrap();
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "v1");
    // …then the new desired reconciliation converges on top.
    let changed = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &changed[0]).unwrap();
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "v2");
}

#[test]
fn returned_failure_never_rolls_back_neighbours_and_retry_converges() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    begin_interrupted(project.path(), alpha);
    let target = Path::new(&alpha.targets[0].path);
    fs::create_dir_all(target).unwrap();
    // Death before any intended file was published; a concurrent neighbour
    // exists inside the target directory.
    fs::write(target.join("NEIGHBOR.md"), "keep\n").unwrap();
    // Simulate a failing retry: the durable stage loses its file's bytes.
    let project_cap = Project::open(project.path()).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    let nonce = receipt.applying.as_ref().unwrap().nonce.clone();
    drop(project_cap);
    let staged_dir = project
        .path()
        .join(".vibe/package-skills/staged")
        .join(&nonce)
        .join("files");
    let staged_name = fs::read_dir(&staged_dir).unwrap().next().unwrap().unwrap();
    let staged_path = staged_dir.join(staged_name.file_name());
    let staged_bytes = fs::read(&staged_path).unwrap();
    fs::remove_file(&staged_path).unwrap();
    let error = recover_project_skill_bindings(project.path()).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("required staged file") && error.contains("is missing"),
        "{error}"
    );
    // No rollback, no deletion: the neighbour stays and nothing was created.
    assert_eq!(
        fs::read_to_string(target.join("NEIGHBOR.md")).unwrap(),
        "keep\n"
    );
    assert!(!target.join("SKILL.md").exists());
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(receipt.applying.is_some(), "intent stays for retry");
    // Repair the stage and retry: converges without touching the neighbour.
    fs::write(&staged_path, staged_bytes).unwrap();
    recover_project_skill_bindings(project.path()).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
    assert_eq!(
        fs::read_to_string(target.join("NEIGHBOR.md")).unwrap(),
        "keep\n"
    );
    let artifacts = alpha
        .targets
        .iter()
        .map(
            |target| vibe_wire::generated::lifecycle_state::StateArtifact {
                id: alpha.artifact_id(target.agent),
                kind: "agent-skill".into(),
                path: vibe_core::machine_json_path(&target.path),
            },
        )
        .collect::<Vec<_>>();
    assert!(probe_project_skill_binding(project.path(), alpha, &artifacts).unwrap());
}

#[test]
fn vanished_reconciliation_removes_owned_and_preserves_neighbours() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    fs::write(target.join("NEIGHBOR.md"), "foreign").unwrap();

    let reports =
        reconcile_vanished_project_skill_bindings(project.path(), &BTreeSet::new()).unwrap();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(!target.join("SKILL.md").exists());
    assert_eq!(
        fs::read_to_string(target.join("NEIGHBOR.md")).unwrap(),
        "foreign"
    );
    assert!(target.exists(), "the neighbour keeps the target directory");
}

#[test]
fn tampered_owned_file_refuses_removal_and_preserves_bytes() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    fs::write(target.join("SKILL.md"), "tampered").unwrap();

    let error =
        reconcile_vanished_project_skill_bindings(project.path(), &BTreeSet::new()).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("refusing to remove tampered owned file"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "tampered"
    );
}

#[cfg(windows)]
fn make_junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .is_ok_and(|status| status.success())
}

/// Red proof that a parent swapped for a junction at the mutation boundary
/// cannot escape the project. Phase A proves the attack hook fires (rename +
/// junction succeeds with no capability pinning the tree, junction attribute
/// real). Phase B proves the defence: while the directory capability is
/// retained, Windows refuses the very rename the swap needs and the pinned
/// write lands in the project's own directory, canary byte-identical.
#[cfg(windows)]
#[test]
fn junction_swapped_at_mutation_boundary_cannot_escape() {
    use std::os::windows::fs::MetadataExt;
    let project = tempfile::tempdir().unwrap();
    let target_rel = [".claude", "skills", "alpha"];
    let target = project.path().join(".claude/skills/alpha");
    fs::create_dir_all(&target).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let canary = outside.path().join("CANARY.md");
    fs::write(&canary, "outside-bytes").unwrap();
    // Phase A — the attack hook fires when nothing pins the namespace.
    let real = project.path().join(".claude-real");
    fs::rename(project.path().join(".claude"), &real).unwrap();
    assert!(
        make_junction(&project.path().join(".claude"), outside.path()),
        "attack hook must be able to fire"
    );
    let hooked = fs::symlink_metadata(project.path().join(".claude")).unwrap();
    assert!(
        hooked.file_attributes() & 0x400 != 0,
        "attack hook fired: `.claude` is a junction"
    );
    // Undo the un-pinned swap.
    fs::remove_dir(project.path().join(".claude")).unwrap();
    fs::rename(&real, project.path().join(".claude")).unwrap();
    // Phase B — under a retained capability the swap cannot even happen…
    let capability = Project::open(project.path()).unwrap();
    let pinned = capability.dir(&target_rel, false).unwrap();
    let swap_denied = fs::rename(project.path().join(".claude"), &real).is_err();
    assert!(
        swap_denied,
        "the namespace swap must be refused while the capability pins it"
    );
    // …and the pinned mutation writes into the project's own directory.
    capability
        .write_atomic_in(&pinned, "SKILL.md", b"pinned-write")
        .unwrap();
    assert_eq!(
        fs::read_to_string(&canary).unwrap(),
        "outside-bytes",
        "outside canary must stay byte-identical"
    );
    assert_eq!(
        fs::read_to_string(project.path().join(".claude/skills/alpha/SKILL.md")).unwrap(),
        "pinned-write"
    );
}

/// A junction already present on the walk refuses before any mutation.
#[cfg(windows)]
#[test]
fn junction_on_the_walk_refuses() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("CANARY.md"), "outside-bytes").unwrap();
    assert!(make_junction(
        &project.path().join(".claude"),
        outside.path()
    ));
    let capability = Project::open(project.path()).unwrap();
    let error = capability
        .dir(&[".claude", "skills", "alpha"], true)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("opening no-follow directory") || error.contains("reparse"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("CANARY.md")).unwrap(),
        "outside-bytes"
    );
}

#[test]
fn nested_selected_files_publish_and_reconcile() {
    let project = tempfile::tempdir().unwrap();
    let one = tempfile::tempdir().unwrap();
    fs::create_dir_all(one.path().join("skills/body/references")).unwrap();
    fs::write(one.path().join("skills/body/SKILL.md"), "body").unwrap();
    fs::write(one.path().join("skills/body/references/guide.md"), "guide").unwrap();
    let mut input = provider(one.path(), "one", "alpha", &["claude"]);
    input.declarations[0].include = vec!["SKILL.md".into(), "references/**".into()];
    let bindings = lower(project.path(), vec![input]);
    let alpha = &bindings[0];
    let reports = reconcile_project_skill_binding(project.path(), alpha).unwrap();
    assert_eq!(reports.len(), 1, "{reports:?}");
    let target = Path::new(&alpha.targets[0].path);
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "body");
    assert_eq!(
        fs::read_to_string(target.join("references/guide.md")).unwrap(),
        "guide"
    );
}

#[test]
fn target_only_deleted_and_tampered_files_reconcile() {
    let project = tempfile::tempdir().unwrap();
    let one = tempfile::tempdir().unwrap();
    fs::create_dir_all(one.path().join("skills/body/references")).unwrap();
    fs::write(one.path().join("skills/body/SKILL.md"), "first").unwrap();
    fs::write(one.path().join("skills/body/references/guide.md"), "guide").unwrap();
    let mut input = provider(one.path(), "one", "alpha", &["claude"]);
    input.declarations[0].include = vec!["SKILL.md".into(), "references/**".into()];
    let bindings = lower(project.path(), vec![input]);
    let alpha = &bindings[0];
    reconcile_project_skill_binding(project.path(), alpha).unwrap();
    let target = Path::new(&alpha.targets[0].path);
    fs::remove_file(target.join("SKILL.md")).unwrap();
    let repaired = reconcile_project_skill_binding(project.path(), alpha).unwrap();
    assert_eq!(repaired[0].status, "updated", "{repaired:?}");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "first"
    );
    fs::write(target.join("SKILL.md"), "tampered").unwrap();
    let healed = reconcile_project_skill_binding(project.path(), alpha).unwrap();
    assert_eq!(healed[0].status, "updated", "{healed:?}");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "first"
    );
}

#[test]
fn unowned_preexisting_target_refuses_wholesale() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    let target = Path::new(&alpha.targets[0].path);
    fs::create_dir_all(target).unwrap();
    fs::write(target.join("HUMAN.md"), "foreign").unwrap();
    let error = reconcile_project_skill_binding(project.path(), alpha).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("refusing unowned pre-existing package skill target"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(target.join("HUMAN.md")).unwrap(),
        "foreign"
    );
}
