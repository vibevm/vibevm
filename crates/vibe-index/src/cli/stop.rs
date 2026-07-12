//! `vibe-index stop <data-dir>` — gracefully stop a running server.
//!
//! Reads the PID from `<data-dir>/state/server.lock` and (on Unix)
//! sends SIGTERM. On Windows, signal-based termination is not
//! straightforwardly supported from a Rust CLI without additional
//! dependencies; the stub instead reports the PID so the operator
//! can `taskkill /PID <n>` themselves. Slice 11's docs cover both
//! platforms in the operator handbook.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};
use crate::lock::ServerLock;

#[derive(Debug, Parser)]
#[command(about = "Gracefully stop a running server (PID-based).")]
pub struct Args {
    pub data_dir: PathBuf,
}

#[specmark::spec(
    deviates = "spec://org.vibevm.ai-native.core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: libc::kill is unsafe by FFI ABI, not by memory — \
              a (pid, SIGTERM) value pair crosses the boundary, no pointers; \
              the pid comes from the server lockfile and a stale one yields \
              ESRCH, which the operator-facing message already covers"
)]
pub fn run(args: Args) -> Result<()> {
    let Some(pid) = ServerLock::read_pid(&args.data_dir) else {
        return Err(Error::InvalidInput(format!(
            "no `state/server.lock` in `{}` — no running server to stop",
            args.data_dir.display()
        )));
    };
    println!("vibe-index server PID is {pid}");
    #[cfg(unix)]
    {
        match unsafe { libc::kill(pid as i32, libc::SIGTERM) } {
            0 => println!("sent SIGTERM"),
            _ => println!("kill(2) returned an error; check the PID is still alive"),
        }
    }
    #[cfg(not(unix))]
    {
        println!(
            "this platform has no portable signal mechanism; \
             stop the process manually (taskkill /PID {pid})"
        );
    }
    Ok(())
}
