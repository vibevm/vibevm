//! The user layer over the launcher catalog (PROP-059 `#launcher-catalog`):
//! the collection, pin and override edits, the seeding rules, what a start
//! counts, and the per-launcher fields the catalog listing quotes from all
//! of it.
//!
//! These are integration tests rather than unit tests of `src/catalog/*`
//! for one mechanical reason: every edit takes the home's mutation lock,
//! and a child forked by a sibling test in the same process briefly
//! inherits every open lock file, which makes a second lock in one test
//! fail as `locked`. This binary forks nothing.

#![cfg(not(miri))]

use std::path::Path;

use vibe_app::{
    ApplicationMenu, AuthoredCommands, Index, Installation, LauncherCatalog, LauncherChange,
    Location, Method, PackageCoordinate, PinTarget, SettingsRoot, Storage, Usage, UserLaunchers,
    app_collection_add, app_collection_create, app_collection_delete, app_collection_order,
    app_collection_remove, app_collection_rename, app_pin, app_unpin, collections, drop_package,
    launcher_override, seed_package,
};
use vibe_core::manifest::{LauncherDecl, LauncherKind};

const PACKAGE: &str = "org.example/editor@1.0.0";
const EDIT: &str = "org.example/editor@1.0.0#edit";
const TOOL: &str = "org.example/editor@1.0.0#tool";

/// A scratch home whose index records `org.example/editor@1.0.0` in the
/// group `Editor Suite`, with `edit` opted into a menu and `tool` not. No
/// payload is placed: nothing here launches anything.
fn home(dir: &Path) -> SettingsRoot {
    let settings = SettingsRoot::new(dir);
    let package = PackageCoordinate {
        group: "org.example".into(),
        name: "editor".into(),
        version: "1.0.0".into(),
    };
    let launcher = |id: &str, kind, shown| LauncherDecl {
        id: id.into(),
        name: id.into(),
        executable: format!("$app_bin/{id}"),
        kind: Some(kind),
        menu: Some(shown),
        ..LauncherDecl::default()
    };
    let mut index = Index::default();
    index.installations.insert(
        package.key(),
        Installation {
            source_evidence: None,
            configuration: None,
            pending: None,
            secret_origins: Default::default(),
            slot: "editor".into(),
            package: package.clone(),
            option: "main".into(),
            platform: "linux".into(),
            arch: "x86_64".into(),
            method: Method::Zip,
            location: Location::Owned {
                library: "default".into(),
                directory: "apps/editor/org.example.editor@1.0.0".into(),
            },
            portable: true,
            launchers: vec![
                launcher("edit", LauncherKind::Gui, true),
                launcher("tool", LauncherKind::Command, false),
            ],
            public_commands: Vec::new(),
            path_dirs: Vec::new(),
            version_file: Vec::new(),
            template: None,
            migrations: Vec::new(),
            migrated: Vec::new(),
            extras: Vec::new(),
            commands: AuthoredCommands::default(),
            features: Vec::new(),
            parameters: Default::default(),
            license: None,
            icon: None,
            menu: ApplicationMenu {
                menu_group: Some("Editor Suite".into()),
                ..Default::default()
            },
            steps: Vec::new(),
            lifecycle: Default::default(),
        },
    );
    index.set_default("editor", Some(package.key()));
    index
        .save(
            &settings,
            &vibe_app::HomeEvent::new(vibe_app::EventKind::Installed),
        )
        .unwrap();
    settings
}

fn installation(settings: &SettingsRoot) -> Installation {
    Index::load(settings).unwrap().installations[PACKAGE].clone()
}

#[test]
fn collections_are_created_filled_ordered_renamed_and_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());

    let layer = app_collection_create(&settings, "Games", None).unwrap();
    assert_eq!(layer.collections[0].name, "Games");
    assert!(!layer.collections[0].seeded, "the user made it");
    assert_eq!(
        app_collection_create(&settings, "Games", None)
            .unwrap_err()
            .code(),
        "collection_exists"
    );
    assert_eq!(
        app_collection_add(&settings, "Absent", &[EDIT.into()])
            .unwrap_err()
            .code(),
        "collection_missing"
    );
    assert_eq!(
        app_collection_add(
            &settings,
            "Games",
            &["org.example/editor@1.0.0#ghost".into()]
        )
        .unwrap_err()
        .code(),
        "launcher_missing"
    );

    let layer =
        app_collection_add(&settings, "Games", &[EDIT.into(), TOOL.into(), EDIT.into()]).unwrap();
    assert_eq!(layer.collections[0].launchers, [EDIT, TOOL], "once each");
    let layer = app_collection_order(&settings, "Games", &[TOOL.into()]).unwrap();
    assert_eq!(layer.collections[0].launchers, [TOOL]);
    assert_eq!(
        layer.collections[0].removed,
        [EDIT],
        "the full order dropped it, so seeding may not reinstate it"
    );
    let layer = app_collection_add(&settings, "Games", &[EDIT.into()]).unwrap();
    assert!(
        layer.collections[0].removed.is_empty(),
        "adding it back clears the removal"
    );
    let layer = app_collection_remove(&settings, "Games", &[EDIT.into()]).unwrap();
    assert_eq!(layer.collections[0].removed, [EDIT]);

    let layer = app_collection_rename(&settings, "Games", "Classics").unwrap();
    assert_eq!(layer.collections[0].name, "Classics");
    assert_eq!(layer.collections[0].launchers, [TOOL], "it keeps its keys");
    assert_eq!(
        app_collection_rename(&settings, "Absent", "Other")
            .unwrap_err()
            .code(),
        "collection_missing"
    );
    assert_eq!(
        app_collection_rename(&settings, "Classics", "Classics")
            .unwrap_err()
            .code(),
        "collection_exists"
    );
    assert_eq!(collections(&settings).unwrap().collections.len(), 1);
    let layer = app_collection_delete(&settings, "Classics").unwrap();
    assert!(layer.collections.is_empty());
    assert_eq!(
        app_collection_delete(&settings, "Classics")
            .unwrap_err()
            .code(),
        "collection_missing"
    );
}

#[test]
fn pins_keep_their_order_and_an_override_row_appears_and_vanishes() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());

    app_pin(&settings, EDIT, PinTarget::Taskbar).unwrap();
    let layer = app_pin(&settings, TOOL, PinTarget::Taskbar).unwrap();
    assert_eq!(layer.pins.taskbar, [EDIT, TOOL]);
    let layer = app_pin(&settings, EDIT, PinTarget::Taskbar).unwrap();
    assert_eq!(
        layer.pins.taskbar,
        [TOOL, EDIT],
        "a repeated pin moves to the end"
    );
    let layer = app_pin(&settings, EDIT, PinTarget::Favorites).unwrap();
    assert_eq!(layer.pins.favorites, [EDIT]);
    let layer = app_unpin(&settings, EDIT, PinTarget::Taskbar).unwrap();
    assert_eq!(layer.pins.taskbar, [TOOL]);
    assert_eq!(layer.pins.favorites, [EDIT], "the other list is untouched");
    assert_eq!(
        app_pin(
            &settings,
            "org.example/absent@1.0.0#main",
            PinTarget::Taskbar
        )
        .unwrap_err()
        .code(),
        "launcher_missing"
    );

    let layer = launcher_override(&settings, EDIT, LauncherChange::Hide).unwrap();
    assert_eq!(layer.overrides[0].key, EDIT);
    assert!(layer.overrides[0].hidden);
    let layer = launcher_override(&settings, EDIT, LauncherChange::Name("Mine".into())).unwrap();
    assert_eq!(layer.overrides[0].name.as_deref(), Some("Mine"));
    let layer = launcher_override(&settings, EDIT, LauncherChange::Show).unwrap();
    assert!(!layer.overrides[0].hidden, "the name keeps the row alive");
    let layer = launcher_override(&settings, EDIT, LauncherChange::Reset).unwrap();
    assert!(layer.overrides.is_empty());
    let layer = launcher_override(&settings, EDIT, LauncherChange::Show).unwrap();
    assert!(
        layer.overrides.is_empty(),
        "a row that says nothing is not kept"
    );
}

#[test]
fn seeding_creates_the_group_once_and_never_re_adds_a_removed_key() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());
    let installation = installation(&settings);

    seed_package(&settings, &installation).unwrap();
    let layer = UserLaunchers::load(&settings).unwrap();
    let seeded = layer.collection("Editor Suite").unwrap();
    assert!(seeded.seeded);
    assert_eq!(seeded.launchers, [EDIT], "only the menu launcher");

    // Seeding again is a no-op, and a removal of the user's own stands.
    seed_package(&settings, &installation).unwrap();
    assert_eq!(
        UserLaunchers::load(&settings)
            .unwrap()
            .collection("Editor Suite")
            .unwrap()
            .launchers,
        [EDIT]
    );
    app_collection_remove(&settings, "Editor Suite", &[EDIT.into()]).unwrap();
    seed_package(&settings, &installation).unwrap();
    let layer = UserLaunchers::load(&settings).unwrap();
    let kept = layer.collection("Editor Suite").unwrap();
    assert!(kept.launchers.is_empty(), "{kept:?}");
    assert_eq!(kept.removed, [EDIT]);

    // A package without a group seeds nothing at all.
    let mut plain = installation.clone();
    plain.menu = ApplicationMenu::default();
    seed_package(&settings, &plain).unwrap();
    assert_eq!(UserLaunchers::load(&settings).unwrap().collections.len(), 1);
}

#[test]
fn an_uninstall_drops_the_keys_and_an_untouched_seeded_group_but_keeps_the_users_own() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());
    let installation = installation(&settings);

    seed_package(&settings, &installation).unwrap();
    app_pin(&settings, TOOL, PinTarget::Taskbar).unwrap();
    launcher_override(&settings, TOOL, LauncherChange::Hide).unwrap();
    Usage::record_start(&settings, EDIT).unwrap();

    drop_package(&settings, &installation).unwrap();
    let layer = UserLaunchers::load(&settings).unwrap();
    assert!(
        layer.collections.is_empty(),
        "the untouched seeded group went with its package: {layer:?}"
    );
    assert!(layer.pins.taskbar.is_empty() && layer.launchers.is_empty());
    assert!(Usage::load(&settings).unwrap().launchers.is_empty());

    // A collection the user edited is the user's and outlives the package.
    let settings = home(dir.path());
    seed_package(&settings, &installation).unwrap();
    app_collection_add(&settings, "Editor Suite", &[TOOL.into()]).unwrap();
    drop_package(&settings, &installation).unwrap();
    let layer = UserLaunchers::load(&settings).unwrap();
    let kept = layer.collection("Editor Suite").unwrap();
    assert!(kept.launchers.is_empty() && !kept.seeded);
}

#[test]
fn a_start_counts_once_and_the_listing_quotes_the_layer_and_the_counts() {
    let dir = tempfile::tempdir().unwrap();
    let settings = home(dir.path());
    let installation = installation(&settings);
    let storage = Storage::load(&settings).unwrap();

    seed_package(&settings, &installation).unwrap();
    app_pin(&settings, EDIT, PinTarget::Favorites).unwrap();
    launcher_override(&settings, EDIT, LauncherChange::Hide).unwrap();
    launcher_override(&settings, EDIT, LauncherChange::Name("Mine".into())).unwrap();
    assert_eq!(Usage::record_start(&settings, EDIT).unwrap().starts, 1);

    let catalog = LauncherCatalog::load(&settings).unwrap();
    assert_eq!(catalog.home(), dir.path());
    assert!(catalog.revision() > 0);
    let listing = vibe_app::launcher_listing(&installation, &storage, &catalog, None);
    assert_eq!(listing.protocol, "vibe-app-launchers/2");
    assert!(!listing.state.hidden && !listing.state.held);
    assert_eq!(listing.state.update_available, None);

    let edit = &listing.launchers[0];
    assert_eq!(edit.key, EDIT);
    assert_eq!(edit.name, "edit", "the package's own name stands");
    assert_eq!(edit.display_name, "Mine");
    assert_eq!(edit.collections, ["Editor Suite"]);
    assert_eq!(edit.usage.starts, 1);
    assert!(edit.usage.last_started.is_some());
    assert!(
        edit.hidden_by.is_some(),
        "the user hid it, the package did not"
    );
    assert_eq!(edit.pinned.len(), 1);

    let tool = &listing.launchers[1];
    assert_eq!(tool.key, TOOL);
    assert_eq!(tool.display_name, "tool");
    assert_eq!(tool.display_icon, None);
    assert_eq!(tool.hidden_by, None);
    assert_eq!(tool.usage.starts, 0);
    assert!(tool.collections.is_empty() && tool.pinned.is_empty());

    // The whole home in one answer, with the revision beside it.
    let index = Index::load(&settings).unwrap();
    let all = vibe_app::launcher_listing_all(&index, &storage, &catalog, None, false);
    assert_eq!(all.protocol, "vibe-app-launchers-all/2");
    assert_eq!(all.revision, catalog.revision());
    assert_eq!(all.home, dir.path().display().to_string());
    assert!(
        all.installations.is_empty(),
        "no payload is placed, so no installation is ready"
    );
}
