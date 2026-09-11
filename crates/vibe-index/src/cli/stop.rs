//! `vibe-index stop <data-dir>` — gracefully stop a running server.
//!
//! Reads the PID from `<data-dir>/state/server.lock` and (on Unix)
//! sends SIGTERM. On Windows, signal-based termination is not
//! straightforwardly supported from a Rust CLI without additional
//! dependencies; the stub instead reports the PID so the operator
//! can `taskkill /PID <n>` themselves. Slice 11's docs cover both
//! platforms in the operator handbook.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;
#[cfg(unix)]
use std::process::Command;

use clap::Parser;

use crate::error::{Error, Result};
use crate::lock::ServerLock;

#[derive(Debug, Parser)]
#[command(about = "Gracefully stop a running server (PID-based).")]
pub struct Args {
    pub data_dir: PathBuf,
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root")]
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
        match Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
        {
            Ok(status) if status.success() => println!("sent SIGTERM"),
            Ok(status) => println!("kill -TERM exited with {status}; check the PID is still alive"),
            Err(error) => println!(
                "could not invoke the POSIX kill utility ({error}); stop PID {pid} manually"
            ),
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
