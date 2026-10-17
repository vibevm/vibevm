//! `vibe show features` — active feature lines per locked package.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::Result;
use serde::Serialize;

use crate::cli::ShowFeaturesArgs;
use crate::output;

use super::resolve_project_root;

// ===================== show features =====================

#[derive(Debug, Serialize)]
struct FeaturesEntry {
    package: String,
    features: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FeaturesReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<FeaturesEntry>,
    /// Total active feature lines, project-wide.
    total: usize,
}

pub(super) fn run_features(ctx: &output::Context, args: ShowFeaturesArgs) -> Result<()> {
    if args.discovery_host.is_some() {
        // Query the host's held admission, before any project/disk preparation.
        let report = query_discovery_host(&args)?;
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(&serde_json::to_string_pretty(&report)?);
        }
        return Ok(());
    }
    let project_root = resolve_project_root(&args.path)?;
    if args.discover {
        if !args.prepare {
            // An unconfigured standalone query has no normalized authority.
            // It must not turn a query into a new disk preparation.
            let report = vibe_workspace::configuration_discovery::discover([]);
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else {
                ctx.summary(&serde_json::to_string_pretty(&report)?);
            }
            return Ok(());
        }
        let capture = vibe_orchestrator::document_analysis::prepare_configuration_discovery(
            &project_root,
            &args.cached_manifest,
        )?;
        let basis = vibe_orchestrator::document_analysis::PreparedConfigurationDiscovery::admit(
            capture,
            vec![],
        );
        let query_args = ShowFeaturesArgs {
            discover: true,
            prepare: false,
            cached_manifest: vec![],
            discovery_host: None,
            capture: None,
            path: project_root.clone(),
        };
        let report = query_prepared_features(&query_args, &basis)?;
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(&serde_json::to_string_pretty(&report)?);
        }
        return Ok(());
    }
    let lockfile_path = project_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        vibe_core::manifest::Lockfile::read(&lockfile_path)?
    } else {
        vibe_core::manifest::Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        )
    };

    let (entries, total) = locked_features(&lockfile);

    if ctx.is_json() {
        ctx.emit_json(&FeaturesReport {
            ok: true,
            command: "show:features",
            project: project_root.display().to_string(),
            packages: entries,
            total,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show features: {} active feature{} across {} package{}",
            total,
            if total == 1 { "" } else { "s" },
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if entries.is_empty() {
        ctx.summary("(no features active in this project)");
        return Ok(());
    }
    for e in &entries {
        ctx.heading(&e.package);
        for f in &e.features {
            ctx.step(f);
        }
    }
    ctx.summary(&format!(
        "\n{} active feature{} across {} package{}",
        total,
        if total == 1 { "" } else { "s" },
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

/// The endpoint is an explicit trusted host input, like the typed MCP host
/// constructor. Its response is presentation only, never install authority.
fn query_discovery_host(args: &ShowFeaturesArgs) -> Result<serde_json::Value> {
    anyhow::ensure!(
        args.discover && !args.prepare && args.cached_manifest.is_empty(),
        "host query requires discovery without preparation"
    );
    let endpoint = args
        .discovery_host
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing discovery host"))?;
    let capture = args
        .capture
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing capture"))?;
    #[cfg(unix)]
    {
        use std::io::{Read, Write};
        let mut connection = std::os::unix::net::UnixStream::connect(endpoint)?;
        connection.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        connection.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
        serde_json::to_writer(
            &mut connection,
            &serde_json::json!({"capture":capture,"root":args.path}),
        )?;
        connection.flush()?;
        connection.shutdown(std::net::Shutdown::Write)?;
        let report: serde_json::Value = serde_json::from_reader(connection.take(64 * 1024 * 1024))?;
        anyhow::ensure!(
            report["capture_identity"].as_str() == Some(capture.as_str()),
            "discovery host returned a different capture"
        );
        Ok(report)
    }
    #[cfg(not(unix))]
    {
        let _ = (endpoint, capture);
        anyhow::bail!("local discovery host transport requires Unix sockets")
    }
}

fn locked_features(lockfile: &vibe_core::manifest::Lockfile) -> (Vec<FeaturesEntry>, usize) {
    let entries: Vec<_> = lockfile
        .packages
        .iter()
        .filter(|p| !p.features.is_empty())
        .map(|p| FeaturesEntry {
            package: format!("{}/{}", p.group, p.name),
            features: p.features.clone(),
        })
        .collect();
    let total = entries.iter().map(|e| e.features.len()).sum();
    (entries, total)
}

/// CLI adapter query over an explicitly prepared basis. Arguments are checked
/// without resolving paths or reading disk; repeat queries retain this basis.
pub(crate) fn query_prepared_features<'a>(
    args: &ShowFeaturesArgs,
    basis: &'a vibe_orchestrator::document_analysis::PreparedConfigurationDiscovery,
) -> Result<&'a vibe_workspace::configuration_discovery::ConfigurationDiscovery> {
    anyhow::ensure!(args.discover, "prepared discovery requires --discover");
    anyhow::ensure!(
        !args.prepare,
        "preparation is separate from a prepared query"
    );
    anyhow::ensure!(args.path == basis.root(), "prepared discovery root differs");
    anyhow::ensure!(
        args.cached_manifest.is_empty(),
        "cached manifest paths belong to explicit preparation"
    );
    Ok(basis.query())
}

#[cfg(test)]
use vibe_orchestrator::document_analysis as discovery_api;
#[cfg(test)]
#[path = "../../../../vibe-orchestrator/src/document_analysis/configuration_discovery_fixture.rs"]
mod discovery_fixture;
#[cfg(test)]
mod m_05_e_1_cases {
    use super::discovery_fixture::*;
    use super::*;
    use clap::Parser;
    fn args(root: &std::path::Path) -> ShowFeaturesArgs {
        let cli = crate::cli::Cli::try_parse_from([
            "vibe",
            "show",
            "features",
            "--discover",
            "--path",
            root.to_str().unwrap(),
        ])
        .unwrap();
        let crate::cli::Command::Show(show) = cli.command else {
            panic!("show command")
        };
        let crate::cli::ShowSubcommand::Features(args) = show.command else {
            panic!("features command")
        };
        args
    }
    #[test]
    fn discovery_renders_candidates_without_selection() {
        let (dir, capture, admission) = fixture();
        let basis = discovery_api::PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        let report = query_prepared_features(&args(dir.path()), &basis).unwrap();
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["status"], "needs_selection");
        assert_eq!(json["candidates"][0]["coordinate"], "org.demo/demo");
        assert_eq!(
            json["candidates"][0]["definition"]["constraints"]["groups"]["mode"]["defaults"][0],
            "red"
        );
        assert_eq!(json["counters"]["solves"], 0);
    }
    #[test]
    fn empty_cached_discovery_unavailable() {
        let (dir, capture, _) = fixture();
        let basis = discovery_api::PreparedConfigurationDiscovery::admit(capture, vec![]);
        assert!(
            crate::cli::Cli::try_parse_from([
                "vibe",
                "show",
                "features",
                "--discover",
                "--cached-manifest",
                "cache/vibe.toml"
            ])
            .is_err()
        );
        assert!(
            crate::cli::Cli::try_parse_from(["vibe", "show", "features", "--prepare"]).is_err()
        );
        assert_eq!(
            query_prepared_features(&args(dir.path()), &basis)
                .unwrap()
                .status,
            "unavailable"
        );
    }
    #[test]
    fn repeated_unresolved_terminates() {
        use vibe_mcp::McpTool;
        let (dir, capture, admission) = fixture();
        let basis = std::sync::Arc::new(discovery_api::PreparedConfigurationDiscovery::admit(
            capture,
            vec![admission],
        ));
        let tool =
            vibe_mcp::tools::configuration_discovery::ConfigurationDiscoveryMcpTool::prepared(
                basis.clone(),
            );
        let args = args(dir.path());
        let first = serde_json::to_value(query_prepared_features(&args, &basis).unwrap()).unwrap();
        assert_eq!(first["status"], "needs_selection");
        std::fs::write(dir.path().join("cache/vibe.toml"), "broken after freeze").unwrap();
        let read = || {
            ["vibe.toml", "vibe.lock", "cache/vibe.toml"]
                .map(|p| std::fs::read(dir.path().join(p)).unwrap())
        };
        let before = read();
        for _ in 0..2 {
            let cli =
                serde_json::to_value(query_prepared_features(&args, &basis).unwrap()).unwrap();
            let mcp = tool
                .run(
                    &serde_json::json!({"capture":basis.capture_id()}),
                    &vibe_mcp::ServerContext::new(dir.path()),
                )
                .unwrap();
            assert_eq!(cli, first);
            assert_eq!(mcp.structured(), &first);
            assert_eq!(read(), before);
        }
        assert!(
            tool.run(
                &serde_json::json!({"capture":"foreign"}),
                &vibe_mcp::ServerContext::new(dir.path())
            )
            .is_err()
        );
    }
    #[test]
    fn legacy_locked_feature_view_preserved() {
        let mut lock = vibe_core::manifest::Lockfile::empty("E1", "2026-10-17T00:00:00Z");
        lock.packages.push(serde_json::from_value(serde_json::json!({
            "kind":"flow", "group":"org.demo", "name":"demo", "version":"1.0.0",
            "registry":"fixture", "source_url":"https://example.invalid/fixture",
            "content_hash":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "features":["extra"], "dependencies":[], "files_written":[], "subskills_active":[], "overridden":false
        })).unwrap());
        let (rows, total) = locked_features(&lock);
        assert_eq!(total, 1);
        assert_eq!(rows[0].package, "org.demo/demo");
        assert_eq!(rows[0].features, ["extra"]);
    }

    #[cfg(unix)]
    #[test]
    fn production_cli_admitted_host_output() {
        // A child test process captures the real command's stdout without
        // replacing its emitter or adding a second presentation path.
        if let Some(socket) = std::env::var_os("M05E1_TEST_HOST_SOCKET") {
            let cli = crate::cli::Cli::try_parse_from([
                "vibe",
                "--json",
                "show",
                "features",
                "--discover",
                "--discovery-host",
                socket.to_str().unwrap(),
                "--capture",
                &std::env::var("M05E1_TEST_CAPTURE").unwrap(),
                "--path",
                &std::env::var("M05E1_TEST_ROOT").unwrap(),
            ])
            .unwrap();
            let crate::cli::Command::Show(show) = cli.command else {
                panic!("show")
            };
            let ctx = output::Context::from_flags(
                false,
                true,
                None,
                false,
                crate::cli::AgentModeArg::Auto,
            );
            crate::commands::show::run(&ctx, show, true).unwrap();
            return;
        }
        use vibe_mcp::McpTool;
        let (dir, capture, admission) = fixture();
        let basis = std::sync::Arc::new(discovery_api::PreparedConfigurationDiscovery::admit(
            capture,
            vec![admission],
        ));
        let tool =
            vibe_mcp::tools::configuration_discovery::ConfigurationDiscoveryMcpTool::prepared(
                basis.clone(),
            );
        let expected = tool
            .run(
                &serde_json::json!({"capture":basis.capture_id()}),
                &vibe_mcp::ServerContext::new(dir.path()),
            )
            .unwrap();
        let socket_dir = tempfile::Builder::new()
            .prefix("m05e1-")
            .tempdir_in("/tmp")
            .unwrap();
        let socket = socket_dir.path().join("query.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let held = basis.clone();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut connection, _) = listener.accept().unwrap();
                held.serve_cli_query(&mut connection).unwrap();
            }
        });
        std::fs::write(dir.path().join("cache/vibe.toml"), "broken after freeze").unwrap();
        let read = || {
            ["vibe.toml", "vibe.lock", "cache/vibe.toml"]
                .map(|p| std::fs::read(dir.path().join(p)).unwrap())
        };
        let before = read();
        for _ in 0..2 {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "commands::show::features::m_05_e_1_cases::production_cli_admitted_host_output",
                    "--nocapture",
                ])
                .env("M05E1_TEST_HOST_SOCKET", &socket)
                .env("M05E1_TEST_CAPTURE", basis.capture_id())
                .env("M05E1_TEST_ROOT", dir.path())
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "{}",
                String::from_utf8_lossy(&child.stderr)
            );
            let stdout = String::from_utf8(child.stdout).unwrap();
            let report: serde_json::Value = serde_json::from_str(
                &stdout[stdout.find('{').unwrap()..=stdout.rfind('}').unwrap()],
            )
            .unwrap();
            assert_eq!(&report, expected.structured());
            assert_eq!(report["status"], "needs_selection");
            assert_eq!(
                report["candidates"][0]["definition"]["constraints"]["groups"]["mode"]["defaults"]
                    [0],
                "red"
            );
            assert_eq!(
                report["candidates"][0]["definition"]["program"]["features"]["red"]
                    .as_array()
                    .unwrap()
                    .len(),
                3
            );
            assert_eq!(read(), before);
        }
        server.join().unwrap();
    }
}
