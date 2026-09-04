//! One manifest/model discovery implementation shared by before and projected-after trees.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::collections::BTreeMap;

use quick_xml::events::Event;
use vibe_safefs::Project;

use crate::model::{EntryKind, Inventory};

use super::model::{HealthError, TestDiscoveryRequest, TestPresence};

const MANIFEST_CAP: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct DiscoveryTree {
    entries: BTreeMap<String, DiscoveryEntry>,
}

struct DiscoveryEntry {
    kind: EntryKind,
    bytes: Option<Vec<u8>>,
}

impl DiscoveryTree {
    pub(crate) fn before(project: &Project, inventory: &Inventory) -> Result<Self, HealthError> {
        let mut entries = BTreeMap::new();
        for entry in &inventory.entries {
            let bytes = if entry.kind == EntryKind::File && is_model_file(&entry.path) {
                project
                    .read_file_bounded(&entry.path, MANIFEST_CAP)
                    .map_err(|error| {
                        HealthError::Preparation(format!(
                            "reading discovery model `{}`: {error:#}",
                            entry.path
                        ))
                    })?
            } else {
                None
            };
            entries.insert(
                entry.path.clone(),
                DiscoveryEntry {
                    kind: entry.kind,
                    bytes,
                },
            );
        }
        Ok(Self { entries })
    }

    pub(crate) fn projected(entries: &[crate::rewrite::ProjectedEntry]) -> Self {
        Self {
            entries: entries
                .iter()
                .map(|entry| {
                    (
                        entry.path.clone(),
                        DiscoveryEntry {
                            kind: entry.kind,
                            bytes: entry.bytes.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn is_file(&self, path: &str) -> bool {
        self.entries
            .get(path)
            .is_some_and(|entry| entry.kind == EntryKind::File)
    }

    fn files_below(&self, root: &str, suffix: &str) -> bool {
        self.entries.iter().any(|(path, entry)| {
            entry.kind == EntryKind::File
                && path.starts_with(&(root.to_owned() + "/"))
                && path.ends_with(suffix)
        })
    }

    fn bytes(&self, path: &str) -> Result<&[u8], HealthError> {
        self.entries
            .get(path)
            .and_then(|entry| entry.bytes.as_deref())
            .ok_or_else(|| {
                HealthError::Preparation(format!(
                    "discovery model `{path}` is absent or has no prepared bytes"
                ))
            })
    }

    fn direct_member_roots(&self, prefix: &str) -> Vec<String> {
        let mut roots = self
            .entries
            .iter()
            .filter(|(path, entry)| {
                entry.kind == EntryKind::Directory
                    && path.starts_with(&(prefix.to_owned() + "/"))
                    && !path[(prefix.len() + 1)..].contains('/')
            })
            .map(|(path, _)| path.clone())
            .filter(|root| self.is_file(&rooted(root, "Cargo.toml")))
            .collect::<Vec<_>>();
        roots.sort();
        roots
    }
}

pub(crate) fn cargo_tests(
    tree: &DiscoveryTree,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let root = parse_toml(tree, &rooted(&request.root, "Cargo.toml"))?;
    let mut roots = Vec::new();
    if root.get("package").is_some() && !request.workspace {
        roots.push(request.root.clone());
    } else if let Some(workspace) = root.get("workspace").and_then(toml::Value::as_table) {
        let key = if request.workspace || root.get("package").is_none() {
            "members"
        } else {
            "default-members"
        };
        let Some(members) = workspace.get(key).and_then(toml::Value::as_array) else {
            return Ok(TestPresence::Indeterminate);
        };
        for member in members {
            let Some(member) = member.as_str() else {
                return Ok(TestPresence::Indeterminate);
            };
            if let Some(prefix) = member.strip_suffix("/*") {
                roots.extend(tree.direct_member_roots(&rooted(&request.root, prefix)));
            } else if member.contains('*')
                || member.contains("${")
                || member
                    .split('/')
                    .any(|part| matches!(part, "" | "." | ".."))
            {
                return Ok(TestPresence::Indeterminate);
            } else {
                roots.push(rooted(&request.root, member));
            }
        }
        if request.workspace && root.get("package").is_some() {
            roots.push(request.root.clone());
        }
    } else {
        return Ok(TestPresence::Indeterminate);
    }
    roots.sort();
    roots.dedup();
    for root in roots {
        let manifest = parse_toml(tree, &rooted(&root, "Cargo.toml"))?;
        match cargo_package_tests(tree, &manifest, &root, request) {
            TestPresence::Present => return Ok(TestPresence::Present),
            TestPresence::Indeterminate => return Ok(TestPresence::Indeterminate),
            TestPresence::Absent => {}
        }
    }
    Ok(TestPresence::Absent)
}

fn cargo_package_tests(
    tree: &DiscoveryTree,
    manifest: &toml::Value,
    root: &str,
    request: &TestDiscoveryRequest,
) -> TestPresence {
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return TestPresence::Indeterminate;
    };
    let enabled = |table: &toml::value::Table| {
        table
            .get("test")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true)
            && table
                .get("required-features")
                .and_then(toml::Value::as_array)
                .map(|features| {
                    features.iter().all(|feature| {
                        feature.as_str().is_some_and(|feature| {
                            request.features.iter().any(|selected| selected == feature)
                        })
                    })
                })
                .unwrap_or(true)
    };
    let explicit = |key: &str| {
        manifest
            .get(key)
            .and_then(toml::Value::as_array)
            .is_some_and(|rows| rows.iter().filter_map(toml::Value::as_table).any(&enabled))
    };
    let auto_lib = package
        .get("autolib")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if manifest
        .get("lib")
        .and_then(toml::Value::as_table)
        .is_some_and(&enabled)
        || (manifest.get("lib").is_none() && auto_lib && tree.is_file(&rooted(root, "src/lib.rs")))
        || explicit("bin")
        || explicit("test")
    {
        return TestPresence::Present;
    }
    if package
        .get("autobins")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
        && (tree.is_file(&rooted(root, "src/main.rs"))
            || tree.files_below(&rooted(root, "src/bin"), ".rs"))
    {
        return TestPresence::Present;
    }
    if package
        .get("autotests")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
        && tree.files_below(&rooted(root, "tests"), ".rs")
    {
        return TestPresence::Present;
    }
    if request.all_targets && (explicit("example") || explicit("bench")) {
        return TestPresence::Present;
    }
    TestPresence::Absent
}

pub(crate) fn maven_tests(
    tree: &DiscoveryTree,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let config = rooted(&request.root, ".mvn/maven.config");
    if tree.is_file(&config) {
        let text = utf8(tree.bytes(&config)?, &config)?;
        if text.split_whitespace().any(|arg| {
            matches!(
                arg,
                "-DskipTests" | "-DskipTests=true" | "-Dmaven.test.skip=true"
            )
        }) {
            return Ok(TestPresence::Absent);
        }
        if text
            .split_whitespace()
            .any(|arg| arg.starts_with("-P") || arg.contains("testSourceDirectory"))
        {
            return Ok(TestPresence::Indeterminate);
        }
    }
    let mut modules = vec![request.root.clone()];
    let mut index = 0;
    while index < modules.len() {
        let root = modules[index].clone();
        index += 1;
        let model = parse_maven_model(tree.bytes(&rooted(&root, "pom.xml"))?)?;
        if model.indeterminate {
            return Ok(TestPresence::Indeterminate);
        }
        let sources = if model.test_sources.is_empty() {
            vec!["src/test/java".to_owned()]
        } else {
            model.test_sources
        };
        if !model.tests_skipped
            && sources.iter().any(|source| {
                !source.contains("${") && tree.files_below(&rooted(&root, source), "")
            })
        {
            return Ok(TestPresence::Present);
        }
        for module in model.modules {
            if module.contains("${")
                || module
                    .split('/')
                    .any(|part| matches!(part, "" | "." | ".."))
            {
                return Ok(TestPresence::Indeterminate);
            }
            modules.push(rooted(&root, &module));
        }
    }
    Ok(TestPresence::Absent)
}

#[derive(Default)]
struct MavenModel {
    modules: Vec<String>,
    test_sources: Vec<String>,
    tests_skipped: bool,
    indeterminate: bool,
}

fn parse_maven_model(bytes: &[u8]) -> Result<MavenModel, HealthError> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<String>::new();
    let mut model = MavenModel::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                model.indeterminate |= name == "profiles";
                stack.push(name);
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Text(text)) => {
                let value = text.decode().map_err(|error| {
                    HealthError::Preparation(format!("decoding Maven model: {error}"))
                })?;
                match stack.last().map(String::as_str) {
                    Some("module") => model.modules.push(value.into_owned()),
                    Some("testSourceDirectory") => model.test_sources.push(value.into_owned()),
                    Some("skipTests" | "maven.test.skip") if value.eq_ignore_ascii_case("true") => {
                        model.tests_skipped = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(HealthError::Preparation(format!(
                    "invalid Maven POM: {error}"
                )));
            }
        }
    }
    Ok(model)
}

fn parse_toml(tree: &DiscoveryTree, path: &str) -> Result<toml::Value, HealthError> {
    toml::from_str(utf8(tree.bytes(path)?, path)?)
        .map_err(|error| HealthError::Preparation(format!("invalid `{path}`: {error}")))
}

fn utf8<'a>(bytes: &'a [u8], path: &str) -> Result<&'a str, HealthError> {
    std::str::from_utf8(bytes)
        .map_err(|error| HealthError::Preparation(format!("`{path}` is not UTF-8: {error}")))
}

fn rooted(root: &str, path: &str) -> String {
    if root == "." {
        path.to_owned()
    } else {
        format!("{root}/{path}")
    }
}

fn is_model_file(path: &str) -> bool {
    path.ends_with("Cargo.toml")
        || path.ends_with("pom.xml")
        || path.ends_with(".mvn/maven.config")
        || path.ends_with("package.json")
}
