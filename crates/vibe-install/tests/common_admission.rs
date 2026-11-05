//! Unsupported common package meaning is refused at its actual consumer boundary.
#![cfg(test)]
use vibe_core::{PackageRef, manifest::Manifest};
use vibe_install::{
    Error,
    configuration_admission::{package_manifest, workspace_inputs},
};
use vibe_workspace::Workspace;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn workspace_own_definition_is_not_its_dependency_input() {
    let root = tempfile::tempdir().unwrap();
    let text = "[package]\ngroup='org.demo'\nname='producer'\nkind='flow'\nversion='1.0.0'\n[configuration]\n";
    std::fs::write(root.path().join("vibe.toml"), text).unwrap();
    let manifest = Manifest::parse_str(text).unwrap();
    let workspace = Workspace::discover(root.path()).unwrap();
    workspace_inputs(&workspace, root.path(), &manifest).unwrap();
    assert!(matches!(
        package_manifest(
            &PackageRef::parse("org.demo/producer@=1.0.0").unwrap(),
            &manifest
        ),
        Err(Error::ConfigurationUnsupported { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(root.path().join("vibe.toml")).unwrap(),
        text
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn candidate_definition_precedes_child_intent_and_diagnostics_are_value_free() {
    let mut manifest=Manifest::parse_str("[package]\ngroup='org.demo'\nname='producer'\nkind='flow'\nversion='1.0.0'\n[configuration]\n[requires.packages.'org.demo/child']\nversion='^1'\nconfiguration={policy='require-selection',parameters={value='PRIVATE_VALUE_CANARY'}}\n").unwrap();
    let package = PackageRef::parse("org.demo/producer@=1.0.0").unwrap();
    let error = package_manifest(&package, &manifest).unwrap_err();
    assert!(
        matches!(&error,Error::ConfigurationUnsupported{package} if package=="org.demo/producer")
    );
    assert!(!format!("{error:?} {error}").contains("PRIVATE_VALUE_CANARY"));
    manifest.configuration = None;
    let error = package_manifest(&package, &manifest).unwrap_err();
    assert!(
        matches!(&error,Error::ConfigurationInputUnsupported{consumer,dependency} if consumer=="package org.demo/producer" && dependency=="org.demo/child")
    );
    assert!(!format!("{error:?} {error}").contains("PRIVATE_VALUE_CANARY"));
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn package_admission_does_not_reject_selected_installer_configuration() {
    let manifest=Manifest::parse_str("[package]\ngroup='org.demo'\nname='app'\nkind='app'\nversion='1.0.0'\n[application]\nid='app'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\nresult='owned'\n[[installer_options.steps]]\nid='place'\ntitle='Place'\nkind='copy'\nfrom='payload'\n[installer_options.configuration]\n").unwrap();
    package_manifest(
        &PackageRef::parse("org.demo/app@=1.0.0").unwrap(),
        &manifest,
    )
    .unwrap();
}
