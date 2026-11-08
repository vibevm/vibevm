//! Successful logical writes of one transfer, attached to its existing sampler.
//! A sink must be the owned unbuffered file boundary. No physical-I/O claim,
//! retained-size measurement, parent aggregation or unrelated-copy accounting.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes");

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::TransferReader;

#[derive(Clone)]
pub(super) struct DiskWrites(Arc<Mutex<Option<u64>>>);

impl DiskWrites {
    pub(super) fn new() -> Self {
        Self(Arc::new(Mutex::new(Some(0))))
    }

    pub(super) fn bytes(&self) -> Option<u64> {
        self.0.lock().ok().and_then(|count| *count)
    }

    fn accepted(&self, bytes: usize) {
        if let Ok(mut count) = self.0.lock() {
            // Overflow loses representability, not the successful write. Never
            // wrap, fabricate zero, or turn a completed write into an error.
            *count = (*count).and_then(|n| n.checked_add(bytes as u64));
        }
    }
}

/// The observed writer half of [`TransferIo`]. `write_all` uses this type's
/// `write`, crediting each successful short write even if a later call fails.
/// It cannot be constructed or extracted independently of the transfer pair.
///
/// ```
/// use vibe_core::progress::transfer::{Destination, Purpose, Transfer, TransferRecord};
/// let transfer = Transfer::begin(TransferRecord::http(Purpose::Other,
///     "https://example.test/data", Destination::Cache), None, None);
/// let mut pair = transfer.reader(&b"data"[..]).with_disk_writer(tempfile::tempfile().unwrap());
/// let (reader, writer) = pair.parts();
/// assert_eq!(std::io::copy(reader, writer).unwrap(), 4);
/// pair.sync_all().unwrap();
/// pair.finish();
/// ```
pub struct TransferWriter<W> {
    inner: W,
    writes: DiskWrites,
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
impl<W: Write> Write for TransferWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.writes.accepted(written);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// An input and owned disk writer belonging to one transfer. Borrowed parts
/// cannot survive a consuming finish/failure/interruption report. This does
/// not turn logical write acceptance into fsync or retained-size evidence.
/// See [`TransferReader::with_disk_writer`] for a runnable File example.
///
/// ```compile_fail
/// use std::io::Write;
/// use vibe_core::progress::transfer::{Destination, Purpose, Transfer, TransferRecord};
/// let transfer = Transfer::begin(TransferRecord::http(Purpose::Other,
///     "https://example.test/a", Destination::Cache), None, None);
/// let mut pair = transfer.reader(&b"x"[..]).with_disk_writer(tempfile::tempfile().unwrap());
/// let (_, writer) = pair.parts();
/// pair.finish(); // the borrowed writer prevents consuming its transfer
/// writer.write_all(b"after terminal").unwrap();
/// ```
pub struct TransferIo<R, W> {
    // Declaration order also closes the writer first on implicit pair drop,
    // before the reader's Transfer drop emits its failed terminal sample.
    writer: TransferWriter<W>,
    reader: TransferReader<R>,
}

impl<R, W> TransferIo<R, W> {
    pub(super) fn new(reader: TransferReader<R>, writer: W, writes: DiskWrites) -> Self {
        Self {
            reader,
            writer: TransferWriter {
                inner: writer,
                writes,
            },
        }
    }

    /// Borrow both halves for the existing caller-owned copy/validation loop.
    pub fn parts(&mut self) -> (&mut TransferReader<R>, &mut TransferWriter<W>) {
        (&mut self.reader, &mut self.writer)
    }

    /// Consume the writer before emitting the final sample and outcome.
    pub fn finish(self) {
        let Self { reader, writer } = self;
        drop(writer);
        reader.finish();
    }

    /// Preserve all successful partial writes, even if cleanup removes the file.
    pub fn fail(self, message: impl Into<String>) {
        let Self { reader, writer } = self;
        drop(writer);
        reader.fail(message);
    }

    /// Report interruption with the successful-write counter accumulated so far.
    pub fn interrupted(self) {
        let Self { reader, writer } = self;
        drop(writer);
        reader.interrupted();
    }
}

impl<R> TransferIo<R, std::fs::File> {
    /// The owned File's durability boundary. Failure does not subtract logical
    /// writes; callers report it through [`Self::fail`].
    pub fn sync_all(&self) -> io::Result<()> {
        self.writer.inner.sync_all()
    }
}

#[cfg(test)]
mod tests;
