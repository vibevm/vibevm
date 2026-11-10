//! The activation precedence, as a matrix
//! (PROP-075 `#VERSION-FILE-FORMATS`: flags > project requirement > version
//! file > slot default > off).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use vibe_core::manifest::VersionFileDecl;

use super::*;
use crate::index::{Installation, Location, Method, PackageCoordinate, SlotState};
use crate::versionfile::Declared;

/// An index with one `runtime` slot provided by `versions` of
/// `org.example/runtime`, defaulting to `default`.
fn home(versions: &[&str], default: Option<&str>, off: bool) -> Index {
    let mut index = Index::default();
    for version in versions {
        let package = PackageCoordinate {
            group: "org.example".to_owned(),
            name: "runtime".to_owned(),
            version: (*version).to_owned(),
        };
        index.installations.insert(
            package.key(),
            Installation {
                source_evidence: None,
                secret_origins: BTreeMap::new(),
                configuration: None,
                pending: None,
                slot: "runtime".to_owned(),
                package,
                option: "main".to_owned(),
                platform: "linux".to_owned(),
                arch: "x86_64".to_owned(),
                method: Method::TarGz,
                location: Location::Owned {
                    library: "default".to_owned(),
                    directory: "d".to_owned(),
                },
                portable: true,
                launchers: Vec::new(),
                public_commands: Vec::new(),
                path_dirs: vec!["bin".to_owned()],
                version_file: vec![VersionFileDecl::Plain {
                    names: vec![".nvmrc".to_owned()],
                }],
                template: None,
                migrations: Vec::new(),
                migrated: Vec::new(),
                commands: Default::default(),
                features: Vec::new(),
                parameters: BTreeMap::new(),
                license: None,
                icon: None,
                menu: Default::default(),
                steps: Vec::new(),
                lifecycle: Default::default(),
            },
        );
    }
    index.slots.insert(
        "runtime".to_owned(),
        SlotState {
            default: default.map(str::to_owned),
            off,
            path_env: Vec::new(),
        },
    );
    index
}

fn settings_with(dir: &Path, index: &Index) -> crate::SettingsRoot {
    let settings = crate::SettingsRoot::new(&dir.join("settings"));
    fs::create_dir_all(settings.applications_dir()).unwrap();
    index.declared_version_files().save(&settings).unwrap();
    settings
}

fn request<'a>(
    uses: &'a [String],
    directory: &'a Path,
    project: Option<&'a Path>,
    requirements: &'a [ProjectRequirement],
    consult_project: bool,
) -> ActivationRequest<'a> {
    ActivationRequest {
        uses,
        directory,
        project,
        requirements,
        consult_project,
        channels: None,
    }
}

/// A channel table a test states outright, standing in for the cached
/// version list a real resolution reads.
struct Channels(&'static [(&'static str, &'static str, &'static [&'static str])]);

impl super::ChannelMembership for Channels {
    fn members(&self, package: &str, channel: &str) -> Option<Vec<semver::Version>> {
        self.0
            .iter()
            .find(|(declared_package, declared_channel, _)| {
                *declared_package == package && *declared_channel == channel
            })
            .map(|(_, _, versions)| versions.iter().map(|v| v.parse().unwrap()).collect())
    }
}

fn requirement(spelling: &str) -> ProjectRequirement {
    ProjectRequirement {
        group: "org.example".to_owned(),
        name: "runtime".to_owned(),
        version: vibe_core::VersionSpec::parse(spelling).unwrap(),
        authored: spelling.to_owned(),
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_precedence_is_flag_then_requirement_then_file_then_default() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".nvmrc"), "2.0.0\n").unwrap();
    let index = home(
        &["1.0.0", "2.0.0", "3.0.0"],
        Some("org.example/runtime@3.0.0"),
        false,
    );
    let settings = settings_with(dir.path(), &index);
    let requirements = [requirement("=1.0.0")];
    let flag = ["runtime=3.0.0".to_owned()];

    // A flag wins over everything else.
    let with_flag = resolve(
        &settings,
        &index,
        &request(&flag, &project, Some(&project), &requirements, true),
    )
    .unwrap();
    let chosen = with_flag.of_slot("runtime").unwrap();
    assert_eq!(chosen.source, Source::Flag);
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@3.0.0"));

    // Without a flag, the project requirement decides before the file.
    let with_requirement = resolve(
        &settings,
        &index,
        &request(&[], &project, Some(&project), &requirements, true),
    )
    .unwrap();
    let chosen = with_requirement.of_slot("runtime").unwrap();
    assert_eq!(chosen.source, Source::ProjectRequirement);
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@1.0.0"));

    // With no requirement, the nearest version file decides.
    let with_file = resolve(
        &settings,
        &index,
        &request(&[], &project, Some(&project), &[], true),
    )
    .unwrap();
    let chosen = with_file.of_slot("runtime").unwrap();
    assert_eq!(chosen.source, Source::VersionFile);
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@2.0.0"));
    assert_eq!(
        chosen.file.as_deref(),
        Some(project.join(".nvmrc").as_path())
    );

    // With neither, the slot's global default answers.
    fs::remove_file(project.join(".nvmrc")).unwrap();
    crate::versionfile::clear_cache(&settings);
    let bare = resolve(
        &settings,
        &index,
        &request(&[], &project, Some(&project), &[], true),
    )
    .unwrap();
    assert!(bare.selections.is_empty(), "{bare:?}");
    let fallback = slot_default(&index, "runtime");
    assert_eq!(fallback.source, Source::SlotDefault);
    assert_eq!(fallback.key.as_deref(), Some("org.example/runtime@3.0.0"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
fn a_slot_that_is_off_stays_off_and_an_explicit_none_switches_one_off() {
    let index = home(&["1.0.0"], Some("org.example/runtime@1.0.0"), true);
    let off = slot_default(&index, "runtime");
    assert_eq!(off.source, Source::SlotOff);
    assert_eq!(off.state, State::Off);
    assert!(off.key.is_none());

    let dir = tempfile::tempdir().unwrap();
    let on = home(&["1.0.0"], Some("org.example/runtime@1.0.0"), false);
    let settings = settings_with(dir.path(), &on);
    let uses = ["runtime=none".to_owned()];
    let activation = resolve(
        &settings,
        &on,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap();
    let selection = activation.of_slot("runtime").unwrap();
    assert_eq!(selection.state, State::Off);
    assert!(selection.is_off());
    assert_eq!(selection.choice().as_deref(), Some("runtime=none"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_newest_matching_version_wins_and_a_missing_one_names_its_install_command() {
    let dir = tempfile::tempdir().unwrap();
    let index = home(&["22.1.0", "22.9.0", "24.0.0"], None, false);
    let settings = settings_with(dir.path(), &index);
    let uses = ["runtime=^22".to_owned()];
    let selection = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap();
    let chosen = selection.of_slot("runtime").unwrap();
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@22.9.0"));

    let uses = ["runtime=20".to_owned()];
    let missing = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap();
    let chosen = missing.of_slot("runtime").unwrap();
    assert_eq!(chosen.state, State::NotInstalled);
    assert_eq!(
        chosen.install.as_deref(),
        Some("vibe install -g org.example/runtime@20")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn latest_and_stable_resolve_over_the_installed_set_without_a_channel_table() {
    let dir = tempfile::tempdir().unwrap();
    let index = home(&["1.0.0", "2.0.0", "3.0.0-rc.1"], None, false);
    let settings = settings_with(dir.path(), &index);
    for (channel, expected) in [
        ("latest", Some("org.example/runtime@3.0.0-rc.1")),
        ("stable", Some("org.example/runtime@2.0.0")),
    ] {
        let uses = [format!("runtime={channel}")];
        let activation = resolve(
            &settings,
            &index,
            &request(&uses, dir.path(), None, &[], false),
        )
        .unwrap();
        let chosen = activation.of_slot("runtime").unwrap();
        assert_eq!(chosen.key.as_deref(), expected, "{channel}: {chosen:?}");
    }
    let uses = ["runtime=lts/jod".to_owned()];
    let activation = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap();
    assert_eq!(
        activation.of_slot("runtime").unwrap().state,
        State::UnresolvedChannel,
        "without a channel table a declared channel stays unresolved, never an error"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_declared_channel_resolves_through_the_membership_seam() {
    let dir = tempfile::tempdir().unwrap();
    let index = home(&["1.0.0", "2.0.0", "3.0.0"], None, false);
    let settings = settings_with(dir.path(), &index);
    let table = Channels(&[
        ("org.example/runtime", "lts", &["1.0.0", "2.0.0"]),
        ("org.example/runtime", "lts/jod", &["2.0.0"]),
        ("org.example/runtime", "nightly", &[]),
    ]);
    for (channel, expected, state) in [
        ("lts", Some("org.example/runtime@2.0.0"), State::Selected),
        (
            "lts/jod",
            Some("org.example/runtime@2.0.0"),
            State::Selected,
        ),
        // Declared and empty, and declared-but-not-installed, are both
        // `not_installed` with the exact install command — never an error,
        // because a prompt hook must not fail a `cd`.
        ("nightly", None, State::NotInstalled),
        // Undeclared stays `unresolved_channel`, which is a different
        // fact and a different message.
        ("canary", None, State::UnresolvedChannel),
    ] {
        let uses = [format!("runtime={channel}")];
        let mut outer = request(&uses, dir.path(), None, &[], false);
        outer.channels = Some(&table);
        let activation = resolve(&settings, &index, &outer).unwrap();
        let chosen = activation.of_slot("runtime").unwrap();
        assert_eq!(chosen.key.as_deref(), expected, "{channel}: {chosen:?}");
        assert_eq!(chosen.state, state, "{channel}: {chosen:?}");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn a_version_file_naming_a_channel_resolves_through_the_same_seam() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".nvmrc"), "lts/jod\n").unwrap();
    let index = home(&["1.0.0", "2.0.0"], Some("1.0.0"), false);
    let settings = settings_with(dir.path(), &index);
    let table = Channels(&[("org.example/runtime", "lts/jod", &["2.0.0"])]);
    let mut outer = request(&[], &project, Some(&project), &[], true);
    outer.channels = Some(&table);
    let activation = resolve(&settings, &index, &outer).unwrap();
    let chosen = activation.of_slot("runtime").unwrap();
    assert_eq!(chosen.source, Source::VersionFile);
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@2.0.0"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn an_installation_key_must_provide_the_slot_it_is_named_for() {
    let dir = tempfile::tempdir().unwrap();
    let index = home(&["1.0.0"], None, false);
    let settings = settings_with(dir.path(), &index);
    let uses = ["runtime=org.example/runtime@1.0.0".to_owned()];
    let activation = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap();
    assert_eq!(
        activation.of_slot("runtime").unwrap().key.as_deref(),
        Some("org.example/runtime@1.0.0")
    );

    let uses = ["editor=org.example/runtime@1.0.0".to_owned()];
    let error = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap_err();
    assert_eq!(error.code(), "not_provider", "{error}");

    let uses = ["runtime=org.example/runtime@9.9.9".to_owned()];
    let error = resolve(
        &settings,
        &index,
        &request(&uses, dir.path(), None, &[], false),
    )
    .unwrap_err();
    assert_eq!(error.code(), "not_installed", "{error}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn a_tool_versions_line_with_extra_versions_is_reported_and_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".tool-versions"), "runtime 2.0.0 1.0.0\n").unwrap();
    let mut index = home(&["1.0.0", "2.0.0"], None, false);
    for record in index.installations.values_mut() {
        record.version_file = vec![VersionFileDecl::tool_versions("runtime")];
    }
    let settings = settings_with(dir.path(), &index);
    let activation = resolve(
        &settings,
        &index,
        &request(&[], &project, Some(&project), &[], true),
    )
    .unwrap();
    let chosen = activation.of_slot("runtime").unwrap();
    assert_eq!(chosen.key.as_deref(), Some("org.example/runtime@2.0.0"));
    assert_eq!(chosen.unused, ["1.0.0"]);
    assert_eq!(activation.notices.len(), 1, "{activation:?}");
    assert!(activation.notices[0].contains("1.0.0"), "{activation:?}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_declared_sidecar_survives_an_index_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let settings = crate::SettingsRoot::new(dir.path());
    let index = home(&["1.0.0"], Some("org.example/runtime@1.0.0"), false);
    index
        .save(&settings, &crate::HomeEvent::new(crate::EventKind::Updated))
        .unwrap();
    let read = Index::load(&settings).unwrap();
    assert_eq!(read, index, "the declared rows come back with the index");
    // The index's own wire form stays at the protocol the home needs for
    // its payload directories, not a newer one.
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(settings.index_path()).unwrap()).unwrap();
    assert_eq!(document["protocol"], "vibe-user-applications/6");
    assert!(
        document["installations"]["org.example/runtime@1.0.0"]
            .get("versionFile")
            .is_none(),
        "the declared rows are not in the index document"
    );
    assert!(
        Declared::load(&settings)
            .installations
            .contains_key("org.example/runtime@1.0.0")
    );
}
