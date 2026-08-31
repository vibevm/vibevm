//! Typed refusals for native artifact resolution and source builds.

use specmark::spec;
use thiserror::Error;

/// Why a native library could not be selected, built, recorded or reused.
#[derive(Debug, Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE")]
pub enum NativeArtifactError {
    #[error(
        "unsupported native platform pair os=`{os}` arch=`{arch}`; supported platform keys are: windows-x86_64, linux-x86_64, macos-aarch64 (spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED); fix: use a supported host platform"
    )]
    UnsupportedPlatform { os: String, arch: String },

    #[error(
        "native extension `{extension}` declares prebuilt `{path}` for `{platform}`, but the path does not have the exact `{suffix}` suffix (spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED); fix: correct the current-platform declaration (source fallback is forbidden while that key is present)"
    )]
    PrebuiltSuffix {
        extension: String,
        platform: String,
        path: String,
        suffix: String,
    },

    #[error(
        "native extension `{extension}` declares prebuilt `{path}` for `{platform}`, but it is unavailable: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED); fix: correct the current-platform declaration (source fallback is forbidden while that key is present)"
    )]
    PrebuiltUnavailable {
        extension: String,
        platform: String,
        path: String,
        reason: String,
    },

    #[error(
        "native extension `{extension}` has neither a `{platform}` prebuilt nor `crate_dir` (declared prebuilt keys: {declared}) (spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED); fix: declare the current prebuilt or a source crate_dir"
    )]
    NoCurrentArtifact {
        extension: String,
        platform: String,
        declared: String,
    },

    #[error(
        "native provider `{provider}` root `{path}` is unusable: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED); fix: restore a contained provider root"
    )]
    ProviderRoot {
        provider: String,
        path: String,
        reason: String,
    },

    #[error(
        "native provider `{provider}` crate_dir `{crate_dir}` is unusable: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: restore the provider source tree and Cargo.toml"
    )]
    CrateDirectory {
        provider: String,
        crate_dir: String,
        reason: String,
    },

    #[error(
        "could not resolve logical `build:cargo` for native source builds: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT); fix: correct the host [mechanisms] route or installed provider"
    )]
    MechanismSelection { reason: String },

    #[error(
        "native source build selected builtin provider `{provider}` with handler `{name}`, not the shipped Cargo builtin (spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT); fix: restore the shipped build:cargo route"
    )]
    UnknownBuiltin { provider: String, name: String },

    #[error(
        "native source build selected non-builtin provider `{provider}` (handler kind `{kind}`), but that build-provider transport is not landed (spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT); fix: remove the displacement or use the shipped build:cargo builtin"
    )]
    TransportNotLanded { provider: String, kind: String },

    #[error(
        "native dependency provider `{provider}` cannot prepare build-output ignores at `{root}`: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: restore the exact dependency-slot ancestry and writable ignore file"
    )]
    BuildIgnore {
        provider: String,
        root: String,
        reason: String,
    },

    #[error(
        "could not run `{program}` for native provider `{provider}`: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: restore the Cargo/Rust toolchain and provider root"
    )]
    Spawn {
        provider: String,
        program: String,
        reason: String,
    },

    #[error(
        "`{program}` failed for native provider `{provider}` with {status}: {detail} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: correct the provider crate or warm the offline dependency cache"
    )]
    NonZero {
        provider: String,
        program: String,
        status: String,
        detail: String,
    },

    #[error(
        "Cargo JSON for native provider `{provider}` is invalid: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: use Cargo JSON diagnostics without non-JSON stdout"
    )]
    CargoJson { provider: String, reason: String },

    #[error(
        "Cargo metadata for native provider `{provider}` identified {found} root package(s) at `{manifest}`; exactly one manifest owner is required (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: point crate_dir at one Cargo package manifest"
    )]
    RootPackage {
        provider: String,
        manifest: String,
        found: usize,
    },

    #[error(
        "Cargo metadata for native provider `{provider}` identified {found} cdylib target(s) in root package `{package}`; exactly one target whose crate_types include `cdylib` is required (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: declare exactly one root cdylib library target"
    )]
    CdylibTarget {
        provider: String,
        package: String,
        found: usize,
    },

    #[error(
        "Cargo build output for native provider `{provider}` matched {found} compiler-artifact message(s) for root cdylib `{target}`; exactly one is required (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: make Cargo emit one root cdylib compiler-artifact"
    )]
    CompilerArtifact {
        provider: String,
        target: String,
        found: usize,
    },

    #[error(
        "Cargo compiler-artifact for native provider `{provider}` contains {found} regular `{suffix}` filename(s) inside `{target_root}`; exactly one is required and filenames are never selected by order (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: make the root target emit one current-platform cdylib"
    )]
    CdylibFilename {
        provider: String,
        suffix: String,
        target_root: String,
        found: usize,
    },

    #[error(
        "Cargo-reported native artifact `{path}` for provider `{provider}` is invalid: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD); fix: keep the regular cdylib inside the provider target root"
    )]
    ReportedArtifact {
        provider: String,
        path: String,
        reason: String,
    },

    #[error(
        "native load image `{path}` is invalid: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE); fix: restore writable selected-project `.vibe` state and retry the admitted artifact"
    )]
    LoadImage { path: String, reason: String },

    #[error(
        "native source witness for provider `{provider}` cannot be computed: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY); fix: restore a valid labelled content hash or readable shippable host tree"
    )]
    SourceWitness { provider: String, reason: String },

    #[error(
        "native artifact record `{record}` could not be written: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY); fix: restore writable selected-project artifact state and run `vibe build`"
    )]
    RecordWrite { record: String, reason: String },

    #[error(
        "native source artifact record `{record}` is missing (spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY); fix: run `vibe build`"
    )]
    SourceRecordMissing { record: String },

    #[error(
        "native source artifact `{record}` is unavailable: {reason} (spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY); fix: run `vibe build`"
    )]
    SourceState { record: String, reason: String },
}
