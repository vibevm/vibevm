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
    /// The home whose installed applications a short name prefers.
    pub home: &'a vibe_app::SettingsRoot,
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
    // A bare `<name>[@<version>]` is qualified here, once, against the very
    // sources this resolution is about to walk (PROP-008 `#short-name`).
    let package = &super::short_name::from_registries(
        ctx,
        if steps_only {
            "vibe app steps"
        } else {
            "vibe app describe"
        },
        &super::short_name::Sources {
            home: source.home,
            settings: &settings,
            inputs: super::short_name::Inputs {
                registry: source.registry,
                embedded: source.embedded_root,
                offline: source.offline,
                operation: source.operation,
            },
        },
        package,
    )?;
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
    let index = vibe_app::Index::load(&settings)?;
    let installed = index.installations.get(&resolved.package.key());
    document.installed_source = installed.and_then(|r| r.source_evidence.clone());
    document.installed_hosting = vibe_registry::source_hosting(document.installed_source.as_ref());
    // What each declared ecosystem adapter last applied and last observed
    // in the installed version, off the inventory's own record: nothing is
    // run to answer `describe` (PROP-075 `#SLOT-EXTRAS`, `AV-FMT-032`).
    if let Some(record) = installed {
        vibe_app::describe_installed_extras(&mut document, record);
    }
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
    println!(
        "{}",
        render_outline(document, crate::output::language::viewport())
    );
}

fn render_outline(document: &ApplicationDescribe, width: usize) -> String {
    use crate::output::language::{Cell, List, Row, rule};

    let package = &document.package;
    let count = document.options.len();
    let plural = if count == 1 { "" } else { "s" };
    let heading = rule(
        &format!(
            "{}/{}@{} · slot {} · {count} installer option{plural}",
            package.group, package.name, package.version, document.application.id
        ),
        None,
    );
    let mut rows = Vec::new();
    for option in &document.options {
        let result = match &option.app_root {
            Some(root) => format!("external `{root}`"),
            None => "owned".into(),
        };
        let portable = if option.portable {
            "portable"
        } else {
            "not portable"
        };
        let mut row = Row::new(vec![
            Cell::text(&option.id),
            Cell::text(&option.name),
            Cell::text(format!("{}/{}", option.platform, option.arch)),
            Cell::text(format!("· result {result} · {portable}")),
        ]);
        for feature in &option.features {
            let default = if feature.default { " [default]" } else { "" };
            row = row.child(format!("feature {}: {}{default}", feature.id, feature.name));
        }
        for (index, step) in option.steps.iter().enumerate() {
            let optional = if step.optional { ", optional" } else { "" };
            let phase = match step.phase {
                vibe_wire::generated::application_describe::Phase::Prepare => "",
                vibe_wire::generated::application_describe::Phase::Finish => ", finish",
            };
            row = row.child(format!(
                "{}. {}  ({}{phase}{optional})  {}",
                index + 1,
                step.id,
                step.kind,
                step.title
            ));
        }
        for pattern in &option.launchers_from {
            row = row.child(format!("launchers from {pattern}"));
        }
        for launcher in &option.launchers {
            let menu = if launcher.menu { ", menu" } else { "" };
            let default = if launcher.default { ", default" } else { "" };
            let kind = launcher
                .kind
                .as_ref()
                .and_then(|kind| serde_json::to_value(kind).ok())
                .and_then(|kind| kind.as_str().map(str::to_owned))
                .unwrap_or_else(|| "override".into());
            row = row.child(format!(
                "launcher {} ({kind}{menu}{default}) → {}",
                launcher.id, launcher.executable
            ));
        }
        for command in &option.public_commands {
            row = row.child(format!("command {} → {}", command.name, command.launcher));
        }
        rows.push(row);
    }
    let body = List::new(rows).render_lossless_at(width);
    if body.is_empty() {
        heading
    } else {
        format!("{heading}\n{body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn describe_outline_measures_children() {
        let option = json!({
            "id": "linux-x64", "name": "Node.js 26.13.0",
            "platform": "linux", "platformName": "Linux", "arch": "x86_64",
            "form": "steps", "result": "owned", "portable": true,
            "features": [{"id": "docs", "name": "Documentation", "default": true,
                "required": false, "requires": []}],
            "steps": [{"id": "node", "title": "Download Node.js", "kind": "archive",
                "phase": "prepare", "optional": false, "when": [], "unless": [], "needs": []}],
            "launchersFrom": ["share/applications/*.desktop"],
            "launchers": [{"id": "node", "name": "Node.js", "executable": "bin/node",
                "args": [], "terminal": true, "default": false, "kind": "command",
                "menu": false, "source": "authored"}],
            "publicCommands": [{"name": "node", "launcher": "node"}],
            "defaultFeatures": [], "profiles": [], "parameters": []
        });
        let mut document: ApplicationDescribe = serde_json::from_value(json!({
            "protocol": "1", "package": {"group": "org.vibevm.node", "name": "node",
                "version": "26.13.0"},
            "application": {"id": "node", "portable": true}, "options": [option]
        }))
        .unwrap();
        let rendered = render_outline(&document, 160);
        let plain = console::strip_ansi_codes(&rendered);
        assert!(plain.contains("org.vibevm.node/node@26.13.0 · slot node · 1 installer option"));
        assert!(plain.contains(
            "  linux-x64  Node.js 26.13.0  linux/x86_64  · result owned · portable"
        ));
        let children = [
            "feature docs: Documentation [default]",
            "1. node  (archive)  Download Node.js",
            "launchers from share/applications/*.desktop",
            "launcher node (command) → bin/node",
            "command node → node",
        ];
        let mut previous = 0;
        for child in children {
            let position = plain
                .find(child)
                .unwrap_or_else(|| panic!("missing {child}: {plain}"));
            assert!(position > previous, "child order: {plain}");
            previous = position;
        }
        assert_eq!(plain.lines().count(), 7);
        for unwanted in ["decision", "done", "next:", "portable true"] {
            assert!(!plain.contains(unwanted), "unexpected {unwanted}: {plain}");
        }

        let first = &mut document.options[0];
        first.portable = false;
        first.app_root = Some("/opt/node".into());
        first.steps[0].phase = vibe_wire::generated::application_describe::Phase::Finish;
        first.steps[0].optional = true;
        first.launchers[0].menu = true;
        first.launchers[0].default = true;
        let mut other = first.launchers[0].clone();
        other.executable = "bin/other-node".into();
        other.kind = None;
        first.launchers.push(other);
        let mut second = first.clone();
        second.id = "a".into();
        second.name = "N".into();
        document.options.push(second);
        let rendered = render_outline(&document, 160);
        let plain = console::strip_ansi_codes(&rendered);
        assert!(plain.contains("2 installer options"));
        assert!(plain.contains("not portable"));
        assert!(plain.contains("external `/opt/node`"));
        assert!(plain.contains("(archive, finish, optional)"));
        assert!(plain.contains("launcher node (command, menu, default) → bin/node"));
        assert!(plain.contains("launcher node (override, menu, default) → bin/other-node"));
        let parents: Vec<_> = plain.lines().filter(|line| line.starts_with("  ")).collect();
        assert_eq!(parents.len(), 14);
        let first = parents
            .iter()
            .find(|line| line.contains("linux-x64"))
            .unwrap();
        let second = parents
            .iter()
            .find(|line| line.starts_with("  a "))
            .unwrap();
        assert_eq!(first.find("linux/x86_64"), second.find("linux/x86_64"));
        assert!(plain.find("linux-x64").unwrap() < plain.find("  a ").unwrap());

        let narrow = render_outline(&document, 40);
        let narrow = console::strip_ansi_codes(&narrow);
        let lines: Vec<_> = narrow.lines().collect();
        let starts: Vec<_> = ["linux-x64", "a"]
            .iter()
            .map(|id| lines.iter().position(|line| line.trim() == *id).unwrap())
            .collect();
        let snapshots: Vec<_> = starts
            .iter()
            .copied()
            .enumerate()
            .map(|(index, start)| {
                let end = starts.get(index + 1).copied().unwrap_or(lines.len());
                let block = &lines[start..end];
                let first_child = block
                    .iter()
                    .position(|line| line.contains("feature docs:"))
                    .unwrap();
                format!("parent {index} physical lines: {:?}", &block[..first_child])
            })
            .collect();
        let has_not_portable = |text: &str| {
            let tokens: Vec<_> = text.split_whitespace().collect();
            tokens.windows(2).any(|pair| pair == ["not", "portable"])
        };
        assert!(!has_not_portable("result external `/opt/node` · notportable"));
        for (index, start) in starts.iter().copied().enumerate() {
            let end = starts.get(index + 1).copied().unwrap_or(lines.len());
            let block = &lines[start..end];
            let first_child = block
                .iter()
                .position(|line| line.contains("feature docs:"))
                .expect("each parent must retain its first feature child");
            assert!(
                has_not_portable(&block[..first_child].join(" ")),
                "parent {index} must retain separate not and portable tokens; {snapshots:#?}"
            );
            let parent: String = block[..first_child]
                .iter()
                .flat_map(|line| line.chars().filter(|ch| !ch.is_whitespace()))
                .collect();
            let expected = if index == 0 {
                "linux-x64 Node.js 26.13.0 linux/x86_64 · result external `/opt/node` · not portable"
            } else {
                "a N linux/x86_64 · result external `/opt/node` · not portable"
            };
            let expected: String = expected.chars().filter(|ch| !ch.is_whitespace()).collect();
            assert_eq!(parent, expected, "parent {index} must retain every fact");
        }
        document.options.clear();
        let empty = render_outline(&document, 160);
        assert!(empty.contains("0 installer options"));
        assert_eq!(empty.lines().count(), 1);
    }
}
