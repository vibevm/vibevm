//! Install-lifecycle hooks (PROP-020): run a package's `pre-install` /
//! `post-install` script in its materialised slot, choosing the interpreter
//! per host OS and gating execution on a trust decision.
//!
//! This module is a self-contained cell with two injectable seams so the
//! selection, trust, and failure logic are unit-tested without spawning a
//! real process: [`InterpreterProbe`] (is `bash` / `powershell` present?)
//! and [`HookRunner`] (execute an invocation). The interactive consent
//! prompt itself is **not** here — [`decide_trust`] returns
//! [`HookTrust::NeedsConsent`] and the CLI resolves it; the library stays
//! standalone (PROP-018 §2.3) and non-interactive.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-020#phases");

use std::path::{Path, PathBuf};
use std::process::Command;

use specmark::spec;
use thiserror::Error;
use vibe_core::Group;
use vibe_core::manifest::HooksDecl;

/// The default trusted groups whose hooks run with no prompt (PROP-020
/// §2.3). `org.vibevm` — the project's own packages — is allow-listed out
/// of the box; an operator extends this via config.
pub const DEFAULT_ALLOWED_GROUPS: &[&str] = &["org.vibevm"];

/// Which lifecycle phase a hook runs in (PROP-020 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPhase {
    /// After the slot is populated, before vibevm uses it — the "bring the
    /// tree into order" hook.
    PreInstall,
    /// After the package install is durable — finalisation.
    PostInstall,
}

impl HookPhase {
    /// The wire / env name of the phase (`pre-install` / `post-install`).
    pub fn as_str(self) -> &'static str {
        match self {
            HookPhase::PreInstall => "pre-install",
            HookPhase::PostInstall => "post-install",
        }
    }

    /// The manifest base path declared for this phase, if any.
    fn base(self, hooks: &HooksDecl) -> Option<&Path> {
        match self {
            HookPhase::PreInstall => hooks.pre_install.as_deref(),
            HookPhase::PostInstall => hooks.post_install.as_deref(),
        }
    }
}

/// The host-OS family that drives interpreter selection (PROP-020 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS / Linux — `.sh` via `bash`.
    Unix,
    /// Windows — `.sh` via Git Bash first, else `.ps1` via PowerShell.
    Windows,
}

impl Platform {
    /// The platform this process runs on.
    pub fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        }
    }
}

/// How a hook script is invoked on this host (PROP-020 §2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookInvocation {
    /// Interpreter program — `bash` or `powershell`.
    pub interpreter: String,
    /// The resolved script file (with extension).
    pub script: PathBuf,
}

/// The trust decision for running a package's hooks (PROP-020 §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTrust {
    /// Allow-listed (or `--allow-hooks`) — run with no prompt.
    Allowed,
    /// Interactive and not allow-listed — the CLI must ask before running.
    NeedsConsent,
    /// Non-interactive and not allow-listed — refuse, never silent-run.
    Refused,
}

/// Per-package identity + slot a hook runs against.
#[derive(Debug, Clone, Copy)]
pub struct HookContext<'a> {
    pub group: &'a Group,
    pub name: &'a str,
    pub version: &'a str,
    pub kind: &'a str,
    /// The materialised slot — the hook's working directory.
    pub slot: &'a Path,
}

/// What running a phase's hook produced — the structured record the CLI
/// renders. A non-fatal outcome (not declared, post-install failure,
/// consent withheld) is `Ok`; a fatal one is the [`HookError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookReport {
    pub phase: &'static str,
    /// `not-declared` / `ran` / `post-install-failed` / `skipped-needs-consent`.
    pub status: &'static str,
    pub note: Option<String>,
}

/// The hook layer's failure surface (PROP-020). Each message cites the
/// violated unit and a fix surface (Class-F grammar).
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-workspace/PROP-020#phases")]
pub enum HookError {
    #[error(
        "package `{group}/{name}` declares a {phase} hook but is not trusted to run it \
         (violates spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate; \
         fix: allow-list `{group}` in [hooks].allowed_groups or pass --allow-hooks)"
    )]
    Untrusted {
        group: String,
        name: String,
        phase: &'static str,
    },

    #[error(
        "{phase} hook for `{group}/{name}` declares `{base}` but no usable interpreter/script \
         was found (violates spec://vibevm/modules/vibe-workspace/PROP-020#script-selection; \
         fix: ship `{base}.sh` (bash / Git Bash) or `{base}.ps1` (PowerShell) and install the \
         interpreter)"
    )]
    NoInterpreter {
        group: String,
        name: String,
        phase: &'static str,
        base: String,
    },

    #[error(
        "pre-install hook for `{group}/{name}` failed with exit {code} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-020#failure; \
         fix: the slot is rolled back — repair the hook script and reinstall)"
    )]
    PreInstallFailed {
        group: String,
        name: String,
        code: i32,
    },

    #[error(
        "spawning the {phase} hook for `{group}/{name}` failed: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-020#script-selection; \
         fix: ensure the hook script is executable and the interpreter is on PATH)"
    )]
    Spawn {
        group: String,
        name: String,
        phase: &'static str,
        reason: String,
    },
}

/// Probe whether an interpreter program is usable on this host. Seam so
/// tests assert selection without depending on the machine's `PATH`.
pub trait InterpreterProbe {
    fn has(&self, program: &str) -> bool;
}

/// Production probe: spawn `<program> --version`.
pub struct SystemProbe;

impl InterpreterProbe for SystemProbe {
    fn has(&self, program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Execute a hook invocation. Seam so tests drive the failure/exit paths
/// without spawning a real process.
pub trait HookRunner {
    /// Run `inv` in `cwd` with `env` set; return the process exit code, or
    /// an error reason if it could not be spawned/waited.
    fn run(
        &self,
        inv: &HookInvocation,
        cwd: &Path,
        env: &[(String, String)],
    ) -> Result<i32, String>;
}

/// Production runner: spawn the interpreter on the script in the slot.
pub struct SystemHookRunner;

impl HookRunner for SystemHookRunner {
    fn run(
        &self,
        inv: &HookInvocation,
        cwd: &Path,
        env: &[(String, String)],
    ) -> Result<i32, String> {
        let mut cmd = Command::new(&inv.interpreter);
        cmd.arg(&inv.script).current_dir(cwd);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let status = cmd.status().map_err(|e| e.to_string())?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Decide whether a package's hooks may run (PROP-020 §2.3): allow-listed
/// groups (or an explicit `--allow-hooks`) run silently; otherwise an
/// interactive run asks for consent and a non-interactive run refuses,
/// never executing third-party code unseen.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate",
    r = 1
)]
pub fn decide_trust(
    group: &Group,
    allowed: &[String],
    interactive: bool,
    allow_hooks: bool,
) -> HookTrust {
    if allow_hooks || allowed.iter().any(|g| g == group.as_str()) {
        HookTrust::Allowed
    } else if interactive {
        HookTrust::NeedsConsent
    } else {
        HookTrust::Refused
    }
}

/// The hook-execution policy the install pipeline threads from the CLI — the
/// groups whose hooks may run silently (the allow-list plus any group the
/// operator consented to interactively) and the `--allow-hooks` force-all
/// override (PROP-020 §2.3).
///
/// Trust is **already resolved** by the time this reaches the pipeline: the
/// interactive consent prompt and the non-interactive abort both happen in
/// the CLI, *before* apply. So [`HookPolicy::trust_for`] only distinguishes
/// "run" from "skip" — a genuine `Refused` (the one case PROP-020 §2.3
/// forbids running) never reaches the pipeline, because the CLI aborts on it.
#[derive(Debug, Clone, Default)]
pub struct HookPolicy {
    /// Groups whose hooks run with no prompt — the static allow-list
    /// (`DEFAULT_ALLOWED_GROUPS` plus config) unioned with the groups the
    /// operator consented to for this run.
    pub allowed_groups: Vec<String>,
    /// `--allow-hooks` — run every package's hooks regardless of group.
    pub allow_hooks: bool,
}

impl HookPolicy {
    /// The trust under which `group`'s hooks run in the pipeline. An
    /// allow-listed/consented group (or `--allow-hooks`) is [`HookTrust::Allowed`]
    /// and runs; any other group is [`HookTrust::NeedsConsent`], which
    /// [`run_package_hook`] reports as skipped — never silently run, never
    /// fatal. The genuine refusal was already caught in the CLI.
    pub fn trust_for(&self, group: &Group) -> HookTrust {
        if self.allow_hooks || self.allowed_groups.iter().any(|g| g == group.as_str()) {
            HookTrust::Allowed
        } else {
            HookTrust::NeedsConsent
        }
    }
}

/// Resolve the interpreter + script for a phase on a given platform
/// (PROP-020 §2.2). `None` means a script was declared but no usable
/// interpreter/script pair exists — the caller maps that to
/// [`HookError::NoInterpreter`].
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-020#script-selection",
    r = 1
)]
pub fn select_invocation(
    slot: &Path,
    base: &Path,
    platform: Platform,
    probe: &dyn InterpreterProbe,
) -> Option<HookInvocation> {
    let sh = slot.join(base).with_extension("sh");
    let ps1 = slot.join(base).with_extension("ps1");
    let sh_via_bash = || {
        (sh.is_file() && probe.has("bash")).then(|| HookInvocation {
            interpreter: "bash".to_string(),
            script: sh.clone(),
        })
    };
    match platform {
        // Unix: `.sh` via bash, or nothing.
        Platform::Unix => sh_via_bash(),
        // Windows: prefer `.sh` via Git Bash; fall back to `.ps1` via
        // PowerShell when no usable bash/script pair exists.
        Platform::Windows => sh_via_bash().or_else(|| {
            (ps1.is_file() && probe.has("powershell")).then(|| HookInvocation {
                interpreter: "powershell".to_string(),
                script: ps1.clone(),
            })
        }),
    }
}

/// The documented environment a hook receives (PROP-020 §2.2). The publish
/// token is deliberately never placed here.
fn build_env(ctx: &HookContext, phase: HookPhase) -> Vec<(String, String)> {
    vec![
        (
            "VIBE_PACKAGE_GROUP".to_string(),
            ctx.group.as_str().to_string(),
        ),
        ("VIBE_PACKAGE_NAME".to_string(), ctx.name.to_string()),
        ("VIBE_PACKAGE_VERSION".to_string(), ctx.version.to_string()),
        ("VIBE_PACKAGE_KIND".to_string(), ctx.kind.to_string()),
        (
            "VIBE_PACKAGE_DIR".to_string(),
            ctx.slot.display().to_string(),
        ),
        ("VIBE_HOOK_PHASE".to_string(), phase.as_str().to_string()),
    ]
}

/// Run one phase's hook for a package (PROP-020 §2.1/§2.3/§2.5).
///
/// - No hook declared for the phase → `not-declared` (Ok).
/// - Trust [`HookTrust::Refused`] → [`HookError::Untrusted`].
/// - Trust [`HookTrust::NeedsConsent`] → `skipped-needs-consent` (Ok) — the
///   CLI is expected to resolve consent to `Allowed`/`Refused` first; this
///   is the safe library default if it does not.
/// - `pre-install` non-zero exit → [`HookError::PreInstallFailed`] (abort,
///   slot rolled back by the caller).
/// - `post-install` non-zero exit → `post-install-failed` (Ok) — the
///   package is already durable, so the failure is surfaced, not fatal.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-020#phases",
    r = 1
)]
pub fn run_package_hook(
    phase: HookPhase,
    hooks: &HooksDecl,
    ctx: &HookContext,
    trust: HookTrust,
    platform: Platform,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> Result<HookReport, HookError> {
    let Some(base) = phase.base(hooks) else {
        return Ok(HookReport {
            phase: phase.as_str(),
            status: "not-declared",
            note: None,
        });
    };

    match trust {
        HookTrust::Allowed => {}
        HookTrust::NeedsConsent => {
            return Ok(HookReport {
                phase: phase.as_str(),
                status: "skipped-needs-consent",
                note: Some("re-run interactively and consent, or --allow-hooks".to_string()),
            });
        }
        HookTrust::Refused => {
            return Err(HookError::Untrusted {
                group: ctx.group.as_str().to_string(),
                name: ctx.name.to_string(),
                phase: phase.as_str(),
            });
        }
    }

    let Some(inv) = select_invocation(ctx.slot, base, platform, probe) else {
        return Err(HookError::NoInterpreter {
            group: ctx.group.as_str().to_string(),
            name: ctx.name.to_string(),
            phase: phase.as_str(),
            base: base.display().to_string().replace('\\', "/"),
        });
    };

    let env = build_env(ctx, phase);
    let code = runner
        .run(&inv, ctx.slot, &env)
        .map_err(|reason| HookError::Spawn {
            group: ctx.group.as_str().to_string(),
            name: ctx.name.to_string(),
            phase: phase.as_str(),
            reason,
        })?;

    if code != 0 {
        return match phase {
            HookPhase::PreInstall => Err(HookError::PreInstallFailed {
                group: ctx.group.as_str().to_string(),
                name: ctx.name.to_string(),
                code,
            }),
            HookPhase::PostInstall => Ok(HookReport {
                phase: phase.as_str(),
                status: "post-install-failed",
                note: Some(format!("exit {code}; package installed but flagged")),
            }),
        };
    }

    Ok(HookReport {
        phase: phase.as_str(),
        status: "ran",
        note: None,
    })
}

#[cfg(test)]
#[path = "hooks/tests.rs"]
mod tests;
