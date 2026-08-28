//! The provenance bundle's reds: one read, one load, one root, and no way to
//! recombine the three.

use super::*;
use crate::install::resolve_project_root;

fn project(body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(Manifest::FILENAME), body).unwrap();
    dir
}

const PLAIN: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";

#[test]
fn one_snapshot_answers_activation_and_still_yields_the_manifest() {
    let dir = project(&format!("{PLAIN}\n[compile]\ntrace = true\n"));
    let root = resolve_project_root(dir.path()).unwrap();
    let snapshot = SelectedManifest::read(&root);
    assert!(snapshot.request(false), "the manifest's standing request");
    let selection = snapshot.prepare();
    assert!(
        selection.request(false),
        "and the bound bundle answers identically — one snapshot, not two",
    );
    assert!(selection.prove().is_ok());
}

/// The rule this type exists for: a parse failure is DEFERRED, never turned
/// into "requests nothing and everything is fine".
#[test]
fn a_parse_failure_is_deferred_to_the_old_boundary_not_swallowed() {
    let dir = project("this is not toml {{{");
    let snapshot = SelectedManifest::read(dir.path());
    // The flag still speaks — an unreadable manifest cannot veto it.
    assert!(snapshot.request(true));
    assert!(!snapshot.request(false));
    let selection = snapshot.prepare();
    assert!(
        selection.loaded_workspace().is_none(),
        "no sound workspace can be built from a manifest that did not parse",
    );
    assert!(
        selection.prove().is_err(),
        "the error the command's own read used to raise must survive",
    );
}

/// The BUNDLE is what the disk cannot reach.
///
/// The manifest is read, the pair is bound, and only THEN is the file
/// corrupted. Every projection still answers from the snapshot, and the proof
/// still succeeds — which is what shows there is no second read hiding behind
/// any of them.
#[test]
fn disk_corruption_after_prepare_cannot_change_the_bundle() {
    let dir = project(PLAIN);
    let root = resolve_project_root(dir.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();

    std::fs::write(root.join(Manifest::FILENAME), "[project\nbroken\n").unwrap();

    assert_eq!(
        selection.root(),
        root,
        "the carried root is a value, not a lookup"
    );
    assert_eq!(selection.loaded_root(), Some(root.as_path()));
    assert_eq!(
        selection
            .parsed_ref()
            .and_then(|m| m.project.as_ref())
            .map(|p| p.name.as_str()),
        Some("demo"),
    );
    let proven = selection.prove().expect("the bundle proves from itself");
    assert_eq!(proven.root(), root);
    assert_eq!(proven.workspace().root, root);
    assert_eq!(
        proven.manifest().project.as_ref().unwrap().name,
        "demo",
        "the proven manifest is the snapshot's, not the corrupted file's",
    );

    // Deleting it entirely changes nothing either.
    std::fs::remove_file(root.join(Manifest::FILENAME)).unwrap();
    let (again_root, again_manifest, again_workspace) = proven.into_parts();
    assert_eq!(again_root, root);
    assert_eq!(again_workspace.root, root);
    assert_eq!(again_manifest.project.as_ref().unwrap().name, "demo");
}

#[test]
fn an_absent_manifest_is_an_error_the_boundary_still_reports() {
    let dir = tempfile::tempdir().unwrap();
    let snapshot = SelectedManifest::read(dir.path());
    assert!(!snapshot.request(false));
    let selection = snapshot.prepare();
    assert!(selection.loaded_workspace().is_none());
    assert!(selection.prove().is_err());
}

/// The FIRST answer is the only answer.
///
/// The manifest is valid but the tree does not load — a sibling that will not
/// parse. The disk is then REPAIRED. A prepared state that merely said "no
/// workspace" would let the execution seam discover again and succeed against a
/// tree the identity and the trace were never prepared for; the bundle carries
/// the first failure instead.
#[test]
fn a_repaired_sibling_cannot_turn_the_first_failure_into_success() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(Manifest::FILENAME),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\"a\"]\n",
    )
    .unwrap();
    let sibling = dir.path().join("a");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join(Manifest::FILENAME), "[package\nbroken\n").unwrap();

    let root = resolve_project_root(dir.path()).unwrap();
    let snapshot = SelectedManifest::read(&root);
    assert!(
        snapshot.parsed_ref().is_some(),
        "the SELECTED manifest itself is fine",
    );
    let selection = snapshot.prepare();
    assert!(
        selection.loaded_root().is_none(),
        "the tree did not load, so there is no canonical root a trace could be stored under",
    );

    // Repair the sibling — a later read would now succeed.
    std::fs::write(
        sibling.join(Manifest::FILENAME),
        "[package]\ngroup = \"org.x\"\nname = \"a\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert!(
        SelectedManifest::read(&root)
            .prepare()
            .loaded_root()
            .is_some(),
        "a SECOND preparation really would succeed — which is exactly why the carried \
         first answer is the one execution must consume",
    );
    let Err(error) = selection.prove() else {
        panic!("the FIRST answer is the only answer, and it was a failure");
    };
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("discovering the workspace enclosing the project"),
        "{rendered}",
    );
}

/// The post-clean reload rebinds the SAME snapshot at a new root, and never
/// re-reads the file.
#[test]
fn the_post_clean_reload_carries_the_snapshot_across_the_wipe() {
    let dir = project(PLAIN);
    let root = resolve_project_root(dir.path()).unwrap();
    let selection = SelectedManifest::read(&root).prepare();

    // The "wipe": the manifest on disk is replaced by one that would not parse.
    std::fs::write(root.join(Manifest::FILENAME), "[project\nbroken\n").unwrap();
    let reloaded = selection
        .reload_after_clean()
        .expect("the tree still loads from the carried snapshot");
    assert_eq!(reloaded.root(), root);
    assert_eq!(
        reloaded
            .parsed_ref()
            .and_then(|m| m.project.as_ref())
            .map(|p| p.name.as_str()),
        Some("demo"),
        "the reload rebinds the ORIGINAL snapshot, it does not read the file again",
    );
}

/// The API fence: an execution entry point takes the BUNDLE, and nothing else.
///
/// The mutation this kills is reintroducing independent `manifest` /
/// `workspace` / `project_root` fields on the public entry points. It is a
/// SOURCE scan rather than a type assertion because the defect is the absence
/// of a field: no signature can assert what it does not declare.
#[test]
fn no_public_execution_entry_takes_a_manifest_and_a_workspace_separately() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    for (file, carrier) in [
        ("install/mod.rs", "InstallExecution"),
        ("phase.rs", "PhaseRun"),
        ("prelude.rs", "RunPrelude"),
    ] {
        // Normalised: this tree is checked out with CRLF on Windows, and the
        // scan below is about declarations, not line endings.
        let body = std::fs::read_to_string(src.join(file))
            .unwrap()
            .replace(char::from(13), "");
        let start = body
            .find(&format!("pub struct {carrier}"))
            .unwrap_or_else(|| panic!("{carrier} is declared in {file}"));
        let end = start + body[start..].find("\n}\n").expect("the struct closes");
        let declaration = &body[start..end];
        for forbidden in [
            "pub manifest:",
            "pub workspace:",
            "pub project_root:",
            "pub selected_root:",
        ] {
            if declaration.contains(forbidden) {
                offenders.push(format!("{file}: {carrier} declares `{forbidden}`"));
            }
        }
        assert!(
            declaration.contains("pub selection:"),
            "{carrier} must carry the one provenance bundle",
        );
    }
    assert!(
        offenders.is_empty(),
        "an execution entry point cannot be handed independently forgeable pieces: {offenders:#?}",
    );
}

/// The snapshot remembers where it was read, so neither initial preparation
/// nor the post-clean reload accepts a caller-supplied replacement root.
#[test]
fn the_snapshot_api_has_no_root_rebinding_seam() {
    let body = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/install/selection.rs"),
    )
    .unwrap()
    .replace(char::from(13), "");

    let declaration_start = body.find("pub struct SelectedManifest").unwrap();
    let declaration_end = declaration_start
        + body[declaration_start..]
            .find("\n}\n")
            .expect("the snapshot declaration closes");
    assert!(
        body[declaration_start..declaration_end].contains("root: PathBuf"),
        "the read root must travel inside the snapshot",
    );

    for function in ["prepare", "reload_after_clean"] {
        let start = body
            .find(&format!("pub fn {function}("))
            .unwrap_or_else(|| panic!("{function} is public"));
        let end = start + body[start..].find(" {").expect("the signature ends");
        let signature = &body[start..end];
        assert_eq!(
            signature,
            format!(
                "pub fn {function}(self) -> {}",
                if function == "prepare" {
                    "PreparedSelection"
                } else {
                    "Result<Self>"
                }
            ),
            "{function} must not accept a replacement root: {signature}",
        );
    }
    assert!(
        !body.contains("pub fn parsed("),
        "a parsed manifest cannot enter without the root it was read from",
    );
}
