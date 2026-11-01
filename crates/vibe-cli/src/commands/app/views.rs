//! Text views of the `vibe app` listings: installed applications (hidden
//! installations in their own section, PROP-059 `#hidden-installations`),
//! installer options and launchers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

use serde_json::Value;

use crate::output::language::{Cell, Glyph, List, Meta, Role, Row, paint, rule};

/// Every installation, exposed ones first and hidden ones under their own
/// rule with what requires them; marks, holds and kept previous versions
/// are state words at the end of each row (PROP-067 `#list`, `#meta`).
pub(super) fn print_list(listing: &Value, quiet: bool) {
    let installations = listing
        .get("installations")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    println!("{} installed application(s)", installations.len());
    if quiet {
        return;
    }
    let hidden = |entry: &&Value| entry.get("hidden").and_then(Value::as_bool) == Some(true);
    let shown: Vec<&Value> = installations
        .iter()
        .filter(|entry| !hidden(entry))
        .collect();
    let concealed: Vec<&Value> = installations.iter().filter(hidden).collect();
    let mut blocks = Vec::new();
    if !shown.is_empty() {
        blocks.push(
            List::new(shown.into_iter().map(list_row).collect())
                .meta_column()
                .render(),
        );
    }
    if !concealed.is_empty() {
        blocks.push(rule("hidden installations", None));
        blocks.push(
            List::new(concealed.into_iter().map(list_row).collect())
                .meta_column()
                .render(),
        );
    }
    if !blocks.is_empty() {
        println!("\n{}", blocks.join("\n\n"));
    }
}

/// One installation: its slot, its key, how it was installed and its
/// status, then what requires it and its state words.
fn list_row(entry: &Value) -> Row {
    let flag = |name: &str| entry.get(name).and_then(Value::as_bool) == Some(true);
    let status = match entry.get("reason").and_then(Value::as_str) {
        Some(reason) => format!(
            "({}, {}: {reason})",
            launcher_field(entry, "method"),
            launcher_field(entry, "status")
        ),
        None => format!(
            "({}, {})",
            launcher_field(entry, "method"),
            launcher_field(entry, "status")
        ),
    };
    let mut meta = Vec::new();
    let required_by: Vec<&str> = entry
        .get("required_by")
        .and_then(Value::as_array)
        .map(|keys| keys.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if !required_by.is_empty() {
        meta.push(Meta::Note(format!(
            "required by {}",
            required_by.join(", ")
        )));
    }
    if let Some(next) = entry.get("superseded_by").and_then(Value::as_str) {
        meta.push(Meta::Note(format!("previous of {next}")));
    }
    if flag("default") {
        meta.push(Meta::State("default".into()));
    }
    if launcher_field(entry, "mark") == "auto" {
        meta.push(Meta::State("auto".into()));
    }
    if flag("held") {
        meta.push(Meta::State("held".into()));
    }
    Row::new(vec![
        Cell::new(launcher_field(entry, "slot"), Role::Accent),
        Cell::text(launcher_field(entry, "key")),
        Cell::new(status, Role::Dim),
    ])
    .meta(meta)
}

pub(super) fn launcher_field<'a>(launcher: &'a Value, field: &str) -> &'a str {
    launcher.get(field).and_then(Value::as_str).unwrap_or("")
}

pub(super) fn print_installers(listing: &Value, package: &str, quiet: bool) {
    let platforms = listing
        .get("platforms")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let installer_count: usize = platforms
        .iter()
        .flat_map(|platform| {
            platform
                .get("architectures")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|arch| {
            arch.get("installers")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        })
        .sum();
    println!("{package}: {installer_count} installer(s)");
    if quiet {
        return;
    }
    if let (Some(platform), Some(arch)) = (
        listing
            .pointer("/detected/platform")
            .and_then(Value::as_str),
        listing.pointer("/detected/arch").and_then(Value::as_str),
    ) {
        println!(
            "{}",
            paint(Role::Dim, &format!("detected {platform}/{arch}"))
        );
    }
    for platform in platforms {
        let platform_id = launcher_field(platform, "id");
        let platform_name = launcher_field(platform, "name");
        let detected = detected_word(platform);
        println!("\n  {platform_name} ({platform_id}){detected}");
        let Some(architectures) = platform.get("architectures").and_then(Value::as_array) else {
            continue;
        };
        for arch in architectures {
            let arch_id = launcher_field(arch, "id");
            let arch_name = launcher_field(arch, "name");
            let detected = detected_word(arch);
            println!(
                "    {} {arch_name} ({arch_id}){detected}",
                Glyph::Child.text()
            );
            let Some(installers) = arch.get("installers").and_then(Value::as_array) else {
                continue;
            };
            for installer in installers {
                let id = launcher_field(installer, "id");
                let name = launcher_field(installer, "name");
                println!(
                    "      {} {}  {name}",
                    Glyph::Child.text(),
                    paint(Role::Accent, id)
                );
                if let Some(installation) = installer.get("installation") {
                    for field in [
                        "type",
                        "source",
                        "sha256",
                        "payload_root",
                        "app_root",
                        "install",
                        "force_install",
                        "update",
                        "uninstall",
                        "force_uninstall",
                        "arguments",
                        "force_arguments",
                        "interpreter",
                        "sources",
                        "recipe",
                    ] {
                        if let Some(value) =
                            installation.get(field).filter(|value| !value.is_null())
                        {
                            println!(
                                "        {}  {value}",
                                paint(Role::Dim, &format!("{field:<15}"))
                            );
                        }
                    }
                }
                for field in ["portable", "pre_install", "post_install", "launchers"] {
                    if let Some(value) = installer.get(field).filter(|value| !value.is_null()) {
                        println!(
                            "        {}  {value}",
                            paint(Role::Dim, &format!("{field:<15}"))
                        );
                    }
                }
            }
        }
    }
}

/// The `[detected]` state word of a platform or an architecture row.
fn detected_word(entry: &Value) -> String {
    if entry.get("detected").and_then(Value::as_bool) == Some(true) {
        format!("  {}", paint(Role::Dim, "[detected]"))
    } else {
        String::new()
    }
}

pub(super) fn print_listing(listing: &Value, quiet: bool) {
    let group = listing.pointer("/package/group").and_then(Value::as_str);
    let name = listing.pointer("/package/name").and_then(Value::as_str);
    let version = listing.pointer("/package/version").and_then(Value::as_str);
    let package = match (group, name, version) {
        (Some(group), Some(name), Some(version)) => format!("{group}/{name}@{version}"),
        _ => "application".to_owned(),
    };
    let default = listing.get("default_launcher_id").and_then(Value::as_str);
    let Some(launchers) = listing.get("launchers").and_then(Value::as_array) else {
        return;
    };
    if quiet {
        println!("{package}: {} launcher(s)", launchers.len());
        return;
    }
    println!("{package}: {} launcher(s)", launchers.len());
    for launcher in launchers {
        let id = launcher_field(launcher, "id");
        let name = launcher_field(launcher, "name");
        let status = launcher_field(launcher, "status");
        let marker = if default == Some(id) {
            format!("  {}", paint(Role::Dim, "[default]"))
        } else {
            String::new()
        };
        let reason = launcher_field(launcher, "reason");
        let state = if reason.is_empty() {
            status.to_owned()
        } else {
            format!("{status}: {reason}")
        };
        println!(
            "\n  {}  {name}{}{marker}",
            paint(Role::Accent, id),
            paint(Role::Dim, &format!(" · {state}"))
        );
        if let Some(description) = launcher.get("description").and_then(Value::as_str) {
            println!("    {description}");
        }
        let terminal = launcher
            .get("terminal")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let menu = launcher
            .get("menu")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let field = |name: &str, value: &str| {
            println!("    {}  {value}", paint(Role::Dim, &format!("{name:<17}")));
        };
        field("kind", launcher_field(launcher, "kind"));
        field("terminal", &terminal.to_string());
        field("menu", &menu.to_string());
        field("source", launcher_field(launcher, "source"));
        for name in ["executable", "args", "working_directory", "icon"] {
            if let Some(value) = launcher.get(name) {
                field(name, &value.to_string());
            }
        }
    }
}
