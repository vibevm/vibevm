//! The environment and global flags of every spawned git: the C locale
//! that keeps stderr parseable, and the silencing of interactive
//! credential entry (PROP-002 §2.2.1).

use std::process::Command;

use super::diagnostics::is_truthy;

/// Apply environment + global git flags that every spawned git
/// invocation needs. Two layers:
///
/// 1. **Always.** `LC_ALL=C` / `LANG=C` for stable error parsing;
///    `GIT_TERMINAL_PROMPT=0` so terminal-style prompts don't block
///    a non-TTY parent; on Windows `CREATE_NO_WINDOW` so a hostless
///    parent doesn't flash a console.
/// 2. **TTY-aware, or forced.** When `force_silence` is set (the
///    backend serves a public `auth = "none"` registry — see
///    [`should_silence_credential_helpers`]) OR we are running
///    unattended (no stdin TTY OR `VIBE_UNATTENDED` env-var truthy),
///    suppress every channel git might use for *interactive* credential
///    entry: GCM popups (`GCM_INTERACTIVE=Never`), system credential
///    helpers (`-c credential.helper=`), `core.askPass`
///    (`-c core.askPass=`), and `GIT_ASKPASS` (set to empty). This is
///    the policy from PROP-002 §2.2.1: a 401 must not become a blocking
///    GUI prompt; the resolver instead classifies it as an
///    `UnknownPackage` (for `auth = "none"` registries) or as an
///    `AuthFailed` halt (for `auth = "token-env"` /
///    `"credential-helper"` registries).
///
/// **Layering note.** This function MUST be called BEFORE the
/// caller pushes the actual git subcommand args, because the
/// silencing layer prepends `-c <key>=<value>` flags that have to
/// come before the subcommand. The `[run, run_raw, preflight]`
/// methods on this module already follow that order.
///
/// Override: `VIBEVM_GIT_SILENCE_HELPERS=1` forces silencing on
/// regardless of TTY; `VIBEVM_GIT_SILENCE_HELPERS=0` forces it off.
/// Neither value is the typical case — the TTY-derived default
/// covers most operators.
pub(super) fn apply_common_env(cmd: &mut Command, force_silence: bool) {
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");

    if should_silence_credential_helpers(force_silence) {
        cmd.env("GCM_INTERACTIVE", "Never");
        // Reset both ends of the system credential machinery for the
        // duration of this invocation. `credential.helper=` empty
        // disables every helper configured in `~/.gitconfig` /
        // `/etc/gitconfig`; `core.askPass=` empty stops git from
        // exec'ing a binary asker. `GIT_ASKPASS` is deliberately
        // NOT set here — on some platforms an empty string for
        // that variable can confuse git's startup probe of the
        // path; clearing the two git-config keys plus the
        // already-set `GIT_TERMINAL_PROMPT=0` is sufficient to
        // close every interactive avenue.
        cmd.arg("-c").arg("credential.helper=");
        cmd.arg("-c").arg("core.askPass=");
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Deep package/cache roots are routine on Windows. Apply this to the
        // invocation rather than requiring a machine-global gitconfig edit.
        cmd.arg("-c").arg("core.longpaths=true");
        cmd.creation_flags(0x0800_0000);
    }
}

/// Decide whether to silence interactive credential mechanisms.
/// Resolution order:
///
/// 0. `force_silence` — the caller's per-backend posture. A backend
///    handed to a public (`auth = "none"`) registry sets this so a
///    401/403 from a public host can never become an interactive
///    credential prompt, regardless of TTY (PROP-002 §2.2.1). A public
///    registry never sends credentials, so there is nothing to prompt
///    for — the 401 must classify as "no answer here" and the walk
///    continues. This wins over every environment/TTY signal below.
/// 1. `VIBEVM_GIT_SILENCE_HELPERS` env-var — explicit operator
///    override (truthy / falsy vocabulary same as `VIBE_UNATTENDED`:
///    `1`, `true`, `yes`, `on` for true; everything else for false).
/// 2. `VIBE_UNATTENDED` env-var truthy — global unattended posture
///    (PROP-002 §2.2.1 + `vibe --unattended` flag).
/// 3. stdin not a TTY — we are running under a non-interactive
///    parent (CI, service, opencode harness).
///
/// Otherwise (interactive TTY without `--unattended`), leave git
/// alone — an operator running `vibe install` against an
/// authenticated registry at a real terminal might genuinely want to
/// type a password.
pub(super) fn should_silence_credential_helpers(force_silence: bool) -> bool {
    if force_silence {
        return true;
    }
    if let Ok(v) = std::env::var("VIBEVM_GIT_SILENCE_HELPERS") {
        return is_truthy(v.trim());
    }
    if let Ok(v) = std::env::var("VIBE_UNATTENDED")
        && is_truthy(v.trim())
    {
        return true;
    }
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

/// The explicit network route of a child that talks to a remote
/// (`PROP-068#proxy-git`), from [`vibe_net::git::for_args`]. Must run
/// before the subcommand arguments.
pub(super) fn apply_network(
    cmd: &mut Command,
    binary: &std::path::Path,
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> Result<(), super::GitError> {
    let net =
        vibe_net::git::for_args(&vibe_net::current(), binary, args, cwd).map_err(|error| {
            super::GitError::CommandFailed {
                cmd: "git (network route)".into(),
                status: -1,
                stderr: error.to_string(),
            }
        })?;
    net.apply(cmd);
    Ok(())
}
