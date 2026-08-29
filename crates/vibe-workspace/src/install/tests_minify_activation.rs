//! `xml-minify` activation END TO END (R4 architecture §2.5 of the R4.2
//! packet; §11 rows 3, 8, 9 and 12).
//!
//! **Why every test here drives a SECOND regeneration.** T10B's ratification
//! is explicit: during `vibe install` the boot lane is written BEFORE the
//! resolution's lock is published, so the on-disk lock is the PRE-install
//! epoch and a world observed against it never existed. Boot regeneration owns
//! no epoch, so it treats "the lock and the tree disagree" as *nothing
//! observable* and writes the exact historical bytes. Every test therefore
//! installs, publishes the lock the install's resolution implies, and asserts
//! on the regeneration AFTER that — which is the first moment the declaration
//! is observable at all. The install pass's own lane is asserted too, as the
//! unobserved baseline, so the two epochs are told apart rather than assumed.
//!
//! What is proved: the owner's lane carries the §7.1 header naming exactly the
//! active list; its bytes are strictly smaller than the unminified twin while
//! every document's parsed node set and every contribution marker survive; a
//! further regeneration is byte-stable; removing the declaration restores the
//! exact historical bytes; and activation is OWNER-scoped — a member activates
//! its own lane and leaves the root's untouched.

use super::test_helpers::*;
use super::*;

use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};

use crate::boot_artifacts;

/// The declaration every fixture activates, at the one stage the kernel can
/// serve.
const MINIFY_DECL: &str = r#"
[[extension]]
id = "minify"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }
"#;

/// The header line an owner activating exactly this declaration writes.
///
/// The key is `<host identity>#<declaration id>`; every fixture node declares
/// a `group`, so its host identity is the grouped coordinate
/// `org.demo/<name>`. The token is that key encoded by the shared
/// generated-comment codec, which leaves it verbatim — no `%`, and its one
/// hyphen is interior.
fn header_line(project: &str) -> String {
    format!("<!-- vibe:transforms org.demo/{project}#minify -->")
}

/// One statically linked dependency carrying a real, indented XML-renderable
/// document — enough structure that minifying it beats the header's own bytes.
fn static_dependency() -> (ResolvedDep, TempDir) {
    let mut body = String::from("# Tools {#root}\n");
    for index in 0..12 {
        body.push_str(&format!(
            "\n## Section {index} {{#s{index}}}\n\nparagraph {index} of the tools flow\n"
        ));
    }
    dep_with_boot(
        "tools",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/tools.md\"\nlink = \"static\"\n",
        "boot/tools.md",
        &body,
    )
}

/// Publish the lock the install's resolution implies — the POST-install epoch.
///
/// Written by the test rather than by `apply_resolution`, because publishing
/// the lock is the command owner's job (`vibe-cli`), and this cell exercises
/// the workspace half. Its contents agree with the materialised tree, which is
/// exactly the condition under which the world becomes observable.
fn publish_lock(root: &Path) {
    let mut lockfile = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lockfile.packages = vec![LockedPackage {
        kind: PackageKind::Flow,
        name: PackageName::parse("tools").expect("a valid name"),
        group: Group::parse("org.vibevm").expect("a valid group"),
        version: ver("1.0.0"),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").expect("a valid hash"),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: Vec::new(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }];
    lockfile
        .write(root.join(Lockfile::FILENAME))
        .expect("the fixture lock writes");
}

/// The root manifest of a single-node fixture, with or without the activating
/// declaration.
fn root_manifest(activated: bool) -> String {
    let mut manifest = String::from(
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/tools\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    if activated {
        manifest.push_str(MINIFY_DECL);
    }
    manifest
}

/// Install a single-node fixture in the XML lane, publish its lock, and hand
/// back the workspace directory. The lane on disk at this point is the
/// install pass's — the UNOBSERVED one.
fn installed(activated: bool) -> (TempDir, TempDir) {
    let ws_dir = TempDir::new().expect("a temp workspace");
    write(ws_dir.path(), "vibe.toml", &root_manifest(activated));
    let (dependency, package) = static_dependency();
    let ws = Workspace::load(ws_dir.path()).expect("the fixture workspace loads");
    apply_resolution_with_spec_format(
        &ws,
        &[dependency],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        None,
        None,
    )
    .expect("the install applies");
    publish_lock(ws_dir.path());
    (ws_dir, package)
}

/// Regenerate one workspace's boot artifacts from the materialised tree — the
/// POST-install pass, which does observe the world.
fn regenerate(root: &Path) {
    let ws = Workspace::load(root).expect("the workspace reloads");
    regenerate_boot_with_spec_format(&ws, SpecFormat::Xml).expect("the regeneration succeeds");
}

/// One node's compiled XML lane.
fn lane(root: &Path, node_rel: &str) -> String {
    let node = if node_rel == "." {
        root.to_path_buf()
    } else {
        root.join(node_rel)
    };
    fs::read_to_string(
        node.join(vibe_core::layout::current_boot_dir())
            .join(boot_artifacts::static_file(SpecFormat::Xml)),
    )
    .expect("the node's XML lane exists")
}

/// Every `<?xml …?> … </spec>` document in one emitted tape, in order.
fn documents(tape: &str) -> Vec<&str> {
    const DECL: &str = "<?xml version=";
    const CLOSE: &str = "</spec>";
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = tape[cursor..].find(DECL) {
        let start = cursor + relative;
        let end = start
            + tape[start..]
                .find(CLOSE)
                .expect("an opened document closes")
            + CLOSE.len();
        found.push(&tape[start..end]);
        cursor = end;
    }
    found
}

/// Every engine-framed comment line, in order — the record a reader recovers
/// from the tape.
fn frame_comments(tape: &str) -> Vec<&str> {
    tape.lines()
        .filter(|line| line.starts_with("<!-- vibe:c1 "))
        .collect()
}

/// Assert `minified` is `baseline` minified: strictly smaller, header
/// present, every document's node set and every frame comment identical.
#[track_caller]
fn assert_minified_twin(baseline: &str, minified: &str, header: &str) {
    assert!(
        minified.len() < baseline.len(),
        "the activated lane is strictly smaller: {} → {}",
        baseline.len(),
        minified.len()
    );
    assert_eq!(
        minified.lines().nth(3),
        Some(header),
        "the §7.1 header names exactly the active list, in its frozen position"
    );
    assert_eq!(minified.matches("vibe:transforms").count(), 1);
    assert_eq!(
        frame_comments(minified),
        frame_comments(baseline),
        "no engine-framed comment moved a byte"
    );
    let before = documents(baseline);
    let after = documents(minified);
    assert!(!before.is_empty(), "the fixture compiles real documents");
    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(&after) {
        assert_eq!(
            vibe_specdoc::from_xml(after).expect("the minified document parses"),
            vibe_specdoc::from_xml(before).expect("the baseline document parses"),
            "minifying preserves every document's parsed node set"
        );
    }
}

/// The whole activation law on a node lane, in one run: the install pass does
/// NOT observe, the post-install regeneration does, the result is the
/// baseline minified, and a further regeneration is byte-stable.
#[test]
fn a_node_lanes_activation_is_observed_only_after_install_and_is_byte_stable() {
    let (activated, _package) = installed(true);
    let install_pass = lane(activated.path(), ".");
    assert!(
        !install_pass.contains("vibe:transforms"),
        "the install pass writes the lane before the lock is published, so it \
         observes no world and takes the empty plan"
    );

    regenerate(activated.path());
    let observed = lane(activated.path(), ".");

    // The unactivated twin: the same world, the same install, no declaration.
    let (plain, _plain_package) = installed(false);
    regenerate(plain.path());
    let baseline = lane(plain.path(), ".");
    assert_eq!(
        baseline, install_pass,
        "an owner that activates nothing writes the exact historical bytes at \
         both epochs"
    );

    assert_minified_twin(&baseline, &observed, &header_line("host"));

    // Idempotent: a further regeneration recompiles from source and lands on
    // the same bytes, which is what makes a fingerprint-fresh skip honest.
    regenerate(activated.path());
    assert_eq!(lane(activated.path(), "."), observed);
}

/// Deactivating restores the EXACT historical bytes.
///
/// The declaration is removed from the manifest and the tree regenerated in
/// place, so what is compared is one workspace before and after — not two
/// fixtures that merely look alike.
#[test]
fn removing_the_declaration_restores_the_exact_historical_bytes() {
    let (workspace, _package) = installed(true);
    let historical = lane(workspace.path(), ".");
    regenerate(workspace.path());
    let minified = lane(workspace.path(), ".");
    assert_ne!(minified, historical);

    write(workspace.path(), "vibe.toml", &root_manifest(false));
    regenerate(workspace.path());
    assert_eq!(
        lane(workspace.path(), "."),
        historical,
        "deactivating restores the byte-for-byte historical lane"
    );
}

/// Activation authority follows the artifact being written: a MEMBER node's
/// own manifest activates the member's lane and leaves the root's untouched.
///
/// This is the first place T10B's member re-seating becomes byte-visible. Both
/// nodes are compiled in one run, from one lock, by one snapshot — and the two
/// lanes disagree, which they could only do if each was scoped by its own
/// manifest.
#[test]
fn a_member_activates_its_own_lane_and_the_roots_lane_is_untouched() {
    let ws_dir = TempDir::new().expect("a temp workspace");
    let root_toml = format!(
        "{}\n[workspace]\nmembers = [\"members/alpha\"]\n",
        root_manifest(false)
    );
    write(ws_dir.path(), "vibe.toml", &root_toml);
    write(
        ws_dir.path(),
        "members/alpha/vibe.toml",
        &format!(
            "[project]\ngroup = \"org.demo\"\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"org.vibevm/tools\" = {{ version = \"^1.0\", link = \"static\" }}\n{MINIFY_DECL}"
        ),
    );
    let (dependency, _package) = static_dependency();
    let ws = Workspace::load(ws_dir.path()).expect("the fixture workspace loads");
    apply_resolution_with_spec_format(
        &ws,
        &[dependency],
        SlotIntegrity::TrustPresence,
        SpecFormat::Xml,
        None,
        None,
    )
    .expect("the install applies");
    publish_lock(ws_dir.path());

    let root_before = lane(ws_dir.path(), ".");
    let member_before = lane(ws_dir.path(), "members/alpha");
    regenerate(ws_dir.path());

    assert_eq!(
        lane(ws_dir.path(), "."),
        root_before,
        "the root activates nothing, so its lane is byte-identical"
    );
    assert_minified_twin(
        &member_before,
        &lane(ws_dir.path(), "members/alpha"),
        &header_line("alpha"),
    );
}
