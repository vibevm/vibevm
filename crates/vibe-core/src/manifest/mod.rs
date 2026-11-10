//! Manifest schemas used throughout vibevm.
//!
//! - [`Manifest`] — the unified `vibe.toml` carried by every node: a plain
//!   project, a workspace member, a published package, a workspace
//!   coordinator. The node's role is expressed by which sections are
//!   present. Schema: `VIBEVM-SPEC.md` §7,
//!   `spec/modules/vibe-workspace/PROP-007-workspace.md`.
//! - [`Lockfile`] — `vibe.lock` at a workspace's absolute root. Schema:
//!   `VIBEVM-SPEC.md` §7.4.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

mod artifact;
pub mod change;
mod compile;
mod configuration;
mod consumer;
mod declarant_path;
mod decor;
mod deploy;
mod document;
mod extension;
mod field_path;
pub mod i18n;
mod lockfile;
mod mechanism;
mod migration;
mod source_descriptor;
mod source_policy;
pub use source_descriptor::{
    SelectedSource, SourceDescriptor, SourceInterpretation, SourceLoading, SourceRole,
    SourceSelectionError, SourceSelector, select_source_rows,
};
pub use source_policy::{
    CanonicalSourceBinding, EffectiveSourcePolicy, HostingSelector, RegistryPin, SourcePolicyDecl,
    SourcePolicyError, validate_locked_registry_pin, validate_new_registry_pin,
    validate_registry_aliases,
};
mod package;
mod plane;
mod project;
pub mod purl;
mod redirect;
mod rendering;
mod run_command;
mod subskill;
mod target_when;

pub use artifact::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget,
    ArtifactsError, ArtifactsSection, PackageTargetProjection, build_target_for_binary,
};
pub use compile::CompileSection;
pub use configuration::basis as configuration_basis;
pub use configuration::{
    ConfigurationDecl, ConfigurationEffect, ConfigurationFeatureDecl,
    ConfigurationFeatureVisibility, ConfigurationGroupDecl, ConfigurationGuard,
    ConfigurationIntentDecl, ConfigurationProfileDecl, ConfigurationSelection, ForwardStrength,
    NormalizedConfigurationInput, PackageConfiguration, PackageConfigurationError,
};
pub use consumer::{ConsumerNode, NodeRole};
pub use declarant_path::{
    DeclarantPathFault, DeclarantPathMode, declarant_path, declarant_path_component,
    declarant_path_pattern, is_windows_device_name, is_windows_unsafe_component,
};
pub use deploy::{
    DeployError, DeployProfile, DeployProfileProjection, DeploySection, DeployTarget,
};
pub use document::{BootSection, Manifest, OriginSection, WorkspaceSection};
pub use extension::{
    ExtensionAppliesTo, ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionIrLevel,
    ExtensionKey, ExtensionPass, ExtensionPassKind, ExtensionUse, ExtensionWhen, ExtensionsControl,
};
pub use field_path::{FieldError, FieldPath};
pub use lockfile::{
    CONFIGURATION_SCHEMA_VERSION, CURRENT_SCHEMA_VERSION, LEGACY_SCHEMA_VERSION,
    LockedEmbeddedSource, LockedPackage, LockedRemap, LockedRemapSource, LockedRemapSourceError,
    LockedRequirer, LockedSubskill, LockedTemplate, Lockfile, LockfileMeta,
    REMAP_SOURCE_SCHEMA_VERSION, SourceKind, TEMPLATE_SCHEMA_VERSION, TemplateRenderChanged,
    VirtualCapabilityRecord, differing_digest_fields,
};
pub use mechanism::{
    HOST_OWNER, MechanismDecl, MechanismFreshness, MechanismFreshnessParseError, MechanismKey,
    MechanismKeyParseError, MechanismRole, MechanismRoleParseError, MechanismRoutes, ProviderOwner,
    ProviderPin, ProviderPinParseError, is_command_name, is_portable_token,
};
pub(crate) use migration::replay_authored_tokens;
pub use migration::{
    BOOT_SOURCE_INSERT, CURRENT_ABSENT, CURRENT_EPOCH1, IDENTITY, LEGACY_ABSENT,
    ManifestMigrationEpoch, ManifestMigrationOutcome, ManifestMigrationPlan,
    MigrationInputRequirement, MigrationRefusal, MigrationRefusalKind, MigrationSelection,
    MigrationTokenEdit, SNAPSHOT_TO_COPY, SOURCE_BOOT_CURRENT, SOURCE_BOOT_LEGACY,
    manifest_migration_profile_epoch, plan_manifest_migration,
};
pub use package::application::{
    ARG_PLACEHOLDERS, AgentMode, ArchiveFormatDecl, ChoiceDecl, Condition, ExternalResult,
    FeatureDecl, IconDecl, IconFinding, IconSet, IconSlot, InstallerMethod, InstallerOptionDecl,
    LAUNCHER_COMMAND_PREFIX, LauncherActionDecl, LauncherDecl, LauncherKind, LauncherSource,
    LicenseDecl, MENU_CATEGORIES, MigrationDecl, MigrationSide, NativeFormatDecl, Need, OwnedWord,
    ParamEquals, ParameterDecl, ParameterType, ParameterValue, ProfileDecl, PublicCommandDecl,
    RecipeDecl, RequirementDecl, RequirementScope, RequirementTable, ResultDecl, ShellInterpreter,
    SourceInputDecl, StepDecl, StepEnvironment, StepEnvironmentError, StepKind, StepOrigin,
    StepPhase, StepVar, TOOL_VERSIONS_FILE, VersionFileDecl, application_requirements,
    expand_step_template, icon_findings, is_launcher_command_name, launcher_command,
    png_dimensions, result_of, step_env_name, steps_of,
};
pub use package::{
    ABSTRACT_LIMIT, AccessLevel, AllowFriendsOverride, ApplicationDecl, Authorship, BinaryDecl,
    BootCategory, BootSnippet, BootSnippetFragment, CHANNEL_NAME_RULE, CODEX_DOC_MAX_BYTES_DEFAULT,
    ChannelRule, ClaudeHarness, CodexHarness, Compatibility, ConditionalTarget, ConflictsList,
    DigestSource, DocMaxBytes, DocumentationDecl, DocumentsDecl, EmbeddedSourceAuth,
    EmbeddedSourceDecl, EmbeddedSourceKind, EntrypointChoice, EntrypointFile, FeaturesTable,
    GitPackageDep, GitRefKind, GlossaryDecl, HarnessDelivery, HarnessTable, HooksDecl, HostChoice,
    LaneForm, LinkType, MCP_ARG_VARS, Materialization, McpServerDecl, MediaDecl,
    NavigationChapterDecl, NavigationDecl, NavigationSectionDecl, Obsoletes, OverrideEntry,
    OverrideTable, OverrideTarget, PackageFormat, PackageMeta, PathPackageDep, PrereleasePolicy,
    Provides, PublishPosture, Recommends, Requires, RequiresAny, Scalar, SkillDecl,
    SkillResourceDecl, SourceExport, Suggests, SystemPromptSection, TargetOs, TargetSelection,
    TemplateSection, TranslatesDecl, Ttl, VarRegistryDep, VersionItem, VersionSource,
    VisibilityMeta, WhenCondition, channel_name_is_valid,
};
pub use project::{
    ActiveSection, AuthKind, DEFAULT_REGISTRY_GITVERSE_NAME, DEFAULT_REGISTRY_GITVERSE_URL,
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, DEFAULT_REGISTRY_URL, LlmSection, MirrorSection,
    NamingConvention, OverrideSection, ProjectSection, RegistrySection, RegistryType,
    RegistryTypeError, SpecFormat, registry_token_env_name,
};
pub use redirect::{RedirectFile, RedirectSection, RefPolicy, parse_redirect_bytes};
pub use run_command::{RunCommandDecl, RunCommandHandler};
pub use subskill::{
    ActivationRules, DeliveryMode, SubskillConflicts, SubskillContent, SubskillManifest,
    SubskillMeta, SubskillRecommends,
};
pub use target_when::{TargetApplicability, TargetWhen};

use std::fs;
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};

pub(crate) fn read_toml<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str::<T>(&text)
        .map_err(|source| Error::parse_toml(path.to_path_buf(), source, &text))
}

pub(crate) fn write_toml<T, P>(path: P, value: &T) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let rendered = toml::to_string_pretty(value)?;
    let existing = fs::read_to_string(path).ok();
    let final_content = decorate_toml(&rendered, existing.as_deref());
    fs::write(path, final_content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Render from an explicit decoration observation without reading a path.
pub(crate) fn render_toml<T: Serialize>(value: &T, existing: Option<&str>) -> Result<String> {
    let rendered = toml::to_string_pretty(value)?;
    Ok(decorate_toml(&rendered, existing))
}

fn decorate_toml(rendered: &str, existing: Option<&str>) -> String {
    match existing {
        Some(existing) => merge_preserving_comments(existing, rendered),
        None => rendered.to_owned(),
    }
}

/// Merge a freshly-rendered TOML payload (`new_rendered`) into the
/// existing file's comment / whitespace decoration so that
/// human-edited comments survive a `vibe install` / `vibe uninstall`
/// / `vibe registry add` write.
///
/// Strategy:
///
/// 1. Parse both strings as `toml_edit::DocumentMut`. The new
///    document is the authoritative source of structure (the values
///    in memory after the schema's serde-Serialize); the existing
///    document is the source of decoration (leading whitespace,
///    `#`-prefixed comments, blank-line padding).
///
/// 2. Copy the **document-level prefix** (everything before the
///    first table — header comments and blank lines) from existing
///    onto new.
///
/// 3. Recursively pair matching table keys and value shapes, copying
///    their formatting decoration from existing. Tables that only
///    exist in new (e.g. `[requires]` after the operator's first
///    install) get their default decoration. Tables that only existed
///    in existing (e.g. `[active]` if something deletes it) drop with
///    their decoration — structural change wins over decoration
///    preservation. Arrays of tables keep their historical index
///    pairing; ordinary arrays recurse only when their lengths match.
///
/// 4. **Document-level suffix** (anything after the last table —
///    typically operator's footer comments) is preserved by
///    setting `trailing` on the merged document.
///
/// On any parse / merge failure, fall back to the unmerged new
/// rendering. Worst case behaviour matches the prior (pre-toml_edit)
/// implementation, so this strictly improves UX.
fn merge_preserving_comments(existing: &str, new_rendered: &str) -> String {
    let Ok(mut new_doc) = new_rendered.parse::<toml_edit::DocumentMut>() else {
        return new_rendered.to_string();
    };
    let Ok(existing_doc) = existing.parse::<toml_edit::DocumentMut>() else {
        return new_rendered.to_string();
    };

    // 1. Document-level header (everything before the first
    //    table). For an empty `vibe.toml`, `existing_doc.decor()`
    //    has no prefix; for one starting with comments, prefix is
    //    those comments verbatim.
    let existing_root = existing_doc.as_table();
    let new_root = new_doc.as_table_mut();
    if let Some(prefix) = existing_root.decor().prefix() {
        new_root.decor_mut().set_prefix(prefix.clone());
    }

    // 2. Nested decoration. The document root keeps its separately-owned
    //    prefix above; the recursive walker starts at its matching children.
    decor::copy_matching_table_items(existing_root, new_doc.as_table_mut());

    // 3. Document-level trailing — anything after the last
    //    table. `DocumentMut::trailing()` returns the
    //    `&RawString` that holds it; `set_trailing` accepts an
    //    `impl Into<RawString>` (a `&str` works directly). The
    //    distinction matters: top-level table `decor().suffix()`
    //    is empty for documents whose last entry is itself a
    //    table — operator-supplied footer comments live in
    //    `trailing` instead.
    let trailing = existing_doc.trailing().clone();
    new_doc.set_trailing(trailing);

    new_doc.to_string()
}

#[cfg(test)]
mod merge_tests;
