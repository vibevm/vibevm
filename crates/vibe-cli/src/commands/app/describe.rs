//! `vibe app steps` and `vibe app describe` (PROP-065 `#record`,
//! `#describe`): the declared choice of a package, rendered without
//! resolving or running anything.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#describe");

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use vibe_core::manifest::{FeatureDecl, StepDecl, steps_of};
use vibe_wire::generated::application_describe::ApplicationDescribe;

use super::global;
use crate::output;

/// Where `show` resolves its package from: an explicit local registry or the
/// operation's sources, with the embedded root and the offline posture.
pub(super) struct DescribeSource<'a> {
    pub registry: Option<&'a Path>,
    pub embedded_root: Option<&'a Path>,
    pub offline: bool,
    pub operation: &'a vibe_package_source::SourceOperation,
}

/// Resolve `package` and print its steps as written (`steps_only`) or the
/// describe outline; `--json` prints the describe document for both.
pub(super) fn show(
    ctx: &output::Context,
    source: DescribeSource<'_>,
    package: &str,
    steps_only: bool,
    configuration: bool,
) -> Result<()> {
    let settings = global::settings_root()?;
    let resolved = global::resolve_package_with_operation(
        &settings,
        source.registry,
        source.embedded_root,
        package,
        source.offline,
        source.operation,
    )?;
    if configuration {
        let mut document = vibe_app::configuration::describe(&resolved);
        document.installed_source = vibe_app::Index::load(&settings)?
            .installations
            .get(&resolved.package.key())
            .and_then(|r| r.source_evidence.clone());
        document.installed_hosting =
            vibe_registry::source_hosting(document.installed_source.as_ref());
        if ctx.is_json() {
            return ctx.emit_json(&document);
        }
        println!("{}", serde_json::to_string_pretty(&document)?);
        return Ok(());
    }
    let mut document = vibe_app::describe(&resolved)?;
    document.installed_source = vibe_app::Index::load(&settings)?
        .installations
        .get(&resolved.package.key())
        .and_then(|r| r.source_evidence.clone());
    document.installed_hosting = vibe_registry::source_hosting(document.installed_source.as_ref());
    if ctx.is_json() {
        return ctx.emit_json(&document);
    }
    if !steps_only {
        print_outline(&document);
        return Ok(());
    }
    #[derive(Serialize)]
    struct Written<'a> {
        #[serde(skip_serializing_if = "<[FeatureDecl]>::is_empty")]
        features: &'a [FeatureDecl],
        steps: Vec<StepDecl>,
    }
    for option in &resolved.options {
        let form = if option.installation.is_some() {
            "method form, shown as its steps"
        } else {
            "steps"
        };
        println!(
            "# installer `{}` ({}/{}), {form}",
            option.id, option.platform, option.arch
        );
        let written = Written {
            features: &option.features,
            steps: steps_of(option),
        };
        println!("{}", toml::to_string(&written)?);
    }
    Ok(())
}

fn print_outline(document: &ApplicationDescribe) {
    let package = &document.package;
    println!(
        "{}/{}@{} — application `{}`, {} installer option(s)",
        package.group,
        package.name,
        package.version,
        document.application.id,
        document.options.len()
    );
    for option in &document.options {
        let result = match &option.app_root {
            Some(root) => format!("external `{root}`"),
            None => "owned".into(),
        };
        println!(
            "  {} — {} ({}/{}), result {result}, portable {}",
            option.id, option.name, option.platform, option.arch, option.portable
        );
        for feature in &option.features {
            let default = if feature.default { " [default]" } else { "" };
            println!("    feature {}: {}{default}", feature.id, feature.name);
        }
        for (index, step) in option.steps.iter().enumerate() {
            let optional = if step.optional { ", optional" } else { "" };
            let phase = match step.phase {
                vibe_wire::generated::application_describe::Phase::Prepare => "",
                vibe_wire::generated::application_describe::Phase::Finish => ", finish",
            };
            println!(
                "    step {}. {} ({}{phase}{optional}): {}",
                index + 1,
                step.id,
                step.kind,
                step.title
            );
        }
        for pattern in &option.launchers_from {
            println!("    launchers from {pattern}");
        }
        for launcher in &option.launchers {
            let menu = if launcher.menu { ", menu" } else { "" };
            let kind = launcher
                .kind
                .as_ref()
                .and_then(|kind| serde_json::to_value(kind).ok())
                .and_then(|kind| kind.as_str().map(str::to_owned))
                .unwrap_or_else(|| "override".into());
            println!(
                "    launcher {} ({kind}{menu}): {}",
                launcher.id, launcher.executable
            );
        }
        for command in &option.public_commands {
            println!("    command {} → {}", command.name, command.launcher);
        }
    }
}
