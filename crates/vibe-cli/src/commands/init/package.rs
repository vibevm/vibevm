//! Package + group creation for `vibe init package` / `vibe init group`.

use super::doc::{self, Translation};
use super::helpers::*;
use super::prompts::{self, ProjectFields};
use crate::cli::InitArgs;
use crate::output;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use vibe_core::PackageKind;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::UserConfig;

use anyhow::{Context, Result, bail};

/// The kind `--kind` asks for, or `tool` when it is silent.
///
/// Until B-134 this was read nowhere: every slot was minted `tool`
/// whatever the flag said, while the flag's own `--help` promised
/// otherwise. The parse is `vibe-core`'s, so an unknown kind is refused
/// by the one list that defines the set.
pub(super) fn requested_kind(args: &InitArgs) -> Result<PackageKind> {
    match args.kind.as_deref() {
        None => Ok(PackageKind::Tool),
        Some(raw) => PackageKind::from_str(raw)
            .with_context(|| format!("`--kind {raw}` is not a package kind")),
    }
}

/// The boot-snippet category a kind contributes under. Only four kinds
/// have a category of their own (PROP-009); the rest contribute as
/// tools, which is what the lane already assumed back when every slot
/// was minted `tool` regardless.
fn boot_category(kind: PackageKind) -> &'static str {
    match kind {
        PackageKind::Flow => "flow",
        PackageKind::Stack => "stack",
        PackageKind::App => "app",
        _ => "tool",
    }
}

fn requested_link<'a>(args: &'a InitArgs, default: &'a str) -> Result<&'a str> {
    match args.link.as_deref().unwrap_or(default) {
        link @ ("static" | "dynamic") => Ok(link),
        other => bail!("`--link {other}` is not a package boot link (expected static or dynamic)"),
    }
}

pub(super) struct PreparedPackage {
    fields: ProjectFields,
    kind: PackageKind,
    link: String,
    translation: Option<Translation>,
}

/// Collect and validate all nested-package choices before creating its container.
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_package(
    ctx: &output::Context,
    args: &InitArgs,
    root: &Path,
    group: &str,
    name: &str,
    user_config: &UserConfig,
    interactive: bool,
) -> Result<PreparedPackage> {
    let (fields, kind) = if interactive {
        ctx.suspend_progress(|| {
            prompts::identity_introduction(ctx);
            prompts::prompt_package_fields(args, group, name, user_config)
        })?
    } else {
        (
            prompts::package_fields_from_args(args, group, name, user_config),
            requested_kind(args)?,
        )
    };
    super::root_role::validate_fields(&fields, crate::cli::InitType::Package)?;
    let translation = match args.translates.as_deref() {
        None => None,
        Some(coordinate) => {
            if kind != PackageKind::Doc {
                bail!("`--translates` requires a doc package; pass --kind doc");
            }
            Some(doc::resolve_translation(root, coordinate, name)?)
        }
    };
    let link = requested_link(args, "dynamic")?.to_string();
    let candidate = if kind == PackageKind::Doc {
        doc::manifest_text(group, name, &fields, translation.as_ref())
    } else {
        nested_manifest_text(group, name, &link, &fields, kind)
    };
    let manifest = Manifest::parse_str(&candidate).context(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: invalid nested manifest; fix surface: init metadata and package kind",
    )?;
    let metadata = manifest.require_package()?;
    if metadata.authors != fields.authors
        || metadata.license.as_deref() != Some(fields.license.as_str())
        || metadata.description.as_deref() != Some(fields.description.as_str())
    {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: nested metadata changes when parsed; fix surface: init author, license and description quoting"
        );
    }
    Ok(PreparedPackage {
        fields,
        kind,
        link,
        translation,
    })
}

/// Create the already validated package in its project container.
pub(super) fn create_package_in_project(
    ctx: &output::Context,
    project_path: &Path,
    group: &str,
    name: &str,
    user_config: &mut UserConfig,
    prepared: PreparedPackage,
) -> Result<()> {
    let path = canonical_no_unc(project_path)?;
    if !path.join(Manifest::FILENAME).exists() {
        bail!("no vibe.toml in `{}`", path.display());
    }
    let PreparedPackage {
        fields,
        kind,
        link,
        translation,
    } = prepared;
    prompts::maybe_save_author(user_config, &fields.authors);
    ctx.heading(&format!(
        "Creating package `{group}/{name}` in `{}`",
        path.display()
    ));

    let mut outcomes = create_package_dirs_from_fields(
        ctx,
        &path,
        group,
        name,
        &link,
        &fields,
        kind,
        translation.as_ref(),
    )?;

    // Regenerate boot artifacts. Documentation contributes none — it
    // never enters a boot lane (PROP-057 `##KIND-DOC-MUST-NOT-EXECUTE`)
    // — but the project's own lane is recomposed anyway, because the
    // project may hold other packages that do.
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
    let group_dir = path
        .join(vibe_core::layout::current_packages_root())
        .join(group);
    ensure_dir(&group_dir)?;
    ctx.created(&display_pathbuf(
        group_dir.strip_prefix(&path).unwrap_or(&group_dir),
    ));
    ctx.summary(&format!("Created group `{group}` in `{}`", path.display()));
    Ok(())
}

/// Create the package directory tree for project+package (static link by default).
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
    create_package_dirs_from_fields(
        ctx,
        project_root,
        group,
        name,
        requested_link(args, default_link)?,
        &fields,
        requested_kind(args)?,
        None,
    )
}

/// Create the package directory tree from explicit fields.
#[allow(clippy::too_many_arguments)]
fn create_package_dirs_from_fields(
    ctx: &output::Context,
    project_root: &Path,
    group: &str,
    name: &str,
    link: &str,
    fields: &ProjectFields,
    package_kind: PackageKind,
    translation: Option<&Translation>,
) -> Result<Vec<Outcome>> {
    let kind = package_kind.as_str();
    let version = &fields.version;
    let pkg_dir = project_root
        .join(vibe_core::layout::current_packages_root())
        .join(group)
        .join(name)
        .join(format!("v{version}"));

    // Documentation is not a tool package with other words in it: no
    // boot lane, no snippet, and a card its siblings do not owe.
    if package_kind == PackageKind::Doc {
        return doc::create_doc_package(
            ctx,
            project_root,
            &pkg_dir,
            group,
            name,
            fields,
            translation,
        );
    }

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
        let manifest_text = nested_manifest_text(group, name, link, fields, package_kind);
        fs::write(&manifest_path, &manifest_text)?;
        ctx.created(&manifest_rel);
        outcomes.push(Outcome {
            path: manifest_rel,
            action: Action::Created,
            reason: "package manifest",
        });
    }

    // The package's boot lane, on the live layout.
    let boot_dir = pkg_dir.join(vibe_core::layout::current_boot_dir());
    ensure_dir(&boot_dir)?;
    let boot_file = boot_dir.join(format!("10-{kind}-{name}.md"));
    let boot_rel = display_pathbuf(boot_file.strip_prefix(project_root).unwrap_or(&boot_file));
    if !boot_file.exists() {
        // The provenance marker names the package by its OWN coordinate,
        // `<group>/<name>` — the spelling the generated lane uses for an
        // origin. A literal `org.` in front of an already-qualified group
        // minted `org.org.example/my-skills` for the group `org.example`,
        // and a group outside the `org.` convention was renamed outright.
        let content = format!(
            "<!-- vibe:static {group}/{name} — boot snippet -->\n\n# {name}\n\nA `{kind}` package.\n"
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

fn nested_manifest_text(
    group: &str,
    name: &str,
    link: &str,
    fields: &ProjectFields,
    kind: PackageKind,
) -> String {
    let authors_line = if fields.authors.is_empty() {
        String::new()
    } else {
        let quoted: Vec<String> = fields.authors.iter().map(|a| format!("\"{a}\"")).collect();
        format!("authors = [{}]\n", quoted.join(", "))
    };
    let kind_text = kind.as_str();
    let category = boot_category(kind);
    let boot_source = display_pathbuf(
        &vibe_core::layout::current_boot_dir().join(format!("10-{kind_text}-{name}.md")),
    );
    format!(
        "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"{kind_text}\"\n\
         version = \"{version}\"\nepoch = 1\n{authors_line}\
         license = \"{license}\"\ndescription = \"{description}\"\nformat = \"{format}\"\n\n\
         [boot_snippet]\nsource = \"{boot_source}\"\ncategory = \"{category}\"\n\
         link = \"{link}\"\n",
        version = fields.version,
        license = fields.license,
        description = fields.description,
        format = fields.format,
    )
}
