//! Shared source-backed fixture, never a replacement Redbook definition.
#![allow(dead_code)]
use super::resolver::resolvo_engine::request::{
    ConfigurationInput, NormalizedRequest, consumer_codec as codec, consumer_wire as wire,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use vibe_core::{PackageRef, manifest::Manifest};

pub fn p(name: &str) -> PackageRef {
    format!("org.vibevm.world/{name}").parse().unwrap()
}

pub fn intent(branch: Option<&str>, additions: &[(&str, &[&str])]) -> NormalizedRequest {
    // Original caller/domain input, not inferred from a solved graph or lock.
    let mut request = codec::decode_request(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    )))
    .unwrap();
    request.edges[0].dependency = "org.vibevm.world/redbook".into();
    request.edges[0].package = wire::Package::Vibe(Box::new(wire::PackageVibe {
        reference: p("redbook").to_string(),
    }));
    request.edges[0].selections.clear();
    let edge = &mut request.edges[0];
    if let Some(branch) = branch {
        edge.selections.insert(
            "continuity".into(),
            wire::Selection {
                intent: wire::SelectionIntent::Exact,
                provenance: "manifest:consumer-a".into(),
                members: Some(vec![branch.into()]),
            },
        );
    }
    for (group, members) in additions {
        edge.selections.insert(
            (*group).into(),
            wire::Selection {
                intent: wire::SelectionIntent::Additive,
                provenance: "manifest:consumer-a".into(),
                members: Some(members.iter().map(|s| (*s).into()).collect()),
            },
        );
    }
    let mut intent = NormalizedRequest::legacy(&[p("redbook")], &Default::default());
    intent.configuration = Some(ConfigurationInput {
        request,
        definitions: vec![],
        defaults: vec![],
    });
    intent
}

/// Copy actual package-owned files into an isolated registry. The C1 reader
/// hashes these real bytes; dependencies are discovered from their manifests.
pub fn registry(destination: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vibevm/vibepacks");
    let mut queue = vec![p("redbook")];
    let mut seen = BTreeSet::new();
    while let Some(package) = queue.pop() {
        if !seen.insert(package.qualified_name()) {
            continue;
        }
        let relative = PathBuf::from(package.qualified_name()).join("v1.1.0");
        let from = source.join(&relative);
        let metadata = vibe_registry::CandidateMetadata::read_local_registry(&from).unwrap();
        queue.extend(metadata.manifest.requires.packages.iter().cloned());
        copy(&from, &destination.join(relative));
    }
    assert_eq!(seen.len(), 28);
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = to.join(entry.file_name());
        assert!(!entry.file_type().unwrap().is_symlink());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &path);
        } else {
            std::fs::copy(entry.path(), path).unwrap();
        }
    }
}

/// A technical OR whose rich alternative is excluded by the root's own
/// constraint. Capturing that alternative must not demand consumer input.
pub fn unused_alternative(destination: &Path) {
    for (name, extra) in [
        ("f1-plain", ""),
        (
            "f1-alternative",
            "[[requires_any]]\none_of=['org.vibevm.world/redbook','org.vibevm.world/f1-plain']\n[conflicts]\npackages=['org.vibevm.world/redbook']\n",
        ),
    ] {
        let dir = destination
            .join("org.vibevm.world")
            .join(name)
            .join("v1.1.0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vibe.toml"), format!(
            "[package]\nname='{name}'\ngroup='org.vibevm.world'\nkind='flow'\nversion='1.1.0'\n{extra}"
        )).unwrap();
    }
}

pub fn alternative_intent(destination: &Path) -> NormalizedRequest {
    use super::resolver::{
        features::configuration::ConfigurationDefinition,
        resolvo_engine::configuration::CandidateConfiguration,
    };
    use vibe_core::manifest::{feature_constraints::Constraints, feature_effects::*};
    let mut intent = intent(None, &[]);
    intent.roots = vec![p("f1-alternative")];
    let input = intent.configuration.as_mut().unwrap();
    input.request.edges[0].dependency = "org.vibevm.world/f1-plain".into();
    input.request.edges[0].package = wire::Package::Vibe(Box::new(wire::PackageVibe {
        reference: p("f1-plain").to_string(),
    }));
    let instance = codec::admit_request(&input.request).unwrap()[0]
        .instance
        .clone();
    let metadata = vibe_registry::CandidateMetadata::read_local_registry(
        &destination.join("org.vibevm.world/f1-plain/v1.1.0"),
    )
    .unwrap();
    let declarations =
        FeatureDeclarations::from_requires(&metadata.manifest.requires, &Default::default())
            .unwrap();
    let program = FeatureProgram::lower_legacy(&metadata.manifest.features, &declarations).unwrap();
    input.definitions.push(CandidateConfiguration {
        package: p("f1-plain"),
        version: semver::Version::new(1, 1, 0),
        identity: metadata.identity,
        definition: ConfigurationDefinition {
            instance,
            program,
            declarations,
            constraints: Constraints::default(),
        },
    });
    intent
}

pub fn expected_names(branch: &str, additions: &[&str]) -> BTreeSet<String> {
    let mut expected: BTreeSet<_> = [
        "addressable-specs",
        "campaign-plans",
        "conflict-protocol",
        "decision-records",
        "git-atomic-commits",
        "git-attribution-policy",
        "git-autonomy",
        "git-conventional-commits",
        "git-practices",
        "licensing",
        "managed-blocks",
        "operating-modes",
        "qualified-naming",
        "redbook",
        "secrets-hygiene",
        "source-mirrors",
        "spec-genres",
        "sync-from-code",
        "tool-design-lessons",
        "two-process-model",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    expected.insert(
        if branch == "mup" {
            "multi-user-planning"
        } else {
            "wal"
        }
        .into(),
    );
    expected.extend(additions.iter().map(|s| (*s).to_owned()));
    expected
}

pub fn project(root: &Path) -> Manifest {
    std::fs::create_dir_all(root).unwrap();
    let text = "[project]\nname='f1-fixture'\nversion='1.1.0'\n[requires.packages]\n'org.vibevm.world/redbook'='=1.1.0'\n";
    std::fs::write(root.join("vibe.toml"), text).unwrap();
    Manifest::parse_str(text).unwrap()
}
