//! Clones and fetches as reported transfers (`PROP-068#git`): git runs
//! with `--progress`, its stderr is read as it arrives, and every
//! `Receiving objects` (or, for a small fetch, `Unpacking objects`) update
//! that names a size becomes a sample of a `git` transfer of precision
//! `reported`. The transfer begins with the first such line, so a fetch
//! that moves nothing leaves no transfer at all; at a successful end the
//! journal learns the size of the repository's object store.
//!
//! The progress lines are kept out of the stderr the failure classifier
//! and the error message read: they would bury the one line that says why.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#git");

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use vibe_core::progress::transfer::{self, Destination, Purpose, Transfer, TransferRecord};

use super::super::transfers;
use super::*;

/// Where the bytes of a clone or fetch come from.
pub(super) enum Remote<'a> {
    /// The URL a clone names.
    Url(&'a str),
    /// The `origin` of the repository a fetch runs in.
    Origin,
    Admitted(&'a str, native_admission::NativeProtocol),
}

impl ShellGit {
    /// Run the clone or fetch `args` (the subcommand first) with
    /// `--progress`, in `cwd`, into the repository at `repository`,
    /// relaying git's progress as a transfer of the observation around
    /// the call.
    pub(super) fn run_transferring(
        &self,
        args: &[&str],
        cwd: Option<&Path>,
        remote: Remote<'_>,
        repository: &Path,
    ) -> Result<Output, GitError> {
        self.run_transferring_with_prefix(&[], args, cwd, remote, repository)
    }

    /// Preserve transfer accounting when global Git options select an object
    /// repository while the process keeps the source's original working context.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION"
    )]
    pub(super) fn run_transferring_with_prefix(
        &self,
        prefix: &[&str],
        args: &[&str],
        cwd: Option<&Path>,
        remote: Remote<'_>,
        repository: &Path,
    ) -> Result<Output, GitError> {
        self.admit_native()?;
        let mut argv: Vec<&str> = Vec::with_capacity(prefix.len() + args.len() + 1);
        argv.extend(prefix);
        argv.extend(args.first());
        argv.push("--progress");
        argv.extend(args.iter().skip(1));

        let mut cmd = Command::new(&self.binary);
        apply_common_env(&mut cmd, self.force_anonymous);
        cmd.args(&argv);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        self.finish_transfer(cmd, &argv, args, cwd, remote, repository)
    }

    pub(super) fn run_admitted_transfer(
        &self,
        command: native_admission::AdmittedCommand,
        argv: &[&str],
        url: &str,
        repository: &Path,
    ) -> Result<Output, GitError> {
        let (command, protocol) = command.into_parts();
        self.finish_transfer(
            command,
            argv,
            argv,
            None,
            Remote::Admitted(url, protocol),
            repository,
        )
    }

    fn finish_transfer(
        &self,
        mut cmd: Command,
        argv: &[&str],
        args: &[&str],
        cwd: Option<&Path>,
        remote: Remote<'_>,
        repository: &Path,
    ) -> Result<Output, GitError> {
        let binary = Path::new(cmd.get_program()).to_path_buf();
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        tracing::debug!(target: "vibe_registry::git", argv = ?render_argv(&binary, argv), cwd = ?cwd, "running git (transfer)");

        let spawn_error = |source| GitError::Io {
            cmd: render_argv(&binary, argv),
            source,
        };
        let cancellation = transfers::current()
            .progress
            .unwrap_or_default()
            .cancellation()
            .clone();
        cancellation
            .checkpoint()
            .map_err(|_| GitError::ContactRefused(crate::ContactRefusal::OperationCancelled))?;
        let began = std::time::Instant::now();
        let mut child =
            super::controlled::spawn_transfer(&mut cmd, &cancellation).map_err(spawn_error)?;
        // stdout is drained beside stderr so neither pipe can fill and
        // stall git while the other is being read.
        let stdout = child.inner().stdout.take().map(|mut pipe| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                bytes
            })
        });
        let mut relay = Relay::default();
        let mut stderr = Vec::new();
        let (segments, receiver) = std::sync::mpsc::sync_channel(32);
        let stderr_reader = child.inner().stderr.take().map(|pipe| {
            std::thread::spawn(move || {
                for_each_segment(pipe, |segment| {
                    let _ = segments.send(segment.to_owned());
                });
            })
        });
        let mut cancelled = false;
        loop {
            if cancellation.checkpoint().is_err() {
                cancelled = true;
                stop_child(&mut child).map_err(spawn_error)?;
                break;
            }
            match receiver.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(segment) => {
                    if let Some(receipt) = Receipt::parse(&segment) {
                        let figure = relay.cumulative(receipt.bytes);
                        let total = receipt.whole.then_some(figure);
                        relay
                            .transfer
                            .get_or_insert_with(|| self.begin_transfer(&remote, cwd, began))
                            .reported(figure, total);
                    }
                    if !is_progress(&segment) {
                        stderr.extend_from_slice(segment.as_bytes());
                        stderr.push(b'\n');
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(receiver);
        if let Some(reader) = stderr_reader {
            let _ = reader.join();
        }
        let status = child.wait().map_err(spawn_error)?;
        let stdout = stdout
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        if cancelled {
            if let Some(transfer) = relay.transfer {
                transfer.interrupted();
            }
            return Err(GitError::ContactRefused(
                crate::ContactRefusal::OperationCancelled,
            ));
        }
        let output = Output {
            status,
            stdout,
            stderr,
        };
        if output.status.success() {
            if let Some(mut transfer) = relay.transfer {
                let store = if self.native.is_some() {
                    repository.join("objects")
                } else {
                    repository.join(".git").join("objects")
                };
                transfer.written(size_on_disk(&store));
                transfer.finish();
            }
            return Ok(output);
        }
        let ssh_error = if matches!(
            remote,
            Remote::Admitted(_, native_admission::NativeProtocol::Ssh)
        ) {
            native_admission::ssh::failure(&output.stderr)
        } else {
            None
        };
        let error = if let Some(error) = ssh_error {
            error
        } else if self.native.is_some() {
            GitError::CommandFailed {
                cmd: "admitted native Git transfer".into(),
                status: output.status.code().unwrap_or(-1),
                stderr:
                    "direct transfer failed; redirects and ambient credential routes are disabled"
                        .into(),
            }
        } else {
            classify_failure(argv, &output)
        };
        if let Some(transfer) = relay.transfer {
            transfer.fail(format!("git {} failed", args.first().unwrap_or(&"")));
        }
        Err(error)
    }

    /// The transfer of this run, dated from when git was started: the
    /// bytes git negotiated and received before its first sized line belong
    /// to the same seconds.
    fn begin_transfer(
        &self,
        remote: &Remote<'_>,
        cwd: Option<&Path>,
        began: std::time::Instant,
    ) -> Transfer {
        let url = match remote {
            Remote::Url(url) | Remote::Admitted(url, _) => (*url).to_owned(),
            Remote::Origin => self.origin_url(cwd),
        };
        let scope = transfers::current();
        let purpose = scope.purpose.unwrap_or(Purpose::PackageClone);
        let mut record = TransferRecord::git(purpose, &url, Destination::Store);
        if let Some(package) = scope.package {
            record = record.package(package);
        }
        transfer::begin_beneath(record, scope.progress.as_ref(), self.transfers.get(), began)
    }

    /// The `origin` URL of the repository in `cwd`; the record reduces it.
    fn origin_url(&self, cwd: Option<&Path>) -> String {
        self.run_raw(&["config", "--get", "remote.origin.url"], cwd)
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| "remote endpoint".to_owned())
    }
}

/// The transfer of one git run and the figures git reported so far.
#[derive(Default)]
struct Relay {
    transfer: Option<Transfer>,
    /// The bytes of the passes before the current one.
    base: u64,
    /// The last figure of the current pass.
    last: u64,
}

impl Relay {
    /// The cumulative figure of a line. A clone that recurses into
    /// submodules prints one pass per repository, each starting from zero;
    /// a figure below the previous one starts a new pass, whose bytes add
    /// to the ones before.
    fn cumulative(&mut self, bytes: u64) -> u64 {
        if bytes < self.last {
            self.base += self.last;
        }
        self.last = bytes;
        self.base + bytes
    }
}

/// One progress line that names the bytes received so far.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Receipt {
    pub(super) bytes: u64,
    /// Git said `100%`: the figure is the whole.
    pub(super) whole: bool,
}

impl Receipt {
    /// `Receiving objects:  45% (900/2000), 12.30 MiB | 2.10 MiB/s` → the
    /// bytes scaled from git's binary unit. A line without a size (git
    /// prints the size only once it has measured a throughput) is none.
    pub(super) fn parse(line: &str) -> Option<Self> {
        let line = line.trim_start();
        let rest = line
            .strip_prefix("Receiving objects:")
            .or_else(|| line.strip_prefix("Unpacking objects:"))?;
        let (head, tail) = rest.split_once("), ")?;
        let percent: u32 = head.trim().split('%').next()?.trim().parse().ok()?;
        let size = tail.split('|').next()?.trim();
        let (number, unit) = size.split_once(' ')?;
        let value: f64 = number.parse().ok()?;
        let scale: f64 = match unit.trim().trim_end_matches(',') {
            "byte" | "bytes" => 1.0,
            "KiB" => 1024.0,
            "MiB" => 1024.0 * 1024.0,
            "GiB" => 1024.0 * 1024.0 * 1024.0,
            _ => return None,
        };
        Some(Self {
            bytes: (value * scale).round() as u64,
            whole: percent == 100,
        })
    }
}

/// Whether a stderr segment is one of git's progress lines, which carry no
/// diagnosis.
fn is_progress(segment: &str) -> bool {
    const PREFIXES: [&str; 10] = [
        "Enumerating objects:",
        "Counting objects:",
        "Compressing objects:",
        "Total ",
        "Receiving objects:",
        "Resolving deltas:",
        "Unpacking objects:",
        "Checking connectivity",
        "Checking objects:",
        "Updating files:",
    ];
    let line = segment.trim_start();
    let line = line.strip_prefix("remote:").map_or(line, str::trim_start);
    line.is_empty() || PREFIXES.iter().any(|prefix| line.starts_with(prefix))
}

/// Call `each` with every segment of `pipe` as it arrives: git separates
/// the updates of one progress line with `\r` and ends lines with `\n`.
fn for_each_segment(mut pipe: impl Read, mut each: impl FnMut(&str)) {
    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        for &byte in &chunk[..read] {
            if byte == b'\r' || byte == b'\n' {
                each(&String::from_utf8_lossy(&pending));
                pending.clear();
            } else {
                pending.push(byte);
            }
        }
    }
    if !pending.is_empty() {
        each(&String::from_utf8_lossy(&pending));
    }
}

/// The bytes of every file under `root`.
fn size_on_disk(root: &Path) -> u64 {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(std::fs::Metadata::is_file)
        .map(|metadata| metadata.len())
        .sum()
}

/// Stop the owned Unix process group or Windows job before reaping it.
fn stop_child(child: &mut super::controlled::TransferChild) -> std::io::Result<()> {
    child.kill()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receiving_line_becomes_bytes_in_git_binary_units() {
        let parse = Receipt::parse;
        assert_eq!(
            parse("Receiving objects:  45% (900/2000), 12.30 MiB | 2.10 MiB/s"),
            Some(Receipt {
                bytes: (12.30 * 1048576.0_f64).round() as u64,
                whole: false
            })
        );
        assert_eq!(
            parse("Receiving objects: 100% (5/5), 300.50 KiB | 1.20 MiB/s, done."),
            Some(Receipt {
                bytes: 307_712,
                whole: true
            })
        );
        assert_eq!(
            parse("Unpacking objects: 100% (3/3), 250 bytes | 250.00 KiB/s, done."),
            Some(Receipt {
                bytes: 250,
                whole: true
            })
        );
        assert_eq!(parse("Receiving objects:  45% (900/2000)"), None);
        assert_eq!(parse("Receiving objects: 100% (3/3), done."), None);
        assert_eq!(parse("Resolving deltas: 100% (2/2), done."), None);
    }

    #[test]
    fn a_second_pass_adds_to_the_first() {
        let mut relay = Relay::default();
        assert_eq!(relay.cumulative(100), 100);
        assert_eq!(relay.cumulative(400), 400);
        assert_eq!(relay.cumulative(50), 450);
    }

    #[test]
    fn progress_lines_stay_out_of_the_diagnosis() {
        assert!(is_progress("remote: Counting objects: 100% (3/3), done."));
        assert!(is_progress("Receiving objects:  12% (1/8)"));
        assert!(!is_progress("fatal: repository 'x' not found"));
    }

    #[test]
    fn segments_split_on_carriage_returns_and_newlines() {
        let mut seen = Vec::new();
        for_each_segment(&b"a: 1%\ra: 50%\ra: 100%, done.\nfatal: no"[..], |s| {
            seen.push(s.to_owned())
        });
        assert_eq!(seen, ["a: 1%", "a: 50%", "a: 100%, done.", "fatal: no"]);
    }
}
