//! A bare `<name>` at the application command boundary (PROP-008
//! `#APP-COMMANDS-SHORT-NAMES`, `#SHORT-NAME-TRUST`, PROP-059
//! `#commands`).
//!
//! The site here is the shape a closed corporate network has and the shape
//! short-name resolution most needs to work in: one `type = "monorepo"`
//! registry — a mirrored single repository, read over Git's `file`
//! transport — under a `[source_policy]` that excludes the public
//! hostings by name. **No run here opens a socket.**
#![cfg(all(test, unix))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A home whose only registry is a local monorepo, under a source policy
/// that excludes the public hostings. The monorepo carries four
/// application packages:
///
/// | package | group | why |
/// |---|---|---|
/// | `org.vibevm.node/widget` | trusted | the single candidate |
/// | `org.vibevm.node/node` | trusted | one half of the collision |
/// | `org.vibevm.alt/node` | trusted | the other half |
/// | `com.acme/gadget` | untrusted by default | never picked for you |
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

        let work = site.path("work");
        for (group, name) in [
            ("org.vibevm.node", "widget"),
            ("org.vibevm.node", "node"),
            ("org.vibevm.alt", "node"),
            ("com.acme", "gadget"),
        ] {
            let package = work.join(format!("{group}/{name}/v1.0.0"));
            fs::create_dir_all(&package).unwrap();
            fs::write(package.join("vibe.toml"), manifest(group, name)).unwrap();
        }
        site.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        site.git(Some(&work), &["add", "-A"]);
        site.git(Some(&work), &["commit", "-q", "-m", "four app packages"]);
        site.git(
            None,
            &[
                "clone",
                "-q",
                "--bare",
                work.to_str().unwrap(),
                site.path("mono.git").to_str().unwrap(),
            ],
        );

        // The corporate profile's own shape: one site mirror, and the
        // public hostings excluded by name. Nothing here can reach a
        // network — the mirror is on disk and no public row is declared.
        fs::write(
            site.path("settings/registry.toml"),
            format!(
                "[[registry]]\nname = 'corp'\ntype = 'monorepo'\n\
                 url = 'file://{}'\nref = 'main'\n\
                 [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
                site.path("mono.git").display()
            ),
        )
        .unwrap();
        site
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    /// Add `groups` to `trusted_groups` of this home's `agents.toml`.
    fn trust(&self, groups: &[&str]) {
        let listed = groups
            .iter()
            .map(|group| format!("\"{group}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            self.path("settings/agents.toml"),
            format!("trusted_groups = [{listed}]\n"),
        )
        .unwrap();
    }

    fn git(&self, cwd: Option<&Path>, args: &[&str]) {
        let mut command = Command::new("git");
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let output = command
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@example.invalid",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
}

/// One minimal app package: an external shell installation, so nothing is
/// fetched and nothing runs for the read-only verbs these tests use.
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

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed ({})\nstdout: {}\nstderr: {}",
        code(output),
        String::from_utf8_lossy(&output.stdout),
        stderr(output),
    );
}

/// One eligible candidate: the command resolves it and says so, once.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn one_candidate_resolves_and_the_provenance_line_says_which() {
    let site = Site::new();
    let listed = site.vibe(&["app", "installers", "widget"]);
    ok(&listed, "vibe app installers widget");
    let said = stderr(&listed);
    assert!(
        said.contains("widget → org.vibevm.node/widget"),
        "the provenance line is missing: {said}"
    );
    assert_eq!(
        said.matches("widget → org.vibevm.node/widget").count(),
        1,
        "the provenance line is printed once: {said}"
    );
}

/// `--quiet` and `--json` keep it: it is one line, it is on stderr, and it
/// says what vibe decided rather than what the command answered.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn the_provenance_line_survives_quiet_and_json() {
    let site = Site::new();
    for extra in [vec!["--quiet"], vec!["--json"]] {
        let mut args = vec!["app", "installers", "widget"];
        args.extend_from_slice(&extra);
        let listed = site.vibe(&args);
        ok(&listed, "vibe app installers widget");
        assert!(
            stderr(&listed).contains("widget → org.vibevm.node/widget"),
            "{extra:?} dropped the provenance line: {}",
            stderr(&listed)
        );
        // `--json`'s stdout stays exactly one envelope.
        if extra == ["--json"] {
            serde_json::from_slice::<serde_json::Value>(&listed.stdout)
                .expect("one JSON envelope on stdout");
        }
    }
}

/// A qualified spelling is not a short name: it passes through untouched
/// and nothing is announced.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn a_qualified_spelling_announces_nothing() {
    let site = Site::new();
    let listed = site.vibe(&["app", "installers", "org.vibevm.node/widget"]);
    ok(&listed, "vibe app installers org.vibevm.node/widget");
    assert!(
        !stderr(&listed).contains('→'),
        "nothing was decided, so nothing is announced: {}",
        stderr(&listed)
    );
}

/// Two trusted groups publishing one bare name is the PROP-008 §2.7
/// collision: the qualified spellings, and exit 7.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn two_groups_under_one_name_refuse_with_exit_seven() {
    let site = Site::new();
    let refused = site.vibe(&["app", "installers", "node"]);
    assert_eq!(code(&refused), 7, "stderr: {}", stderr(&refused));
    let said = stderr(&refused);
    assert!(said.contains("`node` is ambiguous"), "{said}");
    assert!(said.contains("org.vibevm.alt/node"), "{said}");
    assert!(said.contains("org.vibevm.node/node"), "{said}");
    assert!(
        said.contains("vibe app installers org.vibevm.alt/node"),
        "the hint re-runs the command that was typed: {said}"
    );
}

/// A candidate whose group nothing trusts is never picked, and the refusal
/// names it as such with the list to edit.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#SHORT-NAME-TRUST"
)]
fn an_untrusted_group_is_listed_and_never_picked() {
    let site = Site::new();
    let refused = site.vibe(&["app", "installers", "gadget"]);
    assert!(!refused.status.success(), "{}", stderr(&refused));
    let said = stderr(&refused);
    assert!(
        said.contains("their group is untrusted: com.acme/gadget"),
        "{said}"
    );
    assert!(said.contains("trusted_groups"), "{said}");

    // Trust it, and the same command resolves it.
    site.trust(&["com.acme"]);
    let listed = site.vibe(&["app", "installers", "gadget"]);
    ok(&listed, "vibe app installers gadget, com.acme trusted");
    assert!(
        stderr(&listed).contains("gadget → com.acme/gadget"),
        "{}",
        stderr(&listed)
    );
}

/// The refusal names what was consulted and what the effective source
/// policy excluded, so a site whose mirror is configured but unused can
/// see it in the failure itself.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn the_refusal_names_the_consulted_and_the_excluded_registries() {
    let site = Site::new();
    let refused = site.vibe(&["app", "installers", "absent"]);
    assert!(!refused.status.success(), "{}", stderr(&refused));
    let said = stderr(&refused);
    assert!(said.contains("consulted: corp"), "{said}");
    assert!(
        !said.contains("github") || said.contains("source policy: github"),
        "an excluded hosting appears only as excluded, never as walked: {said}"
    );
    assert!(
        said.contains("excluded by the effective source policy: github, gitverse"),
        "{said}"
    );
    assert!(
        said.contains("vibe app installers <group>/absent"),
        "the qualified form that always works: {said}"
    );
}

/// Offline, **with** the index on disk: the site mirror is a local
/// repository, so a closed network resolves the short name with no
/// request at all.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn offline_resolves_from_the_cached_index() {
    let site = Site::new();
    let listed = site.vibe(&["app", "installers", "widget", "--offline"]);
    ok(&listed, "vibe app installers widget --offline");
    assert!(
        stderr(&listed).contains("widget → org.vibevm.node/widget"),
        "{}",
        stderr(&listed)
    );
}

/// Offline, with **no** candidate the cached index can answer with: the
/// refusal says the cache was the whole source and asks for the qualified
/// form, rather than pretending a lookup was performed.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#APP-COMMANDS-SHORT-NAMES"
)]
fn offline_without_a_cached_candidate_asks_for_the_qualified_form() {
    let site = Site::new();
    let refused = site.vibe(&["app", "installers", "absent", "--offline"]);
    assert!(!refused.status.success(), "{}", stderr(&refused));
    let said = stderr(&refused);
    assert!(
        said.contains("`--offline` reads the cached index alone"),
        "{said}"
    );
    assert!(said.contains("Give the qualified form instead"), "{said}");
}

/// A verb that acts on installed state answers from this home's inventory
/// alone: a name it does not carry is *not installed*, a local failure
/// with a local remedy, and the message never reaches for a registry.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#INSTALLED-STATE-RESOLVES-LOCALLY"
)]
fn an_installed_state_verb_refuses_locally() {
    let site = Site::new();
    let refused = site.vibe(&["app", "hold", "widget"]);
    assert!(!refused.status.success(), "{}", stderr(&refused));
    let said = stderr(&refused);
    assert!(said.contains("`widget` is not installed"), "{said}");
    assert!(said.contains("application index"), "{said}");
    assert!(said.contains("vibe app hold <group>/widget"), "{said}");
    assert!(
        !said.contains("consulted"),
        "installed state consults no registry: {said}"
    );
}

/// `vibe app versions` keeps its first two tiers inside the library — a
/// slot id, then an installed package — and gains the third here.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#SHORT-NAME-SLOT-PRECEDENCE"
)]
fn versions_reaches_the_index_only_for_a_word_that_is_neither() {
    let site = Site::new();
    let listed = site.vibe(&["app", "versions", "widget", "--json"]);
    ok(&listed, "vibe app versions widget --json");
    assert!(
        stderr(&listed).contains("widget → org.vibevm.node/widget"),
        "{}",
        stderr(&listed)
    );
    let document: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(
        document["packages"][0]["package"], "org.vibevm.node/widget",
        "{document}"
    );
}

/// …and when the third tier finds nothing at all, `vibe app versions`
/// keeps its own refusal, the one that names the slots this home records.
/// A collision is still the boundary's, because the command's refusal
/// cannot know there were two.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#SHORT-NAME-SLOT-PRECEDENCE"
)]
fn versions_keeps_its_own_refusal_for_a_word_no_registry_carries() {
    let site = Site::new();
    let refused = site.vibe(&["app", "versions", "absent", "--json"]);
    assert!(!refused.status.success(), "{}", stderr(&refused));
    let said = stderr(&refused);
    assert!(said.contains("unknown_target"), "{said}");
    assert!(
        !said.contains("could not resolve the short name"),
        "the boundary stands aside for a word nothing carries: {said}"
    );

    let collision = site.vibe(&["app", "versions", "node", "--json"]);
    assert_eq!(code(&collision), 7, "stderr: {}", stderr(&collision));
    assert!(
        stderr(&collision).contains("`node` is ambiguous"),
        "{}",
        stderr(&collision)
    );
}
