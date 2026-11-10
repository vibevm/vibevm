use std::collections::BTreeMap;

use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl};
use vibe_wire::generated::apps_manifest as wire;

use super::*;
use crate::index::{Index, Lifecycle, Location, Method, PackageCoordinate};

const KEY: &str = "org.example/demo@1.0.0";

fn option(id: &str, platform: &str, arch: &str) -> InstallerOptionDecl {
    let mut option: InstallerOptionDecl = toml::from_str(&format!(
        "id = \"{id}\"\nname = \"{id}\"\nplatform = \"{platform}\"\nplatform_name = \"{platform}\"\narch = \"{arch}\""
    ))
    .unwrap();
    option.features = vec![
        FeatureDecl {
            id: "docs".into(),
            name: "Docs".into(),
            default: true,
            ..Default::default()
        },
        FeatureDecl {
            id: "extras".into(),
            name: "Extras".into(),
            ..Default::default()
        },
    ];
    option
}

fn host() -> String {
    crate::detected_platform().unwrap_or("linux").to_owned()
}

fn row(installer: &str, features: &[&str]) -> wire::Application {
    wire::Application {
        key: KEY.into(),
        installer: installer.into(),
        features: features.iter().map(|id| (*id).to_owned()).collect(),
        parameters: BTreeMap::new(),
        license: None,
        mark: wire::ApplicationMark::Manual,
        held: false,
        hidden: false,
    }
}

fn installed(option: &str, features: &[&str]) -> Index {
    let mut index = Index::default();
    index.installations.insert(
        KEY.into(),
        Installation {
            source_evidence: None,
            secret_origins: Default::default(),
            configuration: None,
            pending: None,
            slot: "demo".into(),
            package: PackageCoordinate {
                group: "org.example".into(),
                name: "demo".into(),
                version: "1.0.0".into(),
            },
            option: option.into(),
            platform: host(),
            arch: detected_arch().into(),
            method: Method::Steps,
            location: Location::Owned {
                library: "default".into(),
                directory: "demo".into(),
            },
            portable: true,
            launchers: vec![],
            public_commands: vec![],
            path_dirs: Vec::new(),
            version_file: Vec::new(),
            template: None,
            migrations: Vec::new(),
            migrated: Vec::new(),
            commands: Default::default(),
            features: features.iter().map(|id| (*id).to_owned()).collect(),
            parameters: BTreeMap::new(),
            license: None,
            icon: None,
            menu: Default::default(),
            steps: vec![],
            lifecycle: Lifecycle::default(),
        },
    );
    index
}

#[test]
fn the_named_installer_is_taken_when_this_host_offers_it() {
    let options = [
        option("main", &host(), detected_arch()),
        option("alt", &host(), detected_arch()),
    ];
    let ImportAction::Install(install) =
        import_action(&Index::default(), &row("alt", &["extras"]), &options, false)
    else {
        panic!("a fresh home installs");
    };
    assert_eq!(install.choice.installer.as_deref(), Some("alt"));
    assert_eq!(install.option.as_deref(), Some("alt"));
    assert_eq!(install.enable, ["extras"]);
    assert_eq!(
        install.disable,
        ["docs"],
        "a default the row left out is deselected"
    );
}

#[test]
fn an_installer_this_host_lacks_falls_back_to_the_detected_option() {
    let options = [
        option("main", &host(), detected_arch()),
        option("msi", "elsewhere", "other"),
    ];
    let ImportAction::Install(install) =
        import_action(&Index::default(), &row("msi", &[]), &options, false)
    else {
        panic!("a fresh home installs");
    };
    assert_eq!(install.choice, InstallChoice::default());
    assert_eq!(install.option.as_deref(), Some("main"));
}

#[test]
fn an_installed_key_is_skipped_or_refused_by_its_choice() {
    let options = [
        option("main", &host(), detected_arch()),
        option("alt", &host(), detected_arch()),
    ];
    let index = installed("main", &["docs"]);
    assert!(matches!(
        import_action(&index, &row("main", &["docs"]), &options, false),
        ImportAction::Skip(_)
    ));
    let ImportAction::Refuse(error) =
        import_action(&index, &row("alt", &["docs"]), &options, false)
    else {
        panic!("a differing choice is refused");
    };
    assert_eq!(error.code(), "choice_differs");
    assert!(error.to_string().contains("vibe app modify"), "{error}");
    assert!(matches!(
        import_action(&index, &row("alt", &["docs"]), &options, true),
        ImportAction::Skip(reason) if reason.contains("kept")
    ));
}

#[test]
fn the_toml_spelling_reads_back_to_the_same_set() {
    let mut first = row("main", &["docs"]);
    first.parameters.insert("prefix".into(), "/opt/demo".into());
    first.license = Some(wire::ApplicationLicense::Accepted);
    first.held = true;
    let set = wire::AppsManifest {
        protocol: APPS_PROTOCOL.into(),
        exported: "2026-11-01T12:00:00Z".into(),
        home: wire::AppsManifestHome::User,
        platform: "linux".into(),
        arch: "x86_64".into(),
        application: vec![first],
        collection: vec![wire::Collection {
            name: "Writing".into(),
            launchers: vec![format!("{KEY}#demo")],
        }],
        pin: vec![wire::Pin {
            target: wire::PinTarget::Taskbar,
            launchers: vec![format!("{KEY}#demo")],
        }],
        launcher: vec![wire::Launcher {
            key: format!("{KEY}#demo"),
            name: Some("Demo".into()),
            hidden: false,
            icon: None,
        }],
    };
    let text = portable_toml(&set).unwrap();
    assert!(text.starts_with("# vibe-apps/1"), "{text}");
    assert!(
        text.contains("held = true") && !text.contains("hidden"),
        "{text}"
    );
    assert_eq!(parse_portable("set.toml", &text).unwrap(), set);
    let json = serde_json::to_string(&set).unwrap();
    assert_eq!(parse_portable("set.json", &json).unwrap(), set);
    let error = parse_portable("set.toml", "protocol = 1").unwrap_err();
    assert_eq!(error.code(), "apps_file_invalid");
}
