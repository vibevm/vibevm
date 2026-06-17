//! Transport abstraction for the MCP server.
//!
//! The trait keeps the server testable without spawning a real child
//! process. Production uses [`StdioTransport`]; tests use
//! [`MemoryTransport`] which buffers input and captures output.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#server");

use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::Mutex;

/// Bidirectional line-delimited transport (PROP-015 §2.1).
///
/// `read_line` returns `Ok(Some(line))` for each newline-terminated
/// chunk in the input stream, `Ok(None)` on EOF, or `Err` on I/O
/// failure. Writers append a single trailing `\n`.
///
/// ```
/// use vibe_mcp::Transport;
/// use std::io;
///
/// // A transport that yields one canned line then EOF, dropping writes.
/// struct OneShot(Option<String>);
/// impl Transport for OneShot {
///     fn read_line(&mut self) -> io::Result<Option<String>> {
///         Ok(self.0.take())
///     }
///     fn write_line(&mut self, _line: &str) -> io::Result<()> {
///         Ok(())
///     }
/// }
///
/// let mut t = OneShot(Some("hello\n".to_string()));
/// assert_eq!(t.read_line().unwrap().as_deref(), Some("hello\n"));
/// assert_eq!(t.read_line().unwrap(), None);
/// ```
pub trait Transport {
    fn read_line(&mut self) -> io::Result<Option<String>>;
    fn write_line(&mut self, line: &str) -> io::Result<()>;
}

/// Stdio transport — reads from `stdin`, writes to `stdout`. The
/// server's lifetime ties to whatever process is wrapping it; on
/// EOF the loop terminates.
///
/// ```
/// use vibe_mcp::StdioTransport;
/// // The production transport over the process's stdin/stdout.
/// let _t = StdioTransport::new();
/// ```
pub struct StdioTransport {
    reader: BufReader<io::Stdin>,
    writer: io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        StdioTransport {
            reader: BufReader::new(io::stdin()),
            writer: io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        StdioTransport::new()
    }
}

impl Transport for StdioTransport {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut handle = self.writer.lock();
        handle.write_all(line.as_bytes())?;
        handle.write_all(b"\n")?;
        handle.flush()?;
        Ok(())
    }
}

/// In-memory transport for tests. Construct with the input string;
/// after `Server::run` returns, call [`MemoryTransport::take_output`]
/// to read everything written.
///
/// ```
/// use vibe_mcp::{MemoryTransport, Transport};
/// let mut t = MemoryTransport::with_input("hello\n");
/// assert_eq!(t.read_line().unwrap().as_deref(), Some("hello\n"));
/// ```
pub struct MemoryTransport {
    input: BufReader<std::io::Cursor<Vec<u8>>>,
    output: Mutex<Vec<u8>>,
}

impl MemoryTransport {
    pub fn with_input(s: impl Into<String>) -> Self {
        let bytes: Vec<u8> = s.into().into_bytes();
        MemoryTransport {
            input: BufReader::new(std::io::Cursor::new(bytes)),
            output: Mutex::new(Vec::new()),
        }
    }

    pub fn take_output(&self) -> String {
        let mut guard = self.output.lock().unwrap_or_else(|e| e.into_inner());
        let bytes = std::mem::take(&mut *guard);
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Transport for MemoryTransport {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.input.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut guard = self.output.lock().unwrap_or_else(|e| e.into_inner());
        guard.extend_from_slice(line.as_bytes());
        guard.push(b'\n');
        Ok(())
    }
}

// `Read` re-export so `MemoryTransport`'s cursor reads through
// `read_line` cleanly without callers caring about the underlying
// type.
const _: () = {
    fn _assert_read<R: Read>() {}
    fn _check() {
        _assert_read::<std::io::Cursor<Vec<u8>>>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_transport_round_trip() {
        let mut t = MemoryTransport::with_input("hello\nworld\n");
        assert_eq!(t.read_line().unwrap().as_deref(), Some("hello\n"));
        assert_eq!(t.read_line().unwrap().as_deref(), Some("world\n"));
        assert!(t.read_line().unwrap().is_none());
        t.write_line("response-1").unwrap();
        t.write_line("response-2").unwrap();
        let out = t.take_output();
        assert_eq!(out, "response-1\nresponse-2\n");
    }
}
