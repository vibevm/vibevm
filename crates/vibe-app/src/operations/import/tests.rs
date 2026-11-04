//! Ordinary preparation failure must not poison the work inventory.
use super::*;
use crate::{AgentHooks, HostFacts, ImportCandidate, PackageCoordinate, ResolverScope};
use std::{fs, path::Path};
use vibe_core::manifest::Manifest;
#[test]
fn unavailable_lookup_before_import_leaves_no_unpublished_cohort_reservation() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("package");
    fs::create_dir_all(&source).unwrap();
    let text = format!(
        "[package]\ngroup='org.example'\nname='reservation'\nversion='1.0.0'\nkind='app'\n[application]\nid='reservation'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='{}'\nplatform_name='Host'\narch='{}'\nresult={{external='/tmp/reservation'}}\n[[installer_options.steps]]\nid='run'\ntitle='Run'\nkind='shell'\ncommand='true'\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    fs::write(source.join("vibe.toml"), &text).unwrap();
    let resolved = crate::from_manifest(
        Manifest::parse_str(&text).unwrap(),
        source,
        PackageCoordinate {
            group: "org.example".into(),
            name: "reservation".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap();
    let set=crate::parse_portable("fixture","protocol='vibe-apps/1'\nexported='2026-11-04T00:00:00Z'\nhome='user'\nplatform='linux'\narch='x86_64'\n[[application]]\nkey='org.example/reservation@1.0.0'\ninstaller='main'\nmark='manual'\n").unwrap();
    let mut prepared = crate::prepare_import(
        &set,
        &[ImportCandidate {
            row: &set.application[0],
            resolved: &resolved,
            option: &resolved.options[0],
        }],
        false,
        false,
        None,
        false,
    )
    .unwrap();
    let lookup = root.path().join("lookup");
    fs::create_dir(&lookup).unwrap();
    let settings = SettingsRoot::new(&root.path().join("settings"));
    let progress = vibe_core::progress::Progress::default();
    let ctx = OperationContext {
        settings: &settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused"),
        facts: HostFacts::default(),
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress: &progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: None,
        resolver_scope: ResolverScope::Registry(&lookup),
        source_reader: None,
        environment: None,
        search_path: None,
    };
    fs::remove_dir(&lookup).unwrap();
    assert!(import_cohort(&ctx, &mut prepared).is_err());
    let listing = crate::work(&settings, None).expect(
        "a failed preflight must not leave an unreadable reservation in the work inventory",
    );
    assert!(listing.entries.is_empty() && listing.imports.is_empty());
    assert!(
        crate::clean_work(&settings, None)
            .unwrap()
            .removed
            .is_empty()
    );
    assert!(!settings.index_path().exists());
}
