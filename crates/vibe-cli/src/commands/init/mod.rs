//! `vibe init` — scaffold a new vibevm project.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1, §11.1.
//! Acceptance: the produced tree matches §4.2; running twice does not destroy
//! user-modified files (idempotent).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#project-initialization");

mod helpers;
mod package;

// Bring the split-out functions into scope so mod.rs reads as before.
use helpers::*;
use package::{
    create_group_in_project, create_package_dirs, create_package_in_project, resolve_authors,
};

// Re-export items accessed from other modules (resolver.rs, install/ etc.).
pub(crate) use helpers::{current_timestamp_utc, strip_unc_public};

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{
    ActiveSection, DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, Lockfile, Manifest,
    NamingConvention, ProjectSection, RegistrySection,
};

use crate::cli::InitArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: InitArgs) -> Result<()> {
    // Parse the positional arguments into an InitTarget.
    let target = parse_positionals(&args.positional)?;
    let project_path = resolve_project_path(&args, &target)?;

    match &target {
        InitTarget::ProjectOnly => {
            create_project(ctx, &args, &project_path, None)?;
        }
        InitTarget::ProjectWithPackage { group, name } => {
            create_project(ctx, &args, &project_path, Some((group, name)))?;
        }
        InitTarget::Package { group, name } => {
            create_package_in_project(ctx, &args, &project_path, group, name)?;
        }
        InitTarget::Group { group } => {
            create_group_in_project(ctx, &project_path, group)?;
        }
    }
    Ok(())
}

/// What `vibe init` should create, parsed from the positional args.
#[derive(Debug, Clone)]
enum InitTarget {
    /// Just a project (vibe.toml + spec tree), no package.
    ProjectOnly,
    /// A project plus a package under packages/<group>/<name>/.
    ProjectWithPackage { group: String, name: String },
    /// Add a package to an existing project root.
    Package { group: String, name: String },
    /// Add a group directory to an existing project root.
    Group { group: String },
}

/// Parse positional args into an InitTarget.
///
/// Forms:
///   []                                      → ProjectOnly (CWD, legacy)
///   ["projectname"]                         → ProjectOnly (in projectname/)
///   ["org.vibevm.apple", "projectname"]     → ProjectWithPackage in projectname/
///   ["org.vibevm.apple"]                    → ProjectWithPackage in CWD
///   ["org.vibevm.apple/orange"]             → ProjectWithPackage in CWD
///   ["package", "org.vibevm.apple/orange"]  → Package in CWD
///   ["package", "org.vibevm.apple/orange", "dir"] → Package in dir/
///   ["group", "org.vibevm.apple"]           → Group in CWD
///   ["group", "org.vibevm.apple", "dir"]    → Group in dir/
fn parse_positionals(positional: &[String]) -> Result<InitTarget> {
    if positional.is_empty() {
        return Ok(InitTarget::ProjectOnly);
    }

    let first = &positional[0];

    // Subcommand forms: `vibe init package ...` / `vibe init group ...`
    if first == "package" {
        let pkgref = positional
            .get(1)
            .context("`vibe init package` requires a pkgref `<group>/<name>`")?;
        let (group, name) = split_pkgref(pkgref)?;
        return Ok(InitTarget::Package { group, name });
    }
    if first == "group" {
        let group = positional
            .get(1)
            .context("`vibe init group` requires a group name (e.g. org.vibevm.apple)")?;
        validate_group(group)?;
        return Ok(InitTarget::Group {
            group: group.clone(),
        });
    }

    // No subcommand keyword — the first arg is either a pkgref or a path.
    if looks_like_group_or_pkgref(first) {
        // First arg is a group or pkgref → project with package in CWD
        // (or in the second positional if it's a path).
        let (group, name) = if first.contains('/') {
            split_pkgref(first)?
        } else {
            // group-only → default package name "main"
            validate_group(first)?;
            (first.clone(), "main".to_string())
        };
        return Ok(InitTarget::ProjectWithPackage { group, name });
    }

    // First arg is a path (no dots → not a group). If there's a second arg,
    // it's a pkgref → project with package at that path.
    if let Some(second) = positional.get(1)
        && looks_like_group_or_pkgref(second)
    {
        let (group, name) = if second.contains('/') {
            split_pkgref(second)?
        } else {
            validate_group(second)?;
            (second.clone(), "main".to_string())
        };
        return Ok(InitTarget::ProjectWithPackage { group, name });
    }

    // Just a path → project only.
    Ok(InitTarget::ProjectOnly)
}

/// Does `s` look like a reverse-FQDN group (`org.vibevm.apple`) or a
/// pkgref (`org.vibevm.apple/orange`)? Heuristic: contains a dot.
fn looks_like_group_or_pkgref(s: &str) -> bool {
    s.contains('.') || s.contains('/')
}

/// Split a pkgref `org.vibevm.apple/orange` into `(group, name)`.
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

/// Validate a group looks like a reverse-FQDN (at least one dot, lowercase).
fn validate_group(group: &str) -> Result<()> {
    if !group.contains('.') {
        bail!("`{group}` is not a valid group — expected a reverse-FQDN like `org.vibevm.apple`");
    }
    // Let the Group parser do the full validation.
    vibe_core::Group::parse(group).with_context(|| format!("`{group}` is not a valid group"))?;
    Ok(())
}

/// Resolve the project root path from args + target.
fn resolve_project_path(args: &InitArgs, _target: &InitTarget) -> Result<PathBuf> {
    // The path is: the last positional that doesn't look like a pkgref/group/subcommand,
    // or `--path`, or CWD.
    let positional_path = args
        .positional
        .iter()
        .rev()
        .find(|s| s != &"package" && s != &"group" && !looks_like_group_or_pkgref(s));
    if let Some(p) = positional_path {
        return Ok(PathBuf::from(p));
    }
    if let Some(p) = &args.path {
        return Ok(p.clone());
    }
    // For Package/Group targets: default to CWD (the project must already exist).
    // For ProjectOnly/ProjectWithPackage: default to CWD too (matches legacy).
    Ok(PathBuf::from("."))
}

/// Create a full project (vibe.toml + spec tree + boot artifacts), optionally
/// with a package under packages/<group>/<name>/v<version>/.
fn create_project(
    ctx: &output::Context,
    args: &InitArgs,
    project_path: &Path,
    package: Option<(&str, &str)>,
) -> Result<()> {
    let display_requested = project_path.to_path_buf();

    fs::create_dir_all(project_path)
        .with_context(|| format!("creating project directory `{}`", project_path.display()))?;

    let path = canonical_no_unc(project_path)?;
    let display_root = normalize_display(&display_requested, &path);

    if !path.is_dir() {
        bail!("target `{}` is not a directory", display_root);
    }

    let project_name = resolve_name(args, &path)?;

    ctx.heading(&format!(
        "Initializing project `{project_name}` in `{display_root}`"
    ));

    let mut outcomes = Vec::<Outcome>::new();

    // 1. spec/ directory tree.
    for sub in ["boot", "flows", "feats", "stacks", "common", "modules"] {
        ensure_dir(&path.join("spec").join(sub))?;
    }

    // 2. User-owned boot snippets.
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/00-core.md"),
        &boot_00_core_template(&project_name),
        "boot: project foundation",
    )?);
    outcomes.push(ensure_file(
        ctx,
        &path,
        &path.join("spec/boot/90-user.md"),
        BOOT_90_USER_TEMPLATE,
        "boot: user overrides",
    )?);

    // 3. Project manifest and empty lockfile.
    let registries = resolve_registry_sections(args);
    let manifest_authors = resolve_authors(args);
    outcomes.push(ensure_project_manifest_full(
        ctx,
        &path,
        &project_name,
        args.stack.as_deref(),
        registries,
        &manifest_authors,
    )?);
    outcomes.push(ensure_empty_lockfile(ctx, &path)?);

    // 4. `.vibe/` cache (gitignored).
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
        let pkg_outcomes = create_package_dirs(
            ctx, &path, group, name, args, "static", // project+package = static link
        )?;
        outcomes.extend(pkg_outcomes);
    }

    // 7. Generate the boot artifacts.
    outcomes.extend(generate_boot_artifacts(ctx, &path)?);

    report(ctx, &project_name, &display_root, &outcomes)?;
    Ok(())
}

/// Generate the PROP-009 boot artifacts for the freshly-scaffolded
/// project at `path`: `spec/boot/INDEX.md` and the managed `<vibevm>`
/// block in each agent instruction file (PROP-012). Returns one
/// [`Outcome`] per artifact, reporting whether it was newly created or
/// regenerated in place.
fn generate_boot_artifacts(ctx: &output::Context, path: &Path) -> Result<Vec<Outcome>> {
    const ARTIFACTS: [&str; 4] = ["spec/boot/INDEX.md", "CLAUDE.md", "AGENTS.md", "GEMINI.md"];
    let preexisting: Vec<bool> = ARTIFACTS.iter().map(|f| path.join(f).exists()).collect();

    let workspace = vibe_workspace::Workspace::load(path)
        .with_context(|| "loading the new project to generate its boot artifacts")?;
    vibe_workspace::install::regenerate_boot(&workspace)
        .with_context(|| "generating the boot artifacts")?;

    let mut outcomes = Vec::with_capacity(ARTIFACTS.len());
    for (artifact, &existed) in ARTIFACTS.iter().zip(&preexisting) {
        if existed {
            ctx.skipped(artifact, "regenerated");
        } else {
            ctx.created(artifact);
        }
        outcomes.push(Outcome {
            path: (*artifact).to_string(),
            action: if existed {
                Action::Kept
            } else {
                Action::Created
            },
            reason: "boot artifact",
        });
    }
    Ok(outcomes)
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Outcome {
    path: String,
    action: Action,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Action {
    Created,
    Kept,
}

fn ensure_file(
    ctx: &output::Context,
    root: &Path,
    path: &Path,
    content: &str,
    reason: &'static str,
) -> Result<Outcome> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let rel = relative_to_root(root, path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason,
        });
    }
    fs::write(path, content).with_context(|| format!("writing `{}`", path.display()))?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason,
    })
}

pub(super) fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating `{}`", path.display()))
}

/// Ensure the project-root `.gitignore` protects the L3 personal-settings file
/// (PROP-040 §9 `#gitignore-autogen`): `/.vibe/settings.local.toml` must never
/// be accidentally committed. Idempotent — if the file already ignores the local
/// settings basename (any line naming `settings.local.toml`), this is a no-op;
/// otherwise it appends a small section, preserving all existing content.
/// Returns `Some(Outcome)` only when it appended, `None` when already covered or
/// no root `.gitignore` is present (the step-5 `ensure_file` owns creation).
fn ensure_local_settings_gitignored(root: &Path) -> Result<Option<Outcome>> {
    let path = root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    // Marker: any line that ignores the local settings file. Match the
    // distinctive basename so an operator's own spelling still counts.
    if existing
        .lines()
        .any(|line| line.contains("settings.local.toml"))
    {
        return Ok(None);
    }
    let mut content = existing;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(
        "\n# vibevm personal settings (L3 — never committed; PROP-040 §9 #gitignore-autogen)\n\
         /.vibe/settings.local.toml\n\
         /.vibe/*.local.toml\n",
    );
    fs::write(&path, &content)
        .with_context(|| format!("appending L3 settings entry to `{}`", path.display()))?;
    Ok(Some(Outcome {
        path: relative_to_root(root, &path),
        action: Action::Created,
        reason: "gitignore: L3 personal settings",
    }))
}

fn ensure_project_manifest_full(
    ctx: &output::Context,
    root: &Path,
    name: &str,
    stack: Option<&str>,
    registries: Vec<RegistrySection>,
    authors: &[String],
) -> Result<Outcome> {
    let path = root.join(Manifest::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "project manifest",
        });
    }

    let manifest = Manifest {
        project: Some(ProjectSection {
            name: name.to_string(),
            version: "0.0.1".to_string(),
            authors: authors.to_vec(),
        }),
        active: stack.map(|s| ActiveSection {
            stack: Some(s.to_string()),
        }),
        registries,
        ..Default::default()
    };

    manifest.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "project manifest",
    })
}

/// Build the `[[registry]]` entries to write into a fresh `vibe.toml`.
///
/// - `--no-registry` → empty (vibe.toml has no `[[registry]]`).
/// - `--registry-url <URL>` → one entry — the operator's custom registry,
///   replacing the defaults entirely. `--registry-ref` further overrides
///   the ref.
/// - default → two entries:
///   The default pair (vibespecs GitHub + GitVerse) is no longer written
///   into per-project `vibe.toml` — it lives in `~/.vibe/registry.toml`,
///   seeded by `ensure_default_global_registry()` on first use. A project's
///   `vibe.toml` stays clean of registry boilerplate; a project only carries
///   `[[registry]]` entries for registries it needs *beyond* the machine
///   default (a local `file://` checkout, a private org, etc.).
///
///   `--registry-url <URL>` still writes a single explicit entry for a
///   project that pins a specific registry. `--no-registry` stays a no-op
///   (it was the same as the new default — empty).
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
    // No `--registry-url` / `--no-registry`: the defaults (vibespecs GitHub +
    // GitVerse) now live in `~/.vibe/registry.toml`, not in the project.
    // An empty vector keeps the project's `vibe.toml` clean.
    Vec::new()
}

fn ensure_empty_lockfile(ctx: &output::Context, root: &Path) -> Result<Outcome> {
    let path = root.join(Lockfile::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "lockfile",
        });
    }
    let lockfile = Lockfile::empty(
        format!("vibe {}", env!("CARGO_PKG_VERSION")),
        current_timestamp_utc(),
    );
    lockfile.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "lockfile",
    })
}
