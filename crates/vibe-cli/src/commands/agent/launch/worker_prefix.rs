//! Admission for the opt-in flag that hands workers this launch's own lane.
//!
//! Only Vibe's option is stripped; native arguments retain their boundaries.
//! Admission runs before workspace selection, even when no lane is available.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY");

use anyhow::{Result, bail};
use vibe_workspace::system_prompt::HarnessKind;

pub(super) fn take(rest: &mut Vec<String>) -> Result<bool> {
    let end = rest
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(rest.len());
    if rest[..end]
        .iter()
        .any(|argument| argument.starts_with("--worker-prefix="))
    {
        bail!(
            "`--worker-prefix` takes no value; it hands subagents the same lane file the launch passes"
        );
    }
    let mut taken = false;
    let mut options = true;
    rest.retain(|argument| {
        if argument == "--" {
            options = false;
        }
        if options && argument == "--worker-prefix" {
            taken = true;
            false
        } else {
            true
        }
    });
    Ok(taken)
}

pub(super) fn admit(kind: HarnessKind, native_arguments: &[String]) -> Result<()> {
    let options = &native_arguments[..native_arguments
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(native_arguments.len())];
    if kind != HarnessKind::Claude {
        bail!(
            "`--worker-prefix` is supported only for launchers of kind `claude`, the one harness with a flag that appends a file to every subagent; this launcher is kind `{}`",
            kind.as_str()
        );
    }
    if !options
        .iter()
        .any(|argument| matches!(argument.as_str(), "-p" | "--print"))
    {
        bail!(
            "`--worker-prefix` needs print mode: Claude Code reads `--append-subagent-system-prompt-file` only with `-p` / `--print`, so add `-p` or launch without `--worker-prefix`"
        );
    }
    for flag in [
        "--append-subagent-system-prompt-file",
        "--append-subagent-system-prompt",
    ] {
        if native_arguments
            .iter()
            .any(|argument| argument == flag || argument.starts_with(&format!("{flag}=")))
        {
            bail!(
                "`{flag}` is already in your own arguments, and `--worker-prefix` passes the subagent flag itself — drop one of the two"
            );
        }
    }
    Ok(())
}
