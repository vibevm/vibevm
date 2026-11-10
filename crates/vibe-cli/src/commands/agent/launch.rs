//! `vibe agent <launcher> [args…]` — start one launcher with this project's
//! composed boot lane already in the harness's own instruction slot.
//!
//! **A thin layer over `vibe prompt flags`, deliberately.** There is one
//! code path that decides which flags a kind takes, and this cell adds only
//! what printing cannot do: it recomposes the lane first, writes Codex's
//! launch-time profile file, refuses a user argument that would conflict
//! with the slot flag, says so when this machine is not the declared
//! `host`, and then becomes the harness (decision D10). In a project that
//! has not opted in it passes no flag and starts the harness unchanged.
//!
//! **The lane is recomposed at every launch**, which is how band U keeps the
//! "current bytes" meaning `#SYS-NO-FROZEN-INDEX` gives it: a mid-session
//! edit of a user boot snippet is seen at the next launch, exactly as it is
//! today once the agent has read it (`#SYS-AGENT-LANE`).
//!
//! **No lane bytes go on a command line.** A single argument is capped at
//! 128 KiB on Linux and the whole command line at about 32,767 characters on
//! Windows, against a lane of roughly 310 KB, so every mechanism here is
//! file-based (`#SYS-HARNESS-DELIVERY`).
//!
//! **Every agent starts softly.** A launcher's job is to start a harness,
//! and vibe's inability to deliver a lane is not a reason to leave a
//! developer without a session. So every delivery failure — a build that
//! failed, a lane that is absent or stale after it, a carrier precondition
//! that did not hold — starts the harness with **no** slot flag and **no**
//! carrier, which is the classic boot the managed block's router already
//! handles, behind one loud warning `--quiet` does not suppress. What
//! remains a refusal is what is not about delivery at all: a project
//! manifest naming a launcher, an unknown launcher name, and a slot flag of
//! the user's own when vibe is about to pass its own
//! (`#SYS-SOFT-LAUNCH`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SOFT-LAUNCH");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_app::{AgentsConfig, AppError};
use vibe_core::manifest::TargetOs;
use vibe_workspace::boot_artifacts::build_node_prompt;
use vibe_workspace::system_prompt::{
    CodexProfile, HarnessKind, build_remedy, host_mismatch_warning,
};

use crate::commands::app::global;
use crate::commands::prompt;
use crate::output;

use super::launchers;

pub(crate) fn run(ctx: &output::Context, argv: Vec<String>) -> Result<()> {
    let mut argv = argv.into_iter();
    let Some(name) = argv.next() else {
        bail!("`vibe agent` needs a launcher name; `vibe agent list` names the ones you have");
    };
    let rest: Vec<String> = argv.collect();

    // A repository must never choose which program runs here, and this is
    // asked **before** the workspace is opened: the manifest parser would
    // reject the table as an unknown field, and "unknown field `agent`" is
    // not the sentence a contributor needs to read.
    launchers::refuse_project_launchers(Path::new("."))?;
    let settings = global::settings_root()?;
    let config = AgentsConfig::load(&settings).map_err(AppError::from)?;
    let launcher = launchers::resolve(&settings, &config, &name)?;

    // **The workspace is opened after the launcher is resolved, and a
    // directory that is not a project is a soft launch.** `vibe agent` is a
    // launcher before it is a delivery mechanism, so it has to work in a
    // scratch directory too; refusing there would be the same defect P9
    // found in the opted-out branch, one directory wider.
    let selected = match prompt::select(&PathBuf::from("."), None) {
        Ok(selected) => selected,
        Err(error) => {
            return soft(
                &launcher,
                &rest,
                &prompt::Undeliverable::hard(
                    format!("this directory is not a vibe project ({error:#})"),
                    "run `vibe agent` from inside a vibe project, or `vibe init` here".to_owned(),
                ),
            );
        }
    };

    // Recompose first, so the slot carries the files as they are now.
    let built = build_node_prompt(&selected.workspace, &selected.node_rel);
    if !ctx.is_quiet() {
        eprintln!("{}", prompt::summary_line(&built));
    }
    // **A failed build is a soft launch, and never an older lane.** The
    // file that happens to be on disk is the previous build's, or half of
    // this one's; delivering it would put bytes in the slot that are not
    // this project's boot lane, which is worse than delivering none.
    if let Err(error) = built {
        return soft(
            &launcher,
            &rest,
            &prompt::Undeliverable::hard(
                format!("the boot prefix could not be built ({error})"),
                format!("run `{}`", build_remedy(&selected.node_rel)),
            ),
        );
    }

    // A project that never opted in has no lane to put in the slot. That is
    // not a delivery failure but an absence of anything to deliver, so it
    // stays a note rather than a warning: the harness boots exactly as it
    // would without `vibe`, by reading the files its instruction file's
    // block names, and a launcher stays usable in every project.
    let opted_in = match prompt::opted_in(&selected) {
        Ok(opted_in) => opted_in,
        Err(error) => {
            return soft(
                &launcher,
                &rest,
                &prompt::Undeliverable::hard(
                    format!("this node's prefix could not be read ({error:#})"),
                    format!("run `{}`", build_remedy(&selected.node_rel)),
                ),
            );
        }
    };
    if !opted_in {
        if !ctx.is_quiet() {
            eprintln!(
                "note: this project has not opted in to a system prompt — starting `{name}` \
                 unchanged; the agent reads its boot lane from disk"
            );
        }
        let mut command: Vec<String> = launcher.command.clone();
        command.extend(rest);
        return exec(&command);
    }

    // **The host mismatch, as a note and nothing more.** `vibe agent`
    // starts a session on *this* machine, so it is the one launch surface
    // that can see what `vibe prompt check` would say about it — and a
    // worker who never runs `check` is exactly the reader the sentence is
    // for. PROP-076 `#SYS-HOST-AND-TARGET` names only `check` and
    // `prepare`, so this is a deliberate addition rather than a stated
    // requirement, and it is kept to the weakest form that still informs:
    // stderr, no flag, no JSON document of its own (this verb has none —
    // it becomes the harness), and never a refusal. Refusing here would be
    // the precise failure the decision forbids, a cross-platform worker
    // unable to start an agent in its own project.
    let mismatch = host_mismatch_warning(
        selected
            .manifest
            .system_prompt
            .as_ref()
            .map(|section| section.host)
            .unwrap_or_default(),
        TargetOs::current(),
    );
    if let Some(warning) = mismatch
        && !ctx.is_quiet()
    {
        eprintln!("note: {warning}");
    }

    // **Any answer but "deliver this" starts the harness bare.** The
    // composition's own refusals are reached only by the surface that
    // prints — `vibe agent` has already asked the opt-in question — so an
    // error here is a reason nobody foresaw, and a reason nobody foresaw is
    // still not a reason to leave a developer without a session.
    let composed = match prompt::compose_flags(&selected, launcher.kind, Some(&launcher)) {
        Ok(composed) => composed,
        Err(error) => {
            return soft(
                &launcher,
                &rest,
                &prompt::Undeliverable::hard(
                    format!("the launch flags could not be composed ({error:#})"),
                    format!("run `{}`", build_remedy(&selected.node_rel)),
                ),
            );
        }
    };
    if let Some(undeliverable) = &composed.undeliverable {
        return soft(&launcher, &rest, undeliverable);
    }
    let composed = composed.flags;
    // **C2 — `vibe agent` injects only for a harness in `launch` delivery**,
    // so the conflict question is asked only when vibe is about to pass the
    // slot flag itself. In `read` delivery it passes none, and a user's own
    // slot flag then conflicts with nothing — the same reasoning the
    // opted-out branch above already applies (`HARNESS-DELIVERY.md` §4).
    if !composed.flags.is_empty() {
        refuse_conflicting(launcher.kind, &rest)?;
    }
    for note in &composed.notes {
        if !ctx.is_quiet() {
            eprintln!("note: {note}");
        }
    }
    if launcher.kind == HarnessKind::Claude && resumes(&rest) && !ctx.is_quiet() {
        eprintln!("note: {RESUME_NOTE}");
    }
    if let Some(profile) = &composed.profile {
        write_profile(profile)?;
    }
    let mut command: Vec<String> = launcher.command.clone();
    command.extend(composed.flags.iter().cloned());
    command.extend(rest);
    exec(&command)
}

/// Start the harness with no slot flag and no carrier, behind one loud
/// warning.
///
/// **The warning is not suppressed by `--quiet`, and that is the whole
/// reason it is a warning rather than a note.** `--quiet` asks for less
/// narration about what vibe did; this sentence is about what the *agent*
/// receives — a session without its boot lane, which the developer is about
/// to spend tokens in. Hiding it would make the two launches
/// indistinguishable at the terminal, and the failure mode of the carriers
/// this guards is already silence.
///
/// No conflict check runs here: vibe passes no slot flag, so a slot flag of
/// the user's own conflicts with nothing — the same reasoning the opted-out
/// and `read`-delivery branches apply (`HARNESS-DELIVERY.md` §4, C2).
fn soft(
    launcher: &launchers::Launcher,
    rest: &[String],
    undeliverable: &prompt::Undeliverable,
) -> Result<()> {
    eprintln!(
        "warning: vibe cannot deliver this project's boot lane to `{}`, so the harness starts \
         WITHOUT it and the agent reads its boot files from disk — the classic boot the managed \
         block's router already handles.\n\
         warning:   reason: {}\n\
         warning:   fix: {}\n\
         warning: `--quiet` does not suppress this: it is about what the agent receives, not \
         about what vibe says.",
        launcher.name, undeliverable.reason, undeliverable.fix
    );
    let mut command: Vec<String> = launcher.command.clone();
    command.extend(rest.iter().cloned());
    exec(&command)
}

/// Measured in the P9 live check (row C7, Claude Code 2.1.322): a resumed
/// session resends the system prompt it was started with, so a lane edited
/// since then does not arrive, and the router cannot notice a changed band-U
/// file because its block carries only the static lane's id.
const RESUME_NOTE: &str = "a resumed Claude Code session keeps the boot lane it started with; \
     boot files edited since then reach the agent only in a new session";

/// Whether the user's own arguments resume an earlier session.
fn resumes(rest: &[String]) -> bool {
    rest.iter().any(|argument| {
        matches!(argument.as_str(), "-c" | "--continue" | "-r" | "--resume")
            || argument.starts_with("--resume=")
    })
}

/// Refuse a user argument that would land a second system prompt in the
/// slot. Passing two is not an error the harness reports — it is a session
/// that silently takes a prompt nobody chose.
fn refuse_conflicting(kind: HarnessKind, rest: &[String]) -> Result<()> {
    for flag in kind.conflicting_flags() {
        if rest
            .iter()
            .any(|argument| argument == flag || argument.starts_with(&format!("{flag}=")))
        {
            bail!(
                "`{flag}` is already in your own arguments, and `vibe agent` passes the slot \
                 flag for kind `{}` itself — drop one of the two, or use `vibe prompt flags \
                 --harness {}` and compose the command line yourself",
                kind.as_str(),
                kind.as_str()
            );
        }
    }
    Ok(())
}

/// Write Codex's launch-time profile file into `$CODEX_HOME`.
///
/// Written by `vibe agent` and by nothing else — never by an install, which
/// has no business writing outside the project, and never by `vibe prompt
/// flags`, which only prints. `$CODEX_HOME` defaults to `~/.codex`, as Codex
/// itself resolves it (decision D4).
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `$CODEX_HOME` is the harness's OWN variable, and \
              the file must land where the Codex this launch is about to \
              exec will look for it. Resolving it at the composition root \
              and threading it in would hand the write a value captured \
              before the launch was decided — a different fact from \
              `where will this child read its profile`, and the one place \
              where reading it later than the root is the correctness \
              requirement rather than a convenience"
)]
fn write_profile(profile: &CodexProfile) -> Result<()> {
    let home = match std::env::var_os("CODEX_HOME") {
        Some(home) => PathBuf::from(home),
        None => dirs::home_dir()
            .context("resolving the home directory for `$CODEX_HOME`")?
            .join(".codex"),
    };
    std::fs::create_dir_all(&home).with_context(|| format!("creating `{}`", home.display()))?;
    let path = home.join(&profile.file_name);
    std::fs::write(&path, &profile.body).with_context(|| format!("writing `{}`", path.display()))
}

/// Become the harness.
///
/// On a POSIX host this process is **replaced**, so the harness inherits the
/// terminal, the signals and the exit status with nothing of vibe's left in
/// the way. `execvp` searches `PATH` itself, so a bare program name needs no
/// resolution here. Windows has no `exec`: spawn, wait, and exit with the
/// child's own code, which is the same observable contract by the only means
/// the platform offers — and there a bare name has to be resolved through
/// `PATHEXT` first, or an npm-installed `claude.cmd` is «program not found»
/// (`super::program`).
#[cfg(unix)]
fn exec(command: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    let error = Command::new(program).args(arguments).exec();
    Err(anyhow::Error::new(error).context(format!("starting `{program}`")))
}

#[cfg(not(unix))]
fn exec(command: &[String]) -> Result<()> {
    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    // The RESOLVED file, not the word: `Command::new("claude")` finds only
    // `claude.exe`, and a `.bat` / `.cmd` target is handed to the standard
    // library whole so its own CVE-2024-24576 argument escaping applies.
    let resolved =
        super::program::resolve(program).with_context(|| format!("starting `{program}`"))?;
    let status = Command::new(&resolved)
        .args(arguments)
        .status()
        .with_context(|| format!("starting `{program}`"))?;
    std::process::exit(status.code().unwrap_or(1));
}
