//! Best-effort raw logs, separate from bounded machine previews and display lines.
use crate::output;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use vibe_core::progress::ProgressTask;
use vibe_lifecycle::process::{OutputStream, ProcessOutputObserver};

pub(super) struct StreamLog {
    pub(super) path: PathBuf,
    state: Mutex<State>,
    binary: bool,
    task: Arc<ProgressTask>,
    ctx: output::Context,
    mode: crate::commands::app::display::LogMode,
}
struct State {
    file: Option<File>,
    failed: bool,
    finished: bool,
    lines: [Vec<u8>; 2],
}
impl StreamLog {
    pub(super) fn open(
        path: PathBuf,
        binary: bool,
        task: Arc<ProgressTask>,
        ctx: output::Context,
        mode: crate::commands::app::display::LogMode,
    ) -> Arc<Self> {
        let file = path
            .parent()
            .and_then(|p| std::fs::create_dir_all(p).ok())
            .and_then(|_| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .ok()
            });
        let failed = file.is_none();
        Arc::new(Self {
            path,
            binary,
            task,
            ctx,
            mode,
            state: Mutex::new(State {
                file,
                failed,
                finished: false,
                lines: [Vec::new(), Vec::new()],
            }),
        })
    }
    pub(super) fn status(&self) -> (bool, bool) {
        self.state
            .lock()
            .map(|s| (s.file.is_some(), s.failed || !s.finished))
            .unwrap_or((false, true))
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    fn display(&self, lines: Vec<String>) {
        if self.ctx.is_quiet()
            || self.ctx.is_json()
            || self.ctx.is_machine_progress()
            || self.mode == crate::commands::app::display::LogMode::Quiet
        {
            return;
        }
        for line in lines {
            if self.ctx.has_interactive_progress() {
                self.task.detail(line);
            } else {
                self.ctx.step(&crate::output::sanitize_progress_text(&line));
            }
        }
    }
}
impl ProcessOutputObserver for StreamLog {
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
    fn bytes(&self, stream: OutputStream, bytes: &[u8]) {
        if self.binary && stream == OutputStream::Stdout {
            return;
        }
        let mut lines = Vec::new();
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if let Some(file) = &mut state.file
                && file.write_all(bytes).is_err()
            {
                state.failed = true;
            }
            let index = usize::from(stream == OutputStream::Stderr);
            for &byte in bytes {
                if byte == b'\n' {
                    lines.push(String::from_utf8_lossy(&state.lines[index]).into_owned());
                    state.lines[index].clear();
                } else if state.lines[index].len() < 4096 {
                    state.lines[index].push(byte);
                }
            }
        }
        self.display(lines);
    }
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
    fn finished(&self, complete: bool) {
        let mut lines = Vec::new();
        let failed;
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            for line in &mut state.lines {
                if !line.is_empty() {
                    lines.push(String::from_utf8_lossy(line).into_owned());
                    line.clear();
                }
            }
            if let Some(file) = &mut state.file
                && file.flush().is_err()
            {
                state.failed = true;
            }
            state.failed |= !complete;
            state.finished = true;
            failed = state.failed;
        }
        self.display(lines);
        if failed {
            self.ctx.diagnostic(&format!(
                "output log incomplete or unavailable: {}",
                self.path.display()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> output::Context {
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
    }
    fn task() -> Arc<ProgressTask> {
        Arc::new(vibe_core::progress::Progress::default().task("test"))
    }
    #[test]
    fn unavailable_destination_is_explicit_and_never_propagates() {
        let root = tempfile::tempdir().unwrap();
        let blocked = root.path().join("blocked");
        std::fs::write(&blocked, b"file").unwrap();
        let log = StreamLog::open(
            blocked.join("output.log"),
            false,
            task(),
            context(),
            crate::commands::app::display::LogMode::Quiet,
        );
        log.bytes(OutputStream::Stdout, b"still successful work");
        log.finished(true);
        assert_eq!(log.status(), (false, true));
    }
    #[test]
    fn raw_full_log_retains_large_and_invalid_bytes_but_not_binary_protocol() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("output.log");
        let log = StreamLog::open(
            path.clone(),
            true,
            task(),
            context(),
            crate::commands::app::display::LogMode::Quiet,
        );
        log.bytes(OutputStream::Stdout, b"PRIVATE-PROTOCOL");
        let bytes = vec![255; 2 * 1024 * 1024];
        for chunk in bytes.chunks(16384) {
            log.bytes(OutputStream::Stderr, chunk);
        }
        log.finished(true);
        assert_eq!(log.status(), (true, false));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
