use super::*;
use std::io::Cursor;

fn request() -> SessionFrame {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-session-frame/e1/valid/request.json"
    )))
    .unwrap()
}
fn initialized() -> SessionFrame {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-session-frame/e1/valid/initialized.json"
    )))
    .unwrap()
}
fn framed(body: &[u8]) -> Vec<u8> {
    let mut frame = (u32::try_from(body.len()).unwrap()).to_be_bytes().to_vec();
    frame.extend_from_slice(body);
    frame
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn two_frames_roundtrip_exact_bytes_and_eof() {
    let mut output = Vec::new();
    write_frame(&mut output, &request()).unwrap();
    write_frame(&mut output, &initialized()).unwrap();
    let first_len = u32::from_be_bytes(output[..4].try_into().unwrap()) as usize;
    assert_eq!(first_len, serde_json::to_vec(&request()).unwrap().len());
    let mut reader = Cursor::new(output);
    assert_eq!(read_frame(&mut reader).unwrap(), Some(request()));
    assert_eq!(read_frame(&mut reader).unwrap(), Some(initialized()));
    assert_eq!(read_frame(&mut reader).unwrap(), None);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn zero_partial_and_over_limit_refuse_before_body_allocation() {
    assert!(matches!(read_frame(&mut Cursor::new([0_u8; 0])), Ok(None)));
    assert!(matches!(
        read_frame(&mut Cursor::new([0, 0, 1])),
        Err(FrameCodecError::TruncatedPrefix)
    ));
    assert!(matches!(
        read_frame(&mut Cursor::new([0, 0, 0, 0])),
        Err(FrameCodecError::ZeroLength)
    ));
    assert!(matches!(
        read_frame(&mut Cursor::new(framed(b"{}")[..5].to_vec())),
        Err(FrameCodecError::TruncatedBody)
    ));
    let too_long = u32::try_from(MAX_FRAME_BYTES + 1).unwrap().to_be_bytes();
    assert!(matches!(
        read_frame(&mut Cursor::new(too_long)),
        Err(FrameCodecError::OverLimit { declared }) if declared == MAX_FRAME_BYTES + 1
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn utf8_json_shape_and_epoch_refuse_before_dispatch() {
    assert!(matches!(
        read_frame(&mut Cursor::new(framed(&[0xff]))),
        Err(FrameCodecError::InvalidUtf8)
    ));
    assert!(matches!(
        read_frame(&mut Cursor::new(framed(b"{"))),
        Err(FrameCodecError::InvalidJson)
    ));
    let mut unknown = serde_json::to_value(request()).unwrap();
    unknown["body"]["path"] = serde_json::json!("/tmp/forbidden");
    assert!(matches!(
        read_frame(&mut Cursor::new(framed(
            &serde_json::to_vec(&unknown).unwrap()
        ))),
        Err(FrameCodecError::InvalidShape)
    ));
    let mut wrong_epoch = request();
    wrong_epoch.schema = 2;
    assert!(matches!(
        read_frame(&mut Cursor::new(framed(
            &serde_json::to_vec(&wrong_epoch).unwrap()
        ))),
        Err(FrameCodecError::UnsupportedEpoch(2))
    ));
}

#[derive(Default)]
struct FlushWriter {
    bytes: Vec<u8>,
    flushes: usize,
}
impl Write for FlushWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn writer_flushes_and_rejects_unsupported_or_oversized_values() {
    let mut writer = FlushWriter::default();
    write_frame(&mut writer, &request()).unwrap();
    assert_eq!(writer.flushes, 1);
    let original = writer.bytes.clone();
    let mut unsupported = request();
    unsupported.schema = 2;
    assert!(matches!(
        write_frame(&mut writer, &unsupported),
        Err(FrameCodecError::UnsupportedEpoch(2))
    ));
    let mut oversized = request();
    let vibe_wire::generated::machine::e1::session_frame::Body::Request(body) = &mut oversized.body
    else {
        panic!("request fixture changed arm")
    };
    body.envelope_json = "x".repeat(MAX_FRAME_BYTES);
    assert!(matches!(
        write_frame(&mut writer, &oversized),
        Err(FrameCodecError::OverLimit { .. })
    ));
    assert_eq!(writer.bytes, original);
    assert_eq!(writer.flushes, 1);
}
