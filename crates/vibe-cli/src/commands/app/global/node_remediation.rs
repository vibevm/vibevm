//! Explicit scoped Node provider; never install from arbitrary command text.
use super::*;
use std::ffi::OsString;
use std::process::Command;
use vibe_app::{CommandSearch, InstallPreparation, Location, NodeRemediation, Storage};

pub(super) struct RuntimePath {
    pub path: OsString,
}

pub(super) fn prepare_provider(
    ctx: &output::Context,
    packages: &Packages,
    operation: &OperationContext<'_>,
    target: &ResolvedPackage,
    token: &NodeRemediation,
    assume_yes: bool,
) -> Result<RuntimePath> {
    let binding = token.runtime();
    let minimum = binding.minimum_node().map_err(anyhow::Error::msg)?;
    let index = vibe_app::Index::load(operation.settings)?;
    let existing = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == "org.vibevm.node" && record.package.name == "node"
        })
        .filter_map(|(key, record)| {
            let version = semver::Version::parse(&record.package.version).ok()?;
            binding
                .accepts_node(&version)
                .ok()
                .filter(|v| *v)
                .map(|_| (version, key.clone()))
        })
        .max_by(|a, b| a.0.cmp(&b.0));
    // Resolve before consent so the operator sees the actual supplier/version.
    let provider = if existing.is_none() {
        let spelling = format!("org.vibevm.node/node@>={minimum}");
        let resolved = packages.resolve(&spelling)?;
        let version = semver::Version::parse(&resolved.package.version)?;
        if !binding.accepts_node(&version).map_err(anyhow::Error::msg)? {
            bail!("selected Node supplier version does not satisfy the verified runtime range");
        }
        Some(resolved)
    } else {
        None
    };
    let selected = existing
        .as_ref()
        .map(|(_, key)| key.clone())
        .or_else(|| provider.as_ref().map(|p| p.package.key()))
        .ok_or_else(|| {
            anyhow::anyhow!("no Node supplier was selected; target was not installed")
        })?;
    if !assume_yes {
        if !attended(ctx) {
            bail!(
                "install-node requires explicit --assume-yes in JSON/unattended execution; target was not installed"
            );
        }
        let source = provider
            .as_ref()
            .map(|p| p.source.registry.as_deref().unwrap_or("admitted registry"))
            .unwrap_or("existing managed installation");
        let accepted = asking(|| {
            output::prompts::confirm(
                &format!(
                    "Use {selected} from {source} as a scoped Node provider? No global default changes; a newly installed provider remains if target installation fails"
                ),
                false,
            )
        })?;
        if !accepted {
            bail!("Node provider selection cancelled; target was not installed");
        }
    }
    // Manual defer never reaches this branch. Only explicit automatic selection
    // may acquire and verify the pinned npm input, before any provider mutation.
    token.verify_runtime_input(operation, target)?;
    let key = if let Some(resolved) = &provider {
        let request = InstallRequest {
            hidden: true,
            recommends: false,
            ..Default::default()
        };
        let provider_ctx = OperationContext {
            missing_command: None,
            ..*operation
        };
        vibe_app::install(&provider_ctx, resolved, &request)?.key
    } else {
        selected
    };
    let index = vibe_app::Index::load(operation.settings)?;
    let record = index
        .installations
        .get(&key)
        .context("selected Node receipt is missing")?;
    let storage = Storage::load(operation.settings)?;
    let root = match &record.location {
        Location::Owned { library, directory } => storage
            .path_of(library)
            .context("Node library is missing")?
            .join(directory),
        Location::External { .. } => {
            bail!("automatic Node remediation requires an owned verified provider")
        }
    };
    let mut dirs = vibe_app::slot_session_dirs(operation.settings, &index, &storage, &key)?;
    if dirs.is_empty() && root.join("bin").is_dir() {
        dirs.push(root.join("bin"));
    }
    let path = std::env::join_paths(dirs.into_iter().chain(std::env::split_paths(
        operation.search_path.unwrap_or(std::ffi::OsStr::new("")),
    )))?;
    if path.to_str().is_none() {
        bail!("scoped runtime PATH cannot be represented losslessly for target steps");
    }
    let search = CommandSearch {
        path: Some(path.clone()),
        pathext: std::env::var_os("PATHEXT"),
        exclude: None,
    };
    let canonical_root = std::fs::canonicalize(&root)?;
    let node = search.locate("node").map_err(anyhow::Error::msg)?;
    let npm = search.locate("npm").map_err(anyhow::Error::msg)?;
    if !node.starts_with(&canonical_root) || !npm.starts_with(&canonical_root) {
        bail!("runtime PATH is shadowed by an executable outside the selected Node provider");
    }
    let version = Command::new(&node)
        .arg("--version")
        .env("PATH", &path)
        .output()?;
    if !version.status.success() {
        bail!("selected Node executable version check failed");
    }
    let actual = semver::Version::parse(
        String::from_utf8_lossy(&version.stdout)
            .trim()
            .trim_start_matches('v'),
    )?;
    if actual.to_string() != record.package.version
        || !binding.accepts_node(&actual).map_err(anyhow::Error::msg)?
    {
        bail!("actual Node version differs from selected receipt or required range");
    }
    let npm_launcher = record
        .launchers
        .iter()
        .find(|l| l.id == "npm")
        .context("selected Node provider has no npm launcher")?;
    let launcher = vibe_app::resolve_launcher(
        npm_launcher,
        &vibe_app::TemplateVars {
            app_root: root,
            vibe_home: operation.settings.path().to_path_buf(),
            project_root: None,
        },
    )
    .map_err(anyhow::Error::msg)?;
    if launcher.executable != node {
        bail!("npm launcher does not use the selected Node executable");
    }
    let npm_version = Command::new(&launcher.executable)
        .args(&launcher.args)
        .arg("--version")
        .env("PATH", &path)
        .output()?;
    if !npm_version.status.success()
        || semver::Version::parse(String::from_utf8_lossy(&npm_version.stdout).trim()).is_err()
    {
        bail!("selected npm version check failed");
    }
    let bare_npm_version = Command::new(&npm)
        .arg("--version")
        .env("PATH", &path)
        .output()?;
    if !bare_npm_version.status.success() || bare_npm_version.stdout != npm_version.stdout {
        bail!("npm selected by target PATH differs from the verified receipt launcher");
    }
    if let Some(script) = launcher.args.first() {
        let expected_script = std::fs::canonicalize(script)?;
        if !cfg!(windows) && npm != expected_script {
            bail!("npm PATH entry is not the selected provider's declared npm script");
        }
    } else {
        bail!("selected npm launcher has no witnessed script identity");
    }
    Ok(RuntimePath { path })
}

pub(super) fn run(
    ctx: &output::Context,
    packages: &Packages,
    operation: &OperationContext<'_>,
    target: &ResolvedPackage,
    request: &InstallRequest,
    assume_yes: bool,
) -> Result<vibe_app::Outcome> {
    match vibe_app::install_preparing_node(operation, target, request)? {
        InstallPreparation::Completed(outcome) => Ok(outcome),
        InstallPreparation::NeedsNode(token) => {
            let runtime = prepare_provider(ctx, packages, operation, target, &token, assume_yes)?;
            let search = CommandSearch {
                path: Some(runtime.path.clone()),
                pathext: std::env::var_os("PATHEXT"),
                exclude: None,
            };
            let present = |name: &str| search.locate(name).is_ok();
            let current = OperationContext {
                search_path: Some(&runtime.path),
                missing_command: None,
                facts: HostFacts {
                    command_present: &present,
                    ..operation.facts
                },
                ..*operation
            };
            Ok(vibe_app::continue_after_node(&current, target, *token)?)
        }
    }
}
