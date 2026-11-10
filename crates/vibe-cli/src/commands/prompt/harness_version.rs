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

use std::process::Command;

use anyhow::Result;
use vibe_workspace::system_prompt::{
    CODEX_VERIFIED_VERSION, CarrierVerdict, HarnessKind, codex_carrier_verdict,
    opencode_launch_blocker,
};

use crate::commands::agent::launchers::Launcher;

use super::flags::Undeliverable;

/// What the gate decided about one kind's carrier.
pub(crate) enum Gate {
    /// The carrier may be delivered. `Some(note)` is a sentence the caller
    /// prints beside the flags; `None` is a verified carrier with nothing
    /// to say.
    Deliver(Option<String>),
    /// The carrier's precondition failed, so it must not be delivered.
    Undeliverable(Undeliverable),
}

/// Gate one kind's carrier against the harness this launch would start.
///
/// The reason names the installed version, the verified one, why the
/// difference matters and the exact table a user who has checked it
/// themselves writes.
pub(crate) fn gate(kind: HarnessKind, launcher: Option<&Launcher>) -> Result<Gate> {
    match kind {
        HarnessKind::Claude => return Ok(Gate::Deliver(None)),
        // **The one precondition that no version satisfies yet.** It is not
        // asked of the installed program at all, because the blocker is in
        // how OpenCode assembles a prompt rather than in which build does
        // it: probing a version to refuse either way would be a subprocess
        // spent on a decision already made (`#SYS-OPENCODE`).
        HarnessKind::OpenCode => {
            return Ok(Gate::Undeliverable(Undeliverable::hard(
                opencode_launch_blocker(),
                "declare `[system_prompt.harness.opencode] delivery = \"read\"`, which is \
                 already this harness's default, and let the managed block's router have the \
                 agent read the boot files"
                    .to_owned(),
            )));
        }
        HarnessKind::Codex => {}
    }
    let name = launcher.map_or(kind.as_str(), |launcher| launcher.name.as_str());
    Ok(match declared_or_probed(launcher) {
        Declared::Stated(version) => match codex_carrier_verdict(&version) {
            CarrierVerdict::Verified => Gate::Deliver(None),
            CarrierVerdict::Unverified { installed } => Gate::Deliver(Some(format!(
                "`[launchers.{name}] harness_version = \"{installed}\"` says this Codex \
                 delivers `developer_instructions` from a layered profile file; vibe verified \
                 that on {CODEX_VERIFIED_VERSION} only, so the claim is yours"
            ))),
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
            CarrierVerdict::Verified => Gate::Deliver(None),
            CarrierVerdict::Unverified { installed } => {
                Gate::Undeliverable(unverified(name, &installed))
            }
            CarrierVerdict::Unknown { reason } => Gate::Deliver(Some(format!(
                "this machine's Codex did not say which version it is ({reason}), so the \
                 profile carrier is unverified here; vibe verified it on \
                 {CODEX_VERIFIED_VERSION}. If `codex --version` differs from that, the lane \
                 may silently not arrive — check with `codex debug prompt-input`, which calls \
                 no model"
            ))),
        },
        Declared::Unreachable(reason) => Gate::Deliver(Some(format!(
            "this machine's Codex could not be asked its version ({reason}), so the profile \
             carrier is unverified here; vibe verified it on {CODEX_VERIFIED_VERSION}"
        ))),
    })
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

/// Ask the launcher's own program, standing arguments and all: a wrapper
/// that selects a different Codex build is the case this exists for, so the
/// probe must go through the wrapper rather than around it.
fn declared_or_probed(launcher: Option<&Launcher>) -> Declared {
    if let Some(version) = launcher.and_then(|launcher| launcher.harness_version.clone()) {
        return Declared::Stated(version);
    }
    let command = launcher.map_or_else(
        || vec![HarnessKind::Codex.as_str().to_owned()],
        |launcher| launcher.command.clone(),
    );
    let Some((program, standing)) = command.split_first() else {
        return Declared::Unreachable("its `command` is an empty argv prefix".to_owned());
    };
    // The same resolution the launch itself performs, or the probe would
    // report «unreachable» for exactly the npm-installed `codex.cmd` the
    // launch goes on to start (PROP-076 `#SYS-HARNESS-EXECUTABLE`).
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
