//! Output helpers. The CLI has two modes: human-readable (default) and JSON
//! (`--json`). `--quiet` collapses human-readable output to a single summary
//! line. See `VIBEVM-SPEC.md` §9.3.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#output-format");

use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use vibe_core::progress::Progress;
use vibe_core::progress::transfer::Transfers;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::cli::AgentModeArg;

pub(crate) mod language;
mod lines;
mod machine;
mod panel;
mod progress;
pub(crate) mod prompts;
mod structured;
pub(crate) use lines::refusal_of;
use machine::MachineRenderer;
pub(crate) use panel::ProgressPanel;
use progress::ProgressRenderer;
pub(crate) use progress::{ProgressMode, sanitize_progress_text};
pub(crate) use prompts::prompt_theme;
pub(crate) use structured::app_error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Human,
    HumanQuiet,
    Json,
}

/// Resolved provenance for `--invoked-by` / `VIBE_INVOKED_BY`. Drives
/// `vibe show config` reporting and lets tests assert which layer
/// supplied the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokedByProvenance {
    /// `--invoked-by <agent>` was passed on the command line.
    CliFlag,
    /// `VIBE_INVOKED_BY` was set in the environment; CLI flag was absent.
    EnvVar,
    /// Neither layer set the value.
    Default,
}

impl InvokedByProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            InvokedByProvenance::CliFlag => "cli-flag",
            InvokedByProvenance::EnvVar => "env",
            InvokedByProvenance::Default => "default",
        }
    }
}

/// Read the `VIBE_INVOKED_BY` env-var. Empty string is treated as
/// unset so a `VIBE_INVOKED_BY=` literal in `~/.bashrc` does not
/// silently shadow the flag-absent path.
fn env_invoked_by() -> Option<String> {
    std::env::var("VIBE_INVOKED_BY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the `VIBE_APP_LOG` env-var: the fallback for `--log` when the flag
/// is absent (PROP-065 `#display`). Resolved here, with the invocation's
/// other `VIBE_*` posture variables, so the application step display takes
/// its mode as data and reads no environment of its own.
pub fn env_app_log() -> Option<String> {
    env_nonempty("VIBE_APP_LOG")
}

/// Read the `VIBE_APP_LOG_TAIL` env-var: the fallback for `--log-tail`.
pub fn env_app_log_tail() -> Option<String> {
    env_nonempty("VIBE_APP_LOG_TAIL")
}

/// Hand the display language this invocation's terminal (PROP-067
/// `#palette`): attributes only where both streams reach a terminal that
/// draws them, nobody asked for plain text and `NO_COLOR` (any nonempty
/// value, as <https://no-color.org> defines it) is unset; the home a path
/// is spelled `~` against.
pub fn configure_language(both_terminals: bool, may_colour: bool) {
    language::configure(language::Setup {
        colour: both_terminals && may_colour && env_nonempty("NO_COLOR").is_none(),
        one_screen: both_terminals,
        home: env_nonempty("HOME")
            .or_else(|| env_nonempty("USERPROFILE"))
            .map(std::path::PathBuf::from),
    });
}

/// One trimmed, nonempty environment value, or nothing: an exported
/// `VIBE_APP_LOG=` must not shadow the flag-absent default.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Resolve the agent context: CLI flag > env-var > unset. Empty
/// strings on either layer are treated as unset.
pub fn resolve_invoked_by(cli_flag: Option<&str>) -> (Option<String>, InvokedByProvenance) {
    if let Some(flag) = cli_flag {
        let trimmed = flag.trim();
        if !trimmed.is_empty() {
            return (Some(trimmed.to_string()), InvokedByProvenance::CliFlag);
        }
    }
    if let Some(env) = env_invoked_by() {
        return (Some(env), InvokedByProvenance::EnvVar);
    }
    (None, InvokedByProvenance::Default)
}

/// Read the `VIBE_UNATTENDED` env-var. Truthy values are `1`,
/// `true`, `yes`, `on` (case-insensitive, leading/trailing
/// whitespace ignored). Anything else — including the empty
/// string and unset — resolves to `false`. The deliberately small
/// vocabulary matches what cloud-init / systemd-style provisioning
/// scripts already speak.
fn env_unattended() -> bool {
    std::env::var("VIBE_UNATTENDED")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .is_some_and(|s| matches!(s.as_str(), "1" | "true" | "yes" | "on"))
}

/// Resolve the unattended posture: CLI flag wins; otherwise consult
/// the env-var. There is no provenance enum for this — the value is
/// boolean and the source rarely matters in practice (logs carry
/// the resolved value plus an `unattended: true` stamp on every
/// JSON envelope).
pub fn resolve_unattended(cli_flag: bool) -> bool {
    cli_flag || env_unattended()
}

/// Read the `VIBE_OFFLINE` env-var. Truthy values are `1`, `true`,
/// `yes`, `on` (case-insensitive, leading/trailing whitespace
/// ignored). Anything else — including the empty string and unset —
/// resolves to `false`. Same vocabulary as [`env_unattended`], by
/// design: one truthy dialect across every `VIBE_*` posture variable.
fn env_offline() -> bool {
    std::env::var("VIBE_OFFLINE")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .is_some_and(|s| matches!(s.as_str(), "1" | "true" | "yes" | "on"))
}

/// Resolve the offline posture (PROP-010 §2.5): CLI flag wins, then
/// the `VIBE_OFFLINE` env-var, then the user-config `[net].offline`
/// key. Unlike [`resolve_unattended`] the signature carries the
/// config rung — `--offline` has a user-config key and `--unattended`
/// does not — so the whole ladder lives in one function and the
/// precedence cannot drift between call sites. `cli_flag` is the OR
/// of every CLI-level input (the root `--offline` and, for
/// `vibe install`, the PROP-030 §3.1 local `--offline`).
pub fn resolve_offline(cli_flag: bool, config_offline: bool) -> bool {
    cli_flag || env_offline() || config_offline
}

/// Resolve `--agent-mode` (PROP-054 `##AGENT-HANDSHAKE`) exactly once per
/// invocation. An explicit `cli`/`agent` wins outright; `auto` resolves to
/// `agent` exactly when the already-resolved invoked-by value is present —
/// something is hosting this process, so an agent row can be handed back to
/// it — and to `cli` otherwise. `invoked_by` arrives resolved (flag > env),
/// so this function does not re-derive that ladder and the two cannot drift.
pub fn resolve_agent_mode(cli_flag: AgentModeArg, invoked_by: Option<&str>) -> RunAgentMode {
    match cli_flag {
        AgentModeArg::Cli => RunAgentMode::Cli,
        AgentModeArg::Agent => RunAgentMode::Agent,
        AgentModeArg::Auto if invoked_by.is_some() => RunAgentMode::Agent,
        AgentModeArg::Auto => RunAgentMode::Cli,
    }
}

#[derive(Clone)]
pub struct Context {
    pub mode: Mode,
    /// Resolved `--invoked-by` value — `None` when neither flag nor env is set.
    invoked_by: Option<String>,
    /// Where `invoked_by` came from. Surfaced via [`Context::invoked_by_provenance`]
    /// to drive `vibe show config`.
    invoked_by_provenance: InvokedByProvenance,
    /// Resolved unattended posture — `--unattended` flag OR
    /// `VIBE_UNATTENDED` env-var truthy. Implies skip-all-confirms
    /// for every mutating subcommand and stamps the JSON envelope
    /// with `"unattended": true`.
    unattended: bool,
    /// Suppress this context's rendering while preserving its behavioural
    /// mode (`is_json`, unattended, invocation provenance). Lifecycle
    /// prerequisite phases use this for a single final outer report.
    suppress_output: bool,
    /// JSON plan documents held back until the invocation's outcome is known.
    ///
    /// A plan is a PREVIEW of the same contributions the final report carries.
    /// The contract is asymmetric, and deliberately so: a run that PARKS emits
    /// exactly ONE document in total, so the buffered preview is dropped
    /// rather than printed beside the handoff; a run that COMPLETES keeps the
    /// pre-R7.3 stream — the preview is flushed first, then the one registered
    /// root — so a completed `--json` invocation is byte-compatible with what
    /// consumers already parse. Shared through `Arc` so a `quiet_child` writes
    /// into its parent's buffer instead of a private copy that would silently
    /// vanish.
    pending_plans: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
    /// The trailer of a failed run, written after its refusal (PROP-067
    /// `#trailer`); shared like `pending_plans` so a child's survives.
    pending_trailer: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    /// When the invocation began: the trailer's time (PROP-067 `#trailer`).
    started: std::time::Instant,
    /// `--plain`: blocks written once, when they end, and no child stream
    /// between them (PROP-067 `#plain`).
    plain: bool,
    /// The resolved handler-facing agent mode for this whole invocation,
    /// decided ONCE here. Every metadata constructor — ordinary lifecycle,
    /// clean, explicit install, update, reinstall — reads this rather than
    /// hard-coding a value, so the mode cannot drift between the phases of
    /// one command.
    agent_mode: RunAgentMode,
    /// The observation handle threaded into libraries by command adapters.
    progress: Progress,
    /// The invocation's journal, limit and verb for downloads
    /// (`PROP-068`): every progress handle this context gives out carries
    /// them, rendered or not.
    transfers: Option<Transfers>,
    /// The one renderer for this invocation. Kept beside `progress` so
    /// ordinary output and prompts can suspend interactive bars safely.
    progress_renderer: Option<Arc<ProgressRenderer>>,
    /// `--progress-format json`: `progress` feeds the machine renderer.
    machine_progress: bool,
    /// Whether standard input is a terminal this invocation may read keys
    /// from (PROP-065 `#display`); resolved once at the composition root.
    terminal_input: bool,
    pub(crate) agent_user_dirs: vibe_agent_projection::agents::AgentUserDirectories,
    /// Detailed rendering policy belongs only to the CLI composition root.
    #[cfg(test)]
    verbose: bool,
}

impl Context {
    pub fn from_flags(
        quiet: bool,
        json: bool,
        invoked_by_cli: Option<&str>,
        unattended_cli: bool,
        agent_mode_cli: AgentModeArg,
    ) -> Self {
        let (invoked_by, invoked_by_provenance) = resolve_invoked_by(invoked_by_cli);
        let agent_mode = resolve_agent_mode(agent_mode_cli, invoked_by.as_deref());
        let unattended = resolve_unattended(unattended_cli);
        let mode = match (quiet, json) {
            (_, true) => Mode::Json,
            (true, false) => Mode::HumanQuiet,
            (false, false) => Mode::Human,
        };
        Context {
            mode,
            invoked_by,
            invoked_by_provenance,
            unattended,
            suppress_output: false,
            pending_plans: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            pending_trailer: std::sync::Arc::new(std::sync::Mutex::new(None)),
            started: std::time::Instant::now(),
            plain: false,
            agent_mode,
            progress: Progress::default(),
            transfers: None,
            progress_renderer: None,
            machine_progress: false,
            terminal_input: false,
            agent_user_dirs: vibe_agent_projection::agents::AgentUserDirectories::ambient(),
            #[cfg(test)]
            verbose: false,
        }
    }

    /// Selects the invocation's CLI progress renderer after all global flags
    /// have been resolved. Quiet and JSON modes remain completely silent.
    pub fn with_progress(mut self, verbose: bool, mode: ProgressMode) -> Self {
        #[cfg(test)]
        {
            self.verbose = verbose;
        }
        if matches!(self.mode, Mode::Human)
            && !self.suppress_output
            && !matches!(mode, ProgressMode::Disabled)
        {
            let renderer =
                ProgressRenderer::detect(verbose, matches!(mode, ProgressMode::Interactive));
            self.progress = Progress::new(renderer.clone());
            self.progress_renderer = Some(renderer);
        }
        self
    }

    /// Selects the machine renderer (PROP-060 `#MACHINE-STREAM`) in every
    /// output mode; nothing suspends it and it never writes stdout.
    pub fn with_machine_progress(mut self) -> Self {
        if !self.suppress_output {
            self.progress = Progress::new(Arc::new(MachineRenderer::stderr()));
            self.progress_renderer = None;
            self.machine_progress = true;
        }
        self
    }

    /// Whether observations go to the machine renderer.
    pub fn is_machine_progress(&self) -> bool {
        self.machine_progress
    }

    /// Returns the current progress scope for explicitly threaded library
    /// work. Cloning it is an `Arc` clone.
    pub fn progress(&self) -> Progress {
        self.progress.clone().with_transfers(self.transfers.clone())
    }

    /// Attach the invocation's transfers (the composition root's, once).
    pub fn with_transfers(mut self, transfers: Transfers) -> Self {
        self.transfers = Some(transfers);
        self
    }

    pub fn transfers(&self) -> Option<Transfers> {
        self.transfers.clone()
    }

    /// Clones this output context beneath a task-created progress scope.
    /// Rendering policy stays invocation-owned and is not reconstructed.
    pub fn with_progress_scope(&self, progress: Progress) -> Self {
        let mut scoped = self.clone();
        scoped.progress = if scoped.progress_renderer.is_some() || scoped.machine_progress {
            progress
        } else {
            Progress::default()
        };
        scoped
    }

    /// Whether detailed human progress is active for this context.
    #[cfg(test)]
    pub fn is_verbose(&self) -> bool {
        self.verbose && self.progress_renderer.is_some()
    }

    /// Pauses animated progress and plain heartbeats while output or a prompt
    /// uses the terminal; suppressed modes run the closure directly.
    pub fn suspend_progress<R>(&self, render: impl FnOnce() -> R) -> R {
        match &self.progress_renderer {
            Some(renderer) => renderer.suspend(render),
            None => render(),
        }
    }

    /// A child context that performs the same operation under the same
    /// invocation posture but emits nothing of its own.
    pub fn quiet_child(&self) -> Self {
        let mut child = self.clone();
        child.suppress_output = true;
        child.progress = Progress::default();
        child.progress_renderer = None;
        child.machine_progress = false;
        child
    }

    /// Suppress child reports while retaining invocation progress and prompt suspension.
    pub fn progress_child(&self) -> Self {
        let mut child = self.clone();
        child.suppress_output = true;
        child
    }

    pub fn is_json(&self) -> bool {
        self.mode == Mode::Json
    }

    pub fn is_quiet(&self) -> bool {
        self.mode == Mode::HumanQuiet || self.suppress_output
    }

    pub(crate) fn suppresses_output(&self) -> bool {
        self.suppress_output
    }

    pub fn invoked_by(&self) -> Option<&str> {
        self.invoked_by.as_deref()
    }

    pub fn invoked_by_provenance(&self) -> InvokedByProvenance {
        self.invoked_by_provenance
    }

    /// Hold a JSON plan document back until the outcome is known.
    ///
    /// Every plan surface goes through here rather than `emit_json`, so a
    /// parked run can honour "exactly one document IN TOTAL" without each call
    /// site having to predict the outcome it cannot yet see. A completed run
    /// still emits its preview plus one registered root — the deferral only
    /// moves WHEN the preview is written, never whether.
    pub fn defer_json_plan<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        if !self.is_json() {
            return Ok(());
        }
        if self.suppress_output {
            return Ok(());
        }
        let mut value = serde_json::to_value(value)?;
        self.stamp_invoked_by(&mut value);
        self.stamp_unattended(&mut value);
        if let Ok(mut pending) = self.pending_plans.lock() {
            pending.push(value);
        }
        Ok(())
    }

    /// The run completed normally: print the held-back plans, in order,
    /// before its final report.
    pub fn flush_json_plans(&self) -> anyhow::Result<()> {
        for value in self.take_pending_plans() {
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Ok(())
    }

    /// The run parked: the handoff document is the whole output, so the
    /// preview is dropped rather than printed beside it.
    pub fn discard_json_plans(&self) {
        let _ = self.take_pending_plans();
    }

    fn take_pending_plans(&self) -> Vec<Value> {
        self.pending_plans
            .lock()
            .map(|mut pending| std::mem::take(&mut *pending))
            .unwrap_or_default()
    }

    /// The one resolved agent mode for this invocation. Every lifecycle
    /// metadata constructor routes through this; there is no later
    /// hard-coded `Cli`.
    pub fn agent_mode(&self) -> RunAgentMode {
        self.agent_mode.clone()
    }

    /// True when `--unattended` was passed on the CLI or
    /// `VIBE_UNATTENDED` resolves truthy in the environment. Mutating
    /// subcommands generally treat this as an implicit "yes" and refuse
    /// interactive wizards. Durable-environment consent boundaries such as
    /// self import and self doctor --fix still require their explicit grant.
    pub fn is_unattended(&self) -> bool {
        self.unattended
    }

    /// Stamp `invoked_by` on the top-level JSON object when the
    /// resolved context carries a value. No-op for non-objects (vibe
    /// envelopes are always objects, but the function is robust on
    /// scalars / arrays so a stray `Vec<_>` payload does not panic).
    /// The caller's value wins if the inner already set its own
    /// `invoked_by` field — flatten semantics for nested envelopes.
    fn stamp_invoked_by(&self, payload: &mut Value) {
        let Some(invoked_by) = &self.invoked_by else {
            return;
        };
        if let Value::Object(map) = payload {
            map.entry("invoked_by".to_string())
                .or_insert_with(|| Value::String(invoked_by.clone()));
        }
    }

    /// Stamp `"unattended": true` on the top-level JSON object when
    /// the resolved context is unattended. Skipped (not stamped at
    /// all) when the run is interactive — log aggregators see the
    /// field only on scripted runs, which is what they want for
    /// filtering. Same flatten semantics as `stamp_invoked_by`: a
    /// caller-supplied value on the inner payload wins.
    fn stamp_unattended(&self, payload: &mut Value) {
        if !self.unattended {
            return;
        }
        if let Value::Object(map) = payload {
            map.entry("unattended".to_string())
                .or_insert(Value::Bool(true));
        }
    }

    pub fn emit_json<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        if !self.is_json() || self.suppress_output {
            return Ok(());
        }
        let rendered = self.render_json(value)?;
        println!("{rendered}");
        Ok(())
    }

    /// Build the JSON string we'd print, with `invoked_by` and
    /// `unattended` stamped onto the top-level object. Pulled out of
    /// `emit_json` so tests can assert the payload shape without
    /// capturing stdout.
    pub fn render_json<T: Serialize>(&self, value: &T) -> anyhow::Result<String> {
        let mut v = serde_json::to_value(value)?;
        self.stamp_invoked_by(&mut v);
        self.stamp_unattended(&mut v);
        Ok(serde_json::to_string_pretty(&v)?)
    }
}

#[cfg(test)]
#[path = "output/tests.rs"]
mod tests;
