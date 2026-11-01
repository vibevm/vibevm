//! The step panel of an application install, update or materialisation
//! (PROP-065 `#display`): on an interactive terminal the running step keeps
//! a fold of its last child-output lines, `l` opens and closes the fold, a
//! child that takes the terminal gets it whole, and a failing step's log
//! tail is printed before its error. Every other context stays flat.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vibe_app::{AppError, StepError};

use crate::cli::{AppLogArg, AppStepArgs};
use crate::output;

/// The default fold: the last five lines of the running step.
const DEFAULT_TAIL: usize = 5;

/// How many lines of a failing step's log are printed before its error.
const FAILURE_TAIL: usize = 30;

/// The resolved child-output policy of one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogMode {
    /// Stream every line; on a panel the fold starts open.
    Live,
    /// Fold the last lines under the running step where a terminal can
    /// draw one, stream live where it cannot.
    Tail,
    /// No child output at all, only each step's log path.
    Quiet,
}

/// Resolve `--log` over `VIBE_APP_LOG`, then the default for this context:
/// a fold on an interactive terminal, the live stream anywhere else.
pub(crate) fn log_mode(
    flag: Option<AppLogArg>,
    environment: Option<&str>,
    interactive: bool,
) -> LogMode {
    let named = flag.or_else(|| match environment?.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(AppLogArg::Auto),
        "live" => Some(AppLogArg::Live),
        "tail" => Some(AppLogArg::Tail),
        "quiet" => Some(AppLogArg::Quiet),
        _ => None,
    });
    match named.unwrap_or(AppLogArg::Auto) {
        AppLogArg::Live => LogMode::Live,
        AppLogArg::Tail => LogMode::Tail,
        AppLogArg::Quiet => LogMode::Quiet,
        AppLogArg::Auto if interactive => LogMode::Tail,
        AppLogArg::Auto => LogMode::Live,
    }
}

/// Resolve `--log-tail` over `VIBE_APP_LOG_TAIL`, then the default of five.
pub(crate) fn tail_size(flag: Option<usize>, environment: Option<&str>) -> usize {
    flag.or_else(|| environment?.trim().parse().ok())
        .unwrap_or(DEFAULT_TAIL)
}

/// What one run shows of its children, and what it remembers of their logs.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#display")]
pub(crate) struct StepDisplay {
    mode: LogMode,
    tail: usize,
    /// Whether this context may draw a panel at all.
    panelled: bool,
    /// `--log quiet` in human output: name each step's log on stderr.
    announce: bool,
    /// The panel of the run in progress.
    session: Mutex<Option<Arc<PanelSession>>>,
    /// The output log of every step that ran a child, newest last.
    logs: Mutex<Vec<(String, PathBuf)>>,
    /// The invocation the step logs are reported to, so its finished
    /// blocks count their lines; absent for a flat display.
    ctx: Option<output::Context>,
}

impl StepDisplay {
    /// The display of a command that runs installer steps.
    pub(crate) fn open(ctx: &output::Context, steps: &AppStepArgs) -> Self {
        let interactive = ctx.has_interactive_progress();
        // The environment fallbacks arrive as data: `output` reads the
        // invocation's `VIBE_*` posture variables, this cell reads none.
        let mut mode = log_mode(steps.log, output::env_app_log().as_deref(), interactive);
        // `--plain` writes blocks alone (PROP-067 `#plain`): unless a mode
        // was named, the child's output stays in its log, whose path the
        // finished block and the trailer print.
        let named = steps.log.is_some() || output::env_app_log().is_some();
        let plain = ctx.is_plain() && !named;
        if plain {
            mode = LogMode::Quiet;
        }
        Self {
            mode,
            tail: tail_size(steps.log_tail, output::env_app_log_tail().as_deref()),
            panelled: interactive,
            announce: mode == LogMode::Quiet && !plain && !ctx.is_json() && !ctx.is_quiet(),
            session: Mutex::new(None),
            logs: Mutex::new(Vec::new()),
            ctx: Some(ctx.clone()),
        }
    }

    /// The display of a command that runs no panel: removals, modifications
    /// and the bulk verbs keep their flat behaviour.
    pub(crate) fn flat() -> Self {
        Self {
            mode: LogMode::Live,
            tail: DEFAULT_TAIL,
            panelled: false,
            announce: false,
            session: Mutex::new(None),
            logs: Mutex::new(Vec::new()),
            ctx: None,
        }
    }

    /// The panel an operation releases around an attached child; absent
    /// while this run stays flat.
    pub(crate) fn panel(&self) -> Option<&dyn vibe_app::StepPanel> {
        self.panelled.then_some(self as &dyn vibe_app::StepPanel)
    }

    /// `--log quiet` keeps child output out of both streams; the run log
    /// still gets every byte.
    pub(crate) fn quiet_children(&self) -> bool {
        self.mode == LogMode::Quiet
    }

    /// Run the operation: under the panel when this context draws one,
    /// otherwise with progress held back exactly as before.
    pub(crate) fn around<R>(&self, ctx: &output::Context, run: impl FnOnce() -> R) -> R {
        let panel = self
            .panelled
            .then(|| ctx.open_progress_panel(self.tail, self.mode == LogMode::Live))
            .flatten();
        let Some(panel) = panel else {
            // A plain stream writes each block once, when it ends (PROP-067
            // `#plain`), so it never interleaves with a live child; only
            // cursor-managed bars must step aside for one.
            if !ctx.has_interactive_progress() {
                return run();
            }
            let _held = ctx.hold_progress();
            return run();
        };
        let _open = Open::hold(self, PanelSession::open(panel, ctx.reads_terminal_keys()));
        run()
    }

    /// A failed run: the failed block already kept its log's tail as its
    /// evidence (PROP-067 `#collapse`); without a renderer the tail is
    /// printed here, before the refusal names the step. Either way the
    /// failure trailer waits for the refusal. `--json` keeps its one
    /// envelope.
    pub(crate) fn report<T>(
        &self,
        ctx: &output::Context,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        if ctx.is_json() || ctx.is_quiet() {
            return result;
        }
        if let Err(error) = &result
            && let Some(step) = failing_step(error)
        {
            let log = self.log_of(step);
            if let Some(log) = &log
                && !ctx.draws_progress()
            {
                print_tail(log);
            }
            ctx.defer_trailer(ctx.trailer_text(false, &format!("failed at {step}")));
        }
        result
    }

    fn log_of(&self, step: &str) -> Option<PathBuf> {
        self.logs.lock().ok().and_then(|logs| {
            logs.iter()
                .find(|(id, _)| id == step)
                .map(|(_, path)| path.clone())
        })
    }

    fn session(&self) -> Option<Arc<PanelSession>> {
        self.session.lock().ok().and_then(|held| held.clone())
    }
}

/// Remember where a step writes its output, and in `--log quiet` say so
/// instead of showing it.
impl vibe_app::StepLogs for StepDisplay {
    fn opening(&self, step: &str, log: &Path) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.retain(|(id, _)| id != step);
            logs.push((step.to_owned(), log.to_owned()));
        }
        if let Some(ctx) = &self.ctx {
            ctx.note_step_log(step, log);
        }
        if self.announce {
            let line = output::language::Block::new(output::language::Glyph::Running, step)
                .with_meta(vec![output::language::Meta::note(format!(
                    "log {}",
                    log.display()
                ))])
                .render_at(usize::MAX);
            asking(|| eprintln!("{line}"));
        }
    }
}

impl vibe_app::StepPanel for StepDisplay {
    fn release(&self) {
        if let Some(session) = self.session() {
            session.release();
        }
    }

    fn reclaim(&self) {
        if let Some(session) = self.session() {
            session.reclaim();
        }
    }
}

/// Holds the open panel for the run: the display and the current thread
/// both find it, and both lose it on every exit path including an unwind.
struct Open<'a> {
    display: &'a StepDisplay,
}

impl<'a> Open<'a> {
    fn hold(display: &'a StepDisplay, session: Arc<PanelSession>) -> Self {
        if let Ok(mut held) = display.session.lock() {
            *held = Some(Arc::clone(&session));
        }
        PANEL.with_borrow_mut(|panel| *panel = Some(session));
        Self { display }
    }
}

impl Drop for Open<'_> {
    fn drop(&mut self) {
        PANEL.with_borrow_mut(|panel| *panel = None);
        let session = self
            .display
            .session
            .lock()
            .ok()
            .and_then(|mut held| held.take());
        if let Some(session) = session {
            session.close();
        }
    }
}

thread_local! {
    /// The panel of the run on this thread, released around every question
    /// and notice that uses the terminal itself.
    static PANEL: RefCell<Option<Arc<PanelSession>>> = const { RefCell::new(None) };
}

/// Ask a question, or print a notice, with the panel released: no drawn
/// bars and no raw mode while the terminal belongs to someone else. A run
/// without a panel keeps its own stream and this is a plain call.
pub(crate) fn asking<R>(ask: impl FnOnce() -> R) -> R {
    let Some(session) = PANEL.with_borrow(Option::clone) else {
        return ask();
    };
    let _restore = Restore::hold(session);
    ask()
}

/// The panel comes back when the question returns, fails or unwinds.
struct Restore(Arc<PanelSession>);

impl Restore {
    fn hold(session: Arc<PanelSession>) -> Self {
        session.release();
        Self(session)
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        self.0.reclaim();
    }
}

/// The terminal side of one live panel: the renderer's fold, the terminal
/// mode and the key reader, each given back on every exit path.
pub(crate) struct PanelSession {
    panel: output::ProgressPanel,
    keys: Mutex<Option<keys::Reader>>,
    /// Whether keys may be read at all: standard input must be a terminal.
    reads_keys: bool,
    /// How deep the release nests, so the panel comes back exactly once.
    released: Mutex<usize>,
}

impl PanelSession {
    fn open(panel: output::ProgressPanel, reads_keys: bool) -> Arc<Self> {
        let session = Arc::new(Self {
            panel,
            keys: Mutex::new(None),
            reads_keys,
            released: Mutex::new(0),
        });
        session.start_keys();
        session
    }

    fn start_keys(&self) {
        if !self.reads_keys {
            return;
        }
        if let Ok(mut keys) = self.keys.lock()
            && keys.is_none()
        {
            *keys = keys::Reader::start(self.panel.clone());
        }
    }

    fn stop_keys(&self) {
        if let Ok(mut keys) = self.keys.lock()
            && let Some(reader) = keys.take()
        {
            reader.stop();
        }
    }

    /// Give the terminal away: stop reading keys, restore its mode and
    /// stop drawing.
    fn release(&self) {
        let first = match self.released.lock() {
            Ok(mut depth) => {
                *depth = depth.saturating_add(1);
                *depth == 1
            }
            Err(_) => false,
        };
        if first {
            self.stop_keys();
            self.panel.hide();
        }
    }

    /// Take the terminal back.
    fn reclaim(&self) {
        let last = match self.released.lock() {
            Ok(mut depth) => {
                *depth = depth.saturating_sub(1);
                *depth == 0
            }
            Err(_) => false,
        };
        if last {
            self.panel.show();
            self.start_keys();
        }
    }

    fn close(&self) {
        self.stop_keys();
        self.panel.close();
    }
}

/// The step a failure or an interruption blames, when it names one.
fn failing_step(error: &AppError) -> Option<&str> {
    match error {
        AppError::Step(StepError::Failed { step, .. } | StepError::FinishFailed { step, .. }) => {
            Some(step)
        }
        AppError::Step(StepError::Interrupted { step, .. }) => step.as_deref(),
        // A hook's step id is the hook's own name.
        AppError::Hook { hook, .. } => Some(hook),
        _ => None,
    }
}

fn print_tail(log: &Path) {
    let Ok(text) = std::fs::read_to_string(log) else {
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return;
    }
    let shown = lines.len().min(FAILURE_TAIL);
    let body = lines[lines.len() - shown..]
        .iter()
        .map(|line| output::language::Line::Evidence(output::sanitize_progress_text(line)))
        .collect();
    let block = output::language::Block::new(
        output::language::Glyph::Failed,
        format!("last {FAILURE_TAIL} lines of {}", log.display()),
    )
    .with_body(body);
    eprintln!("{}", block.render());
}

/// The panel's hotkey, read from the terminal in raw mode on its own
/// thread (PROP-065 `#display`).
///
/// Raw mode takes the console's own Ctrl-C handling away, so the reader
/// puts it back by hand on both families, and `Ctrl-C` under the panel
/// ends the run exactly as it does without one (`PROP-065#cancellation`).
/// On Unix raw mode clears `ISIG`, so the terminal stops generating
/// `SIGINT`: the reader raises `SIGINT` on this process, which is the very
/// signal `super::interrupt::Signals` registered for. On Windows raw mode
/// clears `ENABLE_PROCESSED_INPUT`, so the console stops generating the
/// control event: the reader generates `CTRL_C_EVENT` for the console's
/// own process group, which is what reaches the attached child and the
/// `SetConsoleCtrlHandler` routine of `Signals` alike. Either way the
/// flag is set, the next poll forwards it to the child and the step is
/// recorded `interrupted`.
///
/// The handler is always in place first: `global::Observation::new`
/// installs `Signals` before `StepDisplay::around` opens the panel, and
/// the reader starts only inside the panel.
#[cfg(any(unix, windows))]
mod keys {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::JoinHandle;
    use std::time::Duration;

    use ratatui_crossterm::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use ratatui_crossterm::crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    use crate::output::ProgressPanel;

    /// How long one poll waits, and so how long `stop` may take.
    const POLL: Duration = Duration::from_millis(100);

    pub(super) struct Reader {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Reader {
        /// Start reading keys; `None` when the terminal mode cannot change.
        pub(super) fn start(panel: ProgressPanel) -> Option<Self> {
            enable_raw_mode().ok()?;
            let stop = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&stop);
            let thread = std::thread::Builder::new()
                .name("vibe-app-panel".to_owned())
                .spawn(move || read(&panel, &flag))
                .ok();
            if thread.is_none() {
                let _ = disable_raw_mode();
                return None;
            }
            Some(Self { stop, thread })
        }

        /// Stop reading and restore the terminal mode, even if the thread
        /// itself ended badly.
        pub(super) fn stop(mut self) {
            self.shut_down();
        }

        fn shut_down(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            let _ = disable_raw_mode();
        }
    }

    impl Drop for Reader {
        fn drop(&mut self) {
            self.shut_down();
        }
    }

    fn read(panel: &ProgressPanel, stop: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            match event::poll(POLL) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(_) => return,
            }
            let Ok(Event::Key(key)) = event::read() else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => panel.toggle(),
                KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    interrupt();
                }
                // Unknown keys are ignored: the panel is not a TUI.
                _ => {}
            }
        }
    }

    /// Deliver the interrupt raw mode swallowed.
    #[cfg(unix)]
    fn interrupt() {
        let _ = signal_hook::low_level::raise(signal_hook::consts::SIGINT);
    }

    #[cfg(windows)]
    use console_event::interrupt;

    /// `GenerateConsoleCtrlEvent` on the console this process shares with
    /// its children, declared here like the `SetConsoleCtrlHandler` of
    /// `super::interrupt` rather than through a new dependency.
    #[cfg(windows)]
    mod console_event {
        #![allow(unsafe_code)]

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GenerateConsoleCtrlEvent(ctrl_event: u32, process_group_id: u32) -> i32;
        }

        const CTRL_C_EVENT: u32 = 0;
        /// Every process attached to this console, children included.
        const THIS_CONSOLE: u32 = 0;

        #[specmark::spec(
            deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
            reason = "unsafe-gate: raw mode clears ENABLE_PROCESSED_INPUT, and Rust has no standard-library API for generating a console control event; both arguments are kernel32 constants (spec://org.vibevm.core/vibevm/common/PROP-065#display)."
        )]
        pub(super) fn interrupt() {
            // SAFETY: both arguments are constants of the documented
            // kernel32 call, which borrows nothing.
            let _ = unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, THIS_CONSOLE) };
        }
    }
}

/// Without a terminal whose mode this process may change, the panel draws
/// its fold and takes no keys.
#[cfg(not(any(unix, windows)))]
mod keys {
    use crate::output::ProgressPanel;

    pub(super) struct Reader;

    impl Reader {
        pub(super) fn start(_panel: ProgressPanel) -> Option<Self> {
            None
        }

        pub(super) fn stop(self) {}
    }
}

#[cfg(test)]
#[path = "display/tests.rs"]
mod tests;
