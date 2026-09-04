//! Health-operand validation against the exact projected after tree.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::collections::BTreeMap;

use crate::contract::Healthcheck;
use crate::model::EntryKind;

use super::model::{Applicability, HealthBlocker, PreparedHealth, TestDisposition, TestPresence};

pub fn validate_projected_final(
    contract: &crate::contract::Contract,
    prepared: &PreparedHealth,
    entries: &[crate::rewrite::ProjectedEntry],
) -> Vec<HealthBlocker> {
    let by_path = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let discovery = super::discovery::DiscoveryTree::projected(entries);
    let mut blockers = Vec::new();
    for row in &contract.healthcheck {
        let (id, root, when) = health_common(row);
        if root != "."
            && !by_path
                .get(root)
                .is_some_and(|entry| entry.kind == EntryKind::Directory)
        {
            block(
                &mut blockers,
                id,
                "health-projected-root-missing",
                format!("health root `{root}` does not survive the projected tree"),
            );
            continue;
        }
        if let Some(when) = when {
            let path = rooted(root, &when.path_exists);
            let after = by_path.contains_key(path.as_str());
            let before = prepared
                .checks
                .iter()
                .find(|check| check.id == id)
                .is_some_and(|check| check.applicability == Applicability::Applicable);
            if before != after {
                block(
                    &mut blockers,
                    id,
                    "health-projected-applicability-changed",
                    format!("health applicability operand `{path}` changes across phases"),
                );
            }
        }
        match row {
            Healthcheck::Cargo { .. } => {
                require_projected_file(
                    &by_path,
                    &mut blockers,
                    id,
                    &rooted(root, "Cargo.toml"),
                    "Cargo manifest",
                );
            }
            Healthcheck::Npm {
                lockfile,
                build_script,
                typecheck_script,
                test_script,
                ..
            } => {
                let package = rooted(root, "package.json");
                require_projected_file(&by_path, &mut blockers, id, &package, "npm manifest");
                require_projected_file(
                    &by_path,
                    &mut blockers,
                    id,
                    &rooted(root, lockfile),
                    "npm lockfile",
                );
                if let Some(bytes) = by_path
                    .get(package.as_str())
                    .and_then(|entry| entry.bytes.as_deref())
                {
                    match serde_json::from_slice::<serde_json::Value>(bytes) {
                        Ok(document) => {
                            let scripts = document
                                .get("scripts")
                                .and_then(serde_json::Value::as_object);
                            for script in build_script
                                .iter()
                                .chain(typecheck_script)
                                .chain(test_script)
                            {
                                if !scripts.is_some_and(|values| {
                                    values.get(script).is_some_and(serde_json::Value::is_string)
                                }) {
                                    block(
                                        &mut blockers,
                                        id,
                                        "health-projected-npm-script-missing",
                                        format!(
                                            "npm script `{script}` does not survive the projected manifest"
                                        ),
                                    );
                                }
                            }
                        }
                        Err(error) => block(
                            &mut blockers,
                            id,
                            "health-projected-npm-invalid",
                            format!("projected npm manifest is invalid JSON: {error}"),
                        ),
                    }
                }
            }
            Healthcheck::Maven { .. } => require_projected_file(
                &by_path,
                &mut blockers,
                id,
                &rooted(root, "pom.xml"),
                "Maven model",
            ),
            Healthcheck::PythonPip { source_roots, .. } => {
                for source in source_roots {
                    let source = rooted(root, source);
                    if !by_path
                        .keys()
                        .any(|path| **path == source || path.starts_with(&(source.clone() + "/")))
                    {
                        block(
                            &mut blockers,
                            id,
                            "health-projected-python-source-missing",
                            format!("Python source root `{source}` does not survive"),
                        );
                    }
                }
            }
            // Custom source/snapshot bytes are intentionally externalized
            // before mutation. Their former project paths may be deleted.
            Healthcheck::Custom { .. } => {}
        }
        validate_test_disposition(row, prepared, &by_path, &discovery, &mut blockers);
    }
    blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    blockers.dedup();
    blockers
}

fn validate_test_disposition(
    row: &Healthcheck,
    prepared: &PreparedHealth,
    entries: &BTreeMap<&str, &crate::rewrite::ProjectedEntry>,
    discovery: &super::discovery::DiscoveryTree,
    blockers: &mut Vec<HealthBlocker>,
) {
    let Some(check) = prepared.checks.iter().find(|check| check.id == row.id()) else {
        return;
    };
    let Some(disposition) = check.tests else {
        return;
    };
    if disposition == TestDisposition::SkippedByContract {
        return;
    }
    let present = match row {
        Healthcheck::Cargo {
            id,
            root,
            workspace,
            all_targets,
            features,
            ..
        } => match super::discovery::cargo_tests(
            discovery,
            &super::model::TestDiscoveryRequest {
                check_id: id.clone(),
                kind: super::model::HealthcheckKind::Cargo,
                root: root.clone(),
                selector: None,
                workspace: *workspace,
                all_targets: *all_targets,
                features: features.clone(),
            },
        ) {
            Ok(TestPresence::Present) => true,
            Ok(TestPresence::Absent) => false,
            Ok(TestPresence::Indeterminate) | Err(_) => {
                block(
                    blockers,
                    id,
                    "health-projected-test-indeterminate",
                    "projected Cargo test model is indeterminate".to_owned(),
                );
                return;
            }
        },
        Healthcheck::Npm {
            root, test_script, ..
        } => {
            let package = rooted(root, "package.json");
            entries
                .get(package.as_str())
                .and_then(|entry| entry.bytes.as_deref())
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
                .and_then(|value| value.get("scripts").cloned())
                .and_then(|value| value.as_object().cloned())
                .is_some_and(|scripts| {
                    test_script
                        .as_ref()
                        .is_some_and(|script| scripts.get(script).is_some())
                })
        }
        Healthcheck::Maven { id, root, .. } => match super::discovery::maven_tests(
            discovery,
            &super::model::TestDiscoveryRequest {
                check_id: id.clone(),
                kind: super::model::HealthcheckKind::Maven,
                root: root.clone(),
                selector: None,
                workspace: false,
                all_targets: false,
                features: Vec::new(),
            },
        ) {
            Ok(TestPresence::Present) => true,
            Ok(TestPresence::Absent) => false,
            Ok(TestPresence::Indeterminate) | Err(_) => {
                block(
                    blockers,
                    id,
                    "health-projected-test-indeterminate",
                    "projected Maven test model is indeterminate".to_owned(),
                );
                return;
            }
        },
        Healthcheck::PythonPip { root, .. } => entries.keys().any(|path| {
            if !below(root, path) || !path.ends_with(".py") {
                return false;
            }
            let name = path.rsplit('/').next().unwrap_or(path);
            name.starts_with("test_") || name.ends_with("_test.py")
        }),
        Healthcheck::Custom { .. } => return,
    };
    let planned_present = disposition.runs();
    if present != planned_present {
        block(
            blockers,
            row.id(),
            "health-projected-test-applicability-changed",
            "test target applicability changes across phases".to_owned(),
        );
    }
}

fn require_projected_file(
    entries: &BTreeMap<&str, &crate::rewrite::ProjectedEntry>,
    blockers: &mut Vec<HealthBlocker>,
    id: &str,
    path: &str,
    label: &str,
) {
    if !entries
        .get(path)
        .is_some_and(|entry| entry.kind == EntryKind::File && entry.bytes.is_some())
    {
        block(
            blockers,
            id,
            "health-projected-operand-missing",
            format!("{label} `{path}` does not survive the projected tree"),
        );
    }
}

fn health_common(row: &Healthcheck) -> (&str, &str, Option<&crate::contract::When>) {
    match row {
        Healthcheck::Cargo { id, root, when, .. }
        | Healthcheck::Npm { id, root, when, .. }
        | Healthcheck::Maven { id, root, when, .. }
        | Healthcheck::PythonPip { id, root, when, .. }
        | Healthcheck::Custom { id, root, when, .. } => (id, root, when.as_ref()),
    }
}

fn rooted(root: &str, path: &str) -> String {
    if root == "." {
        path.to_owned()
    } else {
        format!("{root}/{path}")
    }
}

fn below(root: &str, path: &str) -> bool {
    root == "." || path == root || path.starts_with(&(root.to_owned() + "/"))
}

fn block(blockers: &mut Vec<HealthBlocker>, check_id: &str, code: &str, message: String) {
    blockers.push(HealthBlocker {
        code: code.to_owned(),
        check_id: Some(check_id.to_owned()),
        message,
    });
}
