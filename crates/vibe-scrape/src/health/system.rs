//! Stable, read-only system resolver used by ordinary scrape planning.
//!
//! It never invokes a command. `version` is therefore an explicit content
//! identity rather than an unconfined `--version` child; an enforcing backend
//! may later replace it with a bounded, journaled probe.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::path::{Path, PathBuf};

use vibe_safefs::Project;

use super::model::*;
use super::prepare::HealthResolver;

mod discovery;

const ASSET_CAP: usize = 64 * 1024 * 1024;
const MANIFEST_CAP: usize = 4 * 1024 * 1024;

pub struct SystemHealthResolver {
    root: PathBuf,
    node_parent: Option<PathBuf>,
    cargo_parent: Option<PathBuf>,
}

impl SystemHealthResolver {
    #[must_use]
    pub fn new(project: &Project) -> Self {
        Self {
            root: project.root_path().to_path_buf(),
            node_parent: None,
            cargo_parent: None,
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
            version: format!("content:{}", sha256),
            version_kind: VersionKind::Content,
            sha256,
            source,
            live_identity: Some(snapshot.identity),
        };
        Ok(identity)
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
        if matches!(request.role, AssetRole::Rustc | AssetRole::Rustdoc) {
            let parent = self.cargo_parent.as_ref().ok_or_else(|| {
                HealthError::Preparation(
                    "Cargo companion tool resolution requires sealed Cargo first".to_owned(),
                )
            })?;
            let path =
                parent.join(command_names(&selector).into_iter().next().ok_or_else(|| {
                    HealthError::Preparation("Cargo companion selector is empty".to_owned())
                })?);
            return self.resolve_at(request, path, AssetSource::Resolved);
        }
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
        if asset.role == AssetRole::Cargo {
            self.cargo_parent = path.parent().map(Path::to_path_buf);
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
