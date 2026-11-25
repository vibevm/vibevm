//! Explicit local kit recipe intake; not a second installer or network owner.
use crate::output::language::{Block, Glyph, Role, hint, paint, warning};
use crate::{
    cli::{KitArgs, KitCommand},
    output,
};
use anyhow::Result;
use serde::Deserialize;
use specmark::spec;
use std::path::{Component, Path};
use vibe_safefs::{Pinned, Project};

mod build;
mod refusal;
use refusal::refuse;

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

#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface")]
pub fn run(ctx: &output::Context, args: KitArgs) -> Result<()> {
    if ctx.is_json() {
        return Err(refuse(
            "kit_json_unsupported",
            "vibe kit has no machine report yet",
            Some("run it without --json"),
            None,
        ));
    }
    match args.command {
        KitCommand::Build { file, out } => build::run(ctx, &file, &out)?,
        KitCommand::Init { file } => {
            let (project, name) = recipe_scope(&file)?;
            let path = project.root_path().join(&name);
            #[cfg(not(windows))]
            let quoted = crate::shell_syntax::posix_quote(&path.to_string_lossy());
            #[cfg(windows)]
            let quoted = crate::shell_syntax::powershell_quote(&path.to_string_lossy());
            let template = TEMPLATE.replace("--file vibe-kit.toml", &format!("--file {quoted}"));
            project
                .publish_new_in(&project.root_dir()?, &name, template.as_bytes())
                .map_err(|_| {
                    refuse(
                        "kit_recipe_exists",
                        format!("kit recipe {} was not created", path.display()),
                        Some("choose another --file, or inspect what is already there"),
                        Some("the path is taken, or its protected publication failed"),
                    )
                })?;
            answer(
                ctx,
                &format!("created kit recipe {}", path.display()),
                &[
                    hint("demonstration paths and placeholder hashes; edit them, then:"),
                    paint(Role::Accent, &format!("vibe kit validate --file {quoted}")),
                ],
            );
        }
        KitCommand::Validate {
            file,
            structure_only,
        } => {
            let (project, name) = recipe_scope(&file)?;
            let path = project.root_path().join(&name);
            let bytes = project
                .read_file_snapshot_bounded(&name, RECIPE_LIMIT)
                .map_err(|_| {
                    refuse(
                        "kit_recipe_unreadable",
                        format!("kit recipe {} cannot be read safely", path.display()),
                        None,
                        None,
                    )
                })?
                .ok_or_else(|| {
                    refuse(
                        "kit_recipe_missing",
                        format!("kit recipe {} does not exist", path.display()),
                        Some(&format!("`vibe kit init --file {}`", refusal::quote(&path))),
                        None,
                    )
                })?;
            let text = std::str::from_utf8(&bytes.bytes).map_err(|_| {
                refuse(
                    "kit_recipe_invalid",
                    format!("kit recipe {} is not UTF-8", path.display()),
                    None,
                    None,
                )
            })?;
            let recipe: Recipe = toml::from_str(text).map_err(|_| {
                refuse(
                    "kit_recipe_invalid",
                    format!("kit recipe {} is not a valid recipe", path.display()),
                    None,
                    Some("invalid TOML or an unknown field; its contents are not shown"),
                )
            })?;
            recipe.validate_shape()?;
            if structure_only {
                answer(
                    ctx,
                    "kit recipe structure is valid",
                    &[warning("inputs not checked")],
                );
            } else {
                for (label, input) in [
                    ("bundle", &recipe.bundle),
                    ("installer", &recipe.installer),
                    ("site profile", &recipe.site_profile),
                ] {
                    verify_input(&project, input, label)?;
                }
                let profile_path = &recipe.site_profile.path;
                let profile = project
                    .read_file_snapshot_bounded(&recipe.site_profile.path, PROFILE_LIMIT)
                    .map_err(|_| refuse("kit_input_unsafe", format!("the site profile input {profile_path} is not safe, or changed while it was read"), None, None))?
                    .ok_or_else(|| refusal::missing(&project, profile_path, "site profile"))?;
                if profile.sha256 != recipe.site_profile.sha256 {
                    return Err(refuse(
                        "kit_input_mismatch",
                        format!("the site profile input {profile_path} changed during validation"),
                        None,
                        None,
                    ));
                }
                let text = std::str::from_utf8(&profile.bytes).map_err(|_| {
                    refuse(
                        "kit_profile_invalid",
                        format!("the site profile {profile_path} is not UTF-8"),
                        None,
                        None,
                    )
                })?;
                vibe_core::site_profile::SiteProfile::parse(text, "kit site profile").map_err(
                    |_| {
                        refuse(
                            "kit_profile_invalid",
                            format!("the site profile {profile_path} is not a valid site profile"),
                            None,
                            Some("its contents are not shown"),
                        )
                    },
                )?;
                answer(
                    ctx,
                    "kit recipe and its inputs are valid",
                    &[
                        hint(
                            "bundle, installer and site profile match their sha256; the site profile parses",
                        ),
                        warning(
                            "bundle compatibility is not checked; nothing was built or installed",
                        ),
                    ],
                );
            }
        }
    }
    Ok(())
}

/// A finished kit answer: the headline, then its body at column 2;
/// quiet keeps the headline alone.
fn answer(ctx: &output::Context, headline: &str, body: &[String]) {
    if ctx.is_quiet() {
        ctx.summary(headline);
        return;
    }
    let mut text = Block::new(Glyph::Done, headline).render();
    for line in body {
        text.push_str("\n  ");
        text.push_str(line);
    }
    ctx.block(&text);
}

impl Recipe {
    fn validate_shape(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(refuse(
                "kit_recipe_invalid",
                format!(
                    "kit recipe schema_version {} is not supported",
                    self.schema_version
                ),
                None,
                Some("this vibe reads schema_version 1"),
            ));
        }
        if !(!self.kit.name.is_empty()
            && self.kit.name.len() <= 64
            && self
                .kit
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        {
            return Err(refuse(
                "kit_recipe_invalid",
                "the kit name is not valid",
                None,
                Some("a kit name is 1 to 64 ASCII letters, digits, - or _"),
            ));
        }
        if !(vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS
            .contains(&self.kit.target.as_str()))
        {
            return Err(refuse(
                "kit_recipe_invalid",
                "the kit target is not a supported distribution target",
                None,
                Some(&format!(
                    "supported: {}",
                    vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS
                        .join(", ")
                )),
            ));
        }
        let version = semver::Version::parse(&self.kit.version).map_err(|_| {
            refuse(
                "kit_recipe_invalid",
                "the kit version is not semver",
                None,
                None,
            )
        })?;
        if version.to_string() != self.kit.version {
            return Err(refuse(
                "kit_recipe_invalid",
                "the kit version is not canonical semver",
                None,
                None,
            ));
        }
        let mut paths = std::collections::BTreeSet::new();
        for input in [&self.bundle, &self.installer, &self.site_profile] {
            relative_components(&input.path)?;
            if !(paths.insert(&input.path)) {
                return Err(refuse(
                    "kit_recipe_invalid",
                    "kit input paths must be distinct",
                    None,
                    None,
                ));
            }
            if !(input.sha256.len() == 64
                && input
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
            {
                return Err(refuse(
                    "kit_recipe_invalid",
                    "a kit input sha256 is not 64 lowercase hex digits",
                    None,
                    None,
                ));
            }
        }
        Ok(())
    }
}

fn relative_components(path: &str) -> Result<Vec<&str>> {
    if !(!path.is_empty() && !path.contains(['\\', ':']) && !path.chars().any(char::is_control)) {
        return Err(refuse(
            "kit_recipe_invalid",
            format!("kit path {path} is not a safe relative path"),
            None,
            Some("a recipe path is relative, uses /, and has no . or .. part"),
        ));
    }
    let parts: Vec<_> = path.split('/').collect();
    if !(parts
        .iter()
        .all(|p| !p.is_empty() && *p != "." && *p != ".."))
    {
        return Err(refuse(
            "kit_recipe_invalid",
            format!("kit path {path} is not a safe relative path"),
            None,
            Some("a recipe path is relative, uses /, and has no . or .. part"),
        ));
    }
    if !(Path::new(path)
        .components()
        .all(|c| matches!(c, Component::Normal(_))))
    {
        return Err(refuse(
            "kit_recipe_invalid",
            format!("kit path {path} is not a safe relative path"),
            None,
            Some("a recipe path is relative, uses /, and has no . or .. part"),
        ));
    }
    Ok(parts)
}

fn recipe_scope(file: &Path) -> Result<(Project, String)> {
    if !(!file.components().any(|c| matches!(c, Component::ParentDir))) {
        return Err(refuse(
            "kit_path_unsafe",
            format!("kit path {} must not contain a .. part", file.display()),
            None,
            None,
        ));
    }
    let absolute = std::path::absolute(file).map_err(|_| {
        refuse(
            "kit_path_unsafe",
            format!("kit path {} cannot be made absolute", file.display()),
            None,
            None,
        )
    })?;
    let name = absolute
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            refuse(
                "kit_path_unsafe",
                format!("kit path {} has no usable file name", file.display()),
                None,
                None,
            )
        })?
        .to_owned();
    let parent = absolute.parent().ok_or_else(|| {
        refuse(
            "kit_path_unsafe",
            format!("kit path {} has no parent directory", file.display()),
            None,
            None,
        )
    })?;
    // Project::open already walks every absolute ancestor without following links.
    // Reuse that authority instead of implementing a second absolute-path walker.
    let project = Project::open(parent).map_err(|_| {
        refuse(
            "kit_path_unsafe",
            format!(
                "the parent directory of {} is missing or unsafe",
                absolute.display()
            ),
            None,
            Some("kit creates no parent directories and follows no links"),
        )
    })?;
    Ok((project, name))
}

fn input_holder(project: &Project, path: &str, label: &str) -> Result<(Pinned, String)> {
    let mut parts = relative_components(path)?;
    let name = parts
        .pop()
        .ok_or_else(|| {
            refuse(
                "kit_recipe_invalid",
                format!("kit input path {path} has no file name"),
                None,
                None,
            )
        })?
        .to_owned();
    let mut holder = project.root_dir()?;
    for part in parts {
        holder = holder
            .open_child_checked(part)
            .map_err(|_| {
                refuse(
                    "kit_input_unsafe",
                    format!("a directory on the kit input path {path} is not safe"),
                    None,
                    Some("kit follows no links"),
                )
            })?
            .ok_or_else(|| refusal::missing(project, path, label))?;
    }
    Ok((holder, name))
}
fn verify_input(project: &Project, input: &Input, label: &str) -> Result<()> {
    let (holder, name) = input_holder(project, &input.path, label)?;
    let actual = project
        .digest_file_in(&holder, &name)
        .map_err(|_| {
            refuse(
                "kit_input_unsafe",
                format!(
                    "the {label} input {} is not safe, or changed while it was read",
                    input.path
                ),
                None,
                None,
            )
        })?
        .ok_or_else(|| refusal::missing(project, &input.path, label))?;
    let hex: String = actual.sha256.iter().map(|b| format!("{b:02x}")).collect();
    if hex != input.sha256 {
        return Err(refuse(
            "kit_input_mismatch",
            format!(
                "the {label} input {} does not match its sha256 in the recipe",
                input.path
            ),
            Some("put the intended file there, or record its sha256 in the recipe"),
            None,
        ));
    }
    Ok(())
}
