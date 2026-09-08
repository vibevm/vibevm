use super::*;

#[allow(dead_code)]
fn cargo_test_presence(
    project: &Project,
    inventory: &crate::model::Inventory,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let root_manifest = rooted(&request.root, "Cargo.toml");
    let root = read_toml(project, &root_manifest)?;
    let mut package_roots = Vec::new();
    if root.get("package").is_some() && !request.workspace {
        package_roots.push(request.root.clone());
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
            let Some(expanded) = expand_cargo_member(&request.root, member, inventory) else {
                return Ok(TestPresence::Indeterminate);
            };
            package_roots.extend(expanded);
        }
        if request.workspace && root.get("package").is_some() {
            package_roots.push(request.root.clone());
        }
    } else {
        return Ok(TestPresence::Indeterminate);
    }
    package_roots.sort();
    package_roots.dedup();
    for package_root in package_roots {
        let manifest = read_toml(project, &rooted(&package_root, "Cargo.toml"))?;
        match cargo_package_test_target(&manifest, &package_root, inventory, request.all_targets) {
            TestPresence::Present => return Ok(TestPresence::Present),
            TestPresence::Indeterminate => return Ok(TestPresence::Indeterminate),
            TestPresence::Absent => {}
        }
    }
    Ok(TestPresence::Absent)
}

fn cargo_package_test_target(
    manifest: &toml::Value,
    root: &str,
    inventory: &crate::model::Inventory,
    all_targets: bool,
) -> TestPresence {
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return TestPresence::Indeterminate;
    };
    let target_enabled = |table: &toml::value::Table| {
        table
            .get("test")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true)
    };
    let explicit_enabled = |key: &str| {
        manifest
            .get(key)
            .and_then(toml::Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .filter_map(toml::Value::as_table)
                    .any(&target_enabled)
            })
    };
    if manifest
        .get("lib")
        .and_then(toml::Value::as_table)
        .is_some_and(&target_enabled)
        || (manifest.get("lib").is_none() && inventory_file(inventory, &rooted(root, "src/lib.rs")))
        || explicit_enabled("bin")
        || explicit_enabled("test")
    {
        return TestPresence::Present;
    }
    let autobins = package
        .get("autobins")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autobins
        && (inventory_file(inventory, &rooted(root, "src/main.rs"))
            || files_below(inventory, &rooted(root, "src/bin"), ".rs"))
    {
        return TestPresence::Present;
    }
    let autotests = package
        .get("autotests")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autotests && files_below(inventory, &rooted(root, "tests"), ".rs") {
        return TestPresence::Present;
    }
    if all_targets && (explicit_enabled("example") || explicit_enabled("bench")) {
        return TestPresence::Present;
    }
    TestPresence::Absent
}

fn expand_cargo_member(
    workspace_root: &str,
    member: &str,
    inventory: &crate::model::Inventory,
) -> Option<Vec<String>> {
    if member.contains("**") || member.contains(['?', '[', ']', '{', '}', '\\', ':']) {
        return None;
    }
    if let Some(prefix) = member.strip_suffix("/*") {
        let prefix = rooted(workspace_root, prefix);
        let mut roots = inventory
            .entries
            .iter()
            .filter(|entry| {
                entry.kind == crate::model::EntryKind::Directory
                    && entry.path.starts_with(&(prefix.clone() + "/"))
                    && !entry.path[(prefix.len() + 1)..].contains('/')
            })
            .map(|entry| entry.path.clone())
            .filter(|root| inventory_file(inventory, &rooted(root, "Cargo.toml")))
            .collect::<Vec<_>>();
        roots.sort();
        Some(roots)
    } else if member.contains('*') {
        None
    } else {
        Some(vec![rooted(workspace_root, member)])
    }
}

fn read_toml(project: &Project, path: &str) -> Result<toml::Value, HealthError> {
    let bytes = project
        .read_file_bounded(path, MANIFEST_CAP)
        .map_err(|error| HealthError::Preparation(format!("reading `{path}`: {error:#}")))?
        .ok_or_else(|| HealthError::Preparation(format!("manifest `{path}` is absent")))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| HealthError::Preparation(format!("`{path}` is not UTF-8: {error}")))?;
    toml::from_str(text)
        .map_err(|error| HealthError::Preparation(format!("invalid `{path}`: {error}")))
}

fn inventory_file(inventory: &crate::model::Inventory, path: &str) -> bool {
    inventory
        .entries
        .iter()
        .any(|entry| entry.path == path && entry.kind == crate::model::EntryKind::File)
}

fn files_below(inventory: &crate::model::Inventory, root: &str, suffix: &str) -> bool {
    inventory.entries.iter().any(|entry| {
        entry.kind == crate::model::EntryKind::File
            && entry.path.starts_with(&(root.to_owned() + "/"))
            && entry.path.ends_with(suffix)
    })
}

#[allow(dead_code)]
fn maven_test_presence(
    project: &Project,
    inventory: &crate::model::Inventory,
    request: &TestDiscoveryRequest,
) -> Result<TestPresence, HealthError> {
    let config = rooted(&request.root, ".mvn/maven.config");
    if inventory_file(inventory, &config) {
        let bytes = project
            .read_file_bounded(&config, MANIFEST_CAP)
            .map_err(|error| {
                HealthError::Preparation(format!("reading Maven config `{config}`: {error:#}"))
            })?
            .ok_or_else(|| HealthError::Preparation(format!("Maven config `{config}` vanished")))?;
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            HealthError::Preparation(format!("Maven config `{config}` is not UTF-8: {error}"))
        })?;
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
        let module_root = modules[index].clone();
        index += 1;
        let pom = rooted(&module_root, "pom.xml");
        let bytes = project
            .read_file_bounded(&pom, MANIFEST_CAP)
            .map_err(|error| {
                HealthError::Preparation(format!("reading Maven model `{pom}`: {error:#}"))
            })?
            .ok_or_else(|| HealthError::Preparation(format!("Maven model `{pom}` is absent")))?;
        let model = parse_maven_model(&bytes)?;
        if model.indeterminate {
            return Ok(TestPresence::Indeterminate);
        }
        if !model.tests_skipped {
            let sources = if model.test_sources.is_empty() {
                vec!["src/test/java".to_owned()]
            } else {
                model.test_sources
            };
            if sources.iter().any(|source| {
                !source.contains("${")
                    && inventory.entries.iter().any(|entry| {
                        let root = rooted(&module_root, source);
                        entry.kind == crate::model::EntryKind::File
                            && entry.path.starts_with(&(root + "/"))
                    })
            }) {
                return Ok(TestPresence::Present);
            }
        }
        for module in model.modules {
            if module.is_empty()
                || module.contains("${")
                || module.contains(['\\', ':'])
                || module
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
            {
                return Ok(TestPresence::Indeterminate);
            }
            modules.push(rooted(&module_root, &module));
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
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<String>::new();
    let mut model = MavenModel::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                if name == "profiles" {
                    model.indeterminate = true;
                }
                stack.push(name);
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Text(text)) => {
                let value = text.decode().map_err(|error| {
                    HealthError::Preparation(format!("decoding Maven model text: {error}"))
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
                    "invalid Maven POM XML: {error}"
                )));
            }
        }
    }
    model.modules.sort();
    model.modules.dedup();
    model.test_sources.sort();
    model.test_sources.dedup();
    Ok(model)
}
