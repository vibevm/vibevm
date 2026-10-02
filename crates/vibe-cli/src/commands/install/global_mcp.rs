//! User-scoped MCP packages reuse the ordinary project installer. Their
//! project lives under Vibe's settings root, so its manifest, lockfile and
//! slots are independent of whichever directory invoked `vibe install -g`.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_agent_projection::agents::Agent;
use vibe_core::manifest::Lockfile;

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

const USER_PROJECT_MANIFEST: &str = "[project]\nname = \"vibevm-user-mcp\"\nversion = \"1.0.0\"\n";

/// The selected package and agent targets, returned after package
/// materialisation so registration can use the same project and selection.
pub(crate) struct GlobalMcpInstall {
    pub(crate) project_root: PathBuf,
    pub(crate) package: String,
    pub(crate) agents: Vec<Agent>,
}

/// The durable package identity after a user-scoped update. The caller uses
/// this to refresh its agent registrations against the new locked version.
pub(crate) struct GlobalMcpUpdate {
    pub(crate) project_root: PathBuf,
    pub(crate) package: String,
}

/// The dedicated user project used by `vibe install -g mcp:...` and by
/// user-scope package registration. The path is stable across invocations.
pub(crate) fn user_project_root() -> Result<PathBuf> {
    let settings = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })?;
    Ok(settings.join("mcp").join("user-project"))
}

pub(crate) fn run(
    ctx: &output::Context,
    mut args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<GlobalMcpInstall> {
    let package = validate_package(&args)?.to_owned();
    let agents = select_agents(ctx, args.agent.as_deref())?;
    let project_root = user_project_root()?;
    let preparation = ctx.progress().task("Preparing user MCP project");
    if let Err(error) = prepare_project(&project_root) {
        preparation.fail("user MCP project preparation failed");
        return Err(error);
    }
    preparation.finish();
    args.path = project_root.clone();
    // Keep every previously installed user MCP package on its chosen version.
    // A later `-g` install must not silently move another package's endpoint
    // without refreshing its registrations in every agent.
    args.exact = true;
    // The project installer owns resolution, lockfile and slots. Global is
    // solely a CLI routing flag; do not let it select a second resolver.
    args.global = false;
    args.agent = None;
    args.server = None;
    super::run_user_project(
        ctx,
        args,
        embedded_root,
        root_offline,
        super::UserProjectAction::Install,
    )?;
    Ok(GlobalMcpInstall {
        project_root,
        package,
        agents,
    })
}

/// Re-resolve only the named global MCP root. The direct install path accepts
/// an explicit root even when its previous manifest declaration was exact:
/// it resolves the new selection and writes a fresh exact pin, while leaving
/// every other user MCP root at its existing version.
pub(crate) fn update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<GlobalMcpUpdate> {
    if args.packages.len() != 1 || args.all {
        bail!("global MCP update requires exactly one `mcp:<group>/<name>` package");
    }
    if args.from_source || args.local_source || args.binary {
        bail!("--from-source, --local-source and --binary apply only to global applications");
    }
    if args.path != Path::new(".") {
        bail!("global MCP update uses a dedicated user project; omit --path");
    }
    let package = validate_mcp_coordinate(&args.packages[0], false)?.to_owned();
    let project_root = existing_user_project_root()?;
    ensure_locked(&project_root, &package)?;
    let install_args = InstallArgs {
        packages: vec![package.clone()],
        global: false,
        agent: None,
        server: None,
        from_source: false,
        local_source: false,
        path: project_root.clone(),
        registry: args.registry,
        assume_yes: args.assume_yes,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: true,
        auth_required: args.auth_required,
        solver: None,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: false,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        force: false,
        trace_compile: args.trace_compile,
    };
    super::run_user_project(
        ctx,
        install_args,
        embedded_root,
        root_offline,
        super::UserProjectAction::Update,
    )?;
    Ok(GlobalMcpUpdate {
        project_root,
        package,
    })
}

/// Remove a single global MCP root with the ordinary project uninstaller.
/// The user project is never created by a removal request.
pub(crate) fn preflight_uninstall(args: &UninstallArgs) -> Result<PathBuf> {
    if args.path != Path::new(".") {
        bail!("global MCP uninstall uses a dedicated user project; omit --path");
    }
    let package = validate_mcp_coordinate(&args.package, false)?.to_owned();
    let project_root = existing_user_project_root()?;
    ensure_locked(&project_root, &package)?;
    Ok(project_root)
}

pub(crate) fn uninstall(ctx: &output::Context, mut args: UninstallArgs) -> Result<()> {
    args.path = preflight_uninstall(&args)?;
    args.global = false;
    args.agent = None;
    crate::commands::uninstall::run(&ctx.progress_child(), args)
}

fn existing_user_project_root() -> Result<PathBuf> {
    let root = user_project_root()?;
    let manifest = root.join("vibe.toml");
    let root_meta = fs::symlink_metadata(&root).with_context(|| {
        format!(
            "no global MCP packages are installed in `{}`; run `vibe install -g mcp:<group>/<name>` first",
            root.display()
        )
    })?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        bail!(
            "user MCP project root is not a regular directory: {}",
            root.display()
        );
    }
    let manifest_meta = fs::symlink_metadata(&manifest).with_context(|| {
        format!(
            "user MCP project manifest is missing: {}",
            manifest.display()
        )
    })?;
    if manifest_meta.file_type().is_symlink() || !manifest_meta.is_file() {
        bail!(
            "user MCP project manifest is not a regular file: {}",
            manifest.display()
        );
    }
    Ok(root)
}

fn ensure_locked(root: &Path, spelling: &str) -> Result<()> {
    let selected = vibe_core::PackageRef::parse(spelling)?;
    let group = selected
        .group
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("global MCP package must have an explicit group"))?;
    let lock = Lockfile::read(root.join(Lockfile::FILENAME))?;
    let locked = lock
        .find(group, &selected.name)
        .ok_or_else(|| anyhow::anyhow!("global MCP package `{spelling}` is not installed"))?;
    if locked.kind.to_string() != "mcp" {
        bail!("global package `{spelling}` is not an MCP package");
    }
    Ok(())
}

fn validate_package(args: &InstallArgs) -> Result<&str> {
    if args.packages.len() != 1 {
        bail!("global MCP install requires exactly one `mcp:<group>/<name>` package");
    }
    if args.from_source || args.local_source {
        bail!("--from-source and --local-source apply only to global applications");
    }
    if args.path != Path::new(".") {
        bail!("global MCP install uses a dedicated user project; omit --path");
    }
    validate_mcp_coordinate(&args.packages[0], true)
}

fn validate_mcp_coordinate(spelling: &str, allow_version: bool) -> Result<&str> {
    let coordinate = spelling.strip_prefix("mcp:").ok_or_else(|| {
        anyhow::anyhow!("global MCP package must be spelled `mcp:<group>/<name>`")
    })?;
    let (group, name) = coordinate.split_once('/').ok_or_else(|| {
        anyhow::anyhow!("global MCP package must be fully qualified as `mcp:<group>/<name>`")
    })?;
    if group.is_empty() || name.is_empty() {
        bail!("global MCP package must be fully qualified as `mcp:<group>/<name>`");
    }
    if !allow_version && name.contains('@') {
        bail!("global MCP update and uninstall use an unversioned `mcp:<group>/<name>` coordinate");
    }
    let requested = vibe_core::PackageRef::parse(spelling)
        .with_context(|| format!("parsing global MCP package `{spelling}`"))?;
    if requested.group.is_none() {
        bail!("global MCP package must be fully qualified as `mcp:<group>/<name>`");
    }
    Ok(spelling)
}

fn select_agents(ctx: &output::Context, explicit: Option<&str>) -> Result<Vec<Agent>> {
    if let Some(explicit) = explicit {
        return crate::commands::global_mcp_agents::parse_explicit_filter(explicit);
    }
    if !crate::commands::global_mcp_agents::interactive(ctx) {
        bail!(
            "global MCP install needs --agent <name> (or --agent all) outside an interactive terminal"
        );
    }
    let detected: Vec<_> = Agent::ALL
        .iter()
        .copied()
        .filter(|agent| agent.host_present())
        .collect();
    crate::commands::global_mcp_agents::prompt(ctx, Agent::ALL, &detected)
}

fn prepare_project(root: &Path) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(root)
        && (metadata.file_type().is_symlink() || !metadata.is_dir())
    {
        bail!(
            "user MCP project root is not a regular directory: {}",
            root.display()
        );
    }
    fs::create_dir_all(root)
        .with_context(|| format!("creating user MCP project `{}`", root.display()))?;
    let manifest = root.join("vibe.toml");
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&manifest)
    {
        Ok(mut file) => file
            .write_all(USER_PROJECT_MANIFEST.as_bytes())
            .with_context(|| format!("initializing `{}`", manifest.display())),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&manifest)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                bail!(
                    "user MCP project manifest is not a regular file: {}",
                    manifest.display()
                );
            }
            Ok(())
        }
        Err(error) => Err(error).with_context(|| format!("creating `{}`", manifest.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_project_is_initialized_once_without_erasing_installed_requirements() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("mcp-user-project");
        prepare_project(&root).unwrap();
        let manifest = root.join("vibe.toml");
        assert_eq!(
            fs::read_to_string(&manifest).unwrap(),
            USER_PROJECT_MANIFEST
        );
        fs::write(&manifest, "[project]\nname='vibevm-user-mcp'\nversion='1.0.0'\n[requires]\npackages=['mcp:ai.lev/fpf-mcp']\n").unwrap();
        prepare_project(&root).unwrap();
        assert!(
            fs::read_to_string(&manifest)
                .unwrap()
                .contains("mcp:ai.lev/fpf-mcp")
        );
    }

    #[test]
    fn refuses_symlinked_project_manifest() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("mcp-user-project");
        fs::create_dir(&root).unwrap();
        let actual = scratch.path().join("other.toml");
        fs::write(&actual, "untouched").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&actual, root.join("vibe.toml")).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&actual, root.join("vibe.toml")).is_err() {
            return; // Windows without Developer Mode cannot create this test fixture.
        }
        assert!(prepare_project(&root).is_err());
        assert_eq!(fs::read_to_string(actual).unwrap(), "untouched");
    }
}
