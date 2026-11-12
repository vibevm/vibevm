//! Registry-ISSUED short names, end to end (PROP-008 §2.6).
//!
//! The site is a closed one: a single `type = "localdir"` registry under
//! a `[source_policy]` that excludes the public hostings by name. **No
//! run here opens a socket.** The registry carries
//! `org.vibevm.qwen/qwen-code`, and the question each test asks is what
//! the bare spelling `qwen` reaches.
//!
//! Since the owner's ruling of 2026-11-12 the answer is: whatever the
//! registry ISSUED, and nothing else. A package called `qwen-code` is
//! not reachable as `qwen-code` until a row says so — «мы не можем дать
//! людям самим выбирать любое имя».
#![cfg(all(test, unix))]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Site {
    root: tempfile::TempDir,
}

impl Site {
    fn new() -> Self {
        let site = Site {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(site.path("empty"), "").unwrap();
        for directory in ["home", "settings", "cache"] {
            fs::create_dir_all(site.path(directory)).unwrap();
        }
        site.publish("org.vibevm.qwen", "qwen-code");
        fs::write(
            site.path("settings/registry.toml"),
            format!(
                "[[registry]]\nname = 'corp'\ntype = 'localdir'\n\
                 url = 'file://{}'\n\
                 [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
                site.path("reg").display()
            ),
        )
        .unwrap();
        site
    }

    /// Put one minimal application package into the registry tree. A
    /// directory filled by hand is as real as a published one, which is
    /// what a layout registry's projection says.
    fn publish(&self, group: &str, name: &str) {
        let dir = self.path("reg").join(format!("{group}/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), manifest(group, name)).unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn vibe(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(self.root.path())
            .args(args)
            .env("HOME", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_REGISTRY_CACHE", self.path("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("CI", "1")
            .env("NO_COLOR", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .env_remove("GIT_SSH")
            .env_remove("GIT_SSH_COMMAND")
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("VIBE_APP_USE")
            .env_remove("VIBEVM_INSTALL_ROOT")
            .output()
            .unwrap()
    }

    /// `vibe registry alias corp …`.
    fn alias(&self, args: &[&str]) -> Output {
        self.alias_in("corp", args)
    }

    /// `vibe registry alias <registry> …`.
    fn alias_in(&self, registry: &str, args: &[&str]) -> Output {
        let mut all = vec!["registry", "alias", registry];
        all.extend_from_slice(args);
        self.vibe(&all)
    }

    /// Declare a SECOND closed localdir registry, `other`, after `corp`
    /// — the two-registry site the §2.7 collision needs. Both are
    /// `file://`, so still no socket is opened.
    fn with_second_registry(&self) -> &Self {
        fs::create_dir_all(self.path("reg2")).unwrap();
        fs::write(
            self.path("settings/registry.toml"),
            format!(
                "[[registry]]\nname = 'corp'\ntype = 'localdir'\n\
                 url = 'file://{}'\n\
                 [[registry]]\nname = 'other'\ntype = 'localdir'\n\
                 url = 'file://{}'\n\
                 [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
                self.path("reg").display(),
                self.path("reg2").display(),
            ),
        )
        .unwrap();
        self
    }

    /// Put one minimal application package into the second registry.
    fn publish_other(&self, group: &str, name: &str) {
        let dir = self.path("reg2").join(format!("{group}/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), manifest(group, name)).unwrap();
    }
}

fn manifest(group: &str, name: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\ngroup = \"{group}\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
         [application]\nid = \"{name}\"\nportable = false\n\n\
         [[installer_options]]\nid = \"main\"\nname = \"Main\"\n\
         platform = \"{os}\"\nplatform_name = \"{os}\"\narch = \"{arch}\"\n\
         [installer_options.installation]\ntype = \"shell\"\napp_root = \"/opt/{name}\"\n\
         install = \"true\"\nforce_install = \"true\"\n\
         uninstall = \"true\"\nforce_uninstall = \"true\"\ninterpreter = \"posix_sh\"\n",
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed ({})\nstdout: {}\nstderr: {}",
        code(output),
        stdout(output),
        stderr(output),
    );
}

/// The whole feature in one run: the owner offers the spelling, and the
/// application boundary reaches the package through it.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn an_offered_spelling_reaches_the_package_it_names() {
    let site = Site::new();
    let added = site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]);
    ok(&added, "vibe registry alias corp add");

    // The authored table is the registry owner's, and it says so.
    let table = fs::read_to_string(site.path("reg/_index/aliases.toml")).unwrap();
    assert!(
        table.contains("qwen = \"org.vibevm.qwen/qwen-code\""),
        "the table must carry the owner's own row shape:\n{table}"
    );

    let resolved = site.vibe(&["app", "installers", "qwen"]);
    ok(&resolved, "vibe app installers qwen");
    let said = stderr(&resolved);
    assert!(
        said.contains("qwen → org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "the provenance line must name the registry that decided: {said}"
    );
}

/// The compatibility shape, asserted on the bytes a client fetches: an
/// older vibe reading `by-name/qwen.json` sees a name nobody publishes,
/// never a package called `qwen`.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn the_candidate_set_an_older_client_reads_is_empty() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let bytes = fs::read(site.path("reg/_index/by-name/qwen.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        parsed["packages"].as_array().map(Vec::len),
        Some(0),
        "an alias must never ride in `packages[]`: {parsed}"
    );
    assert_eq!(parsed["alias"]["group"], "org.vibevm.qwen");
    assert_eq!(parsed["alias"]["name"], "qwen-code");
}

/// The inverted law, end to end. A package's own name reaches nothing
/// until the registry issues it — and the refusal hands the operator the
/// qualified name to copy instead of guessing on their behalf.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn a_package_name_no_registry_issued_is_refused_with_the_qualified_name_as_a_hint() {
    let site = Site::new();
    // No alias table at all: `qwen-code` is published under that very
    // name, and that grants it nothing.
    let refused = site.vibe(&["app", "installers", "qwen-code"]);
    assert!(
        !refused.status.success(),
        "a package name is not a short name: {}",
        stdout(&refused)
    );
    let said = stderr(&refused);
    assert!(
        said.contains("the short name `qwen-code` is not issued by any consulted registry"),
        "{said}"
    );
    assert!(said.contains("give the full name"), "{said}");
    assert!(said.contains("consulted: corp"), "{said}");
    assert!(
        said.contains("none of them chosen for you"),
        "the hint must not read as a choice: {said}"
    );
    assert!(
        said.contains("org.vibevm.qwen/qwen-code"),
        "the qualified name must be copyable out of the refusal: {said}"
    );

    // The qualified form it names does work.
    ok(
        &site.vibe(&["app", "installers", "org.vibevm.qwen/qwen-code"]),
        "the qualified form resolves",
    );
}

/// And the remedy: the owner ISSUES the package's own name, which the
/// old rule refused outright. `node → org.vibevm.node/node` is this
/// shape, and it is now the common one.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn an_issued_name_equal_to_the_package_name_resolves() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen-code", "org.vibevm.qwen/qwen-code"]),
        "a registry may issue a package's own name",
    );
    let resolved = site.vibe(&["app", "installers", "qwen-code"]);
    ok(&resolved, "vibe app installers qwen-code");
    assert!(
        stderr(&resolved)
            .contains("qwen-code → org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "the provenance line still names who issued it: {}",
        stderr(&resolved)
    );

    // The candidate file carries both halves: the package that bears
    // the name AND the row that issues it.
    let bytes = fs::read(site.path("reg/_index/by-name/qwen-code.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        parsed["packages"].as_array().map(Vec::len),
        Some(1),
        "the real package stays in `packages[]`: {parsed}"
    );
    assert_eq!(parsed["alias"]["name"], "qwen-code");
}

/// A registry the effective source policy excludes is not asked what it
/// issues, exactly as it is not asked what it carries.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn an_excluded_registrys_table_is_never_consulted() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    // Take `corp` itself out of the effective policy, by the hosting
    // selector its `file://` URL answers to.
    fs::write(
        site.path("settings/registry.toml"),
        format!(
            "[[registry]]\nname = 'corp'\ntype = 'localdir'\n\
             url = 'file://{}'\n\
             [source_policy]\nexclude_hosting = ['github', 'gitverse', 'local']\n",
            site.path("reg").display()
        ),
    )
    .unwrap();
    let refused = site.vibe(&["app", "installers", "qwen"]);
    assert!(
        !refused.status.success(),
        "an excluded registry issues nothing here: {}",
        stdout(&refused)
    );
    assert!(
        !stderr(&refused).contains("alias in registry corp"),
        "the excluded registry must not have been asked: {}",
        stderr(&refused)
    );
}

/// `--offline` reads the cached index alone, and the authored table of a
/// local-directory registry is on this disk, so an issued spelling keeps
/// resolving with the network forbidden.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn an_issued_name_resolves_offline() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let resolved = site.vibe(&["app", "installers", "--offline", "qwen"]);
    ok(&resolved, "vibe app installers --offline qwen");
    assert!(
        stderr(&resolved).contains("qwen → org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "{}",
        stderr(&resolved)
    );

    // And a spelling nobody issued says what `--offline` limited.
    let refused = site.vibe(&["app", "installers", "--offline", "qwen-code"]);
    assert!(!refused.status.success(), "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("not issued by any consulted registry"),
        "{}",
        stderr(&refused)
    );
}

/// `vibe search` stays a search: it finds a package by the name it
/// really has, and says which of those names the registry has issued as
/// a short one — because that is now the difference between a row an
/// operator can type and one they cannot.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn search_finds_by_name_and_marks_the_issued_ones() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let found = site.vibe(&["search", "--json", "qwen"]);
    ok(&found, "vibe search --json qwen");
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&found)).unwrap();
    let hits = parsed["hits"].as_array().expect("hits is an array");
    let hit = hits
        .iter()
        .find(|hit| hit["name"] == "qwen-code")
        .unwrap_or_else(|| panic!("search must still find a package by its own name: {parsed}"));
    assert_eq!(
        hit["short_names"].as_array().map(|names| names
            .iter()
            .map(|name| name.as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()),
        Some(vec!["qwen".to_string()]),
        "the issued spelling must be marked on the hit: {parsed}"
    );
}

/// The three authoring refusals a maintainer meets.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn a_row_that_could_not_resolve_is_refused_at_authoring_time() {
    let site = Site::new();

    let absent = site.alias(&["add", "qwen", "org.vibevm.qwen/not-here"]);
    assert!(!absent.status.success());
    assert!(
        stderr(&absent).contains("points INTO the registry"),
        "{}",
        stderr(&absent)
    );

    let bare = site.alias(&["add", "qwen", "qwen-code"]);
    assert!(!bare.status.success());
    assert!(
        stderr(&bare).contains("carries no group"),
        "{}",
        stderr(&bare)
    );

    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let repointed = site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code2"]);
    assert!(!repointed.status.success());
    assert!(
        stderr(&repointed).contains("already means"),
        "{}",
        stderr(&repointed)
    );

    // Nothing was written by any of the three.
    let table = fs::read_to_string(site.path("reg/_index/aliases.toml")).unwrap();
    assert_eq!(table.matches("qwen = ").count(), 1, "{table}");
}

/// Withdrawing the last row leaves the tree of a registry that never
/// offered a spelling — which is what keeps the feature free for every
/// registry that does not use it.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn withdrawing_the_last_row_leaves_no_trace_in_the_catalog() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    assert!(site.path("reg/_index/by-name/qwen.json").exists());

    ok(&site.alias(&["remove", "qwen"]), "alias remove");
    assert!(
        !site.path("reg/_index/by-name/qwen.json").exists(),
        "the alias's candidate-set file must go with its row"
    );
    let listing = fs::read_to_string(site.path("reg/_index/listing.txt")).unwrap();
    assert!(
        !listing.contains("# aliases "),
        "a registry with no rows must ask nothing of an older client:\n{listing}"
    );

    // And the name is unresolvable again.
    let gone = site.vibe(&["app", "installers", "qwen"]);
    assert!(!gone.status.success(), "{}", stdout(&gone));
}

/// `list` is the table, and it needs no write access to answer.
#[test]
fn list_prints_the_table() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let listed = site.alias(&["list"]);
    ok(&listed, "vibe registry alias corp list");
    assert!(
        stdout(&listed).contains("qwen -> org.vibevm.qwen/qwen-code"),
        "{}",
        stdout(&listed)
    );
}

/// Two registries issuing ONE spelling for DIFFERENT packages is the
/// §2.7 collision, with its own exit code `7` — the resolver does not
/// guess which holder meant what, and names both with the registry that
/// issued each.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn two_registries_issuing_one_spelling_for_two_packages_is_exit_seven() {
    let site = Site::new();
    site.with_second_registry();
    site.publish_other("org.vibevm.qwen", "qwen-fork");
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "corp issues qwen",
    );
    ok(
        &site.alias_in("other", &["add", "qwen", "org.vibevm.qwen/qwen-fork"]),
        "other issues qwen differently",
    );

    let refused = site.vibe(&["app", "installers", "qwen"]);
    assert_eq!(
        code(&refused),
        7,
        "a collision carries exit 7\nstdout: {}\nstderr: {}",
        stdout(&refused),
        stderr(&refused)
    );
    let said = stderr(&refused);
    assert!(said.contains("`qwen` is ambiguous"), "{said}");
    assert!(
        said.contains("org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "{said}"
    );
    assert!(
        said.contains("org.vibevm.qwen/qwen-fork (alias in registry other)"),
        "{said}"
    );
}

/// And two registries issuing the SAME spelling for the SAME package is
/// one candidate, not a collision — a mirror agreeing with its source
/// is the common case and must not be refused.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn two_registries_agreeing_on_one_spelling_are_one_candidate() {
    let site = Site::new();
    site.with_second_registry();
    site.publish_other("org.vibevm.qwen", "qwen-code");
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "corp issues qwen",
    );
    ok(
        &site.alias_in("other", &["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "other issues the same",
    );

    let resolved = site.vibe(&["app", "installers", "qwen"]);
    ok(&resolved, "agreement is not a collision");
    assert!(
        stderr(&resolved)
            .contains("qwen → org.vibevm.qwen/qwen-code (alias in registries corp, other)"),
        "the provenance line names both holders: {}",
        stderr(&resolved)
    );
}

/// The project install path obeys the same law: `vibe install <bare>`
/// resolves an issued spelling and refuses a package's own name, with
/// the qualified name offered to copy.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn the_project_install_path_resolves_only_issued_names() {
    let site = Site::new();
    fs::create_dir_all(site.path("proj")).unwrap();
    fs::write(
        site.path("proj/vibe.toml"),
        "[package]\nname = \"consumer\"\ngroup = \"com.example\"\n\
         kind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let install = |spelling: &str| {
        site.vibe(&[
            "install",
            "--path",
            site.path("proj").to_str().unwrap(),
            "--plan-only",
            spelling,
        ])
    };

    // The package's own name is not a short name.
    let refused = install("qwen-code");
    assert!(
        !refused.status.success(),
        "a package name resolves nothing: {}",
        stdout(&refused)
    );
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(
        said.contains("not issued by any consulted registry"),
        "{said}"
    );
    assert!(
        said.contains("org.vibevm.qwen/qwen-code"),
        "the refusal hands over the qualified name: {said}"
    );

    // Issued, and the bare spelling now resolves through it. The
    // install itself may still fail further down (an application
    // package is not a project requirement), so this asserts on the
    // provenance line rather than on the exit code.
    ok(
        &site.alias(&["add", "qc", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let attempted = install("qc");
    let said = format!("{}{}", stdout(&attempted), stderr(&attempted));
    assert!(
        said.contains("qc → org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "the install boundary must resolve the issued spelling: {said}"
    );
}
