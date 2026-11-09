//! `vibe hook bash|zsh|fish|pwsh`: one managed block in that shell's
//! startup file that applies the project's versions on directory change
//! (PROP-075 `#ACTIVATION-HOOK`).
//!
//! The function the block defines **spawns no process in the common case**.
//! It remembers the directory it last resolved and returns immediately with
//! builtins only when `$PWD` has not changed. When it has, it walks up with
//! the shell's own file test over the name list the installed packages
//! declare — baked into the block, so the walk is a fixed number of builtin
//! tests and not a `vibe` call — and invokes `vibe app env --project` once,
//! and only when a candidate file exists or a previous selection must be
//! undone.
//!
//! A hook never installs, never downloads, never writes the index and never
//! fails the prompt: `vibe app env` prints its notices on stderr and its
//! lines on stdout, so an empty answer evaluates to nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK");

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use vibe_app::{DeclaredVersionFiles, HOOK_BLOCK};

use crate::cli::HookArgs;
use crate::commands::app::{facts, global};
use crate::output;
use crate::shell_syntax::Shell;

pub fn run(ctx: &output::Context, args: HookArgs) -> Result<()> {
    let shell = Shell::parse(&args.shell)?;
    if matches!(shell, Shell::Cmd | Shell::Posix) {
        bail!(
            "vibe hook takes bash, zsh, fish or pwsh; cmd has no prompt hook and `vibe shell` is \
             the way there (violates \
             spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK)"
        );
    }
    let settings = global::settings_root()?;
    let names = DeclaredVersionFiles::load(&settings).names();
    let body = block_body(shell, &names);
    if args.print {
        for line in body.lines() {
            println!("{line}");
        }
        return Ok(());
    }
    let Some(home) = facts::user_home() else {
        bail!(
            "vibe hook has no home directory to write a startup file in; use `vibe hook {} \
             --print` and install the block yourself",
            shell.as_str()
        );
    };
    let rc = rc_path(shell, &home);
    if !args.yes && ctx.is_unattended() {
        bail!(
            "consent_required: `vibe hook {}` would write a managed block into `{}`, and this \
             run has nobody to ask (violates \
             spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK; fix: rerun with \
             --yes, or `vibe hook {} --print` and install the block yourself)",
            shell.as_str(),
            rc.display(),
            shell.as_str()
        );
    }
    if !args.yes {
        let accepted = ctx.suspend_progress(|| {
            crate::output::prompts::confirm_then(
                &format!("write the vibe version hook into `{}`?", rc.display()),
                true,
                "vibe hook",
                |accepted| {
                    if accepted {
                        "writing the block".to_owned()
                    } else {
                        "nothing written".to_owned()
                    }
                },
            )
            .context("reading the hook consent answer")
        })?;
        if !accepted {
            ctx.summary("vibe hook: nothing written");
            return Ok(());
        }
    }
    vibe_app::ensure_block(HOOK_BLOCK, &rc, &body).map_err(anyhow::Error::msg)?;
    ctx.summary(&format!(
        "vibe hook: {} block written into `{}`; open a new shell or source it to apply now",
        shell.as_str(),
        rc.display()
    ));
    Ok(())
}

/// The startup file the hook's block is written into. `Shell::rc_path`
/// answers for the POSIX shells; PowerShell's profile is not `.profile`.
pub(crate) fn rc_path(shell: Shell, home: &std::path::Path) -> PathBuf {
    match shell {
        Shell::Pwsh if cfg!(windows) => home
            .join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        Shell::Pwsh => home
            .join(".config")
            .join("powershell")
            .join("Microsoft.PowerShell_profile.ps1"),
        other => other.rc_path(home),
    }
}

/// The body of the managed block for `shell`, with `names` baked in as the
/// fixed list the builtin walk tests.
pub(crate) fn block_body(shell: Shell, names: &[String]) -> String {
    match shell {
        Shell::Bash => posix_body(names, Register::PromptCommand),
        Shell::Zsh => posix_body(names, Register::Precmd),
        Shell::Fish => fish_body(names),
        Shell::Pwsh => pwsh_body(names),
        Shell::Cmd | Shell::Posix => String::new(),
    }
}

/// How the shell is told to run the function before each prompt.
enum Register {
    /// bash: `PROMPT_COMMAND`.
    PromptCommand,
    /// zsh: `precmd_functions`.
    Precmd,
}

/// The POSIX-shell body, shared by bash and zsh: the two differ only in how
/// a function is registered on the prompt.
fn posix_body(names: &[String], register: Register) -> String {
    let shell = match register {
        Register::PromptCommand => "bash",
        Register::Precmd => "zsh",
    };
    let list = names
        .iter()
        .map(|name| format!("'{}'", name.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let registration = match register {
        Register::PromptCommand => {
            "case \":${PROMPT_COMMAND-}:\" in\n  \
             *:__vibe_versions:*) ;;\n  \
             *) PROMPT_COMMAND=\"__vibe_versions${PROMPT_COMMAND:+;$PROMPT_COMMAND}\" ;;\n\
             esac"
        }
        Register::Precmd => {
            "typeset -ga precmd_functions\n\
             if [[ -z ${precmd_functions[(r)__vibe_versions]} ]]; then\n  \
             precmd_functions+=(__vibe_versions)\n\
             fi"
        }
    };
    // `__vibe_version_names` empty means no installed package declares a
    // version file: the function then does nothing at all and costs one
    // string comparison per prompt.
    format!(
        "__vibe_version_names=({list})\n\
         __vibe_version_dir=''\n\
         __vibe_version_applied=''\n\
         __vibe_versions() {{\n  \
         [ \"$PWD\" = \"$__vibe_version_dir\" ] && return 0\n  \
         __vibe_version_dir=$PWD\n  \
         __vibe_version_found=''\n  \
         __vibe_version_walk=$PWD\n  \
         while [ -n \"$__vibe_version_walk\" ]; do\n    \
         for __vibe_version_name in \"${{__vibe_version_names[@]}}\"; do\n      \
         if [ -f \"$__vibe_version_walk/$__vibe_version_name\" ]; then\n        \
         __vibe_version_found=$__vibe_version_walk/$__vibe_version_name\n        \
         break\n      \
         fi\n    \
         done\n    \
         [ -n \"$__vibe_version_found\" ] && break\n    \
         [ \"$__vibe_version_walk\" = / ] && break\n    \
         __vibe_version_walk=${{__vibe_version_walk%/*}}\n    \
         [ -z \"$__vibe_version_walk\" ] && __vibe_version_walk=/\n  \
         done\n  \
         if [ -z \"$__vibe_version_found\" ] && [ -z \"$__vibe_version_applied\" ]; then\n    \
         return 0\n  \
         fi\n  \
         __vibe_version_applied=$__vibe_version_found\n  \
         eval \"$(command vibe app env --project --shell {shell})\" || true\n  \
         return 0\n\
         }}\n\
         {registration}"
    )
}

fn fish_body(names: &[String]) -> String {
    let list = names
        .iter()
        .map(|name| format!("'{}'", name.replace('\'', "\\'")))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "set -g __vibe_version_names {list}\n\
         set -g __vibe_version_dir ''\n\
         set -g __vibe_version_applied ''\n\
         function __vibe_versions --on-variable PWD\n    \
         if test \"$PWD\" = \"$__vibe_version_dir\"\n        \
         return 0\n    \
         end\n    \
         set -g __vibe_version_dir $PWD\n    \
         set -l found ''\n    \
         set -l walk $PWD\n    \
         while test -n \"$walk\"\n        \
         for name in $__vibe_version_names\n            \
         if test -f \"$walk/$name\"\n                \
         set found \"$walk/$name\"\n                \
         break\n            \
         end\n        \
         end\n        \
         if test -n \"$found\"\n            \
         break\n        \
         end\n        \
         if test \"$walk\" = /\n            \
         break\n        \
         end\n        \
         set walk (string replace -r '/[^/]*$' '' -- $walk)\n        \
         if test -z \"$walk\"\n            \
         set walk /\n        \
         end\n    \
         end\n    \
         if test -z \"$found\" -a -z \"$__vibe_version_applied\"\n        \
         return 0\n    \
         end\n    \
         set -g __vibe_version_applied \"$found\"\n    \
         command vibe app env --project --shell fish | source\n    \
         return 0\n\
         end"
    )
}

fn pwsh_body(names: &[String]) -> String {
    let list = names
        .iter()
        .map(|name| format!("'{}'", name.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "$global:VibeVersionNames = @({list})\n\
         $global:VibeVersionDir = ''\n\
         $global:VibeVersionApplied = ''\n\
         function global:__vibe_versions {{\n    \
         if ($PWD.Path -eq $global:VibeVersionDir) {{ return }}\n    \
         $global:VibeVersionDir = $PWD.Path\n    \
         $walk = $PWD.Path\n    \
         $found = ''\n    \
         while ($walk) {{\n        \
         foreach ($name in $global:VibeVersionNames) {{\n            \
         $candidate = Join-Path $walk $name\n            \
         if (Test-Path -LiteralPath $candidate -PathType Leaf) {{ $found = $candidate; break }}\n        \
         }}\n        \
         if ($found) {{ break }}\n        \
         $parent = Split-Path -Parent $walk\n        \
         if (-not $parent -or $parent -eq $walk) {{ break }}\n        \
         $walk = $parent\n    \
         }}\n    \
         if (-not $found -and -not $global:VibeVersionApplied) {{ return }}\n    \
         $global:VibeVersionApplied = $found\n    \
         $lines = & vibe app env --project --shell pwsh\n    \
         if ($lines) {{ Invoke-Expression ($lines -join \"`n\") }}\n\
         }}\n\
         if (-not $global:VibeVersionsPromptWrapped) {{\n    \
         $global:VibeVersionsPromptWrapped = $true\n    \
         $global:VibeVersionsOriginalPrompt = $function:prompt\n    \
         function global:prompt {{\n        \
         __vibe_versions\n        \
         & $global:VibeVersionsOriginalPrompt\n    \
         }}\n\
         }}"
    )
}

#[cfg(test)]
#[path = "hook/tests.rs"]
mod tests;
