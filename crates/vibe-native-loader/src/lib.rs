#![deny(unsafe_code)]
//! Cached safe host-side invocation for VibeVM native extension ABI 1.
//!
//! The loader accepts an already-resolved absolute library path, admits the
//! exact manifest row, and returns only generated wire values. Artifact
//! resolution, building, lifecycle dispatch, and compiler interpretation are
//! intentionally outside this crate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#C-ABI-LAW");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_wire::generated::native::e1::context::Context;
use vibe_wire::generated::native::e1::reply::Reply;

mod admission;
mod error;
#[allow(unsafe_code)]
mod ffi;

pub use error::NativeLoadError;

use ffi::{LibraryHandle, LibraryOpener};

const SCALAR_PREVIEW_CHARS: usize = 96;
const PATH_PREVIEW_CHARS: usize = 180;
const MANIFEST_CAP: usize = ffi::MANIFEST_CAP;
/// The maximum accepted reply allocation is 16 MiB.
const REPLY_CAP: usize = 16 * 1024 * 1024;

/// One explicit native extension invocation.
///
/// The library path must be absolute. This lifecycle call admits phase and slot
/// points; compiler points use [`NativeCompileInvocation`]. Point and optional
/// IR schema expectations are compared exactly with the selected generated
/// manifest row before the borrowed generated context can reach the plugin.
///
/// ```
/// use vibe_native_loader::NativeInvocation;
///
/// fn accepts_invocation(_: Option<NativeInvocation<'_>>) {}
/// accepts_invocation(None);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct NativeInvocation<'a> {
    /// Absolute path of the already-resolved platform library artifact.
    pub library: &'a Path,
    /// Exact manifest extension id selected by the caller.
    pub extension_id: &'a str,
    /// Typed lifecycle phase or package-slot point expected from the manifest.
    ///
    /// The aggregate type remains source-compatible, but compile-family calls
    /// must use [`NativeCompileInvocation`].
    pub point: ExtensionPoint,
    /// Exact optional IR schema expectation; absence is significant.
    pub ir_schema: Option<u32>,
    /// Generated epoch-1 native context borrowed for this call.
    pub context: &'a Context,
}

/// One compile-specific native invocation over already encoded request bytes.
///
/// The compiler point is closed and typed. Manifest admission always requires
/// IR schema 1 internally, so callers cannot omit or downgrade that epoch.
/// The returned bytes are owned and are not decoded by this loader.
///
/// ```
/// use std::path::Path;
/// use vibe_core::lifecycle::CompilePoint;
/// use vibe_native_loader::NativeCompileInvocation;
///
/// let request = br#"{\"already\":\"encoded\"}"#;
/// let invocation = NativeCompileInvocation {
///     library: Path::new("/already/resolved/plugin"),
///     extension_id: "minify",
///     point: CompilePoint::Pass,
///     request,
/// };
/// assert_eq!(invocation.point, CompilePoint::Pass);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct NativeCompileInvocation<'a> {
    /// Absolute path of the already-resolved platform library artifact.
    pub library: &'a Path,
    /// Exact manifest extension id selected by the caller.
    pub extension_id: &'a str,
    /// Typed compiler point expected from the manifest.
    pub point: CompilePoint,
    /// Already encoded compiler-native request bytes borrowed for this call.
    pub request: &'a [u8],
}

/// A strong-handle cache and safe invoker for native ABI 1 libraries.
///
/// Keep one loader for the process lifetime when native extensions may be
/// called repeatedly. Each canonical library remains loaded until this value is
/// dropped, and concurrent first use is serialized into one open operation.
///
/// ```
/// use vibe_native_loader::NativeLoader;
///
/// let loader = NativeLoader::new();
/// drop(loader);
/// ```
pub struct NativeLoader {
    cache: Mutex<HashMap<PathBuf, Arc<dyn LibraryHandle>>>,
    opener: Arc<dyn LibraryOpener>,
}

impl Default for NativeLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeLoader {
    /// Create an empty process-lifetime loader cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            opener: Arc::new(ffi::SystemOpener),
        }
    }

    /// Admit and invoke one native extension, returning its generated reply.
    ///
    /// The generated context is serialized once after ABI and manifest
    /// admission. Any plugin-owned non-null response is freed exactly once on
    /// every return path.
    pub fn invoke(&self, invocation: NativeInvocation<'_>) -> Result<Reply, NativeLoadError> {
        let (library, display_path) = self.admit_library(
            invocation.library,
            invocation.extension_id,
            invocation.point,
            invocation.ir_schema,
            admission::ManifestFamily::Lifecycle,
        )?;
        let request = serde_json::to_vec(invocation.context).map_err(|error| {
            NativeLoadError::ContextSerialization {
                reason: format!("JSON at line {}, column {}", error.line(), error.column()),
            }
        })?;
        let response = invoke_admitted(&library, &request, &display_path)?;
        admission::parse_reply(response.bytes(), &display_path)
    }

    /// Admit and invoke one compiler extension, returning exact owned bytes.
    ///
    /// The request is passed through without decoding. Manifest admission fixes
    /// the expected schema to 1, and any plugin-owned response is copied while
    /// its exact-once free guard remains live.
    pub fn invoke_compile(
        &self,
        invocation: NativeCompileInvocation<'_>,
    ) -> Result<Vec<u8>, NativeLoadError> {
        let (library, display_path) = self.admit_library(
            invocation.library,
            invocation.extension_id,
            ExtensionPoint::Compile(invocation.point),
            Some(1),
            admission::ManifestFamily::Compiler,
        )?;
        let response = invoke_admitted(&library, invocation.request, &display_path)?;
        Ok(response.bytes().to_vec())
    }

    fn admit_library(
        &self,
        path: &Path,
        extension_id: &str,
        point: ExtensionPoint,
        ir_schema: Option<u32>,
        family: admission::ManifestFamily,
    ) -> Result<(Arc<dyn LibraryHandle>, String), NativeLoadError> {
        let (canonical, display_path) = validate_path(path)?;
        let library = self.library(&canonical, &display_path)?;
        let manifest = library.manifest_bytes(&display_path)?;
        admission::select_manifest(
            &manifest,
            extension_id,
            point,
            ir_schema,
            family,
            &display_path,
        )?;
        Ok((library, display_path))
    }

    fn library(
        &self,
        canonical: &Path,
        display_path: &str,
    ) -> Result<Arc<dyn LibraryHandle>, NativeLoadError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| NativeLoadError::CachePoisoned)?;
        if let Some(library) = cache.get(canonical) {
            return Ok(Arc::clone(library));
        }
        let library = self.opener.open(canonical, display_path)?;
        let actual = library.abi();
        if actual != 1 {
            return Err(NativeLoadError::AbiMismatch {
                path: display_path.to_owned(),
                actual,
            });
        }
        cache.insert(canonical.to_owned(), Arc::clone(&library));
        Ok(library)
    }

    #[cfg(test)]
    fn with_opener(opener: Arc<dyn LibraryOpener>) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            opener,
        }
    }
}

fn validate_path(path: &Path) -> Result<(PathBuf, String), NativeLoadError> {
    let display = path_preview(path);
    if !path.is_absolute() {
        return Err(NativeLoadError::PathNotAbsolute { path: display });
    }
    let metadata = std::fs::metadata(path).map_err(|error| NativeLoadError::PathUnavailable {
        path: display.clone(),
        kind: format!("{:?}", error.kind()),
    })?;
    if !metadata.is_file() {
        return Err(NativeLoadError::PathNotFile { path: display });
    }
    let canonical =
        std::fs::canonicalize(path).map_err(|error| NativeLoadError::PathCanonicalization {
            path: display,
            kind: format!("{:?}", error.kind()),
        })?;
    let display = path_preview(&canonical);
    Ok((canonical, display))
}

fn invoke_admitted(
    library: &Arc<dyn LibraryHandle>,
    request: &[u8],
    path: &str,
) -> Result<ffi::PublishedResponse, NativeLoadError> {
    let call = library.invoke(request, Arc::clone(library));
    admitted_response(call, path)
}

fn admitted_response(
    call: ffi::CallResult,
    path: &str,
) -> Result<ffi::PublishedResponse, NativeLoadError> {
    if call.status != 0 {
        return if call.response.is_some() {
            Err(NativeLoadError::PluginStatusWithResponse {
                path: path.to_owned(),
                status: call.status,
            })
        } else if call.len != 0 {
            Err(NativeLoadError::PluginStatusWithLength {
                path: path.to_owned(),
                status: call.status,
                len: call.len,
            })
        } else {
            Err(NativeLoadError::PluginStatus {
                path: path.to_owned(),
                status: call.status,
            })
        };
    }
    let Some(response) = call.response else {
        return if call.len == 0 {
            Err(NativeLoadError::MissingResponse {
                path: path.to_owned(),
            })
        } else {
            Err(NativeLoadError::NullResponseWithLength {
                path: path.to_owned(),
                len: call.len,
            })
        };
    };
    if call.len == 0 {
        return Err(NativeLoadError::ZeroLengthResponse {
            path: path.to_owned(),
        });
    }
    if call.len > REPLY_CAP || call.len > isize::MAX as usize {
        return Err(NativeLoadError::ReplyTooLarge {
            path: path.to_owned(),
            len: call.len,
            cap: REPLY_CAP,
        });
    }
    Ok(response)
}

fn path_preview(path: &Path) -> String {
    bounded_preview(&path.to_string_lossy(), PATH_PREVIEW_CHARS)
}

fn scalar_preview(value: &str) -> String {
    bounded_preview(value, SCALAR_PREVIEW_CHARS)
}

fn bounded_preview(value: &str, cap: usize) -> String {
    let mut chars = value.chars();
    let preview: String = chars.by_ref().take(cap).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod compile_tests;
