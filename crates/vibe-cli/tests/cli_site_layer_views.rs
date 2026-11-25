//! The applied site profile is a registry layer of its own wherever the
//! layers are shown (PROP-002 `#registry-layers`, PROP-019 `#site-profile`):
//! `vibe registry list` labels its rows `site` (and lists it alone with
//! `--scope site`), and `vibe show config` lists every layer's rows with the
//! layer as provenance and reports the profile itself.

mod common;

use std::fs;

struct UserScratch {
    user: common::UserScratch,
    home: tempfile::TempDir,
}

impl std::ops::Deref for UserScratch {
    type Target = common::UserScratch;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl UserScratch {
    fn new() -> Self {
        Self {
            user: common::UserScratch::new(),
            home: tempfile::tempdir().unwrap(),
        }
    }

    fn vibe(&self) -> assert_cmd::Command {
        let mut cmd = self.user.vibe();
        cmd.env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("VIBE_SETTINGS", &self.user.settings)
            .env_remove("VIBEVM_USER_CONFIG")
            .env_remove("VIBEVM_PROXY_MODE")
            .env_remove("VIBEVM_RELEASE_ORIGIN")
            .env_remove("VIBE_VERSIONS_SELECT")
            .env_remove("VIBE_INVOKED_BY");
        cmd
    }
}
use serde_json::Value;

const PROFILE: &str = "schema = 1\nname = \"corp-site\"\n\n\
    [[registry]]\nname = \"corp\"\ntype = \"LocalDir\"\nurl = \"file:///site/corp\"\n\n\
    [[mirror]]\nof = \"corp\"\nurl = \"file:///site/corp-mirror\"\npriority = 5\n";

fn json(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON report")
}

fn setup() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    fs::write(user.settings.join("site-profile.toml"), PROFILE).unwrap();
    fs::write(
        user.settings.join("registry.toml"),
        "[[registry]]\nname = \"mine\"\nurl = \"file:///home/me/registry\"\n",
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
         [[registry]]\nname = \"team\"\nurl = \"file:///team/registry\"\n",
    )
    .unwrap();
    (user, project)
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers"
)]
fn registry_list_shows_site_rows_with_site_scope() {
    let (user, project) = setup();
    let report = json(
        user.vibe()
            .args(["--json", "registry", "list", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows: Vec<(String, String)> = report["registries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap().to_string(),
                row["scope"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("team".to_string(), "project".to_string()),
            ("mine".to_string(), "user".to_string()),
            ("corp".to_string(), "site".to_string()),
        ]
    );

    let site = json(
        user.vibe()
            .args(["--json", "registry", "list", "--scope", "site", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows = site["registries"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "corp");
    assert_eq!(rows[0]["scope"], "site");
    assert_eq!(rows[0]["mirrors"][0]["url"], "file:///site/corp-mirror");

    // The site layer stays read-only for the editing commands.
    let refused = user
        .vibe()
        .args([
            "registry",
            "add",
            "x",
            "file:///x",
            "--scope",
            "site",
            "--path",
        ])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("vibe self apply"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn show_config_lists_every_layer_and_the_site_profile() {
    let (user, project) = setup();
    let report = json(
        user.vibe()
            .args(["--json", "show", "config", "--path"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let rows: Vec<(&str, &str)> = report["registries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap(),
                row["provenance"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("team", "vibe.toml"),
            ("mine", "~/.vibe/registry.toml"),
            ("corp", "site-profile.toml"),
        ]
    );
    let mirrors = report["mirrors"].as_array().unwrap();
    assert_eq!(mirrors.len(), 1);
    assert_eq!(mirrors[0]["provenance"], "site-profile.toml");
    assert_eq!(report["site_profile"]["loaded"], true);
    assert_eq!(report["site_profile"]["name"], "corp-site");
    assert!(
        report["site_profile"]["path"]
            .as_str()
            .unwrap()
            .ends_with("site-profile.toml")
    );

    let text = user
        .vibe()
        .args(["show", "config", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.contains(&format!(
            "[{}]",
            user.settings
                .join("site-profile.toml")
                .display()
                .to_string()
                .replace('\\', "/")
        )),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.trim_start().starts_with("site profile")
                && line.contains("corp-site")
                && line.trim_end().ends_with("[applied]")),
        "{text}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn show_config_reports_effective_layer_paths() {
    let (user, project) = setup();
    let ca = user.home.path().join("ca.pem");
    let second = user.home.path().join("second-ca.pem");
    // Synthetic declarations only: no certificate or network probe.
    let literal =
        |path: &std::path::Path| serde_json::to_string(&path.display().to_string()).unwrap();
    fs::write(
        user.settings.join("config.toml"),
        format!(
            "[tls]\nextra_ca = [{}, {}]\n[self]\nsource = 'github'\n",
            literal(&ca),
            literal(&second)
        ),
    )
    .unwrap();
    let run = |flags: &[&str]| {
        let output = user
            .vibe()
            .args(flags)
            .args(["show", "config", "--path"])
            .arg(project.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "show config must succeed");
        assert!(output.stderr.is_empty(), "stderr must stay empty");
        String::from_utf8(output.stdout).unwrap()
    };
    let plain = run(&["--plain"]);
    let verbose = run(&["--plain", "--verbose"]);
    let verbose_no_progress = run(&["--plain", "--verbose", "--no-progress"]);
    assert_eq!(
        verbose, verbose_no_progress,
        "report verbosity is independent of progress"
    );
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let shown = |path: &std::path::Path| path.display().to_string().replace('\\', "/");
    let compact = words(&plain);
    let ca_row = format!("tls extra_ca {} [user]", shown(&ca));
    let second_row = format!("tls extra_ca {} [user]", shown(&second));
    let proxy_row = "proxy mode none [default]";
    let release_row = "self source github [user]";
    for row in [&ca_row[..], &second_row, proxy_row, release_row] {
        assert_eq!(compact.matches(row).count(), 1, "effective row count");
    }
    assert!(compact.find(&ca_row) < compact.find(&second_row));
    assert!(compact.find(&second_row) < compact.find(proxy_row));
    assert!(compact.find(proxy_row) < compact.find(release_row));
    assert!(plain.contains(&format!(
        "[{}]",
        shown(&user.settings.join("registry.toml"))
    )));
    assert!(plain.contains(&format!("[{}]", shown(&project.path().join("vibe.toml")))));
    assert!(plain.contains(&format!(
        "[{}]",
        shown(&user.settings.join("site-profile.toml"))
    )));
    assert!(!plain.contains("~/.vibe/registry.toml"));
    let description = "Tracing filter (reads `tracing-subscriber::EnvFilter`).";
    assert!(!plain.contains(description));
    assert_eq!(verbose.matches(description).count(), 1);
    assert!(verbose.find("VIBE_LOG") < verbose.find(description));
    assert!(verbose.find(description) < verbose.find("VIBEVM_PUBLISH_TOKEN_GITHUB"));
    let machine: Value = serde_json::from_str(&run(&["--json"])).unwrap();
    let verbose_machine: Value = serde_json::from_str(&run(&["--json", "--verbose"])).unwrap();
    assert_eq!(machine, verbose_machine, "verbose must not change JSON");
    let no_progress_machine: Value =
        serde_json::from_str(&run(&["--json", "--verbose", "--no-progress"])).unwrap();
    assert_eq!(
        machine, no_progress_machine,
        "progress must not change JSON"
    );
    assert_eq!(
        machine["registries"][1]["provenance"],
        "~/.vibe/registry.toml"
    );
    assert_eq!(machine["mirrors"][0]["provenance"], "site-profile.toml");
    assert!(machine.get("tls").is_none());
    assert!(machine.get("proxy").is_none());
    assert!(machine.get("effective").is_none());
    let quiet = run(&["--quiet"]);
    assert_eq!(quiet, run(&["--quiet", "--verbose", "--no-progress"]));
    assert_eq!(quiet.lines().count(), 1);
    assert!(quiet.contains("3 registries, 1 mirror, 0 overrides, 5 env entries"));

    let site_ca = user.home.path().join("site-ca.pem");
    fs::write(
        user.settings.join("site-profile.toml"),
        format!(
            "{PROFILE}\n[tls]\nextra_ca = {}\n[proxy]\nmode = 'none'\n[self]\nsource = 'github'\n",
            literal(&site_ca)
        ),
    )
    .unwrap();
    fs::write(user.settings.join("config.toml"), "").unwrap();
    let inherited = words(&run(&["--plain"]));
    assert!(inherited.contains(&format!("tls extra_ca {} [site]", shown(&site_ca))));
    assert!(inherited.contains("proxy mode none [site]"));
    assert!(inherited.contains("self source github [site]"));
    fs::write(user.settings.join("config.toml"), format!(
        "[tls]\nextra_ca = [{0}, {1}]\n[proxy]\nca_file = {0}\nmode = 'none'\n[self]\nsource = 'github'\n",
        literal(&ca), literal(&second)
    )).unwrap();
    let replaced = words(&run(&["--plain"]));
    assert!(!replaced.contains(&shown(&site_ca)));
    assert_eq!(replaced.matches(&ca_row).count(), 1);
    assert_eq!(replaced.matches(&second_row).count(), 1);
    assert!(replaced.contains("proxy mode none [user]"));
    assert!(replaced.contains("self source github [user]"));
}
