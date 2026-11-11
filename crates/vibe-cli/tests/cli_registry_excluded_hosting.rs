//! `exclude_hosting` genuinely switches a registry off (PROP-073
//! `#SOURCE-POLICY`).
//!
//! The owner's shape: the default `github` and `gitverse` registries sit in
//! `~/.vibe/registry.toml` above a site profile that excludes both hostings
//! and declares a local monorepo. Those two registries stay declared — the
//! policy must disable them, not ask the operator to delete them — and
//! resolution must walk past them to the monorepo instead of dying on the
//! first excluded endpoint.
//!
//! Zero contact is proved mechanically, not read out of the prose: a third
//! excluded registry points at a loopback listener this test owns, and the
//! listener counts every connection it accepts. The count must stay at zero
//! while the listener is demonstrably reachable.
#![cfg(unix)]

use std::fs;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A loopback listener that accepts nothing and counts everything.
struct ContactProbe {
    address: SocketAddr,
    accepted: Arc<AtomicUsize>,
}

impl ContactProbe {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
        let address = listener.local_addr().expect("loopback address");
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = accepted.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_err() {
                    break;
                }
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });
        Self { address, accepted }
    }

    fn url(&self) -> String {
        format!("https://{}/org", self.address)
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    /// The probe only means something if a connection would in fact be
    /// counted; this proves the instrument before the measurement is used.
    fn assert_instrument_works(&self) {
        let before = self.accepted();
        let stream = TcpStream::connect(self.address).expect("the probe accepts a connection");
        drop(stream);
        for _ in 0..200 {
            if self.accepted() > before {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the contact probe never counted its own control connection");
    }
}

struct Site {
    root: tempfile::TempDir,
}

impl Site {
    fn new() -> Self {
        let site = Site {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(site.path("empty"), "").unwrap();
        for dir in ["home", "settings"] {
            fs::create_dir_all(site.path(dir)).unwrap();
        }
        let work = site.path("work");
        site.git(None, &["init", "-q", "-b", "main", work.to_str().unwrap()]);
        let package = work.join("org.fixture/demo/v1.0.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            "[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='1.0.0'\n",
        )
        .unwrap();
        fs::write(package.join("payload"), "corp monorepo v1.0.0\n").unwrap();
        site.git(Some(&work), &["add", "-A"]);
        site.git(Some(&work), &["commit", "-q", "-m", "seed"]);
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

    /// `~/.vibe/registry.toml` as it ships: the public hostings first, in
    /// user scope, written by nobody and removed by nobody.
    fn user_registries(&self, probe: &str) {
        fs::write(
            self.path("settings/registry.toml"),
            format!(
                "[[registry]]\nname='vibespecs'\ntype='github'\nurl='https://github.com/vibespecs'\n\
                 [[registry]]\nname='vibespecs-gitverse'\ntype='gitverse'\n\
                 url='https://gitverse.ru/vibespecs'\n\
                 [[registry]]\nname='probe'\ntype='git'\nurl='{probe}'\n"
            ),
        )
        .unwrap();
    }

    /// The site profile: the corporate monorepo, and the three hostings the
    /// operator does not want contacted.
    fn site_profile(&self) {
        let bare = self.path("mono.git");
        fs::write(
            self.path("settings/site-profile.toml"),
            format!(
                "schema = 1\n\
                 [[registry]]\nname='corp'\ntype='monorepo'\nurl='{}'\nref='main'\n\
                 [source_policy]\n\
                 exclude_hosting=['github','gitverse','host:127.0.0.1']\n",
                bare.display()
            ),
        )
        .unwrap();
    }

    fn project(&self, name: &str) -> PathBuf {
        let project = self.path(name);
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            "[project]\nname='consumer'\nversion='1.0.0'\n\
             [requires.packages]\n'org.fixture/demo'='=1.0.0'\n",
        )
        .unwrap();
        project
    }

    fn vibe(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(cwd)
            .args(args)
            .env("HOME", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_REGISTRY_CACHE", self.path("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("CI", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.path("empty"))
            .env_remove("GIT_SSH")
            .env_remove("GIT_SSH_COMMAND")
            .env_remove("SSH_AUTH_SOCK")
            .output()
            .unwrap()
    }

    fn vibe_ok(&self, cwd: &Path, args: &[&str]) -> Output {
        let output = self.vibe(cwd, args);
        assert!(
            output.status.success(),
            "vibe {args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// 4a — the install the owner could not run: three excluded registries
/// declared above the one that can serve, and the install succeeds from the
/// monorepo without the excluded hosts being contacted at all.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn an_excluded_registry_is_skipped_as_absent_and_never_contacted() {
    let probe = ContactProbe::start();
    probe.assert_instrument_works();
    let control = probe.accepted();

    let site = Site::new();
    site.user_registries(&probe.url());
    site.site_profile();
    let project = site.project("consumer");

    let output = site.vibe_ok(&project, &["-v", "--unattended", "install"]);

    // The monorepo served it.
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("vibe.lock")).unwrap()).unwrap();
    let row = lock["package"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str() == Some("demo"))
        .expect("demo lock row")
        .clone();
    assert_eq!(row["registry"].as_str(), Some("corp"));

    // Nothing reached the loopback registry — the instrument that just
    // counted a control connection counted nothing from the run.
    assert_eq!(
        probe.accepted(),
        control,
        "the excluded loopback registry was contacted"
    );

    // And nothing even opened a clone for an excluded registry.
    let cache = site.path("cache");
    if cache.exists() {
        let mut found = Vec::new();
        collect_files(&cache, &mut found);
        for path in &found {
            let shown = path.display().to_string();
            assert!(
                !shown.contains("github.com")
                    && !shown.contains("gitverse.ru")
                    && !shown.contains("127.0.0.1"),
                "an excluded registry left a cache artifact at {shown}"
            );
        }
    }

    // 4d — `-v` says which registry, at which URL, and why one was skipped.
    let said = text(&output);
    assert!(
        said.contains("registry vibespecs (github.com) skipped: excluded by site policy"),
        "{said}"
    );
    assert!(
        said.contains("registry vibespecs-gitverse (gitverse.ru) skipped: excluded by site policy"),
        "{said}"
    );
    assert!(
        said.contains("checking monorepo registry corp ("),
        "the verbose run must name the registry it did ask: {said}"
    );
    // 4a — said once per run, not once per attempt.
    assert_eq!(
        said.matches("registry vibespecs (github.com) skipped: excluded by site policy")
            .count(),
        1,
        "{said}"
    );
}

/// 4c — both listings mark the disabled registries and neither suggests
/// removing them, and both name the effective primary.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn the_listings_mark_the_disabled_registries_and_leave_them_in_place() {
    let probe = ContactProbe::start();
    let site = Site::new();
    site.user_registries(&probe.url());
    site.site_profile();
    let project = site.project("consumer");

    for (label, args) in [
        ("registry list", vec!["registry", "list"]),
        ("self apply --show", vec!["self", "apply", "--show"]),
    ] {
        let said = text(&site.vibe_ok(&project, &args));
        assert!(
            said.matches("disabled by site policy (exclude_hosting) — not used")
                .count()
                >= 3,
            "{label}: every excluded registry is marked\n{said}"
        );
        assert!(said.contains("effective primary: corp"), "{label}: {said}");
        assert!(
            !said.contains("registry remove"),
            "{label} must not offer to delete a registry the owner wants kept\n{said}"
        );
        // The rows are still listed — disabling is not hiding.
        assert!(said.contains("vibespecs"), "{label}: {said}");
        assert!(said.contains("corp"), "{label}: {said}");
    }

    // `--json` carries the same decision, additively.
    let json: serde_json::Value = serde_json::from_slice(
        &site
            .vibe_ok(&project, &["--json", "registry", "list"])
            .stdout,
    )
    .unwrap();
    assert_eq!(json["effective_primary"], "corp");
    let rows = json["registries"].as_array().unwrap();
    let marked: Vec<&str> = rows
        .iter()
        .filter(|row| row["disabled_by_source_policy"] == true)
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(marked, vec!["vibespecs", "vibespecs-gitverse", "probe"]);
    assert_eq!(rows.len(), 4, "the disabled rows are listed, not hidden");

    let shown: serde_json::Value = serde_json::from_slice(
        &site
            .vibe_ok(&project, &["--json", "self", "apply", "--show"])
            .stdout,
    )
    .unwrap();
    let registries = shown["effective"]["registries"].as_array().unwrap();
    let disabled: Vec<&str> = registries
        .iter()
        .filter_map(|row| row.get("disabled_by").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(disabled, vec!["github", "gitverse", "host:127.0.0.1"]);
}

/// 4a — when the policy disables every configured registry, the failure says
/// so and names both halves of the split.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn every_registry_excluded_fails_naming_allowed_and_skipped() {
    let probe = ContactProbe::start();
    let control = probe.accepted();
    let site = Site::new();
    site.user_registries(&probe.url());
    // No monorepo this time: the site profile excludes and offers nothing.
    fs::write(
        site.path("settings/site-profile.toml"),
        "schema = 1\n[source_policy]\n\
         exclude_hosting=['github','gitverse','host:127.0.0.1']\n",
    )
    .unwrap();
    let project = site.project("consumer");

    let output = site.vibe(&project, &["--unattended", "install"]);
    assert!(!output.status.success(), "{}", text(&output));
    let said = text(&output);
    // The refusal names both halves of the split: nothing allowed, and the
    // endpoints that were excluded and therefore not contacted.
    assert!(
        said.contains(
            "allowed registries: (none — the source policy disables every configured registry)"
        ),
        "{said}"
    );
    for endpoint in [
        "https://github.com/vibespecs",
        "https://gitverse.ru/vibespecs",
    ] {
        assert!(said.contains(endpoint), "{said}");
    }
    assert!(said.contains("were not contacted"), "{said}");
    // And it does not tell the owner to delete the registries they keep.
    assert!(!said.contains("registry remove"), "{said}");
    assert_eq!(probe.accepted(), control, "nothing was contacted");
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            collect_files(&path, out);
        }
    }
}
