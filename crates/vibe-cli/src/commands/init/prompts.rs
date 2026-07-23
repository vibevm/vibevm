//! Interactive prompts for `vibe init` — npm-style field collection.
//!
//! When `vibe init` runs in a TTY (interactive), the user is prompted for
//! project fields (name, version, author, license, description, format).
//! These values populate both the project `vibe.toml` and become defaults
//! for the package fields (when a package is also being created).
//!
//! In non-interactive mode (`--unattended`, no TTY, JSON), values come
//! from CLI flags or built-in defaults.

use anyhow::Result;
use dialoguer::{Input, Select};
use vibe_core::user_config::UserConfig;

use crate::cli::InitArgs;

/// Collected field values for project creation.
pub(super) struct ProjectFields {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: String,
    pub description: String,
    pub format: String,
}

/// Prompt the user for project fields interactively.
pub(super) fn prompt_project_fields(
    default_name: &str,
    user_config: &UserConfig,
) -> Result<ProjectFields> {
    let name: String = Input::<String>::new()
        .with_prompt("Project name")
        .default(default_name.to_string())
        .show_default(true)
        .interact_text()?;

    let version: String = Input::<String>::new()
        .with_prompt("Version")
        .default("0.1.0".to_string())
        .show_default(true)
        .interact_text()?;

    let author_default = user_config
        .init
        .last_author
        .clone()
        .unwrap_or_else(detect_git_author);
    let author: String = Input::<String>::new()
        .with_prompt("Author")
        .default(author_default)
        .show_default(true)
        .allow_empty(true)
        .interact_text()?;

    let license_items = vec!["UPL-1.0", "MIT", "Apache-2.0", "Proprietary"];
    let license_sel = Select::new()
        .with_prompt("License")
        .items(&license_items)
        .default(0)
        .interact()?;
    let license = license_items[license_sel].to_string();

    let description: String = Input::<String>::new()
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()?;

    let format_items = vec!["normal", "simple"];
    let format_sel = Select::new()
        .with_prompt("Format")
        .items(&format_items)
        .default(0)
        .interact()?;
    let format = format_items[format_sel].to_string();

    let authors = if author.trim().is_empty() {
        Vec::new()
    } else {
        vec![author.trim().to_string()]
    };

    Ok(ProjectFields {
        name,
        version,
        authors,
        license,
        description,
        format,
    })
}

/// Build project fields from CLI flags + defaults (non-interactive).
pub(super) fn project_fields_from_args(
    args: &InitArgs,
    default_name: &str,
    user_config: &UserConfig,
) -> ProjectFields {
    let authors = if !args.authors.is_empty() {
        args.authors.clone()
    } else if let Some(last) = &user_config.init.last_author {
        vec![last.clone()]
    } else {
        let git_name = detect_git_author();
        if git_name.is_empty() {
            Vec::new()
        } else {
            vec![git_name]
        }
    };

    ProjectFields {
        name: args
            .name
            .clone()
            .unwrap_or_else(|| default_name.to_string()),
        version: args.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        authors,
        license: args
            .license
            .clone()
            .unwrap_or_else(|| "UPL-1.0".to_string()),
        description: args.description.clone().unwrap_or_default(),
        format: args.format.clone().unwrap_or_else(|| "normal".to_string()),
    }
}

/// Save `last_author` to user config if it changed.
pub(super) fn maybe_save_author(user_config: &mut UserConfig, authors: &[String]) {
    if let Some(first) = authors.first() {
        let current = &user_config.init.last_author;
        if current.as_deref() != Some(first.as_str()) {
            user_config.init.last_author = Some(first.clone());
            let _ = user_config.save();
        }
    }
}

/// Prompt the user for package fields interactively (after project fields).
pub(super) fn prompt_package_fields(
    _group: &str,
    name: &str,
    user_config: &UserConfig,
) -> Result<ProjectFields> {
    let kind_items = vec!["tool", "flow", "feat", "stack", "mcp"];
    let _kind_sel = Select::new()
        .with_prompt("Package kind")
        .items(&kind_items)
        .default(0)
        .interact()?;
    // kind is always "tool" for now (create_package_dirs_from_fields uses it).

    let version: String = Input::<String>::new()
        .with_prompt("Package version")
        .default("0.1.0".to_string())
        .show_default(true)
        .interact_text()?;

    let author_default = user_config
        .init
        .last_author
        .clone()
        .unwrap_or_else(detect_git_author);
    let author: String = Input::<String>::new()
        .with_prompt("Author")
        .default(author_default)
        .show_default(true)
        .allow_empty(true)
        .interact_text()?;

    let license_items = vec!["UPL-1.0", "MIT", "Apache-2.0", "Proprietary"];
    let license_sel = Select::new()
        .with_prompt("License")
        .items(&license_items)
        .default(0)
        .interact()?;
    let license = license_items[license_sel].to_string();

    let description: String = Input::<String>::new()
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()?;

    let format_items = vec!["normal", "simple"];
    let format_sel = Select::new()
        .with_prompt("Format")
        .items(&format_items)
        .default(0)
        .interact()?;
    let format = format_items[format_sel].to_string();

    let authors = if author.trim().is_empty() {
        Vec::new()
    } else {
        vec![author.trim().to_string()]
    };

    Ok(ProjectFields {
        name: name.to_string(),
        version,
        authors,
        license,
        description,
        format,
    })
}

/// Build package fields from CLI flags + defaults (non-interactive).
pub(super) fn package_fields_from_args(
    args: &InitArgs,
    _group: &str,
    name: &str,
    user_config: &UserConfig,
) -> ProjectFields {
    let authors = if !args.authors.is_empty() {
        args.authors.clone()
    } else if let Some(last) = &user_config.init.last_author {
        vec![last.clone()]
    } else {
        let git_name = detect_git_author();
        if git_name.is_empty() {
            Vec::new()
        } else {
            vec![git_name]
        }
    };

    ProjectFields {
        name: name.to_string(),
        version: args.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        authors,
        license: args
            .license
            .clone()
            .unwrap_or_else(|| "UPL-1.0".to_string()),
        description: args.description.clone().unwrap_or_default(),
        format: args.format.clone().unwrap_or_else(|| "normal".to_string()),
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
