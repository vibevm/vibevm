//! Registry-owned short names, end to end (PROP-008 §2.6).
//!
//! The site is a closed one: a single `type = "localdir"` registry under
//! a `[source_policy]` that excludes the public hostings by name. **No
//! run here opens a socket.** The registry carries
//! `org.vibevm.qwen/qwen-code`, and the question each test asks is what
//! the bare spelling `qwen` reaches.
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
        let mut all = vec!["registry", "alias", "corp"];
        all.extend_from_slice(args);
        self.vibe(&all)
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

/// A real package of that name outranks the alias — and a registry may
/// not even offer a spelling it publishes itself.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn a_published_name_outranks_every_offered_spelling() {
    let site = Site::new();
    ok(
        &site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    // Somebody publishes a package actually called `qwen`.
    site.publish("com.acme", "qwen");
    let listed = site.vibe(&["app", "installers", "com.acme/qwen"]);
    ok(&listed, "the real package resolves qualified");

    // And the owner is told on their next edit that the row went bad,
    // rather than finding out from a short name that stopped working.
    site.publish("org.vibevm.qwen", "qwen-tools");
    let refused = site.alias(&["add", "qt", "org.vibevm.qwen/qwen-tools"]);
    assert!(!refused.status.success(), "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("`qwen` is already the name"),
        "the refusal must name the row that went bad: {}",
        stderr(&refused)
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
