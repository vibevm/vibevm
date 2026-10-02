//! Bounded four-byte length prefix and strict generated frame reader.
//! This codec does not dispatch operations or grant filesystem authority.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use std::io::{self, Read, Write};

use vibe_wire::generated::machine::e1::session_frame::SessionFrame;

pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const FRAME_EPOCH: u8 = 1;

#[derive(Debug)]
pub enum FrameCodecError {
    TruncatedPrefix,
    TruncatedBody,
    ZeroLength,
    OverLimit { declared: usize },
    InvalidUtf8,
    InvalidJson,
    InvalidShape,
    UnsupportedEpoch(u8),
    Serialization,
    Io(io::Error),
}

fn read_exact_or<R: Read>(
    reader: &mut R,
    bytes: &mut [u8],
    truncated: FrameCodecError,
) -> Result<(), FrameCodecError> {
    reader.read_exact(bytes).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            truncated
        } else {
            FrameCodecError::Io(error)
        }
    })
}

/// EOF before any prefix byte closes the on-demand session. Every other
/// incomplete frame refuses; the declared bound is checked before allocation.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<SessionFrame>, FrameCodecError> {
    let mut prefix = [0_u8; 4];
    loop {
        match reader.read(&mut prefix[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(FrameCodecError::Io(error)),
        }
    }
    read_exact_or(reader, &mut prefix[1..], FrameCodecError::TruncatedPrefix)?;
    let declared = u32::from_be_bytes(prefix) as usize;
    if declared == 0 {
        return Err(FrameCodecError::ZeroLength);
    }
    if declared > MAX_FRAME_BYTES {
        return Err(FrameCodecError::OverLimit { declared });
    }
    let mut body = vec![0_u8; declared];
    read_exact_or(reader, &mut body, FrameCodecError::TruncatedBody)?;
    decode_body(&body).map(Some)
}

pub(super) fn decode_body(body: &[u8]) -> Result<SessionFrame, FrameCodecError> {
    let text = std::str::from_utf8(body).map_err(|_| FrameCodecError::InvalidUtf8)?;
    let frame: SessionFrame = serde_json::from_str(text).map_err(|error| {
        if error.is_data() {
            FrameCodecError::InvalidShape
        } else {
            FrameCodecError::InvalidJson
        }
    })?;
    if frame.schema != FRAME_EPOCH {
        return Err(FrameCodecError::UnsupportedEpoch(frame.schema));
    }
    Ok(frame)
}

/// Serialize the generated value, then emit one bounded frame and flush it.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
pub fn write_frame<W: Write>(writer: &mut W, frame: &SessionFrame) -> Result<(), FrameCodecError> {
    if frame.schema != FRAME_EPOCH {
        return Err(FrameCodecError::UnsupportedEpoch(frame.schema));
    }
    let body = serde_json::to_vec(frame).map_err(|_| FrameCodecError::Serialization)?;
    if body.is_empty() {
        return Err(FrameCodecError::ZeroLength);
    }
    if body.len() > MAX_FRAME_BYTES {
        return Err(FrameCodecError::OverLimit {
            declared: body.len(),
        });
    }
    let length = u32::try_from(body.len()).expect("eight MiB fits in u32");
    writer
        .write_all(&length.to_be_bytes())
        .map_err(FrameCodecError::Io)?;
    writer.write_all(&body).map_err(FrameCodecError::Io)?;
    writer.flush().map_err(FrameCodecError::Io)
}

#[cfg(test)]
mod tests;
