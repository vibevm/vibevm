//! Actual CLI admission precedes checkout, clean, hooks and durable project writes.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION");
mod common;

use common::UserScratch;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use vibe_core::manifest::{Lockfile, Manifest};

#[derive(Debug, PartialEq, Eq, Serialize)]
enum Entry {
    Directory,
    File { mode: u32, bytes: Vec<u8> },
    Symlink(PathBuf),
}
type Snapshot = BTreeMap<String, Entry>;
fn snapshot(root: &Path) -> Snapshot {
    fn walk(root: &Path, path: &Path, out: &mut Snapshot) {
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let metadata = fs::symlink_metadata(path).unwrap();
        if metadata.file_type().is_symlink() {
            out.insert(name, Entry::Symlink(fs::read_link(path).unwrap()));
        } else if metadata.is_dir() {
            out.insert(name, Entry::Directory);
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), out);
            }
        } else {
            out.insert(
                name,
                Entry::File {
                    mode: metadata.permissions().mode() & 0o777,
                    bytes: fs::read(path).unwrap(),
                },
            );
        }
    }
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        // Command trace/lease bookkeeping is captured separately, never called zero I/O.
        if path.file_name().unwrap() != ".vibe" {
            walk(root, &path, &mut out);
        }
    }
    out
}
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "protocol.file.allow=always",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn write(path: &Path, body: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
fn publish(repo: &Path, name: &str, version: &str, common: bool) -> String {
    if !repo.join(".git").exists() {
        fs::create_dir_all(repo).unwrap();
        git(repo, &["init", "--initial-branch=main"]);
    }
    let configuration = if common {
        "\n[[configuration.features]]\nid='extra'\nname='Extra'\ndefault=true\n"
    } else {
        ""
    };
    let hook = if version == "1.0.0" {
        ""
    } else {
        "\n[hooks]\npre-install='hooks/canary'\n"
    };
    let manifest = format!(
        "[package]\ngroup='org.vibevm'\nname='{name}'\nkind='flow'\nversion='{version}'\nmaterialization='in-place'\n[boot_snippet]\nsource='boot/rules.md'\ncategory='flow'\n{hook}{configuration}"
    );
    let parsed = Manifest::parse_str(&manifest).unwrap();
    if common {
        parsed
            .configuration
            .as_ref()
            .unwrap()
            .compile_choices()
            .unwrap();
    }
    write(&repo.join("vibe.toml"), manifest);
    write(
        &repo.join("boot/rules.md"),
        format!("# {name} {version}\nActual boot contribution.\n"),
    );
    write(&repo.join("payload.txt"), format!("{name} {version}\n"));
    write(&repo.join(".gitignore"), "ignored.tmp\n");
    write(
        &repo.join("hooks/canary.sh"),
        "set -eu\nprintf 'hook ran\\n' >> \"$VIBE_PROJECT_ROOT/inplace-hook.log\"\n",
    );
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "fixture package version"]);
    git(repo, &["tag", &format!("v{version}")]);
    git(repo, &["rev-parse", "HEAD"])
}

#[derive(Debug, Serialize)]
struct PreparedFetch {
    source: String,
    reference: String,
    repository: String,
}
struct Fixture {
    _temp: Option<tempfile::TempDir>,
    root: PathBuf,
    project: PathBuf,
    org: PathBuf,
    user: UserScratch,
    names: Vec<String>,
}
impl Fixture {
    fn new(names: &[&str]) -> Self {
        let evidence = std::env::var_os("NST_CLI_INPLACE_EVIDENCE");
        let temp = if let Some(base) = &evidence {
            fs::create_dir_all(base).unwrap();
            tempfile::Builder::new()
                .prefix("cli-inplace-")
                .tempdir_in(base)
                .unwrap()
        } else {
            tempfile::tempdir().unwrap()
        };
        let (root, temporary) = if evidence.is_some() {
            (temp.keep(), None)
        } else {
            (temp.path().to_owned(), Some(temp))
        };
        let project = root.join("project");
        let org = root.join("org");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&org).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("evidence")).unwrap();
        let f = Self {
            _temp: temporary,
            root,
            project,
            org,
            user: UserScratch::new(),
            names: names.iter().map(|n| (*n).into()).collect(),
        };
        for name in names {
            publish(&f.repo(name), name, "1.0.0", false);
        }
        // Existing cli_registry_mgmt fixture grammar: explicit git+file org,
        // default fqdn naming, one <group>.<name>.git repository per package.
        let url = format!("git+file:///{}", f.org.canonicalize().unwrap().display());
        common::write_project_with_per_package_registry(&f.project, &url);
        let mut manifest = fs::read_to_string(f.project.join("vibe.toml")).unwrap();
        manifest.push_str("\n[requires.packages]\n");
        for name in names {
            manifest.push_str(&format!("'org.vibevm/{name}'='^1'\n"));
        }
        Manifest::parse_str(&manifest).unwrap();
        write(&f.project.join("vibe.toml"), manifest);
        for launcher in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
            write(&f.project.join(launcher), "# Fixture launcher\n");
        }
        let result = f.run("seed", &["install"]);
        f.success(&result);
        for name in names {
            assert_eq!(f.version(name), "1.0.0");
            assert!(f.slot(name).join(".git").is_dir());
            write(
                &f.slot(name).join(".git/sentinel"),
                "keep this repository\n",
            );
        }
        assert!(f.project.join(common::boot_dir()).is_dir());
        f
    }
    fn repo(&self, name: &str) -> PathBuf {
        self.org.join(format!("org.vibevm.{name}.git"))
    }
    fn slot(&self, name: &str) -> PathBuf {
        self.project
            .join(common::deps_root())
            .join(format!("org.vibevm.{name}"))
    }
    fn version(&self, name: &str) -> String {
        Lockfile::read(self.project.join("vibe.lock"))
            .unwrap()
            .packages
            .into_iter()
            .find(|p| p.name.as_str() == name)
            .unwrap()
            .version
            .to_string()
    }
    fn dirty(&self, name: &str) {
        let slot = self.slot(name);
        write(&slot.join("payload.txt"), "TRACKED DIRTY\n");
        write(&slot.join("untracked.tmp"), "UNTRACKED\n");
        write(&slot.join("ignored.tmp"), "IGNORED\n");
        assert_eq!(git(&slot, &["check-ignore", "ignored.tmp"]), "ignored.tmp");
    }
    fn run(&self, label: &str, args: &[&str]) -> Output {
        let trace = self.root.join(format!("evidence/{label}.git-trace.jsonl"));
        let mut cmd = self.user.vibe();
        cmd.current_dir(&self.project)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", "/dev/null")
            .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
            .env("GIT_CONFIG_VALUE_1", "always")
            .env("GIT_TRACE2_EVENT", &trace);
        for var in [
            "VIBE_AGENT_COMMAND",
            "VIBE_AGENT_ARGS",
            "VIBE_INVOKED_BY",
            "VIBE_OFFLINE",
        ] {
            cmd.env_remove(var);
        }
        // --offline excludes git+file registry preparation; file-only Git
        // transport and isolated registry configuration keep this fixture local.
        let output = cmd
            .args(args)
            .args([
                "--path",
                self.project.to_str().unwrap(),
                "--assume-yes",
                "--unattended",
                "--json",
            ])
            .output()
            .unwrap();
        write(
            &self.root.join(format!("evidence/{label}.stdout")),
            &output.stdout,
        );
        write(
            &self.root.join(format!("evidence/{label}.stderr")),
            &output.stderr,
        );
        write(
            &self.root.join(format!("evidence/{label}.status")),
            format!("{:?}\n", output.status.code()),
        );
        output
    }
    fn success(&self, output: &Output) {
        assert!(
            output.status.success(),
            "status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    /// Only the two root-authorized acquisition surfaces are separated from
    /// semantic Git state. HEAD/index/config/refs/reflogs remain in the oracle.
    fn snapshots(&self) -> (Snapshot, Snapshot) {
        let git_roots = self
            .names
            .iter()
            .map(|name| {
                format!(
                    "{}/.git",
                    self.slot(name)
                        .strip_prefix(&self.project)
                        .unwrap()
                        .to_str()
                        .unwrap()
                )
            })
            .collect::<Vec<_>>();
        snapshot(&self.project).into_iter().partition(|(path, _)| {
            !git_roots.iter().any(|git| {
                path == &format!("{git}/FETCH_HEAD")
                    || path == &format!("{git}/objects")
                    || path.starts_with(&format!("{git}/objects/"))
            })
        })
    }
    fn prepared_fetches(&self, label: &str) -> Vec<PreparedFetch> {
        let text = fs::read_to_string(self.root.join(format!("evidence/{label}.git-trace.jsonl")))
            .unwrap();
        let events = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        // Native fetch is performed in the original slot context. Trace2's
        // def_repo event supplies that repository even when argv has no -C.
        let repositories = events
            .iter()
            .filter(|event| event["event"] == "def_repo")
            .filter_map(|event| {
                Some((
                    event["sid"].as_str()?.to_owned(),
                    event["worktree"].as_str()?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        events
            .iter()
            .filter_map(|event| {
                if event["event"] != "start" {
                    return None;
                }
                let repository = repositories.get(event["sid"].as_str()?)?;
                if !self
                    .names
                    .iter()
                    .any(|name| self.slot(name) == Path::new(repository))
                {
                    return None;
                }
                let argv = event["argv"]
                    .as_array()?
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>();
                let fetch = argv.iter().position(|arg| *arg == "fetch")?;
                let separator = argv[fetch + 1..].iter().position(|arg| *arg == "--")? + fetch + 1;
                // Preparation uses the original named remote. Apply may later
                // fetch captured local objects; that is not another selection.
                if argv.get(separator + 1) != Some(&"origin") {
                    return None;
                }
                Some(PreparedFetch {
                    source: argv.get(separator + 1)?.to_string(),
                    reference: argv.get(separator + 2)?.to_string(),
                    repository: repository.clone(),
                })
            })
            .collect()
    }
    fn refuse_unchanged(&self, label: &str, args: &[&str]) -> Vec<PreparedFetch> {
        let (before, acquisition_before) = self.snapshots();
        write(
            &self
                .root
                .join(format!("evidence/{label}.git-acquisition-before.json")),
            serde_json::to_vec_pretty(&acquisition_before).unwrap(),
        );
        let metadata = self.project.join(".vibe");
        let metadata_before = if metadata.exists() {
            snapshot(&metadata)
        } else {
            BTreeMap::new()
        };
        write(
            &self
                .root
                .join(format!("evidence/{label}.semantic-before.json")),
            serde_json::to_vec_pretty(&before).unwrap(),
        );
        write(
            &self
                .root
                .join(format!("evidence/{label}.metadata-before.json")),
            serde_json::to_vec_pretty(&metadata_before).unwrap(),
        );
        let heads: Vec<_> = self
            .names
            .iter()
            .map(|n| (n.clone(), git(&self.slot(n), &["rev-parse", "HEAD"])))
            .collect();
        let output = self.run(label, args);
        let (after, acquisition_after) = self.snapshots();
        write(
            &self
                .root
                .join(format!("evidence/{label}.git-acquisition-after.json")),
            serde_json::to_vec_pretty(&acquisition_after).unwrap(),
        );
        let metadata_after = if metadata.exists() {
            snapshot(&metadata)
        } else {
            BTreeMap::new()
        };
        write(
            &self
                .root
                .join(format!("evidence/{label}.semantic-after.json")),
            serde_json::to_vec_pretty(&after).unwrap(),
        );
        write(
            &self
                .root
                .join(format!("evidence/{label}.metadata-after.json")),
            serde_json::to_vec_pretty(&metadata_after).unwrap(),
        );
        let diagnostic = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success()
                && diagnostic.contains("does not yet evaluate common configuration"),
            "{diagnostic}"
        );
        assert_eq!(
            after, before,
            "refusal changed project checkout/manifest/lock/boot/launchers"
        );
        for (name, head) in heads {
            assert_eq!(git(&self.slot(&name), &["rev-parse", "HEAD"]), head);
            assert_eq!(self.version(&name), "1.0.0");
        }
        assert!(!self.project.join("inplace-hook.log").exists());
        let prepared = self.prepared_fetches(label);
        assert!(
            !prepared.is_empty(),
            "refusal must concern an actual prospective source"
        );
        assert!(
            prepared
                .iter()
                .all(|v| v.reference.contains("refs/tags/v1.1.0"))
        );
        write(
            &self.root.join(format!("evidence/{label}.prepared.json")),
            serde_json::to_vec_pretty(&prepared).unwrap(),
        );
        prepared
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn scoped_common_inplace_target_refuses_without_checkout_clean_or_project_writes() {
    let f = Fixture::new(&["inplace-common"]);
    publish(&f.repo("inplace-common"), "inplace-common", "1.1.0", true);
    f.dirty("inplace-common");
    let prepared = f.refuse_unchanged("common", &["update", "org.vibevm/inplace-common"]);
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].source, "origin");
    assert_eq!(Path::new(&prepared[0].repository), f.slot("inplace-common"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn later_common_target_refuses_before_earlier_legacy_checkout_moves() {
    // Deliberately reverse lexical order. qualify_roots/build_resolved_graph
    // preserve root input order; actual prepared fetch trace must corroborate it.
    let f = Fixture::new(&["z-first", "a-later"]);
    publish(&f.repo("z-first"), "z-first", "1.1.0", false);
    publish(&f.repo("a-later"), "a-later", "1.1.0", true);
    f.dirty("z-first");
    f.dirty("a-later");
    let prepared = f.refuse_unchanged(
        "two",
        &["update", "org.vibevm/z-first", "org.vibevm/a-later"],
    );
    assert_eq!(
        prepared.len(),
        2,
        "both selected candidates must actually be prepared: {prepared:?}"
    );
    assert_eq!(
        Path::new(&prepared[0].repository),
        f.slot("z-first"),
        "{prepared:?}"
    );
    assert_eq!(
        Path::new(&prepared[1].repository),
        f.slot("a-later"),
        "{prepared:?}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn legacy_inplace_control_updates_real_commit_and_preserves_repository() {
    let f = Fixture::new(&["inplace-legacy"]);
    let slot = f.slot("inplace-legacy");
    let old = git(&slot, &["rev-parse", "HEAD"]);
    let wanted = publish(&f.repo("inplace-legacy"), "inplace-legacy", "1.1.0", false);
    f.dirty("inplace-legacy");
    let output = f.run("legacy", &["update", "org.vibevm/inplace-legacy"]);
    f.success(&output);
    assert_ne!(old, wanted);
    assert_eq!(git(&slot, &["rev-parse", "HEAD"]), wanted);
    assert_eq!(f.version("inplace-legacy"), "1.1.0");
    assert_eq!(
        fs::read_to_string(slot.join(".git/sentinel")).unwrap(),
        "keep this repository\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("payload.txt")).unwrap(),
        "inplace-legacy 1.1.0\n"
    );
    assert!(!slot.join("untracked.tmp").exists());
    assert!(!slot.join("ignored.tmp").exists());
    assert!(f.project.join("inplace-hook.log").exists());
    let lock = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    assert_eq!(
        lock.packages[0].resolved_commit.as_deref(),
        Some(wanted.as_str())
    );
    assert_eq!(f.prepared_fetches("legacy").len(), 1);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn actual_checkout_origin_is_admitted_instead_of_unrelated_legacy_metadata() {
    let name = "origin-split";
    let f = Fixture::new(&[name]);
    publish(&f.repo(name), name, "1.1.0", false);
    let alternate = f.root.join("alternate-origin.git");
    publish(&alternate, name, "1.1.0", true);
    let metadata = git(&f.repo(name), &["show", "v1.1.0:vibe.toml"]);
    assert!(
        Manifest::parse_str(&metadata)
            .unwrap()
            .configuration
            .is_none()
    );
    let actual = git(&alternate, &["show", "v1.1.0:vibe.toml"]);
    assert!(
        Manifest::parse_str(&actual)
            .unwrap()
            .configuration
            .is_some()
    );
    git(
        &f.slot(name),
        &["remote", "set-url", "origin", alternate.to_str().unwrap()],
    );
    f.dirty(name);
    let prepared = f.refuse_unchanged("origin", &["update", "org.vibevm/origin-split"]);
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].source, "origin");
    assert_eq!(
        git(&f.slot(name), &["remote", "get-url", "origin"]),
        alternate.to_str().unwrap()
    );
}

#[cfg(test)]
#[path = "cli_common_package_inplace/git_source.rs"]
mod git_source;
