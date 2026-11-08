//! The stop owner of one project operation: platform stop signals in, the
//! operation's cancellation token out, with the attended question between.
//!
//! One watcher thread reads the signal flags and the question's answer and
//! carries out [`super::question::step`]. While the question is on the
//! terminal the operation's checkpoints are held (the token's poll waits),
//! so no new phase starts; in-flight children keep running. Supervised
//! children — lifecycle executions such as package hooks, Git transfers,
//! builds — are spawned by `command-group` as leaders of process groups of
//! their own, so the terminal's Ctrl-C reaches vibe and not them, and a
//! confirmed stop reaps their whole group at the waiting loop's next
//! checkpoint. A legacy hook runner that still spawns into vibe's own group
//! receives the same Ctrl-C; a hook that dies of it fails its step, and the
//! operation rolls back as a failure instead of reporting a continuation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP");

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use vibe_core::cancellation::CancellationToken;

use super::question::{self, Action, Answer, Event, Phase, Posture};
use super::signals::Source;

/// How long the watcher waits for an answer before it reads the signal
/// flags again.
const POLL: Duration = Duration::from_millis(25);

/// The phase every checkpoint consults, and the question being asked.
pub(super) struct Shared {
    phase: Mutex<Phase>,
    changed: Condvar,
    question: Mutex<Option<JoinHandle<()>>>,
}

impl Shared {
    pub(super) fn new() -> Self {
        Self {
            phase: Mutex::new(Phase::Running),
            changed: Condvar::new(),
            question: Mutex::new(None),
        }
    }

    fn phase(&self) -> MutexGuard<'_, Phase> {
        self.phase.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wait while the question is on the terminal: a checkpoint reached
    /// meanwhile starts nothing new until the person answered.
    pub(super) fn hold(&self) {
        let mut phase = self.phase();
        while *phase == Phase::Asking {
            phase = self
                .changed
                .wait(phase)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn stopping(&self) -> bool {
        *self.phase() == Phase::Stopping
    }
}

/// The running stop owner; settling it ends the question and the watcher
/// and gives the signals their default action back.
pub(super) struct Watch {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Dropped last: until the watcher ended, a signal is still caught.
    source: Option<Source>,
}

impl Watch {
    /// Start watching; `None` when the platform cannot catch the signals,
    /// which leaves Ctrl-C its default action.
    pub(super) fn start(
        posture: Posture,
        shared: Arc<Shared>,
        token: CancellationToken,
        ctx: crate::output::Context,
    ) -> Option<Self> {
        let source = Source::install().ok()?;
        let stop = Arc::new(AtomicBool::new(false));
        let owner = Owner {
            posture,
            shared: Arc::clone(&shared),
            token,
            ctx,
            answers: channel(),
        };
        let flags = source.flags();
        let ended = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("vibe-stop".to_owned())
            .spawn(move || owner.run(&flags, &ended))
            .ok()?;
        Some(Self {
            shared,
            stop,
            thread: Some(thread),
            source: Some(source),
        })
    }

    /// Whether the person asked to stop and the operation never reached a
    /// checkpoint that honoured it.
    pub(super) fn settle(mut self, token: &CancellationToken) -> bool {
        self.shut_down();
        self.shared.stopping() && !token.acknowledged()
    }

    fn shut_down(&mut self) {
        // An open question is answered before the run reports: the
        // terminal comes back out of raw mode, and the answer is honoured.
        let question = self
            .shared
            .question
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(question) = question {
            let _ = question.join();
        }
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.source.take();
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// The watcher thread's state.
struct Owner {
    posture: Posture,
    shared: Arc<Shared>,
    token: CancellationToken,
    ctx: crate::output::Context,
    answers: (Sender<Answer>, Receiver<Answer>),
}

impl Owner {
    fn run(self, flags: &super::signals::Flags, ended: &AtomicBool) {
        loop {
            for event in flags.take() {
                self.apply(event);
            }
            match self.answers.1.recv_timeout(POLL) {
                Ok(answer) => self.apply(Event::Answered(answer)),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
            }
            if ended.load(Ordering::SeqCst) {
                // The question was joined before `ended` was set, so its
                // answer, if any, is already in the channel.
                while let Ok(answer) = self.answers.1.try_recv() {
                    self.apply(Event::Answered(answer));
                }
                for event in flags.take() {
                    self.apply(event);
                }
                return;
            }
        }
    }

    fn apply(&self, event: Event) {
        let mut phase = self.shared.phase();
        let (next, action) = question::step(self.posture, *phase, event);
        match action {
            // Requested before the held checkpoints wake, so each of them
            // sees the stop.
            Action::Stop => self.token.cancel(),
            Action::Ask => {
                if !self.ask() {
                    // Nobody can be asked after all: honour the Ctrl-C.
                    self.token.cancel();
                    *phase = Phase::Stopping;
                    drop(phase);
                    self.shared.changed.notify_all();
                    return;
                }
            }
            Action::Resume | Action::Nothing => {}
        }
        *phase = next;
        drop(phase);
        self.shared.changed.notify_all();
    }

    /// Show the question on its own thread, with the progress display
    /// paused; whether it could be started.
    fn ask(&self) -> bool {
        let ctx = self.ctx.clone();
        let answers = self.answers.0.clone();
        let asked = std::thread::Builder::new()
            .name("vibe-stop-question".to_owned())
            .spawn(move || {
                let result = ctx.suspend_progress(|| {
                    crate::output::prompts::confirm(question::QUESTION, false)
                });
                let _ = answers.send(question::answer(&result));
            });
        match asked {
            Ok(handle) => {
                *self
                    .shared
                    .question
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(handle);
                true
            }
            Err(_) => false,
        }
    }
}
