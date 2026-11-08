//! The platform's stop signals for one project operation, caught as flags.
//!
//! Unix: `SIGINT` (Ctrl-C) and `SIGTERM` set a flag instead of ending the
//! process; once the guard drops, a later signal takes its default action
//! again. Windows: the console's Ctrl-C and Ctrl-Break are recorded by the
//! console control handler the application installer already owns.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP");

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::question::Event;

/// The flags the watcher reads; cheap to hand to its thread.
#[derive(Clone, Default)]
pub(super) struct Flags {
    interrupt: Arc<AtomicBool>,
    terminate: Arc<AtomicBool>,
}

impl Flags {
    /// The stop inputs that arrived since the last call, a terminate first.
    pub(super) fn take(&self) -> Vec<Event> {
        #[cfg(windows)]
        {
            let (interrupt, brk) = crate::commands::app::interrupt::windows::take_split();
            if interrupt {
                self.interrupt.store(true, Ordering::SeqCst);
            }
            if brk {
                self.terminate.store(true, Ordering::SeqCst);
            }
        }
        let mut events = Vec::new();
        if self.terminate.swap(false, Ordering::SeqCst) {
            events.push(Event::Terminate);
        }
        if self.interrupt.swap(false, Ordering::SeqCst) {
            events.push(Event::Interrupt);
        }
        events
    }
}

/// Signals are caught for the lifetime of this guard.
pub(super) struct Source {
    flags: Flags,
    #[cfg(unix)]
    done: Arc<AtomicBool>,
    #[cfg(windows)]
    _console: crate::commands::app::interrupt::windows::Console,
}

impl Source {
    pub(super) fn install() -> std::io::Result<Self> {
        let flags = Flags::default();
        #[cfg(unix)]
        {
            use signal_hook::consts::{SIGINT, SIGTERM};
            let done = Arc::new(AtomicBool::new(false));
            for (signal, flag) in [(SIGINT, &flags.interrupt), (SIGTERM, &flags.terminate)] {
                signal_hook::flag::register_conditional_default(signal, Arc::clone(&done))?;
                signal_hook::flag::register(signal, Arc::clone(flag))?;
            }
            Ok(Self { flags, done })
        }
        #[cfg(windows)]
        {
            Ok(Self {
                flags,
                _console: crate::commands::app::interrupt::windows::Console::install()?,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "no stop signals on this platform",
            ))
        }
    }

    pub(super) fn flags(&self) -> Flags {
        self.flags.clone()
    }
}

#[cfg(unix)]
impl Drop for Source {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
    }
}
