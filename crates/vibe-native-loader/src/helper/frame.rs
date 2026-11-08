//! Bounded scratch-file frames of the private helper control channel.
//!
//! Each frame is published by an atomic rename into the session's private
//! scratch directory and consumed (removed) by its reader. Plugin stdout and
//! stderr never share this channel.

use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

/// The largest accepted frame payload: a native request or reply plus framing.
pub const FRAME_CAP: usize = 64 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"VIBENH01";
const HEADER: usize = MAGIC.len() + 1 + SESSION_BYTES + 8 + 4;
/// Bytes of the per-session random identity carried in every frame.
pub const SESSION_BYTES: usize = 16;

/// Which side of the channel published a frame.
///
/// ```
/// use vibe_native_loader::helper::{Direction, publish, take};
/// let dir = tempfile::tempdir().unwrap();
/// let session = [1_u8; vibe_native_loader::helper::SESSION_BYTES];
/// publish(dir.path(), Direction::Request, &session, 1, b"ping").unwrap();
/// assert_eq!(take(dir.path(), Direction::Request, &session, 1).unwrap(), Some(b"ping".to_vec()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Parent to helper.
    Request,
    /// Helper to parent.
    Reply,
}

impl Direction {
    const fn tag(self) -> u8 {
        match self {
            Self::Request => 1,
            Self::Reply => 2,
        }
    }

    const fn stem(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Reply => "reply",
        }
    }
}

/// Parse the lowercase-hex session identity passed on the helper argv.
///
/// ```
/// use vibe_native_loader::helper::parse_session;
/// assert!(parse_session("00112233445566778899aabbccddeeff").is_ok());
/// assert!(parse_session("not-hex").is_err());
/// ```
pub fn parse_session(hex: &str) -> Result<[u8; SESSION_BYTES], String> {
    let bytes = hex.as_bytes();
    if bytes.len() != SESSION_BYTES * 2 {
        return Err(format!(
            "session identity must be {} hex characters",
            SESSION_BYTES * 2
        ));
    }
    let mut session = [0_u8; SESSION_BYTES];
    for (index, [high, low]) in bytes.as_chunks::<2>().0.iter().enumerate() {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            _ => Err("session identity must be lowercase hex".to_owned()),
        };
        session[index] = (digit(*high)? << 4) | digit(*low)?;
    }
    Ok(session)
}

/// Render a session identity in the argv spelling [`parse_session`] reads.
pub fn session_hex(session: &[u8; SESSION_BYTES]) -> String {
    session.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn frame_path(dir: &Path, direction: Direction, sequence: u64) -> PathBuf {
    dir.join(format!("{}-{sequence:016}.frame", direction.stem()))
}

/// Publish one frame atomically; the reader never observes a partial file.
pub fn publish(
    dir: &Path,
    direction: Direction,
    session: &[u8; SESSION_BYTES],
    sequence: u64,
    payload: &[u8],
) -> Result<(), String> {
    if payload.len() > FRAME_CAP {
        return Err(format!(
            "frame payload of {} bytes exceeds the {FRAME_CAP}-byte channel cap",
            payload.len()
        ));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| "frame payload length does not fit the frame header".to_owned())?;
    let target = frame_path(dir, direction, sequence);
    let pending = dir.join(format!(".{}-{sequence:016}.pending", direction.stem()));
    let written = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(MAGIC)?;
        file.write_all(&[direction.tag()])?;
        file.write_all(session)?;
        file.write_all(&sequence.to_le_bytes())?;
        file.write_all(&length.to_le_bytes())?;
        file.write_all(payload)?;
        file.sync_data()?;
        drop(file);
        std::fs::rename(&pending, &target)
    })();
    written.map_err(|error| {
        let _ = std::fs::remove_file(&pending);
        format!("publishing {} frame {sequence}: {error}", direction.stem())
    })
}

/// Consume the frame for `sequence` when it has been published.
pub fn take(
    dir: &Path,
    direction: Direction,
    session: &[u8; SESSION_BYTES],
    sequence: u64,
) -> Result<Option<Vec<u8>>, String> {
    let path = frame_path(dir, direction, sequence);
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("opening {} frame: {error}", direction.stem())),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((HEADER + FRAME_CAP + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading {} frame: {error}", direction.stem()))?;
    drop(file);
    std::fs::remove_file(&path)
        .map_err(|error| format!("consuming {} frame: {error}", direction.stem()))?;
    decode(&bytes, direction, session, sequence).map(Some)
}

fn decode(
    bytes: &[u8],
    direction: Direction,
    session: &[u8; SESSION_BYTES],
    sequence: u64,
) -> Result<Vec<u8>, String> {
    let invalid = |what: &str| format!("malformed {} frame {sequence}: {what}", direction.stem());
    let header = bytes.get(..HEADER).ok_or_else(|| invalid("short header"))?;
    let (magic, rest) = header.split_at(MAGIC.len());
    if magic != MAGIC {
        return Err(invalid("wrong magic"));
    }
    let (tag, rest) = rest.split_at(1);
    if tag[0] != direction.tag() {
        return Err(invalid("wrong direction"));
    }
    let (identity, rest) = rest.split_at(SESSION_BYTES);
    if identity != session {
        return Err(invalid("foreign session"));
    }
    let (number, length) = rest.split_at(8);
    let number: [u8; 8] = number.try_into().map_err(|_| invalid("sequence"))?;
    if u64::from_le_bytes(number) != sequence {
        return Err(invalid("out-of-order sequence"));
    }
    let length: [u8; 4] = length.try_into().map_err(|_| invalid("length"))?;
    let length = usize::try_from(u32::from_le_bytes(length)).map_err(|_| invalid("length"))?;
    if length > FRAME_CAP || bytes.len() != HEADER + length {
        return Err(invalid("length differs from payload"));
    }
    Ok(bytes[HEADER..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip_once_and_refuse_foreign_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let session = [7_u8; SESSION_BYTES];
        publish(dir.path(), Direction::Request, &session, 3, b"payload").unwrap();
        assert_eq!(
            take(dir.path(), Direction::Request, &session, 3).unwrap(),
            Some(b"payload".to_vec())
        );
        assert_eq!(
            take(dir.path(), Direction::Request, &session, 3).unwrap(),
            None
        );
        publish(dir.path(), Direction::Reply, &session, 4, b"x").unwrap();
        let foreign = [8_u8; SESSION_BYTES];
        assert!(
            take(dir.path(), Direction::Reply, &foreign, 4)
                .unwrap_err()
                .contains("foreign session")
        );
        assert_eq!(
            parse_session(&session_hex(&session)).unwrap(),
            session,
            "argv spelling round-trips"
        );
    }
}
