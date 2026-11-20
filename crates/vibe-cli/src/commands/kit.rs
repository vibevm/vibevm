//! Explicit local kit recipe intake; not a second installer or network owner.
use crate::{
    cli::{KitArgs, KitCommand},
    output,
};
use anyhow::{Result, anyhow, ensure};
use serde::Deserialize;
use std::path::{Component, Path};
use vibe_safefs::{Pinned, Project};

mod build;

const TEMPLATE: &str = include_str!("kit/template.toml");
const RECIPE_LIMIT: usize = 64 * 1024;
const PROFILE_LIMIT: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    schema_version: u32,
    kit: Kit,
    bundle: Input,
    installer: Input,
    site_profile: Input,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Kit {
    name: String,
    target: String,
    version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: String,
    sha256: String,
}

pub fn run(ctx: &output::Context, args: KitArgs) -> Result<()> {
    ensure!(
        !ctx.is_json(),
        "kit commands do not expose a machine report protocol yet"
    );
    match args.command {
        KitCommand::Build { file, out } => build::run(ctx, &file, &out)?,
        KitCommand::Init { file } => {
            let (project, name) = recipe_scope(&file)?;
            project.publish_new_in(&project.root_dir()?, &name, TEMPLATE.as_bytes())
                .map_err(|_| anyhow!("kit recipe not created: existing entry or protected publication failure; inspect destination before retry"))?;
            ctx.summary("Created commented kit recipe; demonstration paths and hashes are not verified inputs.");
        }
        KitCommand::Validate {
            file,
            structure_only,
        } => {
            let (project, name) = recipe_scope(&file)?;
            let bytes = project
                .read_file_snapshot_bounded(&name, RECIPE_LIMIT)
                .map_err(|_| anyhow!("kit recipe unavailable or unsafe"))?
                .ok_or_else(|| anyhow!("kit recipe missing"))?;
            let text = std::str::from_utf8(&bytes.bytes)
                .map_err(|_| anyhow!("kit recipe must be UTF-8"))?;
            let recipe: Recipe = toml::from_str(text).map_err(|_| {
                anyhow!("invalid kit recipe TOML or unknown fields; raw input omitted")
            })?;
            recipe.validate_shape()?;
            if structure_only {
                ctx.summary(
                    "Kit recipe structure valid; input availability and readiness NOT checked.",
                );
            } else {
                for (label, input) in [
                    ("bundle", &recipe.bundle),
                    ("installer", &recipe.installer),
                    ("site profile", &recipe.site_profile),
                ] {
                    verify_input(&project, input, label)?;
                }
                let profile = project
                    .read_file_snapshot_bounded(&recipe.site_profile.path, PROFILE_LIMIT)
                    .map_err(|_| anyhow!("kit site profile unavailable or unsafe"))?
                    .ok_or_else(|| anyhow!("kit site profile missing"))?;
                ensure!(
                    profile.sha256 == recipe.site_profile.sha256,
                    "kit site profile changed during validation"
                );
                let text = std::str::from_utf8(&profile.bytes)
                    .map_err(|_| anyhow!("kit site profile must be UTF-8"))?;
                vibe_core::site_profile::SiteProfile::parse(text, "kit site profile")
                    .map_err(|_| anyhow!("invalid kit site profile; contents omitted"))?;
                ctx.summary("Kit recipe and local input hashes/profile valid; no kit built or installed, bundle compatibility not yet qualified.");
            }
        }
    }
    Ok(())
}

impl Recipe {
    fn validate_shape(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "unsupported kit recipe schema_version"
        );
        ensure!(
            !self.kit.name.is_empty()
                && self.kit.name.len() <= 64
                && self
                    .kit
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "invalid kit name"
        );
        ensure!(
            self.kit.target == "x86_64-unknown-linux-gnu",
            "unsupported kit target; first slice requires x86_64-unknown-linux-gnu"
        );
        let version = semver::Version::parse(&self.kit.version)
            .map_err(|_| anyhow!("invalid kit version"))?;
        ensure!(
            version.to_string() == self.kit.version,
            "kit version must be canonical semver"
        );
        let mut paths = std::collections::BTreeSet::new();
        for input in [&self.bundle, &self.installer, &self.site_profile] {
            relative_components(&input.path)?;
            ensure!(
                paths.insert(&input.path),
                "kit input paths must be distinct"
            );
            ensure!(
                input.sha256.len() == 64
                    && input
                        .sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "kit input sha256 must be 64 lowercase hex digits"
            );
        }
        Ok(())
    }
}

fn relative_components(path: &str) -> Result<Vec<&str>> {
    ensure!(
        !path.is_empty() && !path.contains(['\\', ':']) && !path.chars().any(char::is_control),
        "unsafe kit relative path"
    );
    let parts: Vec<_> = path.split('/').collect();
    ensure!(
        parts
            .iter()
            .all(|p| !p.is_empty() && *p != "." && *p != ".."),
        "unsafe kit relative path"
    );
    ensure!(
        Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "unsafe kit relative path"
    );
    Ok(parts)
}

fn recipe_scope(file: &Path) -> Result<(Project, String)> {
    ensure!(
        !file.components().any(|c| matches!(c, Component::ParentDir)),
        "kit recipe path must not traverse parents"
    );
    let absolute = std::path::absolute(file).map_err(|_| anyhow!("kit recipe path unavailable"))?;
    let name = absolute
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("kit recipe filename invalid"))?
        .to_owned();
    let parent = absolute
        .parent()
        .ok_or_else(|| anyhow!("kit recipe parent missing"))?;
    // Project::open already walks every absolute ancestor without following links.
    // Reuse that authority instead of implementing a second absolute-path walker.
    let project = Project::open(parent)
        .map_err(|_| anyhow!("kit recipe parent missing or unsafe; parents are not created"))?;
    Ok((project, name))
}

fn input_holder(project: &Project, path: &str) -> Result<(Pinned, String)> {
    let mut parts = relative_components(path)?;
    let name = parts
        .pop()
        .ok_or_else(|| anyhow!("kit input name missing"))?
        .to_owned();
    let mut holder = project.root_dir()?;
    for part in parts {
        holder = holder
            .open_child_checked(part)
            .map_err(|_| anyhow!("kit input ancestor unsafe"))?
            .ok_or_else(|| anyhow!("kit input ancestor missing"))?;
    }
    Ok((holder, name))
}
fn verify_input(project: &Project, input: &Input, label: &str) -> Result<()> {
    let (holder, name) = input_holder(project, &input.path)?;
    let actual = project
        .digest_file_in(&holder, &name)
        .map_err(|_| anyhow!("kit {label} input unsafe or changed"))?
        .ok_or_else(|| anyhow!("kit {label} input missing"))?;
    let hex: String = actual.sha256.iter().map(|b| format!("{b:02x}")).collect();
    ensure!(hex == input.sha256, "kit {label} input hash mismatch");
    Ok(())
}
