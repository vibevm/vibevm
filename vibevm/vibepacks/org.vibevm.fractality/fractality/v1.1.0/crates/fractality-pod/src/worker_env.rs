//! Phase 4 worker-environment seams: the nesting injections and the
//! post-kill orphan audit.

specmark::scope!("spec://fractality/PROP-001#invariants");

/// Deliberate composition-root injections on top of the clean-slate env
/// (I1 intact — this is construction, not inheritance): the worker gets
/// `FRACTALITY_HOME` so its own `fractality` calls land on THIS
/// mission-control (a scratch-home run must not leak spawns into the
/// real home, F16), and the pod's directory joins the PATH head so the
/// worker resolves the same fractality build that supervises it.
pub(crate) fn augment_worker_env(
    env: &mut std::collections::BTreeMap<String, String>,
    home: &camino::Utf8Path,
    pod_exe_dir: Option<&std::path::Path>,
) {
    env.insert("FRACTALITY_HOME".to_owned(), home.to_string());
    if let Some(dir) = pod_exe_dir {
        let dir = dir.to_string_lossy();
        let sep = if cfg!(windows) { ';' } else { ':' };
        match env.get_mut("PATH") {
            Some(path) => *path = format!("{dir}{sep}{path}"),
            None => {
                env.insert("PATH".to_owned(), dir.into_owned());
            }
        }
    }
}

/// Materializes the ask_boss MCP config (D18 layer 3): the invocation
/// names this file; the broker it launches is this build's own CLI
/// binary, resolved beside the pod. A missing sibling binary degrades
/// loudly to a broker-less worker rather than failing the run.
pub(crate) fn write_mcp_config(
    path: &camino::Utf8Path,
    pod_exe_dir: Option<&std::path::Path>,
) -> Result<(), String> {
    let bin_name = if cfg!(windows) {
        "fractality.exe"
    } else {
        "fractality"
    };
    let bin = pod_exe_dir.map(|d| d.join(bin_name));
    match bin.filter(|p| p.is_file()) {
        Some(bin) => {
            let config = serde_json::json!({
                "mcpServers": {
                    "fractality": {
                        "command": bin.to_string_lossy(),
                        "args": ["mcp-broker"],
                    },
                },
            });
            std::fs::write(
                path.as_std_path(),
                serde_json::to_string_pretty(&config)
                    .map_err(|e| format!("encoding mcp config: {e}"))?,
            )
            .map_err(|e| format!("writing `{path}`: {e}"))
        }
        None => {
            tracing::warn!(
                "ask_boss requested but no fractality binary beside the pod; \
                 the worker runs without the broker"
            );
            Ok(())
        }
    }
}

/// Post-kill orphan sweep (P5): probes the worker pid and reports. The
/// Job Object close is an OS guarantee — this is the trust-but-verify
/// log line the manual test cites.
pub(crate) fn sweep_for_orphan(worker_pid: u32) {
    let mut system = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(worker_pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
    match system.process(target) {
        Some(_) => tracing::error!(
            worker_pid,
            "orphan sweep: worker pid still present after kill"
        ),
        None => tracing::info!(worker_pid, "orphan sweep clean: worker tree is gone"),
    }
}

#[cfg(test)]
mod tests {
    use super::augment_worker_env;
    use std::collections::BTreeMap;

    #[test]
    fn worker_env_gains_home_and_pod_dir_on_path_head() {
        let mut env = BTreeMap::new();
        env.insert("PATH".to_owned(), "C:/existing".to_owned());
        let dir = std::path::Path::new("C:/fractality/bin");
        augment_worker_env(&mut env, camino::Utf8Path::new("C:/scratch/.fr"), Some(dir));
        assert_eq!(
            env.get("FRACTALITY_HOME").map(String::as_str),
            Some("C:/scratch/.fr"),
            "nested spawns must land on the pod's own mission-control (F16)"
        );
        let path = env.get("PATH").expect("PATH survives");
        let sep = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(
            path,
            &format!("C:/fractality/bin{sep}C:/existing"),
            "pod dir prepends — the worker resolves the supervising build first"
        );
    }

    #[test]
    fn missing_path_is_created_not_paniced() {
        let mut env = BTreeMap::new();
        augment_worker_env(
            &mut env,
            camino::Utf8Path::new("/h"),
            Some(std::path::Path::new("/bin")),
        );
        assert_eq!(env.get("PATH").map(String::as_str), Some("/bin"));
    }
}
