//! The launch carrier's version gate: ask the installed harness what it is
//! before trusting it to deliver the lane.
//!
//! **Why a gate exists for Codex and not for Claude Code.** Claude Code's
//! slot is a documented command-line flag: pass it to a build that does not
//! have it and the program says so and exits. Codex's slot is a
//! *configuration key*, and a configuration key the program does not read
//! produces no warning, no error and no non-zero exit — the session starts,
//! looks exactly right, and has no boot lane in it. P5 measured precisely
//! that: the same key, in the same file, arrives at the top level and
//! vanishes inside a `[profiles.<name>]` table, with exit 0 either way. A
//! carrier whose failure is unobservable may not be attempted on a version
//! nobody has probed (`PROP-076#SYS-CODEX-PROFILE-TOP-LEVEL`).
//!
//! **What this cell does not do.** It never starts a session, never passes a
//! prompt and never reaches a provider: the probe is `--version`, which
//! `HARNESS-DELIVERY.md` §8.5 already sanctions as the one question a
//! read-only verb may ask a harness binary. A program that cannot be run or
//! whose answer carries no version number yields *unknown*, which is a note
//! and not a verdict against the carrier — "cannot tell" is not "wrong".
//!
//! **The gate decides nothing about the launch itself.** It answers
//! "may this carrier be delivered", and the caller decides what to do with
//! a no: `vibe prompt flags` refuses, because flags for a carrier that may
//! not arrive are the one answer a shell alias turns silently into a
//! session with no lane in it, while `vibe agent` starts the harness
//! anyway, without the carrier and with one loud warning
//! (`#SYS-SOFT-LAUNCH`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SOFT-LAUNCH");

use std::path::Path;
use std::process::Command;

use anyhow::Result;
use vibe_app::SettingsRoot;
use vibe_workspace::system_prompt::{
    CODEX_VERIFIED_VERSION, CarrierVerdict, HarnessKind, codex_carrier_verdict,
    qwen_launch_blocker, reported_version,
};

use crate::commands::agent::launchers::Launcher;

use super::flags::Undeliverable;
use super::supplied_base;

/// What the gate decided about one kind's carrier.
pub(crate) enum Gate {
    /// The carrier may be delivered, with the sentences the caller prints
    /// beside the flags and — for a kind whose carrier is a supplied base —
    /// the composed document itself.
    ///
    /// The composition is returned rather than recomposed by the caller for
    /// one reason: it took a package read, a digest check and a subprocess
    /// to produce, and a second attempt could answer differently.
    Deliver {
        /// Sentences the caller prints beside the flags.
        notes: Vec<String>,
        /// The carrier document, for a kind that has one.
        supplied: Option<supplied_base::Composed>,
    },
    /// The carrier's precondition failed, so it must not be delivered.
    Undeliverable(Undeliverable),
}

/// What the gate is asked about, beyond the kind and the launcher.
///
/// A supplied-base carrier needs the lane's path (it goes in the document)
/// and the launch's own arguments (they may name the model), neither of
/// which a version question has any use for. They travel together so a kind
/// that needs none of them still takes one parameter.
pub(crate) struct GateRequest<'a> {
    /// This machine's settings root, for the installed-package lookup.
    pub(crate) settings: &'a SettingsRoot,
    /// The project the launch would start in, so a probe that discovers
    /// project-level configuration answers about the right project.
    pub(crate) project_root: &'a Path,
    /// The absolute path of the composed lane.
    pub(crate) lane: &'a Path,
    /// The user's own launch arguments, in order.
    pub(crate) launch_args: &'a [String],
}

/// Gate one kind's carrier against the harness this launch would start.
///
/// The reason names the installed version, the verified one, why the
/// difference matters and the exact table a user who has checked it
/// themselves writes.
pub(crate) fn gate(
    kind: HarnessKind,
    launcher: Option<&Launcher>,
    request: &GateRequest<'_>,
) -> Result<Gate> {
    match kind {
        HarnessKind::Claude => return Ok(Gate::deliver()),
        // **The one carrier gated on data rather than on a constant.**
        // Which base an OpenCode launch must be given is a fact about the
        // installed version and this launch's model, recorded in OpenCode's
        // own package; so the gate asks the program what it is and then
        // asks that version's own `PROVENANCE.toml` whether its set has
        // been proved (`#SYS-SUPPLIED-BASE`).
        HarnessKind::OpenCode => return opencode(launcher, request),
        HarnessKind::Qwen => {
            return Ok(Gate::Undeliverable(Undeliverable::hard(
                qwen_launch_blocker(),
                "declare `[system_prompt.harness.qwen] delivery = \"read\"`, which is already \
                 this harness's default, and let the managed block's router have the agent read \
                 the boot files — Qwen Code reads `AGENTS.md` itself, so the block arrives \
                 without a flag"
                    .to_owned(),
            )));
        }
        HarnessKind::Codex => {}
    }
    let name = launcher.map_or(kind.as_str(), |launcher| launcher.name.as_str());
    Ok(match declared_or_probed(launcher) {
        Declared::Stated(version) => match codex_carrier_verdict(&version) {
            CarrierVerdict::Verified => Gate::deliver(),
            CarrierVerdict::Unverified { installed } => Gate::noting(format!(
                "`[launchers.{name}] harness_version = \"{installed}\"` says this Codex \
                 delivers `developer_instructions` from a layered profile file; vibe verified \
                 that on {CODEX_VERIFIED_VERSION} only, so the claim is yours"
            )),
            CarrierVerdict::Unknown { reason } => Gate::Undeliverable(Undeliverable::hard(
                format!(
                    "`[launchers.{name}] harness_version = \"{version}\"` is not a version \
                     number ({reason}), so vibe cannot tell whether this Codex reads the \
                     `developer_instructions` carrier"
                ),
                format!(
                    "write the number `codex --version` prints, such as \
                     \"{CODEX_VERIFIED_VERSION}\""
                ),
            )),
        },
        Declared::Probed(reported) => match codex_carrier_verdict(&reported) {
            CarrierVerdict::Verified => Gate::deliver(),
            CarrierVerdict::Unverified { installed } => {
                Gate::Undeliverable(unverified(name, &installed))
            }
            CarrierVerdict::Unknown { reason } => Gate::noting(format!(
                "this machine's Codex did not say which version it is ({reason}), so the \
                 profile carrier is unverified here; vibe verified it on \
                 {CODEX_VERIFIED_VERSION}. If `codex --version` differs from that, the lane \
                 may silently not arrive — check with `codex debug prompt-input`, which calls \
                 no model"
            )),
        },
        Declared::Unreachable(reason) => Gate::noting(format!(
            "this machine's Codex could not be asked its version ({reason}), so the profile \
             carrier is unverified here; vibe verified it on {CODEX_VERIFIED_VERSION}"
        )),
    })
}

impl Gate {
    /// A carrier that may be delivered with nothing to say.
    fn deliver() -> Self {
        Gate::Deliver {
            notes: Vec::new(),
            supplied: None,
        }
    }

    /// A carrier that may be delivered, beside one sentence.
    fn noting(note: String) -> Self {
        Gate::Deliver {
            notes: vec![note],
            supplied: None,
        }
    }
}

/// Gate OpenCode's carrier: ask the installed program which version it is,
/// then ask that version's own recorded set whether it may be delivered.
///
/// **An unknown version is a refusal here and a note for Codex, and the
/// difference is not an inconsistency.** Codex's carrier is one
/// configuration key that either arrives or does not; the worst an unknown
/// version costs is a session without a lane, which the warning names.
/// OpenCode's carrier *replaces* the harness's own prompt, so a wrong guess
/// does not cost the lane — it costs the harness's instructions, silently,
/// and there is no version-shaped guess that is safe to make
/// (`#SYS-SUPPLIED-BASE`).
fn opencode(launcher: Option<&Launcher>, request: &GateRequest<'_>) -> Result<Gate> {
    let name = launcher.map_or(HarnessKind::OpenCode.as_str(), |launcher| {
        launcher.name.as_str()
    });
    let unknown = |reason: String| {
        Gate::Undeliverable(Undeliverable::hard(
            format!(
                "vibe could not learn which OpenCode this launch would start ({reason}), and \
                 the base it must supply is recorded per harness version, so no set can be \
                 chosen — and a base from the wrong version would replace the harness's own \
                 instructions with no error anywhere"
            ),
            format!(
                "say which version it is with `[launchers.{name}] harness_version = \"<the \
                 number `opencode --version` prints>\"`, or declare \
                 `[system_prompt.harness.opencode] delivery = \"read\"` and let the managed \
                 block's router have the agent read the boot files"
            ),
        ))
    };
    let reported = match declared_or_probed_for(HarnessKind::OpenCode, launcher) {
        Declared::Stated(version) | Declared::Probed(version) => version,
        Declared::Unreachable(reason) => return Ok(unknown(reason)),
    };
    let Some(version) = reported_version(&reported) else {
        return Ok(unknown(format!(
            "`{}` carries no version number",
            reported.trim()
        )));
    };
    let command = program_argv(HarnessKind::OpenCode, launcher);
    let Some((program, standing)) = command.split_first() else {
        return Ok(unknown("its `command` is an empty argv prefix".to_owned()));
    };
    let resolved = match crate::commands::agent::program::resolve(program) {
        Ok(resolved) => resolved,
        Err(error) => return Ok(unknown(format!("`{program}`: {error}"))),
    };
    Ok(
        match supplied_base::carrier(&supplied_base::Request {
            settings: request.settings,
            kind: HarnessKind::OpenCode,
            harness_version: &version,
            program: &resolved,
            standing,
            project_root: request.project_root,
            launch_args: request.launch_args,
            lane: request.lane,
        }) {
            Ok(composed) => Gate::Deliver {
                notes: vec![supplied_base::note(&composed)],
                supplied: Some(composed),
            },
            Err(undeliverable) => Gate::Undeliverable(undeliverable),
        },
    )
}

/// The unverified-version reason, as one sentence per thing the reader has
/// to decide.
fn unverified(name: &str, installed: &str) -> Undeliverable {
    Undeliverable::hard(
        format!(
            "this machine's Codex is {installed} and vibe verified the launch carrier on \
             {CODEX_VERIFIED_VERSION} only. The carrier is the configuration key \
             `developer_instructions`, delivered through a profile file `-p` layers on top of \
             `$CODEX_HOME/config.toml`; a Codex that does not read that key reports nothing \
             and exits 0, which is why an unprobed version is not attempted"
        ),
        format!(
            "check this version offline with `codex debug prompt-input`, which renders the \
             prompt and calls no model — the lane must appear at the top of the second \
             developer message — and then say so with `[launchers.{name}]`'s \
             `harness_version = \"{installed}\"`; or start Codex yourself and pass the flags \
             `vibe prompt flags --harness codex` prints"
        ),
    )
}

/// Where the version came from.
enum Declared {
    /// Its user stated it in the launcher's own table.
    Stated(String),
    /// The program answered `--version`.
    Probed(String),
    /// The program could not be run at all.
    Unreachable(String),
}

/// The argv prefix a launch of this kind would exec: the launcher's own, or
/// the kind's name when there is no launcher.
///
/// A wrapper that selects a different build is the case this exists for, so
/// every probe goes **through** the wrapper rather than around it.
fn program_argv(kind: HarnessKind, launcher: Option<&Launcher>) -> Vec<String> {
    launcher.map_or_else(
        || vec![kind.as_str().to_owned()],
        |launcher| launcher.command.clone(),
    )
}

/// Ask the launcher's own program, standing arguments and all, what version
/// it is — or take the version its user stated instead.
fn declared_or_probed(launcher: Option<&Launcher>) -> Declared {
    declared_or_probed_for(HarnessKind::Codex, launcher)
}

/// The same question for any kind. One probe, one flag, no session: the
/// question is `--version`, which `HARNESS-DELIVERY.md` §8.5 sanctions as
/// the one thing a read-only verb may ask a harness binary.
fn declared_or_probed_for(kind: HarnessKind, launcher: Option<&Launcher>) -> Declared {
    if let Some(version) = launcher.and_then(|launcher| launcher.harness_version.clone()) {
        return Declared::Stated(version);
    }
    let command = program_argv(kind, launcher);
    let Some((program, standing)) = command.split_first() else {
        return Declared::Unreachable("its `command` is an empty argv prefix".to_owned());
    };
    // The same resolution the launch itself performs, or the probe would
    // report «unreachable» for exactly the npm-installed `codex.cmd` or
    // `opencode.cmd` the launch goes on to start (PROP-076
    // `#SYS-HARNESS-EXECUTABLE`).
    let resolved = match crate::commands::agent::program::resolve(program) {
        Ok(resolved) => resolved,
        Err(error) => return Declared::Unreachable(format!("`{program} --version`: {error}")),
    };
    match Command::new(&resolved)
        .args(standing)
        .arg("--version")
        .output()
    {
        Ok(output) => {
            let reported = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if reported.is_empty() {
                Declared::Probed(String::from_utf8_lossy(&output.stderr).trim().to_owned())
            } else {
                Declared::Probed(reported)
            }
        }
        Err(error) => Declared::Unreachable(format!("`{program} --version`: {error}")),
    }
}
