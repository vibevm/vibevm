//! Qwen Code's launch carrier — the **identity sentence** slot — and the
//! three pure questions a launch through it must answer before it may be
//! used (PROP-076 `#SYS-QWEN`).
//!
//! **What the carrier is.** `QWEN_SYSTEM_IDENTITY_MD` names a file whose
//! contents replace the base prompt's **first paragraph** — its one identity
//! sentence — and nothing else; upstream's own tests pin both halves («should
//! replace only the identity sentence when identity env points to a file»,
//! «keeps the style section under a `QWEN_SYSTEM_IDENTITY_MD` override»). So
//! the file vibe writes is Qwen Code's **own** identity sentence, a blank
//! line, and the lane — and the prompt the program then builds is its own
//! prompt with one insertion in it.
//!
//! **Why that is clean where `QWEN_SYSTEM_MD` was not.** The whole base
//! prompt is a function of the session — the interaction mode, the tool
//! surface, the model, whether the directory is a git repository, whether a
//! sandbox is set — and a launcher cannot know it without starting a second
//! session, which is what closed that slot. The identity **sentence** is a
//! function of two things only, the interaction mode and whether an output
//! style is active, and vibe never writes it down: it is sampled at launch
//! from the **installed** package's own `getCoreSystemPrompt`, exactly as
//! upstream's own test samples it, and everything else in the prompt Qwen
//! Code still builds live for the session it is actually starting.
//!
//! **This cell is pure.** The classification, the preconditions and the
//! carrier file's name are decided here over values; the sampling, the
//! subprocess and the file live in the command layer, which can reach a
//! program, a settings tree and an environment.
//!
//! Every answer here is scoped to [`QWEN_VERIFIED_VERSION`], because each is
//! a reading of one program's own option table and prompt assembly rather
//! than a floor.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-QWEN");

use std::fmt;
use std::path::{Path, PathBuf};

/// The environment variable the carrier travels in: a **path**, never the
/// lane's bytes (`#SYS-HARNESS-DELIVERY`).
pub const QWEN_IDENTITY_ENV: &str = "QWEN_SYSTEM_IDENTITY_MD";

/// The variable that replaces the **whole** base prompt. While it is active
/// Qwen Code never reaches the identity slot at all, which upstream's own
/// test pins («should ignore identity env when `QWEN_SYSTEM_MD` is set»).
pub const QWEN_SYSTEM_MD_ENV: &str = "QWEN_SYSTEM_MD";

/// The one Qwen Code version whose identity carrier this build has measured,
/// offline and at the real lane size.
///
/// **One version, not a floor**, for the reason Codex's constant is one: the
/// claim is a measurement of how one program assembles a prompt, and the
/// failure mode of a carrier that stops being honoured is **silence** — the
/// session starts, looks right, and has no lane in it. An unverified version
/// soft-launches rather than being attempted.
pub const QWEN_VERIFIED_VERSION: &str = "0.27.2";

/// The directory, under vibe's settings root, that holds the carrier files.
pub const QWEN_CARRIER_DIR: &str = "agent/qwen";

/// Which identity sentence a launch will have.
///
/// Qwen Code's own three — `resolveInteractionMode`'s ACP > interactive >
/// headless precedence — because the sentence names the mode's role and the
/// three differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenMode {
    /// A terminal session: `-i <prompt>`, or no prompt at all with a tty.
    Interactive,
    /// One non-interactive turn: `-p <prompt>`, or anything without a tty.
    Headless,
    /// An editor or agent host: `--acp`, or `--input-format stream-json`.
    Acp,
}

impl QwenMode {
    /// The spelling Qwen Code's own `SystemPromptInteractionMode` uses,
    /// which is also what the sampling helper is given.
    pub fn as_str(self) -> &'static str {
        match self {
            QwenMode::Interactive => "interactive",
            QwenMode::Headless => "headless",
            QwenMode::Acp => "acp",
        }
    }
}

impl fmt::Display for QwenMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What the classification is asked about: the launch's own arguments and
/// whether the launch's standard input is a terminal.
///
/// A struct because the terminal is an **ambient** fact the caller reads and
/// this cell must not: a classification that read `isatty` itself could not
/// be tested, and `vibe prompt flags` and `vibe agent` are two different
/// sessions asking the same question.
#[derive(Debug, Clone, Copy)]
pub struct QwenSession<'a> {
    /// The user's own arguments, in order, exactly as they will be passed.
    pub args: &'a [&'a str],
    /// Whether the launch's standard input is a terminal.
    pub stdin_is_tty: bool,
}

/// Qwen Code's value-taking options on [`QWEN_VERIFIED_VERSION`], long
/// spellings and aliases.
///
/// **Read off its own option table**, not guessed: every entry of
/// `TOP_LEVEL_GLOBAL_OPTIONS` declared `string`, `number` or `array`, plus
/// each one's alias. It exists for one question — is this bare word a
/// previous option's **value** or the session's prompt — and the direction
/// of an error is deliberate: a missing entry turns a bare word into an
/// unclassifiable session and a soft launch, while a wrong extra entry would
/// swallow a real prompt. So the list errs by being short.
const VALUE_OPTIONS: &[&str] = &[
    "--telemetry-target",
    "--telemetry-otlp-endpoint",
    "--telemetry-otlp-protocol",
    "--telemetry-outfile",
    "--proxy",
    "--model",
    "-m",
    "--advisor",
    "--workflow-mode",
    "--fallback-model",
    "--prompt",
    "-p",
    "--prompt-interactive",
    "-i",
    "--system-prompt",
    "--append-system-prompt",
    "--output-style",
    "--sandbox",
    "-s",
    "--sandbox-image",
    "--approval-mode",
    "--channel",
    "--allowed-mcp-server-names",
    "--disable-mcp-server",
    "--mcp-config",
    "--allowed-tools",
    "--extensions",
    "-e",
    "--include-directories",
    "--add-dir",
    "--openai-logging-dir",
    "--openai-api-key",
    "--openai-base-url",
    "--input-format",
    "--output-format",
    "-o",
    "--json-fd",
    "--json-file",
    "--json-schema",
    "--input-file",
    "--resume",
    "-r",
    "--session-id",
    "--worktree",
    "--max-session-turns",
    "--max-wall-time",
    "--max-tool-calls",
    "--max-subagent-depth",
    "--core-tools",
    "--exclude-tools",
    "--disabled-slash-commands",
    "--auth-type",
];

/// One option's value in a launch's arguments: `--name value`, `--name=value`
/// or, for a one-letter alias, both of those.
///
/// `None` when the option is absent; `Some("")` when it is present with no
/// value, which Qwen Code reads as falsy — and which therefore decides a
/// different branch from an absent option in one place, `-i`.
fn option_value<'a>(args: &[&'a str], names: &[&str]) -> Option<&'a str> {
    let mut found = None;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index];
        if names.contains(&argument) {
            // The next token is the value, unless it is another option or
            // there is none: `qwen -i` is the option with no value, which
            // Qwen Code reads as falsy.
            match args.get(index + 1) {
                Some(value) if !value.starts_with('-') => {
                    found = Some(*value);
                    index += 1;
                }
                _ => found = Some(""),
            }
        } else if let Some(value) = names
            .iter()
            .find_map(|name| argument.strip_prefix(&format!("{name}=")))
        {
            found = Some(value);
        }
        index += 1;
    }
    found
}

/// Whether a boolean option is **on**, in either of the two spellings a
/// shell may write it.
///
/// `--name=false` and `--name=0` are off, because that is what the harness's
/// own argument parser makes of them — and a launch that read `--acp=false`
/// as ACP would compose the sentence for a session shape nobody asked for.
fn flag_present(args: &[&str], name: &str) -> bool {
    args.iter().any(|argument| {
        if *argument == name {
            return true;
        }
        argument
            .strip_prefix(&format!("{name}="))
            .is_some_and(|value| !matches!(value.to_ascii_lowercase().as_str(), "false" | "0"))
    })
}

/// The first bare word in the arguments that is neither an option nor an
/// option's value — Qwen Code's positional `query`, or a subcommand, and
/// this cell cannot tell which.
fn bare_word<'a>(args: &[&'a str]) -> Option<&'a str> {
    let mut previous: Option<&str> = None;
    for argument in args {
        if argument.starts_with('-') {
            previous = Some(argument);
            continue;
        }
        let consumed = previous.is_some_and(|previous| VALUE_OPTIONS.contains(&previous));
        previous = Some(argument);
        if !consumed {
            return Some(argument);
        }
    }
    None
}

/// Which identity sentence this launch needs, or why the question cannot be
/// answered.
///
/// **Qwen Code's own rule, and nothing of our own.**
/// `resolveInteractionMode` is ACP first, then `isInteractiveInvocation`:
/// interactive when `--prompt-interactive` carries a value, or when there is
/// no prompt, no positional query **and** standard input is a terminal;
/// headless otherwise.
///
/// **The one answer that is not a mode.** A bare word on the command line is
/// either the session's prompt — in which case the session is headless — or
/// a subcommand, in which case it is not a prompt at all and a session with a
/// terminal is interactive. Nothing short of Qwen Code's own command table
/// decides which, so with a terminal present the mode is **unknown** and the
/// launch is soft: handing over the wrong sentence is the failure this whole
/// mechanism exists to avoid. Without a terminal both readings are headless,
/// so the word costs nothing there.
///
/// ```
/// use vibe_workspace::system_prompt::{QwenMode, QwenSession, qwen_mode};
///
/// let tty = |args: &[&str]| qwen_mode(&QwenSession { args, stdin_is_tty: true });
/// let piped = |args: &[&str]| qwen_mode(&QwenSession { args, stdin_is_tty: false });
///
/// assert_eq!(tty(&[]), Ok(QwenMode::Interactive));
/// assert_eq!(tty(&["-i", "do the thing"]), Ok(QwenMode::Interactive));
/// assert_eq!(tty(&["-p", "one turn"]), Ok(QwenMode::Headless));
/// assert_eq!(piped(&[]), Ok(QwenMode::Headless));
/// assert_eq!(tty(&["--acp"]), Ok(QwenMode::Acp));
/// assert_eq!(tty(&["--input-format", "stream-json"]), Ok(QwenMode::Acp));
/// // `-m` takes a value, so `fake-1` is not a prompt.
/// assert_eq!(tty(&["-m", "fake-1"]), Ok(QwenMode::Interactive));
/// // A bare word with a terminal: prompt or subcommand, nobody can say.
/// assert!(tty(&["mcp"]).is_err());
/// assert_eq!(piped(&["mcp"]), Ok(QwenMode::Headless));
/// ```
pub fn qwen_mode(session: &QwenSession<'_>) -> Result<QwenMode, String> {
    let args = session.args;
    if flag_present(args, "--acp") || flag_present(args, "--experimental-acp") {
        return Ok(QwenMode::Acp);
    }
    if option_value(args, &["--input-format"]) == Some("stream-json") {
        return Ok(QwenMode::Acp);
    }
    if option_value(args, &["--prompt-interactive", "-i"]).is_some_and(|value| !value.is_empty()) {
        return Ok(QwenMode::Interactive);
    }
    let prompted = option_value(args, &["--prompt", "-p"]).is_some_and(|value| !value.is_empty());
    let bare = bare_word(args);
    if prompted {
        return Ok(QwenMode::Headless);
    }
    match (bare, session.stdin_is_tty) {
        (None, true) => Ok(QwenMode::Interactive),
        (None | Some(_), false) => Ok(QwenMode::Headless),
        (Some(word), true) => Err(format!(
            "vibe cannot tell whether `{word}` is this session's prompt or a subcommand, and \
             Qwen Code's identity sentence differs between the two session shapes — a headless \
             sentence in an interactive session would silently misdescribe the session to the \
             model"
        )),
    }
}

/// Whether an output style is active for this launch, as far as a launcher
/// can tell.
///
/// Qwen Code's identity sentence has a second variable: with a style active
/// it points at the style instead of at software engineering. `--output-style
/// <name>` is the one spelling a **launcher** can read — `default` selects no
/// style, which the flag's own help text says — and a style configured in
/// `general.outputStyle` is **not** knowable here without reimplementing
/// another program's settings precedence, which `#SYS-QWEN` refuses. The
/// residual that leaves is stated rather than guessed at: a session with a
/// configured style receives Qwen Code's unstyled identity sentence, one
/// clause of one sentence, and the style's own section still lands, which
/// upstream's own test pins.
pub fn qwen_styled(args: &[&str]) -> bool {
    option_value(args, &["--output-style"])
        .is_some_and(|value| !value.is_empty() && !value.eq_ignore_ascii_case("default"))
}

/// What a launch knows about the environment it would hand to Qwen Code.
///
/// Read by the caller, as [`QwenSession::stdin_is_tty`] is, so the rule over
/// it stays pure.
#[derive(Debug, Clone, Copy, Default)]
pub struct QwenAmbient<'a> {
    /// `QWEN_SYSTEM_MD`, if set.
    pub system_md: Option<&'a str>,
    /// `QWEN_SYSTEM_IDENTITY_MD`, if set — possibly ours, inherited from an
    /// outer vibe-launched session.
    pub identity_md: Option<&'a str>,
}

/// Whether `QWEN_SYSTEM_MD` is **active**, by Qwen Code's own reading of it:
/// unset, empty, `0` or `false` is off, and anything else — including `1` and
/// `true`, which mean `~/.qwen/system.md` — is on.
fn system_md_active(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        let value = value.trim();
        !matches!(value.to_ascii_lowercase().as_str(), "" | "0" | "false")
    })
}

/// Whether a `QWEN_SYSTEM_IDENTITY_MD` already in the environment is one of
/// **ours**, inherited from an outer vibe-launched session.
///
/// It decides two different things: such a value is not a foreign owner of
/// the slot and so does not stop a launch, and it must be **removed** from
/// the child's environment on a launch that delivers nothing, or an outer
/// session's carrier would ride into a session vibe just declined to deliver
/// to.
pub fn qwen_carrier_is_ours(value: &str, carrier_dir: &Path) -> bool {
    Path::new(value.trim()).starts_with(carrier_dir)
}

/// Why this launch must not use the identity carrier, as one clause, or
/// `None` when nothing is in the way.
///
/// **Every entry is a soft launch and none is a refusal**, which is the one
/// place this kind differs from `opencode`'s `--agent`: each of these is a
/// user asking Qwen Code for something vibe's carrier would contradict, and
/// honouring the user while saying what was not delivered is better than
/// refusing to start their session (`#SYS-SOFT-LAUNCH`).
///
/// ```
/// use std::path::Path;
/// use vibe_workspace::system_prompt::{QwenAmbient, qwen_precondition};
///
/// let dir = Path::new("/home/u/.vibe/agent/qwen");
/// let clean = QwenAmbient::default();
/// assert!(qwen_precondition(&[], &clean, dir).is_none());
/// assert!(qwen_precondition(&["--safe-mode"], &clean, dir).is_some());
/// assert!(qwen_precondition(&["--bare"], &clean, dir).is_some());
/// assert!(qwen_precondition(&["--system-prompt", "mine.md"], &clean, dir).is_some());
///
/// let replaced = QwenAmbient { system_md: Some("1"), identity_md: None };
/// assert!(qwen_precondition(&[], &replaced, dir).is_some());
/// // Ours, inherited from an outer launch: not a foreign owner.
/// let inherited = QwenAmbient {
///     system_md: None,
///     identity_md: Some("/home/u/.vibe/agent/qwen/qwen-identity-interactive-abc.md"),
/// };
/// assert!(qwen_precondition(&[], &inherited, dir).is_none());
/// let foreign = QwenAmbient { system_md: None, identity_md: Some("/etc/brand.md") };
/// assert!(qwen_precondition(&[], &foreign, dir).is_some());
/// ```
pub fn qwen_precondition(
    args: &[&str],
    ambient: &QwenAmbient<'_>,
    carrier_dir: &Path,
) -> Option<String> {
    if option_value(args, &["--system-prompt"]).is_some() {
        return Some(
            "`--system-prompt` replaces the whole prompt the identity sentence lives in, so a \
             carrier in it would be deleted rather than delivered"
                .to_owned(),
        );
    }
    if flag_present(args, "--safe-mode") {
        return Some(
            "`--safe-mode` asks Qwen Code to disable every customization — context files, \
             hooks, extensions, skills, MCP servers — and this project's boot lane is one; \
             delivering it anyway would be the opposite of what was asked"
                .to_owned(),
        );
    }
    if flag_present(args, "--bare") {
        return Some(
            "`--bare` asks Qwen Code to honour only the inputs given explicitly on its command \
             line, and vibe's carrier is neither on it nor the user's own"
                .to_owned(),
        );
    }
    if system_md_active(ambient.system_md) {
        return Some(format!(
            "`{QWEN_SYSTEM_MD_ENV}` is set, and while it is Qwen Code replaces the whole base \
             prompt and never reads the identity slot at all — its own tests pin that"
        ));
    }
    if let Some(identity) = ambient.identity_md
        && !identity.trim().is_empty()
        && !qwen_carrier_is_ours(identity, carrier_dir)
    {
        return Some(format!(
            "`{QWEN_IDENTITY_ENV}` already names a file that is not vibe's (`{identity}`), so \
             something else owns this session's identity sentence and vibe will not overwrite it"
        ));
    }
    None
}

/// The carrier file's path **prefix** for one launch: everything but the
/// content digest and the extension.
///
/// **Content-addressed, and that is a correctness requirement rather than a
/// convenience.** A running Qwen Code session re-reads this file on **every**
/// prompt rebuild and throws when it is missing, so the file must be
/// persistent and must never be rewritten under a live session: a later
/// launch carrying a different lane writes a **different** file. The digest
/// the command layer appends is of the carrier's own bytes, which it has and
/// this cell does not — so what is decided here is the shape of the name, and
/// the shape carries the two facts a reader needs to tell two files apart.
///
/// ```
/// use std::path::Path;
/// use vibe_workspace::system_prompt::{QwenMode, qwen_carrier_prefix};
///
/// let dir = Path::new("/home/u/.vibe/agent/qwen");
/// assert_eq!(
///     qwen_carrier_prefix(dir, QwenMode::Interactive, false),
///     Path::new("/home/u/.vibe/agent/qwen/qwen-identity-interactive-")
/// );
/// assert_eq!(
///     qwen_carrier_prefix(dir, QwenMode::Headless, true),
///     Path::new("/home/u/.vibe/agent/qwen/qwen-identity-headless-styled-")
/// );
/// ```
pub fn qwen_carrier_prefix(dir: &Path, mode: QwenMode, styled: bool) -> PathBuf {
    let styled = if styled { "-styled" } else { "" };
    dir.join(format!("qwen-identity-{}{styled}-", mode.as_str()))
}

/// One launch's composed identity carrier: the file, and the measurements a
/// note reports it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QwenCarrier {
    /// The variable the path travels in — always [`QWEN_IDENTITY_ENV`], held
    /// as a field so the one place that spells another program's variable
    /// name is the composition rather than every reader of it.
    pub env: String,
    /// The carrier file: Qwen Code's own identity sentence, a blank line,
    /// and the lane.
    pub path: PathBuf,
    /// The session shape its sentence was sampled for.
    pub mode: QwenMode,
    /// Whether the styled sentence was sampled.
    pub styled: bool,
    /// How many bytes of Qwen Code's own sentence went in front of the lane.
    pub identity_bytes: usize,
    /// The whole file's size.
    pub carrier_bytes: usize,
    /// The installed Qwen Code's version, as it reported it.
    pub version: String,
}

/// The one sentence a note reports a composed carrier with.
pub fn qwen_carrier_note(carrier: &QwenCarrier, lane: &Path) -> String {
    format!(
        "`{}` names a {}-byte file: Qwen Code's OWN identity sentence for the {} session shape \
         ({} bytes, sampled at this launch from the installed {} package's own \
         `getCoreSystemPrompt`, never stored), a blank line, and `{}` — the lane's bytes in a \
         file, never on a command line or in an environment value. Qwen Code replaces its base \
         prompt's FIRST PARAGRAPH with that file and builds every other byte of the prompt \
         itself, for the session it is actually starting; a subagent never enters that path and \
         receives none of it",
        carrier.env,
        carrier.carrier_bytes,
        carrier.mode,
        carrier.identity_bytes,
        carrier.version,
        lane.display()
    )
}

#[cfg(test)]
#[path = "qwen/tests.rs"]
mod tests;
