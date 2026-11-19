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
    CodexProfile, EnvAssignment, HarnessKind, build_remedy, host_mismatch_warning,
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

    let mut native_arguments = launcher.command.iter().skip(1).cloned().collect::<Vec<_>>();
    native_arguments.extend(rest.iter().cloned());
    if launcher.kind == HarnessKind::Hermes
        && let Some(request) = super::native::hermes_ready_session::parse(&native_arguments)?
    {
        let program =
            super::program::locate(launcher.command.first().context("empty Hermes launcher")?)?;
        // Explicit transport is not prompt delivery: never compose, bind a
        // plugin, or silently enter this route after a delivery failure.
        std::process::exit(super::native::hermes_ready_session::run(
            settings.path(),
            &program,
            request,
        )?);
    }
    // READY above is local-only attachment. Every other unqualified harness
    // must refuse an explicitly selected corporate route before native effects.
    admit_route_kind(launcher.kind, &native_arguments)?;
    let native_selection = if launcher.kind == HarnessKind::Hermes
        && super::native::hermes_selected::requested(&native_arguments)
    {
        let program =
            super::program::locate(launcher.command.first().context("empty Hermes launcher")?)?;
        Some(super::native::hermes_selected::Selection::start(
            settings.path(),
            &program,
            &std::env::current_dir()?,
            &native_arguments,
        )?)
    } else {
        None
    };
    let effective_cwd = native_selection
        .as_ref()
        .map(|selection| selection.cwd().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // **The workspace is opened after the launcher is resolved, and a
    // directory that is not a project is a soft launch.** `vibe agent` is a
    // launcher before it is a delivery mechanism, so it has to work in a
    // scratch directory too; refusing there would be the same defect P9
    // found in the opted-out branch, one directory wider.
    let selected = match prompt::select(&effective_cwd, None) {
        Ok(selected) => selected,
        Err(error) => {
            if let Some(selection) = native_selection {
                eprintln!(
                    "warning: selected Hermes project prefix unavailable; native read delivery only ({error})"
                );
                std::process::exit(selection.launch(None)?);
            }
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
        if let Some(selection) = native_selection {
            eprintln!(
                "warning: selected Hermes project prefix could not be built; native read delivery only ({error})"
            );
            std::process::exit(selection.launch(None)?);
        }
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
            if let Some(selection) = native_selection {
                eprintln!(
                    "warning: selected Hermes project prefix unavailable; native read delivery only ({error})"
                );
                std::process::exit(selection.launch(None)?);
            }
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
        if let Some(selection) = native_selection {
            std::process::exit(selection.launch(None)?);
        }
        if !ctx.is_quiet() {
            eprintln!(
                "note: this project has not opted in to a system prompt — starting `{name}` \
                 unchanged; the agent reads its boot lane from disk"
            );
        }
        let mut command: Vec<String> = launcher.command.clone();
        command.extend(rest);
        return exec(launcher.kind, &command, &[], &scrub(launcher.kind));
    }

    if let Some(selection) = native_selection {
        let prefix = if launcher.kind.delivery(&selected.manifest)
            == vibe_core::manifest::HarnessDelivery::Launch
        {
            Some(prompt::native_launch_lane(ctx, &selected)?)
        } else {
            None
        };
        std::process::exit(selection.launch(prefix.as_deref())?);
    }

    if matches!(launcher.kind, HarnessKind::Pi | HarnessKind::Hermes)
        && launcher.kind.delivery(&selected.manifest)
            == vibe_core::manifest::HarnessDelivery::Launch
    {
        let prefix = prompt::native_launch_lane(ctx, &selected)?;
        let (program, standing) = launcher
            .command
            .split_first()
            .context("empty native Pi launcher")?;
        let program = super::program::locate(program)?;
        let mut arguments = standing.to_vec();
        arguments.extend(rest);
        let prepared = if launcher.kind == HarnessKind::Hermes {
            super::native::prepare_hermes(
                settings.path(),
                &program,
                &prefix,
                &selected.node_dir,
                &arguments,
            )?
        } else {
            super::native::prepare_pi(
                settings.path(),
                &program,
                &prefix,
                &selected.node_dir,
                &arguments,
            )?
        };
        return exec(launcher.kind, &prepared.command, &[], &[]);
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
    let composed =
        match prompt::compose_launch_flags(ctx, &selected, launcher.kind, Some(&launcher), &rest) {
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
    if !composed.flags.is_empty() || !composed.env.is_empty() {
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
    let scrub = if composed.env.is_empty() {
        scrub(launcher.kind)
    } else {
        Vec::new()
    };
    exec(launcher.kind, &command, &composed.env, &scrub)
}

/// Variables of **vibe's own** that must not ride into a child this launch
/// delivers nothing to.
///
/// **One carrier needs this and the others cannot.** Codex's profile is
/// selected by a flag and OpenCode's document travels in a variable whose
/// value *is* the document, so a launch that passes neither delivers
/// nothing by construction. Qwen Code's carrier is a **path**, and a path
/// an outer vibe-launched session exported is still valid in this child —
/// so a session the warning just told the developer has no boot lane would
/// quietly have the outer session's one. Only a file inside vibe's own
/// carrier directory is removed; a `QWEN_SYSTEM_IDENTITY_MD` of the user's
/// own is theirs (`#SYS-QWEN`, `#SYS-SOFT-LAUNCH`).
fn scrub(kind: HarnessKind) -> Vec<&'static str> {
    if kind != HarnessKind::Qwen {
        return Vec::new();
    }
    global::settings_root()
        .ok()
        .and_then(|settings| prompt::inherited_carrier_to_scrub(&settings))
        .into_iter()
        .collect()
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
    exec(launcher.kind, &command, &[], &scrub(launcher.kind))
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
fn exec(
    kind: HarnessKind,
    command: &[String],
    env: &[EnvAssignment],
    scrub: &[&str],
) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    let mut spawn = Command::new(program);
    spawn.args(arguments);
    for name in scrub {
        spawn.env_remove(name);
    }
    for assignment in env {
        spawn.env(&assignment.name, &assignment.value);
    }
    if let Some(code) = route_child(kind, &mut spawn)? {
        std::process::exit(code);
    }
    let error = spawn.exec();
    Err(anyhow::Error::new(error).context(format!("starting `{program}`")))
}

#[cfg(not(unix))]
fn exec(
    kind: HarnessKind,
    command: &[String],
    env: &[EnvAssignment],
    scrub: &[&str],
) -> Result<()> {
    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    // The RESOLVED file, not the word: `Command::new("claude")` finds only
    // `claude.exe`, and a `.bat` / `.cmd` target is handed to the standard
    // library whole so its own CVE-2024-24576 argument escaping applies.
    let resolved =
        super::program::resolve(program).with_context(|| format!("starting `{program}`"))?;
    let mut spawn = Command::new(&resolved);
    spawn.args(arguments);
    for name in scrub {
        spawn.env_remove(name);
    }
    for assignment in env {
        spawn.env(&assignment.name, &assignment.value);
    }
    if let Some(code) = route_child(kind, &mut spawn)? {
        std::process::exit(code);
    }
    let status = spawn
        .status()
        .with_context(|| format!("starting `{program}`"))?;
    std::process::exit(status.code().unwrap_or(1));
}

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

fn admit_route_kind(kind: HarnessKind, arguments: &[String]) -> Result<()> {
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
    if kind != HarnessKind::Pi && configured {
        bail!(
            "configured corporate proxy routing is not yet qualified for harness `{}`; no agent was started (use a qualified Pi launcher or retain native/unmanaged invocation explicitly)",
            kind.as_str()
        );
    }
    Ok(())
}

/// Admitted native routes share one lifetime owner; other adapters remain separate work.
fn route_child(kind: HarnessKind, command: &mut Command) -> Result<Option<i32>> {
    if !matches!(
        kind,
        HarnessKind::Pi | HarnessKind::OpenCode | HarnessKind::Codex | HarnessKind::Qwen
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
