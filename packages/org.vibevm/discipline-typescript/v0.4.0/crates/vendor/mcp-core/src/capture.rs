//! The stderr capture guard (MCP-CORE §5): everything a tool run says —
//! including CHILD PROCESSES like a floor's cargo/prettier/node — goes
//! to fd 2 / the process std-error handle, so the only capture that
//! sees a whole run is a process-level redirect around the call. A
//! threaded `&mut dyn Write` was tried on paper and rejected: children
//! inherit the PROCESS handle and would bypass it entirely (the plan's
//! Wave-0 S3 finding).
//!
//! Shape: redirect the process stderr into a fresh temp FILE (a file,
//! not a pipe — a filling pipe blocks the writer and deadlocks a chatty
//! floor), run the closure, restore, read, delete. The redirect is
//! process-global state, so captures do not nest and do not run
//! concurrently — the serve loop dispatches tools sequentially, which
//! is exactly the licence this cell needs; a second simultaneous
//! `capture` refuses. Restoration happens in `Drop`, so a panicking
//! tool cannot leave the process mute.
//!
//! Platform notes: on unix the redirect is `dup2` over fd 2; on Windows
//! it is `SetStdHandle(STD_ERROR_HANDLE, …)` — Rust's own stderr writes
//! and spawned children both resolve that handle, and the capture test
//! pins both facts on every platform this crate builds on.

specmark::scope!("spec://discipline-core/mechanisms/MCP-CORE-v0.1#capture");

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::McpCoreError;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SEQ: AtomicU64 = AtomicU64::new(0);

fn err(op: &str, detail: impl std::fmt::Display) -> McpCoreError {
    McpCoreError::Capture {
        op: op.to_string(),
        detail: detail.to_string(),
    }
}

/// Run `f` with the process's stderr captured; returns `f`'s value and
/// everything written to stderr meanwhile — this process AND its
/// children. Refuses to nest.
///
/// ```
/// let (value, said) = mcp_core::capture(|| {
///     eprintln!("the run's own words");
///     42
/// })
/// .unwrap();
/// assert_eq!(value, 42);
/// assert!(said.contains("the run's own words"));
/// // Nothing leaks past the guard: stderr is restored afterwards.
/// eprintln!("(post-capture stderr works again)");
/// ```
pub fn capture<T>(f: impl FnOnce() -> T) -> Result<(T, String), McpCoreError> {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return Err(err(
            "begin",
            "a capture is already active — tool dispatch is sequential by design, \
             and captures never nest",
        ));
    }
    // Hold the active flag through a guard so every early return below
    // releases it.
    struct ActiveFlag;
    impl Drop for ActiveFlag {
        fn drop(&mut self) {
            ACTIVE.store(false, Ordering::SeqCst);
        }
    }
    let _flag = ActiveFlag;

    let path = std::env::temp_dir().join(format!(
        "mcp-capture-{}-{}.log",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst),
    ));
    let sink = fs::File::create(&path).map_err(|e| err("create", e))?;

    let restore = platform::redirect_stderr_to(&sink).map_err(|e| err("redirect", e))?;
    // From here stderr is the file; RestoreOnDrop puts it back even if
    // `f` panics (the unwind drops it before propagating).
    let guard = RestoreOnDrop {
        restore: Some(restore),
        path: path.clone(),
    };
    let value = f();
    drop(guard);

    let said = fs::read_to_string(&path).map_err(|e| err("read", e))?;
    let _ = fs::remove_file(&path);
    Ok((value, said))
}

struct RestoreOnDrop {
    restore: Option<platform::Restore>,
    path: PathBuf,
}

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        if let Some(r) = self.restore.take() {
            // A failed restore leaves the process mute on stderr; there
            // is no channel left to complain on, so flush best-effort
            // and carry on — the capture file (kept on this path) still
            // holds the run's words for a post-mortem.
            let _ = std::io::stderr().flush();
            if platform::restore_stderr(r).is_err() {
                let _ = fs::write(
                    self.path.with_extension("restore-failed"),
                    "stderr restore failed; see MCP-CORE-v0.1#capture",
                );
            }
        }
    }
}

#[cfg(unix)]
mod platform {
    use std::fs::File;
    use std::io;
    use std::os::fd::AsRawFd;

    unsafe extern "C" {
        fn dup(fd: i32) -> i32;
        fn dup2(oldfd: i32, newfd: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    /// The saved real stderr, to put back.
    pub struct Restore {
        saved_fd: i32,
    }

    pub fn redirect_stderr_to(sink: &File) -> io::Result<Restore> {
        // SAFETY: plain POSIX fd shuffling on this process's own
        // descriptors; the saved fd is owned by Restore and closed on
        // restore. This cell is the audited home of that unsafety —
        // nothing else in the crate touches raw fds.
        unsafe {
            let saved_fd = dup(2);
            if saved_fd < 0 {
                return Err(io::Error::last_os_error());
            }
            if dup2(sink.as_raw_fd(), 2) < 0 {
                let e = io::Error::last_os_error();
                close(saved_fd);
                return Err(e);
            }
            Ok(Restore { saved_fd })
        }
    }

    pub fn restore_stderr(r: Restore) -> io::Result<()> {
        // SAFETY: as above — restoring the descriptor this module saved.
        unsafe {
            let rc = dup2(r.saved_fd, 2);
            close(r.saved_fd);
            if rc < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::AsRawHandle;

    type Handle = *mut c_void;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn SetStdHandle(std_handle: u32, handle: Handle) -> i32;
    }

    /// The saved real std-error handle, to put back.
    pub struct Restore {
        saved: Handle,
    }

    // A raw HANDLE crossing the Drop boundary; this module only ever
    // hands it back to SetStdHandle on the same process.
    unsafe impl Send for Restore {}

    pub fn redirect_stderr_to(sink: &File) -> io::Result<Restore> {
        // SAFETY: swapping this process's own std-error handle; the
        // previous handle is saved and restored by `restore_stderr`.
        // Rust's stderr writes resolve GetStdHandle per operation and
        // spawned children inherit the current handle, so both are
        // redirected (the capture test pins it).
        unsafe {
            let saved = GetStdHandle(STD_ERROR_HANDLE);
            if SetStdHandle(STD_ERROR_HANDLE, sink.as_raw_handle().cast::<c_void>()) == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Restore { saved })
        }
    }

    pub fn restore_stderr(r: Restore) -> io::Result<()> {
        // SAFETY: as above — restoring the handle this module saved.
        unsafe {
            if SetStdHandle(STD_ERROR_HANDLE, r.saved) == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One CHILD-process line onto the real process stderr. The unit
    /// suite runs under libtest, which diverts THIS thread's
    /// `eprintln!` into its own per-test buffer before the std handle —
    /// so in-process asserts here would test libtest, not the guard.
    /// Children write to the inherited handle itself (the exact channel
    /// the Wave-0 S3 finding is about), and the rustdoc example on
    /// [`capture`] — which runs as its own process, libtest-free — pins
    /// the in-process `eprintln!` path.
    fn child_says(word: &str) {
        #[cfg(windows)]
        let status = std::process::Command::new("cmd")
            .args(["/C", &format!("echo {word} 1>&2")])
            .status();
        #[cfg(unix)]
        let status = std::process::Command::new("sh")
            .args(["-c", &format!("echo {word} >&2")])
            .status();
        assert!(status.expect("spawn child").success());
    }

    /// The whole guard in ONE test: the redirect is process-global
    /// state and libtest runs tests on parallel threads — separate
    /// capture tests would race each other's fd 2. Phases, in order:
    /// (1) child-process stderr is captured (the S3 load-bearing
    /// property a threaded Write could never have); (2) nesting refuses
    /// while the outer capture survives; (3) a panicking tool leaves
    /// the guard restored and reusable; (4) sequential captures do not
    /// bleed into each other.
    #[test]
    fn capture_guard_end_to_end() {
        // (1) the child's words land in the capture, and the value
        // threads through.
        let (value, said) = capture(|| {
            child_says("child-speaks");
            41 + 1
        })
        .unwrap();
        assert_eq!(value, 42);
        assert!(said.contains("child-speaks"), "{said}");

        // (2) nesting refuses; the outer capture still reports.
        let (inner, said) = capture(|| {
            child_says("outer-still-heard");
            capture(|| ()).map(|_| ())
        })
        .unwrap();
        let msg = inner.expect_err("nesting must refuse").to_string();
        assert!(msg.contains("never nest"), "{msg}");
        assert!(msg.contains("MCP-CORE-v0.1#capture"), "{msg}");
        assert!(said.contains("outer-still-heard"), "{said}");

        // (3) a panicking tool cannot leave the process mute or the
        // guard locked.
        let result = std::panic::catch_unwind(|| {
            let _ = capture(|| panic!("tool exploded"));
        });
        assert!(result.is_err());
        let ((), said) = capture(|| child_says("alive-after-panic")).unwrap();
        assert!(said.contains("alive-after-panic"), "{said}");

        // (4) sequential captures are isolated.
        let ((), first) = capture(|| child_says("first-run")).unwrap();
        let ((), second) = capture(|| child_says("second-run")).unwrap();
        assert!(
            first.contains("first-run") && !first.contains("second-run"),
            "{first}"
        );
        assert!(
            second.contains("second-run") && !second.contains("first-run"),
            "{second}"
        );
    }
}
