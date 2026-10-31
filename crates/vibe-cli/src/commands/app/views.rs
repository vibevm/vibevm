//! Text views of the `vibe app` listings: installed applications (hidden
//! installations in their own section, PROP-059 `#hidden-installations`),
//! installer options and launchers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

use serde_json::Value;

/// Every installation, exposed ones first and hidden ones in their own
/// section with what requires them; marks, holds and kept previous
/// versions are shown beside each.
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
    for entry in installations.iter().filter(|entry| !hidden(entry)) {
        println!("{}", list_line(entry));
    }
    let concealed: Vec<&Value> = installations.iter().filter(hidden).collect();
    if concealed.is_empty() {
        return;
    }
    println!("hidden installation(s):");
    for entry in concealed {
        let required_by: Vec<&str> = entry
            .get("required_by")
            .and_then(Value::as_array)
            .map(|keys| keys.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if required_by.is_empty() {
            println!("{}", list_line(entry));
        } else {
            println!(
                "{} required by {}",
                list_line(entry),
                required_by.join(", ")
            );
        }
    }
}

fn list_line(entry: &Value) -> String {
    let key = launcher_field(entry, "key");
    let slot = launcher_field(entry, "slot");
    let method = launcher_field(entry, "method");
    let status = launcher_field(entry, "status");
    let flag = |name: &str| entry.get(name).and_then(Value::as_bool) == Some(true);
    let mut marks = String::new();
    if flag("default") {
        marks.push_str(" [default]");
    }
    if launcher_field(entry, "mark") == "auto" {
        marks.push_str(" [auto]");
    }
    if flag("held") {
        marks.push_str(" [held]");
    }
    if let Some(next) = entry.get("superseded_by").and_then(Value::as_str) {
        marks.push_str(&format!(" [previous of {next}]"));
    }
    match entry.get("reason").and_then(Value::as_str) {
        Some(reason) => format!("  {slot}: {key} ({method}, {status}: {reason}){marks}"),
        None => format!("  {slot}: {key} ({method}, {status}){marks}"),
    }
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
        println!("Detected: {platform}/{arch}");
    }
    for platform in platforms {
        let platform_id = launcher_field(platform, "id");
        let platform_name = launcher_field(platform, "name");
        let detected = if platform.get("detected").and_then(Value::as_bool) == Some(true) {
            " [detected]"
        } else {
            ""
        };
        println!("  {platform_name} ({platform_id}){detected}");
        let Some(architectures) = platform.get("architectures").and_then(Value::as_array) else {
            continue;
        };
        for arch in architectures {
            let arch_id = launcher_field(arch, "id");
            let arch_name = launcher_field(arch, "name");
            let detected = if arch.get("detected").and_then(Value::as_bool) == Some(true) {
                " [detected]"
            } else {
                ""
            };
            println!("    {arch_name} ({arch_id}){detected}");
            let Some(installers) = arch.get("installers").and_then(Value::as_array) else {
                continue;
            };
            for installer in installers {
                let id = launcher_field(installer, "id");
                let name = launcher_field(installer, "name");
                println!("      {id}: {name}");
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
                            println!("        {field}: {value}");
                        }
                    }
                }
                for field in ["portable", "pre_install", "post_install", "launchers"] {
                    if let Some(value) = installer.get(field).filter(|value| !value.is_null()) {
                        println!("        {field}: {value}");
                    }
                }
            }
        }
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
            " [default]"
        } else {
            ""
        };
        let reason = launcher_field(launcher, "reason");
        if reason.is_empty() {
            println!("  {id}{marker}: {name} ({status})");
        } else {
            println!("  {id}{marker}: {name} ({status}: {reason})");
        }
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
        println!(
            "    kind: {}, terminal: {terminal}, menu: {menu}, source: {}",
            launcher_field(launcher, "kind"),
            launcher_field(launcher, "source")
        );
        for field in ["executable", "args", "working_directory", "icon"] {
            if let Some(value) = launcher.get(field) {
                println!("    {field}: {value}");
            }
        }
    }
}
