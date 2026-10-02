//! Field collection for init. Callers suspend progress around every dialogue.

use crate::cli::{InitArgs, InitType};
use anyhow::Result;
use dialoguer::{Input, Select};
use vibe_core::{PackageKind, user_config::UserConfig};

pub(super) fn identity_introduction(ctx: &crate::output::Context) {
    ctx.summary("Ваш проект или пакет будет называться org.acme/package-name. «org.acme» называется «группой», «package-name» — это имя внутри группы.");
}

pub(super) struct ProjectFields {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: String,
    pub description: String,
    pub format: String,
}

pub(super) fn project_fields_from_args(
    args: &InitArgs,
    default_name: &str,
    user_config: &UserConfig,
) -> ProjectFields {
    let authors = if !args.authors.is_empty() {
        args.authors.clone()
    } else {
        user_config
            .init
            .last_author
            .clone()
            .or_else(|| {
                let author = detect_git_author();
                (!author.is_empty()).then_some(author)
            })
            .into_iter()
            .collect()
    };
    ProjectFields {
        name: args
            .name
            .clone()
            .unwrap_or_else(|| default_name.to_string()),
        version: args.version.clone().unwrap_or_else(|| "0.0.1".to_string()),
        authors,
        license: args
            .license
            .clone()
            .unwrap_or_else(|| "UPL-1.0".to_string()),
        description: args.description.clone().unwrap_or_default(),
        format: args.format.clone().unwrap_or_else(|| "normal".to_string()),
    }
}

/// Supplied metadata is authoritative; only missing values ask questions.
pub(super) fn prompt_fields(
    args: &InitArgs,
    mut fields: ProjectFields,
    role: InitType,
) -> Result<ProjectFields> {
    if args.name.is_none() {
        fields.name = text(
            if role == InitType::Package {
                "Package name"
            } else {
                "Project name"
            },
            &fields.name,
            false,
        )?;
    }
    if args.version.is_none() {
        fields.version = text("Version", &fields.version, false)?;
    }
    if args.authors.is_empty() {
        let author = text(
            "Author",
            fields.authors.first().map_or("", String::as_str),
            true,
        )?;
        fields.authors = (!author.trim().is_empty())
            .then(|| author.trim().to_string())
            .into_iter()
            .collect();
    }
    if args.license.is_none() {
        fields.license = choice(
            "License",
            &["UPL-1.0", "MIT", "Apache-2.0", "Proprietary"],
            &fields.license,
        )?;
    }
    if args.description.is_none() {
        fields.description = text("Description", "", true)?;
    }
    if args.format.is_none() {
        fields.format = choice("Format", &["normal", "simple"], &fields.format)?;
    }
    Ok(fields)
}

pub(super) fn text(label: &str, default: &str, empty: bool) -> Result<String> {
    Ok(Input::<String>::new()
        .with_prompt(label)
        .default(default.to_string())
        .show_default(true)
        .allow_empty(empty)
        .interact_text()?)
}

fn choice(label: &str, items: &[&str], default: &str) -> Result<String> {
    let selected = Select::new()
        .with_prompt(label)
        .items(items)
        .default(items.iter().position(|item| *item == default).unwrap_or(0))
        .interact()?;
    Ok(items[selected].to_string())
}

pub(super) fn prompt_kind(args: &InitArgs) -> Result<PackageKind> {
    if args.kind.is_some() {
        return super::package::requested_kind(args);
    }
    use std::str::FromStr;
    Ok(PackageKind::from_str(&choice(
        "Package kind",
        &["tool", "flow", "feat", "stack", "mcp", "lang", "doc", "app"],
        "tool",
    )?)?)
}

pub(super) fn prompt_package_fields(
    args: &InitArgs,
    group: &str,
    name: &str,
    user_config: &UserConfig,
) -> Result<(ProjectFields, PackageKind)> {
    let fields = package_fields_from_args(args, group, name, user_config);
    // The positional identity is authoritative for the legacy nested form.
    let fixed_identity = InitArgs {
        name: Some(name.to_string()),
        ..args.clone()
    };
    let prompted = prompt_fields(&fixed_identity, fields, InitType::Package)?;
    Ok((prompted, prompt_kind(args)?))
}

pub(super) fn package_fields_from_args(
    args: &InitArgs,
    _group: &str,
    name: &str,
    user_config: &UserConfig,
) -> ProjectFields {
    let mut fields = project_fields_from_args(args, name, user_config);
    fields.name = name.to_string();
    if args.version.is_none() {
        fields.version = "0.1.0".to_string();
    }
    fields
}

pub(super) fn maybe_save_author(user_config: &mut UserConfig, authors: &[String]) {
    if let Some(first) = authors.first()
        && user_config.init.last_author.as_deref() != Some(first.as_str())
    {
        user_config.init.last_author = Some(first.clone());
        let _ = user_config.save();
    }
}

/// Detect the git user name for the author default.
pub(super) fn detect_git_author() -> String {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}
