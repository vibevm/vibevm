//! The helper-process side: raw ABI-1 operations on images it re-validates.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::frame::{self, Direction, SESSION_BYTES};
use super::wire::{Refusal, Reply, Request};
use crate::NativeLoadError;
use crate::ffi::{LibraryHandle, LibraryOpener, SystemOpener};

const IDLE_MIN: Duration = Duration::from_micros(200);
const IDLE_MAX: Duration = Duration::from_millis(20);

/// Serve one helper session until the parent closes the liveness pipe on
/// this process's stdin.
///
/// The composition owner calls this from its hidden helper entry before any
/// ordinary CLI boot, settings or journal initialization. Frames live in the
/// private `scratch` directory; the helper never writes to stdout, and plugin
/// stdout and stderr stay the inherited process streams.
///
/// ```
/// let scratch = tempfile::tempdir().unwrap();
/// let refused = vibe_native_loader::helper::serve(scratch.path(), "short");
/// assert!(refused.is_err());
/// ```
pub fn serve(scratch: &Path, session: &str) -> Result<(), String> {
    let orphaned = Arc::new(AtomicBool::new(false));
    let watch = Arc::clone(&orphaned);
    std::thread::Builder::new()
        .name("vibe-native-helper-liveness".to_owned())
        .spawn(move || {
            let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
            watch.store(true, Ordering::SeqCst);
        })
        .map_err(|error| format!("starting the helper liveness watch: {error}"))?;
    serve_until(scratch, session, &orphaned)
}

/// The session loop of [`serve`] with an explicit liveness flag: it returns
/// once `closed` is set and no request is pending.
pub fn serve_until(scratch: &Path, session: &str, closed: &AtomicBool) -> Result<(), String> {
    let session = frame::parse_session(session)?;
    if !scratch.is_absolute() || !scratch.is_dir() {
        return Err("helper scratch must be an existing absolute directory".to_owned());
    }
    Session {
        scratch,
        session,
        handles: Vec::new(),
    }
    .run(closed)
}

struct Session<'a> {
    scratch: &'a Path,
    session: [u8; SESSION_BYTES],
    handles: Vec<Arc<dyn LibraryHandle>>,
}

impl Session<'_> {
    fn run(mut self, orphaned: &AtomicBool) -> Result<(), String> {
        let mut sequence = 1_u64;
        let mut pause = IDLE_MIN;
        loop {
            match frame::take(self.scratch, Direction::Request, &self.session, sequence)? {
                Some(payload) => {
                    let reply = match Request::decode(&payload) {
                        Ok(request) => self.handle(request),
                        Err(detail) => Reply::Refused {
                            refusal: Refusal::Protocol,
                            detail,
                        },
                    };
                    frame::publish(
                        self.scratch,
                        Direction::Reply,
                        &self.session,
                        sequence,
                        &reply.encode()?,
                    )?;
                    sequence += 1;
                    pause = IDLE_MIN;
                }
                None if orphaned.load(Ordering::SeqCst) => return Ok(()),
                None => {
                    std::thread::sleep(pause);
                    pause = (pause * 2).min(IDLE_MAX);
                }
            }
        }
    }

    fn handle(&mut self, request: Request) -> Reply {
        match request {
            Request::Open {
                path,
                digest,
                bytes,
            } => self.open(&path, &digest, bytes),
            Request::Manifest { handle } => match self.library(handle) {
                Ok(library) => match library.manifest_bytes(&display(handle)) {
                    Ok(bytes) => Reply::Manifest(bytes),
                    Err(error) => refusal(&error),
                },
                Err(reply) => reply,
            },
            Request::Invoke { handle, request } => match self.library(handle) {
                Ok(library) => match library.invoke(&request, Arc::clone(&library)) {
                    Ok(call) => {
                        let copy = call.status == 0
                            && call.len > 0
                            && call.len <= crate::REPLY_CAP
                            && call.len <= isize::MAX as usize;
                        let response = match (&call.response, copy) {
                            (Some(response), true) => response.bytes().to_vec(),
                            _ => Vec::new(),
                        };
                        Reply::Invoked {
                            status: call.status,
                            len: call.len as u64,
                            has_response: call.response.is_some(),
                            response,
                        }
                    }
                    Err(error) => refusal(&error),
                },
                Err(reply) => reply,
            },
        }
    }

    fn open(&mut self, path: &Path, digest: &str, bytes: u64) -> Reply {
        if !path.is_absolute() {
            return protocol("helper open requires an absolute image path".to_owned());
        }
        if let Err(detail) = revalidate(path, digest, bytes) {
            return Reply::Refused {
                refusal: Refusal::ImageChanged,
                detail,
            };
        }
        let Ok(handle) = u32::try_from(self.handles.len()) else {
            return protocol("helper handle table is full".to_owned());
        };
        match SystemOpener.open(path, &display(handle)) {
            Ok(library) => {
                let abi = library.abi();
                self.handles.push(library);
                Reply::Opened { handle, abi }
            }
            Err(error) => refusal(&error),
        }
    }

    fn library(&self, handle: u32) -> Result<Arc<dyn LibraryHandle>, Reply> {
        usize::try_from(handle)
            .ok()
            .and_then(|index| self.handles.get(index))
            .map(Arc::clone)
            .ok_or_else(|| protocol(format!("unknown helper handle {handle}")))
    }
}

fn revalidate(path: &Path, digest: &str, bytes: u64) -> Result<(), String> {
    let content = std::fs::read(path).map_err(|error| format!("reading image: {error}"))?;
    let actual = format!("{:x}", Sha256::digest(&content));
    if content.len() as u64 != bytes || actual != digest {
        return Err("image digest or length differs from the parent admission".to_owned());
    }
    Ok(())
}

fn display(handle: u32) -> String {
    format!("<helper handle {handle}>")
}

fn protocol(detail: String) -> Reply {
    Reply::Refused {
        refusal: Refusal::Protocol,
        detail,
    }
}

fn refusal(error: &NativeLoadError) -> Reply {
    match error {
        NativeLoadError::LibraryOpen { .. } => Reply::Refused {
            refusal: Refusal::LibraryOpen,
            detail: String::new(),
        },
        NativeLoadError::MissingSymbol { symbol, .. } => Reply::Refused {
            refusal: Refusal::MissingSymbol,
            detail: (*symbol).to_owned(),
        },
        NativeLoadError::ManifestPointerNull { .. } => Reply::Refused {
            refusal: Refusal::ManifestPointerNull,
            detail: String::new(),
        },
        NativeLoadError::ManifestTooLarge { .. } => Reply::Refused {
            refusal: Refusal::ManifestTooLarge,
            detail: String::new(),
        },
        other => protocol(other.to_string()),
    }
}
