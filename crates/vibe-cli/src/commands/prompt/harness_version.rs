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
//! and not a refusal — refusing on "cannot tell" would make the command
//! unusable on a machine where nothing is wrong.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY");

use std::process::Command;

use anyhow::{Result, bail};
use vibe_workspace::system_prompt::{
    CODEX_VERIFIED_VERSION, CarrierVerdict, HarnessKind, codex_carrier_verdict,
};

use crate::commands::agent::launchers::Launcher;

/// Gate one kind's carrier against the harness this launch would start.
///
/// `Ok(Some(note))` is a sentence the caller prints beside the flags;
/// `Ok(None)` is a verified carrier with nothing to say. The refusal names
/// the installed version, the verified one, why the difference matters and
/// the exact table a user who has checked it themselves writes.
pub(crate) fn gate(kind: HarnessKind, launcher: Option<&Launcher>) -> Result<Option<String>> {
    if kind != HarnessKind::Codex {
        return Ok(None);
    }
    let name = launcher.map_or(kind.as_str(), |launcher| launcher.name.as_str());
    match declared_or_probed(launcher) {
        Declared::Stated(version) => match codex_carrier_verdict(&version) {
            CarrierVerdict::Verified => Ok(None),
            CarrierVerdict::Unverified { installed } => Ok(Some(format!(
                "`[launchers.{name}] harness_version = \"{installed}\"` says this Codex \
                 delivers `developer_instructions` from a layered profile file; vibe verified \
                 that on {CODEX_VERIFIED_VERSION} only, so the claim is yours"
            ))),
            CarrierVerdict::Unknown { reason } => bail!(
                "`[launchers.{name}] harness_version = \"{version}\"` is not a version number \
                 ({reason}) — write the number `codex --version` prints, such as \
                 \"{CODEX_VERIFIED_VERSION}\""
            ),
        },
        Declared::Probed(reported) => match codex_carrier_verdict(&reported) {
            CarrierVerdict::Verified => Ok(None),
            CarrierVerdict::Unverified { installed } => bail!("{}", refusal(name, &installed)),
            CarrierVerdict::Unknown { reason } => Ok(Some(format!(
                "this machine's Codex did not say which version it is ({reason}), so the \
                 profile carrier is unverified here; vibe verified it on \
                 {CODEX_VERIFIED_VERSION}. If `codex --version` differs from that, the lane \
                 may silently not arrive — check with `codex debug prompt-input`, which calls \
                 no model"
            ))),
        },
        Declared::Unreachable(reason) => Ok(Some(format!(
            "this machine's Codex could not be asked its version ({reason}), so the profile \
             carrier is unverified here; vibe verified it on {CODEX_VERIFIED_VERSION}"
        ))),
    }
}

/// The refusal, as one sentence per thing the reader has to decide.
fn refusal(name: &str, installed: &str) -> String {
    format!(
        "this machine's Codex is {installed} and vibe verified the launch carrier on \
         {CODEX_VERIFIED_VERSION} only, so `vibe agent {name}` refuses rather than starting a \
         session that may have no boot lane in it. The carrier is the configuration key \
         `developer_instructions`, delivered through a profile file `-p` layers on top of \
         `$CODEX_HOME/config.toml`; a Codex that does not read that key reports nothing and \
         exits 0, which is why an unprobed version is refused instead of attempted. Two ways \
         forward: check this version offline with `codex debug prompt-input`, which renders \
         the prompt and calls no model — the lane must appear at the top of the second \
         developer message — and then say so with `[launchers.{name}]`'s \
         `harness_version = \"{installed}\"`; or start Codex yourself and pass the flags \
         `vibe prompt flags --harness codex` prints"
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
