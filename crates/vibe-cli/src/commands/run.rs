//! `vibe run` — explicit host-owned commands, never arbitrary shell strings.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#HOST-RUN-COMMANDS");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, RunCommandDecl, RunCommandHandler};
use vibe_workspace::hooks::{Platform, SystemProbe, select_invocation};

use crate::cli::RunArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: RunArgs) -> Result<i32> {
    let root = super::resolve_project_root(&args.path)?;
    let manifest = Manifest::read(root.join(Manifest::FILENAME))?;
    let declaration = select(&manifest.run_commands, &args.command)?;
    let RunCommandHandler::Script { base } = &declaration.handler;
    let invocation =
        select_invocation(&root, base, Platform::current(), &SystemProbe).ok_or_else(|| {
            anyhow::anyhow!(
                "command `{}` has no usable script for `{}` on this host",
                declaration.id,
                base.display()
            )
        })?;
    let script = contained_script(&root, &invocation.script, &declaration.id)?;
    let executable = std::env::current_exe().context("resolving the running vibe executable")?;
    let (program, mut command_args) = if invocation.interpreter == "powershell" {
        (
            PathBuf::from("powershell"),
            vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                script.into_os_string(),
            ],
        )
    } else {
        (
            PathBuf::from(invocation.interpreter),
            vec![script.into_os_string()],
        )
    };
    command_args.extend(args.args.iter().map(Into::into));
    let status = ctx.suspend_progress(|| {
        std::process::Command::new(&program)
            .args(&command_args)
            .current_dir(&root)
            .env("VIBE_PROJECT_ROOT", &root)
            .env("VIBE_EXECUTABLE", executable)
            .env("VIBE_RUN_COMMAND", &declaration.id)
            .status()
            .with_context(|| format!("spawning command `{}`", declaration.id))
    })?;
    Ok(status.code().unwrap_or(1))
}

fn select<'a>(commands: &'a [RunCommandDecl], id: &str) -> Result<&'a RunCommandDecl> {
    if let Some(command) = commands.iter().find(|command| command.id == id) {
        return Ok(command);
    }
    let available = commands
        .iter()
        .map(|command| command.id.as_str())
        .collect::<Vec<_>>();
    if available.is_empty() {
        bail!("project declares no [[command]] entries")
    }
    bail!(
        "project declares no command `{id}`; available: {}",
        available.join(", ")
    )
}

fn contained_script(root: &Path, script: &Path, id: &str) -> Result<PathBuf> {
    let root = vibe_workspace::strip_unc_prefix(
        root.canonicalize()
            .with_context(|| format!("canonicalizing project root `{}`", root.display()))?,
    );
    let script = vibe_workspace::strip_unc_prefix(script.canonicalize().with_context(|| {
        format!(
            "canonicalizing command `{id}` script `{}`",
            script.display()
        )
    })?);
    if !script.starts_with(&root) {
        bail!("command `{id}` script escapes the selected project root")
    }
    Ok(script)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vibe_core::manifest::{RunCommandDecl, RunCommandHandler};

    use super::select;

    fn command(id: &str) -> RunCommandDecl {
        RunCommandDecl {
            id: id.into(),
            description: None,
            handler: RunCommandHandler::Script {
                base: PathBuf::from("tooling/run"),
            },
        }
    }

    #[test]
    fn selection_is_exact_and_lists_available_commands() {
        let commands = [command("docs"), command("release")];
        assert_eq!(select(&commands, "docs").unwrap().id, "docs");
        let error = select(&commands, "missing").unwrap_err().to_string();
        assert!(error.contains("docs, release"), "{error}");
    }
}
