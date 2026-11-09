//! Launcher kinds, menu metadata, icon sets and desktop-entry fields
//! (`PROP-059#launch-points`, `#menu`, `#desktop-entries`).

use super::Manifest;
use crate::manifest::{IconDecl, IconSlot, LauncherKind, icon_findings};

const HEAD: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.0.0"
[application]
id = "demo"
"#;

const OPTION: &str = r#"
[[installer_options]]
id = "linux-zip"
name = "Archive"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
"#;

const INSTALLATION: &str = r#"[installer_options.installation]
type = "zip"
source = "releases/demo.zip"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
"#;

/// A manifest whose one launcher carries `launcher` (TOML lines after
/// `id`, `name` and `executable`); `application` extends `[application]`
/// and `option` the option table.
fn manifest(application: &str, option: &str, launcher: &str) -> String {
    format!(
        "{HEAD}{application}{OPTION}{option}{INSTALLATION}[[installer_options.launchers]]\nid = \"main\"\nname = \"Demo\"\nexecutable = \"bin/demo\"\n{launcher}"
    )
}

fn refused(text: &str) -> String {
    Manifest::parse_str(text).unwrap_err().to_string()
}

fn first_launcher(text: &str) -> crate::manifest::LauncherDecl {
    Manifest::parse_str(text).unwrap().installer_options[0].launchers[0].clone()
}

#[test]
fn kind_is_required_and_terminal_follows_it() {
    assert!(refused(&manifest("", "", "")).contains("kind"));
    for (kind, terminal) in [
        (LauncherKind::Gui, false),
        (LauncherKind::Tui, true),
        (LauncherKind::Command, true),
    ] {
        let launcher = first_launcher(&manifest(
            "",
            "",
            &format!("kind = \"{}\"\n", kind.as_str()),
        ));
        assert_eq!(launcher.kind, Some(kind));
        assert_eq!(launcher.is_terminal(), terminal);
        assert!(!launcher.shows_in_menu());
    }
    let agreeing = manifest("", "", "kind = \"tui\"\nterminal = true\n");
    assert!(Manifest::parse_str(&agreeing).is_ok());
    let error = refused(&manifest("", "", "kind = \"gui\"\nterminal = true\n"));
    assert!(error.contains("disagrees with kind"), "{error}");
    assert!(refused(&manifest("", "", "kind = \"daemon\"\n")).contains("unknown variant"));
}

#[test]
fn menu_metadata_parses_and_round_trips() {
    let text = manifest(
        "menu_group = \"Demo Suite\"\ncategories = [\"Development\", \"Utility\"]\nkeywords = [\"demo\"]\nicon = { svg = \"share/demo.svg\", png = { \"48\" = \"share/48.png\" }, maskable = true }\n",
        "",
        r#"kind = "gui"
menu = true
args = ["--open", "{files}"]
categories = ["Graphics"]
keywords = ["draw", "paint"]
generic_name = "Image editor"
window_class = "demo-main"
mime_types = ["image/png", "image/svg+xml"]
names = { de = "Demo (de)", "pt-BR" = "Demo (pt)", "sr@latin" = "Demo (sr)" }
descriptions = { de = "Bilder bearbeiten" }
icon = { svg = "$app_root/share/demo.svg", png = { "16" = "share/16.png", "256" = "share/256.png" }, ico = "demo.ico", icns = "demo.icns", symbolic = "share/demo-symbolic.svg" }
[[installer_options.launchers.actions]]
id = "new-window"
name = "New Window"
args = ["--new-window", "{url}"]
icon = "share/new.svg"
"#,
    );
    let parsed = Manifest::parse_str(&text).unwrap();
    let application = parsed.application.as_ref().unwrap();
    assert_eq!(application.menu_group.as_deref(), Some("Demo Suite"));
    assert_eq!(application.categories, ["Development", "Utility"]);
    assert!(application.icon.as_ref().unwrap().maskable());
    let launcher = &parsed.installer_options[0].launchers[0];
    assert!(launcher.shows_in_menu());
    assert_eq!(launcher.names["pt-BR"], "Demo (pt)");
    assert_eq!(launcher.actions[0].id, "new-window");
    let slots: Vec<IconSlot> = launcher
        .icon
        .as_ref()
        .unwrap()
        .files()
        .into_iter()
        .map(|(slot, _)| slot)
        .collect();
    assert_eq!(
        slots,
        [
            IconSlot::Svg,
            IconSlot::Png(16),
            IconSlot::Png(256),
            IconSlot::Ico,
            IconSlot::Icns,
            IconSlot::Symbolic
        ]
    );
    let again = Manifest::parse_str(&toml::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(again, parsed);
}

#[test]
fn categories_words_tags_and_mime_types_are_checked() {
    for (launcher, needle) in [
        ("categories = [\"TextEditor\"]\n", "is not one of"),
        ("categories = [\"Utility\", \"Utility\"]\n", "duplicated"),
        ("keywords = [\"\"]\n", "keywords entries"),
        ("keywords = [\"a;b\"]\n", "keywords entries"),
        ("generic_name = \" \"\n", "generic_name"),
        ("window_class = \"\"\n", "window_class"),
        ("mime_types = [\"text\"]\n", "TYPE/SUBTYPE"),
        ("names = { \"x\" = \"Demo\" }\n", "language tag"),
        ("names = { \"de@\" = \"Demo\" }\n", "language tag"),
        ("descriptions = { de = \"\" }\n", "nonempty text"),
    ] {
        let error = refused(&manifest("", "", &format!("kind = \"gui\"\n{launcher}")));
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
    let error = refused(&manifest(
        "categories = [\"Games\"]\n",
        "",
        "kind = \"gui\"\n",
    ));
    assert!(error.contains("[application] category `Games`"), "{error}");
    let error = refused(&manifest("menu_group = \"\"\n", "", "kind = \"gui\"\n"));
    assert!(error.contains("menu_group"), "{error}");
}

#[test]
fn argument_placeholders_are_spelled_exactly() {
    for good in [
        "{file}",
        "--open={file}",
        "{files}",
        "{url}",
        "{urls}",
        "${app_root}",
        "{}",
        "a{b c}",
    ] {
        let text = manifest("", "", &format!("kind = \"gui\"\nargs = [\"{good}\"]\n"));
        assert!(Manifest::parse_str(&text).is_ok(), "{good}");
    }
    for (bad, needle) in [
        ("{path}", "unknown placeholder"),
        ("{File}", "unknown placeholder"),
        ("--all={files}", "whole argument"),
    ] {
        let error = refused(&manifest(
            "",
            "",
            &format!("kind = \"gui\"\nargs = [\"{bad}\"]\n"),
        ));
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
    let action = "kind = \"gui\"\n[[installer_options.launchers.actions]]\nid = \"a\"\nname = \"A\"\nargs = [\"{nope}\"]\n";
    assert!(refused(&manifest("", "", action)).contains("unknown placeholder"));
}

#[test]
fn icons_have_their_slot_extension_and_decimal_sizes() {
    let short = first_launcher(&manifest(
        "",
        "",
        "kind = \"gui\"\nicon = \"share/demo.png\"\n",
    ));
    assert_eq!(short.icon, Some(IconDecl::Path("share/demo.png".into())));
    for (icon, needle) in [
        ("icon = \"share/demo.ico\"", "must end in .png or .svg"),
        (
            "icon = { png = { \"048\" = \"a.png\" } }",
            "decimal pixel size",
        ),
        ("icon = { png = { big = \"a.png\" } }", "decimal pixel size"),
        (
            "icon = { png = { \"48\" = \"a.svg\" } }",
            "must end in .png",
        ),
        ("icon = { svg = \"a.png\" }", "must end in .svg"),
        ("icon = { ico = \"a.png\" }", "must end in .ico"),
        ("icon = { maskable = true }", "names no file"),
        (
            "icon = { svg = \"a.svg\", webp = \"a.webp\" }",
            "unknown field",
        ),
        ("icon = { svg = \"../a.svg\" }", "icon"),
    ] {
        let error = refused(&manifest("", "", &format!("kind = \"gui\"\n{icon}\n")));
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
    let error = refused(&manifest(
        "icon = { svg = \"/abs.svg\" }\n",
        "",
        "kind = \"gui\"\n",
    ));
    assert!(error.contains("[application].icon"), "{error}");
    let template = manifest("icon = \"$app_root/a.svg\"\n", "", "kind = \"gui\"\n");
    assert!(refused(&template).contains("not a template"));
}

#[test]
fn launchers_from_admits_override_rows_and_derived_commands() {
    let from = "launchers_from = [\"share/applications/*.desktop\"]\n";
    let override_row = manifest("", from, "menu = false\n");
    let launcher = first_launcher(&override_row);
    assert!(!launcher.is_complete());
    assert_eq!(launcher.menu, Some(false));
    assert!(refused(&manifest("", "", "menu = false\n")).contains("must declare"));
    let derived_command = format!(
        "{}[[installer_options.public_commands]]\nname = \"edit\"\nlauncher = \"org-edit\"\n",
        manifest("", from, "kind = \"gui\"\n")
    );
    assert!(Manifest::parse_str(&derived_command).is_ok());
    let unknown = derived_command.replace(from, "");
    assert!(refused(&unknown).contains("unknown launcher"));
    for (pattern, needle) in [
        ("../share/*.desktop", "must not traverse"),
        (
            "/usr/share/applications/*.desktop",
            "relative to the application root",
        ),
        ("share/a**b/*.desktop", "relative to the application root"),
    ] {
        let text = manifest(
            "",
            &format!("launchers_from = [\"{pattern}\"]\n"),
            "kind = \"gui\"\n",
        );
        let error = refused(&text);
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes
}

#[test]
fn icon_files_are_checked_against_the_package_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("share")).unwrap();
    std::fs::write(root.join("share/48.png"), png(48, 48)).unwrap();
    std::fs::write(root.join("share/wide.png"), png(64, 32)).unwrap();
    std::fs::write(root.join("share/256.png"), png(128, 128)).unwrap();
    std::fs::write(root.join("share/big.png"), png(512, 512)).unwrap();
    std::fs::write(root.join("share/app.svg"), "<svg/>").unwrap();
    let check = |application: &str, launcher: &str| {
        let parsed = Manifest::parse_str(&manifest(
            application,
            "",
            &format!("kind = \"gui\"\n{launcher}"),
        ))
        .unwrap();
        icon_findings(root, parsed.application.as_ref(), &parsed.installer_options)
            .into_iter()
            .map(|finding| finding.message)
            .collect::<Vec<_>>()
    };
    assert!(check("", "icon = { png = { \"48\" = \"share/48.png\" } }").is_empty());
    assert!(check("", "icon = \"share/missing.svg\"")[0].contains("does not exist"));
    assert!(check("", "icon = \"share/wide.png\"")[0].contains("not square"));
    assert!(
        check("", "icon = { png = { \"256\" = \"share/256.png\" } }")[0]
            .contains("header says 128")
    );
    // A payload file is checked when the launcher is listed, not here.
    assert!(check("", "icon = { ico = \"$app_root/vim92/vim.ico\" }").is_empty());
    let menu = check(
        "",
        "menu = true\nicon = { png = { \"48\" = \"share/48.png\" } }",
    );
    assert!(menu[0].contains("menu = true"), "{menu:?}");
    assert!(check("", "menu = true\nicon = \"share/big.png\"").is_empty());
    assert!(
        check(
            "",
            "menu = true\nicon = { svg = \"$app_root/share/app.svg\" }"
        )
        .is_empty()
    );
    assert!(check("icon = \"share/app.svg\"\n", "menu = true").is_empty());
    assert!(check("icon = \"share/none.svg\"\n", "")[0].contains("[application] icon"));
}

/// A launcher `executable` may name a command resolved on PATH at launch
/// instead of a payload file (`PROP-065#launcher-command`): the name must
/// be bare, and the template and containment rules do not apply to it.
#[test]
fn a_command_executable_must_name_a_bare_command() {
    use crate::manifest::{is_launcher_command_name, launcher_command};

    let with = |executable: &str| {
        format!(
            "{HEAD}{OPTION}{INSTALLATION}[[installer_options.launchers]]\n\
             id = \"main\"\nname = \"Demo\"\nkind = \"command\"\nexecutable = \"{executable}\"\n"
        )
    };

    let parsed = Manifest::parse_str(&with("command:node")).unwrap();
    let launcher = &parsed.installer_options[0].launchers[0];
    assert_eq!(launcher.executable, "command:node");
    assert_eq!(launcher_command(&launcher.executable), Some("node"));
    // The spelling survives a round trip through the manifest.
    let again = Manifest::parse_str(&toml::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(
        again.installer_options[0].launchers[0].executable,
        "command:node"
    );

    // A payload path is not a command, and neither is a template or a
    // name that could not be looked up on PATH.
    for bad in [
        "command:bin/node",
        "command:../node",
        "command:$app_bin",
        "command:",
        "command:a b",
        "command:node:1",
        "command:node\\exe",
    ] {
        let message = refused(&with(bad));
        assert!(
            message.contains("must name a bare command after `command:`"),
            "{bad}: {message}"
        );
        assert!(message.contains("[[launchers]] `main`"), "{bad}: {message}");
    }

    // An ordinary executable keeps the template rules: an absolute literal
    // path and a `..` escape are still refused there.
    assert!(launcher_command("$app_bin/node").is_none());
    assert!(refused(&with("/usr/bin/node")).contains("absolute"));
    assert!(is_launcher_command_name("node") && !is_launcher_command_name("bin/node"));
}
