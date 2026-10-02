//! Bounded incremental codec values, not a public transport grammar or admission service.
//! Generic polling helpers require a nonblocking/polling port: arbitrary Read/Write
//! implementations can block. Only TcpFramePort establishes nonblocking mode here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use super::framing::{decode_body, FrameCodecError, FRAME_EPOCH};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use vibe_wire::generated::machine::e1::session_frame::SessionFrame;

pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const POLL_BYTES: usize = 4096;

#[derive(Debug)]
pub enum ReadPoll {
    Pending,
    Frame(SessionFrame),
    Closed,
    Terminal,
}
#[derive(Debug, PartialEq, Eq)]
pub enum WritePoll {
    Idle,
    Pending,
    Complete,
    Terminal,
}
#[derive(Debug)]
pub enum QueueError {
    Busy,
    Terminal,
    Codec(FrameCodecError),
}

pub struct Decoder {
    prefix: [u8; 4],
    prefix_pos: usize,
    body: Vec<u8>,
    body_pos: usize,
    terminal: bool,
}
impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Decoder {
    pub fn new() -> Self {
        Self {
            prefix: [0; 4],
            prefix_pos: 0,
            body: Vec::new(),
            body_pos: 0,
            terminal: false,
        }
    }
    /// Reads at most POLL_BYTES and returns at most one complete frame.
    /// WouldBlock and Interrupted yield with every partial position intact.
    pub fn poll_read<R: Read>(&mut self, reader: &mut R) -> Result<ReadPoll, FrameCodecError> {
        if self.terminal {
            return Ok(ReadPoll::Terminal);
        }
        let result = self.read_inner(reader);
        if result.is_err() || matches!(result, Ok(ReadPoll::Closed)) {
            self.terminal = true;
        }
        result
    }
    fn read_inner<R: Read>(&mut self, reader: &mut R) -> Result<ReadPoll, FrameCodecError> {
        let mut budget = POLL_BYTES;
        while budget > 0 {
            let prefix = self.prefix_pos < 4;
            let buffer = if prefix {
                &mut self.prefix[self.prefix_pos..]
            } else {
                &mut self.body[self.body_pos..]
            };
            let limit = buffer.len().min(budget);
            let count = match reader.read(&mut buffer[..limit]) {
                Ok(0) if prefix && self.prefix_pos == 0 => return Ok(ReadPoll::Closed),
                Ok(0) => {
                    return Err(if prefix {
                        FrameCodecError::TruncatedPrefix
                    } else {
                        FrameCodecError::TruncatedBody
                    })
                }
                Ok(count) => count,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(ReadPoll::Pending)
                }
                Err(e) => return Err(FrameCodecError::Io(e)),
            };
            budget -= count;
            if prefix {
                self.prefix_pos += count;
                if self.prefix_pos == 4 {
                    let declared = u32::from_be_bytes(self.prefix) as usize;
                    if declared == 0 {
                        return Err(FrameCodecError::ZeroLength);
                    }
                    if declared > MAX_FRAME_BYTES {
                        return Err(FrameCodecError::OverLimit { declared });
                    }
                    self.body = vec![0; declared];
                }
            } else {
                self.body_pos += count;
                if self.body_pos == self.body.len() {
                    let frame = decode_body(&self.body)?;
                    self.prefix_pos = 0;
                    self.body_pos = 0;
                    self.body.clear();
                    return Ok(ReadPoll::Frame(frame));
                }
            }
        }
        Ok(ReadPoll::Pending)
    }
}

// Abort serialization as soon as its output exceeds the cap. Never build a
// second unbounded Vec for a large caller-owned typed frame.
struct Collector {
    bytes: Vec<u8>,
    over: Option<usize>,
}
impl Write for Collector {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let declared = self
            .bytes
            .len()
            .saturating_sub(4)
            .saturating_add(bytes.len());
        if declared > MAX_FRAME_BYTES {
            self.over = Some(declared);
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct Encoder {
    bytes: Vec<u8>,
    offset: usize,
    finishing: bool,
    terminal: bool,
}
impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Encoder {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            offset: 0,
            finishing: false,
            terminal: false,
        }
    }
    /// A queued frame cannot be replaced, even when its final flush is pending.
    pub fn enqueue(&mut self, frame: &SessionFrame) -> Result<(), QueueError> {
        if self.terminal {
            return Err(QueueError::Terminal);
        }
        if !self.bytes.is_empty() {
            return Err(QueueError::Busy);
        }
        if frame.schema != FRAME_EPOCH {
            return Err(QueueError::Codec(FrameCodecError::UnsupportedEpoch(
                frame.schema,
            )));
        }
        let mut collector = Collector {
            bytes: Vec::with_capacity(MAX_FRAME_BYTES + 4),
            over: None,
        };
        collector.bytes.extend_from_slice(&[0; 4]);
        if serde_json::to_writer(&mut collector, frame).is_err() {
            return Err(QueueError::Codec(match collector.over {
                Some(declared) => FrameCodecError::OverLimit { declared },
                None => FrameCodecError::Serialization,
            }));
        }
        let length = collector.bytes.len() - 4;
        if length == 0 {
            return Err(QueueError::Codec(FrameCodecError::ZeroLength));
        }
        collector.bytes[..4].copy_from_slice(&(length as u32).to_be_bytes());
        self.bytes = collector.bytes;
        self.offset = 0;
        self.finishing = false;
        Ok(())
    }
    /// Writes at most POLL_BYTES; interrupted writes/flushes yield without retry loops.
    pub fn poll_write<W: Write>(&mut self, writer: &mut W) -> Result<WritePoll, FrameCodecError> {
        if self.terminal {
            return Ok(WritePoll::Terminal);
        }
        let result = self.write_inner(writer);
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    fn write_inner<W: Write>(&mut self, writer: &mut W) -> Result<WritePoll, FrameCodecError> {
        if self.bytes.is_empty() {
            return Ok(WritePoll::Idle);
        }
        let mut budget = POLL_BYTES;
        while !self.finishing && budget > 0 {
            let end = self.bytes.len().min(self.offset + budget);
            match writer.write(&self.bytes[self.offset..end]) {
                Ok(0) => return Err(FrameCodecError::Io(io::ErrorKind::WriteZero.into())),
                Ok(count) => {
                    self.offset += count;
                    budget -= count;
                    self.finishing = self.offset == self.bytes.len();
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(WritePoll::Pending)
                }
                Err(e) => return Err(FrameCodecError::Io(e)),
            }
        }
        if !self.finishing {
            return Ok(WritePoll::Pending);
        }
        match writer.flush() {
            Ok(()) => {
                self.bytes.clear();
                self.offset = 0;
                self.finishing = false;
                Ok(WritePoll::Complete)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(WritePoll::Pending)
            }
            Err(e) => Err(FrameCodecError::Io(e)),
        }
    }
}

/// Owns only a connected socket. The caller host must authenticate and admit
/// that connection before construction; codec values confer no RootScope authority.
/// Drop or terminal failure closes this connection, never a domain job.
pub struct TcpFramePort {
    stream: Option<TcpStream>,
    decoder: Decoder,
    encoder: Encoder,
}
impl TcpFramePort {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream: Some(stream),
            decoder: Decoder::new(),
            encoder: Encoder::new(),
        })
    }
    pub fn enqueue(&mut self, frame: &SessionFrame) -> Result<(), QueueError> {
        if self.stream.is_none() {
            return Err(QueueError::Terminal);
        }
        self.encoder.enqueue(frame)
    }
    pub fn poll_read(&mut self) -> Result<ReadPoll, FrameCodecError> {
        let Some(stream) = self.stream.as_mut() else {
            return Ok(ReadPoll::Terminal);
        };
        let result = self.decoder.poll_read(stream);
        if result.is_err() || matches!(result, Ok(ReadPoll::Closed | ReadPoll::Terminal)) {
            self.stream.take();
        }
        result
    }
    pub fn poll_write(&mut self) -> Result<WritePoll, FrameCodecError> {
        let Some(stream) = self.stream.as_mut() else {
            return Ok(WritePoll::Terminal);
        };
        let result = self.encoder.poll_write(stream);
        if result.is_err() {
            self.stream.take();
        }
        result
    }
}
#[cfg(test)]
mod tests;
