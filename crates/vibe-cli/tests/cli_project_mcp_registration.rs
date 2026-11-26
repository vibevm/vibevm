//! Project installation grants registration only through explicit addressed intent.
use std::{
    fs,
    process::{Command, Output, Stdio},
};

struct Site {
    root: tempfile::TempDir,
}
impl Site {
    fn new() -> Self {
        let site = Self {
            root: tempfile::tempdir().unwrap(),
        };
        for dir in ["project", "home", "settings", "registry"] {
            fs::create_dir_all(site.path(dir)).unwrap();
        }
        fs::write(
            site.path("project/vibe.toml"),
            "[project]\nname='intent'\ngroup='org.test'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            site.path("project/.mcp.json"),
            "{\"mcpServers\":{\"foreign\":{\"command\":\"untouched\"}}}\n",
        )
        .unwrap();
        fs::write(site.path("home/.claude.json"), "{\"sentinel\":true}\n").unwrap();
        for (name, kind) in [("server", "mcp"), ("ordinary", "flow")] {
            let slot = site.path(&format!("registry/org.test/{name}/v1.0.0"));
            fs::create_dir_all(&slot).unwrap();
            let server = if kind == "mcp" {
                "\n[[mcp_server]]\nname='expected'\ntransport='streamable-http'\nurl='https://127.0.0.1:9/mcp'\n"
            } else {
                ""
            };
            fs::write(slot.join("vibe.toml"), format!("[package]\nname='{name}'\ngroup='org.test'\nkind='{kind}'\nversion='1.0.0'\n{server}")).unwrap();
        }
        site
    }
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.root.path().join(name)
    }
    fn install(&self, flags: &[&str]) -> Output {
        self.install_with(&["mcp:org.test/server", "flow:org.test/ordinary"], flags)
    }
    fn install_with(&self, packages: &[&str], flags: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .current_dir(self.path("project"))
            .arg("install")
            .args(packages)
            .args(["--offline", "--assume-yes", "--registry"])
            .arg(self.path("registry"))
            .args(flags)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", self.path("home"))
            .env("USERPROFILE", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("NO_COLOR", "1")
            .env("CI", "1")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
    fn config(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.path("project/.mcp.json")).unwrap()).unwrap()
    }
}
fn unchanged(site: &Site, before: &[u8]) {
    assert_eq!(fs::read(site.path("project/.mcp.json")).unwrap(), before);
    assert_eq!(
        fs::read(site.path("home/.claude.json")).unwrap(),
        b"{\"sentinel\":true}\n"
    );
}
#[test]
fn project_install_mcp_registration_is_explicit_and_addressed() {
    let site = Site::new();
    let result = site.install(&["--register-mcp", "org.test/server"]);
    assert!(
        result.status.success(),
        "explicit project registration failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let config = site.config();
    assert_eq!(config["mcpServers"]["foreign"]["command"], "untouched");
    assert!(config["mcpServers"]["expected"].is_object());
    assert_eq!(config["mcpServers"].as_object().unwrap().len(), 2);
    assert_eq!(
        fs::read(site.path("home/.claude.json")).unwrap(),
        b"{\"sentinel\":true}\n"
    );
}
#[test]
fn project_install_mcp_registration_absence_never_grants_consent() {
    for flags in [&[][..], &["--unattended"][..]] {
        let site = Site::new();
        let before = fs::read(site.path("project/.mcp.json")).unwrap();
        assert!(site.install(flags).status.success());
        unchanged(&site, &before);
    }
}
#[test]
fn project_install_mcp_registration_collision_refuses_before_publication() {
    let site = Site::new();
    let slot = site.path("registry/org.test/other/v1.0.0");
    fs::create_dir_all(&slot).unwrap();
    fs::write(slot.join("vibe.toml"), "[package]\nname='other'\ngroup='org.test'\nkind='mcp'\nversion='1.0.0'\n[[mcp_server]]\nname='expected'\ntransport='streamable-http'\nurl='https://127.0.0.1:9/other'\n").unwrap();
    let before = fs::read(site.path("project/.mcp.json")).unwrap();
    let result = site.install_with(
        &["mcp:org.test/server", "mcp:org.test/other"],
        &[
            "--register-mcp",
            "org.test/server",
            "--register-mcp",
            "org.test/other",
        ],
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("requested MCP packages collide"),
        "expected project destination collision"
    );
    unchanged(&site, &before);
}
#[test]
fn project_install_mcp_registration_json_retains_final_install_report() {
    let baseline = Site::new();
    let plain = baseline.install(&["--json"]);
    assert!(plain.status.success());
    let parse = |bytes: &[u8]| {
        serde_json::Deserializer::from_slice(bytes)
            .into_iter::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .expect("complete JSON document stream")
    };
    let plain_docs = parse(&plain.stdout);
    let site = Site::new();
    let result = site.install(&["--register-mcp", "org.test/server", "--json"]);
    assert!(result.status.success());
    let docs = parse(&result.stdout);
    let commands: Vec<_> = docs.iter().map(|doc| doc["command"].clone()).collect();
    assert_eq!(
        commands,
        plain_docs
            .iter()
            .map(|doc| doc["command"].clone())
            .collect::<Vec<_>>()
    );
    assert!(
        commands.iter().all(|command| matches!(
            command.as_str(),
            Some("install:plan" | "lifecycle:plan" | "install:closure-diff" | "install")
        )),
        "unexpected document type"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| command.as_str() == Some("install"))
            .count(),
        1
    );
    assert_eq!(
        commands.last().and_then(|command| command.as_str()),
        Some("install")
    );
    let report = docs.last().expect("final install report");
    let rows = report["contributions"]
        .as_array()
        .expect("typed install contributions");
    assert_eq!(
        rows.iter()
            .filter(
                |row| row["key"] == "project-mcp-registration:org.test/server"
                    && row["status"] == "ok"
            )
            .count(),
        1
    );
    assert!(site.config()["mcpServers"]["expected"].is_object());
}
#[test]
fn project_install_mcp_registration_preflights_all_selectors() {
    for bad in ["server", "org.test/absent", "org.test/ordinary"] {
        let site = Site::new();
        let before = fs::read(site.path("project/.mcp.json")).unwrap();
        let result = site.install(&["--register-mcp", "org.test/server", "--register-mcp", bad]);
        assert!(!result.status.success());
        let reason = String::from_utf8_lossy(&result.stderr);
        let expected = if bad == "server" {
            "--register-mcp requires an unversioned GROUP/NAME"
        } else {
            "requested project MCP package is absent, ambiguous, or not MCP"
        };
        assert!(reason.contains(expected), "expected selector refusal");
        unchanged(&site, &before);
    }
}

/// The historical optional script fails inside registration, not at the outer
/// install boundary. The addressed replacement succeeds on the same non-TTY
/// fixture without building the real Rust toolchain packages.
#[cfg(unix)]
#[test]
fn closed_stdin_distinguishes_the_1_0_12_registration_script_from_addressed_intent() {
    let site = Site::new();
    let slot = site.path("registry/org.test/server/v1.0.0");
    fs::create_dir_all(slot.join("hooks")).unwrap();
    fs::write(
        slot.join("hooks/register.sh"),
        include_str!("../../../vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-mcp/v1.0.12/tooling/post-install/register-mcp.sh"),
    ).unwrap();
    let manifest_path = slot.join("vibe.toml");
    let original = fs::read_to_string(&manifest_path).unwrap();
    fs::write(&manifest_path, format!("{original}\n[[extension]]\nid='register-mcp'\npoint='slot:post-install'\nhandler={{kind='script',base='hooks/register'}}\noptional=true\napplies_to={{packages=['org.test/server']}}\n")).unwrap();
    let before = fs::read(site.path("project/.mcp.json")).unwrap();
    let result = site.install_with(&["mcp:org.test/server"], &["--json"]);
    assert!(
        result.status.success(),
        "optional hook does not fail the install: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    unchanged(&site, &before);
    assert!(site.config()["mcpServers"].get("expected").is_none());
    let docs = serde_json::Deserializer::from_slice(&result.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let report = docs.last().unwrap();
    assert_eq!(report["command"], "install");
    let rows = report["contributions"].as_array().unwrap();
    let registration = rows
        .iter()
        .filter(|row| {
            row["key"]
                .as_str()
                .is_some_and(|key| key.contains("#register-mcp@"))
        })
        .collect::<Vec<_>>();
    assert_eq!(registration.len(), 1);
    assert_eq!(registration[0]["status"], "fail");
    assert_eq!(registration[0]["flagged"], true);
    let reason = registration[0]["stderr"].as_str().unwrap();
    assert!(
        reason.contains("no --scope and stdin is not a TTY"),
        "{reason}"
    );
    assert!(
        reason.contains("--scope project|user|both") && reason.contains("--auto"),
        "{reason}"
    );

    let addressed = Site::new();
    let result = addressed.install_with(
        &["mcp:org.test/server"],
        &["--register-mcp", "org.test/server", "--json"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let docs = serde_json::Deserializer::from_slice(&result.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let report = docs.last().unwrap();
    assert_eq!(report["command"], "install");
    let rows = report["contributions"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .filter(
                |row| row["key"] == "project-mcp-registration:org.test/server"
                    && row["status"] == "ok"
            )
            .count(),
        1
    );
    assert!(addressed.config()["mcpServers"]["expected"].is_object());
    assert_eq!(
        addressed.config()["mcpServers"]["foreign"]["command"],
        "untouched"
    );
    assert_eq!(
        fs::read(addressed.path("home/.claude.json")).unwrap(),
        b"{\"sentinel\":true}\n"
    );
}
