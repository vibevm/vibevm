//! The host's process calls behind `vibe app running`, `stop` and `watch`
//! (PROP-059 `#launcher-runtime`): `vibe-app` decides from files, and this
//! module answers whether a process id is alive (with its start time, so a
//! reused id is told apart) and delivers the end — `SIGTERM` then `SIGKILL`
//! to the launcher's process group on Unix, a console break through the
//! launcher's own console then `TerminateProcess` on Windows — the way `interrupt.rs` forwards a signal
//! to a step (PROP-065 `#cancellation`).

#![allow(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use vibe_app::{ProcessState, ProcessTable};
use vibe_core::manifest::LauncherKind;

/// The running system's processes.
pub(crate) struct HostProcesses;

impl ProcessTable for HostProcesses {
    fn probe(&self, pid: u32) -> ProcessState {
        platform::probe(pid)
    }

    fn ask_to_end(&self, pid: u32, kind: LauncherKind) -> bool {
        platform::ask_to_end(pid, kind)
    }

    fn force_end(&self, pid: u32) -> bool {
        platform::force_end(pid)
    }
}

#[cfg(unix)]
mod platform {
    use signal_hook::consts::{SIGKILL, SIGTERM};
    use vibe_app::ProcessState;
    use vibe_core::manifest::LauncherKind;

    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }

    /// `EPERM`: the process exists and belongs to someone else.
    const NOT_PERMITTED: i32 = 1;

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for signalling a process it did not spawn in this run; the id comes from the home's running registry, whose liveness check precedes every call, and the signal is a signal-hook platform constant or 0 (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    fn signal(pid: i32, signal: i32) -> Result<(), i32> {
        // SAFETY: `kill` borrows nothing; `pid` and `signal` are plain
        // integers and a stale id fails with ESRCH rather than misbehaving.
        if unsafe { kill(pid, signal) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0))
        }
    }

    fn positive(pid: u32) -> Option<i32> {
        i32::try_from(pid).ok().filter(|pid| *pid > 0)
    }

    /// The launcher's process group first: a detached start made the child
    /// the leader of its own group, and a shell launcher's children belong
    /// to it too. The process alone when it left the group.
    fn deliver(pid: u32, which: i32) -> bool {
        let Some(pid) = positive(pid) else {
            return false;
        };
        signal(-pid, which).is_ok() || signal(pid, which).is_ok()
    }

    fn exists(pid: u32) -> bool {
        positive(pid).is_some_and(|pid| match signal(pid, 0) {
            Ok(()) => true,
            Err(errno) => errno == NOT_PERMITTED,
        })
    }

    /// On Linux `/proc/<pid>/stat` answers both questions: a zombie (`Z`) or
    /// dead (`X`) state is gone, and field 22 is the start time in clock
    /// ticks since boot — fixed for the life of the process.
    #[cfg(target_os = "linux")]
    pub(super) fn probe(pid: u32) -> ProcessState {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(stat) => {
                let Some((_, rest)) = stat.rsplit_once(')') else {
                    return ProcessState::Gone;
                };
                let fields: Vec<&str> = rest.split_whitespace().collect();
                match fields.first() {
                    Some(&("Z" | "X")) | None => ProcessState::Gone,
                    Some(_) => ProcessState::Alive {
                        started: fields.get(19).map(|ticks| (*ticks).to_owned()),
                    },
                }
            }
            Err(_) if std::path::Path::new("/proc/self").exists() => ProcessState::Gone,
            Err(_) => fallback(pid),
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn probe(pid: u32) -> ProcessState {
        fallback(pid)
    }

    /// Without `/proc` the platform gives no start time this module reads,
    /// so liveness alone answers.
    fn fallback(pid: u32) -> ProcessState {
        if exists(pid) {
            ProcessState::Alive { started: None }
        } else {
            ProcessState::Gone
        }
    }

    pub(super) fn ask_to_end(pid: u32, _kind: LauncherKind) -> bool {
        deliver(pid, SIGTERM)
    }

    pub(super) fn force_end(pid: u32) -> bool {
        deliver(pid, SIGKILL)
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    use vibe_app::ProcessState;
    use vibe_core::manifest::LauncherKind;

    type Handle = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
        fn GetProcessTimes(
            process: Handle,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn TerminateProcess(process: Handle, exit_code: u32) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn GenerateConsoleCtrlEvent(ctrl_event: u32, process_group_id: u32) -> i32;
        fn FreeConsole() -> i32;
        fn AttachConsole(process_id: u32) -> i32;
        fn SetConsoleCtrlHandler(handler: Option<CtrlHandler>, add: i32) -> i32;
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn SetStdHandle(std_handle: u32, handle: Handle) -> i32;
    }

    type CtrlHandler = unsafe extern "system" fn(u32) -> i32;

    const PROCESS_TERMINATE: u32 = 0x0001;
    const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6; // (DWORD)-10
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // (DWORD)-11
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4; // (DWORD)-12
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    /// Ctrl-C cannot be sent to a process group; Ctrl-Break can.
    const CTRL_BREAK_EVENT: u32 = 1;
    /// `(DWORD)-1`: the console of the process that started this one.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

    /// An open process handle, closed on drop.
    struct Process(Handle);

    impl Process {
        #[specmark::spec(
            deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
            reason = "unsafe-gate: Rust has no standard-library API for opening a process it did not spawn in this run; the handle is checked for null and owned by the returned guard (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
        )]
        fn open(pid: u32, access: u32) -> Option<Self> {
            // SAFETY: plain integer arguments; a null result is refused.
            let handle = unsafe { OpenProcess(access, 0, pid) };
            (!handle.is_null()).then_some(Self(handle))
        }
    }

    impl Drop for Process {
        #[specmark::spec(
            deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
            reason = "unsafe-gate: closes the handle this guard owns, exactly once (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
        )]
        fn drop(&mut self) {
            // SAFETY: the handle came from OpenProcess and is closed once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    /// A live process answers its creation time, fixed for its life.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: GetExitCodeProcess and GetProcessTimes write into locals this function owns, through a handle the guard keeps open (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    pub(super) fn probe(pid: u32) -> ProcessState {
        let Some(process) = Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION) else {
            return ProcessState::Gone;
        };
        let mut code = 0u32;
        // SAFETY: a live handle and a pointer to a local.
        if unsafe { GetExitCodeProcess(process.0, &mut code) } == 0 || code != STILL_ACTIVE {
            return ProcessState::Gone;
        }
        let mut times = [FileTime::default(); 4];
        let [creation, exit, kernel, user] = &mut times;
        // SAFETY: a live handle and four pointers to locals.
        let read = unsafe { GetProcessTimes(process.0, creation, exit, kernel, user) } != 0;
        let started =
            read.then(|| ((u64::from(times[0].high) << 32) | u64::from(times[0].low)).to_string());
        ProcessState::Alive { started }
    }

    /// A terminal launcher kept a console and its own process group, so a
    /// console break asks it to end; a window of its own has nothing to
    /// break.
    pub(super) fn ask_to_end(pid: u32, kind: LauncherKind) -> bool {
        match kind {
            LauncherKind::Gui => false,
            LauncherKind::Tui | LauncherKind::Command => break_through_console(pid),
        }
    }

    /// While the break is generated this process is attached to the
    /// launcher's console; the event targets the launcher's group only, and
    /// the handler is a guard should the console deliver it here anyway.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: SetConsoleCtrlHandler requires a system-ABI callback; this one touches no state and only reports the event handled (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    unsafe extern "system" fn swallow(_ctrl_type: u32) -> i32 {
        1
    }

    /// `GenerateConsoleCtrlEvent` reaches only processes sharing the
    /// caller's console, and `vibe app stop` runs from another one, so the
    /// plain call failed every time and the stop went straight to
    /// `TerminateProcess`. Attaching to the launcher's console first makes
    /// the break land. When the launcher has no console to attach to (it
    /// freed it, or never had one), the answer is `false` and the caller's
    /// terminate branch runs as before. No unit test can stand in for a
    /// second console; PROP-059 `#launcher-runtime` names the manual check.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for console attachment or control events; every argument is an integer or a handler function with the declared signature and static lifetime, and the console and handler are restored before returning (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    fn break_through_console(pid: u32) -> bool {
        // `AttachConsole` points the standard handles at the console it
        // attaches, so a redirected stdout or stderr (the shell's pipe, a
        // file) would be lost with the trailer: keep the three and put them
        // back once this process is on its own console again.
        // SAFETY: integer arguments; a handle value is only stored here.
        let kept = unsafe {
            [
                (STD_INPUT_HANDLE, GetStdHandle(STD_INPUT_HANDLE)),
                (STD_OUTPUT_HANDLE, GetStdHandle(STD_OUTPUT_HANDLE)),
                (STD_ERROR_HANDLE, GetStdHandle(STD_ERROR_HANDLE)),
            ]
        };
        let sent = break_attached(pid);
        for (which, handle) in kept {
            // SAFETY: restores a handle this process already owned.
            let _ = unsafe { SetStdHandle(which, handle) };
        }
        sent
    }

    /// The attach, the break and the way back; the caller restores the
    /// standard handles around it.
    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API for console attachment or control events; every argument is an integer or a handler function with the declared signature and static lifetime, and the console and handler are restored before returning (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    fn break_attached(pid: u32) -> bool {
        // SAFETY: borrows nothing; failing without a console is harmless.
        let _ = unsafe { FreeConsole() };
        // SAFETY: an integer argument; failure is answered below.
        if unsafe { AttachConsole(pid) } == 0 {
            // SAFETY: an integer argument; without a parent console it fails
            // and this process stays detached, as a detached start leaves it.
            let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
            return false;
        }
        // SAFETY: `swallow` is a static function of the handler signature.
        let guarded = unsafe { SetConsoleCtrlHandler(Some(swallow), 1) } != 0;
        // SAFETY: integer arguments; the launcher is its own process group,
        // so its id is the group id.
        let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) } != 0;
        // SAFETY: borrows nothing; detaches from the launcher's console.
        let _ = unsafe { FreeConsole() };
        // SAFETY: an integer argument; restores the console this process had.
        let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
        if guarded {
            // SAFETY: removes the handler added above.
            let _ = unsafe { SetConsoleCtrlHandler(Some(swallow), 0) };
        }
        sent
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: TerminateProcess through a handle the guard keeps open; Rust can only kill a Child it spawned in this run (spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime)."
    )]
    pub(super) fn force_end(pid: u32) -> bool {
        Process::open(pid, PROCESS_TERMINATE)
            // SAFETY: a live handle opened with PROCESS_TERMINATE.
            .is_some_and(|process| unsafe { TerminateProcess(process.0, 1) } != 0)
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use vibe_app::ProcessState;
    use vibe_core::manifest::LauncherKind;

    pub(super) fn probe(_pid: u32) -> ProcessState {
        ProcessState::Gone
    }

    pub(super) fn ask_to_end(_pid: u32, _kind: LauncherKind) -> bool {
        false
    }

    pub(super) fn force_end(_pid: u32) -> bool {
        false
    }
}
