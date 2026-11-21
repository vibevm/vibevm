//! Which native invocations may inherit this invocation's corporate proxy
//! route, and the supervised child that carries a managed one.
//!
//! Split from `launch.rs` along the seam it already had: the launch cell
//! decides how the boot lane reaches a harness, this one decides whether a
//! configured route may reach it at all. A shape that is not recognised as a
//! fresh, finite, owned invocation is refused rather than guessed at.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SOFT-LAUNCH");

use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_workspace::system_prompt::HarnessKind;

/// Initial supported native shape: global --standalone followed by run.
/// After `run`, positional prompt text (including web/serve/attach) is opaque.
/// Only known value-consuming run options are walked; unknown options refuse.
fn standalone_opencode_run(arguments: &[String]) -> bool {
    if arguments.first().map(String::as_str) != Some("--standalone")
        || arguments.get(1).map(String::as_str) != Some("run")
    {
        return false;
    }
    let mut index = 2;
    while let Some(argument) = arguments.get(index) {
        match argument.as_str() {
            "--" => return true,
            "--attach" => return false,
            "-m" | "--model" | "--agent" | "--session" | "--title" | "--format" | "--file"
            | "-f" => {
                index += 1;
                if arguments.get(index).is_none() {
                    return false;
                }
            }
            "--continue" | "-c" | "--fork" => {}
            value if value.starts_with("--attach=") => return false,
            value if value.starts_with('-') => return false,
            _ => {}
        }
        index += 1;
    }
    true
}

/// First Codex route: fresh noninteractive exec only. The native parser owns
/// the rest; this gate recognizes a finite option subset, never prompt prose.
fn fresh_codex_exec(arguments: &[String]) -> bool {
    if arguments.first().map(String::as_str) != Some("exec") {
        return false;
    }
    let mut index = 1;
    while let Some(argument) = arguments.get(index) {
        match argument.as_str() {
            "--" => return true,
            "--skip-git-repo-check" | "--ephemeral" | "--json" => {}
            "--sandbox" | "-s" => {
                index += 1;
                if arguments.get(index).map(String::as_str) != Some("read-only") {
                    return false;
                }
            }
            "--model" | "-m" => {
                index += 1;
                if arguments.get(index).is_none() {
                    return false;
                }
            }
            value if value.starts_with('-') => return false,
            "resume" | "fork" | "review" | "help" => return false,
            _ => return index + 1 == arguments.len(),
        }
        index += 1;
    }
    true
}

/// Known Q2 headless request flags only. Prompt values are opaque, and no
/// resumed/interactive/ACP/server mode can inherit a fresh invocation lease.
fn fresh_qwen_prompt(arguments: &[String]) -> bool {
    let mut index = 0;
    let mut prompt = false;
    while let Some(argument) = arguments.get(index) {
        match argument.as_str() {
            "-p" | "--prompt" => {
                if prompt {
                    return false;
                }
                prompt = true;
            }
            "--auth-type" | "--openai-api-key" | "--openai-base-url" | "-m" | "--model" => {}
            _ => return false,
        }
        index += 1;
        if arguments.get(index).is_none() {
            return false;
        }
        index += 1;
    }
    prompt
}

/// Fresh native Claude print only; no resume/background/remote lifetime.
fn fresh_claude_print(arguments: &[String]) -> bool {
    let mut index = 0;
    let mut print = false;
    let mut prompt = false;
    while let Some(argument) = arguments.get(index) {
        match argument.as_str() {
            "-p" | "--print" => {
                if print {
                    return false;
                }
                print = true;
            }
            "--model" => {
                index += 1;
                if arguments
                    .get(index)
                    .is_none_or(|value| value.starts_with('-'))
                {
                    return false;
                }
            }
            "--tools" => {
                index += 1;
                if arguments.get(index).map(String::as_str) != Some("") {
                    return false;
                }
            }
            "--no-session-persistence" => {}
            value if !value.starts_with('-') => {
                if prompt {
                    return false;
                }
                prompt = true;
            }
            _ => return false,
        }
        index += 1;
    }
    print && prompt
}

fn fresh_hermes_query(arguments: &[String]) -> bool {
    matches!(arguments, [chat, query, _] if chat == "chat" && (query == "-q" || query == "--query"))
}

pub(super) fn admit_route_kind(kind: HarnessKind, arguments: &[String]) -> Result<()> {
    let configured = vibe_net::current().route().mode_name() != "direct";
    if kind == HarnessKind::OpenCode && configured {
        if !standalone_opencode_run(arguments) {
            bail!(
                "configured corporate proxy requires an explicitly recognized standalone OpenCode run; an existing server owner or ambiguous invocation cannot inherit this invocation route"
            );
        }
        return Ok(());
    }
    if kind == HarnessKind::Codex && configured {
        if !fresh_codex_exec(arguments) {
            bail!(
                "configured corporate proxy requires a recognized fresh noninteractive Codex exec; resume, remote and ambiguous invocations are not qualified"
            );
        }
        return Ok(());
    }
    if kind == HarnessKind::Qwen && configured {
        if !fresh_qwen_prompt(arguments) {
            bail!(
                "configured corporate proxy requires a recognized fresh headless Qwen -p invocation; resume, ACP, interactive and ambiguous forms are not qualified"
            );
        }
        return Ok(());
    }
    if kind == HarnessKind::Claude && configured {
        if !fresh_claude_print(arguments) {
            bail!(
                "configured corporate proxy requires a recognized fresh Claude print invocation; resume, background and ambiguous forms are not qualified"
            );
        }
        return Ok(());
    }
    if kind == HarnessKind::Hermes && configured {
        if vibe_net::current().route().px().is_none() || !fresh_hermes_query(arguments) {
            bail!(
                "configured corporate proxy requires an owned Hermes finite chat query with managed px; shared, resumed and external routes are not qualified"
            );
        }
        return Ok(());
    }
    if kind != HarnessKind::Pi && configured {
        bail!(
            "configured corporate proxy routing is not yet qualified for harness `{}`; no agent was started (use a qualified Pi launcher or retain native/unmanaged invocation explicitly)",
            kind.as_str()
        );
    }
    Ok(())
}

/// Admitted native routes share one lifetime owner; other adapters remain separate work.
pub(super) fn route_child(kind: HarnessKind, command: &mut Command) -> Result<Option<i32>> {
    if !matches!(
        kind,
        HarnessKind::Pi
            | HarnessKind::OpenCode
            | HarnessKind::Codex
            | HarnessKind::Qwen
            | HarnessKind::Claude
    ) {
        return Ok(None);
    }
    let Some(route) = crate::commands::tool::px::AgentRoute::selected()? else {
        return Ok(None);
    };
    route.apply(command);
    if !route.managed() {
        return Ok(None);
    }
    let result = (|| {
        let mut supervisor = crate::process_supervision::Supervisor::install()?;
        let child = command
            .spawn()
            .context("starting agent with the selected managed proxy")?;
        supervisor
            .wait(child)
            .context("waiting for agent with the selected managed proxy")
    })();
    route.close();
    let status = result?;
    if let Some(code) = status.code() {
        return Ok(Some(code));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Ok(Some(128 + status.signal().unwrap_or(1)))
    }
    #[cfg(not(unix))]
    Ok(Some(1))
}

#[cfg(test)]
mod proxy_argument_tests {
    use super::standalone_opencode_run;
    #[test]
    fn hermes_proxy_admits_only_owned_finite_query() {
        let check = |args: &[&str]| {
            super::fresh_hermes_query(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        };
        assert!(check(&["chat", "-q", "--resume"]));
        assert!(!check(&["chat", "--resume", "id", "-q", "hello"]));
        assert!(!check(&["gateway", "run"]));
        assert!(!check(&["chat"]));
    }

    #[test]
    fn claude_proxy_admits_native_print_not_persistent_modes() {
        let check = |args: &[&str]| {
            super::fresh_claude_print(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        };
        assert!(check(&[
            "-p",
            "hello",
            "--tools",
            "",
            "--no-session-persistence"
        ]));
        assert!(check(&["--model", "claude-opus-5-5", "--print", "hello"]));
        assert!(!check(&["--resume", "id", "-p", "hello"]));
        assert!(!check(&["--background", "-p", "hello"]));
        assert!(!check(&["-p", "--continue"]));
        assert!(!check(&["-p", "--resume"]));
        assert!(!check(&["-p"]));
        assert!(!check(&["-p", "hello", "--unknown"]));
    }

    #[test]
    fn qwen_proxy_admits_opaque_prompt_not_other_lifetimes() {
        let check = |args: &[&str]| {
            super::fresh_qwen_prompt(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        };
        assert!(check(&["-p", "--resume"]));
        assert!(check(&[
            "--auth-type",
            "openai",
            "-m",
            "fake-1",
            "-p",
            "web"
        ]));
        assert!(!check(&["--resume", "last", "-p", "hello"]));
        assert!(!check(&["--acp", "-p", "hello"]));
        assert!(!check(&["--prompt-interactive", "hello"]));
        assert!(!check(&["-p", "hello", "--unknown"]));
        assert!(!check(&["-p"]));
    }

    #[test]
    fn codex_proxy_admits_fresh_exec_not_resumed_or_remote_owners() {
        let check = |args: &[&str]| {
            super::fresh_codex_exec(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        };
        assert!(check(&["exec", "--sandbox", "read-only", "hello"]));
        assert!(check(&["exec", "--", "resume"]));
        assert!(check(&["exec", "--model", "sentinel", "web"]));
        assert!(!check(&["exec", "resume", "--last"]));
        assert!(!check(&["exec", "fork", "hello"]));
        assert!(!check(&["cloud", "exec", "hello"]));
        assert!(!check(&["exec", "--config", "unknown=1", "hello"]));
        assert!(!check(&[
            "exec",
            "--sandbox",
            "danger-full-access",
            "hello"
        ]));
    }

    #[test]
    fn opencode_proxy_classifies_boundaries_not_prompt_prose() {
        let check = |args: &[&str]| {
            standalone_opencode_run(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        };
        assert!(check(&["--standalone", "run", "web"]));
        assert!(check(&["--standalone", "run", "--", "--attach", "serve"]));
        assert!(check(&[
            "--standalone",
            "run",
            "-m",
            "fake/fake-1",
            "attach"
        ]));
        assert!(!check(&["--standalone", "attach", "http://localhost"]));
        assert!(!check(&["--standalone", "serve"]));
        assert!(!check(&[
            "--standalone",
            "run",
            "--attach",
            "http://localhost"
        ]));
        assert!(!check(&["--standalone", "run", "--unknown", "web"]));
    }
}
