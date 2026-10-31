//! Public Linux fixture for start-menu launchers and desktop entries
//! (PROP-059 `#menu`, `#desktop-entries`): a payload shipping
//! `share/applications/*.desktop` and a hicolor icon theme derives its
//! launchers through `launchers_from`, an authored row overrides one field
//! by field, `app launchers --all --menu` answers for a start menu,
//! `app icon` picks the best existing file and `import-desktop` prints rows
//! the manifest parser reads back.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

const EDITOR: &str = "org.example/editor@1.0.0";

/// A PNG signature and `IHDR` chunk declaring a square of `size` pixels.
fn png(size: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(size.to_be_bytes());
    bytes.extend(size.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes
}

fn executable(path: &Path, marker: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\necho \"{marker} $*\"\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The upstream payload: two programs, three desktop entries (a shown
/// editor with an action, a `NoDisplay` terminal helper and a hidden one)
/// and a hicolor theme with 48 and 256 pixel rasters and a scalable SVG.
fn payload(root: &Path) {
    executable(&root.join("bin/edit"), "EDIT_RAN");
    executable(&root.join("bin/tool"), "TOOL_RAN");
    executable(&root.join("bin/helper"), "HELPER_RAN");
    let theme = root.join("share/icons/hicolor");
    for size in [48, 256] {
        let apps = theme.join(format!("{size}x{size}/apps"));
        fs::create_dir_all(&apps).unwrap();
        fs::write(apps.join("edit.png"), png(size)).unwrap();
    }
    fs::create_dir_all(theme.join("scalable/apps")).unwrap();
    fs::write(theme.join("scalable/apps/edit.svg"), "<svg/>").unwrap();
    let entries = root.join("share/applications");
    fs::create_dir_all(&entries).unwrap();
    fs::write(
        entries.join("edit.desktop"),
        "[Desktop Entry]\nType=Application\nName=Edit\nName[de]=Bearbeiten\nGenericName=Text editor\n\
         Comment=Edit text files\nExec=edit %F\nIcon=edit\nTerminal=false\n\
         Categories=Development;TextEditor;\nKeywords=text;notes;\nMimeType=text/plain;\n\
         StartupWMClass=Edit\nActions=new-window;\n\n\
         [Desktop Action new-window]\nName=New Window\nExec=edit --new-window\n",
    )
    .unwrap();
    fs::write(
        entries.join("helper.desktop"),
        "[Desktop Entry]\nType=Application\nName=Helper\nExec=helper\nTerminal=true\nNoDisplay=true\n",
    )
    .unwrap();
    fs::write(
        entries.join("ghost.desktop"),
        "[Desktop Entry]\nType=Application\nName=Ghost\nExec=ghost\nHidden=true\n",
    )
    .unwrap();
}

fn manifest(name: &str, application: &str, option: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
         [application]\nid = \"{name}\"\n{application}\n\
         [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\n\
         arch = \"{arch}\"\n{option}",
        arch = std::env::consts::ARCH,
    )
}

const COPY: &str = "\n[[installer_options.steps]]\nid = \"place\"\ntitle = \"Place the payload\"\nkind = \"copy\"\nfrom = \"payload\"\n";

/// `org.example/editor` deriving its launchers from the payload, and
/// `org.example/plain` with one authored launcher that is not in a menu.
fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let editor = apps.package("editor", "1.0.0", "editor", "");
    fs::write(
        editor.join("vibe.toml"),
        manifest(
            "editor",
            "menu_group = \"Editor Suite\"\ncategories = [\"Utility\"]\nkeywords = [\"text\"]\n",
            &format!(
                "launchers_from = [\"share/applications/*.desktop\"]\n{COPY}\n\
                 [[installer_options.launchers]]\nid = \"edit\"\nname = \"Editor (authored)\"\ndefault = true\n\n\
                 [[installer_options.launchers]]\nid = \"tool\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\n\n\
                 [[installer_options.public_commands]]\nname = \"edit\"\nlauncher = \"edit\"\n"
            ),
        ),
    )
    .unwrap();
    payload(&editor.join("payload"));
    let plain = apps.package("plain", "1.0.0", "plain", "");
    fs::write(
        plain.join("vibe.toml"),
        manifest(
            "plain",
            "",
            &format!(
                "{COPY}\n[[installer_options.launchers]]\nid = \"main\"\nname = \"Plain\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n"
            ),
        ),
    )
    .unwrap();
    executable(&plain.join("payload/bin/tool"), "PLAIN_RAN");
    apps
}

fn launcher<'a>(listing: &'a Value, id: &str) -> &'a Value {
    listing["launchers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no launcher `{id}` in {listing}"))
}

fn icon_file<'a>(icon_set: &'a Value, slot: &str, size: Option<u64>) -> &'a Value {
    icon_set["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["slot"] == slot && file["size"].as_u64() == size)
        .unwrap_or_else(|| panic!("no {slot} {size:?} in {icon_set}"))
}

#[test]
fn desktop_entries_derive_launchers_an_authored_row_overrides_and_a_menu_lists_them() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/editor", &[]));
    ok(&apps.app(&["install", "-g"], "org.example/plain", &[]));

    let listing = json(&apps.vibe(&["app", "launchers", "-g", "org.example/editor", "--json"]));
    let ids: Vec<&str> = listing["launchers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["edit", "tool", "helper"],
        "the hidden entry derives nothing"
    );
    assert_eq!(listing["menu_group"], "Editor Suite");
    assert_eq!(listing["default_launcher_id"], "edit");
    let edit = launcher(&listing, "edit");
    assert_eq!(edit["source"], "desktop-entry");
    assert_eq!(edit["name"], "Editor (authored)", "the authored field wins");
    assert_eq!(
        edit["generic_name"], "Text editor",
        "the derived fields stay"
    );
    assert_eq!(edit["kind"], "gui");
    assert_eq!(edit["terminal"], false);
    assert_eq!(edit["menu"], true);
    assert_eq!(edit["status"], "available");
    assert_eq!(edit["categories"], serde_json::json!(["Development"]));
    assert_eq!(edit["names"]["de"], "Bearbeiten");
    assert_eq!(edit["window_class"], "Edit");
    assert_eq!(edit["mime_types"], serde_json::json!(["text/plain"]));
    assert_eq!(edit["actions"][0]["id"], "new-window");
    assert_eq!(edit["args"]["template"], serde_json::json!(["{files}"]));
    assert!(
        edit["executable"]["resolved"]
            .as_str()
            .unwrap()
            .ends_with("/bin/edit")
    );
    let raster = icon_file(&edit["icon_set"], "png", Some(48));
    let path = raster["path"].as_str().unwrap();
    assert!(
        Path::new(path).is_absolute() && Path::new(path).is_file(),
        "{raster}"
    );
    assert!(path.ends_with("share/icons/hicolor/48x48/apps/edit.png"));
    assert!(icon_file(&edit["icon_set"], "svg", None)["path"].is_string());
    let tool = launcher(&listing, "tool");
    assert_eq!(tool["source"], "authored");
    assert_eq!(tool["kind"], "command");
    assert_eq!(tool["menu"], false);
    assert_eq!(
        tool["categories"],
        serde_json::json!(["Utility"]),
        "the application default"
    );
    let helper = launcher(&listing, "helper");
    assert_eq!(helper["kind"], "tui");
    assert_eq!(helper["menu"], false, "NoDisplay keeps it out of the menu");

    let record = &apps.index()["installations"][EDITOR];
    assert_eq!(record["launchers"][0]["source"], "desktop-entry");
    assert_eq!(record["launchers"][1].get("source"), None);
    assert_eq!(record["menuGroup"], "Editor Suite");

    // The public command names a derived launcher; `{files}` takes the
    // arguments.
    let ran = ok(&apps.vibe(&[
        "app",
        "run",
        "-g",
        "org.example/editor",
        "edit",
        "--",
        "a.txt",
        "b.txt",
    ]));
    assert_eq!(ran.trim(), "EDIT_RAN a.txt b.txt");
    let proxied = std::process::Command::new(apps.settings().join("opt/bin/edit"))
        .arg("c.txt")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&proxied.stdout).trim(),
        "EDIT_RAN c.txt"
    );

    let row = apps.listed(EDITOR);
    assert_eq!(row["menu_group"], "Editor Suite");
    assert_eq!(row["launchers"][0]["kind"], "gui");
    assert_eq!(row["launchers"][0]["menu"], true);
    assert_eq!(row["launchers"][0]["source"], "desktop-entry");
    assert!(icon_file(&row["launchers"][0]["icon_set"], "png", Some(256))["path"].is_string());
    assert_eq!(row["launchers"][0]["generic_name"], "Text editor");
    assert_eq!(row["launchers"][0]["actions"][0]["id"], "new-window");

    let described = json(&apps.app(&["app", "describe"], "org.example/editor", &["--json"]));
    assert_eq!(described["application"]["menuGroup"], "Editor Suite");
    let option = &described["options"][0];
    assert_eq!(
        option["launchersFrom"],
        serde_json::json!(["share/applications/*.desktop"])
    );
    assert_eq!(option["launchers"][0]["source"], "authored");
    assert_eq!(option["launchers"][0].get("kind"), None, "an override row");
    assert_eq!(option["launchers"][1]["kind"], "command");

    let everything = json(&apps.vibe(&["app", "launchers", "-g", "--all", "--json"]));
    assert_eq!(everything["menu_only"], false);
    assert_eq!(everything["installations"].as_array().unwrap().len(), 2);
    let menu = json(&apps.vibe(&["app", "launchers", "-g", "--all", "--menu", "--json"]));
    let installations = menu["installations"].as_array().unwrap();
    assert_eq!(
        installations.len(),
        1,
        "plain lists nothing in a menu: {menu}"
    );
    assert_eq!(installations[0]["key"], EDITOR);
    assert_eq!(installations[0]["default"], true);
    let menu_ids: Vec<&str> = installations[0]["launchers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert_eq!(menu_ids, ["edit"]);
    assert!(failed(&apps.vibe(&["app", "launchers", "-g", "--menu"])).contains("--all"));
}

#[test]
fn the_icon_choice_and_the_import_rows() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/editor", &[]));
    ok(&apps.app(&["install", "-g"], "org.example/plain", &[]));
    let icon = |extra: &[&str]| {
        let mut args = vec!["app", "icon", "-g", "org.example/editor", "--json"];
        args.extend_from_slice(extra);
        json(&apps.vibe(&args))
    };
    let small = icon(&["--launcher", "edit", "--size", "48"]);
    assert_eq!(small["slot"], "png");
    assert_eq!(small["size"], 48);
    assert!(
        small["path"]
            .as_str()
            .unwrap()
            .ends_with("48x48/apps/edit.png")
    );
    assert_eq!(icon(&["--launcher", "edit", "--size", "100"])["size"], 256);
    assert_eq!(icon(&["--launcher", "edit", "--size", "1024"])["size"], 256);
    let svg = icon(&["--launcher", "edit", "--size", "48", "--prefer", "svg"]);
    assert_eq!(svg["slot"], "svg");
    assert_eq!(icon(&["--size", "48"])["launcher"], Value::Null);
    assert_eq!(
        icon(&["--size", "48"])["size"],
        48,
        "the default launcher's set"
    );
    let text = ok(&apps.vibe(&["app", "icon", "-g", "org.example/editor", "--size", "48"]));
    assert!(text.trim().ends_with("48x48/apps/edit.png"), "{text}");
    let none = apps.vibe(&["app", "icon", "-g", "org.example/plain", "--size", "48"]);
    assert_eq!(none.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&none.stderr).contains("icon_unavailable"));

    let root = apps.registry().join("org.example/editor/v1.0.0/payload");
    let entries = root.join("share/applications");
    let rows = ok(&apps.vibe(&[
        "app",
        "import-desktop",
        entries.join("edit.desktop").to_str().unwrap(),
        entries.join("ghost.desktop").to_str().unwrap(),
        "--app-root",
        root.to_str().unwrap(),
    ]));
    assert!(rows.contains("[[installer_options.launchers]]"), "{rows}");
    assert!(rows.contains("ghost.desktop: skipped"), "{rows}");
    let text = manifest(
        "imported",
        "",
        &format!(
            "[installer_options.installation]\ntype = \"zip\"\nsource = \"edit.zip\"\n\
             sha256 = \"{}\"\n\n{rows}",
            "a".repeat(64)
        ),
    );
    let parsed = vibe_core::manifest::Manifest::parse_str(&text).unwrap();
    let imported = &parsed.installer_options[0].launchers[0];
    assert_eq!(imported.id, "edit");
    assert_eq!(imported.executable, "$app_bin/edit");
    assert_eq!(imported.args, ["{files}"]);
    assert_eq!(imported.kind, Some(vibe_core::manifest::LauncherKind::Gui));
    assert!(imported.shows_in_menu());
    assert_eq!(imported.actions[0].args, ["--new-window"]);
    let files: Vec<&str> = imported
        .icon
        .as_ref()
        .unwrap()
        .files()
        .into_iter()
        .map(|(_, path)| path)
        .collect();
    assert!(
        files.contains(&"$app_root/share/icons/hicolor/256x256/apps/edit.png"),
        "{files:?}"
    );
}

const BUILT: &str = "org.example/built@1.0.0";

/// `org.example/built`: an owned `source_build` whose recipe builds the
/// payload from a package-local source. Its application icon and its menu
/// launcher's icon set live only in the package tree, outside the payload.
fn built_fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let package = apps.registry().join("org.example/built/v1.0.0");
    fs::create_dir_all(package.join("icons/png")).unwrap();
    fs::write(package.join("icons/built.svg"), "<svg/>").unwrap();
    fs::write(package.join("icons/png/built-48.png"), png(48)).unwrap();
    let source = "echo \"BUILT_RAN $*\"\n";
    fs::write(package.join("tool.sh.in"), source).unwrap();
    fs::write(
        package.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n\
         { echo '#!/bin/sh'; cat \"$VIBE_SOURCES/tool/tool.sh.in\"; } > \"$VIBE_OUT/bin/tool\"\n\
         chmod 755 \"$VIBE_OUT/bin/tool\"\n",
    )
    .unwrap();
    let sha = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(source.as_bytes()))
    };
    fs::write(
        package.join("vibe.toml"),
        manifest(
            "built",
            "icon = \"icons/built.svg\"\n",
            &format!(
                "[installer_options.installation]\ntype = \"source_build\"\n\
                 [[installer_options.installation.sources]]\nname = \"tool\"\npath = \"tool.sh.in\"\nsha256 = \"{sha}\"\n\
                 [installer_options.installation.recipe]\ninterpreter = \"posix_sh\"\nscript = \"build.sh\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"Built\"\nexecutable = \"$app_bin/tool\"\n\
                 kind = \"gui\"\nmenu = true\ndefault = true\n\
                 icon = {{ svg = \"icons/built.svg\", png = {{ \"48\" = \"icons/png/built-48.png\" }} }}\n"
            ),
        ),
    )
    .unwrap();
    apps
}

/// A path the home keeps: absolute, existing and under its icon copies.
fn kept(apps: &AppRegistry, path: &str, tail: &str) {
    let copies = apps
        .settings()
        .join("applications/icons/org.example.built@1.0.0");
    let path = Path::new(path);
    assert!(path.is_absolute() && path.is_file(), "{}", path.display());
    assert_eq!(path, copies.join(tail));
}

#[test]
fn package_relative_icons_are_copied_into_the_home_survive_a_move_and_go_with_uninstall() {
    let apps = built_fixture();
    ok(&apps.app(&["install", "-g"], "org.example/built", &[]));
    let copies = apps
        .settings()
        .join("applications/icons/org.example.built@1.0.0");
    assert!(copies.join("icons/built.svg").is_file());
    assert!(copies.join("icons/png/built-48.png").is_file());
    let payload = apps
        .settings()
        .join("opt/apps/built/org.example.built@1.0.0");
    assert!(payload.join("bin/tool").is_file());
    assert!(
        !payload.join("icons").exists(),
        "the icons are not part of the payload"
    );

    let menu = json(&apps.vibe(&["app", "launchers", "-g", "--all", "--menu", "--json"]));
    let installation = &menu["installations"][0];
    assert_eq!(installation["key"], BUILT);
    let main = launcher(installation, "main");
    kept(
        &apps,
        icon_file(&main["icon_set"], "svg", None)["path"]
            .as_str()
            .unwrap(),
        "icons/built.svg",
    );
    kept(
        &apps,
        icon_file(&main["icon_set"], "png", Some(48))["path"]
            .as_str()
            .unwrap(),
        "icons/png/built-48.png",
    );
    kept(
        &apps,
        icon_file(&installation["icon_set"], "svg", None)["path"]
            .as_str()
            .unwrap(),
        "icons/built.svg",
    );
    let row = apps.listed(BUILT);
    kept(&apps, row["icon"].as_str().unwrap(), "icons/built.svg");

    let printed = ok(&apps.vibe(&["app", "icon", "-g", "org.example/built", "--size", "48"]));
    kept(&apps, printed.trim(), "icons/built.svg");
    let raster = ok(&apps.vibe(&[
        "app",
        "icon",
        "-g",
        "org.example/built",
        "--launcher",
        "main",
        "--size",
        "48",
    ]));
    kept(&apps, raster.trim(), "icons/png/built-48.png");

    // A storage move relocates the payload; the copies stay with the home.
    let usb = apps.root.path().join("usb");
    ok(&apps.vibe(&["storage", "add", usb.to_str().unwrap(), "--id", "usb"]));
    ok(&apps.vibe(&["storage", "move", BUILT, "--to", "usb"]));
    assert!(!payload.exists(), "the payload moved");
    assert_eq!(apps.listed(BUILT)["library"], "usb");
    let printed = ok(&apps.vibe(&["app", "icon", "-g", "org.example/built", "--size", "48"]));
    kept(&apps, printed.trim(), "icons/built.svg");

    ok(&apps.vibe(&["uninstall", "-g", "org.example/built"]));
    assert!(!copies.exists(), "uninstall removes the copies");
}
