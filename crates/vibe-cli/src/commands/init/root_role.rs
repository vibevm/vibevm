//! Pure planning of the root identity, before any scaffold or settings write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE");

use std::path::Path;

use anyhow::{Context, Result, bail};
use dialoguer::Select;
use vibe_core::user_config::UserConfig;
use vibe_core::{
    PackageKind,
    manifest::{Manifest, ProjectSection},
};

use super::{
    prompts::{self, ProjectFields},
    validate_group,
};
use crate::{
    cli::{InitArgs, InitType},
    output,
};

pub(super) struct RootPlan {
    pub role: InitType,
    pub fields: ProjectFields,
    pub manifest: Manifest,
}

pub(super) fn plan(
    ctx: &output::Context,
    args: &InitArgs,
    root: &Path,
    user_config: &UserConfig,
    interactive: bool,
) -> Result<RootPlan> {
    if args.translates.is_some() {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: `--translates` requires the nested `vibe init package <group>/<name>` form; fix surface: init package --kind doc --translates"
        );
    }
    let manifest_path = root.join(Manifest::FILENAME);
    if manifest_path.exists() {
        return existing(args, Manifest::read(&manifest_path)?);
    }
    let default_name = super::helpers::resolve_name(args, root)?;
    let fields = prompts::project_fields_from_args(args, &default_name, user_config);
    let (role, fields, group, kind) = if interactive {
        ctx.suspend_progress(|| {
            let mut dialogue = TerminalIdentity { ctx };
            let (role, fields, group) = collect_identity(args, fields, &mut dialogue)?;
            let fixed_identity = InitArgs {
                name: Some(fields.name.clone()),
                ..args.clone()
            };
            let fields = prompts::prompt_fields(&fixed_identity, fields, role)?;
            let kind = if role == InitType::Package {
                prompts::prompt_kind(args)?
            } else {
                super::package::requested_kind(args)?
            };
            Ok::<_, anyhow::Error>((role, fields, Some(group), kind))
        })?
    } else {
        let role = args.init_type.unwrap_or(InitType::Project);
        let mut fields = fields;
        if role == InitType::Package && args.version.is_none() {
            fields.version = "0.1.0".to_string();
        }
        (
            role,
            fields,
            args.group.clone(),
            super::package::requested_kind(args)?,
        )
    };
    validate_fields(&fields, role)?;
    if let Some(group) = &group {
        validate_group(group)?;
    }
    let manifest = new_manifest(args, role, group.as_deref(), kind, &fields)?;
    Ok(RootPlan {
        role,
        fields,
        manifest,
    })
}

/// The identity dialogue finishes before metadata and before any writes.
trait IdentityDialogue {
    fn introduction(&mut self);
    fn group(&mut self) -> Result<String>;
    fn name(&mut self, default: &str) -> Result<String>;
    fn role(&mut self) -> Result<InitType>;
}

struct TerminalIdentity<'a> {
    ctx: &'a output::Context,
}

impl IdentityDialogue for TerminalIdentity<'_> {
    fn introduction(&mut self) {
        prompts::identity_introduction(self.ctx);
    }

    fn group(&mut self) -> Result<String> {
        prompts::text("Group (reverse-DNS)", "", false)
    }

    fn name(&mut self, default: &str) -> Result<String> {
        prompts::text("Name within the group", default, false)
    }

    fn role(&mut self) -> Result<InitType> {
        let selected = Select::new()
            .with_prompt("Root declaration")
            .items(["project", "package"])
            .default(0)
            .interact()?;
        Ok(if selected == 0 {
            InitType::Project
        } else {
            InitType::Package
        })
    }
}

fn collect_identity(
    args: &InitArgs,
    mut fields: ProjectFields,
    dialogue: &mut impl IdentityDialogue,
) -> Result<(InitType, ProjectFields, String)> {
    dialogue.introduction();
    let group = match &args.group {
        Some(group) => group.clone(),
        None => dialogue.group()?.trim().to_string(),
    };
    validate_group(&group)?;
    if args.name.is_none() {
        fields.name = dialogue.name(&fields.name)?;
    }
    if fields.name.trim().is_empty() {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: initialization name must not be empty; fix surface: init --name or the name prompt"
        );
    }
    let role = match args.init_type {
        Some(role) => role,
        None => dialogue.role()?,
    };
    if role == InitType::Package {
        vibe_core::PackageName::parse(&fields.name).context("violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: invalid initialization name; fix surface: init --name")?;
        if args.version.is_none() {
            fields.version = "0.1.0".to_string();
        }
    }
    Ok((role, fields, group))
}

pub(super) fn validate_fields(fields: &ProjectFields, role: InitType) -> Result<()> {
    if role == InitType::Package {
        vibe_core::PackageName::parse(&fields.name).context("violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: invalid initialization name; fix surface: init --name")?;
    } else if fields.name.trim().is_empty() {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: project name must not be empty; fix surface: init --name"
        );
    }
    semver::Version::parse(&fields.version).context("violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: invalid initialization version; fix surface: init --version")?;
    if !matches!(fields.format.as_str(), "normal" | "simple") {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: invalid initialization format `{}`; expected normal or simple; fix surface: init --format",
            fields.format
        );
    }
    Ok(())
}

fn new_manifest(
    args: &InitArgs,
    role: InitType,
    group: Option<&str>,
    kind: PackageKind,
    fields: &ProjectFields,
) -> Result<Manifest> {
    let mut manifest = Manifest {
        active: args
            .stack
            .as_ref()
            .map(|stack| vibe_core::manifest::ActiveSection {
                stack: Some(stack.clone()),
            }),
        registries: super::resolve_registry_sections(args),
        ..Default::default()
    };
    match role {
        InitType::Project => {
            manifest.project = Some(ProjectSection {
                name: fields.name.clone(),
                group: group.map(vibe_core::Group::parse).transpose()?,
                version: fields.version.clone(),
                spec_format: None,
                sources: Vec::new(),
                authors: fields.authors.clone(),
            })
        }
        InitType::Package => {
            let group = group.context("violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: a package root requires `--group <reverse-DNS>`; fix surface: init --group")?;
            if kind == PackageKind::Doc {
                let doc = super::doc::root_manifest(group, &fields.name, fields)?;
                manifest.package = doc.package;
                manifest.documents = doc.documents;
                manifest.i18n = doc.i18n;
            } else {
                let mut table = toml::Table::new();
                for (key, value) in [
                    ("name", fields.name.as_str()),
                    ("group", group),
                    ("kind", kind.as_str()),
                    ("version", fields.version.as_str()),
                    ("license", fields.license.as_str()),
                    ("description", fields.description.as_str()),
                    ("format", fields.format.as_str()),
                ] {
                    table.insert(key.to_string(), toml::Value::String(value.to_string()));
                }
                table.insert("epoch".into(), toml::Value::Integer(1));
                table.insert(
                    "authors".into(),
                    toml::Value::Array(
                        fields
                            .authors
                            .iter()
                            .cloned()
                            .map(toml::Value::String)
                            .collect(),
                    ),
                );
                manifest.package = Some(toml::Value::Table(table).try_into()?);
            }
        }
    }
    manifest.validate()?;
    Ok(manifest)
}

fn existing(args: &InitArgs, manifest: Manifest) -> Result<RootPlan> {
    let (role, name, group, version, authors, license, description, format) = if let Some(package) =
        &manifest.package
    {
        (
            InitType::Package,
            package.name.clone(),
            Some(package.group.as_str()),
            package.version.to_string(),
            package.authors.clone(),
            package.license.clone().unwrap_or_default(),
            package.description.clone().unwrap_or_default(),
            match package.format {
                vibe_core::manifest::PackageFormat::Normal => "normal",
                vibe_core::manifest::PackageFormat::Simple => "simple",
            }
            .to_string(),
        )
    } else if let Some(project) = &manifest.project {
        (
            InitType::Project,
            project.name.clone(),
            project.group.as_ref().map(vibe_core::Group::as_str),
            project.version.clone(),
            project.authors.clone(),
            String::new(),
            String::new(),
            "normal".to_string(),
        )
    } else {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: existing root has no project or package identity; init does not convert workspace roles; fix surface: use a fresh init directory"
        );
    };
    if args.init_type.is_some_and(|requested| requested != role) {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: existing root declaration is {role:?}; init does not convert project/package roles; fix surface: init --type or use a fresh directory"
        );
    }
    let requested_group = args
        .group
        .as_deref()
        .map(|group| {
            validate_group(group)?;
            Ok::<_, anyhow::Error>(vibe_core::Group::parse(group)?)
        })
        .transpose()?;
    if requested_group
        .as_ref()
        .is_some_and(|requested| Some(requested.as_str()) != group)
        || args
            .name
            .as_deref()
            .is_some_and(|requested| requested != name)
    {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-024#INIT-ROOT-ROLE: requested group/name differs from the existing root identity; init preserves authored manifests; fix surface: init --group/--name or use a fresh directory"
        );
    }
    let requested = prompts::project_fields_from_args(args, &name, &UserConfig::default());
    validate_fields(&requested, role)?;
    super::package::requested_kind(args)?;
    Ok(RootPlan {
        role,
        fields: ProjectFields {
            name,
            version,
            authors,
            license,
            description,
            format,
        },
        manifest,
    })
}

#[cfg(test)]
#[path = "root_role/tests.rs"]
mod tests;
