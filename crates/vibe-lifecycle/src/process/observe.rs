//! Best-effort byte observation of file-backed process capture.
//! Independent read handles never change the child's shared file offset.
use super::{CaptureFile, ProcessSpec, StreamMode};
use std::fs::File;
use std::io::{Read, Seek};
use std::sync::Arc;

/// The source of a byte chunk; ordering is guaranteed within each stream only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Invocation-owned optional diagnostics. Failures must not change process outcomes.
///
/// ```
/// use vibe_lifecycle::process::{OutputStream, ProcessOutputObserver};
/// struct Ignore;
/// impl ProcessOutputObserver for Ignore {
///     fn bytes(&self, _: OutputStream, _: &[u8]) {}
///     fn finished(&self, _: bool) {}
/// }
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
pub trait ProcessOutputObserver: Send + Sync {
    fn bytes(&self, stream: OutputStream, bytes: &[u8]);
    /// False means capture could not be fully observed, not a child failure.
    fn finished(&self, complete: bool);
}

pub(super) struct Observation {
    sink: Arc<dyn ProcessOutputObserver>,
    readers: Vec<(OutputStream, File)>,
    complete: bool,
    reaped: bool,
}

impl Observation {
    pub(super) fn new(
        sink: Option<Arc<dyn ProcessOutputObserver>>,
        spec: &ProcessSpec,
        stdout: Option<&CaptureFile>,
        stderr: Option<&CaptureFile>,
    ) -> Option<Self> {
        let sink = sink?;
        let mut this = Self {
            sink,
            readers: Vec::new(),
            complete: true,
            reaped: false,
        };
        for (kind, mode, capture) in [
            (OutputStream::Stdout, spec.stdout, stdout),
            (OutputStream::Stderr, spec.stderr, stderr),
        ] {
            if mode != StreamMode::Capture {
                this.complete = false;
                continue;
            }
            match capture.and_then(|c| File::open(&c.path).ok().map(|f| (c, f))) {
                Some((capture, file)) => {
                    let same = file
                        .metadata()
                        .ok()
                        .zip(capture.file.metadata().ok())
                        .is_some_and(|(a, b)| same_file(&a, &b));
                    if same {
                        this.readers.push((kind, file));
                    } else {
                        this.complete = false;
                    }
                }
                None => this.complete = false,
            }
        }
        Some(this)
    }

    pub(super) fn reaped(&mut self) {
        self.reaped = true;
    }

    /// Each polling pass is bounded so continuous output cannot starve cancellation.
    pub(super) fn poll(&mut self) {
        for (kind, file) in &mut self.readers {
            let mut bytes = [0u8; 16 * 1024];
            for _ in 0..8 {
                match file.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => self.sink.bytes(*kind, &bytes[..n]),
                    Err(_) => {
                        self.complete = false;
                        break;
                    }
                }
            }
        }
    }
}

impl Drop for Observation {
    fn drop(&mut self) {
        // Drain only the measured post-wait extent, not an indefinitely writing child.
        for (kind, file) in &mut self.readers {
            let result = (|| -> std::io::Result<()> {
                let end = file.metadata()?.len();
                let mut remaining = end.saturating_sub(file.stream_position()?);
                let mut bytes = [0u8; 16 * 1024];
                while remaining > 0 {
                    let wanted = remaining.min(bytes.len() as u64) as usize;
                    let n = file.read(&mut bytes[..wanted])?;
                    if n == 0 {
                        return Err(std::io::ErrorKind::UnexpectedEof.into());
                    }
                    self.sink.bytes(*kind, &bytes[..n]);
                    remaining -= n as u64;
                }
                if file.metadata()?.len() != end {
                    self.complete = false;
                }
                Ok(())
            })();
            if result.is_err() {
                self.complete = false;
            }
        }
        self.sink.finished(self.complete && self.reaped);
    }
}

fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        a.is_file() && b.is_file()
    }
}
