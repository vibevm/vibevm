//! The provider layer's one error enum.
//!
//! Every variant is a repairable state of a manifest, a toolchain or a
//! foreign message stream — never a program bug — so each names what was
//! asked for, what the world answered, and the surface that fixes it.
//!
//! Cargo-shaped variants sit in a provider-layer enum for the same reason
//! `DispatchError` carries `InvalidLogConfig`: the builtin set is closed
//! and engine-owned, so its members' refusals are the layer's refusals. A
//! second builtin build provider adds variants here; it does not get an
//! enum of its own to drift.
//!
//! Text that came from outside — a Cargo message, a package name, a path
//! read off a foreign stream — is BOUNDED before it enters a message: a
//! refusal is read by a human repairing a manifest, not a place to paste a
//! megabyte of compiler output.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use specmark::spec;
use thiserror::Error;

/// How much foreign text a refusal quotes before it truncates.
const PREVIEW: usize = 200;

/// Bound one untrusted value for a diagnostic.
pub(crate) fn preview(value: &str) -> String {
    if value.chars().count() <= PREVIEW {
        return value.to_owned();
    }
    format!(
        "{}… (truncated)",
        value.chars().take(PREVIEW).collect::<String>()
    )
}

/// Why a builtin build provider could not plan, fingerprint, apply or
/// verify one target.
///
/// ```
/// use vibe_lifecycle::MechanismError;
///
/// let refusal = MechanismError::NoExecutable {
///     target: "vibe-helper".into(),
///     output: "vibe-helper.exe".into(),
///     bin: "vibe-helper".into(),
/// };
/// assert!(refusal.to_string().contains("carried no `executable`"));
/// assert!(refusal.to_string().contains("PROP-054#ONE-MACHINE"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MechanismError {
    /// A declared output names a kind this provider cannot produce.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` declares kind `{kind}`, which the \
         builtin Cargo provider does not produce; it produces: {supported} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: declare `kind = \
         \"executable\"`, or route the target to a \
         provider that produces `{kind}`)"
    )]
    UnsupportedKind {
        target: String,
        output: String,
        kind: String,
        supported: String,
    },

    /// One `config` member is missing, mistyped or unknown.
    #[error(
        "[[artifacts.build]] `{target}` config member `{member}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: correct \
         the member in the target's `config` table)"
    )]
    Config {
        target: String,
        member: String,
        reason: String,
    },

    /// One output's `select` table is missing, mistyped or unknown.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` has an invalid `select` member \
         `{member}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: select a Cargo \
         artifact with `package` and/or `bin` string \
         members)"
    )]
    Select {
        target: String,
        output: String,
        member: String,
        reason: String,
    },

    /// The target's `workdir` does not resolve to a usable directory.
    #[error(
        "[[artifacts.build]] `{target}` workdir `{path}` is unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: point `workdir` \
         at the directory holding the Cargo \
         manifest this target builds)"
    )]
    Workdir {
        target: String,
        path: String,
        reason: String,
    },

    /// The toolchain program could not be started at all.
    #[error(
        "could not run `{program}` for [[artifacts.build]] `{target}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: install a Rust \
         toolchain and make `cargo`/`rustc` reachable \
         on PATH, or route `build:cargo` to another provider)"
    )]
    Spawn {
        target: String,
        program: String,
        reason: String,
    },

    /// The toolchain program ran and refused.
    #[error(
        "`{program}` failed for [[artifacts.build]] `{target}` with {status}: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: repair \
         the build the command reports, then rerun)"
    )]
    NonZero {
        target: String,
        program: String,
        status: String,
        detail: String,
    },

    /// A line of Cargo's own message stream is not the shape the reader
    /// speaks. Unknown *fields* are ignored by design — this is a line
    /// that is not a Cargo message at all.
    #[error(
        "line {line} of `cargo build --message-format=json-render-diagnostics` output for \
         [[artifacts.build]] `{target}` is not a Cargo message: {reason}; the line was `{value}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: this reader \
         speaks Cargo's `reason`-tagged JSON stream — a \
         changed Cargo message format needs the reader updated, never a guessed artifact path)"
    )]
    MessageDecode {
        target: String,
        line: usize,
        reason: String,
        value: String,
    },

    /// `cargo metadata`'s output is not the shape the reader speaks.
    #[error(
        "`cargo metadata` output for [[artifacts.build]] `{target}` is not readable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: this reader \
         speaks `--format-version 1`; a changed metadata \
         format needs the reader updated)"
    )]
    MetadataDecode { target: String, reason: String },

    /// `select.package` names a package the resolved workspace does not
    /// contain.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` selects package `{package}`, which the \
         resolved workspace does not declare; it declares: {candidates} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: name a package \
         `cargo metadata` reports, or point the \
         target's `workdir`/`manifest_path` at the right workspace)"
    )]
    UnknownPackage {
        target: String,
        output: String,
        package: String,
        candidates: String,
    },

    /// `select.bin` names a `[[bin]]` target no selected package declares.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` selects bin `{bin}`, which no selected \
         package declares; the declared bin targets are: {candidates} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: name a \
         `[[bin]]` target `cargo metadata` reports)"
    )]
    UnknownBin {
        target: String,
        output: String,
        bin: String,
        candidates: String,
    },

    /// No compiler-artifact message matched the output's predicate. The
    /// engine refuses instead of falling back to a guessed
    /// `target/<profile>/<name>` path — §5's law 3 by name.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` matched no Cargo compiler-artifact \
         message for {predicate}; {considered} executable-bearing artifact message(s) were read \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: correct the \
         output's `select`, or the target's `config`, so \
         the build actually produces it — an artifact path is NEVER guessed from `target/`)"
    )]
    NoArtifact {
        target: String,
        output: String,
        predicate: String,
        considered: usize,
    },

    /// More than one compiler-artifact message matched. Resolving it by
    /// first-match would make the record's identity depend on Cargo's
    /// emission order.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` matched {matched} Cargo \
         compiler-artifact messages for {predicate}: {names} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: narrow the \
         output's `select` with `package` and/or `bin` so \
         exactly one artifact answers — an ambiguous selection is never resolved by taking the \
         first)"
    )]
    AmbiguousArtifact {
        target: String,
        output: String,
        predicate: String,
        matched: usize,
        names: String,
    },

    /// The matching artifact message carried `"executable": null`.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` matched the Cargo artifact for `{bin}`, \
         but that message carried no `executable` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: select a `bin` \
         target — a library artifact has no \
         executable, and this provider will not guess `target/<profile>/<name>`)"
    )]
    NoExecutable {
        target: String,
        output: String,
        bin: String,
    },

    /// Cargo named an executable that is not there when verify looks.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` was reported at `{path}`, but verify \
         found no readable regular file there: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: rerun the \
         build; a produced artifact that vanished between \
         apply and verify is never recorded as produced)"
    )]
    OutputMissing {
        target: String,
        output: String,
        path: String,
        reason: String,
    },

    /// Cargo named an executable outside the engine-owned build root, so
    /// the engine cannot give it a project-relative identity.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` was reported at `{path}`, which is \
         outside the engine-owned build root `{build_root}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: remove any \
         `--target-dir`/`CARGO_TARGET_DIR` override — the \
         engine owns artifact paths and a provider may not mint one)"
    )]
    OutputOutsideBuildRoot {
        target: String,
        output: String,
        path: String,
        build_root: String,
    },

    /// The produced bytes could not be read for digesting.
    #[error(
        "[[artifacts.build]] `{target}` output `{output}` could not be digested at `{path}`: \
         {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: rerun the \
         build; a record never carries a digest of \
         anything but the produced bytes)"
    )]
    Digest {
        target: String,
        output: String,
        path: String,
        reason: String,
    },
}
