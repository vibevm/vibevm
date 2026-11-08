//! In-process global MCP command integration with explicit isolated agent dirs.

use super::*;
use clap::Parser;
use rust_ai_native_env_audit::EnvGuard;
use std::fs;
use std::sync::Arc;
use vibe_agent_projection::agents::{Agent, AgentUserDirectories, Scope};
use vibe_core::manifest::{Lockfile, Materialization};
use vibe_core::progress::Progress;

use crate::cli::{AgentModeArg, Cli, Command};

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-SELECTIVE-REMOVAL"
);

#[cfg(test)]
struct Harness {
    _env: EnvGuard,
    root: tempfile::TempDir,
    directories: AgentUserDirectories,
}

#[cfg(test)]
impl Harness {
    fn new() -> Self {
        let mut env = EnvGuard::lock();
        let root = tempfile::tempdir().unwrap();
        let settings = root.path().join("settings");
        fs::create_dir_all(&settings).unwrap();
        env.set("VIBE_SETTINGS", settings.to_str().unwrap());
        env.set(
            "VIBE_REGISTRY_CACHE",
            root.path().join("cache").to_str().unwrap(),
        );
        env.set(
            "VIBEVM_SEARCH_CACHE_DIR",
            root.path().join("search-cache").to_str().unwrap(),
        );
        env.set("VIBE_NO_DEFAULT_REGISTRY", "1");
        env.unset("VIBE_UNATTENDED").unset("VIBE_INVOKED_BY");
        let directories = AgentUserDirectories {
            home: Some(root.path().join("home")),
            config: Some(root.path().join("config")),
        };
        Self {
            _env: env,
            root,
            directories,
        }
    }

    fn registry(&self) -> PathBuf {
        self.root.path().join("registry")
    }

    fn seed(&self, name: &str, version: &str) {
        let root = self
            .registry()
            .join("ai.lev")
            .join(name)
            .join(format!("v{version}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("vibe.toml"), format!(
            "[package]\nname='{name}'\ngroup='ai.lev'\nkind='mcp'\nversion='{version}'\n\
             [[mcp_server]]\nname='a'\ntransport='streamable-http'\nurl='https://example.org/a/{version}'\n\
             [[mcp_server]]\nname='b'\ntransport='streamable-http'\nurl='https://example.org/b/{version}'\n"
        )).unwrap();
    }

    fn run(&self, args: &[&str]) -> Result<()> {
        self.run_observed(args, None)
    }

    fn run_observed(&self, args: &[&str], observer: Option<Arc<Recorder>>) -> Result<()> {
        let mut argv = vec!["vibe".to_owned()];
        argv.extend(args.iter().map(|arg| (*arg).to_owned()));
        if matches!(args.first(), Some(&"install" | &"update")) {
            argv.extend([
                "--registry".to_owned(),
                self.registry().to_str().unwrap().to_owned(),
            ]);
        }
        let cli = Cli::try_parse_from(argv)?;
        let mut ctx = output::Context::from_flags(
            cli.quiet,
            cli.json,
            cli.invoked_by.as_deref(),
            cli.unattended,
            AgentModeArg::Cli,
        );
        ctx.agent_user_dirs = self.directories.clone();
        if let Some(observer) = observer {
            ctx = ctx
                .with_progress(
                    cli.verbose,
                    if cli.no_progress {
                        output::ProgressMode::Disabled
                    } else {
                        output::ProgressMode::Plain
                    },
                )
                .with_progress_scope(Progress::new(observer));
        }
        match cli.command {
            Command::Mcp(args) => crate::commands::mcp::run(
                &ctx,
                args,
                crate::commands::mcp::McpRuntime {
                    root_offline: true,
                    embedded_registry_root: None,
                    seed_default_registry: false,
                },
            ),
            Command::Install(args) => run_install(&ctx, args, None, true, false),
            Command::Uninstall(args) => run_uninstall(&ctx, args, true),
            Command::Update(args) => run_update(&ctx, args, None, true),
            _ => bail!("unsupported test command"),
        }
    }

    fn install(&self, agent: &str) {
        self.run(&[
            "install",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            agent,
            "--server",
            "a,b",
            "--assume-yes",
            "--json",
        ])
        .unwrap();
    }

    fn user_project(&self) -> PathBuf {
        self.root.path().join("settings/mcp/user-project")
    }

    fn slot(&self, version: &str) -> PathBuf {
        self.user_project()
            .join(format!("vibevm/vibedeps/ai.lev.fpf/{version}/vibe.toml"))
    }

    fn config(&self, agent: Agent) -> PathBuf {
        let path = agent
            .config_path_with_dirs(Scope::User, None, &self.directories)
            .unwrap()
            .unwrap();
        assert!(path.starts_with(self.root.path()));
        path
    }

    fn receipt(&self, agent: Agent) -> PathBuf {
        let config = self.config(agent);
        config.with_file_name(format!(
            "{}.vibevm-mcp.json",
            config.file_name().unwrap().to_str().unwrap()
        ))
    }

    fn removal_deletes_package(&self, agents: &[Agent]) -> bool {
        let mut context = output::Context::from_flags(false, true, None, false, AgentModeArg::Cli);
        context.agent_user_dirs = self.directories.clone();
        let plan =
            mcp::preflight_unregister_global_package(&context, "mcp:ai.lev/fpf", agents).unwrap();
        mcp::removes_global_package(&plan)
    }

    fn names(&self, agent: Agent) -> Vec<String> {
        let doc: serde_json::Value = if agent == Agent::Codex {
            let value: toml::Value =
                toml::from_str(&fs::read_to_string(self.config(agent)).unwrap()).unwrap();
            serde_json::to_value(value).unwrap()
        } else {
            serde_json::from_slice(&fs::read(self.config(agent)).unwrap()).unwrap()
        };
        doc.get(agent.mcp_section_key())
            .and_then(serde_json::Value::as_object)
            .map(|entries| entries.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[test]
#[specmark::verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-SELECTIVE-REMOVAL"
)]
fn selected_agent_removes_both_servers_retains_other_agent_and_last_removes_slot() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("codex");
    harness.install("cursor");
    let cursor_before = fs::read(harness.config(Agent::Cursor)).unwrap();
    assert!(!harness.removal_deletes_package(&[Agent::Codex]));
    harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex",
            "--json",
        ])
        .unwrap();
    assert!(harness.names(Agent::Codex).is_empty());
    assert_eq!(harness.names(Agent::Cursor), ["a", "b"]);
    assert_eq!(
        fs::read(harness.config(Agent::Cursor)).unwrap(),
        cursor_before
    );
    assert!(harness.slot("1.0.0").exists());
    assert!(harness.removal_deletes_package(&[Agent::Cursor]));
    harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "cursor",
            "--json",
        ])
        .unwrap();
    assert!(!harness.slot("1.0.0").exists());
    assert!(!harness.receipt(Agent::Cursor).exists());
}

#[test]
fn explicit_all_json_unattended_and_assume_yes_remove_every_owned_registration() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    for flags in [
        vec!["--agent", "all", "--json"],
        vec!["--json"],
        vec!["--unattended", "--json"],
        vec!["--assume-yes", "--quiet"],
    ] {
        harness.install("codex");
        harness.install("cursor");
        let mut args = vec!["uninstall", "-g", "mcp:ai.lev/fpf"];
        args.extend(flags);
        harness.run(&args).unwrap();
        assert!(harness.names(Agent::Codex).is_empty());
        assert!(harness.names(Agent::Cursor).is_empty());
        assert!(!harness.slot("1.0.0").exists());
    }
}

#[test]
fn bad_agent_path_and_version_refuse_without_config_or_inventory_changes() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("codex");
    harness.install("cursor");
    let before = fs::read(harness.config(Agent::Codex)).unwrap();
    let receipt = fs::read(harness.receipt(Agent::Codex)).unwrap();
    for flags in [
        vec!["--agent", "qwen"],
        vec!["--agent", "unknown"],
        vec!["--agent", "codex", "--path", "other"],
    ] {
        let mut args = vec!["uninstall", "-g", "mcp:ai.lev/fpf", "--json"];
        args.extend(flags);
        assert!(harness.run(&args).is_err());
        assert_eq!(fs::read(harness.config(Agent::Codex)).unwrap(), before);
        assert_eq!(fs::read(harness.receipt(Agent::Codex)).unwrap(), receipt);
        assert!(harness.slot("1.0.0").exists());
    }
    assert!(
        harness
            .run(&[
                "uninstall",
                "-g",
                "mcp:ai.lev/fpf@=1.0.0",
                "--agent",
                "codex",
                "--json"
            ])
            .is_err()
    );
    assert_eq!(fs::read(harness.config(Agent::Codex)).unwrap(), before);
}

#[test]
fn final_package_guard_failure_restores_exact_config_and_receipt_bytes() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("codex");
    let config = fs::read(harness.config(Agent::Codex)).unwrap();
    let receipt = fs::read(harness.receipt(Agent::Codex)).unwrap();
    let lock_path = harness.user_project().join(Lockfile::FILENAME);
    let mut lock = Lockfile::read(&lock_path).unwrap();
    lock.packages
        .iter_mut()
        .find(|package| package.name == "fpf")
        .unwrap()
        .materialization = Materialization::InPlace;
    lock.write(lock_path).unwrap();
    let error = harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex",
            "--json",
        ])
        .unwrap_err();
    assert!(format!("{error:#}").contains("materialised in-place"));
    assert_eq!(fs::read(harness.config(Agent::Codex)).unwrap(), config);
    assert_eq!(fs::read(harness.receipt(Agent::Codex)).unwrap(), receipt);
    assert!(harness.slot("1.0.0").exists());
}

#[test]
fn quiet_human_subset_retains_referenced_package() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("codex");
    harness.install("cursor");
    harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex",
            "--quiet",
        ])
        .unwrap();
    assert!(harness.names(Agent::Codex).is_empty());
    assert_eq!(harness.names(Agent::Cursor), ["a", "b"]);
    assert!(harness.slot("1.0.0").exists());
}

#[test]
fn server_subset_update_preserves_registered_names_and_invalid_selector_is_atomic() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness
        .run(&[
            "install",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex",
            "--server",
            "a",
            "--json",
        ])
        .unwrap();
    harness
        .run(&[
            "install",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "cursor",
            "--server",
            "a,b",
            "--json",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::Codex), ["a"]);
    assert_eq!(harness.names(Agent::Cursor), ["a", "b"]);
    let before = fs::read(harness.config(Agent::Codex)).unwrap();
    let receipt_before = fs::read(harness.receipt(Agent::Codex)).unwrap();
    let manifest_before = fs::read(harness.user_project().join("vibe.toml")).unwrap();
    let lock_before = Lockfile::read(harness.user_project().join("vibe.lock")).unwrap();
    assert!(
        harness
            .run(&[
                "install",
                "-g",
                "mcp:ai.lev/fpf",
                "--agent",
                "codex",
                "--server",
                "a,missing",
                "--json"
            ])
            .is_err()
    );
    assert_eq!(fs::read(harness.config(Agent::Codex)).unwrap(), before);
    assert_eq!(
        fs::read(harness.receipt(Agent::Codex)).unwrap(),
        receipt_before
    );
    assert_eq!(
        fs::read(harness.user_project().join("vibe.toml")).unwrap(),
        manifest_before
    );
    // Package installation completes before registration selection. Its lock
    // timestamp may refresh; package identity and registration effects must not.
    let mut lock_after = Lockfile::read(harness.user_project().join("vibe.lock")).unwrap();
    lock_after.meta.generated_at = lock_before.meta.generated_at.clone();
    assert_eq!(
        serde_json::to_value(lock_after).unwrap(),
        serde_json::to_value(lock_before).unwrap()
    );
    harness.seed("fpf", "1.0.1");
    let package_manifest = harness.registry().join("ai.lev/fpf/v1.0.1/vibe.toml");
    let mut declaration = fs::read_to_string(&package_manifest).unwrap();
    declaration.push_str(
        "\n[[mcp_server]]\nname='c'\ntransport='streamable-http'\nurl='https://example.org/c'\n",
    );
    fs::write(package_manifest, declaration).unwrap();
    harness
        .run(&["update", "-g", "mcp:ai.lev/fpf", "--assume-yes", "--json"])
        .unwrap();
    assert_eq!(harness.names(Agent::Codex), ["a"]);
    assert_eq!(harness.names(Agent::Cursor), ["a", "b"]);
    assert!(
        fs::read_to_string(harness.config(Agent::Codex))
            .unwrap()
            .contains("/1.0.1")
    );
    assert!(harness.slot("1.0.1").exists());
}

#[test]
fn install_without_agent_and_application_agent_flag_refuse_before_mutation() {
    let harness = Harness::new();
    let error = harness
        .run(&["install", "-g", "mcp:ai.lev/fpf", "--json"])
        .unwrap_err();
    assert!(error.to_string().contains("--agent"));
    assert!(!harness.user_project().exists());
    assert!(
        harness
            .run(&[
                "install",
                "-g",
                "mcp:ai.lev/fpf",
                "--agent",
                "codex,unknown",
                "--server",
                "a",
                "--json"
            ])
            .is_err()
    );
    assert!(!harness.user_project().exists());
    assert!(
        harness
            .run(&[
                "uninstall",
                "-g",
                "org.example/demo",
                "--agent",
                "codex",
                "--json"
            ])
            .is_err()
    );
    assert!(!harness.root.path().join("settings/applications").exists());
    assert!(
        Cli::try_parse_from(["vibe", "uninstall", "mcp:ai.lev/fpf", "--agent", "codex"]).is_err()
    );
}

#[test]
fn explicit_agent_subset_selects_only_named_clients() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness
        .run(&[
            "install",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex,cursor",
            "--server",
            "a,b",
            "--json",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::Codex), ["a", "b"]);
    assert_eq!(harness.names(Agent::Cursor), ["a", "b"]);
    assert!(!harness.config(Agent::QwenCode).exists());
    harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "cursor,codex",
            "--json",
        ])
        .unwrap();
    assert!(!harness.slot("1.0.0").exists());
}

#[test]
fn ownership_drift_refuses_all_agents_without_partial_removal() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    harness.install("codex");
    harness.install("cursor");
    let cursor_before = fs::read(harness.config(Agent::Cursor)).unwrap();
    let codex = harness.config(Agent::Codex);
    let changed = fs::read_to_string(&codex)
        .unwrap()
        .replace("https://example.org/a/1.0.0", "https://changed.example/a");
    fs::write(&codex, &changed).unwrap();
    let error = harness
        .run(&[
            "uninstall",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "all",
            "--json",
        ])
        .unwrap_err();
    assert!(format!("{error:#}").contains("changed outside vibe"));
    assert_eq!(fs::read_to_string(codex).unwrap(), changed);
    assert_eq!(
        fs::read(harness.config(Agent::Cursor)).unwrap(),
        cursor_before
    );
    assert!(harness.slot("1.0.0").exists());
}

#[cfg(test)]
#[path = "tests/progress.rs"]
mod progress;

use progress::Recorder;

#[test]
fn human_global_url_install_and_update_need_no_materialization_confirmation() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    let parsed = Cli::try_parse_from([
        "vibe",
        "install",
        "-g",
        "mcp:ai.lev/fpf",
        "--agent",
        "codex",
        "--server",
        "a",
    ])
    .unwrap();
    let Command::Install(args) = parsed.command else {
        panic!("install arguments")
    };
    assert!(!args.assume_yes);
    assert!(!parsed.unattended);
    assert!(!parsed.json);
    harness
        .run(&[
            "install",
            "-g",
            "mcp:ai.lev/fpf",
            "--agent",
            "codex",
            "--server",
            "a",
        ])
        .unwrap();
    assert_eq!(harness.names(Agent::Codex), ["a"]);
    assert!(!harness.config(Agent::Cursor).exists());
    harness.seed("fpf", "1.0.1");
    harness.run(&["update", "-g", "mcp:ai.lev/fpf"]).unwrap();
    assert_eq!(harness.names(Agent::Codex), ["a"]);
    assert!(harness.slot("1.0.1").exists());
}

#[cfg(test)]
#[path = "tests/activation.rs"]
mod activation;
