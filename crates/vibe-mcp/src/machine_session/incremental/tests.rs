use super::*;
use std::collections::VecDeque;
use std::io::Cursor;
use std::net::TcpListener;
use std::time::{Duration, Instant};

fn frame() -> SessionFrame {
    serde_json::from_str(r#"{"schema":1,"session_id":null,"request_id":"r","body":{"kind":"request","context_id":"c","target_id":null,"envelope_json":"{}"}}"#).unwrap()
}
fn wire(frame: &SessionFrame) -> Vec<u8> {
    let mut out = Vec::new();
    super::super::framing::write_frame(&mut out, frame).unwrap();
    out
}
fn framed(body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(body);
    out
}
struct SplitReader {
    bytes: Cursor<Vec<u8>>,
    split: usize,
    pauses: usize,
    polled: usize,
}
impl Read for SplitReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let pos = self.bytes.position() as usize;
        if pos == self.split && self.pauses < 2 {
            self.pauses += 1;
            return Err(if self.pauses == 1 {
                io::ErrorKind::WouldBlock
            } else {
                io::ErrorKind::Interrupted
            }
            .into());
        }
        let limit = if pos < self.split {
            out.len().min(self.split - pos)
        } else {
            out.len()
        };
        let count = self.bytes.read(&mut out[..limit])?;
        self.polled += count;
        Ok(count)
    }
}
#[test]
fn every_prefix_and_body_split_retains_positions_and_delivers_once() {
    let bytes = wire(&frame());
    for split in 0..=bytes.len() {
        let mut reader = SplitReader {
            bytes: Cursor::new(bytes.clone()),
            split,
            pauses: 0,
            polled: 0,
        };
        let mut decoder = Decoder::new();
        let mut delivered = 0;
        for _ in 0..8 {
            reader.polled = 0;
            match decoder.poll_read(&mut reader).unwrap() {
                ReadPoll::Frame(value) => {
                    assert_eq!(value, frame());
                    delivered += 1;
                }
                ReadPoll::Closed => break,
                ReadPoll::Pending => (),
                ReadPoll::Terminal => panic!("premature terminal"),
            }
            assert!(reader.polled <= POLL_BYTES);
        }
        assert_eq!(delivered, 1, "split {split}");
        assert!(matches!(
            decoder.poll_read(&mut reader),
            Ok(ReadPoll::Terminal)
        ));
    }
}
#[test]
fn consecutive_frames_are_not_consumed_or_spliced() {
    let mut bytes = wire(&frame());
    let mut second = frame();
    second.request_id = Some("second".into());
    bytes.extend(wire(&second));
    let mut reader = Cursor::new(bytes);
    let mut decoder = Decoder::new();
    assert!(matches!(decoder.poll_read(&mut reader), Ok(ReadPoll::Frame(f)) if f == frame()));
    assert_eq!(reader.position() as usize, wire(&frame()).len());
    assert!(matches!(decoder.poll_read(&mut reader), Ok(ReadPoll::Frame(f)) if f == second));
    assert!(matches!(
        decoder.poll_read(&mut reader),
        Ok(ReadPoll::Closed)
    ));
}
#[test]
fn eof_invalid_input_and_epoch_poison_without_retry() {
    let mut wrong = frame();
    wrong.schema = 2;
    let cases = vec![
        (vec![0], 0),
        (vec![0, 0, 0], 0),
        (vec![0, 0, 0, 2, b'{'], 1),
        (vec![0; 4], 2),
        (((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().to_vec(), 3),
        (framed(&[0xff]), 4),
        (framed(b"{"), 5),
        (framed(b"{}"), 6),
        (framed(&serde_json::to_vec(&wrong).unwrap()), 7),
    ];
    for (bytes, expected) in cases {
        let mut reader = Cursor::new(bytes);
        let mut decoder = Decoder::new();
        let error = decoder.poll_read(&mut reader).unwrap_err();
        assert!(match expected {
            0 => matches!(error, FrameCodecError::TruncatedPrefix),
            1 => matches!(error, FrameCodecError::TruncatedBody),
            2 => matches!(error, FrameCodecError::ZeroLength),
            3 =>
                matches!(error, FrameCodecError::OverLimit { declared } if declared == MAX_FRAME_BYTES + 1),
            4 => matches!(error, FrameCodecError::InvalidUtf8),
            5 => matches!(error, FrameCodecError::InvalidJson),
            6 => matches!(error, FrameCodecError::InvalidShape),
            7 => matches!(error, FrameCodecError::UnsupportedEpoch(2)),
            _ => false,
        });
        let pos = reader.position();
        assert!(matches!(
            decoder.poll_read(&mut reader),
            Ok(ReadPoll::Terminal)
        ));
        assert_eq!(pos, reader.position());
        if matches!(expected, 0 | 2 | 3) {
            assert_eq!(decoder.body.capacity(), 0);
        } else if expected == 1 {
            assert_eq!(decoder.body.len(), 2);
            assert!(decoder.body.capacity() <= MAX_FRAME_BYTES);
        }
    }
    let mut unknown = serde_json::to_value(frame()).unwrap();
    unknown["body"]["extra"] = serde_json::json!(true);
    assert!(matches!(
        Decoder::new().poll_read(&mut Cursor::new(framed(
            &serde_json::to_vec(&unknown).unwrap()
        ))),
        Err(FrameCodecError::InvalidShape)
    ));
}
struct ScriptWriter {
    out: Vec<u8>,
    actions: VecDeque<io::Result<usize>>,
    flushes: VecDeque<io::Result<()>>,
    polled: usize,
}
impl Write for ScriptWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self
            .actions
            .pop_front()
            .unwrap_or(Ok(bytes.len()))?
            .min(bytes.len());
        self.out.extend_from_slice(&bytes[..count]);
        self.polled += count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes.pop_front().unwrap_or(Ok(()))
    }
}
fn writer() -> ScriptWriter {
    ScriptWriter {
        out: Vec::new(),
        actions: VecDeque::new(),
        flushes: VecDeque::new(),
        polled: 0,
    }
}
#[test]
fn interrupted_partial_writes_and_finishing_flush_preserve_exact_frame() {
    for split in 1..wire(&frame()).len() {
        let mut encoder = Encoder::new();
        encoder.enqueue(&frame()).unwrap();
        let mut output = writer();
        output.actions = VecDeque::from([
            Ok(split),
            Err(io::ErrorKind::WouldBlock.into()),
            Err(io::ErrorKind::Interrupted.into()),
        ]);
        output.flushes = VecDeque::from([
            Err(io::ErrorKind::WouldBlock.into()),
            Err(io::ErrorKind::Interrupted.into()),
            Ok(()),
        ]);
        for _ in 0..4 {
            assert_eq!(encoder.poll_write(&mut output).unwrap(), WritePoll::Pending);
            assert!(matches!(encoder.enqueue(&frame()), Err(QueueError::Busy)));
        }
        assert!(encoder.finishing);
        assert_eq!(
            encoder.poll_write(&mut output).unwrap(),
            WritePoll::Complete
        );
        assert_eq!(output.out, wire(&frame()));
        assert_eq!(encoder.poll_write(&mut output).unwrap(), WritePoll::Idle);
        encoder.enqueue(&frame()).unwrap();
    }
}
#[test]
fn write_zero_io_errors_are_terminal_and_queue_refusals_are_typed() {
    for error in [Ok(0), Err(io::ErrorKind::BrokenPipe.into())] {
        let mut output = writer();
        output.actions.push_back(error);
        let mut encoder = Encoder::new();
        encoder.enqueue(&frame()).unwrap();
        assert!(
            matches!(encoder.poll_write(&mut output), Err(FrameCodecError::Io(e)) if matches!(e.kind(), io::ErrorKind::WriteZero | io::ErrorKind::BrokenPipe))
        );
        assert_eq!(
            encoder.poll_write(&mut output).unwrap(),
            WritePoll::Terminal
        );
        assert!(matches!(
            encoder.enqueue(&frame()),
            Err(QueueError::Terminal)
        ));
    }
    let mut output = writer();
    output
        .flushes
        .push_back(Err(io::ErrorKind::BrokenPipe.into()));
    let mut encoder = Encoder::new();
    encoder.enqueue(&frame()).unwrap();
    assert!(matches!(
        encoder.poll_write(&mut output),
        Err(FrameCodecError::Io(_))
    ));
    assert_eq!(
        encoder.poll_write(&mut output).unwrap(),
        WritePoll::Terminal
    );
    let mut invalid = frame();
    invalid.schema = 2;
    let mut encoder = Encoder::new();
    assert!(matches!(
        encoder.enqueue(&invalid),
        Err(QueueError::Codec(FrameCodecError::UnsupportedEpoch(2)))
    ));
    let mut huge = frame();
    let vibe_wire::generated::machine::e1::session_frame::Body::Request(body) = &mut huge.body
    else {
        unreachable!()
    };
    body.envelope_json = "x".repeat(MAX_FRAME_BYTES * 4);
    assert!(matches!(
        encoder.enqueue(&huge),
        Err(QueueError::Codec(FrameCodecError::OverLimit { .. }))
    ));
    assert!(encoder.bytes.is_empty());
    encoder.enqueue(&frame()).unwrap();
}
#[test]
fn work_and_persistent_serialized_buffers_stay_bounded() {
    let mut large = frame();
    let vibe_wire::generated::machine::e1::session_frame::Body::Request(body) = &mut large.body
    else {
        unreachable!()
    };
    body.envelope_json = "x".repeat(MAX_FRAME_BYTES - 200);
    let mut encoder = Encoder::new();
    encoder.enqueue(&large).unwrap();
    assert!(encoder.bytes.len() <= MAX_FRAME_BYTES + 4);
    assert!(encoder.bytes.capacity() <= MAX_FRAME_BYTES + 4);
    let mut output = writer();
    for _ in 0..100 {
        output.polled = 0;
        let result = encoder.poll_write(&mut output).unwrap();
        assert!(output.polled <= POLL_BYTES);
        if result == WritePoll::Complete {
            break;
        }
    }
    assert_eq!(output.out, wire(&large));
    let mut reader = SplitReader {
        bytes: Cursor::new(output.out),
        split: usize::MAX,
        pauses: 0,
        polled: 0,
    };
    let mut decoder = Decoder::new();
    let mut delivered = false;
    for _ in 0..100 {
        reader.polled = 0;
        let result = decoder.poll_read(&mut reader).unwrap();
        assert!(reader.polled <= POLL_BYTES);
        assert!(decoder.body.len() <= MAX_FRAME_BYTES);
        assert!(decoder.body.capacity() <= MAX_FRAME_BYTES);
        if let ReadPoll::Frame(f) = result {
            assert_eq!(f, large);
            delivered = true;
            break;
        }
    }
    assert!(delivered);
}
fn pair() -> (TcpFramePort, TcpStream) {
    // Listening and accepting are test setup only; the production constructor owns a supplied socket.
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (stream, _) = listener.accept().unwrap();
    peer.set_nonblocking(true).unwrap();
    (TcpFramePort::new(stream).unwrap(), peer)
}
#[test]
fn real_loopback_fragmented_input_output_and_eof() {
    let (mut port, mut peer) = pair();
    assert!(matches!(port.poll_read(), Ok(ReadPoll::Pending)));
    let bytes = wire(&frame());
    for (index, byte) in bytes.iter().enumerate() {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match peer.write(&[*byte]) {
                Ok(1) => break,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    ()
                }
                result => panic!("unexpected write {result:?}"),
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        if index + 1 < bytes.len() {
            assert!(matches!(port.poll_read(), Ok(ReadPoll::Pending)));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match port.poll_read().unwrap() {
            ReadPoll::Frame(value) => {
                assert_eq!(value, frame());
                break;
            }
            ReadPoll::Pending => (),
            result => panic!("unexpected read {result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    port.enqueue(&frame()).unwrap();
    let mut received = Vec::new();
    let mut complete = false;
    while received.len() < bytes.len() || !complete {
        if !complete {
            complete = port.poll_write().unwrap() == WritePoll::Complete;
        }
        let mut short = [0; 3];
        match peer.read(&mut short) {
            Ok(n) if n > 0 => received.extend_from_slice(&short[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                ()
            }
            result => panic!("unexpected read {result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(received, bytes);
    drop(peer);
    loop {
        match port.poll_read().unwrap() {
            ReadPoll::Closed => break,
            ReadPoll::Pending => (),
            result => panic!("{result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(port.stream.is_none());
    assert!(matches!(port.enqueue(&frame()), Err(QueueError::Terminal)));
}
#[test]
fn real_tcp_inbound_remains_readable_while_outbound_is_blocked() {
    let (mut port, mut peer) = pair();
    let mut large = frame();
    let vibe_wire::generated::machine::e1::session_frame::Body::Request(body) = &mut large.body
    else {
        unreachable!()
    };
    body.envelope_json = "x".repeat(MAX_FRAME_BYTES - 200);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut sent = 0;
    let mut blocked = false;
    for _ in 0..(64 * 1024 * 1024 / POLL_BYTES) {
        if port.encoder.bytes.is_empty() {
            port.enqueue(&large).unwrap();
        }
        let before = port.encoder.offset;
        let result = port.poll_write().unwrap();
        if result == WritePoll::Pending && port.encoder.offset == before {
            blocked = true;
            break;
        }
        sent += POLL_BYTES;
        assert!(
            Instant::now() < deadline,
            "send did not block within deadline; {sent} bytes budgeted"
        );
    }
    assert!(
        blocked,
        "socket did not backpressure within bounded send budget"
    );
    assert!(matches!(port.enqueue(&frame()), Err(QueueError::Busy)));
    let bytes = wire(&frame());
    let mut offset = 0;
    while offset < bytes.len() {
        match peer.write(&bytes[offset..]) {
            Ok(n) if n > 0 => offset += n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                ()
            }
            result => panic!("{result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    loop {
        match port.poll_read().unwrap() {
            ReadPoll::Frame(value) => {
                assert_eq!(value, frame());
                break;
            }
            ReadPoll::Pending => (),
            result => panic!("{result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn read_io_error_is_typed_terminal_and_port_malformed_input_closes() {
    struct FailedReader;
    impl Read for FailedReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::ConnectionReset.into())
        }
    }
    let mut decoder = Decoder::new();
    assert!(
        matches!(decoder.poll_read(&mut FailedReader), Err(FrameCodecError::Io(e)) if e.kind() == io::ErrorKind::ConnectionReset)
    );
    assert!(matches!(
        decoder.poll_read(&mut FailedReader),
        Ok(ReadPoll::Terminal)
    ));
    let (mut port, mut peer) = pair();
    let mut position = 0;
    let deadline = Instant::now() + Duration::from_secs(3);
    while position < 4 {
        match peer.write(&[0; 4][position..]) {
            Ok(n) if n > 0 => position += n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                ()
            }
            result => panic!("{result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    loop {
        match port.poll_read() {
            Err(FrameCodecError::ZeroLength) => break,
            Ok(ReadPoll::Pending) => (),
            result => panic!("{result:?}"),
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(port.stream.is_none());
    assert_eq!(port.poll_write().unwrap(), WritePoll::Terminal);
}
