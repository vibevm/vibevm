//! A registry working copy is **data**, not a user's checkout: it holds the
//! exact bytes its blobs record, whatever the machine's Git says about line
//! endings (`vibe_registry::git_backend::verbatim`, PROP-073
//! `#NATIVE-MATERIALISATION`).
//!
//! The shape here is the corporate one — one mirrored monorepo registry
//! carrying a template snapshot in its `_index/`, with the public hostings
//! excluded by name, which is also what puts the operation on the controlled
//! transport and so stages the registry's tree through native materialisation.
//! The client's Git converts line endings: `core.autocrlf=true`, which the Git
//! for Windows installer writes into the machine's *system* configuration, so
//! it is the stock Windows default rather than an exotic setting. A
//! `.gitattributes` saying `* text=auto eol=crlf` in the registry itself is the
//! second lever, and the one `core.autocrlf=false` alone does not answer.
//!
//! What went wrong without this: the snapshot's digest documents were staged
//! with `\r\n`, so they no longer hashed to the `digest_sha256` their rows
//! record; `Snapshot::prime` warned once per version and primed nothing, and
//! the closed network — whose only source of those documents is the snapshot —
//! could render no template-only version at all. The second run of each test
//! is `--offline` with no remap row, so the snapshot is the only thing that can
//! answer it.
//!
//! The repository is local and read over Git's `file` transport and the
//! template's version list is one `[[remap]]` row onto a frozen copy of
//! upstream's index, so **no run here opens a socket**.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use common::template_mirror::{Fixture, PACKAGE, mirror, ok};

/// Snapshot the recent versions, so the client has digest documents to prime
/// and `app versions` has a template to answer from.
const SELECT: &str = r#"schema = 1
[[template]]
package = "org.vibevm.node/node"
versions = [">=22"]
"#;

/// The warning `Snapshot::prime` raises when a digest document's bytes do not
/// hash to the row that describes them — the symptom this suite exists for.
const NOT_PRIMED: &str = "does not hash";

/// A mirrored monorepo registry plus a client whose Git converts line endings.
struct Site {
    user: UserScratch,
    fixture: Fixture,
    root: tempfile::TempDir,
}

impl Site {
    /// `attributes` is written into the registry repository as its
    /// `.gitattributes`, so one of the two tests drives the conversion the
    /// *payload* asks for rather than the one the configuration asks for.
    fn new(attributes: Option<&str>) -> Self {
        let site = Site {
            user: UserScratch::new(),
            fixture: Fixture::new(),
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(site.path("empty"), "").unwrap();
        fs::create_dir_all(site.path("home")).unwrap();
        fs::create_dir_all(site.path("settings")).unwrap();

        // The mirror writes the snapshot into the target registry tree: real
        // bytes, real `digest_sha256` rows, written by the command that owns
        // the layout.
        let config = site.fixture.config(SELECT);
        ok(&mirror(&site.user, &site.fixture, &config, &[]));

        // That tree, committed as the one repository a closed network has.
        let work = site.fixture.target();
        if let Some(attributes) = attributes {
            fs::write(work.join(".gitattributes"), attributes).unwrap();
        }
        site.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        // Recorded under `core.autocrlf=false` so the blobs hold LF whatever
        // the developer running the suite has configured: the question here is
        // what a checkout writes, never what a commit recorded.
        site.git(Some(&work), &["-c", "core.autocrlf=false", "add", "-A"]);
        site.git(
            Some(&work),
            &["commit", "-q", "-m", "the mirrored registry"],
        );
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

        // The client's own Git configuration: the Git for Windows default,
        // written where a user's global configuration lives.
        fs::write(
            site.path("gitconfig"),
            "[core]\n\tautocrlf = true\n\tsafecrlf = true\n",
        )
        .unwrap();
        site
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    /// Fixture git, with the developer's own configuration kept out.
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

    /// `vibe app versions <PACKAGE>` against the configured monorepo, under a
    /// Git whose `core.autocrlf` is on.
    fn versions(&self, extra: &[&str]) -> Output {
        let remap = self.fixture.root.path().join("remap.toml");
        let mut command = self.user.vibe();
        command
            .current_dir(self.root.path())
            .args(["app", "versions", PACKAGE, "--all", "--json"])
            .args(extra)
            .env("HOME", self.path("home"))
            .env(vibe_core::settings::SETTINGS_DIR_ENV, self.path("settings"))
            .env("CI", "1")
            .env("NO_COLOR", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("gitconfig"))
            .env_remove("VIBE_APP_USE");
        if !extra.contains(&"--offline") {
            command.args(["--remap", remap.to_str().unwrap()]);
        }
        command.output().unwrap()
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// The template rows of a `--json` listing, which only a primed snapshot (or a
/// network this suite does not have) can produce.
fn template_rows(output: &Output) -> Vec<String> {
    let listing: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let package = &listing["packages"][0];
    assert_eq!(package["package"], PACKAGE);
    package["versions"]
        .as_array()
        .expect("the rows")
        .iter()
        .filter(|row| row["origin"] == "template")
        .map(|row| row["version"].as_str().unwrap().to_string())
        .collect()
}

/// The whole flow under a converting Git, twice: once with the frozen upstream
/// documents reachable, and once `--offline` with no remap row at all, where
/// the snapshot is the only thing that can answer.
fn assert_renders(site: &Site) {
    let listed = site.versions(&[]);
    let text = stderr(&listed);
    assert!(listed.status.success(), "{text}");
    assert!(
        !text.contains(NOT_PRIMED),
        "the snapshot did not prime:\n{text}"
    );
    assert!(!template_rows(&listed).is_empty(), "{text}");

    let closed = site.versions(&["--offline"]);
    let text = stderr(&closed);
    assert!(closed.status.success(), "the closed network:\n{text}");
    assert!(!text.contains(NOT_PRIMED), "{text}");
    assert!(
        template_rows(&closed).contains(&"26.13.0".to_string()),
        "a template-only version rendered with no upstream at all:\n{text}"
    );
}

/// `core.autocrlf=true` in the user's global configuration — the stock Windows
/// machine — leaves the registry's tree byte-identical to its blobs, so the
/// snapshot primes with no complaint and the template answers offline.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn a_converting_git_still_reads_the_registry_byte_for_byte() {
    assert_renders(&Site::new(None));
}

/// The registry's own `.gitattributes` asks for the conversion this time.
/// `core.autocrlf=false` does not answer that one — `$GIT_DIR/info/attributes`,
/// which outranks every `.gitattributes` in the tree, does.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION")]
fn a_registrys_own_eol_attribute_does_not_reach_what_vibe_hashes() {
    assert_renders(&Site::new(Some("* text=auto eol=crlf\n")));
}
