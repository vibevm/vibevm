//! Unsupported common package input must refuse before scoped maintenance effects.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES");
mod common;
use common::UserScratch;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use vibe_core::manifest::{Lockfile, Manifest};

const PACKAGE: &str = "org.example/maintenance";

#[derive(Debug, PartialEq, Eq, Serialize)]
enum Bytes {
    Directory,
    File(Vec<u8>),
}
type Snapshot = BTreeMap<String, Bytes>;

fn snapshot(root: &Path, omit_metadata: bool) -> Snapshot {
    fn walk(root: &Path, path: &Path, out: &mut Snapshot) {
        let name = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            out.insert(name, Bytes::Directory);
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), out);
            }
        } else {
            out.insert(name, Bytes::File(fs::read(path).unwrap()));
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if omit_metadata && path.file_name().unwrap() == ".vibe" {
                continue;
            }
            walk(root, &path, &mut out);
        }
    }
    out
}

struct Fixture {
    _temporary: Option<tempfile::TempDir>,
    root: PathBuf,
    project: PathBuf,
    registry: PathBuf,
    user: UserScratch,
}
impl Fixture {
    fn new(case: &str) -> Self {
        let temp = if let Some(base) = std::env::var_os("NST_COMMON_MAINTENANCE_EVIDENCE") {
            fs::create_dir_all(&base).unwrap();
            tempfile::Builder::new()
                .prefix(case)
                .tempdir_in(base)
                .unwrap()
        } else {
            tempfile::tempdir().unwrap()
        };
        let (root, temporary) = if std::env::var_os("NST_COMMON_MAINTENANCE_EVIDENCE").is_some() {
            (temp.keep(), None)
        } else {
            (temp.path().to_owned(), Some(temp))
        };
        let project = root.join("project");
        let registry = root.join("registry");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("evidence")).unwrap();
        let f = Self {
            _temporary: temporary,
            root,
            project,
            registry,
            user: UserScratch::new(),
        };
        f.package("1.0.0", false, false);
        let manifest = format!(
            "[project]\nname='maintenance-fixture'\nversion='1.0.0'\n[requires.packages.'{PACKAGE}']\nversion='^1'\nlink='static'\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            f.registry.display()
        );
        Manifest::parse_str(&manifest).unwrap();
        fs::write(f.project.join("vibe.toml"), manifest).unwrap();
        for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
            fs::write(f.project.join(name), "# Fixture launcher\n").unwrap();
        }
        let seeded = f.run("seed", &["install"]);
        assert!(
            seeded.status.success(),
            "seed failed: {}",
            String::from_utf8_lossy(&seeded.stderr)
        );
        assert_eq!(f.version(), "1.0.0");
        assert!(f.project.join(common::deps_root()).is_dir());
        let boot = snapshot(&f.project.join(common::boot_dir()), false);
        assert!(
            !boot.is_empty(),
            "the fixture needs actual generated boot bytes"
        );
        f
    }
    fn package(&self, version: &str, common: bool, hook: bool) {
        let dir = self
            .registry
            .join("org.example/maintenance")
            .join(format!("v{version}"));
        fs::create_dir_all(dir.join("boot")).unwrap();
        fs::create_dir_all(dir.join("hooks")).unwrap();
        let configuration = if common {
            "\n[[configuration.features]]\nid='extra'\nname='Extra'\ndefault=true\n"
        } else {
            ""
        };
        let hooks = if hook {
            "\n[hooks]\npre-install='hooks/canary'\n"
        } else {
            ""
        };
        let text = format!(
            "[package]\ngroup='org.example'\nname='maintenance'\nkind='flow'\nversion='{version}'\n[boot_snippet]\nsource='boot/rules.md'\ncategory='flow'\n{hooks}{configuration}"
        );
        let parsed = Manifest::parse_str(&text).unwrap();
        if common {
            parsed
                .configuration
                .as_ref()
                .unwrap()
                .compile_choices()
                .unwrap();
        }
        fs::write(dir.join("vibe.toml"), text).unwrap();
        fs::write(
            dir.join("boot/rules.md"),
            format!("# Maintenance {{#root}}\n\nVERSION {version}\n"),
        )
        .unwrap();
        fs::write(dir.join("payload.txt"), format!("PAYLOAD {version}\n")).unwrap();
        fs::write(
            dir.join("hooks/canary.sh"),
            "set -eu\nprintf 'HOOK EXECUTED\\n' > \"$VIBE_PROJECT_ROOT/maintenance-hook.txt\"\n",
        )
        .unwrap();
    }
    fn run(&self, label: &str, args: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command
            .current_dir(&self.project)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"));
        for name in ["VIBE_AGENT_COMMAND", "VIBE_AGENT_ARGS", "VIBE_INVOKED_BY"] {
            command.env_remove(name);
        }
        command.args(args);
        if args.first() == Some(&"reinstall") {
            command.arg(&self.project);
        } else {
            command.args(["--path", self.project.to_str().unwrap()]);
        }
        let output = command
            .args(["--offline", "--assume-yes", "--json"])
            .output()
            .unwrap();
        fs::write(
            self.root.join(format!("evidence/{label}.stdout")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            self.root.join(format!("evidence/{label}.stderr")),
            &output.stderr,
        )
        .unwrap();
        fs::write(
            self.root.join(format!("evidence/{label}.status")),
            format!("{:?}\n", output.status.code()),
        )
        .unwrap();
        output
    }
    fn version(&self) -> String {
        Lockfile::read(self.project.join("vibe.lock"))
            .unwrap()
            .packages
            .iter()
            .find(|p| p.name.as_str() == "maintenance")
            .unwrap()
            .version
            .to_string()
    }
    fn save_snapshot(&self, name: &str, snapshot: &Snapshot) {
        fs::write(
            self.root.join(format!("evidence/{name}.json")),
            serde_json::to_vec_pretty(snapshot).unwrap(),
        )
        .unwrap();
    }
    fn refusal(&self, label: &str, args: &[&str]) {
        let before = snapshot(&self.project, true);
        self.save_snapshot(&format!("{label}-semantic-before"), &before);
        self.save_snapshot(
            &format!("{label}-metadata-before"),
            &snapshot(&self.project.join(".vibe"), false),
        );
        let output = self.run(label, args);
        let after = snapshot(&self.project, true);
        self.save_snapshot(&format!("{label}-semantic-after"), &after);
        self.save_snapshot(
            &format!("{label}-metadata-after"),
            &snapshot(&self.project.join(".vibe"), false),
        );
        let changed = before
            .keys()
            .chain(after.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|p| before.get(*p) != after.get(*p))
            .cloned()
            .collect::<Vec<_>>();
        let diagnostic = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let retained_refusal = match label {
            "installed-provider" => diagnostic.contains(&format!("package `{PACKAGE}` requires retained configuration but has no lock record"))
                && diagnostic.contains("PROP-069#PERSISTENCE"),
            "force-input" => diagnostic.contains(&format!("retained input changed (`{PACKAGE}` original configuration is not recorded in vibe.lock)"))
                && diagnostic.contains("PROP-069#EFFECT-CONSUMERS"),
            _ => false,
        };
        let refused = !output.status.success()
            && (diagnostic.contains("does not yet evaluate common configuration")
                || diagnostic.contains("does not yet evaluate configuration input")
                || retained_refusal);
        let hook = self.project.join("maintenance-hook.txt").exists();
        let facts = serde_json::json!({"exit":output.status.code(),"configuration_refused":refused,"hook_ran":hook,"changed_project_semantic_paths":changed,"project":self.project,"settings":self.user.settings,"registry_cache":self.user.cache,"note":"root .vibe cache/trace/lifecycle metadata is recorded separately; no zero-all-IO claim"});
        fs::write(
            self.root.join(format!("evidence/{label}-observed.json")),
            serde_json::to_vec_pretty(&facts).unwrap(),
        )
        .unwrap();
        assert!(
            refused && before == after && !hook,
            "unsupported common maintenance crossed its pre-effect boundary: {facts}\n{diagnostic}"
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES")]
fn scoped_update_refuses_valid_common_candidate_before_semantic_writes_or_hook() {
    let f = Fixture::new("scoped-candidate-");
    f.package("1.1.0", true, true);
    f.refusal("scoped", &["update", PACKAGE]);
}
#[test]
fn legacy_scoped_update_still_installs_new_bytes_and_runs_its_real_hook() {
    let f = Fixture::new("legacy-control-");
    let before = snapshot(&f.project, true);
    f.package("1.1.0", false, true);
    let output = f.run("legacy-update", &["update", PACKAGE]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(f.version(), "1.1.0");
    assert_eq!(
        fs::read_to_string(f.project.join("maintenance-hook.txt")).unwrap(),
        "HOOK EXECUTED\n"
    );
    assert_ne!(before, snapshot(&f.project, true));
    let slot = f.project.join(common::slot_rel(
        "org.example.maintenance",
        "1.1.0",
        "payload.txt",
    ));
    assert_eq!(fs::read_to_string(slot).unwrap(), "PAYLOAD 1.1.0\n");
}
#[test]
fn whole_update_and_direct_install_keep_the_existing_common_refusal() {
    for whole in [true, false] {
        let f = Fixture::new(if whole {
            "whole-control-"
        } else {
            "install-control-"
        });
        f.package("1.1.0", true, true);
        if whole {
            // Force this control through candidate admission. An unchanged
            // offline whole update may legitimately reuse the old fresh lock.
            let path = f.project.join("vibe.toml");
            let mut manifest = Manifest::read(&path).unwrap();
            *manifest
                .requires
                .packages
                .iter_mut()
                .find(|p| p.name == "maintenance")
                .unwrap() = vibe_core::PackageRef::parse("org.example/maintenance@=1.1.0").unwrap();
            manifest.write(&path).unwrap();
            f.refusal("whole", &["update"]);
        } else {
            f.refusal("install", &["install", "org.example/maintenance@=1.1.0"]);
        }
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES")]
fn scoped_update_does_not_ignore_authored_configuration_on_a_workspace_sibling() {
    let f = Fixture::new("sibling-intent-");
    f.package("1.1.0", false, true);
    let root = f.project.join("vibe.toml");
    let mut text = fs::read_to_string(&root).unwrap();
    text.push_str("\n[workspace]\nmembers=['member']\n");
    Manifest::parse_str(&text).unwrap();
    fs::write(root, text).unwrap();
    fs::create_dir(f.project.join("member")).unwrap();
    let sibling = "[project]\nname='member'\nversion='1.0.0'\n[requires.packages.'org.example/maintenance']\nversion='^1'\nconfiguration={policy='require-selection'}\n";
    Manifest::parse_str(sibling).unwrap();
    fs::write(f.project.join("member/vibe.toml"), sibling).unwrap();
    f.refusal("sibling-scoped", &["update", PACKAGE]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn raw_source_input_refuses_before_git_declaration_or_forced_reinstall() {
    for force in [false, true] {
        let f = Fixture::new("source-edit-input-");
        let path = f.project.join("vibe.toml");
        let mut manifest = Manifest::read(&path).unwrap();
        let input=Manifest::parse_str("[project]\nname='input'\nversion='1.0.0'\n[requires.packages.'org.example/maintenance']\nversion='^1'\nconfiguration={policy='require-selection'}\n").unwrap();
        manifest.requires.configurations = input.requires.configurations;
        manifest.write(&path).unwrap();
        if force {
            f.refusal("force-input", &["reinstall", "--force"]);
        } else {
            f.refusal(
                "git-input",
                &[
                    "install",
                    PACKAGE,
                    "--git",
                    "file:///unreachable-admission-source",
                    "--tag",
                    "v1.1.0",
                ],
            );
        }
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn fresh_install_and_plain_reinstall_admit_the_actual_installed_provider() {
    for reinstall in [false, true] {
        let f = Fixture::new("installed-definition-");
        let path = f.project.join(common::slot_rel(
            "org.example.maintenance",
            "1.0.0",
            "vibe.toml",
        ));
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("\n[configuration]\n");
        Manifest::parse_str(&text).unwrap();
        fs::write(path, text).unwrap();
        f.refusal(
            "installed-provider",
            if reinstall {
                &["reinstall"]
            } else {
                &["install"]
            },
        );
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn scoped_transitive_candidate_and_child_intent_refuse_before_any_slot_effect() {
    for rich_definition in [false, true] {
        let f = Fixture::new("transitive-input-");
        f.package("1.1.0", false, true);
        let root = f.registry.join("org.example/maintenance/v1.1.0/vibe.toml");
        let mut text = fs::read_to_string(&root).unwrap();
        text.push_str(if rich_definition {"\n[requires.packages]\n'org.example/child'='1'\n"} else {"\n[requires.packages.'org.example/child']\nversion='1'\nconfiguration={policy='require-selection'}\n"});
        Manifest::parse_str(&text).unwrap();
        fs::write(root, text).unwrap();
        let child = f.registry.join("org.example/child/v1.0.0");
        fs::create_dir_all(&child).unwrap();
        fs::write(
            child.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.example'\nname='child'\nkind='flow'\nversion='1.0.0'\n{}",
                if rich_definition {
                    "[configuration]\n"
                } else {
                    ""
                }
            ),
        )
        .unwrap();
        f.refusal("transitive", &["update", PACKAGE]);
    }
}
