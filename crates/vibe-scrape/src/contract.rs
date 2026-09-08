//! Strict schema-1 scrape contract and semantic validation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::glob::{Glob, PortablePath};
use crate::model::ScrapeError;

mod validation;

pub const MAX_HEALTH_STREAM_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_HEALTH_RESULT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema: u32,
    pub id: String,
    pub policy: Policy,
    pub scope: Scope,
    pub commit: Commit,
    pub health: Health,
    pub classify: Vec<ClassifyRule>,
    #[serde(default)]
    pub baseline: Vec<Baseline>,
    #[serde(default)]
    pub rewrite: Vec<RewriteRule>,
    #[serde(default)]
    pub relocate: Vec<Relocation>,
    #[serde(rename = "assert")]
    pub assertions: Vec<Assertion>,
    pub healthcheck: Vec<Healthcheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub unclassified: Refuse,
    pub links: Refuse,
    pub concurrent_change: Refuse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub closed_roots: Vec<String>,
    pub outside: ImplicitKeep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub contract: ContractAction,
}

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename = $value)] $variant),+ }
    };
}

string_enum!(Refuse { Refuse => "refuse" });
string_enum!(ImplicitKeep { ImplicitKeep => "implicit-keep" });
string_enum!(ContractAction { DeleteLast => "delete-last", Preserve => "preserve" });
string_enum!(Owner { Project => "project", Vibe => "vibe" });
string_enum!(Proof { ContractAssertionV1 => "contract-assertion-v1", Sha256V1 => "sha256-v1", VibeGeneratedV1 => "vibe-generated-v1" });
string_enum!(ModifiedPolicy { Refuse => "refuse", Keep => "keep", Delete => "delete" });
string_enum!(SetMatches { ZeroOrMore => "zero-or-more", OneOrMore => "one-or-more", ExactlyOne => "exactly-one" });
string_enum!(PerFileMatches { ZeroOrOnePerFile => "zero-or-one-per-file", ExactlyOnePerFile => "exactly-one-per-file" });
string_enum!(RustForm { Scope => "scope", Spec => "spec", Verifies => "verifies", Cell => "cell" });
string_enum!(NodeManager { Npm => "npm", Pnpm => "pnpm", Yarn => "yarn" });
string_enum!(DependencyManager { Cargo => "cargo", Npm => "npm", Pnpm => "pnpm", Yarn => "yarn", Go => "go" });
string_enum!(Language { Rust => "rust", TypeScript => "typescript", Go => "go" });
string_enum!(BaselineMode { Strict => "strict", NoRegression => "no-regression" });
string_enum!(AfterFailure { Rollback => "rollback" });
string_enum!(NetworkPolicy { Deny => "deny", ToolOffline => "tool-offline", Inherit => "inherit" });
string_enum!(TestsMode { Skip => "skip", IfPresent => "if-present", Required => "required" });
string_enum!(CargoBuild { Check => "check", Build => "build" });
string_enum!(CargoProfile { Dev => "dev", Release => "release" });
string_enum!(InstallMode { None => "none", Ci => "ci" });
string_enum!(MavenRunner { WrapperFirst => "wrapper-first", Explicit => "explicit" });
string_enum!(CustomProtocol { ExitCode => "exit-code", VibeHealthJsonV1 => "vibe-health-json-v1" });
string_enum!(ConflictPolicy { Refuse => "refuse" });

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ClassifyRule {
    #[serde(rename = "keep")]
    Keep {
        id: String,
        patterns: Vec<String>,
        owner: Owner,
        require_match: bool,
    },
    #[serde(rename = "delete")]
    Delete {
        id: String,
        patterns: Vec<String>,
        owner: Owner,
        proof: Proof,
        modified: ModifiedPolicy,
        require_match: bool,
    },
    #[serde(rename = "generated")]
    Generated {
        id: String,
        patterns: Vec<String>,
        owner: Owner,
        proof: Proof,
        modified: ModifiedPolicy,
        require_match: bool,
    },
}

impl ClassifyRule {
    pub fn id(&self) -> &str {
        match self {
            Self::Keep { id, .. } | Self::Delete { id, .. } | Self::Generated { id, .. } => id,
        }
    }

    pub fn patterns(&self) -> &[String] {
        match self {
            Self::Keep { patterns, .. }
            | Self::Delete { patterns, .. }
            | Self::Generated { patterns, .. } => patterns,
        }
    }

    pub fn require_match(&self) -> bool {
        match self {
            Self::Keep { require_match, .. }
            | Self::Delete { require_match, .. }
            | Self::Generated { require_match, .. } => *require_match,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RewriteRule {
    #[serde(rename = "managed-block-remove-v1")]
    ManagedBlockRemoveV1 {
        id: String,
        paths: Vec<String>,
        marker: String,
        matches: PerFileMatches,
    },
    #[serde(rename = "rust-specmark-strip-v1")]
    RustSpecmarkStripV1 {
        id: String,
        patterns: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        forms: Vec<RustForm>,
        matches: SetMatches,
    },
    #[serde(rename = "cargo-package-remove-v1")]
    CargoPackageRemoveV1 {
        id: String,
        manifests: Vec<String>,
        package: String,
        #[serde(default)]
        aliases: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "node-package-remove-v1")]
    NodePackageRemoveV1 {
        id: String,
        package_json: String,
        lockfile: String,
        manager: NodeManager,
        packages: Vec<String>,
        #[serde(default)]
        script_paths: Vec<Vec<String>>,
        #[serde(default)]
        config_paths: Vec<Vec<String>>,
        matches: SetMatches,
    },
    #[serde(rename = "go-module-remove-v1")]
    GoModuleRemoveV1 {
        id: String,
        go_mod: String,
        #[serde(default)]
        go_sum: Option<String>,
        modules: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "toml-array-values-remove-v1")]
    TomlArrayValuesRemoveV1 {
        id: String,
        path: String,
        table: Vec<String>,
        key: String,
        values: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "typescript-spec-comments-strip-v1")]
    TypeScriptSpecCommentsStripV1 {
        id: String,
        patterns: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "go-spec-directives-strip-v1")]
    GoSpecDirectivesStripV1 {
        id: String,
        patterns: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "json-member-remove-v1")]
    JsonMemberRemoveV1 {
        id: String,
        path: String,
        object: Vec<String>,
        members: Vec<String>,
        matches: SetMatches,
    },
    #[serde(rename = "text-exact-replace-v1")]
    TextExactReplaceV1 {
        id: String,
        path: String,
        sha256: String,
        before: String,
        after: String,
        occurrences: u64,
    },
}

impl RewriteRule {
    pub fn id(&self) -> &str {
        match self {
            Self::ManagedBlockRemoveV1 { id, .. }
            | Self::RustSpecmarkStripV1 { id, .. }
            | Self::CargoPackageRemoveV1 { id, .. }
            | Self::NodePackageRemoveV1 { id, .. }
            | Self::GoModuleRemoveV1 { id, .. }
            | Self::TomlArrayValuesRemoveV1 { id, .. }
            | Self::TypeScriptSpecCommentsStripV1 { id, .. }
            | Self::GoSpecDirectivesStripV1 { id, .. }
            | Self::JsonMemberRemoveV1 { id, .. }
            | Self::TextExactReplaceV1 { id, .. } => id,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::ManagedBlockRemoveV1 { .. } => "managed-block-remove-v1",
            Self::RustSpecmarkStripV1 { .. } => "rust-specmark-strip-v1",
            Self::CargoPackageRemoveV1 { .. } => "cargo-package-remove-v1",
            Self::NodePackageRemoveV1 { .. } => "node-package-remove-v1",
            Self::GoModuleRemoveV1 { .. } => "go-module-remove-v1",
            Self::TomlArrayValuesRemoveV1 { .. } => "toml-array-values-remove-v1",
            Self::TypeScriptSpecCommentsStripV1 { .. } => "typescript-spec-comments-strip-v1",
            Self::GoSpecDirectivesStripV1 { .. } => "go-spec-directives-strip-v1",
            Self::JsonMemberRemoveV1 { .. } => "json-member-remove-v1",
            Self::TextExactReplaceV1 { .. } => "text-exact-replace-v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relocation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub conflict: ConflictPolicy,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Assertion {
    #[serde(rename = "paths-absent-v1")]
    PathsAbsentV1 { id: String, patterns: Vec<String> },
    #[serde(rename = "text-literal-absent-v1")]
    TextLiteralAbsentV1 {
        id: String,
        patterns: Vec<String>,
        needles: Vec<String>,
    },
    #[serde(rename = "cargo-path-prefix-absent-v1")]
    CargoPathPrefixAbsentV1 {
        id: String,
        manifests: Vec<String>,
        prefixes: Vec<String>,
    },
    #[serde(rename = "language-metadata-absent-v1")]
    LanguageMetadataAbsentV1 {
        id: String,
        language: Language,
        patterns: Vec<String>,
    },
    #[serde(rename = "dependency-identities-absent-v1")]
    DependencyIdentitiesAbsentV1 {
        id: String,
        manager: DependencyManager,
        manifests: Vec<String>,
        identities: Vec<String>,
    },
}

impl Assertion {
    pub fn id(&self) -> &str {
        match self {
            Self::PathsAbsentV1 { id, .. }
            | Self::TextLiteralAbsentV1 { id, .. }
            | Self::CargoPathPrefixAbsentV1 { id, .. }
            | Self::LanguageMetadataAbsentV1 { id, .. }
            | Self::DependencyIdentitiesAbsentV1 { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub baseline: BaselineMode,
    pub before_failure: Refuse,
    pub after_failure: AfterFailure,
    pub parallel: bool,
    pub network: NetworkPolicy,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
    pub max_result_bytes: u64,
    pub termination_grace_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    pub path_exists: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Healthcheck {
    #[serde(rename = "cargo")]
    Cargo {
        id: String,
        root: String,
        build: CargoBuild,
        workspace: bool,
        locked: bool,
        all_targets: bool,
        tests: TestsMode,
        profile: CargoProfile,
        features: Vec<String>,
        timeout_seconds: u64,
        #[serde(default)]
        when: Option<When>,
        #[serde(default)]
        network: Option<NetworkPolicy>,
    },
    #[serde(rename = "npm")]
    Npm {
        id: String,
        root: String,
        manager: NodeManager,
        lockfile: String,
        install: InstallMode,
        #[serde(default)]
        build_script: Option<String>,
        #[serde(default)]
        typecheck_script: Option<String>,
        tests: TestsMode,
        #[serde(default)]
        test_script: Option<String>,
        timeout_seconds: u64,
        #[serde(default)]
        when: Option<When>,
        #[serde(default)]
        network: Option<NetworkPolicy>,
    },
    #[serde(rename = "maven")]
    Maven {
        id: String,
        root: String,
        runner: MavenRunner,
        goal: String,
        offline: bool,
        tests: TestsMode,
        timeout_seconds: u64,
        #[serde(default)]
        when: Option<When>,
        #[serde(default)]
        network: Option<NetworkPolicy>,
    },
    #[serde(rename = "python-pip")]
    PythonPip {
        id: String,
        root: String,
        interpreter: String,
        source_roots: Vec<String>,
        dependency_check: bool,
        build: bool,
        tests: TestsMode,
        #[serde(default)]
        test_runner: Option<String>,
        timeout_seconds: u64,
        #[serde(default)]
        when: Option<When>,
        #[serde(default)]
        network: Option<NetworkPolicy>,
    },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        root: String,
        source: String,
        snapshot: Vec<String>,
        interpreter: String,
        argv: Vec<String>,
        protocol: CustomProtocol,
        reads: Vec<String>,
        writes: Vec<String>,
        spawn: bool,
        timeout_seconds: u64,
        #[serde(default)]
        when: Option<When>,
        network: NetworkPolicy,
    },
}

impl Healthcheck {
    pub fn id(&self) -> &str {
        match self {
            Self::Cargo { id, .. }
            | Self::Npm { id, .. }
            | Self::Maven { id, .. }
            | Self::PythonPip { id, .. }
            | Self::Custom { id, .. } => id,
        }
    }

    pub fn tests(&self) -> Option<TestsMode> {
        match self {
            Self::Cargo { tests, .. }
            | Self::Npm { tests, .. }
            | Self::Maven { tests, .. }
            | Self::PythonPip { tests, .. } => Some(*tests),
            Self::Custom { .. } => None,
        }
    }

    pub fn when(&self) -> Option<&When> {
        match self {
            Self::Cargo { when, .. }
            | Self::Npm { when, .. }
            | Self::Maven { when, .. }
            | Self::PythonPip { when, .. }
            | Self::Custom { when, .. } => when.as_ref(),
        }
    }
}

impl Contract {
    pub fn parse(bytes: &[u8]) -> Result<Self, ScrapeError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| ScrapeError::contract(format!("contract is not UTF-8: {error}")))?;
        let value: Self = toml::from_str(text)
            .map_err(|error| ScrapeError::contract(format!("invalid schema-1 TOML: {error}")))?;
        value.validate()?;
        Ok(value)
    }
}

fn validate_root(root: &str) -> Result<(), ScrapeError> {
    if root == "." {
        Ok(())
    } else {
        PortablePath::parse(root).map(|_| ())
    }
}

fn validate_token(value: &str, field: &str) -> Result<(), ScrapeError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return invalid(format!("{field} must be a nonempty portable token"));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &str) -> Result<(), ScrapeError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return invalid(format!("{field} must use sha256:<64-hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return invalid(format!("{field} must use lowercase sha256:<64-hex>"));
    }
    Ok(())
}

fn unique_id(seen: &mut BTreeSet<String>, id: &str) -> Result<(), ScrapeError> {
    validate_token(id, "row id")?;
    if !seen.insert(id.to_owned()) {
        return invalid(format!("duplicate contract id `{id}`"));
    }
    Ok(())
}

fn nonempty(value: &str, field: &str) -> Result<(), ScrapeError> {
    if value.is_empty() {
        invalid(format!("{field} must be nonempty"))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, ScrapeError> {
    Err(ScrapeError::contract(message))
}

#[cfg(test)]
mod tests;
