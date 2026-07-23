//! Package + group creation for `vibe init package` / `vibe init group`
//! and the project-with-package forms.

use super::*;
use crate::cli::InitArgs;
use crate::output;
use std::fs;
use std::path::Path;

/// Create a package in an existing project root (dynamic link).
pub(super) fn create_package_in_project(
    ctx: &output::Context,
    args: &InitArgs,
    project_path: &Path,
    group: &str,
    name: &str,
) -> Result<()> {
    let path = canonical_no_unc(project_path)?;
    if !path.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}` — `vibe init package` must be run inside a project root",
            path.display()
        );
    }
    ctx.heading(&format!(
        "Creating package `{group}/{name}` in `{}`",
        path.display()
    ));

    let mut outcomes = create_package_dirs(ctx, &path, group, name, args, "dynamic")?;

    // Regenerate boot artifacts for the project.
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

/// Create the package directory tree: packages/<group>/<name>/v<version>/
/// with vibe.toml, spec/boot/, README.md.
pub(super) fn create_package_dirs(
    ctx: &output::Context,
    project_root: &Path,
    group: &str,
    name: &str,
    args: &InitArgs,
    default_link: &str,
) -> Result<Vec<Outcome>> {
    let version = args.version.as_deref().unwrap_or("0.1.0");
    let kind = args.kind.as_deref().unwrap_or("tool");
    let pkg_dir = project_root
        .join("packages")
        .join(group)
        .join(name)
        .join(format!("v{version}"));

    let mut outcomes = Vec::new();

    // Package manifest.
    let manifest_path = pkg_dir.join(Manifest::FILENAME);
    let rel = display_pathbuf(
        &pkg_dir
            .strip_prefix(project_root)
            .unwrap_or(&pkg_dir)
            .join(Manifest::FILENAME),
    );
    if manifest_path.exists() {
        ctx.skipped(&rel, "already exists");
        outcomes.push(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "package manifest",
        });
    } else {
        ensure_dir(&pkg_dir)?;
        let authors = resolve_authors(args);
        let license = args.license.as_deref().unwrap_or("UPL-1.0");
        let description = args.description.as_deref().unwrap_or("");
        let format = args.format.as_deref().unwrap_or("simple");
        let link = args.link.as_deref().unwrap_or(default_link);

        let manifest_text = format!(
            "[package]\n\
             group = \"{group}\"\n\
             name = \"{name}\"\n\
             kind = \"{kind}\"\n\
             version = \"{version}\"\n\
             {authors_line}\
             license = \"{license}\"\n\
             description = \"{description}\"\n\
             format = \"{format}\"\n\
             \n\
             [boot_snippet]\n\
             source = \"spec/boot/10-tool-{name}.md\"\n\
             category = \"tool\"\n\
             link = \"{link}\"\n",
            authors_line = if authors.is_empty() {
                String::new()
            } else {
                let quoted: Vec<String> = authors.iter().map(|a| format!("\"{a}\"")).collect();
                format!("authors = [{}]\n", quoted.join(", "))
            },
        );
        fs::write(&manifest_path, &manifest_text)?;
        ctx.created(&rel);
        outcomes.push(Outcome {
            path: rel,
            action: Action::Created,
            reason: "package manifest",
        });
    }

    // spec/boot/ for the package.
    let boot_dir = pkg_dir.join("spec").join("boot");
    ensure_dir(&boot_dir)?;
    let boot_file = boot_dir.join(format!("10-tool-{name}.md"));
    let boot_rel = display_pathbuf(boot_file.strip_prefix(project_root).unwrap_or(&boot_file));
    if !boot_file.exists() {
        let boot_content = format!(
            "<!-- vibe:static org.{group}/{name} — boot snippet -->\n\n# {name}\n\nA `{kind}` package.\n"
        );
        fs::write(&boot_file, &boot_content)?;
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

/// Resolve authors from `--author` flags or git config.
pub(super) fn resolve_authors(args: &InitArgs) -> Vec<String> {
    if !args.authors.is_empty() {
        return args.authors.clone();
    }
    // Try git config user.name.
    let name = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    name.into_iter().collect()
}
