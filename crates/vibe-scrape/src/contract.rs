//! Strict schema-1 scrape contract and semantic validation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::glob::{Glob, PortablePath};
use crate::model::ScrapeError;

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

    pub fn validate(&self) -> Result<(), ScrapeError> {
        if self.schema != 1 {
            return invalid("schema must equal 1");
        }
        validate_token(&self.id, "contract id")?;
        if self.classify.is_empty() {
            return invalid("at least one classify row is required");
        }
        if self.assertions.is_empty() {
            return invalid("at least one assertion is required");
        }
        if self.healthcheck.is_empty() {
            return invalid("at least one healthcheck is required");
        }
        if self.scope.closed_roots.is_empty() {
            return invalid("scope.closed_roots must be nonempty");
        }
        if self.health.parallel {
            return invalid("health.parallel must be false in schema 1");
        }
        if self.health.max_stdout_bytes == 0
            || self.health.max_stderr_bytes == 0
            || self.health.max_result_bytes == 0
        {
            return invalid("health output/result caps must be positive");
        }
        if self.health.max_stdout_bytes > MAX_HEALTH_STREAM_BYTES
            || self.health.max_stderr_bytes > MAX_HEALTH_STREAM_BYTES
            || self.health.max_result_bytes > MAX_HEALTH_RESULT_BYTES
        {
            return invalid(format!(
                "health caps exceed engine maxima (stdout/stderr {MAX_HEALTH_STREAM_BYTES}, result {MAX_HEALTH_RESULT_BYTES})"
            ));
        }
        if self.health.termination_grace_seconds == 0 {
            return invalid("health.termination_grace_seconds must be positive");
        }
        if !self.healthcheck.iter().any(|check| check.when().is_none()) {
            return invalid("at least one healthcheck must be unconditional");
        }

        let mut ids = BTreeSet::new();
        for rule in &self.classify {
            unique_id(&mut ids, rule.id())?;
            validate_nonempty_globs(rule.patterns(), "classify.patterns")?;
            match rule {
                ClassifyRule::Keep { owner, .. } if *owner != Owner::Project => {
                    return invalid("keep requires owner = project");
                }
                ClassifyRule::Delete { owner, .. } | ClassifyRule::Generated { owner, .. }
                    if *owner != Owner::Vibe =>
                {
                    return invalid("delete/generated require owner = vibe");
                }
                _ => {}
            }
            reject_git_selection(rule.patterns(), &[])?;
        }
        validate_unique_literals(&self.scope.closed_roots, "scope.closed_roots")?;
        reject_git_literals(&self.scope.closed_roots, "scope.closed_roots")?;

        let mut baselines = BTreeMap::new();
        for baseline in &self.baseline {
            PortablePath::parse(&baseline.path)?;
            reject_git_literal(&baseline.path, "baseline.path")?;
            validate_digest(&baseline.sha256, "baseline.sha256")?;
            if baselines
                .insert(baseline.path.as_str(), baseline.sha256.as_str())
                .is_some()
            {
                return invalid(format!("duplicate baseline path `{}`", baseline.path));
            }
        }

        for rewrite in &self.rewrite {
            unique_id(&mut ids, rewrite.id())?;
            validate_rewrite(rewrite)?;
        }
        for relocation in &self.relocate {
            unique_id(&mut ids, &relocation.id)?;
            PortablePath::parse(&relocation.from)?;
            PortablePath::parse(&relocation.to)?;
            reject_git_literal(&relocation.from, "relocate.from")?;
            reject_git_literal(&relocation.to, "relocate.to")?;
            if relocation.from == relocation.to {
                return invalid(format!(
                    "relocation `{}` has identical source and destination",
                    relocation.id
                ));
            }
        }
        validate_relocation_graph(&self.relocate)?;
        for assertion in &self.assertions {
            unique_id(&mut ids, assertion.id())?;
            validate_assertion(assertion)?;
        }
        for check in &self.healthcheck {
            unique_id(&mut ids, check.id())?;
            validate_healthcheck(check, self.health.baseline)?;
        }
        Ok(())
    }
}

fn validate_rewrite(rule: &RewriteRule) -> Result<(), ScrapeError> {
    match rule {
        RewriteRule::ManagedBlockRemoveV1 { paths, marker, .. } => {
            validate_unique_literals(paths, "rewrite.paths")?;
            reject_git_literals(paths, "rewrite.paths")?;
            validate_token(marker, "managed marker")?;
            if marker != "vibevm" {
                return invalid(format!(
                    "managed marker `{marker}` is not a registered schema-1 provider identity"
                ));
            }
        }
        RewriteRule::RustSpecmarkStripV1 {
            patterns,
            exclude,
            forms,
            matches,
            ..
        } => {
            validate_nonempty_globs(patterns, "rewrite.patterns")?;
            validate_globs(exclude, "rewrite.exclude")?;
            reject_git_selection(patterns, exclude)?;
            if forms.is_empty() || forms.iter().collect::<BTreeSet<_>>().len() != forms.len() {
                return invalid("rust forms must be nonempty and unique");
            }
            if *matches == SetMatches::ExactlyOne {
                return invalid("rust-specmark-strip-v1 does not admit exactly-one");
            }
        }
        RewriteRule::CargoPackageRemoveV1 {
            manifests,
            package,
            aliases,
            ..
        } => {
            validate_nonempty_globs(manifests, "rewrite.manifests")?;
            nonempty(package, "Cargo package")?;
            validate_optional_unique_nonempty(aliases, "Cargo aliases")?;
            reject_git_selection(manifests, &[])?;
        }
        RewriteRule::NodePackageRemoveV1 {
            package_json,
            lockfile,
            packages,
            script_paths,
            config_paths,
            ..
        } => {
            PortablePath::parse(package_json)?;
            PortablePath::parse(lockfile)?;
            reject_git_literal(package_json, "rewrite.package_json")?;
            reject_git_literal(lockfile, "rewrite.lockfile")?;
            validate_unique_nonempty(packages, "Node packages")?;
            validate_component_paths(script_paths, "script_paths")?;
            validate_component_paths(config_paths, "config_paths")?;
        }
        RewriteRule::GoModuleRemoveV1 {
            go_mod,
            go_sum,
            modules,
            ..
        } => {
            PortablePath::parse(go_mod)?;
            if let Some(path) = go_sum {
                PortablePath::parse(path)?;
            }
            reject_git_literal(go_mod, "rewrite.go_mod")?;
            if let Some(path) = go_sum {
                reject_git_literal(path, "rewrite.go_sum")?;
            }
            validate_unique_nonempty(modules, "Go modules")?;
        }
        RewriteRule::TomlArrayValuesRemoveV1 {
            path,
            table,
            key,
            values,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_components(table, "TOML table")?;
            nonempty(key, "TOML key")?;
            validate_unique_nonempty(values, "TOML values")?;
        }
        RewriteRule::TypeScriptSpecCommentsStripV1 {
            patterns, exclude, ..
        }
        | RewriteRule::GoSpecDirectivesStripV1 {
            patterns, exclude, ..
        } => {
            validate_nonempty_globs(patterns, "rewrite.patterns")?;
            validate_globs(exclude, "rewrite.exclude")?;
            reject_git_selection(patterns, exclude)?;
        }
        RewriteRule::JsonMemberRemoveV1 {
            path,
            object,
            members,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_components(object, "JSON object")?;
            validate_unique_nonempty(members, "JSON members")?;
        }
        RewriteRule::TextExactReplaceV1 {
            path,
            sha256,
            before,
            occurrences,
            ..
        } => {
            PortablePath::parse(path)?;
            reject_git_literal(path, "rewrite.path")?;
            validate_digest(sha256, "rewrite.sha256")?;
            if before.is_empty() {
                return invalid("text exact-replace before string must be nonempty");
            }
            if *occurrences == 0 {
                return invalid("text exact-replace occurrences must be positive");
            }
        }
    }
    Ok(())
}

fn validate_assertion(assertion: &Assertion) -> Result<(), ScrapeError> {
    match assertion {
        Assertion::PathsAbsentV1 { patterns, .. } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])
        }
        Assertion::TextLiteralAbsentV1 {
            patterns, needles, ..
        } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])?;
            validate_unique_nonempty(needles, "assert.needles")
        }
        Assertion::CargoPathPrefixAbsentV1 {
            manifests,
            prefixes,
            ..
        } => {
            validate_nonempty_globs(manifests, "assert.manifests")?;
            reject_git_selection(manifests, &[])?;
            validate_unique_nonempty(prefixes, "assert.prefixes")
        }
        Assertion::LanguageMetadataAbsentV1 { patterns, .. } => {
            validate_nonempty_globs(patterns, "assert.patterns")?;
            reject_git_selection(patterns, &[])
        }
        Assertion::DependencyIdentitiesAbsentV1 {
            manifests,
            identities,
            ..
        } => {
            validate_nonempty_globs(manifests, "assert.manifests")?;
            reject_git_selection(manifests, &[])?;
            validate_unique_nonempty(identities, "assert.identities")
        }
    }
}

fn validate_healthcheck(check: &Healthcheck, baseline: BaselineMode) -> Result<(), ScrapeError> {
    let (root, timeout, when, network) = match check {
        Healthcheck::Cargo {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::Npm {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::Maven {
            root,
            timeout_seconds,
            when,
            network,
            ..
        }
        | Healthcheck::PythonPip {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (root, timeout_seconds, when, *network),
        Healthcheck::Custom {
            root,
            timeout_seconds,
            when,
            network,
            ..
        } => (root, timeout_seconds, when, Some(*network)),
    };
    validate_root(root)?;
    if *timeout == 0 {
        return invalid(format!(
            "healthcheck `{}` timeout must be positive",
            check.id()
        ));
    }
    if let Some(when) = when {
        PortablePath::parse(&when.path_exists)?;
    }
    if matches!(network, Some(NetworkPolicy::ToolOffline))
        && matches!(check, Healthcheck::Custom { .. })
    {
        return invalid("custom health cannot use tool-offline network policy");
    }
    match check {
        Healthcheck::Npm {
            build_script,
            typecheck_script,
            tests,
            test_script,
            lockfile,
            ..
        } => {
            PortablePath::parse(lockfile)?;
            if build_script.as_ref().is_some_and(|s| s.is_empty())
                || typecheck_script.as_ref().is_some_and(|s| s.is_empty())
            {
                return invalid("npm script names must be nonempty");
            }
            if build_script.is_some() == typecheck_script.is_some() {
                return invalid("npm requires exactly one build_script or typecheck_script");
            }
            validate_test_runner(*tests, test_script.as_deref(), "npm test_script")?;
        }
        Healthcheck::Maven { goal, .. } => nonempty(goal, "Maven goal")?,
        Healthcheck::PythonPip {
            interpreter,
            source_roots,
            tests,
            test_runner,
            ..
        } => {
            nonempty(interpreter, "Python interpreter")?;
            validate_unique_literals(source_roots, "Python source_roots")?;
            validate_test_runner(*tests, test_runner.as_deref(), "Python test_runner")?;
        }
        Healthcheck::Custom {
            source,
            snapshot,
            interpreter,
            argv,
            protocol,
            reads,
            writes,
            ..
        } => {
            PortablePath::parse(source)?;
            nonempty(interpreter, "custom interpreter")?;
            validate_nonempty_globs(snapshot, "custom snapshot")?;
            validate_globs(reads, "custom reads")?;
            validate_globs(writes, "custom writes")?;
            let mut contains_source = false;
            for pattern in snapshot {
                contains_source |= Glob::parse(pattern)?.matches(source);
            }
            if !contains_source {
                return invalid("custom snapshot must contain its source");
            }
            validate_argv(argv, *protocol)?;
            if baseline == BaselineMode::NoRegression
                && *protocol != CustomProtocol::VibeHealthJsonV1
            {
                return invalid("no-regression requires structured custom health");
            }
        }
        Healthcheck::Cargo { .. } => {}
    }
    if let Healthcheck::Cargo { features, .. } = check {
        validate_optional_unique_nonempty(features, "Cargo features")?;
    }
    if let Healthcheck::Custom { writes, .. } = check {
        reject_git_selection(writes, &[])?;
    }
    if baseline == BaselineMode::NoRegression
        && !matches!(
            check,
            Healthcheck::Custom {
                protocol: CustomProtocol::VibeHealthJsonV1,
                ..
            }
        )
    {
        return invalid("no-regression admits only structured custom healthchecks");
    }
    Ok(())
}

fn validate_argv(argv: &[String], protocol: CustomProtocol) -> Result<(), ScrapeError> {
    let allowed = ["{root}", "{phase}", "{scratch}", "{result}"];
    let result_count = argv.iter().filter(|arg| arg.as_str() == "{result}").count();
    for arg in argv {
        if (arg.contains('{') || arg.contains('}')) && !allowed.contains(&arg.as_str()) {
            return invalid(format!("invalid or embedded custom placeholder `{arg}`"));
        }
    }
    match protocol {
        CustomProtocol::VibeHealthJsonV1 if result_count != 1 => {
            invalid("JSON custom protocol requires {result} exactly once")
        }
        CustomProtocol::ExitCode if result_count != 0 => {
            invalid("exit-code custom protocol forbids {result}")
        }
        _ => Ok(()),
    }
}

fn validate_test_runner(
    mode: TestsMode,
    value: Option<&str>,
    field: &str,
) -> Result<(), ScrapeError> {
    if mode == TestsMode::Skip {
        if value.is_some() {
            return invalid(format!("{field} is forbidden when tests = skip"));
        }
    } else {
        nonempty(value.unwrap_or(""), field)?;
    }
    Ok(())
}

fn validate_relocation_graph(rows: &[Relocation]) -> Result<(), ScrapeError> {
    for (index, left) in rows.iter().enumerate() {
        for right in &rows[index + 1..] {
            if paths_overlap(&left.from, &right.from)
                || paths_overlap(&left.to, &right.to)
                || paths_overlap(&left.from, &right.to)
                || paths_overlap(&left.to, &right.from)
            {
                return invalid(format!(
                    "relocations `{}` and `{}` overlap",
                    left.id, right.id
                ));
            }
        }
        if left.to.starts_with(&(left.from.clone() + "/")) {
            return invalid(format!("relocation `{}` moves into itself", left.id));
        }
    }
    Ok(())
}

fn paths_overlap(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&(b.to_owned() + "/")) || b.starts_with(&(a.to_owned() + "/"))
}

fn reject_git_selection(patterns: &[String], excludes: &[String]) -> Result<(), ScrapeError> {
    let excluded_all = excludes.iter().any(|p| p == ".git/**" || p == "**");
    for pattern in patterns {
        let glob = Glob::parse(pattern)?;
        if glob.can_match_git() && !excluded_all {
            return invalid(format!(
                "pattern `{pattern}` can select protected .git internals without a complete .git/** exclusion"
            ));
        }
    }
    Ok(())
}

fn validate_nonempty_globs(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    validate_globs(values, field)
}

fn validate_globs(values: &[String], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        Glob::parse(value)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry `{value}`"));
        }
    }
    Ok(())
}

fn validate_unique_literals(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        PortablePath::parse(value)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry `{value}`"));
        }
    }
    Ok(())
}

fn validate_component_paths(values: &[Vec<String>], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_components(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} entry"));
        }
    }
    Ok(())
}

fn validate_components(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    for value in values {
        if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\', ':']) {
            return invalid(format!("invalid {field} component `{value}`"));
        }
    }
    Ok(())
}

fn validate_unique_nonempty(values: &[String], field: &str) -> Result<(), ScrapeError> {
    if values.is_empty() {
        return invalid(format!("{field} must be nonempty"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        nonempty(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} value `{value}`"));
        }
    }
    Ok(())
}

fn validate_optional_unique_nonempty(values: &[String], field: &str) -> Result<(), ScrapeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonempty(value, field)?;
        if !seen.insert(value) {
            return invalid(format!("duplicate {field} value `{value}`"));
        }
    }
    Ok(())
}

fn reject_git_literals(values: &[String], field: &str) -> Result<(), ScrapeError> {
    for value in values {
        reject_git_literal(value, field)?;
    }
    Ok(())
}

fn reject_git_literal(value: &str, field: &str) -> Result<(), ScrapeError> {
    if value == ".git" || value.starts_with(".git/") {
        invalid(format!("{field} addresses protected .git metadata"))
    } else {
        Ok(())
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
mod tests {
    use super::*;

    fn minimal() -> String {
        r#"schema = 1
id = "org.example.scrape"
[policy]
unclassified = "refuse"
links = "refuse"
concurrent_change = "refuse"
[scope]
closed_roots = ["vibevm"]
outside = "implicit-keep"
[commit]
contract = "delete-last"
[[classify]]
id = "delete"
kind = "delete"
patterns = ["vibevm", "vibevm/**"]
owner = "vibe"
proof = "contract-assertion-v1"
modified = "delete"
require_match = true
[[assert]]
id = "absent"
kind = "paths-absent-v1"
patterns = ["vibevm", "vibevm/**"]
[health]
baseline = "strict"
before_failure = "refuse"
after_failure = "rollback"
parallel = false
network = "tool-offline"
max_stdout_bytes = 1
max_stderr_bytes = 1
max_result_bytes = 1
termination_grace_seconds = 1
[[healthcheck]]
id = "cargo"
kind = "cargo"
root = "."
build = "check"
workspace = true
locked = true
all_targets = true
tests = "skip"
profile = "dev"
features = []
timeout_seconds = 1
"#
        .to_owned()
    }

    #[test]
    fn strict_shape_rejects_unknown_wrong_schema_and_duplicate_global_id() {
        assert!(Contract::parse(minimal().replace("schema = 1", "schema = 2").as_bytes()).is_err());
        assert!(
            Contract::parse(
                minimal()
                    .replace(
                        "id = \"org.example.scrape\"",
                        "id = \"org.example.scrape\"\nunknown = true"
                    )
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            Contract::parse(
                minimal()
                    .replace("id = \"absent\"", "id = \"delete\"")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            Contract::parse(
                minimal()
                    .replace("kind = \"cargo\"", "kind = \"unknown\"")
                    .as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn all_rewrite_assertion_and_health_variants_parse() {
        let mut text = minimal();
        let insert = r#"
[[rewrite]]
id = "managed"
kind = "managed-block-remove-v1"
paths = ["AGENTS.md"]
marker = "vibevm"
matches = "zero-or-one-per-file"
[[rewrite]]
id = "rust"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope", "spec", "verifies", "cell"]
matches = "zero-or-more"
[[rewrite]]
id = "cargo-rw"
kind = "cargo-package-remove-v1"
manifests = ["Cargo.toml"]
package = "core-ai-native-specmark"
aliases = []
matches = "zero-or-more"
[[rewrite]]
id = "node"
kind = "node-package-remove-v1"
package_json = "web/package.json"
lockfile = "web/package-lock.json"
manager = "npm"
packages = ["vibe"]
script_paths = [["scripts", "vibe"]]
config_paths = []
matches = "exactly-one"
[[rewrite]]
id = "gomod"
kind = "go-module-remove-v1"
go_mod = "go.mod"
go_sum = "go.sum"
modules = ["example.org/vibe"]
matches = "one-or-more"
[[rewrite]]
id = "toml"
kind = "toml-array-values-remove-v1"
path = "Cargo.toml"
table = ["workspace"]
key = "exclude"
values = ["vibevm"]
matches = "zero-or-more"
[[rewrite]]
id = "ts"
kind = "typescript-spec-comments-strip-v1"
patterns = ["src/**/*.ts"]
matches = "zero-or-more"
[[rewrite]]
id = "go"
kind = "go-spec-directives-strip-v1"
patterns = ["src/**/*.go"]
matches = "zero-or-more"
[[rewrite]]
id = "json"
kind = "json-member-remove-v1"
path = "package.json"
object = ["scripts"]
members = ["vibe"]
matches = "exactly-one"
[[rewrite]]
id = "text"
kind = "text-exact-replace-v1"
path = "Makefile"
sha256 = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
before = "vibe"
after = "native"
occurrences = 1
[[relocate]]
id = "move"
from = "vibevm/vibespecs"
to = "docs/specs"
conflict = "refuse"
required = false
[[assert]]
id = "text-absent"
kind = "text-literal-absent-v1"
patterns = ["src/**"]
needles = ["vibe"]
[[assert]]
id = "cargo-path"
kind = "cargo-path-prefix-absent-v1"
manifests = ["Cargo.toml"]
prefixes = ["vibevm/"]
[[assert]]
id = "metadata"
kind = "language-metadata-absent-v1"
language = "rust"
patterns = ["src/**/*.rs"]
[[assert]]
id = "deps"
kind = "dependency-identities-absent-v1"
manager = "cargo"
manifests = ["Cargo.toml"]
identities = ["core-ai-native-specmark"]
[[healthcheck]]
id = "npm"
kind = "npm"
root = "web"
manager = "npm"
lockfile = "package-lock.json"
install = "none"
build_script = "build"
tests = "required"
test_script = "test"
timeout_seconds = 1
[[healthcheck]]
id = "maven"
kind = "maven"
root = "java"
runner = "wrapper-first"
goal = "verify"
offline = true
tests = "required"
timeout_seconds = 1
[[healthcheck]]
id = "python"
kind = "python-pip"
root = "python"
interpreter = "python"
source_roots = ["src"]
dependency_check = true
build = true
tests = "required"
test_runner = "pytest"
timeout_seconds = 1
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "tools/health.py"
snapshot = ["tools/health.py"]
interpreter = "python"
argv = ["{phase}", "{result}"]
protocol = "vibe-health-json-v1"
reads = ["**"]
writes = []
spawn = true
network = "deny"
timeout_seconds = 1
"#;
        text.push_str(insert);
        let parsed = Contract::parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.rewrite.len(), 10);
        assert_eq!(parsed.assertions.len(), 5);
        assert_eq!(parsed.healthcheck.len(), 5);
    }

    #[test]
    fn semantic_reds_refuse_git_bad_cardinality_and_protocol() {
        assert!(
            Contract::parse(
                minimal()
                    .replace(
                        "patterns = [\"vibevm\", \"vibevm/**\"]",
                        "patterns = [\"**\"]"
                    )
                    .as_bytes()
            )
            .is_err()
        );
        let bad = minimal()
            + r#"
[[rewrite]]
id = "rust"
kind = "rust-specmark-strip-v1"
patterns = ["src/**/*.rs"]
forms = ["scope"]
matches = "exactly-one"
"#;
        assert!(Contract::parse(bad.as_bytes()).is_err());
        let bad = minimal()
            + r#"
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = ["prefix-{result}"]
protocol = "vibe-health-json-v1"
reads = []
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#;
        assert!(Contract::parse(bad.as_bytes()).is_err());
    }

    #[test]
    fn custom_health_requires_explicit_reads_and_writes() {
        let base = minimal()
            + r#"
[[healthcheck]]
id = "custom"
kind = "custom"
root = "."
source = "health.py"
snapshot = ["health.py"]
interpreter = "python"
argv = []
protocol = "exit-code"
reads = ["**"]
writes = []
spawn = false
network = "deny"
timeout_seconds = 1
"#;
        assert!(Contract::parse(base.replace("reads = [\"**\"]\n", "").as_bytes()).is_err());
        assert!(Contract::parse(base.replace("writes = []\n", "").as_bytes()).is_err());
    }

    #[test]
    fn health_caps_grace_and_unconditional_row_are_explicit() {
        let base = minimal();
        assert!(Contract::parse(base.replace("max_result_bytes = 1\n", "").as_bytes()).is_err());
        assert!(
            Contract::parse(
                base.replace("termination_grace_seconds = 1\n", "")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            Contract::parse(
                base.replace("max_result_bytes = 1", "max_result_bytes = 0")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            Contract::parse(
                base.replace(
                    "max_stdout_bytes = 1",
                    &format!("max_stdout_bytes = {}", MAX_HEALTH_STREAM_BYTES + 1),
                )
                .as_bytes()
            )
            .is_err()
        );
        assert!(
            Contract::parse((base + "when = { path_exists = \"Cargo.toml\" }\n").as_bytes())
                .is_err()
        );
    }
}
