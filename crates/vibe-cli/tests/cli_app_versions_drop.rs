//! The `in drop` mark of `vibe app versions` end to end through the real
//! `vibe` binary (PROP-066 `#drops`, PROP-075 `#VERSIONS-LIST-OUTPUT`):
//! which rows an `[artifacts] packages` drop already holds the payload of
//! for this platform, the `--in-drop` narrowing, the `--json` member, and
//! the two things the mark deliberately will not do — read a byte of a
//! payload, or guess a file name a skeleton render cannot settle.
//!
//! The domain side is the same real `org.vibevm.node/node` template over
//! upstream's frozen documents the sibling suite drives, behind one
//! `[[remap]]` row, so **no run here opens a socket**; the drop side is a
//! directory this suite fills by hand, exactly as a person does.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

const TEMPLATE: &str = include_str!("../../vibe-template/tests/fixtures/node/vibe.toml");
const REVISION: &str = "1.0.0";
const INDEX: &str = include_str!("../../vibe-template/tests/fixtures/node/index-excerpt.json");
const PUBLISHED_24: &str =
    include_str!("../../vibe-template/tests/fixtures/node/vibe-24.22.2.toml");
const ORIGIN: &str = "https://nodejs.org/dist/";

/// The archive Node publishes for this host — the one file the installer
/// option an unattended install selects here downloads, and therefore the
/// one name the drop is searched by.
fn node_archive(version: &str) -> String {
    let slice = match std::env::consts::ARCH {
        "x86_64" => "linux-x64",
        "aarch64" => "linux-arm64",
        other => panic!("the fixture template publishes no linux option for `{other}`"),
    };
    format!("node-v{version}-{slice}.tar.gz")
}

struct Fixture {
    registry: AppRegistry,
}

impl Fixture {
    /// The template registry with upstream's frozen version list behind
    /// one remap row and the newest concrete tag published beside the
    /// template, plus an empty drop directory.
    fn new() -> Self {
        let registry = AppRegistry::new();
        let root = registry.root.path();
        let dir = registry
            .registry()
            .join("org.vibevm.node/node")
            .join(format!("v{REVISION}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), TEMPLATE).unwrap();
        let concrete = registry.registry().join("org.vibevm.node/node/v24.22.2");
        fs::create_dir_all(&concrete).unwrap();
        fs::write(concrete.join("vibe.toml"), PUBLISHED_24).unwrap();

        let mirror = root.join("dist");
        fs::create_dir_all(&mirror).unwrap();
        fs::write(mirror.join("index.json"), INDEX).unwrap();
        fs::write(
            root.join("remap.toml"),
            format!(
                "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
                 reason = \"the suite's frozen copy of the Node.js release host\"\n",
                mirror.display()
            ),
        )
        .unwrap();
        fs::create_dir_all(root.join("drop")).unwrap();
        Fixture { registry }
    }

    fn drop_dir(&self) -> PathBuf {
        self.registry.root.path().join("drop")
    }

    /// Declare the drop in `config.toml`, as PROP-066 `#drops` has a
    /// person do.
    fn configure_drop(&self) {
        fs::write(
            self.registry.settings().join("config.toml"),
            format!(
                "[artifacts]\npackages = \"{}\"\n",
                self.drop_dir().display()
            ),
        )
        .unwrap();
    }

    /// Put a file of `bytes` at `relative` inside the drop. The bytes are
    /// deliberately not a real archive: the listing must never read them.
    fn place(&self, relative: &str) -> PathBuf {
        let path = self.drop_dir().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not an archive, and never read by a listing\n").unwrap();
        path
    }

    fn remap(&self) -> PathBuf {
        self.registry.root.path().join("remap.toml")
    }

    /// `vibe app versions <target> --registry … --remap … <extra…>`, human.
    fn human(&self, target: &str, extra: &[&str]) -> String {
        let remap = self.remap();
        let mut args: Vec<&str> = vec!["--remap", remap.to_str().unwrap()];
        args.extend_from_slice(extra);
        ok(&self.registry.app(&["app", "versions"], target, &args))
    }

    /// The same with `--json`, through the generated reader.
    fn listing(&self, target: &str, extra: &[&str]) -> Value {
        let remap = self.remap();
        let mut args: Vec<&str> = vec!["--remap", remap.to_str().unwrap(), "--json"];
        args.extend_from_slice(extra);
        json(&self.registry.app(&["app", "versions"], target, &args))
    }

    fn refusal(&self, target: &str, extra: &[&str]) -> String {
        let remap = self.remap();
        let mut args: Vec<&str> = vec!["--remap", remap.to_str().unwrap()];
        args.extend_from_slice(extra);
        failed(&self.registry.app(&["app", "versions"], target, &args))
    }
}

/// The rows of the one heading, as `(version, drop paths)`.
fn marks(listed: &Value) -> Vec<(String, Vec<String>)> {
    listed["packages"][0]["versions"]
        .as_array()
        .expect("the rows")
        .iter()
        .map(|row| {
            (
                row["version"].as_str().unwrap_or_default().to_owned(),
                row["drop"]
                    .as_array()
                    .map(|paths| {
                        paths
                            .iter()
                            .map(|path| path.as_str().unwrap_or_default().to_owned())
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn mark_of(listed: &Value, version: &str) -> Vec<String> {
    marks(listed)
        .into_iter()
        .find(|(row, _)| row == version)
        .unwrap_or_else(|| panic!("the row of {version}"))
        .1
}

/// Every rung of the by-name convention serves the mark, both for a
/// version the template produces and for one the registry publishes
/// concretely — and the mark is the path the file was found at.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn each_rung_of_the_name_convention_marks_a_row_and_names_where_it_found_it() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    // Rung 1, the one a person is told to create: `<G>/<N>/<V>/<F>`. The
    // version is produced by the template and nothing is published for it.
    let produced = fixture.place(&format!(
        "org.vibevm.node/node/26.13.0/{}",
        node_archive("26.13.0")
    ));
    // Rung 1 again, for the version published concretely beside the
    // template — the row whose manifest is read off disk rather than
    // rendered (PROP-075 `#TEMPLATE-CONCRETE-WINS`).
    let published = fixture.place(&format!(
        "org.vibevm.node/node/24.22.2/{}",
        node_archive("24.22.2")
    ));
    // Rung 2, the version-less package directory: `<G>/<N>/<F>`.
    let versionless = fixture.place(&format!("org.vibevm.node/node/{}", node_archive("22.23.4")));
    // Rung 3, the flat drop: `<F>`.
    let flat = fixture.place(&node_archive("22.9.0"));

    let listed = fixture.listing("org.vibevm.node/node", &["--all"]);
    assert_eq!(
        mark_of(&listed, "26.13.0"),
        [produced.display().to_string()]
    );
    assert_eq!(
        mark_of(&listed, "24.22.2"),
        [published.display().to_string()]
    );
    assert_eq!(
        mark_of(&listed, "22.23.4"),
        [versionless.display().to_string()]
    );
    assert_eq!(mark_of(&listed, "22.9.0"), [flat.display().to_string()]);
    // Every other row is unmarked, and that is the common case.
    let unmarked = marks(&listed)
        .into_iter()
        .filter(|(_, paths)| paths.is_empty())
        .count();
    assert!(unmarked > 400, "only four rows are marked: {unmarked}");

    // The human listing carries the mark in the same trailing segment the
    // local state uses, and only on the marked rows.
    let human = fixture.human("org.vibevm.node/node", &["--all"]);
    assert!(
        human.contains("26.13.0 (template) [lts, lts/lithium] — in drop"),
        "{human}"
    );
    assert!(
        human.contains("24.22.2 (published (overrides template)) [lts, lts/krypton] — in drop"),
        "{human}"
    );
    assert_eq!(
        human.matches("— in drop").count(),
        4,
        "an unmarked row carries no segment at all: {human}"
    );
    assert!(
        human.contains("12.12.0 (template)\n"),
        "an unmarked row with no channels is bare: {human}"
    );
}

/// The digest-keyed rung is **not** searched: a listing has no digest
/// document, so a file filed only under `<drop>/<sha256>/<F>` goes
/// unmarked rather than being found by a guessed digest.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn the_digest_keyed_rung_is_not_consulted_by_a_listing() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    // The real digest of Node 26.13.0's linux-x64 archive, as upstream's
    // own frozen `SHASUMS256.txt` carries it — the rung an install finds
    // and a listing deliberately does not.
    let digest = "9b0d4aa4d20a4f1afaf2cdcf99175ad90d1856d24c7d31dc46f72369b73e2d35";
    fixture.place(&format!("{digest}/{}", node_archive("26.13.0")));
    let listed = fixture.listing("org.vibevm.node/node", &["--limit", "5"]);
    assert!(
        mark_of(&listed, "26.13.0").is_empty(),
        "the digest rung is never searched by name alone"
    );
}

/// A row is marked only when **every** input the selected option
/// downloads is found: half a payload installs nothing.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn a_row_needs_every_input_of_its_option_in_the_drop() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    // The archive of another platform is not this host's input, so it
    // marks nothing — which is the whole point of the mark being about
    // the option an install here would select.
    fixture.place("org.vibevm.node/node/26.13.0/node-v26.13.0-win-x64.zip");
    let listed = fixture.listing("org.vibevm.node/node", &["--limit", "5"]);
    assert!(mark_of(&listed, "26.13.0").is_empty(), "{listed}");
    // With this host's own archive beside it, the row is marked.
    fixture.place(&format!(
        "org.vibevm.node/node/26.13.0/{}",
        node_archive("26.13.0")
    ));
    let listed = fixture.listing("org.vibevm.node/node", &["--limit", "5"]);
    assert_eq!(mark_of(&listed, "26.13.0").len(), 1, "{listed}");
}

/// `--in-drop` narrows **before** the limit, so `total`, `shown` and
/// `more` describe the narrowed set — exactly as `PATTERN` does.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn in_drop_narrows_before_the_limit_and_the_totals_describe_the_narrowed_set() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    for version in ["26.13.0", "24.22.2", "22.23.4"] {
        fixture.place(&format!(
            "org.vibevm.node/node/{version}/{}",
            node_archive(version)
        ));
    }
    let whole = fixture.listing("org.vibevm.node/node", &["--all"]);
    let domain = whole["packages"][0]["total"].as_u64().unwrap();
    assert!(domain > 400, "the whole frozen window: {domain}");

    let narrowed = fixture.listing("org.vibevm.node/node", &["--in-drop"]);
    let package = &narrowed["packages"][0];
    // Three rows, and no `more` — the default limit of twenty never
    // enters, because the narrowing happened first.
    assert_eq!(package["total"], 3);
    assert_eq!(package["shown"], 3);
    assert_eq!(package["more"], 0);
    assert_eq!(
        marks(&narrowed)
            .into_iter()
            .map(|(version, _)| version)
            .collect::<Vec<_>>(),
        ["26.13.0", "24.22.2", "22.23.4"]
    );
    assert!(
        marks(&narrowed).iter().all(|(_, paths)| paths.len() == 1),
        "every kept row carries its paths: {narrowed}"
    );

    // A limit below the narrowed count still counts the rest of the
    // narrowed set, not of the domain.
    let limited = fixture.listing("org.vibevm.node/node", &["--in-drop", "--limit", "2"]);
    assert_eq!(limited["packages"][0]["total"], 3);
    assert_eq!(limited["packages"][0]["shown"], 2);
    assert_eq!(limited["packages"][0]["more"], 1);

    // And it composes with PATTERN, both narrowings before the limit.
    let both = fixture.listing("org.vibevm.node/node", &["--in-drop", "22"]);
    assert_eq!(both["packages"][0]["total"], 1);
    assert_eq!(marks(&both)[0].0, "22.23.4");
}

/// No drop configured is no mark and no error; `--in-drop` without one is
/// a refusal that names `[artifacts] packages`, because the narrowing it
/// asks for cannot be performed and an empty listing would read as an
/// empty domain.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn without_a_drop_nothing_is_marked_and_in_drop_refuses_by_name() {
    let fixture = Fixture::new();
    // The file is in place, but no drop is declared, so it is not a drop.
    fixture.place(&format!(
        "org.vibevm.node/node/26.13.0/{}",
        node_archive("26.13.0")
    ));
    let listed = fixture.listing("org.vibevm.node/node", &["--limit", "5"]);
    assert!(
        marks(&listed).iter().all(|(_, paths)| paths.is_empty()),
        "{listed}"
    );
    let human = fixture.human("org.vibevm.node/node", &["--limit", "5"]);
    assert!(!human.contains("in drop"), "{human}");

    let refusal = fixture.refusal("org.vibevm.node/node", &["--in-drop"]);
    assert!(refusal.contains("no_package_drop"), "{refusal}");
    assert!(refusal.contains("[artifacts]"), "{refusal}");
    assert!(refusal.contains("packages"), "{refusal}");
}

/// A version whose file names a **skeleton** render cannot settle is
/// unmarked, never guessed.
///
/// A digest variable is legal only in the digest-bearing fields, so a
/// file name can never itself depend on a digest document; what can go
/// unknown is a name built from a version record's own metadata that one
/// version does not carry. The rule is the same either way: no manifest
/// at the skeleton stage, no mark.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-TWO-STAGE")]
fn a_version_whose_name_needs_more_than_a_skeleton_is_unmarked() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    // A second template package whose archive name embeds a `{meta.…}`
    // field only one of its two versions carries.
    let dir = fixture
        .registry
        .registry()
        .join("org.example/metatool/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"metatool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
             version = \"1.0.0\"\ntemplate = true\n\n[application]\nid = \"metatool\"\n\n\
             [template.versions]\nkind = \"list\"\n\n\
             [[template.versions.item]]\nversion = \"2.0.0\"\nmeta = {{ slice = \"wide\" }}\n\n\
             [[template.versions.item]]\nversion = \"1.5.0\"\n\n\
             [template.digests]\nkind = \"sha256sums\"\n\
             url = \"https://h.example/v{{version}}/SHASUMS256.txt\"\n\n\
             [[installer_options]]\nid = \"linux\"\nname = \"Metatool {{version}}\"\n\
             platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
             path_dirs = [\"bin\"]\n\n\
             [[installer_options.steps]]\nid = \"unpack\"\ntitle = \"Unpack\"\n\
             kind = \"archive\"\nformat = \"tar_gz\"\n\
             source = \"https://h.example/metatool-{{version}}-{{meta.slice}}.tar.gz\"\n\
             sha256 = \"{{sha256:metatool-{{version}}-{{meta.slice}}.tar.gz}}\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    // Both candidate names are in the drop; only the version whose record
    // settles the name may be marked.
    fixture.place("org.example/metatool/2.0.0/metatool-2.0.0-wide.tar.gz");
    fixture.place("org.example/metatool/1.5.0/metatool-1.5.0-.tar.gz");
    fixture.place("org.example/metatool/1.5.0/metatool-1.5.0-wide.tar.gz");

    let listed = fixture.listing("org.example/metatool", &["--all"]);
    assert_eq!(mark_of(&listed, "2.0.0").len(), 1, "{listed}");
    assert!(
        mark_of(&listed, "1.5.0").is_empty(),
        "a name the skeleton cannot settle is unknown, not guessed: {listed}"
    );
    // And the filter keeps only what it could answer for.
    let narrowed = fixture.listing("org.example/metatool", &["--in-drop", "--all"]);
    assert_eq!(narrowed["packages"][0]["total"], 1);
    assert_eq!(marks(&narrowed)[0].0, "2.0.0");
}

/// The whole 410-row window is marked with **no digest document and no
/// render at all** — the cost budget of PROP-075 `#TEMPLATE-TWO-STAGE`,
/// asserted as the absence of the artefacts either would leave behind,
/// with a wall-clock bound beside it.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-TWO-STAGE")]
fn marking_every_row_costs_no_digest_document_and_no_render() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    fixture.place(&format!(
        "org.vibevm.node/node/26.13.0/{}",
        node_archive("26.13.0")
    ));
    let started = std::time::Instant::now();
    let listed = fixture.listing("org.vibevm.node/node", &["--all"]);
    let elapsed = started.elapsed();
    let total = listed["packages"][0]["total"].as_u64().unwrap();
    assert!(total > 400, "the whole window was marked: {total}");
    assert_eq!(mark_of(&listed, "26.13.0").len(), 1, "{listed}");

    let cache: &Path = fixture.registry.settings();
    let template = cache.join("cache/templates/org.vibevm.node/node");
    assert!(
        !template.join("digests").exists(),
        "no row cost a digest document"
    );
    assert!(
        !template.join("rendered").exists(),
        "a listing renders nothing into a store"
    );
    // One process, 410 probed rows, no network and no digest: a generous
    // ceiling that a per-row fetch or render could not possibly meet.
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "410 marked rows took {elapsed:?}"
    );
}

/// `--in-drop` and `--installed` are refused together: `--installed`
/// answers from this home and the template cache alone, with no registry
/// and so no manifest to take a file name from, so there is nothing for
/// the filter to narrow.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn in_drop_and_installed_are_refused_together() {
    let fixture = Fixture::new();
    fixture.configure_drop();
    let refusal = fixture.refusal("org.vibevm.node/node", &["--in-drop", "--installed"]);
    assert!(refusal.contains("--installed"), "{refusal}");
    assert!(refusal.contains("in-drop"), "{refusal}");
}
