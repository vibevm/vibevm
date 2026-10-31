//! Signals during an application install, update or uninstall (PROP-065
//! `#cancellation`): while the operation runs, Ctrl-C, `SIGTERM` and a
//! console control event are caught instead of killing vibe; the running
//! child receives the signal, vibe waits for it and records the step
//! `interrupted`. Mirrors the PATH proxy's supervision.

#![allow(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#cancellation");

use std::sync::atomic::{AtomicBool, Ordering};

/// Caught for the lifetime of the guard; dropping it restores the default
/// termination for later signals.
pub(crate) struct Signals {
    seen: AtomicBool,
    #[cfg(unix)]
    unix: unix::Flags,
    #[cfg(windows)]
    _console: windows::Console,
}

impl Signals {
    pub(crate) fn install() -> std::io::Result<Self> {
        Ok(Self {
            seen: AtomicBool::new(false),
            #[cfg(unix)]
            unix: unix::Flags::install()?,
            #[cfg(windows)]
            _console: windows::Console::install()?,
        })
    }
}

impl vibe_app::Interrupt for Signals {
    fn poll(&self, child: Option<u32>) -> bool {
        #[cfg(unix)]
        if self.unix.forward(child) {
            self.seen.store(true, Ordering::SeqCst);
        }
        #[cfg(windows)]
        {
            // The console delivers the event to the child itself.
            let _ = child;
            if windows::take() {
                self.seen.store(true, Ordering::SeqCst);
            }
        }
        #[cfg(not(any(unix, windows)))]
        let _ = child;
        self.seen.load(Ordering::SeqCst)
    }
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use signal_hook::consts::{SIGINT, SIGTERM};

    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }

    pub(super) struct Flags {
        interrupt: Arc<AtomicBool>,
        terminate: Arc<AtomicBool>,
        /// Set on drop: a later signal takes its default action again.
        done: Arc<AtomicBool>,
    }

    impl Flags {
        pub(super) fn install() -> io::Result<Self> {
            let flags = Self {
                interrupt: Arc::new(AtomicBool::new(false)),
                terminate: Arc::new(AtomicBool::new(false)),
                done: Arc::new(AtomicBool::new(false)),
            };
            for (signal, flag) in [(SIGINT, &flags.interrupt), (SIGTERM, &flags.terminate)] {
                signal_hook::flag::register_conditional_default(signal, Arc::clone(&flags.done))?;
                signal_hook::flag::register(signal, Arc::clone(flag))?;
            }
            Ok(flags)
        }

        /// Forward the signals received since the last call to `child`;
        /// whether any arrived.
        pub(super) fn forward(&self, child: Option<u32>) -> bool {
            let mut arrived = false;
            for (signal, flag) in [(SIGINT, &self.interrupt), (SIGTERM, &self.terminate)] {
                if flag.swap(false, Ordering::SeqCst) {
                    arrived = true;
                    if let Some(pid) = child {
                        let _ = signal_child(pid, signal);
                    }
                }
            }
            arrived
        }
    }

    impl Drop for Flags {
        fn drop(&mut self) {
            self.done.store(true, Ordering::SeqCst);
        }
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for sending a signal to a child; the process id comes from Child::id and the signal is a signal-hook platform constant (spec://org.vibevm.core/vibevm/common/PROP-065#cancellation)."
    )]
    fn signal_child(pid: u32, signal: i32) -> io::Result<()> {
        let pid = i32::try_from(pid).map_err(io::Error::other)?;
        // SAFETY: `pid` is the live child returned by `Child::id`, and
        // `signal` is a signal-hook platform constant.
        if unsafe { kill(pid, signal) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    type HandlerRoutine = Option<unsafe extern "system" fn(u32) -> i32>;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler_routine: HandlerRoutine, add: i32) -> i32;
    }

    static RECEIVED: AtomicBool = AtomicBool::new(false);

    /// Whether a console control event arrived since the last call.
    pub(super) fn take() -> bool {
        RECEIVED.swap(false, Ordering::SeqCst)
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: SetConsoleCtrlHandler requires a system-ABI callback; this one only records Ctrl-C / Ctrl-Break in a static flag (spec://org.vibevm.core/vibevm/common/PROP-065#cancellation)."
    )]
    unsafe extern "system" fn console_control(event: u32) -> i32 {
        if matches!(event, 0 | 1) {
            RECEIVED.store(true, Ordering::SeqCst);
            1
        } else {
            0
        }
    }

    pub(super) struct Console;

    impl Console {
        #[specmark::spec(
            deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
            reason = "unsafe-gate: registers the static callback above; no borrowed state outlives it (spec://org.vibevm.core/vibevm/common/PROP-065#cancellation)."
        )]
        pub(super) fn install() -> io::Result<Self> {
            // SAFETY: the callback is static, has the required ABI and is
            // removed by the guard's drop.
            if unsafe { SetConsoleCtrlHandler(Some(console_control), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self)
        }
    }

    impl Drop for Console {
        #[specmark::spec(
            deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
            reason = "unsafe-gate: removes the same static callback the guard registered (spec://org.vibevm.core/vibevm/common/PROP-065#cancellation)."
        )]
        fn drop(&mut self) {
            // SAFETY: removes the handler registered by `install`.
            let _ = unsafe { SetConsoleCtrlHandler(Some(console_control), 0) };
        }
    }
}
