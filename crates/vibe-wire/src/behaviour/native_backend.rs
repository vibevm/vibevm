//! Strict admission for compiler-backend byte replies.

use std::fmt;

use crate::generated::native::e1::backend_reply;

pub const ENVELOPE_EPOCH: u32 = 1;
pub const DIAGNOSTIC_CAP_BYTES: usize = 8 * 1024;
pub const DECODED_CAP_BYTES: usize = 16 * 1024 * 1024;
pub const REPLY_CAP_BYTES: usize = 24 * 1024 * 1024;
const ENCODED_CAP_BYTES: usize = DECODED_CAP_BYTES.div_ceil(3) * 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmittedBackendReply {
    Ok {
        bytes: Vec<u8>,
        message: Option<String>,
    },
    Fail {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendReplyError {
    ReplyCap,
    Json,
    Envelope,
    Base64,
    DecodedCap,
    Message,
}

impl fmt::Display for BackendReplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReplyCap => "native backend reply exceeds its byte cap",
            Self::Json => "native backend reply is not strict ABI-1 JSON",
            Self::Envelope => "native backend reply envelope is not 1",
            Self::Base64 => "native backend bytes are not canonical padded standard base64",
            Self::DecodedCap => "native backend decoded bytes exceed their cap",
            Self::Message => "native backend reply message violates its bounded text law",
        })
    }
}

impl std::error::Error for BackendReplyError {}

pub fn decode_reply(raw: &[u8]) -> Result<AdmittedBackendReply, BackendReplyError> {
    if raw.len() > REPLY_CAP_BYTES {
        return Err(BackendReplyError::ReplyCap);
    }
    let reply = serde_json::from_slice(raw).map_err(|_| BackendReplyError::Json)?;
    validate_reply(&reply)
}

pub fn validate_reply(
    reply: &backend_reply::BackendReply,
) -> Result<AdmittedBackendReply, BackendReplyError> {
    match reply {
        backend_reply::BackendReply::Ok(value) => {
            envelope(value.envelope)?;
            message(value.message.as_deref(), false)?;
            let bytes = decode_base64(&value.bytes_b64)?;
            Ok(AdmittedBackendReply::Ok {
                bytes,
                message: value.message.clone(),
            })
        }
        backend_reply::BackendReply::Fail(value) => {
            envelope(value.envelope)?;
            message(Some(&value.message), true)?;
            Ok(AdmittedBackendReply::Fail {
                message: value.message.clone(),
            })
        }
    }
}

pub fn ok_reply(
    bytes: &[u8],
    message_value: Option<String>,
) -> Result<backend_reply::BackendReply, BackendReplyError> {
    if bytes.len() > DECODED_CAP_BYTES {
        return Err(BackendReplyError::DecodedCap);
    }
    message(message_value.as_deref(), false)?;
    Ok(backend_reply::BackendReply::Ok(Box::new(
        backend_reply::BackendReplyOk {
            envelope: ENVELOPE_EPOCH,
            bytes_b64: encode_base64(bytes),
            message: message_value,
        },
    )))
}

pub fn fail_reply(message_value: String) -> Result<backend_reply::BackendReply, BackendReplyError> {
    message(Some(&message_value), true)?;
    Ok(backend_reply::BackendReply::Fail(Box::new(
        backend_reply::BackendReplyFail {
            envelope: ENVELOPE_EPOCH,
            message: message_value,
        },
    )))
}

fn envelope(value: u32) -> Result<(), BackendReplyError> {
    if value == ENVELOPE_EPOCH {
        Ok(())
    } else {
        Err(BackendReplyError::Envelope)
    }
}

fn message(value: Option<&str>, nonblank: bool) -> Result<(), BackendReplyError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.len() > DIAGNOSTIC_CAP_BYTES
        || value.chars().any(char::is_control)
        || (nonblank && value.trim().is_empty())
    {
        return Err(BackendReplyError::Message);
    }
    Ok(())
}

fn decode_base64(value: &str) -> Result<Vec<u8>, BackendReplyError> {
    let bytes = value.as_bytes();
    if bytes.len() > ENCODED_CAP_BYTES {
        return Err(BackendReplyError::DecodedCap);
    }
    if !bytes.len().is_multiple_of(4) {
        return Err(BackendReplyError::Base64);
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return Err(BackendReplyError::Base64);
    }
    let decoded_len = bytes.len() / 4 * 3 - padding;
    if decoded_len > DECODED_CAP_BYTES {
        return Err(BackendReplyError::DecodedCap);
    }
    let mut output = Vec::with_capacity(decoded_len);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let a = digit(chunk[0])?;
        let b = digit(chunk[1])?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            digit(chunk[2])?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            digit(chunk[3])?
        };
        let chunk_padding = usize::from(chunk[3] == b'=') + usize::from(chunk[2] == b'=');
        if (!last && chunk_padding != 0)
            || (chunk[2] == b'=' && chunk[3] != b'=')
            || (chunk_padding == 2 && b & 0x0f != 0)
            || (chunk_padding == 1 && c & 0x03 != 0)
        {
            return Err(BackendReplyError::Base64);
        }
        output.push((a << 2) | (b >> 4));
        if chunk_padding < 2 {
            output.push((b << 4) | (c >> 2));
        }
        if chunk_padding == 0 {
            output.push((c << 6) | d);
        }
    }
    output.truncate(decoded_len);
    Ok(output)
}

fn digit(value: u8) -> Result<u8, BackendReplyError> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(value - b'a' + 26),
        b'0'..=b'9' => Ok(value - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(BackendReplyError::Base64),
    }
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}
