//! Derivation proofs: launchers from desktop entries, the hicolor icon
//! set, field-by-field overrides and the import rows.

use std::fs;
use std::path::Path;

use vibe_core::manifest::{IconDecl, LauncherDecl, LauncherKind, LauncherSource, Manifest};

use super::*;

fn png(size: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(size.to_be_bytes());
    bytes.extend(size.to_be_bytes());
    bytes
}

/// A payload with `bin/edit`, a hicolor theme and three entries: a shown
/// GUI editor, a hidden one and a `NoDisplay` terminal helper.
fn payload(root: &Path) {
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin/edit"), "#!/bin/sh\n").unwrap();
    let theme = root.join("share/icons/hicolor");
    for size in [48, 256] {
        let dir = theme.join(format!("{size}x{size}/apps"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("edit.png"), png(size)).unwrap();
    }
    fs::create_dir_all(theme.join("scalable/apps")).unwrap();
    fs::write(theme.join("scalable/apps/edit.svg"), "<svg/>").unwrap();
    fs::create_dir_all(theme.join("symbolic/apps")).unwrap();
    fs::write(theme.join("symbolic/apps/edit-symbolic.svg"), "<svg/>").unwrap();
    fs::create_dir_all(theme.join("bogus/apps")).unwrap();
    let apps = root.join("share/applications");
    fs::create_dir_all(&apps).unwrap();
    fs::write(
        apps.join("edit.desktop"),
        "[Desktop Entry]\nType=Application\nName=Edit\nName[de]=Bearbeiten\nGenericName=Text editor\n\
         Comment=Edit text\nExec=edit --title \"Cost: $5\" %F %i\nPath=/nowhere\nIcon=edit\n\
         Categories=Utility;TextEditor;Development;\nKeywords=text;\nMimeType=text/plain;\n\
         StartupWMClass=Edit\nTryExec=edit\nX-Custom=1\nActions=new;\n\n\
         [Desktop Action new]\nName=New Window\nExec=edit --new %u\n",
    )
    .unwrap();
    fs::write(
        apps.join("ghost.desktop"),
        "[Desktop Entry]\nType=Application\nName=Ghost\nExec=ghost\nHidden=true\n",
    )
    .unwrap();
    fs::write(
        apps.join("helper.desktop"),
        "[Desktop Entry]\nType=Application\nName=Helper\nExec=/opt/elsewhere/helper\nTerminal=true\nNoDisplay=true\n",
    )
    .unwrap();
    fs::write(
        apps.join("link.desktop"),
        "[Desktop Entry]\nType=Link\nName=Web\nURL=https://example.com\n",
    )
    .unwrap();
    fs::write(apps.join("broken.desktop"), "not a desktop entry\n").unwrap();
}

#[test]
fn shown_application_entries_become_launchers() {
    let dir = tempfile::tempdir().unwrap();
    payload(dir.path());
    let launchers = derive_launchers(dir.path(), &["share/applications/*.desktop".into()], None);
    let ids: Vec<&str> = launchers
        .iter()
        .map(|launcher| launcher.id.as_str())
        .collect();
    assert_eq!(ids, ["edit", "helper"]);
    let edit = &launchers[0];
    assert_eq!(edit.source, LauncherSource::DesktopEntry);
    assert_eq!(edit.kind, Some(LauncherKind::Gui));
    assert_eq!(edit.menu, Some(true));
    assert_eq!(edit.executable, "$app_bin/edit");
    assert_eq!(edit.args, ["--title", "Cost: $$5", "{files}"]);
    assert_eq!(edit.working_directory, None);
    assert_eq!(edit.categories, ["Utility", "Development"]);
    assert_eq!(edit.names["de"], "Bearbeiten");
    assert_eq!(edit.generic_name.as_deref(), Some("Text editor"));
    assert_eq!(edit.window_class.as_deref(), Some("Edit"));
    assert_eq!(edit.mime_types, ["text/plain"]);
    assert_eq!(edit.actions[0].args, ["--new", "{url}"]);
    let Some(IconDecl::Set(set)) = &edit.icon else {
        panic!("a theme icon is a set: {:?}", edit.icon);
    };
    assert_eq!(set.png.keys().collect::<Vec<_>>(), ["256", "48"]);
    assert_eq!(
        set.svg.as_deref(),
        Some("$app_root/share/icons/hicolor/scalable/apps/edit.svg")
    );
    assert_eq!(
        set.symbolic.as_deref(),
        Some("$app_root/share/icons/hicolor/symbolic/apps/edit-symbolic.svg")
    );
    let helper = &launchers[1];
    assert_eq!(helper.kind, Some(LauncherKind::Tui));
    assert_eq!(helper.menu, Some(false));
    assert_eq!(helper.executable, "$app_bin/helper");
}

#[test]
fn an_authored_row_overrides_field_by_field() {
    let dir = tempfile::tempdir().unwrap();
    payload(dir.path());
    let derived = derive_launchers(dir.path(), &["share/applications/*.desktop".into()], None);
    let override_row = LauncherDecl {
        id: "edit".into(),
        name: "Editor".into(),
        menu: Some(false),
        default: true,
        ..LauncherDecl::default()
    };
    let tool = LauncherDecl {
        id: "tool".into(),
        name: "Tool".into(),
        executable: "bin/tool".into(),
        kind: Some(LauncherKind::Command),
        ..LauncherDecl::default()
    };
    let merged = merge(&[override_row, tool], derived.clone()).unwrap();
    let ids: Vec<&str> = merged.iter().map(|launcher| launcher.id.as_str()).collect();
    assert_eq!(ids, ["edit", "tool", "helper"]);
    assert_eq!(merged[0].name, "Editor");
    assert_eq!(merged[0].menu, Some(false));
    assert!(merged[0].default);
    assert_eq!(merged[0].executable, "$app_bin/edit");
    assert_eq!(merged[0].source, LauncherSource::DesktopEntry);
    assert_eq!(merged[1].terminal, Some(true));
    assert_eq!(merged[1].menu, Some(false));
    let kind_change = LauncherDecl {
        id: "edit".into(),
        kind: Some(LauncherKind::Tui),
        ..LauncherDecl::default()
    };
    let merged = merge(&[kind_change], derived.clone()).unwrap();
    assert_eq!(merged[0].terminal, Some(true));
    let orphan = LauncherDecl {
        id: "missing".into(),
        menu: Some(true),
        ..LauncherDecl::default()
    };
    assert!(
        merge(&[orphan], derived)
            .unwrap_err()
            .contains("no desktop entry")
    );
}

#[test]
fn import_rows_round_trip_through_the_manifest_parser() {
    let dir = tempfile::tempdir().unwrap();
    payload(dir.path());
    let apps = dir.path().join("share/applications");
    let files = vec![apps.join("edit.desktop"), apps.join("ghost.desktop")];
    let rows = import_desktop(&files, Some(dir.path())).unwrap();
    assert!(rows.contains("ghost.desktop: skipped"), "{rows}");
    assert!(
        rows.contains("[[installer_options.launchers.actions]]"),
        "{rows}"
    );
    let text = format!(
        "[package]\nname = \"edit\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\
         [application]\nid = \"edit\"\n[[installer_options]]\nid = \"linux\"\nname = \"Linux\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n\
         [installer_options.installation]\ntype = \"zip\"\nsource = \"edit.zip\"\n\
         sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n{rows}"
    );
    let manifest = Manifest::parse_str(&text).unwrap();
    let launcher = &manifest.installer_options[0].launchers[0];
    assert_eq!(launcher.id, "edit");
    assert_eq!(launcher.kind, Some(LauncherKind::Gui));
    assert_eq!(launcher.terminal, None);
    assert!(import_desktop(&[apps.join("broken.desktop")], None).is_err());
}
