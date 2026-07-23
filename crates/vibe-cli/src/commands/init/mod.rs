//! `vibe init` — scaffold a new vibevm project, package, or group.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1, §11.1.
//!
//! Forms:
//!   vibe init [dir]                              — project only (dir defaults to CWD)
//!   vibe init package <group>/<name> [dir]       — package (creates project if absent)
//!   vibe init group <group> [dir]                — group dir (creates project if absent)

specmark::scope!("spec://vibevm/VIBEVM-SPEC#project-initialization");

mod helpers;
mod package;
mod prompts;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{
    ActiveSection, DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, Manifest, NamingConvention,
    RegistrySection,
};
use vibe_core::user_config::UserConfig;

use crate::cli::InitArgs;
use crate::output;

// Re-export items accessed from other modules (resolver.rs, install/ etc.).
pub(crate) use helpers::{current_timestamp_utc, strip_unc_public};

pub fn run(ctx: &output::Context, args: InitArgs) -> Result<()> {
    let positional = &args.positional;

    // Determine the mode from the first positional arg.
    let mode = if positional.is_empty() {
        InitMode::Project
    } else if positional[0] == "package" {
        InitMode::Package
    } else if positional[0] == "group" {
        InitMode::Group
    } else {
        InitMode::Project
    };

    // Extract pkgref/group and path from positionals.
    let (pkgref, path) = match mode {
        InitMode::Project => {
            // First positional (if any) is the directory path.
            let p = positional
                .first()
                .map(PathBuf::from)
                .or_else(|| args.path.clone());
            (None, p.unwrap_or_default())
        }
        InitMode::Package | InitMode::Group => {
            // First positional was the keyword; second is the pkgref/group;
            // third (if any) is the path.
            let pkgref = positional.get(1).cloned();
            let path = positional
                .get(2)
                .map(PathBuf::from)
                .or_else(|| args.path.clone())
                .unwrap_or_default();
            (pkgref, path)
        }
    };

    // Resolve the project root path. Empty = CWD.
    let project_path = if path.as_os_str().is_empty() || path.as_os_str() == "." {
        PathBuf::from(".")
    } else {
        path
    };

    // Load user config for defaults (last_author).
    let mut user_config = UserConfig::load().unwrap_or_default();

    // Determine if interactive prompts should run.
    let interactive = console::user_attended() && !ctx.is_json() && !ctx.is_unattended();

    match mode {
        InitMode::Project => {
            create_project(
                ctx,
                &args,
                &project_path,
                &mut user_config,
                interactive,
                None,
            )?;
        }
        InitMode::Package => {
            let pkgref =
                pkgref.context("`vibe init package` requires a pkgref `<group>/<name>`")?;
            let (group, name) = split_pkgref(&pkgref)?;
            // Ensure the project exists first.
            ensure_project_exists(ctx, &args, &project_path, &mut user_config, interactive)?;
            package::create_package_in_project(
                ctx,
                &args,
                &project_path,
                &group,
                &name,
                &mut user_config,
                interactive,
            )?;
        }
        InitMode::Group => {
            let group_str = pkgref.context("`vibe init group` requires a group name")?;
            validate_group(&group_str)?;
            ensure_project_exists(ctx, &args, &project_path, &mut user_config, interactive)?;
            package::create_group_in_project(ctx, &project_path, &group_str)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum InitMode {
    Project,
    Package,
    Group,
}

/// If `project_path` does not contain a `vibe.toml`, create the project first.
fn ensure_project_exists(
    ctx: &output::Context,
    args: &InitArgs,
    project_path: &Path,
    user_config: &mut UserConfig,
    interactive: bool,
) -> Result<()> {
    let path = if project_path.as_os_str() == "." {
        std::env::current_dir().unwrap_or_default()
    } else {
        project_path.to_path_buf()
    };
    if !path.join(Manifest::FILENAME).exists() {
        create_project(ctx, args, project_path, user_config, interactive, None)?;
    }
    Ok(())
}

/// Create a full project (vibe.toml + spec tree + boot artifacts), optionally
/// with a package.
fn create_project(
    ctx: &output::Context,
    args: &InitArgs,
    project_path: &Path,
    user_config: &mut UserConfig,
    interactive: bool,
    package: Option<(&str, &str)>,
) -> Result<()> {
    use helpers::*;

    fs::create_dir_all(project_path)
        .with_context(|| format!("creating project directory `{}`", project_path.display()))?;

    let path = canonical_no_unc(project_path)?;
    let display_root = normalize_display(project_path, &path);

    if !path.is_dir() {
        bail!("target `{}` is not a directory", display_root);
    }

    let project_name = resolve_name(args, &path)?;

    // Gather project field values (interactive prompts or flags/defaults).
    let fields = if interactive {
        prompts::prompt_project_fields(&project_name, user_config)?
    } else {
        prompts::project_fields_from_args(args, &project_name, user_config)
    };

    // Save last_author if it changed.
    prompts::maybe_save_author(user_config, &fields.authors);

    ctx.heading(&format!(
        "Initializing project `{}` in `{display_root}`",
        fields.name
    ));

    let mut outcomes = Vec::<helpers::Outcome>::new();

    // 1. spec/ directory tree.
    for sub in ["boot", "flows", "feats", "stacks", "common", "modules"] {
        ensure_dir(&path.join("spec").join(sub))?;
    }

    // 2. User-owned boot snippets.
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/00-core.md"),
        &boot_00_core_template(&fields.name),
        "boot: project foundation",
    )?);
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/90-user.md"),
        BOOT_90_USER_TEMPLATE,
        "boot: user overrides",
    )?);

    // 3. Project manifest + empty lockfile.
    let registries = resolve_registry_sections(args);
    outcomes.push(ensure_project_manifest(
        ctx,
        &path,
        &fields.name,
        args.stack.as_deref(),
        registries,
        &fields.authors,
    )?);
    outcomes.push(ensure_empty_lockfile(ctx, &path)?);

    // 4. `.vibe/` cache.
    ensure_dir(&path.join(".vibe/cache"))?;
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join(".vibe/.gitignore"),
        "*\n",
        "gitignore: cache",
    )?);

    // 5. .gitignore at project root.
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join(".gitignore"),
        ROOT_GITIGNORE_TEMPLATE,
        "gitignore: root",
    )?);
    if let Some(o) = ensure_local_settings_gitignored(&path)? {
        outcomes.push(o);
    }

    // 6. Package (if requested).
    if let Some((group, name)) = package {
        outcomes.extend(package::create_package_dirs(
            ctx, &path, group, name, args, "static", &fields,
        )?);
    }

    // 7. Generate boot artifacts.
    outcomes.extend(generate_boot_artifacts(ctx, &path)?);

    report(ctx, &fields.name, &display_root, &outcomes)?;
    Ok(())
}

// ==== Utilities ==========================================================

fn split_pkgref(s: &str) -> Result<(String, String)> {
    let (group, name) = s.split_once('/').context(format!(
        "`{s}` is not a valid pkgref — expected `<group>/<name>` (e.g. org.vibevm.apple/orange)"
    ))?;
    validate_group(group)?;
    if name.is_empty() {
        bail!("package name after `/` is empty in `{s}`");
    }
    Ok((group.to_string(), name.to_string()))
}

fn validate_group(group: &str) -> Result<()> {
    if !group.contains('.') {
        bail!("`{group}` is not a valid group — expected a reverse-FQDN like `org.vibevm.apple`");
    }
    vibe_core::Group::parse(group).with_context(|| format!("`{group}` is not a valid group"))?;
    Ok(())
}

fn resolve_registry_sections(args: &InitArgs) -> Vec<RegistrySection> {
    if args.no_registry {
        return Vec::new();
    }
    if let Some(url) = &args.registry_url {
        return vec![RegistrySection {
            name: DEFAULT_REGISTRY_NAME.to_string(),
            url: url.clone(),
            r#ref: args
                .registry_ref
                .clone()
                .unwrap_or_else(|| DEFAULT_REGISTRY_REF.to_string()),
            naming: NamingConvention::Fqdn,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
            enabled: true,
        }];
    }
    Vec::new()
}
