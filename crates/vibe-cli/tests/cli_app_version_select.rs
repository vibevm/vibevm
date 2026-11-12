//! `[versions] select` end to end through the real `vibe` binary
//! (PROP-075 `#VERSIONS-SELECT`, PROP-066 `#drops`).
//!
//! The scenario is the owner's own, built small enough to state exactly:
//! a directory registry carries **one template package** whose version
//! document — read off this filesystem through one `[[remap]]` row, so no
//! run here opens a socket — offers `22.23.4`, `22.23.5` and `24.22.3`,
//! while the `[artifacts] packages` drop holds the payload of `22.23.4`
//! alone, because that is the archive somebody carried in. `newest` then
//! resolves `tool@22` to a version whose bytes are nowhere on the
//! machine; `prefer-drop` resolves it to the one that is here and says
//! so once; `drop-only` refuses when nothing the requirement admits is.
//!
//! Every run is `install -g --plan-only`, which resolves the package and
//! installs nothing: the plan names the version the policy landed on,
//! which is the fact under test.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

/// The versions the fixture's template produces. `22.23.5` is the newest
/// of the `22` line and is deliberately **not** the one in the drop.
const VERSIONS: [&str; 3] = ["22.23.4", "22.23.5", "24.22.3"];

/// A digest the fixture's template states literally, because it declares
/// no `[template.digests]` (PROP-075 `#TEMPLATE-NO-DIGEST`): no run here
/// downloads or hashes a byte, so the value only has to be well formed.
const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";

const ORIGIN: &str = "https://fixture.example/";

/// The archive the installer option this host selects downloads — the one
/// name the drop is searched by.
fn archive(version: &str) -> String {
    format!("tool-{version}-linux-{}.tar.gz", std::env::consts::ARCH)
}

/// One template package producing [`VERSIONS`], with one installer option
/// for this host whose single `archive` step downloads [`archive`].
fn template() -> String {
    format!(
        "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
         version = \"1.0.0\"\ntemplate = true\n\
         description = \"Tool {{version}}\"\n\n\
         [application]\nid = \"tool\"\n\n\
         [template.versions]\nkind = \"json\"\nurl = \"{ORIGIN}versions.json\"\n\
         select = \"\"\nversion_field = \"version\"\n\n\
         [[template.channel]]\nrule = \"truthy\"\nname = \"lts\"\nfrom = \"lts\"\n\n\
         [[installer_options]]\nid = \"linux\"\nname = \"Tool {{version}}\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
         path_dirs = [\"bin\"]\n\n\
         [[installer_options.steps]]\nid = \"unpack\"\n\
         title = \"Unpack Tool {{version}}\"\nkind = \"archive\"\nformat = \"tar_gz\"\n\
         source = \"{ORIGIN}v{{version}}/{file}\"\nsha256 = \"{SHA256}\"\n\
         payload_root = \"tool-{{version}}\"\n\n\
         [[installer_options.launchers]]\nid = \"tool\"\nname = \"tool\"\n\
         executable = \"bin/tool\"\nkind = \"command\"\ndefault = true\n",
        arch = std::env::consts::ARCH,
        file = archive("{version}"),
    )
}

/// The version document, upstream-shaped: a bare array of records. The
/// `22` line is LTS, so `tool@lts` decides something here.
fn document() -> String {
    let rows: Vec<String> = VERSIONS
        .iter()
        .rev()
        .map(|version| {
            let lts = if version.starts_with("22.") {
                "\"Jod\""
            } else {
                "false"
            };
            format!("{{\"version\":\"{version}\",\"lts\":{lts}}}")
        })
        .collect();
    format!("[\n{}\n]\n", rows.join(",\n"))
}

struct Fixture {
    registry: AppRegistry,
}

impl Fixture {
    fn new() -> Self {
        let registry = AppRegistry::new();
        let root = registry.root.path();
        let dir = registry.registry().join("org.example/tool/v1.0.0");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), template()).unwrap();

        let upstream = root.join("upstream");
        fs::create_dir_all(&upstream).unwrap();
        fs::write(upstream.join("versions.json"), document()).unwrap();
        fs::write(
            root.join("remap.toml"),
            format!(
                "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
                 reason = \"the suite's own frozen release host\"\n",
                upstream.display()
            ),
        )
        .unwrap();
        fs::create_dir_all(root.join("drop")).unwrap();
        // The project `vibe show config` reads its layers against.
        fs::write(
            root.join("vibe.toml"),
            "[project]\nname = \"probe\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        Fixture { registry }
    }

    fn drop_dir(&self) -> PathBuf {
        self.registry.root.path().join("drop")
    }

    fn remap(&self) -> PathBuf {
        self.registry.root.path().join("remap.toml")
    }

    fn config(&self) -> PathBuf {
        self.registry.settings().join("config.toml")
    }

    fn site_profile(&self) -> PathBuf {
        self.registry.settings().join("site-profile.toml")
    }

    /// Declare the drop, and optionally the policy, in the user's own
    /// `config.toml` — the two tables a person writes by hand.
    fn declare(&self, policy: Option<&str>) {
        let mut body = format!(
            "[artifacts]\npackages = \"{}\"\n",
            self.drop_dir().display()
        );
        if let Some(policy) = policy {
            body.push_str(&format!("\n[versions]\nselect = \"{policy}\"\n"));
        }
        fs::write(self.config(), body).unwrap();
    }

    /// Put the payload of `version` in the drop, at the first rung of the
    /// convention. The bytes are deliberately not an archive: nothing in
    /// these runs may read them.
    fn fill(&self, version: &str) {
        let path = self
            .drop_dir()
            .join("org.example/tool")
            .join(version)
            .join(archive(version));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not an archive, and never read by a plan\n").unwrap();
    }

    /// `vibe install -g <spelling> --registry … --remap … --plan-only`.
    fn plan(&self, spelling: &str) -> Output {
        let remap = self.remap();
        self.registry.app(
            &["--remap", remap.to_str().unwrap(), "install", "-g"],
            spelling,
            &["--plan-only", "--unattended"],
        )
    }

    /// The version the plan resolved, out of the human answer.
    fn planned(&self, spelling: &str) -> (String, String) {
        let output = self.plan(spelling);
        let text = ok(&output);
        let note = String::from_utf8_lossy(&output.stderr).to_string();
        (text, note)
    }

    fn show_config(&self) -> Value {
        json(&self.registry.vibe(&["show", "config", "--json"]))
    }
}

/// The version a plan names, found in its text: the fixture's package has
/// exactly one coordinate per version, so the spelling is unambiguous.
fn resolved_version(text: &str) -> String {
    VERSIONS
        .iter()
        .find(|version| text.contains(&format!("tool@{version}")))
        .unwrap_or_else(|| panic!("no planned version in:\n{text}"))
        .to_string()
}

/// The default, and nvm's own rule: the newest version the requirement
/// matches, although the drop holds only the one below it. This is the
/// behaviour the owner kept — an open network must not change because a
/// closed one needs another rule.
#[test]
fn newest_resolves_the_newest_matching_version_although_the_drop_holds_an_older_one() {
    let fixture = Fixture::new();
    fixture.declare(None);
    fixture.fill("22.23.4");
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.5");
    assert!(
        !note.contains("in drop"),
        "the default says nothing about the drop: {note}"
    );
}

/// `prefer-drop` resolves to the newest version of the requirement whose
/// payload is already here, and says so **once**, on stderr, naming both
/// versions — an operator is never silently moved off the newest.
#[test]
fn prefer_drop_resolves_the_in_drop_version_and_says_so_once() {
    let fixture = Fixture::new();
    fixture.declare(Some("prefer-drop"));
    fixture.fill("22.23.4");
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.4");
    assert!(
        note.contains(
            "org.example/tool@22 → 22.23.4 (in drop; 22.23.5 is newer but not in the drop)"
        ),
        "the one note, verbatim: {note}"
    );
    assert_eq!(note.matches("in drop;").count(), 1, "said once: {note}");
}

/// `drop-only` admits the same version when the drop holds one, and the
/// note is the same: the two policies differ only in what they do with an
/// empty drop.
#[test]
fn drop_only_resolves_the_in_drop_version_too() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    fixture.fill("22.23.4");
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.4");
    assert!(
        note.contains("22.23.5 is newer but not in the drop"),
        "{note}"
    );
}

/// Nothing the requirement admits is in the drop: `drop-only` refuses,
/// naming the version `newest` would have picked and the **exact path**
/// its payload belongs at — which is the whole of what the operator has
/// to do next.
#[test]
fn drop_only_refuses_naming_the_version_and_where_its_payload_belongs() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    fixture.fill("22.23.4");
    let refusal = failed(&fixture.plan("org.example/tool@24"));
    assert!(refusal.contains("drop-only"), "{refusal}");
    assert!(refusal.contains("24.22.3"), "{refusal}");
    let expected = fixture
        .drop_dir()
        .join("org.example/tool/24.22.3")
        .join(archive("24.22.3"));
    assert!(
        refusal.contains(expected.to_str().unwrap()),
        "the first rung of the convention, in full:\nwanted {}\ngot {refusal}",
        expected.display()
    );
    assert!(
        refusal.contains("vibe artifacts where"),
        "the probe that prints every file and digest: {refusal}"
    );
    assert!(
        refusal.contains("prefer-drop"),
        "the other policy: {refusal}"
    );
    assert!(
        refusal.contains("PROP-075#VERSIONS-SELECT"),
        "the record it enforces: {refusal}"
    );
}

/// An **exact** requirement is never redirected by any policy: a person
/// who named one version meant that version, and `drop-only` resolves it
/// as before rather than refusing on policy grounds.
#[test]
fn an_exact_requirement_is_never_redirected() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    fixture.fill("22.23.4");
    let (text, note) = fixture.planned("org.example/tool@=22.23.5");
    assert_eq!(resolved_version(&text), "22.23.5");
    assert!(note.is_empty() || !note.contains("in drop;"), "{note}");
}

/// With no drop configured at all, `prefer-drop` has nothing to prefer
/// and is `newest`: a machine whose drop has not been declared yet still
/// installs, and still gets the ordinary refusal about the missing file
/// rather than a second one about policy.
#[test]
fn prefer_drop_without_a_drop_is_newest() {
    let fixture = Fixture::new();
    fs::write(fixture.config(), "[versions]\nselect = \"prefer-drop\"\n").unwrap();
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.5");
    assert!(!note.contains("in drop;"), "{note}");
}

/// `drop-only` cannot honour "only what the drop holds" when there is no
/// drop, and says so by naming the key to write — never by resolving the
/// newest version the policy exists to refuse.
#[test]
fn drop_only_without_a_drop_refuses_naming_the_key_to_write() {
    let fixture = Fixture::new();
    fs::write(fixture.config(), "[versions]\nselect = \"drop-only\"\n").unwrap();
    let refusal = failed(&fixture.plan("org.example/tool@22"));
    assert!(refusal.contains("declares no drop"), "{refusal}");
    assert!(refusal.contains("[artifacts]"), "{refusal}");
    assert!(refusal.contains("packages = "), "{refusal}");
}

/// A version **installed** here counts as available beside the drop's own
/// contents, because installing it downloads nothing at all. Proved from
/// the other side: with an empty drop and no installation, `drop-only`
/// over the same requirement refuses — so availability is the union of
/// the two grounds and not the drop alone.
#[test]
fn an_empty_drop_with_nothing_installed_leaves_drop_only_nothing_to_pick() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    let refusal = failed(&fixture.plan("org.example/tool@22"));
    assert!(
        refusal.contains("the drop holds none that satisfy"),
        "{refusal}"
    );
    assert!(refusal.contains("22.23.5"), "{refusal}");
}

/// A channel requirement is narrowed **within its own members**: `lts` is
/// the `22` line here, so `prefer-drop` lands on the in-drop member of it
/// rather than on the newest version overall.
#[test]
fn a_channel_requirement_is_narrowed_within_its_members() {
    let fixture = Fixture::new();
    fixture.declare(Some("prefer-drop"));
    fixture.fill("22.23.4");
    let (text, note) = fixture.planned("org.example/tool@lts");
    assert_eq!(resolved_version(&text), "22.23.4");
    assert!(
        note.contains("22.23.5 is newer but not in the drop"),
        "{note}"
    );
}

/// The policy a **site profile** declares is in force for the whole
/// installation: `install -g tool@22` picks the in-drop version although
/// the user's own `config.toml` says nothing about selection. This is the
/// corporate shape — one profile, every developer.
#[test]
fn a_site_profile_policy_decides_the_installation() {
    let fixture = Fixture::new();
    fixture.declare(None);
    fixture.fill("22.23.4");
    fs::write(
        fixture.site_profile(),
        "schema = 1\nname = \"corp\"\n\n[versions]\nselect = \"prefer-drop\"\n",
    )
    .unwrap();
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.4");
    assert!(
        note.contains("22.23.5 is newer but not in the drop"),
        "{note}"
    );
    // And the installation says where the value came from.
    let shown = fixture.show_config();
    assert_eq!(shown["versions"]["select"], "prefer-drop");
    assert_eq!(shown["versions"]["provenance"], "site profile");
}

/// The user's own file still wins over the site's: a developer who writes
/// `newest` gets `newest`, which is the layering rule every other
/// user-config value follows.
#[test]
fn the_users_own_setting_overrides_the_site_profiles() {
    let fixture = Fixture::new();
    fixture.declare(Some("newest"));
    fixture.fill("22.23.4");
    fs::write(
        fixture.site_profile(),
        "schema = 1\nname = \"corp\"\n\n[versions]\nselect = \"drop-only\"\n",
    )
    .unwrap();
    let (text, note) = fixture.planned("org.example/tool@22");
    assert_eq!(resolved_version(&text), "22.23.5");
    assert!(!note.contains("in drop;"), "{note}");
    let shown = fixture.show_config();
    assert_eq!(shown["versions"]["select"], "newest");
    assert_eq!(shown["versions"]["provenance"], "config.toml");
}

/// The whole ladder, in one place: the built-in default, the site
/// profile, the user's own file, and the environment over all of them.
#[test]
fn the_effective_policy_names_the_layer_that_decided_it() {
    let fixture = Fixture::new();
    let shown = fixture.show_config();
    assert_eq!(shown["versions"]["select"], "newest");
    assert_eq!(shown["versions"]["provenance"], "default");

    fs::write(
        fixture.site_profile(),
        "schema = 1\n\n[versions]\nselect = \"prefer-drop\"\n",
    )
    .unwrap();
    let shown = fixture.show_config();
    assert_eq!(shown["versions"]["select"], "prefer-drop");
    assert_eq!(shown["versions"]["provenance"], "site profile");

    fixture.declare(Some("drop-only"));
    let shown = fixture.show_config();
    assert_eq!(shown["versions"]["select"], "drop-only");
    assert_eq!(shown["versions"]["provenance"], "config.toml");

    let shown = json(&{
        let mut command = fixture.registry.user.vibe();
        command
            .current_dir(fixture.registry.root.path())
            .env("HOME", fixture.registry.home())
            .env("VIBE_VERSIONS_SELECT", "newest")
            .args(["show", "config", "--json"]);
        command.output().unwrap()
    });
    assert_eq!(shown["versions"]["select"], "newest");
    assert_eq!(shown["versions"]["provenance"], "env");
}

/// A misspelled policy refuses rather than falling through to `newest`: a
/// value that silently restored the default is the one failure a closed
/// network cannot afford to debug.
#[test]
fn a_misspelled_policy_refuses_by_naming_the_three_spellings() {
    let fixture = Fixture::new();
    let output = {
        let mut command = fixture.registry.user.vibe();
        command
            .current_dir(fixture.registry.root.path())
            .env("HOME", fixture.registry.home())
            .env("VIBE_VERSIONS_SELECT", "oldest")
            .args(["show", "config"]);
        command.output().unwrap()
    };
    let refusal = failed(&output);
    assert!(refusal.contains("VIBE_VERSIONS_SELECT=oldest"), "{refusal}");
    assert!(
        refusal.contains("newest, prefer-drop, drop-only"),
        "{refusal}"
    );
}

/// `vibe app versions` says the policy in its trailer, and under
/// `drop-only` says which rows are selectable — but it does **not**
/// filter the rows: hiding the unselectable ones would hide the version
/// an operator has to carry in.
#[test]
fn the_listing_says_the_policy_and_still_shows_every_row() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    fixture.fill("22.23.4");
    let remap = fixture.remap();
    let listing = ok(&fixture.registry.app(
        &["app", "versions"],
        "org.example/tool",
        &["--remap", remap.to_str().unwrap()],
    ));
    assert!(
        listing.contains("versions select drop-only"),
        "the trailer: {listing}"
    );
    assert!(
        listing.contains("only an `in drop` or installed row is selectable"),
        "{listing}"
    );
    for version in VERSIONS {
        assert!(
            listing.contains(version),
            "row {version} missing:\n{listing}"
        );
    }
}

/// `vibe artifacts where` is exempt by construction: under `drop-only` it
/// still answers for the newest version, because the payload an operator
/// is about to carry in is exactly the one the drop does not hold.
#[test]
fn artifacts_where_still_answers_for_the_newest_version_under_drop_only() {
    let fixture = Fixture::new();
    fixture.declare(Some("drop-only"));
    fixture.fill("22.23.4");
    let remap = fixture.remap();
    let registry = fixture.registry.registry();
    let shown = ok(&fixture.registry.vibe(&[
        "--remap",
        remap.to_str().unwrap(),
        "artifacts",
        "where",
        "org.example/tool@24",
        "--registry",
        registry.to_str().unwrap(),
    ]));
    assert!(
        shown.contains(&archive("24.22.3")),
        "the file of the version the drop lacks: {shown}"
    );
}
