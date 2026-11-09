//! `vibe tool px exec|serve|status` (`PROP-068#proxy-tool`): the
//! vibe-managed px, lent to a tool vibe does not run itself.
//!
//! A corporate network that needs px needs it for `pip`, `npm`, `cargo`
//! and `curl` too, and each of them has its own idea of how a proxy is
//! configured. px is already the one thing that knows how to authenticate
//! here, so rather than teach every tool the route, vibe starts exactly
//! the px its own operations start — the same `[proxy]` resolution, the
//! same companion lookup, the same lifetime — and hands the child the
//! loopback address in the variables every one of those tools reads.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool");

use std::process::Command;

use anyhow::{Result, anyhow, bail};
use vibe_core::cancellation::CancellationToken;
use vibe_core::net::ProxyRoute;

use super::{ToolEnv, which};
use crate::cli::{Command as Verb, ToolCommand, ToolPxCommand, ToolPxExecArgs, ToolPxServeArgs};
use crate::output;

/// The proxy variables a child is given, uppercase and lowercase: the
/// tools that need px disagree about which they read, and a tool that
/// reads only one must not be left without a route.
const PROXY_VARS: &[&str] = &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"];
/// The bypass variables, in both spellings for the same reason.
const NO_PROXY_VARS: &[&str] = &["NO_PROXY", "no_proxy"];

/// What the child is told to use, and how it was decided.
struct Lent {
    /// The proxy URL the child receives, or `None` for a direct run.
    url: Option<String>,
    /// What the report says the child goes through.
    how: String,
    /// The bypass list the child receives.
    no_proxy: String,
}

/// The fixed loopback port a `vibe tool px` invocation asked for, read
/// at the composition root: one session serves the whole invocation, and
/// it is built before any command runs, so `--port` has to be applied to
/// the route that session is created from rather than to a later copy.
pub fn requested_px_port(command: &Verb) -> Option<u16> {
    let Verb::Tool(tool) = command else {
        return None;
    };
    let ToolCommand::Px(px) = &tool.command;
    match &px.command {
        ToolPxCommand::Exec(exec) => exec.port,
        ToolPxCommand::Serve(serve) => serve.port,
        ToolPxCommand::Status => None,
    }
}

/// The bypass list a child receives: loopback always, then the
/// configured list — the same list vibe hands its own children.
fn no_proxy_of(route: &ProxyRoute) -> String {
    let mut list = String::from("localhost,127.0.0.1,::1");
    if let Some(configured) = route.no_proxy() {
        list.push(',');
        list.push_str(configured);
    }
    list
}

/// Start what the route asks for and say what the child is given.
///
/// `px` starts px and lends its loopback address. `explicit` lends the
/// configured proxy URL as written — px is not involved and starting one
/// would add a hop the user did not ask for. No proxy at all is a
/// refusal, because `vibe tool px exec` is asked for precisely when the
/// child cannot reach the network on its own; `--allow-direct` runs it
/// anyway, with no proxy variables.
fn lend(session: &std::sync::Arc<vibe_net::NetworkSession>, allow_direct: bool) -> Result<Lent> {
    let route = session.route();
    let no_proxy = no_proxy_of(route);
    if route.px().is_some() {
        let address = session.px_address()?;
        return Ok(Lent {
            url: Some(format!("http://{address}")),
            how: format!("the vibe-managed px on {address}"),
            no_proxy,
        });
    }
    if let Some((https, http)) = route.explicit_urls() {
        let chosen = https
            .or(http)
            .ok_or_else(|| anyhow!("`[proxy] mode = \"explicit\"` names no proxy URL to lend"))?;
        let chosen = as_written(chosen);
        return Ok(Lent {
            // The printable form: a configured credential is handed to the
            // child through the environment, never printed.
            how: format!(
                "the configured proxy {}",
                vibe_core::progress::transfer::safe_url(&chosen)
            ),
            url: Some(chosen),
            no_proxy,
        });
    }
    if allow_direct {
        return Ok(Lent {
            url: None,
            how: "no proxy (--allow-direct)".to_owned(),
            no_proxy,
        });
    }
    bail!(
        "`[proxy]` configures no proxy, so there is nothing to lend \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool; \
          fix: set `[proxy] mode = \"px\"` with `pac` or `upstream` in ~/.vibe/config.toml or \
          the site profile, or pass `--allow-direct` to run the command with no proxy)"
    )
}

/// A proxy URL as the user wrote it: a bare `http://host:port` gains a
/// path of `/` when it is parsed, and the child then reports a proxy
/// nobody configured. Nothing else is touched — a credential, a path or a
/// query travels exactly as the `[proxy]` table says.
fn as_written(url: &reqwest::Url) -> String {
    let text = url.to_string();
    if url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && let Some(trimmed) = text.strip_suffix('/')
    {
        return trimmed.to_owned();
    }
    text
}

/// What `exec` answers when the command it was given is not there: the
/// shell's own code for it, which a wrapper's caller tests for, rather
/// than vibe's generic refusal exit.
pub const COMMAND_NOT_FOUND: i32 = 127;

/// `NAME=VALUE`, as `--env` takes it.
fn parse_env(raw: &str) -> Result<(String, String)> {
    let (name, value) = raw.split_once('=').ok_or_else(|| {
        anyhow!(
            "`--env {raw}` is not `NAME=VALUE` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool; \
              fix: write `--env NAME=VALUE`; `--proxy-env NAME` sets NAME to the proxy's URL)"
        )
    })?;
    if name.trim().is_empty() {
        bail!(
            "`--env {raw}` has no variable name \
             (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool; \
              fix: write `--env NAME=VALUE`)"
        );
    }
    Ok((name.to_owned(), value.to_owned()))
}

/// `vibe tool px exec [--port N] -- <command> …`: start px as vibe's own
/// operations start it, run the command through it, and answer with the
/// command's exit code. px is stopped when the command exits, by the same
/// supervisor (Unix) or job object (Windows) that ends it when vibe does.
pub fn run_exec(
    ctx: &output::Context,
    args: &ToolPxExecArgs,
    env: &ToolEnv,
    cancellation: &CancellationToken,
) -> Result<i32> {
    let (name, rest) = args
        .command
        .split_first()
        .ok_or_else(|| anyhow!("`vibe tool px exec` needs a command after `--`"))?;
    let extra: Vec<(String, String)> = args
        .env
        .iter()
        .map(|raw| parse_env(raw))
        .collect::<Result<_>>()?;

    let Some(program) = which::locate(name, env.path.as_deref(), env.pathext.as_deref()) else {
        // Reported here rather than returned, because the answer is the
        // shell's own 127 and not vibe's refusal exit: a wrapper around
        // `exec` fails the way the command it wraps would.
        ctx.error_with_suffix(
            &anyhow!(
                "`{name}` is not on PATH, so there is nothing to run through px \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool; \
                  fix: put `{name}` on PATH, or name it by path)"
            ),
            None,
        );
        return Ok(COMMAND_NOT_FOUND);
    };

    // One px for this invocation: the operation's resource the
    // composition root created, killed and reaped when the invocation
    // ends — which is after the child has exited.
    let session = vibe_net::current();
    let lent = lend(&session, args.allow_direct)?;
    // `exec` is transparent: stdout and stderr are the child's, so what
    // vibe has to say about the route is said on stderr and only when
    // asked for. A sentence on stdout would end up inside
    // `pxpip freeze > requirements.txt`.
    if env.verbose {
        ctx.block_err(&format!(
            "running `{}` through {}",
            program.display(),
            lent.how
        ));
    }

    let mut command = Command::new(&program);
    command.args(rest);
    for name in PROXY_VARS {
        match &lent.url {
            Some(url) => command.env(name, url),
            // A direct run must not inherit a proxy the parent carries.
            None => command.env_remove(name),
        };
    }
    for name in NO_PROXY_VARS {
        command.env(name, &lent.no_proxy);
    }
    // A tool that reads its own variable for the proxy cannot be told the
    // address in advance, so it is named instead of hard-coded here.
    if let Some(url) = &lent.url {
        for name in &args.proxy_env {
            command.env(name, url);
        }
    }
    // Last, so a caller can override any of the above.
    for (name, value) in &extra {
        command.env(name, value);
    }

    let status = ctx.suspend_progress(|| {
        command.status().map_err(|error| {
            anyhow!(
                "running `{}` failed: {error} \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy-tool; \
                  fix: check that the file is a program this machine can run)",
                program.display()
            )
        })
    })?;
    // A child killed by a signal has no code; 130 is the conventional
    // answer for an interrupted one and 1 for anything else.
    Ok(status
        .code()
        .unwrap_or(if cancellation.is_cancelled() { 130 } else { 1 }))
}

/// `vibe tool px serve [--port N]`: px in the foreground until Ctrl-C,
/// for a tool vibe cannot start itself (an editor, a long-lived shell).
pub fn run_serve(
    ctx: &output::Context,
    _args: &ToolPxServeArgs,
    cancellation: &CancellationToken,
) -> Result<()> {
    let session = vibe_net::current();
    let lent = lend(&session, false)?;
    let url = lent
        .url
        .clone()
        .ok_or_else(|| anyhow!("`[proxy]` configures no proxy to serve"))?;
    let pac = session
        .route()
        .px()
        .and_then(|settings| settings.pac.as_ref())
        .map(|pac| match pac {
            vibe_core::user_config::PacSource::System => match vibe_net::system_proxy::resolve() {
                Ok(resolved) => resolved.describe(),
                Err(error) => format!("system → unresolved: {error}"),
            },
            other => other.px_location().unwrap_or_default(),
        });
    let upstream = session
        .route()
        .px()
        .and_then(|settings| settings.upstream.clone());
    if _args.json {
        ctx.emit_json(&serde_json::json!({
            "protocol": "vibe-px-serve/1",
            "url": url,
            "pac": pac,
            "upstream": upstream,
            "no_proxy": lent.no_proxy,
        }))?;
    } else {
        ctx.heading(&format!("px is listening on {url}"));
        if let Some(pac) = &pac {
            ctx.step(&format!("pac       {pac}"));
        }
        if let Some(upstream) = &upstream {
            ctx.step(&format!("upstream  {upstream}"));
        }
        ctx.step(&format!("no_proxy  {}", lent.no_proxy));
        ctx.step("export HTTPS_PROXY and HTTP_PROXY to that address; Ctrl-C stops px");
    }
    // px lives as long as the invocation; the wait ends when the
    // invocation is stopped, and the operation then kills and reaps it.
    while !cancellation.is_cancelled() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}
