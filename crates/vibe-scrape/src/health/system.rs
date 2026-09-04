//! Stable, read-only system resolver used by ordinary scrape planning.
//!
//! It never invokes a command. `version` is therefore an explicit content
//! identity rather than an unconfined `--version` child; an enforcing backend
//! may later replace it with a bounded, journaled probe.

use std::path::{Path, PathBuf};

use vibe_safefs::Project;

use super::model::*;
use super::prepare::HealthResolver;

const ASSET_CAP: usize = 64 * 1024 * 1024;
const MANIFEST_CAP: usize = 4 * 1024 * 1024;

pub struct SystemHealthResolver {
    root: PathBuf,
    node_parent: Option<PathBuf>,
}

impl SystemHealthResolver {
    #[must_use]
    pub fn new(project: &Project) -> Self {
        Self {
            root: project.root_path().to_path_buf(),
            node_parent: None,
        }
    }

    fn project(&self) -> Result<Project, HealthError> {
        Project::open(&self.root).map_err(|error| {
            HealthError::Preparation(format!("reopening project capability: {error:#}"))
        })
    }

    fn command_candidates(&self, selector: &str) -> Result<Vec<PathBuf>, HealthError> {
        let selected = Path::new(selector);
        if selected.is_absolute() {
            return Ok(vec![selected.to_path_buf()]);
        }
        if selected.components().count() != 1 {
            return Err(HealthError::Preparation(format!(
                "relative executable selector `{selector}` is not one PATH token"
            )));
        }
        let path = std::env::var_os("PATH").ok_or_else(|| {
            HealthError::Preparation("PATH is absent while resolving health tools".to_owned())
        })?;
        let names = command_names(selector);
        let mut candidates = Vec::new();
        for directory in std::env::split_paths(&path) {
            if !directory.is_absolute() {
                return Err(HealthError::Preparation(format!(
                    "PATH contains relative entry `{}`",
                    directory.display()
                )));
            }
            for name in &names {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    reject_script_launcher(&candidate)?;
                    candidates.push(candidate);
                }
            }
        }
        if candidates.is_empty() {
            Err(HealthError::Preparation(format!(
                "health executable `{selector}` is unavailable on PATH"
            )))
        } else {
            Ok(candidates)
        }
    }

    fn resolve_at(
        &self,
        request: ResolveAssetRequest,
        path: PathBuf,
        source: AssetSource,
    ) -> Result<AssetIdentity, HealthError> {
        reject_script_launcher(&path)?;
        let project = self.project()?;
        let pinned = Project::pin_absolute_file(&path).map_err(|error| {
            HealthError::Preparation(format!(
                "pinning health asset `{}` no-follow: {error:#}",
                path.display()
            ))
        })?;
        let snapshot = pinned
            .read_snapshot_bounded(&project, ASSET_CAP)
            .map_err(|error| {
                HealthError::Preparation(format!(
                    "reading health asset `{}` stably: {error:#}",
                    path.display()
                ))
            })?;
        let sha256 = format!("sha256:{}", snapshot.sha256);
        if request.role == AssetRole::MavenLauncher && snapshot.bytes.starts_with(b"#!") {
            return Err(HealthError::Unsupported(format!(
                "Maven launcher `{}` is a shebang script; implicit interpreter selection is forbidden",
                path.display()
            )));
        }
        let identity = AssetIdentity {
            id: request.id,
            role: request.role,
            display_path: portable_display(&path),
            bytes: snapshot.size,
            mode: snapshot.unix_mode,
            // FileIdentity is intentionally opaque and has no stable wire
            // token. Security comparison uses `live_identity`; no Debug/raw
            // platform layout is recreated for JSON.
            platform_identity: "opaque-live-only".to_owned(),
            version: String::new(),
            sha256,
            source,
            live_identity: Some(snapshot.identity),
        };
        Err(HealthError::Unsupported(format!(
            "bounded journaled version probe is unavailable for `{}` ({})",
            identity.display_path, identity.sha256
        )))
    }

    fn npm_cli(&self) -> Result<PathBuf, HealthError> {
        let parent = self.node_parent.as_ref().ok_or_else(|| {
            HealthError::Preparation("npm CLI resolution requires sealed Node first".to_owned())
        })?;
        [
            parent.join("node_modules/npm/bin/npm-cli.js"),
            parent.join("../node_modules/npm/bin/npm-cli.js"),
        ]
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            HealthError::Preparation(format!(
                "no npm-cli.js asset is adjacent to sealed Node `{}`",
                parent.display()
            ))
        })
    }

    fn resolve_command_asset(
        &self,
        request: ResolveAssetRequest,
        selector: &str,
    ) -> Result<(AssetIdentity, PathBuf), HealthError> {
        let mut last_error = None;
        for path in self.command_candidates(selector)? {
            match self.resolve_at(request.clone(), path.clone(), AssetSource::Resolved) {
                Ok(asset) => return Ok((asset, path)),
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            HealthError::Preparation(format!("no stable `{selector}` candidate was resolved"))
        }))
    }
}

impl HealthResolver for SystemHealthResolver {
    fn resolve_asset(
        &mut self,
        request: ResolveAssetRequest,
    ) -> Result<AssetIdentity, HealthError> {
        if request.role == AssetRole::MavenLauncher {
            if request.selector == "maven-wrapper-first" {
                return Err(HealthError::Unsupported(
                    "Maven wrapper-first requires sealed wrapper metadata, wrapper JAR, and Java assets; no complete chain is available"
                        .to_owned(),
                ));
            }
            if cfg!(windows) {
                return Err(HealthError::Unsupported(
                    "Windows explicit Maven needs a native Java/launcher chain; mvn.cmd reparsing is forbidden"
                        .to_owned(),
                ));
            }
        }
        let selector = request.selector.clone();
        let (asset, path) = match request.role {
            AssetRole::NpmCli => {
                let path = self.npm_cli()?;
                let asset = self.resolve_at(request, path.clone(), AssetSource::Resolved)?;
                (asset, path)
            }
            AssetRole::MavenLauncher => self.resolve_command_asset(request, "mvn")?,
            _ => self.resolve_command_asset(request, &selector)?,
        };
        if asset.role == AssetRole::Node {
            self.node_parent = path.parent().map(Path::to_path_buf);
        }
        Ok(asset)
    }

    fn resolve_custom_launch(
        &mut self,
        check_id: &str,
        interpreter: &str,
        source: &str,
    ) -> Result<ResolvedCustomLaunch, HealthError> {
        if interpreter == "direct" {
            let request = ResolveAssetRequest {
                id: format!("{check_id}/custom-launch"),
                role: AssetRole::CustomNative,
                selector: source.to_owned(),
            };
            let path = self
                .root
                .join(source.replace('/', std::path::MAIN_SEPARATOR_STR));
            let asset = self.resolve_at(
                request,
                path,
                AssetSource::Bundle {
                    path: source.to_owned(),
                },
            )?;
            return Ok(ResolvedCustomLaunch {
                asset,
                style: CustomLaunchStyle::Direct,
            });
        }
        let request = ResolveAssetRequest {
            id: format!("{check_id}/custom-launch"),
            role: AssetRole::CustomInterpreter,
            selector: interpreter.to_owned(),
        };
        let (asset, _) = self.resolve_command_asset(request, interpreter)?;
        Ok(ResolvedCustomLaunch {
            asset,
            style: CustomLaunchStyle::Interpreter,
        })
    }

    fn discover_tests(
        &mut self,
        project: &Project,
        inventory: &crate::model::Inventory,
        request: &TestDiscoveryRequest,
    ) -> Result<TestPresence, HealthError> {
        let under_root = |path: &str| {
            request.root == "."
                || path == request.root
                || path.starts_with(&(request.root.clone() + "/"))
        };
        match request.kind {
            HealthcheckKind::Cargo => {
                let tree = super::discovery::DiscoveryTree::before(project, inventory)?;
                super::discovery::cargo_tests(&tree, request)
            }
            HealthcheckKind::Npm => {
                let package_json = rooted(&request.root, "package.json");
                let bytes = project
                    .read_file_bounded(&package_json, MANIFEST_CAP)
                    .map_err(|error| {
                        HealthError::Preparation(format!(
                            "reading `{package_json}` for npm test discovery: {error:#}"
                        ))
                    })?
                    .ok_or_else(|| {
                        HealthError::Preparation(format!("npm manifest `{package_json}` is absent"))
                    })?;
                super::protocol::reject_duplicate_keys(&bytes)?;
                let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
                    HealthError::Preparation(format!("invalid `{package_json}`: {error}"))
                })?;
                let selector = request.selector.as_deref().ok_or_else(|| {
                    HealthError::Preparation("npm test discovery has no declared script".to_owned())
                })?;
                Ok(
                    if value
                        .get("scripts")
                        .and_then(serde_json::Value::as_object)
                        .is_some_and(|scripts| scripts.get(selector).is_some_and(|v| v.is_string()))
                    {
                        TestPresence::Present
                    } else {
                        TestPresence::Absent
                    },
                )
            }
            HealthcheckKind::Maven => {
                let tree = super::discovery::DiscoveryTree::before(project, inventory)?;
                super::discovery::maven_tests(&tree, request)
            }
            HealthcheckKind::PythonPip => {
                if request.selector.as_deref() != Some("pytest") {
                    return Ok(TestPresence::Indeterminate);
                }
                Ok(
                    if inventory.entries.iter().any(|entry| {
                        if !under_root(&entry.path) || !entry.path.ends_with(".py") {
                            return false;
                        }
                        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
                        name.starts_with("test_") || name.ends_with("_test.py")
                    }) {
                        TestPresence::Present
                    } else {
                        TestPresence::Absent
                    },
                )
            }
            HealthcheckKind::Custom => Ok(TestPresence::Indeterminate),
        }
    }
}

#[allow(dead_code)]
fn cargo_test_presence(
    project: &Project,
    inventory: &crate::model::Inventory,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let root_manifest = rooted(&request.root, "Cargo.toml");
    let root = read_toml(project, &root_manifest)?;
    let mut package_roots = Vec::new();
    if root.get("package").is_some() && !request.workspace {
        package_roots.push(request.root.clone());
    } else if let Some(workspace) = root.get("workspace").and_then(toml::Value::as_table) {
        let key = if request.workspace || root.get("package").is_none() {
            "members"
        } else {
            "default-members"
        };
        let Some(members) = workspace.get(key).and_then(toml::Value::as_array) else {
            return Ok(TestPresence::Indeterminate);
        };
        for member in members {
            let Some(member) = member.as_str() else {
                return Ok(TestPresence::Indeterminate);
            };
            let Some(expanded) = expand_cargo_member(&request.root, member, inventory) else {
                return Ok(TestPresence::Indeterminate);
            };
            package_roots.extend(expanded);
        }
        if request.workspace && root.get("package").is_some() {
            package_roots.push(request.root.clone());
        }
    } else {
        return Ok(TestPresence::Indeterminate);
    }
    package_roots.sort();
    package_roots.dedup();
    for package_root in package_roots {
        let manifest = read_toml(project, &rooted(&package_root, "Cargo.toml"))?;
        match cargo_package_test_target(&manifest, &package_root, inventory, request.all_targets) {
            TestPresence::Present => return Ok(TestPresence::Present),
            TestPresence::Indeterminate => return Ok(TestPresence::Indeterminate),
            TestPresence::Absent => {}
        }
    }
    Ok(TestPresence::Absent)
}

fn cargo_package_test_target(
    manifest: &toml::Value,
    root: &str,
    inventory: &crate::model::Inventory,
    all_targets: bool,
) -> TestPresence {
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return TestPresence::Indeterminate;
    };
    let target_enabled = |table: &toml::value::Table| {
        table
            .get("test")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true)
    };
    let explicit_enabled = |key: &str| {
        manifest
            .get(key)
            .and_then(toml::Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .filter_map(toml::Value::as_table)
                    .any(&target_enabled)
            })
    };
    if manifest
        .get("lib")
        .and_then(toml::Value::as_table)
        .is_some_and(&target_enabled)
        || (manifest.get("lib").is_none() && inventory_file(inventory, &rooted(root, "src/lib.rs")))
        || explicit_enabled("bin")
        || explicit_enabled("test")
    {
        return TestPresence::Present;
    }
    let autobins = package
        .get("autobins")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autobins
        && (inventory_file(inventory, &rooted(root, "src/main.rs"))
            || files_below(inventory, &rooted(root, "src/bin"), ".rs"))
    {
        return TestPresence::Present;
    }
    let autotests = package
        .get("autotests")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autotests && files_below(inventory, &rooted(root, "tests"), ".rs") {
        return TestPresence::Present;
    }
    if all_targets && (explicit_enabled("example") || explicit_enabled("bench")) {
        return TestPresence::Present;
    }
    TestPresence::Absent
}

fn expand_cargo_member(
    workspace_root: &str,
    member: &str,
    inventory: &crate::model::Inventory,
) -> Option<Vec<String>> {
    if member.contains("**") || member.contains(['?', '[', ']', '{', '}', '\\', ':']) {
        return None;
    }
    if let Some(prefix) = member.strip_suffix("/*") {
        let prefix = rooted(workspace_root, prefix);
        let mut roots = inventory
            .entries
            .iter()
            .filter(|entry| {
                entry.kind == crate::model::EntryKind::Directory
                    && entry.path.starts_with(&(prefix.clone() + "/"))
                    && !entry.path[(prefix.len() + 1)..].contains('/')
            })
            .map(|entry| entry.path.clone())
            .filter(|root| inventory_file(inventory, &rooted(root, "Cargo.toml")))
            .collect::<Vec<_>>();
        roots.sort();
        Some(roots)
    } else if member.contains('*') {
        None
    } else {
        Some(vec![rooted(workspace_root, member)])
    }
}

fn read_toml(project: &Project, path: &str) -> Result<toml::Value, HealthError> {
    let bytes = project
        .read_file_bounded(path, MANIFEST_CAP)
        .map_err(|error| HealthError::Preparation(format!("reading `{path}`: {error:#}")))?
        .ok_or_else(|| HealthError::Preparation(format!("manifest `{path}` is absent")))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| HealthError::Preparation(format!("`{path}` is not UTF-8: {error}")))?;
    toml::from_str(text)
        .map_err(|error| HealthError::Preparation(format!("invalid `{path}`: {error}")))
}

fn inventory_file(inventory: &crate::model::Inventory, path: &str) -> bool {
    inventory
        .entries
        .iter()
        .any(|entry| entry.path == path && entry.kind == crate::model::EntryKind::File)
}

fn files_below(inventory: &crate::model::Inventory, root: &str, suffix: &str) -> bool {
    inventory.entries.iter().any(|entry| {
        entry.kind == crate::model::EntryKind::File
            && entry.path.starts_with(&(root.to_owned() + "/"))
            && entry.path.ends_with(suffix)
    })
}

#[allow(dead_code)]
fn maven_test_presence(
    project: &Project,
    inventory: &crate::model::Inventory,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let config = rooted(&request.root, ".mvn/maven.config");
    if inventory_file(inventory, &config) {
        let bytes = project
            .read_file_bounded(&config, MANIFEST_CAP)
            .map_err(|error| {
                HealthError::Preparation(format!("reading Maven config `{config}`: {error:#}"))
            })?
            .ok_or_else(|| HealthError::Preparation(format!("Maven config `{config}` vanished")))?;
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            HealthError::Preparation(format!("Maven config `{config}` is not UTF-8: {error}"))
        })?;
        if text.split_whitespace().any(|arg| {
            matches!(
                arg,
                "-DskipTests" | "-DskipTests=true" | "-Dmaven.test.skip=true"
            )
        }) {
            return Ok(TestPresence::Absent);
        }
        if text
            .split_whitespace()
            .any(|arg| arg.starts_with("-P") || arg.contains("testSourceDirectory"))
        {
            return Ok(TestPresence::Indeterminate);
        }
    }

    let mut modules = vec![request.root.clone()];
    let mut index = 0;
    while index < modules.len() {
        let module_root = modules[index].clone();
        index += 1;
        let pom = rooted(&module_root, "pom.xml");
        let bytes = project
            .read_file_bounded(&pom, MANIFEST_CAP)
            .map_err(|error| {
                HealthError::Preparation(format!("reading Maven model `{pom}`: {error:#}"))
            })?
            .ok_or_else(|| HealthError::Preparation(format!("Maven model `{pom}` is absent")))?;
        let model = parse_maven_model(&bytes)?;
        if model.indeterminate {
            return Ok(TestPresence::Indeterminate);
        }
        if !model.tests_skipped {
            let sources = if model.test_sources.is_empty() {
                vec!["src/test/java".to_owned()]
            } else {
                model.test_sources
            };
            if sources.iter().any(|source| {
                !source.contains("${")
                    && inventory.entries.iter().any(|entry| {
                        let root = rooted(&module_root, source);
                        entry.kind == crate::model::EntryKind::File
                            && entry.path.starts_with(&(root + "/"))
                    })
            }) {
                return Ok(TestPresence::Present);
            }
        }
        for module in model.modules {
            if module.is_empty()
                || module.contains("${")
                || module.contains(['\\', ':'])
                || module
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
            {
                return Ok(TestPresence::Indeterminate);
            }
            modules.push(rooted(&module_root, &module));
        }
    }
    Ok(TestPresence::Absent)
}

#[derive(Default)]
struct MavenModel {
    modules: Vec<String>,
    test_sources: Vec<String>,
    tests_skipped: bool,
    indeterminate: bool,
}

fn parse_maven_model(bytes: &[u8]) -> Result<MavenModel, HealthError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<String>::new();
    let mut model = MavenModel::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                if name == "profiles" {
                    model.indeterminate = true;
                }
                stack.push(name);
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Text(text)) => {
                let value = text.decode().map_err(|error| {
                    HealthError::Preparation(format!("decoding Maven model text: {error}"))
                })?;
                match stack.last().map(String::as_str) {
                    Some("module") => model.modules.push(value.into_owned()),
                    Some("testSourceDirectory") => model.test_sources.push(value.into_owned()),
                    Some("skipTests" | "maven.test.skip") if value.eq_ignore_ascii_case("true") => {
                        model.tests_skipped = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(HealthError::Preparation(format!(
                    "invalid Maven POM XML: {error}"
                )));
            }
        }
    }
    model.modules.sort();
    model.modules.dedup();
    model.test_sources.sort();
    model.test_sources.dedup();
    Ok(model)
}

fn command_names(selector: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        if Path::new(selector).extension().is_some() {
            vec![selector.to_owned()]
        } else {
            vec![format!("{selector}.exe")]
        }
    }
    #[cfg(not(windows))]
    {
        vec![selector.to_owned()]
    }
}

fn reject_script_launcher(path: &Path) -> Result<(), HealthError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("cmd")
        || extension.eq_ignore_ascii_case("bat")
        || extension.eq_ignore_ascii_case("ps1")
    {
        Err(HealthError::Unsupported(format!(
            "script launcher `{}` would require command reparsing",
            path.display()
        )))
    } else {
        Ok(())
    }
}

fn rooted(root: &str, path: &str) -> String {
    if root == "." {
        path.to_owned()
    } else {
        format!("{root}/{path}")
    }
}

fn portable_display(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
