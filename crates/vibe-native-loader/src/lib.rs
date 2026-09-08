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
use vibe_core::manifest::MechanismKey;
use vibe_wire::behaviour::native_deploy::{self, DeployOperation};
use vibe_wire::generated::native::e1::context::Context;
use vibe_wire::generated::native::e1::deploy_reply::DeployReply;
use vibe_wire::generated::native::e1::deploy_request::DeployRequest;
use vibe_wire::generated::native::e1::mechanism_manifest::{
    MechanismDescriptor, NativeDeployOperation,
};
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

/// One exactly admitted compiler extension backed by a strong cached handle.
pub struct NativeCompiler {
    library: Arc<dyn LibraryHandle>,
    display_path: String,
    extension_id: String,
    point: CompilePoint,
}

impl NativeCompiler {
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub const fn point(&self) -> CompilePoint {
        self.point
    }

    pub fn invoke(&self, request: &[u8]) -> Result<Vec<u8>, NativeLoadError> {
        let response = invoke_admitted(&self.library, request, &self.display_path)?;
        Ok(response.bytes().to_vec())
    }
}

/// One exact package-supplied deploy mechanism invocation.
#[derive(Debug, Clone, Copy)]
pub struct NativeMechanismInvocation<'a> {
    /// Absolute path of the admitted immutable native image.
    pub library: &'a Path,
    /// Exact selected provider identity carried into the request.
    pub provider: &'a str,
    /// Exact selected descriptor id.
    pub mechanism_id: &'a str,
    /// Exact logical mechanism key selected from the registry.
    pub logical_key: &'a MechanismKey,
    /// Generated host-authored request.
    pub request: &'a DeployRequest,
}

/// One admitted mechanism image with owned manifest-derived descriptor data.
pub struct NativeMechanism {
    library: Arc<dyn LibraryHandle>,
    display_path: String,
    provider: String,
    logical_key: MechanismKey,
    expected_role: vibe_core::manifest::MechanismRole,
    expected_name: String,
    expected_protocol: u32,
    descriptor: MechanismDescriptor,
}

impl NativeMechanism {
    /// The exact owned descriptor selected from the package manifest.
    pub const fn descriptor(&self) -> &MechanismDescriptor {
        &self.descriptor
    }

    /// The exact owned registry key admitted with the descriptor.
    pub const fn logical_key(&self) -> &MechanismKey {
        &self.logical_key
    }

    pub const fn expected_role(&self) -> vibe_core::manifest::MechanismRole {
        self.expected_role
    }

    pub fn expected_name(&self) -> &str {
        &self.expected_name
    }

    pub const fn expected_protocol(&self) -> u32 {
        self.expected_protocol
    }

    /// Invoke one admitted operation through the shared ABI-1 response guard.
    pub fn invoke(&self, request: &DeployRequest) -> Result<DeployReply, NativeLoadError> {
        if self.expected_role != vibe_core::manifest::MechanismRole::Deploy {
            return Err(NativeLoadError::MechanismRoleInvocation {
                path: self.display_path.clone(),
                role: self.expected_role.to_string(),
            });
        }
        let operation =
            native_deploy::validate_request(request, &self.provider, &self.descriptor.id).map_err(
                |error| NativeLoadError::MechanismRequestAdmission {
                    path: self.display_path.clone(),
                    id: scalar_preview(&self.descriptor.id),
                    reason: error.to_string(),
                },
            )?;
        if !self
            .descriptor
            .operations
            .contains(&manifest_operation(operation))
        {
            return Err(NativeLoadError::MechanismRequestAdmission {
                path: self.display_path.clone(),
                id: scalar_preview(&self.descriptor.id),
                reason: "requested operation is absent from the selected descriptor".to_owned(),
            });
        }
        let encoded = serde_json::to_vec(request).map_err(|error| {
            NativeLoadError::MechanismRequestSerialization {
                path: self.display_path.clone(),
                id: scalar_preview(&self.descriptor.id),
                reason: format!("JSON at line {}, column {}", error.line(), error.column()),
            }
        })?;
        let response = invoke_admitted(&self.library, &encoded, &self.display_path)?;
        admission::parse_mechanism_reply(
            response.bytes(),
            operation,
            &self.descriptor.id,
            &self.display_path,
        )
    }
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
        self.admit_compile(
            invocation.library,
            invocation.extension_id,
            invocation.point,
        )?
        .invoke(invocation.request)
    }

    /// Admit ABI 1 and one exact compiler manifest row without invoking it.
    pub fn admit_compile(
        &self,
        library_path: &Path,
        extension_id: &str,
        point: CompilePoint,
    ) -> Result<NativeCompiler, NativeLoadError> {
        let (library, display_path) = self.admit_library(
            library_path,
            extension_id,
            ExtensionPoint::Compile(point),
            Some(1),
            admission::ManifestFamily::Compiler,
        )?;
        Ok(NativeCompiler {
            library,
            display_path,
            extension_id: extension_id.to_owned(),
            point,
        })
    }

    /// Admit one exact mechanism descriptor and retain its owned data beside
    /// the shared cached library handle.
    pub fn admit_mechanism(
        &self,
        library_path: &Path,
        provider: &str,
        mechanism_id: &str,
        logical_key: &MechanismKey,
    ) -> Result<NativeMechanism, NativeLoadError> {
        let (canonical, display_path) = validate_path(library_path)?;
        let library = self.library(&canonical, &display_path)?;
        let manifest = library.manifest_bytes(&display_path)?;
        let descriptor = admission::select_mechanism_manifest(
            &manifest,
            mechanism_id,
            logical_key,
            &display_path,
        )?;
        Ok(NativeMechanism {
            library,
            display_path,
            provider: provider.to_owned(),
            logical_key: logical_key.clone(),
            expected_role: logical_key.role(),
            expected_name: logical_key.name().to_owned(),
            expected_protocol: native_deploy::PROTOCOL_EPOCH,
            descriptor,
        })
    }

    /// Admit and invoke one package-supplied deploy mechanism.
    pub fn invoke_mechanism(
        &self,
        invocation: NativeMechanismInvocation<'_>,
    ) -> Result<DeployReply, NativeLoadError> {
        self.admit_mechanism(
            invocation.library,
            invocation.provider,
            invocation.mechanism_id,
            invocation.logical_key,
        )?
        .invoke(invocation.request)
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

fn manifest_operation(operation: DeployOperation) -> NativeDeployOperation {
    match operation {
        DeployOperation::Plan => NativeDeployOperation::Plan,
        DeployOperation::Fingerprint => NativeDeployOperation::Fingerprint,
        DeployOperation::Apply => NativeDeployOperation::Apply,
        DeployOperation::Verify => NativeDeployOperation::Verify,
        DeployOperation::Remove => NativeDeployOperation::Remove,
        DeployOperation::Recover => NativeDeployOperation::Recover,
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
