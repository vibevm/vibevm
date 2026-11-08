//! The raw ABI-1 operations carried over the helper channel.
//!
//! The helper performs only open, manifest and invoke on its cached handles;
//! every manifest, request and reply admission stays in the parent loader.

use std::path::PathBuf;

const OPEN: u8 = 1;
const MANIFEST: u8 = 2;
const INVOKE: u8 = 3;
const OPENED: u8 = 0x81;
const MANIFEST_BYTES: u8 = 0x82;
const INVOKED: u8 = 0x83;
const REFUSED: u8 = 0xff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    Open {
        path: PathBuf,
        digest: String,
        bytes: u64,
    },
    Manifest {
        handle: u32,
    },
    Invoke {
        handle: u32,
        request: Vec<u8>,
    },
}

/// Helper-side refusals the parent re-renders with its own display path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    LibraryOpen = 1,
    MissingSymbol = 2,
    ManifestPointerNull = 3,
    ManifestTooLarge = 4,
    ImageChanged = 5,
    Protocol = 6,
}

impl Refusal {
    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::LibraryOpen,
            2 => Self::MissingSymbol,
            3 => Self::ManifestPointerNull,
            4 => Self::ManifestTooLarge,
            5 => Self::ImageChanged,
            6 => Self::Protocol,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reply {
    Opened {
        handle: u32,
        abi: u32,
    },
    Manifest(Vec<u8>),
    Invoked {
        status: i32,
        len: u64,
        has_response: bool,
        /// Copied only for a well-formed successful response inside the cap.
        response: Vec<u8>,
    },
    Refused {
        refusal: Refusal,
        detail: String,
    },
}

impl Request {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        match self {
            Self::Open {
                path,
                digest,
                bytes,
            } => {
                let path = path
                    .to_str()
                    .ok_or_else(|| "helper image path is not valid UTF-8".to_owned())?;
                out.push(OPEN);
                out.extend_from_slice(&bytes.to_le_bytes());
                put_bytes(&mut out, digest.as_bytes())?;
                out.extend_from_slice(path.as_bytes());
            }
            Self::Manifest { handle } => {
                out.push(MANIFEST);
                out.extend_from_slice(&handle.to_le_bytes());
            }
            Self::Invoke { handle, request } => {
                out.push(INVOKE);
                out.extend_from_slice(&handle.to_le_bytes());
                out.extend_from_slice(request);
            }
        }
        Ok(out)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor(bytes);
        Ok(match cursor.u8()? {
            OPEN => {
                let bytes = cursor.u64()?;
                let digest = utf8(cursor.sized()?)?;
                let path = PathBuf::from(utf8(cursor.rest())?);
                Self::Open {
                    path,
                    digest,
                    bytes,
                }
            }
            MANIFEST => Self::Manifest {
                handle: cursor.finish_u32()?,
            },
            INVOKE => Self::Invoke {
                handle: cursor.u32()?,
                request: cursor.rest().to_vec(),
            },
            other => return Err(format!("unknown helper request tag {other}")),
        })
    }
}

impl Reply {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        match self {
            Self::Opened { handle, abi } => {
                out.push(OPENED);
                out.extend_from_slice(&handle.to_le_bytes());
                out.extend_from_slice(&abi.to_le_bytes());
            }
            Self::Manifest(bytes) => {
                out.push(MANIFEST_BYTES);
                out.extend_from_slice(bytes);
            }
            Self::Invoked {
                status,
                len,
                has_response,
                response,
            } => {
                out.push(INVOKED);
                out.extend_from_slice(&status.to_le_bytes());
                out.extend_from_slice(&len.to_le_bytes());
                out.push(u8::from(*has_response));
                out.extend_from_slice(response);
            }
            Self::Refused { refusal, detail } => {
                out.push(REFUSED);
                out.push(*refusal as u8);
                put_bytes(&mut out, detail.as_bytes())?;
            }
        }
        Ok(out)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor(bytes);
        Ok(match cursor.u8()? {
            OPENED => {
                let handle = cursor.u32()?;
                Self::Opened {
                    handle,
                    abi: cursor.finish_u32()?,
                }
            }
            MANIFEST_BYTES => Self::Manifest(cursor.rest().to_vec()),
            INVOKED => {
                let status = i32::from_le_bytes(cursor.array()?);
                let len = cursor.u64()?;
                let has_response = match cursor.u8()? {
                    0 => false,
                    1 => true,
                    other => return Err(format!("invalid response presence {other}")),
                };
                Self::Invoked {
                    status,
                    len,
                    has_response,
                    response: cursor.rest().to_vec(),
                }
            }
            REFUSED => {
                let code = cursor.u8()?;
                let refusal = Refusal::from_code(code)
                    .ok_or_else(|| format!("unknown helper refusal code {code}"))?;
                let detail = utf8(cursor.sized()?)?;
                if !cursor.rest().is_empty() {
                    return Err("trailing bytes after helper refusal".to_owned());
                }
                Self::Refused { refusal, detail }
            }
            other => return Err(format!("unknown helper reply tag {other}")),
        })
    }
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    let length =
        u32::try_from(bytes.len()).map_err(|_| "helper field exceeds u32 length".to_owned())?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn utf8(bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| "helper text field is not UTF-8".to_owned())
}

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        if self.0.len() < count {
            return Err("truncated helper message".to_owned());
        }
        let (head, tail) = self.0.split_at(count);
        self.0 = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?
            .try_into()
            .map_err(|_| "truncated helper message".to_owned())
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.array::<1>()?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn finish_u32(&mut self) -> Result<u32, String> {
        let value = self.u32()?;
        if self.0.is_empty() {
            Ok(value)
        } else {
            Err("trailing bytes after helper message".to_owned())
        }
    }

    fn sized(&mut self) -> Result<&'a [u8], String> {
        let length = usize::try_from(self.u32()?)
            .map_err(|_| "helper field length does not fit usize".to_owned())?;
        self.take(length)
    }

    fn rest(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_round_trips() {
        for request in [
            Request::Open {
                path: PathBuf::from("/abs/lib.so"),
                digest: "ab".repeat(32),
                bytes: 42,
            },
            Request::Manifest { handle: 3 },
            Request::Invoke {
                handle: 1,
                request: b"{}".to_vec(),
            },
        ] {
            assert_eq!(
                Request::decode(&request.encode().unwrap()).unwrap(),
                request
            );
        }
        for reply in [
            Reply::Opened { handle: 0, abi: 1 },
            Reply::Manifest(b"{\"extensions\":[]}".to_vec()),
            Reply::Invoked {
                status: 0,
                len: 2,
                has_response: true,
                response: b"{}".to_vec(),
            },
            Reply::Refused {
                refusal: Refusal::MissingSymbol,
                detail: "vibe_ext_free".to_owned(),
            },
        ] {
            assert_eq!(Reply::decode(&reply.encode().unwrap()).unwrap(), reply);
        }
        assert!(Reply::decode(&[0x7f]).is_err());
    }
}
