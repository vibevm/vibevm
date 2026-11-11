//! `vibe app versions` over a `type = "monorepo"` registry — the shape a
//! closed corporate network has, where the one `[[registry]]` is a
//! mirrored single repository (PROP-075 `#VERSIONS-CHANNEL-DISCOVERY`,
//! PROP-002 `#registry-type`).
//!
//! The other `app versions` suite passes `--registry DIR`, which hands the
//! listing a directory root outright; nothing there can notice a
//! **configured** registry whose root was never enumerated. That is
//! precisely what a monorepo was: its working copy is a directory
//! registry like any other, but the roots enumeration only knew
//! `localdir`, so no template was discovered, the listing fell back to the
//! published version union, and a channel the template declares read as
//! "this package declares no channel `lts`; channels: (none)". The
//! versions were right, which is what made it hard to see.
//!
//! The repository here is local and reached over Git's `file` transport,
//! and the template's version list is one `[[remap]]` row onto a frozen
//! copy of upstream's index — so **no run here opens a socket**.
#![cfg(all(test, unix))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The real Node template, its revision, and the frozen version list the
/// template suites drive (`crates/vibe-template/tests/fixtures/node/`).
const TEMPLATE: &str = include_str!("../../vibe-template/tests/fixtures/node/vibe.toml");
const REVISION: &str = "1.0.0";
const INDEX: &str = include_str!("../../vibe-template/tests/fixtures/node/index-excerpt.json");
const ORIGIN: &str = "https://nodejs.org/dist/";

/// A home whose only registry is a local monorepo carrying the Node
/// template, plus the remap row that keeps the version list on disk.
struct Site {
    root: tempfile::TempDir,
}

impl Site {
    fn new() -> Self {
        let site = Site {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(site.path("empty"), "").unwrap();
        for directory in ["home", "settings"] {
            fs::create_dir_all(site.path(directory)).unwrap();
        }

        // The repository, laid out `<group>/<name>/v<version>/` as every
        // directory-shaped registry is.
        let work = site.path("work");
        let template = work
            .join("org.vibevm.node/node")
            .join(format!("v{REVISION}"));
        fs::create_dir_all(&template).unwrap();
        fs::write(template.join("vibe.toml"), TEMPLATE).unwrap();
        site.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        site.git(Some(&work), &["add", "-A"]);
        site.git(Some(&work), &["commit", "-q", "-m", "the node template"]);
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

        // The template's version list, read off this filesystem: the
        // mechanism a closed network uses anyway (PROP-066 `#locator`).
        let dist = site.path("dist");
        fs::create_dir_all(&dist).unwrap();
        fs::write(dist.join("index.json"), INDEX).unwrap();
        fs::write(
            site.path("remap.toml"),
            format!(
                "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
                 reason = \"the suite's frozen copy of the Node.js release host\"\n",
                dist.display()
            ),
        )
        .unwrap();

        // The one registry: the monorepo, with the public hostings
        // excluded by name — the corporate profile's own shape.
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

    /// `vibe app versions org.vibevm.node/node <extra…>` — with **no**
    /// `--registry`, so the configured monorepo is the only source.
    fn versions(&self, target: &str, extra: &[&str]) -> Output {
        let remap = self.path("remap.toml");
        let mut args: Vec<&str> = vec!["app", "versions", target];
        let remap = remap.to_str().unwrap().to_owned();
        args.extend_from_slice(&["--remap", &remap]);
        args.extend_from_slice(extra);
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(self.root.path())
            .args(&args)
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
            .output()
            .unwrap()
    }

    fn json(&self, target: &str, extra: &[&str]) -> Value {
        let mut args = extra.to_vec();
        args.push("--json");
        let output = self.versions(target, &args);
        assert!(
            output.status.success(),
            "vibe app versions {target} {extra:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// The channel table, the origins and the template provenance all come
/// from the configured monorepo's working copy, with nothing passed on the
/// command line to point at it.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-CHANNEL-DISCOVERY"
)]
fn a_configured_monorepo_is_a_directory_registry_for_channel_discovery() {
    let site = Site::new();
    let listed = site.json("org.vibevm.node/node", &["--all"]);
    let package = &listed["packages"][0];
    assert_eq!(package["package"], "org.vibevm.node/node");

    // The template was discovered: its rows say so, and so does its
    // provenance. Before the fix every row read `published` and
    // `template` was absent.
    let rows = package["versions"].as_array().expect("the rows");
    assert!(rows.len() > 100, "the frozen window: {}", rows.len());
    assert_eq!(rows[0]["version"], "26.13.0");
    assert_eq!(rows[0]["origin"], "template");
    assert_eq!(package["template"]["coordinate"], "org.vibevm.node/node");
    assert_eq!(package["template"]["revision"], REVISION);

    // And the channel table the template's rules produced is there,
    // which is the answer `nvm ls-remote --lts` is.
    let channels = &package["channels"];
    assert_eq!(channels["lts"][0], "26.13.0");
    assert!(channels["lts/jod"].is_array(), "{channels}");
    assert_eq!(rows[0]["channels"][0], "lts");
}

/// `--channel lts` over the same home: the narrowing a coordinate asks
/// for, which answered `unknown_channel … channels: (none)`.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-CHANNEL-DISCOVERY"
)]
fn the_channel_flag_narrows_over_a_configured_monorepo() {
    let site = Site::new();
    let lts = site.json("org.vibevm.node/node", &["--channel", "lts", "--all"]);
    let rows = lts["packages"][0]["versions"]
        .as_array()
        .expect("the rows")
        .clone();
    assert!(!rows.is_empty(), "`--channel lts` selected nothing");
    assert!(
        rows.iter().all(|row| row["channels"]
            .as_array()
            .is_some_and(|channels| channels.iter().any(|channel| channel == "lts"))),
        "every row of the narrowing is a member"
    );

    // A named line narrows further, and a codename the template's window
    // does not reach is still refused by name — the refusal has to stay a
    // refusal, not become an empty listing.
    let jod = site.json("org.vibevm.node/node", &["--channel", "lts/jod", "--all"]);
    let jod_rows = jod["packages"][0]["versions"]
        .as_array()
        .expect("the rows")
        .clone();
    assert!(
        jod_rows.iter().all(|row| row["version"]
            .as_str()
            .is_some_and(|v| v.starts_with("22."))),
        "`lts/jod` is one codename's releases"
    );
    let refused = site.versions(
        "org.vibevm.node/node",
        &["--channel", "lts/argon", "--json"],
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success(), "{stderr}");
    assert!(
        stderr.contains("unknown_channel") && stderr.contains("lts/argon"),
        "{stderr}"
    );
    // And the refusal lists the channels it does know, so it can never
    // again read as a package with no channels at all.
    assert!(stderr.contains("`lts`"), "{stderr}");
}
