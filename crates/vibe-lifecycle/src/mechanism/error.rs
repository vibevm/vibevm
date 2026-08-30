//! The provider layer's one error enum.
//!
//! Every variant is a repairable state of a manifest, a toolchain or a
//! foreign message stream — never a program bug — so each names what was
//! asked for, what the world answered, and the surface that fixes it.
//!
//! Cargo-shaped variants sit in a provider-layer enum for the same reason
//! `DispatchError` carries `InvalidLogConfig`: the builtin set is closed
//! and engine-owned, so its members' refusals are the layer's refusals. A
//! second builtin provider adds variants here; it does not get an enum of
//! its own to drift — which is exactly what the package-provider family
//! does. `Config`, `UnsupportedKind` and the containment/digest family are
//! shared; the rest name one provider's own law.
//!
//! Text that came from outside — a Cargo message, a package name, a path
//! read off a foreign stream — is BOUNDED before it enters a message: a
//! refusal is read by a human repairing a manifest, not a place to paste a
//! megabyte of compiler output.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use specmark::spec;
use thiserror::Error;

pub(crate) mod deploy;

pub use deploy::DeployProviderError;

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

/// Why a builtin build or package provider could not plan, fingerprint,
/// apply or verify one target.
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
        "target `{target}` output `{output}` declares kind `{kind}`, which the builtin provider \
         `{provider}` does not produce; it produces: {supported} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: declare one of \
         the kinds the selected provider produces, or route the target to a \
         provider that produces `{kind}`)"
    )]
    UnsupportedKind {
        target: String,
        provider: String,
        output: String,
        kind: String,
        supported: String,
    },

    /// A target declares a number of outputs the provider's own law does
    /// not admit — §6.1 produces exactly one file, §6.2 exactly one
    /// directory.
    #[error(
        "target `{target}` declares {found} output(s), but the builtin provider `{provider}` \
         produces {expected} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: declare the one \
         output the provider produces)"
    )]
    OutputCount {
        target: String,
        provider: String,
        expected: String,
        found: usize,
    },

    /// A declared source document or directory is not readable.
    #[error(
        "target `{target}` names source `{path}`, which the builtin provider `{provider}` cannot \
         read: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: point the \
         target's `source` at the directory that really holds the packaged sources)"
    )]
    SourceMissing {
        target: String,
        provider: String,
        path: String,
        reason: String,
    },

    /// The Agent Skills frontmatter block is missing or one member is not
    /// what §6.1 requires.
    #[error(
        "[[artifacts.package]] `{target}` frontmatter member `{member}` is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: correct the \
         `SKILL.md` frontmatter block — a static skill is built only from a document whose \
         frontmatter this engine fully understands)"
    )]
    Frontmatter {
        target: String,
        member: String,
        reason: String,
    },

    /// §6.1's "aligns directory/name identity".
    #[error(
        "[[artifacts.package]] `{target}` declares frontmatter name `{declared}` in a skill \
         directory named `{directory}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: make the `name` \
         member and the source directory's own name the same word — a skill has one identity)"
    )]
    SkillIdentity {
        target: String,
        declared: String,
        directory: String,
    },

    /// A line mentions the include token in a shape that is not a
    /// directive, so it would survive into the output as text.
    #[error(
        "[[artifacts.package]] `{target}` line {line} mentions `vibe:include` but is not a \
         directive: `{value}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: write the whole \
         line as `<!-- vibe:include <name> -->` — a malformed directive is never left in place, \
         because that is how a skill claims to be static while dropping a resource)"
    )]
    IncludeMalformed {
        target: String,
        line: usize,
        value: String,
    },

    /// §6.1's "unresolved sibling references".
    #[error(
        "[[artifacts.package]] `{target}` includes `{name}`, which is not a declared resource of \
         this target; declared: {declared} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: declare the \
         resource as an `inputs` row, or correct the directive)"
    )]
    IncludeUnknown {
        target: String,
        name: String,
        declared: String,
    },

    /// §6.1's exactly-once law, in the "more than once" direction.
    #[error(
        "[[artifacts.package]] `{target}` includes `{name}` a second time at line {line} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: include each \
         declared resource exactly once — a resource inlined twice is two copies claiming one \
         origin)"
    )]
    IncludeDuplicate {
        target: String,
        name: String,
        line: usize,
    },

    /// §6.1's exactly-once law, in the "never" direction — the refusal
    /// that keeps a static build from silently dropping resources.
    #[error(
        "[[artifacts.package]] `{target}` declares resource(s) no `vibe:include` directive \
         consumes: {names} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: include each \
         declared resource exactly once, or stop declaring it — a static skill never claims to \
         have absorbed a resource it dropped)"
    )]
    ResourceUnconsumed { target: String, names: String },

    /// §6.1's "rejects executable scripts, shebang-bearing program files,
    /// binary assets".
    #[error(
        "[[artifacts.package]] `{target}` cannot inline resource `{name}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: a static skill \
         inlines textual resources only; ship a program or a binary asset through a directory \
         skill or a plugin instead)"
    )]
    ResourceRejected {
        target: String,
        name: String,
        reason: String,
    },

    /// §6.1's "unsafe traversal" — a declared resource outside the skill's
    /// own source directory.
    #[error(
        "[[artifacts.package]] `{target}` declares resource `{name}`, which is not inside the \
         skill source directory `{source_dir}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: keep a static \
         skill's resources inside its own directory — and its entry document is `SKILL.md`, never \
         a declared resource)"
    )]
    ResourceOutsideSource {
        target: String,
        name: String,
        /// Spelled `source_dir` rather than `source`: `thiserror` reads a
        /// field literally named `source` as the error's CAUSE, and a
        /// directory name is not one.
        source_dir: String,
    },

    /// A consumed artifact was handed to a provider that reads text.
    #[error(
        "[[artifacts.package]] `{target}` consumes artifact `{input}`, which the builtin provider \
         `{provider}` does not accept \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: a static skill \
         is built from declared textual resources of its own source directory — §6.1 requires \
         exact digests and origin framing, and explicitly not a decompiler)"
    )]
    ArtifactInputRejected {
        target: String,
        provider: String,
        input: String,
    },

    /// §6.2's fixed directory shape refused one entry.
    #[error(
        "[[artifacts.package]] `{target}` plugin source entry `{entry}` is not admissible: \
         {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: an Agent \
         Plugin 1.0 directory holds `plugin.json`, `skills/<name>/SKILL.md`, an optional \
         `mcp.json` and reverse-domain client-extension directories — and no links)"
    )]
    PluginShape {
        target: String,
        entry: String,
        reason: String,
    },

    /// §6.2's local 1.0.0 manifest validation refused a member.
    #[error(
        "[[artifacts.package]] `{target}` manifest `{file}` member `{member}` is invalid: \
         {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: correct the \
         member in the plugin source tree — the published 1.0.0 shapes are validated locally, \
         and an unreadable manifest is never packaged)"
    )]
    PluginManifest {
        target: String,
        file: String,
        member: String,
        reason: String,
    },

    /// A distributable could not be written into the engine-owned output
    /// directory.
    #[error(
        "[[artifacts.package]] `{target}` could not write `{path}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: make the \
         project's `target/` writable, then rerun the package phase)"
    )]
    PackageWrite {
        target: String,
        path: String,
        reason: String,
    },

    /// A produced distributable is not inside the engine-owned package
    /// directory, so the engine has no project-relative identity to mint.
    #[error(
        "[[artifacts.package]] `{target}` output `{output}` was produced at `{path}`, which is \
         outside the engine-owned package directory `{package_root}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: this is a \
         defect in the producing provider — the engine owns artifact paths and a provider may \
         not mint one)"
    )]
    PackageOutsideRoot {
        target: String,
        output: String,
        path: String,
        package_root: String,
    },

    /// A produced distributable is not there when verify looks.
    #[error(
        "[[artifacts.package]] `{target}` output `{output}` was produced at `{path}`, but verify \
         found no readable regular file there: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: rerun the \
         package phase; a distributable that vanished between apply and verify is never recorded \
         as produced)"
    )]
    PackageOutputMissing {
        target: String,
        output: String,
        path: String,
        reason: String,
    },

    /// The canonical directory digest refused one entry of the produced
    /// tree.
    #[error(
        "[[artifacts.package]] `{target}` output `{output}` could not be digested at entry \
         `{entry}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: rerun \
         the package phase; a directory digest covers every regular file of the tree and follows \
         no link)"
    )]
    PackageTree {
        target: String,
        output: String,
        entry: String,
        reason: String,
    },

    /// A provider was handed a plan validated for the other package role.
    /// Unreachable through the executor, which builds each plan with the
    /// provider that will consume it, and a refusal rather than a panic
    /// for exactly that reason.
    #[error(
        "the builtin provider `{provider}` was handed a plan validated for a different package \
         role \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE; fix: this is a \
         defect in the package executor — one provider plans and applies one target)"
    )]
    PlanRoleMismatch { provider: String },

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

    /// The §6.3 client adapters' capability refusals.
    ///
    /// Transparent, and a section rather than a second enum, for the reason
    /// this file's own header gives: the layer has ONE error type. It lives
    /// beside its providers rather than under `error/` because it is one
    /// provider FAMILY's law — an adapter's capability matrix — and not a
    /// role's, which is the seam the deploy section below was split on.
    #[error(transparent)]
    Projection(#[from] crate::mechanism::client_projection::ClientProjectionError),

    /// The deploy role's provider refusals — §7.1's destination laws.
    ///
    /// Transparent, and a section rather than a second enum: this layer
    /// still has ONE error type, and a caller that renders a
    /// `MechanismError` renders the deploy refusal verbatim. It lives in
    /// its own cell for the reason the file budget exists — three roles'
    /// refusals in one file stopped being one readable surface — and the
    /// seam is the architecture's own: the deploy role is the one with a
    /// DESTINATION, two extra §3.2 verbs and laws no producing provider
    /// has.
    #[error(transparent)]
    Deploy(#[from] DeployProviderError),

    /// A deploy provider reported one completed operation and the engine
    /// could not make that checkpoint durable. §7.2 requires apply to
    /// checkpoint, so a checkpoint that only ever existed in memory is a
    /// broken transaction, not a slow one — the apply stops here.
    #[error(
        "the deploy checkpoint for resource `{resource}` could not be recorded: {reason}          (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make          the vibevm settings directory writable, then rerun — an apply that cannot checkpoint          cannot be recovered from)"
    )]
    DeployCheckpoint { resource: String, reason: String },

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
