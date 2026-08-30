use specmark::spec;
use thiserror::Error;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE");

/// A typed refusal from native library admission or invocation.
///
/// Diagnostics intentionally contain only bounded path/scalar previews; native
/// request, manifest, and reply bodies are never retained in an error.
///
/// ```
/// use vibe_native_loader::NativeLoadError;
///
/// fn classify(error: &NativeLoadError) -> String {
///     error.to_string()
/// }
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE")]
pub enum NativeLoadError {
    /// The caller supplied a path that was not absolute.
    #[error(
        "native library path is not absolute: `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: pass an absolute native library path)"
    )]
    PathNotAbsolute { path: String },
    /// Metadata for the caller-supplied path could not be read.
    #[error(
        "native library path is unavailable: `{path}` ({kind}) (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: restore a readable native library file at that path)"
    )]
    PathUnavailable { path: String, kind: String },
    /// The caller-supplied path did not identify a regular file.
    #[error(
        "native library path is not a file: `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: pass the absolute path of a regular native library file)"
    )]
    PathNotFile { path: String },
    /// The file path could not be canonicalized.
    #[error(
        "native library path could not be canonicalized: `{path}` ({kind}) (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: repair the path and its filesystem links, then retry)"
    )]
    PathCanonicalization { path: String, kind: String },
    /// Another thread poisoned the loader's cache lock.
    #[error(
        "native library cache is poisoned (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: discard this loader instance and report the preceding panic)"
    )]
    CachePoisoned,
    /// The platform loader refused the canonical library file.
    #[error(
        "native library could not be opened: `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: supply a loadable library for the current platform and architecture)"
    )]
    LibraryOpen { path: String },
    /// A required ABI symbol was absent.
    #[error(
        "native library `{path}` is missing required symbol `{symbol}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: rebuild the plugin with the vibe-ext ABI export macro)"
    )]
    MissingSymbol { path: String, symbol: &'static str },
    /// The plugin ABI is not epoch 1.
    #[error(
        "native library `{path}` reports unsupported ABI {actual}; expected 1; rebuild: vibe build (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: rebuild the plugin against ABI 1 with `vibe build`)"
    )]
    AbiMismatch { path: String, actual: u32 },
    /// The manifest export returned null.
    #[error(
        "native manifest pointer is null for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: publish a non-null NUL-terminated ABI-1 manifest)"
    )]
    ManifestPointerNull { path: String },
    /// No manifest terminator occurred inside the fixed bound.
    #[error(
        "native manifest exceeds the {cap}-byte bound for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: publish a smaller NUL-terminated manifest)"
    )]
    ManifestTooLarge { path: String, cap: usize },
    /// Manifest bytes were not UTF-8.
    #[error(
        "native manifest is not UTF-8 for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: serialize the manifest as UTF-8 JSON)"
    )]
    ManifestUtf8 { path: String },
    /// Manifest JSON did not satisfy the generated root.
    #[error(
        "native manifest is invalid for `{path}` ({reason}) (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: emit the generated ABI-1 Manifest wire shape)"
    )]
    ManifestJson { path: String, reason: String },
    /// Two manifest rows used the same extension id.
    #[error(
        "native manifest contains duplicate extension id `{id}` for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: declare each native extension id exactly once)"
    )]
    DuplicateExtensionId { path: String, id: String },
    /// The requested extension id was not declared.
    #[error(
        "native manifest does not declare extension id `{id}` for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: request an id declared by the plugin manifest)"
    )]
    MissingExtensionId { path: String, id: String },
    /// The selected manifest row used an invalid extension point.
    #[error(
        "native manifest extension `{id}` has invalid point `{point}` for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: declare a valid extension point in the plugin manifest)"
    )]
    InvalidExtensionPoint {
        path: String,
        id: String,
        point: String,
    },
    /// The selected manifest point differed from the typed expectation.
    #[error(
        "native manifest extension `{id}` declares point `{actual}`, expected `{expected}`, for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: align the invocation point with the selected manifest row)"
    )]
    ExtensionPointMismatch {
        path: String,
        id: String,
        actual: String,
        expected: String,
    },
    /// The selected manifest IR schema differed, including presence.
    #[error(
        "native manifest extension `{id}` declares ir_schema {actual}, expected {expected}, for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: align the invocation IR schema with the selected manifest row)"
    )]
    IrSchemaMismatch {
        path: String,
        id: String,
        actual: String,
        expected: String,
    },
    /// The generated context could not be serialized.
    #[error(
        "native context serialization failed ({reason}) (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: report the generated ABI-1 context serialization defect)"
    )]
    ContextSerialization { reason: String },
    /// The plugin returned nonzero without publishing response ownership.
    #[error(
        "native invocation failed with plugin status {status} for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: correct the native handler and retry the invocation)"
    )]
    PluginStatus { path: String, status: i32 },
    /// The plugin returned nonzero after illegally publishing a response.
    #[error(
        "native invocation failed with plugin status {status} after publishing a response for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: return nonzero without publishing response ownership)"
    )]
    PluginStatusWithResponse { path: String, status: i32 },
    /// The plugin returned nonzero with null ownership but a nonzero length.
    #[error(
        "native invocation failed with plugin status {status} and null response length {len} for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: zero the response length when returning no pointer)"
    )]
    PluginStatusWithLength {
        path: String,
        status: i32,
        len: usize,
    },
    /// A null response pointer was paired with a nonzero length.
    #[error(
        "native invocation returned null with length {len} for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: publish a non-null response or zero its length)"
    )]
    NullResponseWithLength { path: String, len: usize },
    /// A successful invocation published no response.
    #[error(
        "native invocation returned no response for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: publish one owned ABI-1 Reply on success)"
    )]
    MissingResponse { path: String },
    /// A non-null response pointer was paired with a zero length.
    #[error(
        "native invocation returned a non-null zero-length response for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: publish the serialized reply length with its pointer)"
    )]
    ZeroLengthResponse { path: String },
    /// The response length exceeded the host's safe fixed bound.
    #[error(
        "native reply length {len} exceeds the {cap}-byte bound for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: return a smaller ABI-1 Reply)"
    )]
    ReplyTooLarge {
        path: String,
        len: usize,
        cap: usize,
    },
    /// Reply bytes were not UTF-8.
    #[error(
        "native reply is not UTF-8 for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: serialize the Reply as UTF-8 JSON)"
    )]
    ReplyUtf8 { path: String },
    /// Reply JSON did not satisfy the strict generated root.
    #[error(
        "native reply is invalid for `{path}` ({reason}) (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: emit exactly the strict generated ABI-1 Reply shape)"
    )]
    ReplyJson { path: String, reason: String },
    /// The reply envelope was not epoch 1.
    #[error(
        "native reply envelope is {actual}, expected 1, for `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; fix: return a Reply with envelope 1)"
    )]
    ReplyEnvelope { path: String, actual: u32 },
}
