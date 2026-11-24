//! Public metadata-only plans list declarations without executing their scripts.
mod common;

use std::{collections::BTreeMap, fs, path::Path};

use common::UserScratch;

/// Two selected packages share an id but retain distinct titles and graph order.
struct Fixture {
    root: tempfile::TempDir,
    user: UserScratch,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let user = UserScratch::new_in(root.path());
        for dir in ["project", "home", "temporary", "registry"] {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        for (name, title, requires) in [
            (
                "root",
                "Root preparation",
                "[requires.packages]\n'org.plan/child'='=1.0.0'\n",
            ),
            ("child", "Child preparation", ""),
        ] {
            let package = root.path().join(format!("registry/org.plan/{name}/v1.0.0"));
            fs::create_dir_all(package.join("hooks")).unwrap();
            fs::write(package.join("vibe.toml"), format!(
                "[package]\ngroup='org.plan'\nname='{name}'\nversion='1.0.0'\nkind='tool'\n{requires}\n\
                 [[extension]]\nid='shared-step'\npoint='slot:post-install'\ntitle='{title}'\n\
                 handler={{kind='script',base='hooks/post'}}\n\
                 [[extension]]\nid='another-step'\npoint='slot:post-install'\ntitle='Second {name} declaration'\n\
                 handler={{kind='script',base='hooks/post'}}\n"
            )).unwrap();
            fs::write(
                package.join("hooks/post.sh"),
                "set -eu\nprintf executed > \"$VIBE_PROJECT_ROOT/SENTINEL\"\n",
            )
            .unwrap();
            fs::write(package.join("hooks/post.ps1"),
                "Set-Content -LiteralPath (Join-Path $env:VIBE_PROJECT_ROOT 'SENTINEL') -Value executed\n").unwrap();
        }
        fs::write(root.path().join("project/vibe.toml"), format!(
            "[project]\nname='plan-fixture'\nversion='1.0.0'\n[[registry]]\nname='selected'\nurl='file://{}'\n",
            root.path().join("registry").display()
        )).unwrap();
        Self { root, user }
    }

    fn preview(&self, json: bool) -> std::process::Output {
        let home = self.root.path().join("home");
        let mut command = self.user.vibe();
        command
            .args([
                "install",
                "org.plan/root@=1.0.0",
                "--plan-only",
                "--offline",
                "--no-default-registry",
                "--path",
            ])
            .arg(self.root.path().join("project"))
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("CODEX_HOME", home.join(".codex"))
            .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("XDG_CACHE_HOME", home.join(".cache"))
            .env("XDG_STATE_HOME", home.join(".local/state"))
            .env("XDG_RUNTIME_DIR", home.join(".runtime"))
            .env("NO_COLOR", "1")
            .env("COLUMNS", "120")
            .env_remove("VIBE_AGENT_COMMAND")
            .env_remove("VIBE_AGENT_ARGS");
        for key in ["TMPDIR", "TMP", "TEMP"] {
            command.env(key, self.root.path().join("temporary"));
        }
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let key = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    out.insert(format!("{key}/"), Vec::new());
                    collect(root, &path, out);
                } else {
                    out.insert(key, fs::read(path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        collect(self.root.path(), self.root.path(), &mut out);
        out
    }

    fn assert_unpublished(&self, before: &BTreeMap<String, Vec<u8>>) {
        assert_eq!(
            *before,
            self.snapshot(),
            "preview mutated an owned fixture root"
        );
        let project = self.root.path().join("project");
        assert!(!project.join("SENTINEL").exists(), "script executed");
        assert!(!project.join("vibe.lock").exists());
        assert!(!project.join(common::deps_root()).exists());
    }
}

#[test]
fn plan_shows_step_without_running_it() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let output = fixture.preview(false);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.assert_unpublished(&before);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("plan for 2 packages"), "{text}");
    let child = text.find("org.plan/child").unwrap();
    let root = text.find("org.plan/root").unwrap();
    assert!(root < child, "selected metadata order: {text}");
    for (section, title, second) in [
        (
            &text[root..child],
            "Root preparation",
            "Second root declaration",
        ),
        (
            &text[child..],
            "Child preparation",
            "Second child declaration",
        ),
    ] {
        let first_line = section
            .lines()
            .find(|line| line.contains("shared-step"))
            .expect("declared step under selected package");
        assert!(
            first_line.contains("1.")
                && first_line.contains("script")
                && first_line.contains(title),
            "{section}"
        );
        let second_line = section
            .lines()
            .find(|line| line.contains("another-step"))
            .unwrap();
        assert!(
            second_line.contains("2.")
                && second_line.contains("script")
                && second_line.contains(second),
            "{section}"
        );
        assert!(section.find("shared-step").unwrap() < section.find("another-step").unwrap());
    }
}

#[test]
fn plan_json_keeps_the_existing_contract_without_contribution_fields() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let output = fixture.preview(true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.assert_unpublished(&before);
    let docs = common::hook_slot::documents(&output.stdout);
    assert_eq!(docs.len(), 1, "{docs:?}");
    let plan = &docs[0];
    let _: vibe_wire::generated::install::e2::plan::InstallPlan =
        serde_json::from_value(plan.clone()).unwrap();
    assert_eq!(plan["command"], "install:plan");
    assert_eq!(plan["preview_complete"], true);
    assert_eq!(plan["remaps"], serde_json::json!([]));
    assert_eq!(plan["packages"].as_array().unwrap().len(), 2);
    for key in plan.as_object().unwrap().keys() {
        assert!(
            [
                "command",
                "packages",
                "preview_complete",
                "remaps",
                "hostings"
            ]
            .contains(&key.as_str()),
            "{key}"
        );
    }
    for row in plan["packages"].as_array().unwrap() {
        for key in row.as_object().unwrap().keys() {
            assert!(
                [
                    "package",
                    "version",
                    "source_evidence",
                    "source_hosting",
                    "source_candidates"
                ]
                .contains(&key.as_str()),
                "{key}"
            );
        }
    }
    assert!(!String::from_utf8_lossy(&output.stdout).contains("shared-step"));
}
