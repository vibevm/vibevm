//! The package role's provider-protocol value types — what the four
//! §3.2 operations of a [`PackageProvider`] hand back.
//!
//! They are shared by every builtin packaging provider on purpose. §6.0.1
//! rules "a crate-internal `PackageProvider` trait beside `BuildProvider`
//! … same operations, same engine-owns list"; a second value vocabulary
//! per provider would make the trait a shape rather than a protocol, and
//! the only genuinely provider-specific thing — the validated `config`
//! table — is carried as one closed variant set rather than smeared across
//! the cells. §7.0.8's windows-zip and §6.3's shared client projection
//! config joined as further variants without creating another protocol.
//!
//! [`PackageProvider`]: crate::mechanism::PackageProvider

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::PathBuf;

use vibe_core::manifest::ArtifactKind;
use vibe_wire::generated::artifact_record::ArtifactShape;

use crate::mechanism::client_projection::config::ClientProjectionConfig;
use crate::mechanism::plugin::config::AgentPluginConfig;
use crate::mechanism::skill::config::StaticSkillConfig;
use crate::mechanism::zip::config::WindowsZipConfig;

/// Where one resolved input came from, and — when it came through the
/// engine's own record — WHAT the record says it is.
///
/// The distinction is recorded rather than derived, because it is exactly
/// the sentence §6.0.2 makes law: an input naming a build output "reads
/// the A2 record the build executor wrote (engine-owned state, never a
/// guessed path)", while a workspace source path "stays a plain contained
/// read". A reader of the evidence can tell which happened.
///
/// The recorded KIND travels with the first arm and only with it, which is
/// the whole of §6.3.0.3's admission law: a client projection "consumes
/// exactly one recorded `agent-plugin` directory artifact", and the only
/// thing entitled to say an artifact is an Agent Plugin is the record its
/// producer wrote. A workspace path carries no recorded kind — there is
/// nobody to have said so — and treating every directory on disk as a
/// canonical plugin is precisely the confusion the typed member removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputOrigin {
    /// Found through `.vibe/state/artifacts/<id>.json` and re-proven,
    /// carrying the kind that record declares.
    ArtifactRecord { kind: ArtifactKind },
    /// Read directly out of the workspace, under containment. It has no
    /// recorded kind, because nothing recorded it.
    WorkspacePath,
}

impl InputOrigin {
    /// The two evidence spellings, unchanged by the typed kind: the
    /// evidence census counts ORIGINS, and a reader of an existing record
    /// must keep reading the same two words.
    ///
    /// They are constants rather than a method on the value because the
    /// census HEADER names both origins while holding neither, and the one
    /// caller that does hold a value asks it for its KIND. A method with no
    /// caller would be a second spelling waiting to part from these.
    pub(crate) const RECORD_SPELLING: &'static str = "artifact-record";
    pub(crate) const WORKSPACE_SPELLING: &'static str = "workspace-path";

    /// The kind the engine's own record declares, when there is a record.
    pub(crate) const fn recorded_kind(self) -> Option<ArtifactKind> {
        match self {
            Self::ArtifactRecord { kind } => Some(kind),
            Self::WorkspacePath => None,
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
    /// a provider's own law admits one; text providers refuse a directory
    /// by name, while §6.3 admits one only with recorded AgentPlugin kind.
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
    /// The three §6.3 client projections share ONE validated table, because
    /// §6.3.0.3 gives them one: a component subset. Which client is asked
    /// for is the SELECTED PROVIDER's identity, never a config member.
    ClientProjection(ClientProjectionConfig),
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
