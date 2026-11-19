use super::*;
use std::fs;
use std::time::SystemTime;

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn token() -> CancellationToken {
    CancellationToken::new()
}

fn names_of(path: &Path) -> usize {
    fs::read_dir(path).unwrap().count()
}

fn mtime(path: &Path) -> (SystemTime, u64) {
    let metadata = fs::metadata(path).unwrap();
    (metadata.modified().unwrap(), metadata.len())
}

#[test]
fn status_of_an_absent_root_is_absent_and_creates_nothing() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    assert!(matches!(status(&root), Observation::Absent));
    assert!(!root.exists());
    assert_eq!(fs::read_dir(outer.path()).unwrap().count(), 0);
}

#[test]
fn a_deep_absent_path_is_absent() {
    let outer = scratch();
    let root = outer.path().join("a/b/user-project");
    assert!(matches!(status(&root), Observation::Absent));
    assert!(!outer.path().join("a").exists());
}

#[test]
fn status_of_an_empty_existing_directory_is_empty_and_creates_nothing() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    assert!(matches!(status(&root), Observation::Empty));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[test]
fn init_publishes_the_ordinary_manifest_and_authored_directories() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let created = init(&root, &token()).unwrap();
    let report = created.report();
    assert_eq!(report.outcome, Outcome::Created);
    assert_eq!(report.state, State::Initialized);
    assert_eq!(report.root.as_deref(), Some(root.as_path()));
    assert_eq!(report.effects, Effects::Changed);
    assert_eq!(report.publication, Publication::Committed);
    assert!(report.is_success());
    assert!(report.root.is_some());

    // The manifest is what the ordinary parser and role admission accept.
    let text = fs::read_to_string(root.join(MANIFEST_NAME)).unwrap();
    assert_eq!(text, initial_manifest());
    let parsed = Manifest::parse_str(&text).unwrap();
    assert_eq!(parsed.require_project().unwrap().name, INITIAL_NAME);
    assert!(root.join("vibevm/vibespecs").is_dir());
    assert!(root.is_dir());

    // Nothing else: no lock, no boot output, no `.vibe`, no ignore file.
    let mut entries: Vec<String> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, ["vibe.toml", "vibevm"]);
}

#[test]
fn a_second_init_leaves_bytes_and_mtimes_untouched() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    assert!(matches!(
        init(&root, &token()).unwrap(),
        Initialized::Created(_)
    ));
    let manifest = root.join(MANIFEST_NAME);
    // Backdate, so a rewrite would be visible as a changed time.
    let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(&manifest)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let before = mtime(&manifest);
    let specs = names_of(&root.join("vibevm/vibespecs"));
    match init(&root, &token()).unwrap() {
        Initialized::AlreadyInitialized(report) => {
            assert_eq!(report.outcome, Outcome::AlreadyInitialized);
            assert_eq!(report.effects, Effects::None);
            assert_eq!(report.publication, Publication::NotStarted);
        }
        Initialized::Created(_) => panic!("a second init must not create"),
    }
    assert_eq!(mtime(&manifest), before);
    assert_eq!(names_of(&root.join("vibevm/vibespecs")), specs);
}

#[test]
fn an_admitted_project_without_authored_directories_needs_no_filling() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join(MANIFEST_NAME),
        "[project]\nname = \"mine\"\nversion = \"2.0.0\"\n",
    )
    .unwrap();
    assert!(matches!(
        status(&root),
        Observation::Initialized { ref name } if name == "mine"
    ));
    assert!(matches!(
        init(&root, &token()).unwrap(),
        Initialized::AlreadyInitialized(_)
    ));
    assert!(!root.join("vibevm").exists());
}

#[test]
fn a_project_with_a_workspace_table_is_admitted_without_member_traversal() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join(MANIFEST_NAME),
        "[project]\nname = \"host\"\nversion = \"1.0.0\"\n\n[workspace]\n\
         members = [\"never-there/*\"]\n",
    )
    .unwrap();
    assert!(matches!(status(&root), Observation::Initialized { .. }));
}

#[test]
fn a_package_only_manifest_is_refused() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join(MANIFEST_NAME),
        "[package]\ngroup = \"org.example\"\nname = \"tool\"\n\
         version = \"0.1.0\"\nkind = \"tool\"\n",
    )
    .unwrap();
    let Observation::Unavailable(refusal) = status(&root) else {
        panic!("a package-only manifest must be unavailable");
    };
    assert_eq!(refusal.report().outcome, Outcome::Observed);
    assert!(
        refusal
            .report()
            .reason
            .as_deref()
            .unwrap()
            .contains("package-only")
    );
    assert!(init(&root, &token()).is_err());
    // Refusals never touch the bytes they read.
    assert!(root.join(MANIFEST_NAME).exists());
}

#[test]
fn a_virtual_workspace_manifest_is_refused() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(MANIFEST_NAME), "[workspace]\nmembers = [\"a\"]\n").unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
}

#[test]
fn a_malformed_manifest_is_refused_without_its_text_in_the_reason() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    let sentinel = "PRIVATE-SENTINEL";
    fs::write(
        root.join(MANIFEST_NAME),
        format!("[project\nname = \"{sentinel}\"\nversion = \"1.0.0\"\n"),
    )
    .unwrap();
    let Observation::Unavailable(refusal) = status(&root) else {
        panic!("malformed TOML must be unavailable");
    };
    let reason = refusal.report().reason.clone().unwrap();
    assert!(reason.contains("does not parse"));
    assert!(!reason.contains(sentinel));
}

#[test]
fn a_nonempty_foreign_directory_is_refused_and_kept() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/file.txt"), b"keep me").unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert_eq!(fs::read(root.join("nested/file.txt")).unwrap(), b"keep me");
    assert!(!root.join(MANIFEST_NAME).exists());
}

#[test]
fn a_root_holding_only_a_directory_is_nonempty_residue() {
    // The empty question is the ROOT's emptiness, not a recursive scan.
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir_all(root.join("left/over")).unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(root.join("left/over").is_dir());
    assert!(!root.join(MANIFEST_NAME).exists());
}

#[test]
fn a_file_at_the_root_is_refused() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::write(&root, b"i am not a directory").unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert_eq!(fs::read(&root).unwrap(), b"i am not a directory");
}

#[test]
fn a_directory_at_the_manifest_name_is_refused() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join(MANIFEST_NAME)).unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
}

#[cfg(unix)]
#[test]
fn a_link_at_the_root_refuses() {
    let outer = scratch();
    let real = outer.path().join("real");
    fs::create_dir(&real).unwrap();
    let root = outer.path().join("user-project");
    std::os::unix::fs::symlink(&real, &root).unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
}

#[cfg(unix)]
#[test]
fn a_link_in_an_existing_ancestor_refuses() {
    let outer = scratch();
    let real = outer.path().join("real/deeper");
    fs::create_dir_all(&real).unwrap();
    let alias = outer.path().join("alias");
    std::os::unix::fs::symlink(outer.path().join("real"), &alias).unwrap();
    let root = alias.join("deeper/user-project");
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(!real.join("user-project").exists());
}

#[test]
fn a_concurrent_manifest_publication_never_overwrites() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    // What a racing initializer leaves just after its own publication.
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    let theirs_tree = fs::read_dir(root.join("vibevm")).unwrap().count();
    fs::write(
        root.join(MANIFEST_NAME),
        "[project]\nname = \"theirs\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    match init(&root, &token()).unwrap() {
        Initialized::AlreadyInitialized(_) => {}
        Initialized::Created(_) => panic!("the adversary's project must not be recreated"),
    }
    let text = fs::read_to_string(root.join(MANIFEST_NAME)).unwrap();
    assert!(text.contains("theirs"));
    assert_eq!(
        fs::read_dir(root.join("vibevm")).unwrap().count(),
        theirs_tree
    );
}

#[test]
fn a_foreign_directory_behind_the_authored_one_is_refused() {
    // Pre-publication residue under `vibevm/` cannot be adopted: it never
    // ran the manifest rule.
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir_all(root.join("vibevm/journal")).unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(root.join("vibevm/journal").is_dir());
    assert!(!root.join(MANIFEST_NAME).exists());
}

#[test]
fn cancellation_before_the_first_write_reports_refused_not_started() {
    let outer = scratch();
    let root = outer.path().join("deep/user-project");
    let cancellation = token();
    cancellation.cancel();
    let refusal = init(&root, &cancellation).unwrap_err();
    let report = refusal.report();
    assert_eq!(report.outcome, Outcome::Refused);
    assert_eq!(report.state, State::Unavailable);
    assert_eq!(report.effects, Effects::None);
    assert_eq!(report.publication, Publication::NotStarted);
    assert!(report.root.is_some());
    assert!(!root.exists());
}

#[test]
fn publication_marks_the_token_committed() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let cancellation = token();
    init(&root, &cancellation).unwrap();
    assert_eq!(cancellation.publication_state(), "committed");
    cancellation.cancel();
    assert!(root.join(MANIFEST_NAME).is_file());
}

#[test]
fn an_empty_existing_root_is_initialized_in_place() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    let created = init(&root, &token()).unwrap();
    assert_eq!(created.report().outcome, Outcome::Created);
    assert!(root.join(MANIFEST_NAME).is_file());
    assert!(root.join("vibevm/vibespecs").is_dir());
}

#[test]
fn a_refusal_carries_the_admitted_root_and_a_remedy() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("stray.txt"), b"x").unwrap();
    let refusal = init(&root, &token()).unwrap_err();
    let report = refusal.report();
    // A refusal keeps the whole observation: the root is known, the reason
    // and remedy are set, and it is not a success.
    assert!(report.root.is_some());
    assert!(report.reason.is_some());
    assert!(report.remedy.is_some());
    assert!(!report.is_success());
    assert_eq!(report.publication, Publication::NotStarted);
}

#[cfg(unix)]
#[test]
fn a_link_in_the_middle_of_the_path_refuses() {
    // The link is an existing component, so the walk through it refuses.
    let outer = scratch();
    let real = outer.path().join("real");
    fs::create_dir_all(real.join("inner")).unwrap();
    let alias = outer.path().join("real/inner/alias");
    std::os::unix::fs::symlink(real.join("inner"), &alias).unwrap();
    let root = alias.join("user-project");
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(!real.join("inner/user-project").exists());
}

#[cfg(unix)]
#[test]
fn a_link_as_the_last_existing_ancestor_refuses_without_mutation() {
    let outer = scratch();
    let real = outer.path().join("real");
    fs::create_dir(&real).unwrap();
    let alias = outer.path().join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let root = alias.join("user-project");
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(!real.join("user-project").exists());
}

#[test]
fn a_deep_root_under_a_present_parent_is_created_from_the_middle() {
    let outer = scratch();
    let root = outer.path().join("deep/user-project");
    assert!(matches!(
        init(&root, &token()).unwrap(),
        Initialized::Created(_)
    ));
    assert!(root.join(MANIFEST_NAME).is_file());
    assert!(root.join("vibevm/vibespecs").is_dir());
}

#[test]
fn a_root_under_an_absent_chain_under_a_file_refuses() {
    // The parent is itself missing, so absence is not provable: the chain
    // holds a regular file.
    let outer = scratch();
    let file = outer.path().join("file");
    fs::write(&file, b"x").unwrap();
    let root = file.join("a/user-project");
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
}

#[test]
fn the_initial_manifest_parses_as_a_project_manifest() {
    let parsed = Manifest::parse_str(&initial_manifest()).unwrap();
    let project = parsed.require_project().unwrap();
    assert_eq!(project.name, INITIAL_NAME);
    assert_eq!(project.version, INITIAL_VERSION);
    assert!(parsed.workspace.is_none());
    assert!(parsed.package.is_none());
}

#[test]
fn a_trailing_dot_component_does_not_capture_the_root() {
    let outer = scratch();
    let dot = outer.path().join("user-project/.");
    assert!(matches!(status(&dot), Observation::Absent));
    assert!(matches!(
        init(&dot, &token()).unwrap(),
        Initialized::Created(_)
    ));
    assert!(outer.path().join("user-project/vibe.toml").is_file());
}

#[test]
fn parent_steps_refuse_without_creating_a_project_outside_the_root() {
    let outer = scratch();
    let root = outer.path().join("user-project/..");
    assert!(matches!(status(&root), Observation::Unavailable(_)));
    assert!(init(&root, &token()).is_err());
    assert!(!outer.path().join("vibe.toml").exists());
}

#[test]
fn a_transaction_stage_residue_does_not_make_the_root_nonempty() {
    // The create-new stage lives at the root only while a publication is in
    // flight; a crash can leave one, and it must not read as a foreign root.
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(".vibe-stage-1-0"), initial_manifest()).unwrap();
    assert!(matches!(status(&root), Observation::Unavailable(_)));
}

#[test]
fn init_is_a_single_visible_publication_step() {
    // Nothing may be left beside the manifest: no stage residue survives a
    // clean publication.
    let outer = scratch();
    let root = outer.path().join("user-project");
    init(&root, &token()).unwrap();
    let names: Vec<String> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !names.iter().any(|name| name.starts_with(".vibe-stage-")),
        "{names:?}"
    );
}

#[test]
fn a_root_under_an_existing_empty_chain_is_created_by_one_init() {
    let outer = scratch();
    let root = outer.path().join("a/b/c/user-project");
    assert!(matches!(
        init(&root, &token()).unwrap(),
        Initialized::Created(_)
    ));
    assert!(outer.path().join("a/b/c").is_dir());
    assert!(root.join(MANIFEST_NAME).is_file());
}

#[test]
fn a_stop_before_the_root_exists_reports_from_the_starting_state() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let cancellation = token();
    cancellation.cancel();
    let refusal = init(&root, &cancellation).unwrap_err();
    // What was found is recorded so the reader can tell "nothing here" from
    // "a refusal over existing data".
    let report = refusal.report();
    assert_eq!(report.publication, Publication::NotStarted);
    assert_eq!(report.outcome, Outcome::Refused);
    assert!(
        report
            .reason
            .as_deref()
            .unwrap()
            .contains("before the first write")
    );
}

#[test]
fn a_component_that_is_a_file_where_a_directory_belongs_refuses() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("vibevm"), b"not a directory").unwrap();
    // The root is nonempty and has no manifest; members are not inspected
    // recursively, so this is the plain nonempty refusal.
    assert!(matches!(status(&root), Observation::Unavailable(_)));
}

#[test]
fn init_over_an_empty_root_creates_a_parseable_project_only() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    init(&root, &token()).unwrap();
    let text = fs::read_to_string(root.join(MANIFEST_NAME)).unwrap();
    let manifest = Manifest::parse_str(&text).unwrap();
    assert!(manifest.require_project().is_ok());
    assert!(manifest.package.is_none());
}

#[test]
fn an_incomplete_report_is_never_a_success() {
    let report = Report::refused(Command::Init, None, "interrupted", "inspect residue")
        .refused_with("interrupted");
    assert_eq!(report.outcome, Outcome::Incomplete);
    assert!(!report.is_success());
}

#[path = "failure_tests.rs"]
mod failures;
