//! Operation-owned helper transport for the unchanged synchronous ABI-1 call.
//!
//! A synchronous FFI call cannot be interrupted in-process. A controlled
//! operation therefore loads and invokes its native images in one helper
//! process while this crate's loader keeps every manifest, request and reply
//! admission in the parent: the helper performs only open, manifest and raw
//! invoke. The parent-side [`HelperChannel`] owns the process (spawn, stop,
//! kill and reap); this module owns the frames, the messages and the helper's
//! [`serve`] loop.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

mod frame;
mod serve;
mod wire;

pub use frame::{Direction, FRAME_CAP, SESSION_BYTES, parse_session, publish, session_hex, take};
pub use serve::{serve, serve_until};

use crate::NativeLoadError;
use crate::ffi::{CallResult, LibraryHandle, LibraryOpener, PublishedResponse, REQUIRED_SYMBOLS};
use wire::{Refusal, Reply, Request};

/// The hidden argv mode word the composition owner re-executes itself with.
///
/// The helper argv is exactly `<trusted program> HELPER_MODE_ARG <scratch>
/// <session>`; nothing in it comes from plugin metadata or ambient lookup.
pub const HELPER_MODE_ARG: &str = "--__vibe-native-helper";

/// Why one helper exchange did not produce a reply.
///
/// ```
/// use vibe_native_loader::helper::HelperFault;
/// let fault = HelperFault::Stopped("operation stop acknowledged".into());
/// assert_ne!(fault, HelperFault::StoppedBeforeCall);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperFault {
    /// The stop was acknowledged before the request was sent.
    StoppedBeforeCall,
    /// The stop was acknowledged mid-call; the helper tree was killed and reaped.
    Stopped(String),
    /// The channel or helper process failed.
    Transport(String),
}

/// The parent's private channel to one operation's helper process.
///
/// Implementations own the process: they spawn it lazily, observe the
/// operation stop while waiting, and kill and reap the whole helper tree on
/// an acknowledged stop. One channel serves every call of one operation.
///
/// ```
/// use std::sync::Arc;
/// use vibe_native_loader::NativeLoader;
/// use vibe_native_loader::helper::{HelperChannel, HelperFault};
///
/// struct Stopped;
/// impl HelperChannel for Stopped {
///     fn exchange(&self, _: Vec<u8>) -> Result<Vec<u8>, HelperFault> {
///         Err(HelperFault::StoppedBeforeCall)
///     }
/// }
/// let loader = NativeLoader::with_helper(Arc::new(Stopped));
/// drop(loader);
/// ```
pub trait HelperChannel: Send + Sync {
    /// Carry one encoded request payload and return the helper's reply payload.
    fn exchange(&self, request: Vec<u8>) -> Result<Vec<u8>, HelperFault>;
}

impl crate::NativeLoader {
    /// A loader whose images are opened and invoked through `channel`.
    ///
    /// Admission is the same code as the in-process loader; every image must
    /// first be recorded with [`Self::expect_image`].
    #[must_use]
    pub fn with_helper(channel: Arc<dyn HelperChannel>) -> Self {
        Self::with_opener(Arc::new(RemoteOpener {
            channel,
            admitted: Mutex::new(HashMap::new()),
        }))
    }

    /// Record the parent-verified digest and length of one image.
    ///
    /// The in-process loader ignores the record (its caller has just
    /// verified the path); a helper loader refuses to open an unrecorded
    /// image, and its helper re-validates these bytes before loading.
    pub fn expect_image(
        &self,
        library: &Path,
        digest: &str,
        bytes: u64,
    ) -> Result<(), NativeLoadError> {
        let (canonical, _) = crate::validate_path(library)?;
        self.opener.admit_image(&canonical, digest, bytes)
    }
}

struct RemoteOpener {
    channel: Arc<dyn HelperChannel>,
    admitted: Mutex<HashMap<PathBuf, (String, u64)>>,
}

impl LibraryOpener for RemoteOpener {
    fn open(
        &self,
        canonical_path: &Path,
        display_path: &str,
    ) -> Result<Arc<dyn LibraryHandle>, NativeLoadError> {
        let (digest, bytes) = self
            .admitted
            .lock()
            .map_err(|_| NativeLoadError::CachePoisoned)?
            .get(canonical_path)
            .cloned()
            .ok_or_else(|| NativeLoadError::HelperImageUnadmitted {
                path: display_path.to_owned(),
            })?;
        let reply = exchange(
            self.channel.as_ref(),
            &Request::Open {
                path: canonical_path.to_owned(),
                digest,
                bytes,
            },
        )?;
        match reply {
            Reply::Opened { handle, abi } => Ok(Arc::new(RemoteLibrary {
                channel: Arc::clone(&self.channel),
                handle,
                abi,
            })),
            other => Err(unexpected(other, display_path)),
        }
    }

    fn admit_image(
        &self,
        canonical_path: &Path,
        digest: &str,
        bytes: u64,
    ) -> Result<(), NativeLoadError> {
        let mut admitted = self
            .admitted
            .lock()
            .map_err(|_| NativeLoadError::CachePoisoned)?;
        match admitted.get(canonical_path) {
            Some((known, length)) if known != digest || *length != bytes => {
                Err(NativeLoadError::HelperImageChanged {
                    path: crate::path_preview(canonical_path),
                    reason: "one load-image path was admitted with two digests".to_owned(),
                })
            }
            Some(_) => Ok(()),
            None => {
                admitted.insert(canonical_path.to_owned(), (digest.to_owned(), bytes));
                Ok(())
            }
        }
    }
}

struct RemoteLibrary {
    channel: Arc<dyn HelperChannel>,
    handle: u32,
    abi: u32,
}

impl LibraryHandle for RemoteLibrary {
    fn abi(&self) -> u32 {
        self.abi
    }

    fn manifest_bytes(&self, path: &str) -> Result<Vec<u8>, NativeLoadError> {
        match exchange(
            self.channel.as_ref(),
            &Request::Manifest {
                handle: self.handle,
            },
        )? {
            Reply::Manifest(bytes) => Ok(bytes),
            other => Err(unexpected(other, path)),
        }
    }

    fn invoke(
        &self,
        request: &[u8],
        _owner: Arc<dyn LibraryHandle>,
    ) -> Result<CallResult, NativeLoadError> {
        let reply = exchange(
            self.channel.as_ref(),
            &Request::Invoke {
                handle: self.handle,
                request: request.to_vec(),
            },
        )?;
        let Reply::Invoked {
            status,
            len,
            has_response,
            response,
        } = reply
        else {
            return Err(unexpected(reply, "<helper invocation>"));
        };
        let len = usize::try_from(len).map_err(|_| transport("reply length exceeds usize"))?;
        let copied = status == 0 && has_response && len > 0 && len <= crate::REPLY_CAP;
        if (copied && response.len() != len) || (!copied && !response.is_empty()) {
            return Err(transport(
                "helper reply bytes differ from the reported length",
            ));
        }
        Ok(CallResult {
            status,
            len,
            response: has_response.then(|| PublishedResponse::owned(response)),
        })
    }
}

fn exchange(channel: &dyn HelperChannel, request: &Request) -> Result<Reply, NativeLoadError> {
    let encoded = request.encode().map_err(|reason| transport(&reason))?;
    let reply = channel.exchange(encoded).map_err(|fault| match fault {
        HelperFault::StoppedBeforeCall => NativeLoadError::HelperStoppedBeforeCall,
        HelperFault::Stopped(reason) => NativeLoadError::HelperCancelled { reason },
        HelperFault::Transport(reason) => NativeLoadError::HelperTransport { reason },
    })?;
    Reply::decode(&reply).map_err(|reason| transport(&reason))
}

fn unexpected(reply: Reply, path: &str) -> NativeLoadError {
    let path = path.to_owned();
    match reply {
        Reply::Refused { refusal, detail } => match refusal {
            Refusal::LibraryOpen => NativeLoadError::LibraryOpen { path },
            Refusal::MissingSymbol => REQUIRED_SYMBOLS
                .iter()
                .copied()
                .find(|symbol| *symbol == detail)
                .map_or_else(
                    || transport(&format!("helper named unknown symbol `{detail}`")),
                    |symbol| NativeLoadError::MissingSymbol { path, symbol },
                ),
            Refusal::ManifestPointerNull => NativeLoadError::ManifestPointerNull { path },
            Refusal::ManifestTooLarge => NativeLoadError::ManifestTooLarge {
                path,
                cap: crate::MANIFEST_CAP,
            },
            Refusal::ImageChanged => NativeLoadError::HelperImageChanged {
                path,
                reason: detail,
            },
            Refusal::Protocol => transport(&format!("helper refused `{path}`: {detail}")),
        },
        _ => transport("helper answered with a reply of the wrong kind"),
    }
}

fn transport(reason: &str) -> NativeLoadError {
    NativeLoadError::HelperTransport {
        reason: reason.to_owned(),
    }
}
