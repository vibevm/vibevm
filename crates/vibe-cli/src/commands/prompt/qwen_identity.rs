//! Composing Qwen Code's identity carrier on this machine (PROP-076
//! `#SYS-QWEN`).
//!
//! **The rule lives in `vibe-workspace`; the machine lives here.**
//! `system_prompt::qwen` classifies the session, judges the preconditions
//! and names the file; this cell finds the installed package, finds the
//! Node the program itself runs on, runs vibe's sampling helper against
//! that package and turns its answer into a carrier or into one sentence
//! saying why there is none.
//!
//! **Why a helper and why JavaScript.** The one fact a launch needs —
//! Qwen Code's own identity sentence for the session shape it is about to
//! start — can only be had by *calling* the installed program's own
//! `getCoreSystemPrompt`. Writing that sentence down in vibe would be
//! exactly the hard-coded copy of someone else's prompt that
//! `#SYS-SUPPLIED-BASE` exists to prevent, and it would be stale on the
//! first upgrade. So vibe ships a small program of its own, carrying no word
//! of anyone's prompt, and runs it against the package that is installed.
//!
//! **Why this cell writes a file although `vibe prompt flags` prints.** The
//! carrier's *value* **is** a path, and an answer naming a file that is not
//! there is not an answer: Qwen Code re-reads it on every prompt rebuild and
//! throws when it is missing. That is the one way it differs from Codex's
//! profile, whose value is a profile *name* that `vibe agent` can write the
//! file for at launch. The write is content-addressed, idempotent, inside
//! vibe's own settings tree, and never replaces a file another session may
//! still be reading.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-QWEN");

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use vibe_app::SettingsRoot;
use vibe_workspace::system_prompt::{
    QWEN_CARRIER_DIR, QWEN_IDENTITY_ENV, QWEN_SYSTEM_MD_ENV, QWEN_VERIFIED_VERSION, QwenAmbient,
    QwenCarrier, QwenSession, qwen_carrier_is_ours, qwen_carrier_prefix, qwen_mode,
    qwen_precondition, qwen_styled,
};

use super::flags::Undeliverable;

/// The npm package the installed `qwen` program belongs to.
///
/// **Packaging identity, never prompt text.** It is how a resolved program
/// is matched to the directory whose own `getCoreSystemPrompt` the helper
/// must call, and a directory that does not identify itself as this package
/// at the version the program reported is refused rather than sampled.
const QWEN_PACKAGE: &str = "@qwen-code/qwen-code";

/// vibe's own sampling helper, shipped inside the binary and written to the
/// settings tree at launch.
const HELPER: &str = include_str!("qwen_identity/carrier.mjs");

/// `QWEN_WRITE_SYSTEM_MD` — the variable that makes Qwen Code *dump* its
/// base prompt to a file. Removed from the helper's environment, because an
/// inherited one would turn a sampling call into a file write.
const QWEN_WRITE_SYSTEM_MD_ENV: &str = "QWEN_WRITE_SYSTEM_MD";

/// What one composition needs: the machine, the program it would start, and
/// the launch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Request<'a> {
    /// This machine's settings root; the carrier files live under it.
    pub(crate) settings: &'a SettingsRoot,
    /// The version the installed program reported.
    pub(crate) harness_version: &'a str,
    /// The resolved program a launch would exec.
    pub(crate) program: &'a Path,
    /// The project the launch would start in, so the helper samples with the
    /// same working directory the session will have.
    pub(crate) project_root: &'a Path,
    /// The user's own launch arguments, in order.
    pub(crate) launch_args: &'a [String],
    /// The absolute path of the composed lane.
    pub(crate) lane: &'a Path,
}

/// A composed carrier and the facts a note reports it with.
#[derive(Debug, Clone)]
pub(crate) struct Composed {
    /// The carrier itself.
    pub(crate) carrier: QwenCarrier,
    /// The package whose own function sampled the sentence.
    pub(crate) package: PathBuf,
    /// Whether this launch wrote the file or found it already there.
    pub(crate) written: bool,
}

/// Compose this launch's carrier, or say why it cannot be.
///
/// The order of the questions is the order of the answers a reader can act
/// on: the version first, because an unverified program makes every later
/// answer beside the point; then what the user asked for that the carrier
/// would contradict; then which session shape this is; and only then the
/// machine — the package, the Node, the helper and its proof.
pub(crate) fn carrier(request: &Request<'_>) -> Result<Composed, Undeliverable> {
    let Request {
        settings,
        harness_version,
        program,
        project_root,
        launch_args,
        lane,
    } = *request;
    if harness_version != QWEN_VERIFIED_VERSION {
        return Err(unverified(harness_version));
    }
    let dir = settings.path().join(QWEN_CARRIER_DIR);
    let args: Vec<&str> = launch_args.iter().map(String::as_str).collect();
    let system_md = ambient(QWEN_SYSTEM_MD_ENV);
    let identity_md = ambient(QWEN_IDENTITY_ENV);
    let ambient = QwenAmbient {
        system_md: system_md.as_deref(),
        identity_md: identity_md.as_deref(),
    };
    if let Some(reason) = qwen_precondition(&args, &ambient, &dir) {
        return Err(Undeliverable::hard(
            reason,
            "drop that argument or variable to have this project's boot lane delivered, or \
             declare `[system_prompt.harness.qwen] delivery = \"read\"` and let the managed \
             block's router have the agent read the boot files — Qwen Code reads `AGENTS.md` \
             itself, so the block arrives either way"
                .to_owned(),
        ));
    }
    let mode = qwen_mode(&QwenSession {
        args: &args,
        stdin_is_tty: std::io::stdin().is_terminal(),
    })
    .map_err(|reason| {
        Undeliverable::hard(
            reason,
            "say which it is — `-p <prompt>` for one non-interactive turn, `-i <prompt>` for a \
             terminal session — or run the subcommand without vibe in front of it"
                .to_owned(),
        )
    })?;
    let styled = qwen_styled(&args);
    let package = package_dir(program, harness_version).map_err(|reason| {
        Undeliverable::hard(
            format!(
                "vibe could not find the installed Qwen Code package behind `{}` ({reason}), and \
                 the identity sentence this launch must carry is sampled from that package's own \
                 `getCoreSystemPrompt` rather than stored anywhere",
                program.display()
            ),
            "install Qwen Code so its `qwen` program resolves into its own npm package \
             directory, or declare `[system_prompt.harness.qwen] delivery = \"read\"`"
                .to_owned(),
        )
    })?;
    let node = node_program(&package).map_err(|reason| {
        Undeliverable::hard(
            format!(
                "vibe found the installed Qwen Code package but no Node to run its own code with \
                 ({reason}); the identity sentence can only be had by calling that package's own \
                 `getCoreSystemPrompt`"
            ),
            "put the `node` that runs Qwen Code on `PATH`, or declare \
             `[system_prompt.harness.qwen] delivery = \"read\"`"
                .to_owned(),
        )
    })?;
    let helper = write_helper(&dir).map_err(|reason| {
        Undeliverable::hard(
            format!("vibe's own sampling helper could not be written ({reason})"),
            format!("make `{}` writable", dir.display()),
        )
    })?;
    let prefix = qwen_carrier_prefix(&dir, mode, styled);
    let answer = run_helper(
        &node,
        &helper,
        &package,
        project_root,
        lane,
        &prefix,
        mode,
        styled,
    )
    .map_err(|reason| {
        Undeliverable::hard(
            format!(
                "the installed Qwen Code {harness_version} did not pass the carrier's own \
                     check ({reason}), so vibe will not put a file in its identity slot"
            ),
            "run `vibe prompt flags --harness qwen` for the whole answer, or declare \
                 `[system_prompt.harness.qwen] delivery = \"read\"` and let the managed block's \
                 router have the agent read the boot files"
                .to_owned(),
        )
    })?;
    let path = PathBuf::from(&answer.carrier);
    if !path.starts_with(&dir) || !path.is_file() {
        return Err(Undeliverable::hard(
            format!(
                "the carrier file the helper reported (`{}`) is not a file inside `{}`",
                answer.carrier,
                dir.display()
            ),
            "report this as a vibe defect; nothing was delivered".to_owned(),
        ));
    }
    Ok(Composed {
        carrier: QwenCarrier {
            env: QWEN_IDENTITY_ENV.to_owned(),
            path,
            mode,
            styled,
            identity_bytes: answer.identity_bytes,
            carrier_bytes: answer.carrier_bytes,
            version: harness_version.to_owned(),
        },
        package,
        written: answer.written,
    })
}

/// The reason an unverified Qwen Code is not delivered to.
///
/// **A refusal rather than a note**, which is the same judgement Codex's
/// gate makes for a different reason: a version that replaces more of the
/// prompt than its first paragraph, or stops honouring the variable, fails
/// **silently** — the session starts and simply has no lane in it, or has
/// the harness's own instructions swapped. Neither is a thing to discover
/// after a day's work.
fn unverified(installed: &str) -> Undeliverable {
    Undeliverable::hard(
        format!(
            "this machine's Qwen Code is {installed} and vibe measured the identity carrier on \
             {QWEN_VERIFIED_VERSION} only. The carrier is `{QWEN_IDENTITY_ENV}`, which on that \
             version replaces the base prompt's first paragraph and nothing else; a version that \
             replaced more, or stopped reading the variable, would swap the harness's own \
             instructions or drop the lane with no error anywhere"
        ),
        "re-verify this version offline with `campaigns/sys-prompt-2026-11/live/\
         qwen-identity-prove.sh` in the vibevm repository and raise `QWEN_VERIFIED_VERSION`, or \
         declare `[system_prompt.harness.qwen] delivery = \"read\"` and let the managed block's \
         router have the agent read the boot files"
            .to_owned(),
    )
}

/// One of Qwen Code's own variables as this process inherited it.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `QWEN_SYSTEM_MD` and `QWEN_SYSTEM_IDENTITY_MD` \
              are the HARNESS's own variables, and the fact being asked is \
              what the child will inherit at the moment it is about to be \
              started — a different fact from anything a composition root \
              could have captured, since an outer vibe-launched session may \
              have set one of them. The rule over the values \
              (`qwen_precondition`) is pure and lives in `vibe-workspace`; \
              this is the read, exactly as `program::resolve`'s `PATH` read \
              in the same command family"
)]
fn ambient(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Whether a `QWEN_SYSTEM_IDENTITY_MD` this process inherited is one of
/// **ours**, and therefore must be removed from the child's environment on a
/// launch that delivers nothing.
///
/// An outer vibe-launched session's carrier riding into a session vibe just
/// declined to deliver to would be the one thing worse than delivering
/// nothing: a lane the warning said was not delivered.
pub(crate) fn inherited_carrier_to_scrub(settings: &SettingsRoot) -> Option<&'static str> {
    let value = ambient(QWEN_IDENTITY_ENV)?;
    qwen_carrier_is_ours(&value, &settings.path().join(QWEN_CARRIER_DIR))
        .then_some(QWEN_IDENTITY_ENV)
}

/// The installed package directory behind a resolved `qwen` program.
///
/// **Three candidates, each a real npm layout, and every one is checked
/// against the package's own `package.json`.** A directory that does not
/// name itself `@qwen-code/qwen-code` at the version the program just
/// reported is not sampled: handing the helper some other installation's
/// code would sample a sentence from a program this launch is not starting.
///
/// 1. the realpath of the program's own target, whose parent is the package
///    when `bin/qwen` is npm's symlink into it (the POSIX global install);
/// 2. `<dir of the program>/node_modules/<package>` — a `qwen.cmd` shim in
///    an npm bin directory, which is the Windows layout, and a local
///    `node_modules/.bin` install;
/// 3. `<dir of the program>/../lib/node_modules/<package>` — the POSIX
///    prefix layout, for a program that is a wrapper script rather than a
///    symlink.
fn package_dir(program: &Path, version: &str) -> Result<PathBuf, String> {
    let resolved = std::fs::canonicalize(program).unwrap_or_else(|_| program.to_path_buf());
    let mut candidates = Vec::new();
    if let Some(parent) = resolved.parent() {
        candidates.push(parent.to_path_buf());
    }
    if let Some(bin) = program.parent() {
        candidates.push(bin.join("node_modules").join(QWEN_PACKAGE));
        candidates.push(
            bin.join("..")
                .join("lib")
                .join("node_modules")
                .join(QWEN_PACKAGE),
        );
    }
    for candidate in &candidates {
        if identifies_as(candidate, version) {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "none of {} identifies itself as `{QWEN_PACKAGE}` {version} in its own `package.json`",
        candidates
            .iter()
            .map(|candidate| format!("`{}`", candidate.display()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Whether a directory is this package at this version, by its own
/// `package.json` and nothing else.
fn identifies_as(dir: &Path, version: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    parsed.get("name").and_then(serde_json::Value::as_str) == Some(QWEN_PACKAGE)
        && parsed.get("version").and_then(serde_json::Value::as_str) == Some(version)
}

/// The Node the helper runs under.
///
/// **The one Qwen Code itself runs on, when the layout says which that is.**
/// A global npm install puts the package at `<prefix>/lib/node_modules/…`
/// and the interpreter at `<prefix>/bin/node`, so the package's own path
/// names its Node — and on a machine with several Node versions managed side
/// by side that is the only way to pick the right one, since `PATH` may name
/// another. The Windows global layout puts `node_modules` directly in the
/// prefix beside `node.exe`, which is the second candidate. Otherwise `node`
/// is resolved on `PATH` exactly as the launch resolves `qwen`, which is
/// also what the program's own `#!/usr/bin/env node` shebang would do.
fn node_program(package: &Path) -> Result<PathBuf, String> {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    for ancestor in package.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) != Some("node_modules") {
            continue;
        }
        let Some(parent) = ancestor.parent() else {
            continue;
        };
        let prefix = if parent.file_name().and_then(|name| name.to_str()) == Some("lib") {
            parent.parent()
        } else {
            Some(parent)
        };
        if let Some(prefix) = prefix {
            for candidate in [prefix.join("bin").join(executable), prefix.join(executable)] {
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }
    crate::commands::agent::program::resolve("node").map_err(|error| error.to_string())
}

/// Write vibe's sampling helper into the carrier directory, content-
/// addressed, and answer its path.
///
/// Content-addressed for the reason the carrier file is: a launch must never
/// rewrite a file another process may be reading, and a build of vibe with a
/// different helper in it must not be able to pick up this one.
fn write_helper(dir: &Path) -> Result<PathBuf, String> {
    let digest = format!(
        "{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(HELPER.as_bytes())
    );
    let path = dir.join(format!("carrier-{}.mjs", &digest[..16]));
    if std::fs::read_to_string(&path).is_ok_and(|current| current == HELPER) {
        return Ok(path);
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let temporary = dir.join(format!(
        "carrier-{}.{}.tmp",
        &digest[..16],
        std::process::id()
    ));
    std::fs::write(&temporary, HELPER).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

/// What the helper answered.
struct Answer {
    carrier: String,
    identity_bytes: usize,
    carrier_bytes: usize,
    written: bool,
}

/// Run the helper against the installed package and read its one JSON line.
///
/// It calls nothing and reaches nothing: the helper imports the package's own
/// chunk, calls one string-building function four times and writes one file.
/// The harness's own three prompt variables are removed from its
/// environment, because an inherited one would change the sentence it
/// samples — and `QWEN_WRITE_SYSTEM_MD` would turn the call into a dump.
#[allow(clippy::too_many_arguments)]
fn run_helper(
    node: &Path,
    helper: &Path,
    package: &Path,
    project_root: &Path,
    lane: &Path,
    prefix: &Path,
    mode: vibe_workspace::system_prompt::QwenMode,
    styled: bool,
) -> Result<Answer, String> {
    let mut command = Command::new(node);
    command
        .arg(helper)
        .arg(package)
        .arg("--lane")
        .arg(lane)
        .arg("--out-prefix")
        .arg(prefix)
        .arg("--mode")
        .arg(mode.as_str())
        .current_dir(project_root)
        .env_remove(QWEN_SYSTEM_MD_ENV)
        .env_remove(QWEN_IDENTITY_ENV)
        .env_remove(QWEN_WRITE_SYSTEM_MD_ENV);
    if styled {
        command.arg("--styled");
    }
    let output = command
        .output()
        .map_err(|error| format!("`{}`: {error}", node.display()))?;
    let printed = String::from_utf8_lossy(&output.stdout);
    let line = printed.lines().last().unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(line).map_err(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let said = if line.is_empty() { &*stderr } else { line };
        format!(
            "it answered no verdict: {}",
            said.chars().take(300).collect::<String>()
        )
    })?;
    let field = |name: &str| {
        parsed
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as usize
    };
    if parsed.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(parsed
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("no reason given")
            .to_owned());
    }
    Ok(Answer {
        carrier: parsed
            .get("carrier")
            .and_then(serde_json::Value::as_str)
            .ok_or("it reported no carrier file")?
            .to_owned(),
        identity_bytes: field("identityBytes"),
        carrier_bytes: field("carrierBytes"),
        written: parsed
            .get("written")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_default(),
    })
}

/// The one sentence a note says a composed carrier's *provenance* with. The
/// carrier itself is described by `qwen_carrier_note`, which is the same
/// sentence under both deliveries.
pub(crate) fn note(composed: &Composed) -> String {
    format!(
        "the identity sentence was sampled from `{}` — the installed Qwen Code {} package — at \
         this launch, and the carrier's own check then confirmed on that package that the \
         variable yields the plain prompt with ONE insertion in it, for a gated and an ungated \
         tool surface alike. The {}-byte file {}",
        composed.package.display(),
        composed.carrier.version,
        composed.carrier.carrier_bytes,
        if composed.written {
            "was written by this launch"
        } else {
            "was already there, byte for byte, from an earlier launch of the same lane"
        }
    )
}

#[cfg(test)]
#[path = "qwen_identity/tests.rs"]
mod tests;
