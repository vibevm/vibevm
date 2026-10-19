use std::io;
use std::process::{Child, ExitStatus};

#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

#[cfg(windows)]
type HandlerRoutine = Option<unsafe extern "system" fn(u32) -> i32>;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler_routine: HandlerRoutine, add: i32) -> i32;
}

#[cfg(windows)]
const CTRL_C_EVENT: u32 = 0;
#[cfg(windows)]
const CTRL_BREAK_EVENT: u32 = 1;

#[cfg(windows)]
fn handles_console_control(control: u32) -> bool {
    matches!(control, CTRL_C_EVENT | CTRL_BREAK_EVENT)
}

#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "Windows requires an unsafe system-ABI callback for SetConsoleCtrlHandler; this stateless callback only classifies the control value and returns BOOL."
)]
unsafe extern "system" fn console_control_handler(control: u32) -> i32 {
    if handles_console_control(control) {
        1
    } else {
        0
    }
}

#[cfg(windows)]
struct ConsoleControlHandler;

#[cfg(windows)]
impl ConsoleControlHandler {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "SetConsoleCtrlHandler has no standard-library wrapper; the registered callback is static, has the required ABI, and carries no borrowed state."
    )]
    fn install() -> io::Result<Self> {
        // SAFETY: `console_control_handler` has the required static lifetime and
        // ABI, and remains valid until process exit. The guard unregisters the
        // exact same function when supervision finishes.
        if unsafe { SetConsoleCtrlHandler(Some(console_control_handler), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for ConsoleControlHandler {
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "SetConsoleCtrlHandler has no standard-library wrapper; drop removes the same static callback registered by the guard and ignores only cleanup failure during process exit."
    )]
    fn drop(&mut self) {
        // SAFETY: this removes the handler registered by `install`; there is no
        // callback state to outlive the registration.
        let _ = unsafe { SetConsoleCtrlHandler(Some(console_control_handler), 0) };
    }
}

pub(crate) struct Supervisor {
    #[cfg(unix)]
    signals: signal_hook::iterator::Signals,
    #[cfg(windows)]
    _console_control_handler: ConsoleControlHandler,
}

impl Supervisor {
    #[cfg(unix)]
    pub(crate) fn install() -> io::Result<Self> {
        Ok(Self {
            signals: signal_hook::iterator::Signals::new([
                signal_hook::consts::SIGINT,
                signal_hook::consts::SIGQUIT,
                signal_hook::consts::SIGTERM,
            ])?,
        })
    }

    #[cfg(windows)]
    pub(crate) fn install() -> io::Result<Self> {
        Ok(Self {
            _console_control_handler: ConsoleControlHandler::install()?,
        })
    }

    #[cfg(not(any(unix, windows)))]
    pub(crate) fn install() -> io::Result<Self> {
        Ok(Self {})
    }

    pub(crate) fn wait(&mut self, mut child: Child) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
                for signal in self.signals.pending() {
                    if signal == signal_hook::consts::SIGTERM {
                        let _ = forward_unix_signal(child.id(), signal);
                    }
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        #[cfg(not(unix))]
        child.wait()
    }
}

#[cfg(unix)]
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "Rust has no standard-library API for sending SIGTERM to a child; the process id comes from Child::id and the signal is a signal-hook platform constant."
)]
fn forward_unix_signal(child_id: u32, signal: i32) -> io::Result<()> {
    // SAFETY: `child_id` is the live OS process identifier returned by
    // `Child::id`, and `signal` comes from signal-hook's platform constants.
    if unsafe { kill(child_id as i32, signal) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::io::{BufRead, BufReader};
    #[cfg(unix)]
    use std::process::{Command, Stdio};

    #[cfg(windows)]
    #[test]
    fn console_handler_consumes_only_interrupt_events() {
        assert!(handles_console_control(CTRL_C_EVENT));
        assert!(handles_console_control(CTRL_BREAK_EVENT));
        assert!(!handles_console_control(2));
    }

    #[cfg(unix)]
    #[test]
    fn sigterm_is_forwarded_without_forcing_the_child_exit_status() {
        let mut child = Command::new("/bin/sh")
            .args([
                "-c",
                "trap 'exit 42' TERM; printf 'ready\\n'; while :; do :; done",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut ready = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready, "ready\n");

        forward_unix_signal(child.id(), signal_hook::consts::SIGTERM).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(42));
    }
}
