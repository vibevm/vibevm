//! The LSP base-protocol framing cell: `Content-Length` headers, one
//! JSON body per frame, both directions (TCG-PROTOCOL-RUST §1's inner
//! hop). Pure over `Read`/`Write` so the whole cell replays on
//! cursors.

specmark::scope!("spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#parity");

use std::io::{BufRead, Write};

use crate::TcgBridgeError;

/// Write one LSP frame: `Content-Length: N\r\n\r\n<utf-8 json>`.
///
/// ```
/// let mut out = Vec::new();
/// rust_ai_native_tcg_bridge::frame::write_frame(
///     &mut out,
///     &serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "shutdown"}),
/// )
/// .expect("write");
/// let text = String::from_utf8(out).expect("utf8");
/// assert!(text.starts_with("Content-Length: "));
/// assert!(text.contains("\r\n\r\n{"));
/// ```
pub fn write_frame(out: &mut impl Write, value: &serde_json::Value) -> Result<(), TcgBridgeError> {
    let body = value.to_string();
    write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())
        .and_then(|()| out.flush())
        .map_err(|e| TcgBridgeError::OracleCrashed {
            detail: format!("writing a frame: {e}"),
        })
}

/// Read one LSP frame's JSON body. `Ok(None)` is clean EOF (the child
/// exited); a malformed header or truncated body is a protocol error.
///
/// ```
/// use std::io::BufReader;
/// let wire = b"Content-Length: 17\r\n\r\n{\"jsonrpc\":\"2.0\"}";
/// let mut reader = BufReader::new(&wire[..]);
/// let frame = rust_ai_native_tcg_bridge::frame::read_frame(&mut reader)
///     .expect("read")
///     .expect("some");
/// assert_eq!(frame["jsonrpc"], "2.0");
/// assert!(
///     rust_ai_native_tcg_bridge::frame::read_frame(&mut reader)
///         .expect("eof read")
///         .is_none()
/// );
/// ```
pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<serde_json::Value>, TcgBridgeError> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| TcgBridgeError::OracleCrashed {
                detail: format!("reading a frame header: {e}"),
            })?;
        if n == 0 {
            // EOF between frames is a clean end; EOF mid-headers is not.
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(TcgBridgeError::Protocol {
                    detail: "EOF inside frame headers".to_string(),
                })
            };
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break; // the blank line ends the header block
        }
        if let Some(rest) = header_value(trimmed, "Content-Length") {
            content_length = Some(rest.trim().parse().map_err(|_| TcgBridgeError::Protocol {
                detail: format!("unparseable Content-Length: {rest:?}"),
            })?);
        }
        // Content-Type headers are legal and ignored.
    }
    let len = content_length.ok_or_else(|| TcgBridgeError::Protocol {
        detail: "frame headers carried no Content-Length".to_string(),
    })?;
    let mut body = vec![0u8; len];
    std::io::Read::read_exact(reader, &mut body).map_err(|e| TcgBridgeError::Protocol {
        detail: format!("truncated frame body ({len} bytes expected): {e}"),
    })?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|e| TcgBridgeError::Protocol {
            detail: format!("unparseable frame body: {e}"),
        })
}

/// Case-insensitive `Name: value` header match.
fn header_value<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (head, value) = line.split_once(':')?;
    head.trim().eq_ignore_ascii_case(name).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::{read_frame, write_frame};

    #[test]
    fn roundtrips_a_frame() {
        let value = serde_json::json!({"id": 7, "result": {"ok": true}});
        let mut wire = Vec::new();
        write_frame(&mut wire, &value).expect("write");
        let mut reader = BufReader::new(wire.as_slice());
        let back = read_frame(&mut reader).expect("read").expect("some");
        assert_eq!(back, value);
    }

    #[test]
    fn tolerates_extra_headers_and_case() {
        let wire = b"content-length: 2\r\nContent-Type: application/vscode-jsonrpc\r\n\r\n{}";
        let mut reader = BufReader::new(&wire[..]);
        let frame = read_frame(&mut reader).expect("read").expect("some");
        assert_eq!(frame, serde_json::json!({}));
    }

    #[test]
    fn truncated_body_is_a_protocol_error() {
        let wire = b"Content-Length: 10\r\n\r\n{}";
        let mut reader = BufReader::new(&wire[..]);
        let err = read_frame(&mut reader).expect_err("truncated");
        assert_eq!(err.wire_kind(), "protocol");
    }

    #[test]
    fn missing_length_is_a_protocol_error() {
        let wire = b"Content-Type: x\r\n\r\n{}";
        let mut reader = BufReader::new(&wire[..]);
        let err = read_frame(&mut reader).expect_err("no length");
        assert_eq!(err.wire_kind(), "protocol");
    }
}
