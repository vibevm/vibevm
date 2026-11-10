//! The decidable half of the ecosystem adapter — which installation a
//! verb runs in, which adapters a request selects, how the `list` verb's
//! answer file is read, what the work directory and its record look like,
//! and the arithmetic of the extras listing — none of which runs a child
//! (PROP-075 `#ECOSYSTEM-ADAPTER`, `#SLOT-EXTRAS`).

use std::path::Path;

use vibe_core::manifest::{EcosystemDecl, Manifest};

use super::*;
use crate::index::{
    AuthoredCommands, Installation, Location, Method, PackageCoordinate, SlotState,
};
use crate::settings::SettingsRoot;

/// A package whose `[application]` declares `adapters`, resolved the way
/// the library resolves one.
fn package(adapters: &str) -> crate::resolve::ResolvedPackage {
    let text = format!(
        "[package]\ngroup = \"org.example\"\nname = \"runtime\"\nkind = \"app\"\n\
         version = \"2.0.0\"\n\n[application]\nid = \"runtime\"\n{adapters}"
    );
    let manifest = Manifest::parse_str(&text).unwrap();
    manifest.validate().unwrap();
    crate::from_manifest(
        manifest,
        Path::new("/tmp/runtime").to_path_buf(),
        PackageCoordinate {
            group: "org.example".into(),
            name: "runtime".into(),
            version: "2.0.0".into(),
        },
    )
    .unwrap()
}

/// `install` plus whatever `extra` adds.
fn adapter(id: &str, extra: &str) -> String {
    format!(
        "\n[[application.ecosystem]]\nid = \"{id}\"\nname = \"{id} packages\"\n{extra}\n\
         [application.ecosystem.install]\nid = \"install\"\ntitle = \"Install\"\n\
         kind = \"recipe\"\nscript = \"eco/install.sh\"\n"
    )
}

fn record(version: &str, features: &[&str]) -> Installation {
    Installation {
        source_evidence: None,
        secret_origins: Default::default(),
        configuration: None,
        pending: None,
        slot: "runtime".into(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: "runtime".into(),
            version: version.into(),
        },
        option: "linux".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        method: Method::Steps,
        location: Location::External {
            app_root: Path::new("/opt/runtime").to_path_buf(),
        },
        portable: false,
        launchers: Vec::new(),
        public_commands: Vec::new(),
        path_dirs: Vec::new(),
        version_file: Vec::new(),
        template: None,
        migrations: Vec::new(),
        migrated: Vec::new(),
        extras: Vec::new(),
        commands: AuthoredCommands::default(),
        features: features.iter().map(|id| (*id).to_owned()).collect(),
        parameters: Default::default(),
        license: None,
        icon: None,
        menu: Default::default(),
        steps: Vec::new(),
        lifecycle: crate::index::Lifecycle::default(),
    }
}

/// Two installed versions of one slot, the newer one its default.
fn home(features: &[&str]) -> Index {
    let mut index = Index::default();
    for version in ["1.0.0", "2.0.0"] {
        let row = record(version, features);
        index.installations.insert(row.package.key(), row);
    }
    index.slots.insert(
        "runtime".into(),
        SlotState {
            default: Some("org.example/runtime@2.0.0".into()),
            ..SlotState::default()
        },
    );
    index
}

fn request<'a>(ecosystem: Option<&'a str>, verb: EcosystemVerb) -> EcosystemRequest<'a> {
    EcosystemRequest {
        slot: "runtime",
        version: None,
        ecosystem,
        verb,
        items: &[],
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn a_verb_runs_in_the_named_version_or_in_the_slots_default() {
    let index = home(&[]);
    assert_eq!(
        target(&index, "runtime", None).unwrap(),
        ("org.example/runtime@2.0.0".to_owned(), "2.0.0".to_owned())
    );
    assert_eq!(
        target(&index, "runtime", Some("1.0.0")).unwrap(),
        ("org.example/runtime@1.0.0".to_owned(), "1.0.0".to_owned())
    );
    assert_eq!(
        target(&index, "runtime", Some("org.example/runtime@1.0.0")).unwrap(),
        ("org.example/runtime@1.0.0".to_owned(), "1.0.0".to_owned())
    );
    assert_eq!(
        target(&index, "other", None).unwrap_err().code(),
        "unknown_slot"
    );
    // A slot that is off, or has no default, has no version to work in.
    let mut off = home(&[]);
    off.slots.get_mut("runtime").unwrap().off = true;
    assert_eq!(
        target(&off, "runtime", None).unwrap_err().code(),
        "not_installed"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn a_request_selects_one_adapter_or_every_one_and_refuses_what_is_not_declared() {
    let several = package(&format!("{}{}", adapter("npm", ""), adapter("pip", "")));
    let ids: Vec<&str> = selected(&several, &request(None, EcosystemVerb::Install))
        .unwrap()
        .iter()
        .map(|row| row.id.as_str())
        .collect();
    assert_eq!(ids, ["npm", "pip"]);
    assert_eq!(
        selected(&several, &request(Some("npm"), EcosystemVerb::Install))
            .unwrap()
            .len(),
        1
    );
    let error = selected(&several, &request(Some("gems"), EcosystemVerb::Install)).unwrap_err();
    assert_eq!(error.code(), "unknown_ecosystem");
    assert!(error.to_string().contains("npm, pip"), "{error}");
    // A package with no adapter at all: the mechanism is optional, and
    // asking for it says so by name.
    let none = package("");
    assert_eq!(
        selected(&none, &request(None, EcosystemVerb::Install))
            .unwrap_err()
            .code(),
        "no_ecosystem"
    );
    // Only `install` is required, so an explicitly named adapter that
    // declares no `self_upgrade` is a refusal naming what it does declare.
    let error = selected(&several, &request(Some("npm"), EcosystemVerb::SelfUpgrade)).unwrap_err();
    assert_eq!(error.code(), "ecosystem_verb_undeclared");
    assert!(error.to_string().contains("install"), "{error}");
    // With no `--ecosystem` the same shape is not an error: the runner
    // reports those adapters `skipped`.
    assert_eq!(
        selected(&several, &request(None, EcosystemVerb::SelfUpgrade))
            .unwrap()
            .len(),
        2
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn the_list_verbs_answer_is_a_file_read_line_by_line_and_an_absent_one_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("items");
    // An absent file is an empty list and never an error, which is what
    // makes a declared `list` verb safe to run on a fresh version.
    assert!(crate::migrate::observed(&file).is_empty());
    std::fs::write(&file, "").unwrap();
    assert!(crate::migrate::observed(&file).is_empty());
    // Lines are trimmed and the empty ones dropped; nothing else is
    // parsed, so a notice-free item with spaces inside it survives.
    std::fs::write(
        &file,
        "  typescript  \n\n@scope/pkg@^1.2\n\nlocal pkg\n   \n",
    )
    .unwrap();
    assert_eq!(
        crate::migrate::observed(&file),
        ["typescript", "@scope/pkg@^1.2", "local pkg"]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn the_work_directory_is_kept_per_slot_version_and_run_and_carries_the_record() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    assert!(kept_ecosystem_work(&settings).is_empty());
    let first = record::open(&settings, "runtime", "2.0.0").unwrap();
    let second = record::open(&settings, "runtime", "2.0.0").unwrap();
    assert_ne!(
        first, second,
        "a run never replaces the evidence of another"
    );
    assert!(
        first.starts_with(settings.app_ecosystem_dir().join("runtime").join("2.0.0")),
        "{first:?}"
    );
    let kept = kept_ecosystem_work(&settings);
    assert_eq!(kept.len(), 2);
    assert_eq!(slot_ecosystem_work(&settings, "runtime"), kept);
    assert!(slot_ecosystem_work(&settings, "other").is_empty());
    // No record yet: a kept directory with none is unknown history, never
    // an error.
    assert!(ecosystem_summary(&first).is_none());
    let mut document = record::empty(
        &request(None, EcosystemVerb::List),
        "org.example/runtime@2.0.0",
        &first,
    );
    document.adapters.push(wire::Adapter {
        id: "npm".into(),
        name: "npm packages".into(),
        outcome: wire::Outcome::Failed,
        items: Vec::new(),
        observed: Vec::new(),
        step: Some("list".into()),
        log: None,
        reason: Some("exit status 1".into()),
        retry: None,
        started: None,
        finished: None,
    });
    record::save(&first, &document).unwrap();
    assert_eq!(record::read(&first).unwrap(), document);
    let summary = ecosystem_summary(&first).unwrap();
    assert_eq!(summary.key, "ecosystem:runtime:2.0.0:list");
    assert_eq!(summary.failed.as_deref(), Some("list"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_failure_reports_the_exact_retry_command_and_never_rolls_anything_back() {
    let items = ["typescript".to_owned(), "pnpm@9".to_owned()];
    let remove = EcosystemRequest {
        slot: "runtime",
        version: None,
        ecosystem: None,
        verb: EcosystemVerb::Remove,
        items: &items,
    };
    assert_eq!(
        remove.retry("npm"),
        "vibe app extras remove runtime --ecosystem npm typescript pnpm@9"
    );
    assert_eq!(
        request(None, EcosystemVerb::SelfUpgrade).retry("npm"),
        "vibe app extras self-upgrade runtime --ecosystem npm"
    );
    // The retry of a failed install is the apply command, because what
    // failed is "put my extras in this version".
    assert_eq!(
        request(None, EcosystemVerb::Install).retry("npm"),
        "vibe app extras apply runtime --ecosystem npm"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_listing_reads_the_enabled_state_off_the_recorded_feature_set() {
    let declared: Vec<EcosystemDecl> = package(&adapter(
        "npm",
        "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"corepack\"\n\
         item = \"corepack@latest\"\ndefault = true\nreason = \"Shims\"\n\
         \n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"typescript\"\n\
         item = \"typescript\"\nname = \"TypeScript\"\n",
    ))
    .application
    .ecosystem
    .clone();
    // A recommendation is in the list exactly when its synthesised
    // feature is in the set the installation recorded.
    let row = super::listing::describe(&declared[0], &["corepack".to_owned()]);
    assert_eq!(row.id, "npm");
    assert_eq!(row.verbs, [extras_wire_verb()]);
    assert_eq!(row.recommendations.len(), 2);
    assert!(row.recommendations[0].enabled && row.recommendations[0].default);
    assert_eq!(row.recommendations[1].name, "TypeScript");
    assert!(!row.recommendations[1].enabled);
    assert_eq!(row.items, ["corepack@latest"]);
    // Nothing chosen: nothing would be installed, and the adapter is then
    // not run at all.
    let row = super::listing::describe(&declared[0], &[]);
    assert!(row.items.is_empty());
    // Both chosen: declaration order, de-duplicated by exact string.
    let row = super::listing::describe(
        &declared[0],
        &["typescript".to_owned(), "corepack".to_owned()],
    );
    assert_eq!(row.items, ["corepack@latest", "typescript"]);
}

/// The `install` verb's wire spelling, which every adapter declares.
fn extras_wire_verb() -> vibe_wire::generated::application_extras::Verb {
    vibe_wire::generated::application_extras::Verb::Install
}
