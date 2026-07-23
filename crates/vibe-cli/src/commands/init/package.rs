//! Package + group creation for `vibe init package` / `vibe init group`.

use super::helpers::*;
use super::prompts::{self, ProjectFields};
use crate::cli::InitArgs;
use crate::output;
use std::fs;
use std::path::Path;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::UserConfig;

use anyhow::{Result, bail};

/// Create a package in an existing project root (dynamic link).
pub(super) fn create_package_in_project(
    ctx: &output::Context,
    args: &InitArgs,
    project_path: &Path,
    group: &str,
    name: &str,
    user_config: &mut UserConfig,
    interactive: bool,
) -> Result<()> {
    let path = canonical_no_unc(project_path)?;
    if !path.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}` — `vibe init package` must be run inside a project root",
            path.display()
        );
    }

    // Gather package field values (interactive or defaults from project fields).
    let fields = if interactive {
        prompts::prompt_package_fields(group, name, user_config)?
    } else {
        prompts::package_fields_from_args(args, group, name, user_config)
    };
    prompts::maybe_save_author(user_config, &fields.authors);

    ctx.heading(&format!(
        "Creating package `{group}/{name}` in `{}`",
        path.display()
    ));

    let mut outcomes =
        create_package_dirs_from_fields(ctx, &path, group, name, "dynamic", &fields)?;

    // Regenerate boot artifacts.
    outcomes.extend(generate_boot_artifacts(ctx, &path)?);

    report(
        ctx,
        &format!("{group}/{name}"),
        &display_pathbuf(&path),
        &outcomes,
    )?;
    Ok(())
}

/// Create a group directory (packages/<group>/) in an existing project root.
pub(super) fn create_group_in_project(
    ctx: &output::Context,
    project_path: &Path,
    group: &str,
) -> Result<()> {
    let path = canonical_no_unc(project_path)?;
    let group_dir = path.join("packages").join(group);
    ensure_dir(&group_dir)?;
    ctx.created(&display_pathbuf(
        group_dir.strip_prefix(&path).unwrap_or(&group_dir),
    ));
    ctx.summary(&format!("Created group `{group}` in `{}`", path.display()));
    Ok(())
}

/// Create the package directory tree for project+package (static link).
pub(super) fn create_package_dirs(
    ctx: &output::Context,
    project_root: &Path,
    group: &str,
    name: &str,
    args: &InitArgs,
    default_link: &str,
    project_fields: &ProjectFields,
) -> Result<Vec<Outcome>> {
    let fields = ProjectFields {
        name: name.to_string(),
        version: args.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        authors: project_fields.authors.clone(),
        license: args
            .license
            .clone()
            .unwrap_or_else(|| project_fields.license.clone()),
        description: args.description.clone().unwrap_or_default(),
        format: args.format.clone().unwrap_or_else(|| "normal".to_string()),
    };
    create_package_dirs_from_fields(ctx, project_root, group, name, default_link, &fields)
}

/// Create the package directory tree from explicit fields.
fn create_package_dirs_from_fields(
    ctx: &output::Context,
    project_root: &Path,
    group: &str,
    name: &str,
    link: &str,
    fields: &ProjectFields,
) -> Result<Vec<Outcome>> {
    let kind = "tool";
    let version = &fields.version;
    let pkg_dir = project_root
        .join("packages")
        .join(group)
        .join(name)
        .join(format!("v{version}"));

    let mut outcomes = Vec::new();

    let manifest_path = pkg_dir.join(Manifest::FILENAME);
    let manifest_rel = display_pathbuf(
        &pkg_dir
            .strip_prefix(project_root)
            .unwrap_or(&pkg_dir)
            .join(Manifest::FILENAME),
    );
    if manifest_path.exists() {
        ctx.skipped(&manifest_rel, "already exists");
        outcomes.push(Outcome {
            path: manifest_rel,
            action: Action::Kept,
            reason: "package manifest",
        });
    } else {
        ensure_dir(&pkg_dir)?;
        let authors_line = if fields.authors.is_empty() {
            String::new()
        } else {
            let quoted: Vec<String> = fields.authors.iter().map(|a| format!("\"{a}\"")).collect();
            format!("authors = [{}]\n", quoted.join(", "))
        };
        let manifest_text = format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"{kind}\"\n\
             version = \"{version}\"\n{authors_line}\
             license = \"{license}\"\ndescription = \"{description}\"\nformat = \"{format}\"\n\n\
             [boot_snippet]\nsource = \"spec/boot/10-tool-{name}.md\"\ncategory = \"tool\"\nlink = \"{link}\"\n",
            license = fields.license,
            description = fields.description,
            format = fields.format,
        );
        fs::write(&manifest_path, &manifest_text)?;
        ctx.created(&manifest_rel);
        outcomes.push(Outcome {
            path: manifest_rel,
            action: Action::Created,
            reason: "package manifest",
        });
    }

    // spec/boot/.
    let boot_dir = pkg_dir.join("spec").join("boot");
    ensure_dir(&boot_dir)?;
    let boot_file = boot_dir.join(format!("10-tool-{name}.md"));
    let boot_rel = display_pathbuf(boot_file.strip_prefix(project_root).unwrap_or(&boot_file));
    if !boot_file.exists() {
        let content = format!(
            "<!-- vibe:static org.{group}/{name} — boot snippet -->\n\n# {name}\n\nA `{kind}` package.\n"
        );
        fs::write(&boot_file, &content)?;
        ctx.created(&boot_rel);
        outcomes.push(Outcome {
            path: boot_rel,
            action: Action::Created,
            reason: "package boot snippet",
        });
    } else {
        ctx.skipped(&boot_rel, "already exists");
        outcomes.push(Outcome {
            path: boot_rel,
            action: Action::Kept,
            reason: "package boot snippet",
        });
    }

    // README.md.
    let readme_path = pkg_dir.join("README.md");
    let readme_rel = display_pathbuf(
        readme_path
            .strip_prefix(project_root)
            .unwrap_or(&readme_path),
    );
    if !readme_path.exists() {
        fs::write(
            &readme_path,
            format!("# {name}\n\nA `{kind}` package in group `{group}`.\n"),
        )?;
        ctx.created(&readme_rel);
        outcomes.push(Outcome {
            path: readme_rel,
            action: Action::Created,
            reason: "package README",
        });
    } else {
        ctx.skipped(&readme_rel, "already exists");
        outcomes.push(Outcome {
            path: readme_rel,
            action: Action::Kept,
            reason: "package README",
        });
    }

    Ok(outcomes)
}
