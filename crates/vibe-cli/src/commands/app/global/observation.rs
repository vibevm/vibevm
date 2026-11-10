//! What one application operation observes, and the stderr notice it
//! prints (PROP-065 `#events`, `#work`, `#display`).
//!
//! Split out of `global.rs`: that file's subject is the commands —
//! install, update, uninstall and the request each builds — and this
//! one's is the per-run observation they all share. It is also where the
//! run's ambient facts are gathered, the user's own `config.toml` among
//! them, because the composition boundary is here and not in `vibe-app`.

use anyhow::{Context, Result};
use vibe_app::{Interrupt, UserEnvironment};
use vibe_core::progress::Progress;

use super::super::display::{StepDisplay, asking};
use super::super::interrupt::Signals;
use super::{settings_root, source_operation};
use crate::output;

/// A run's one-line notice printer (PROP-065 `#work`).
type Notice = Box<dyn Fn(&str)>;

/// What one operation observes: the progress handle (the machine stream
/// only; the human renderer keeps showing child output as before) and the
/// caught signals of a mutating command.
pub(crate) struct Observation<'d> {
    pub(crate) http_client: vibe_package_source::OperationHttpClient,
    pub(crate) progress: Progress,
    pub(crate) signals: Option<Signals>,
    /// The durable user environment env steps change (PROP-065 `#env-step`).
    pub(crate) environment: Option<Box<dyn UserEnvironment>>,
    /// The caller's PATH, behind the requirements a step sees first.
    pub(crate) search_path: Option<std::ffi::OsString>,
    /// The user's own `config.toml`, resolved once here because this is
    /// the layer that owns the process boundary: `vibe-app` reads and
    /// interprets the file but never resolves `$VIBE_SETTINGS` or
    /// `VIBEVM_USER_CONFIG` itself (PROP-075 `#SLOT-EXTRAS`).
    pub(crate) user_config: Option<std::path::PathBuf>,
    /// One-line run notices on stderr, so `--json` stays one envelope
    /// (PROP-065 `#work`); absent when the caller asked for quiet.
    notice: Option<Notice>,
    /// What this run shows of its children (PROP-065 `#display`); the flat
    /// display of a command that runs no panel.
    pub(super) display: &'d StepDisplay,
}

impl<'d> Observation<'d> {
    /// `mutating` operations catch Ctrl-C / SIGTERM (PROP-065 `#cancellation`).
    pub(crate) fn new(
        ctx: &output::Context,
        mutating: bool,
        display: &'d StepDisplay,
    ) -> Result<Self> {
        Ok(Self {
            http_client: source_operation(Default::default())?
                .http_client(std::time::Duration::from_secs(600)),
            // Every human renderer draws the steps as blocks (PROP-067
            // `#collapse`, `#plain`); a run with none observes nothing but
            // the machine stream.
            progress: if ctx.is_machine_progress() || ctx.draws_progress() {
                ctx.progress()
            } else {
                // Unrendered, still journaled (`PROP-068#journal`).
                Progress::default()
                    .with_transfers(ctx.transfers())
                    .with_cancellation(ctx.progress().cancellation().clone())
            },
            signals: if mutating {
                Some(Signals::install().context("catching interrupt signals")?)
            } else {
                None
            },
            environment: super::super::environment::for_host(
                settings_root()?.path(),
                super::super::facts::user_home().as_deref(),
            ),
            search_path: super::super::facts::search_path(),
            user_config: vibe_core::user_config::UserConfig::default_path(),
            notice: (!ctx.is_quiet())
                .then(|| Box::new(|text: &str| asking(|| notice(text))) as Notice),
            display,
        })
    }

    pub(crate) fn with_source_operation(
        mut self,
        operation: &vibe_package_source::SourceOperation,
    ) -> Self {
        self.http_client = operation.http_client(std::time::Duration::from_secs(600));
        self
    }

    pub(super) fn notice(&self) -> Option<&dyn Fn(&str)> {
        self.notice.as_deref()
    }

    pub(super) fn interrupt(&self) -> Option<&dyn Interrupt> {
        self.signals
            .as_ref()
            .map(|signals| signals as &dyn Interrupt)
    }
}

/// A run notice: a dim line between blocks.
fn notice(text: &str) {
    use output::language::{Kind, Role, Stream, before, paint};
    eprintln!(
        "{}{}",
        before(Stream::Err, Kind::Block),
        paint(Role::Dim, text)
    );
}
