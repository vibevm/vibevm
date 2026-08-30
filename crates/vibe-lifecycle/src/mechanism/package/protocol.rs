//! The package role's provider-protocol value types — what the four
//! §3.2 operations of a [`PackageProvider`] hand back.
//!
//! They are shared by every builtin packaging provider on purpose. §6.0.1
//! rules "a crate-internal `PackageProvider` trait beside `BuildProvider`
//! … same operations, same engine-owns list"; a second value vocabulary
//! per provider would make the trait a shape rather than a protocol, and
//! the only genuinely provider-specific thing — the validated `config`
//! table — is carried as one closed variant set rather than smeared across
//! the cells. §7.0.8's windows-zip joined it as a third variant and needed
//! nothing else.
//!
//! [`PackageProvider`]: crate::mechanism::PackageProvider

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::PathBuf;

use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::plugin::config::AgentPluginConfig;
use crate::mechanism::skill::config::StaticSkillConfig;
use crate::mechanism::zip::config::WindowsZipConfig;

/// Where one resolved input came from.
///
/// The distinction is recorded rather than derived, because it is exactly
/// the sentence §6.0.2 makes law: an input naming a build output "reads
/// the A2 record the build executor wrote (engine-owned state, never a
/// guessed path)", while a workspace source path "stays a plain contained
/// read". A reader of the evidence can tell which happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputOrigin {
    /// Found through `.vibe/state/artifacts/<id>.json` and re-proven.
    ArtifactRecord,
    /// Read directly out of the workspace, under containment.
    WorkspacePath,
}

impl InputOrigin {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactRecord => "artifact-record",
            Self::WorkspacePath => "workspace-path",
        }
    }
}

/// One declared input, resolved and proven by the ENGINE before any
/// provider sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedInput {
    /// The identity a provider's own config names it by: the artifact id
    /// for a consumed artifact, the canonical relative spelling for a
    /// workspace path.
    pub(crate) name: String,
    /// The rendered declaration row — `artifact:<id>` or `path:<rel>` —
    /// the one spelling that also enters the config fingerprint.
    pub(crate) reference: String,
    pub(crate) absolute: PathBuf,
    /// Project-relative, forward-slashed.
    pub(crate) relative: String,
    /// 64 lowercase hex over the bytes that are really there NOW — the
    /// file's SHA-256, or the canonical tree digest of a directory.
    pub(crate) digest: String,
    pub(crate) bytes: u64,
    /// The physical shape on disk. A directory input is legal only where
    /// a provider's own law admits one; the two §6 packaging providers
    /// read text and refuse it by name at the point they try to.
    pub(crate) shape: ArtifactShape,
    pub(crate) origin: InputOrigin,
}

/// The validated `config` table of one package target, in its role's own
/// shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PackageConfig {
    StaticSkill(StaticSkillConfig),
    AgentPlugin(AgentPluginConfig),
    WindowsZip(WindowsZipConfig),
}

/// One declared output, resolved against the provider's own grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedPackageOutput {
    pub(crate) id: String,
    pub(crate) kind: ArtifactKind,
    pub(crate) shape: ArtifactShape,
    /// Where inside the engine-owned output directory it lands. `"."`
    /// names the output directory itself, which is what a DIRECTORY
    /// distributable is.
    pub(crate) relative: String,
    /// The media type a `file`-shape distributable declares, when its
    /// format is fixed by the provider's own law.
    pub(crate) media_type: Option<String>,
}

/// What `plan` reports: the validated config, the engine's output
/// directory, the declared inputs as rows, and the outputs this provider
/// would produce. Producing it spawns nothing and touches no path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackagePlan {
    pub(crate) config: PackageConfig,
    pub(crate) output_dir: PathBuf,
    /// The rendered declaration rows, in declaration order.
    pub(crate) inputs: Vec<String>,
    pub(crate) outputs: Vec<PlannedPackageOutput>,
    /// A control-free one-line summary of what the plan would do, for the
    /// record's evidence.
    pub(crate) summary: String,
}

/// The engine-fresh fingerprint over one target's complete closed input
/// set — §4.1's "Engine freshness is legal only when the complete input
/// set is closed and hashable".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageFingerprint {
    /// 64 lowercase hex over every input the distributable is derived
    /// from, in a canonical order.
    pub(crate) digest: String,
    /// How many inputs entered it, so evidence can say the census was
    /// complete rather than merely non-empty.
    pub(crate) counted: usize,
}

/// One distributable a provider reported producing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StagedArtifact {
    pub(crate) output_id: String,
    pub(crate) kind: ArtifactKind,
    pub(crate) shape: ArtifactShape,
    /// Exactly the path the provider says it wrote.
    pub(crate) absolute: PathBuf,
    /// The media type recorded for a `file`-shape distributable.
    pub(crate) media_type: Option<String>,
}

/// One distributable `verify` independently proved and digested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedPackageArtifact {
    pub(crate) output_id: String,
    /// Forward-slashed absolute placement.
    pub(crate) path_absolute: String,
    /// Forward-slashed project-relative identity.
    pub(crate) path_relative: String,
    /// 64 lowercase hex — over the file's bytes, or over the canonical
    /// directory manifest.
    pub(crate) digest: String,
    pub(crate) bytes: u64,
    /// How many files the digest covers. One for a file distributable;
    /// the exact tree census for a directory one, which is what makes a
    /// silently skipped file visible in the evidence.
    pub(crate) files: usize,
}
