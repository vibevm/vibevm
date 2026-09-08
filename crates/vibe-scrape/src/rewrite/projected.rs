use super::cargo_lock::cargo_contains_identity;
use super::cargo_topology::*;
use super::go_node::{assert_language_absent, cargo_path_prefix_present, go_module_on_line};
use super::text::*;
use super::*;

pub(super) fn validate_projected_final(
    contract: &Contract,
    projected_entries: &[ProjectedEntry],
) -> Result<(), ScrapeError> {
    contract.validate()?;
    let mut by_path = BTreeMap::new();
    for entry in projected_entries {
        crate::glob::PortablePath::parse(&entry.path)?;
        if by_path.insert(entry.path.as_str(), entry).is_some() {
            return Err(fail(format!(
                "projected final inventory contains duplicate path `{}`",
                entry.path
            )));
        }
        if (entry.kind == EntryKind::File) != entry.bytes.is_some() {
            return Err(fail(format!(
                "projected final entry `{}` has bytes inconsistent with its kind",
                entry.path
            )));
        }
    }
    for entry in projected_entries {
        let mut descendant = entry.path.as_str();
        while let Some((parent, _)) = descendant.rsplit_once('/') {
            if by_path
                .get(parent)
                .is_some_and(|ancestor| ancestor.kind == EntryKind::File)
            {
                return Err(fail(format!(
                    "projected final path `{}` has file `{parent}` as an ancestor",
                    entry.path
                )));
            }
            descendant = parent;
        }
    }
    let files = projected_entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    let cargo_topology = CargoTopology::build(
        projected_entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File && entry.path.ends_with("Cargo.toml"))
            .map(|entry| {
                (
                    entry.path.as_str(),
                    entry
                        .bytes
                        .as_deref()
                        .expect("file-kind projected entry has bytes"),
                )
            }),
        files
            .iter()
            .filter(|path| path.ends_with("Cargo.lock"))
            .map(String::as_str),
    )?;
    for assertion in &contract.assertions {
        match assertion {
            Assertion::PathsAbsentV1 { id, patterns } => {
                if let Some(path) = selected_paths(&files, patterns, &[])?.first() {
                    return Err(fail(format!(
                        "assertion `{id}` leaves selected path `{path}` in the scraped tree"
                    )));
                }
            }
            Assertion::TextLiteralAbsentV1 {
                id,
                patterns,
                needles,
            } => {
                for path in selected_paths(&files, patterns, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    if let Some(needle) = needles
                        .iter()
                        .find(|needle| find_subslice(bytes, needle.as_bytes()).is_some())
                    {
                        return Err(fail(format!(
                            "assertion `{id}` finds literal `{needle}` in `{path}`"
                        )));
                    }
                }
            }
            Assertion::CargoPathPrefixAbsentV1 {
                id,
                manifests,
                prefixes,
            } => {
                for path in selected_paths(&files, manifests, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let text = std::str::from_utf8(bytes).map_err(|_| {
                        fail(format!(
                            "assertion `{id}` Cargo target `{path}` is not UTF-8"
                        ))
                    })?;
                    let document = text.parse::<toml_edit::DocumentMut>().map_err(|error| {
                        fail(format!("assertion `{id}` cannot parse `{path}`: {error}"))
                    })?;
                    if cargo_path_prefix_present(document.as_table(), prefixes) {
                        return Err(fail(format!(
                            "assertion `{id}` finds a forbidden Cargo path prefix in `{path}`"
                        )));
                    }
                }
            }
            Assertion::LanguageMetadataAbsentV1 {
                id,
                language,
                patterns,
            } => {
                for path in selected_paths(&files, patterns, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let empty_aliases = BTreeSet::new();
                    let rust_aliases = if *language == Language::Rust {
                        contracted_specmark_aliases_for_source(contract, &cargo_topology, &path)?
                    } else {
                        BTreeSet::new()
                    };
                    if !assert_language_absent(
                        bytes,
                        *language,
                        if *language == Language::Rust {
                            &rust_aliases
                        } else {
                            &empty_aliases
                        },
                        path.ends_with(".tsx"),
                    )? {
                        return Err(fail(format!(
                            "assertion `{id}` finds registered language metadata in `{path}`"
                        )));
                    }
                }
            }
            Assertion::DependencyIdentitiesAbsentV1 {
                id,
                manager,
                manifests,
                identities,
            } => {
                for path in selected_paths(&files, manifests, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let present = match manager {
                        DependencyManager::Cargo => {
                            let mut found = false;
                            for identity in identities {
                                found |= cargo_document_contains_identity(bytes, identity)?;
                            }
                            found
                        }
                        DependencyManager::Npm
                        | DependencyManager::Pnpm
                        | DependencyManager::Yarn => {
                            let value: serde_json::Value =
                                serde_json::from_slice(bytes).map_err(|error| {
                                    fail(format!("assertion `{id}` cannot parse `{path}`: {error}"))
                                })?;
                            [
                                "dependencies",
                                "devDependencies",
                                "optionalDependencies",
                                "peerDependencies",
                            ]
                            .iter()
                            .filter_map(|table| {
                                value.get(*table).and_then(serde_json::Value::as_object)
                            })
                            .any(|table| {
                                identities
                                    .iter()
                                    .any(|identity| table.contains_key(identity))
                            })
                        }
                        DependencyManager::Go => {
                            let text = std::str::from_utf8(bytes).map_err(|_| {
                                fail(format!("assertion `{id}` target `{path}` is not UTF-8"))
                            })?;
                            identities.iter().any(|identity| {
                                text.lines()
                                    .any(|line| go_module_on_line(line, identity, None))
                            })
                        }
                    };
                    if present {
                        return Err(fail(format!(
                            "assertion `{id}` finds a forbidden dependency identity in `{path}`"
                        )));
                    }
                }
            }
        }
    }
    validate_registered_rewrite_residue(contract, &by_path, &files, &cargo_topology)
}

fn projected_bytes<'a>(
    entries: &BTreeMap<&str, &'a ProjectedEntry>,
    path: &str,
) -> Result<&'a [u8], ScrapeError> {
    entries
        .get(path)
        .and_then(|entry| entry.bytes.as_deref())
        .ok_or_else(|| {
            fail(format!(
                "projected final file `{path}` has no complete bytes"
            ))
        })
}

fn cargo_document_contains_identity(bytes: &[u8], package: &str) -> Result<bool, ScrapeError> {
    let text = std::str::from_utf8(bytes).map_err(|_| fail("Cargo manifest is not UTF-8"))?;
    let document = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| fail(format!("cannot parse Cargo manifest: {error}")))?;
    Ok(cargo_contains_identity(
        document.as_table(),
        package,
        &BTreeSet::new(),
    ))
}

fn contracted_specmark_aliases_for_source(
    contract: &Contract,
    topology: &CargoTopology,
    source: &str,
) -> Result<BTreeSet<String>, ScrapeError> {
    let owner = topology.source_manifest(source)?;
    let matching = contract
        .rewrite
        .iter()
        .filter_map(|rule| {
            let RewriteRule::CargoPackageRemoveV1 {
                manifests,
                package,
                aliases,
                ..
            } = rule
            else {
                return None;
            };
            (package == "core-ai-native-specmark").then_some((manifests, package, aliases))
        })
        .filter_map(
            |row| match cargo_rule_selects_manifest(row.0, &owner.path) {
                Ok(true) => Some(Ok(row)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let (_, package, aliases) = match matching.as_slice() {
        [only] => *only,
        [] => return Ok(BTreeSet::new()),
        _ => {
            return Err(fail(format!(
                "Rust source `{source}` has multiple owning Specmark Cargo removal rules"
            )));
        }
    };
    Ok(aliases
        .iter()
        .map(|alias| alias.replace('-', "_"))
        .chain(std::iter::once(package.replace('-', "_")))
        .collect())
}

fn validate_registered_rewrite_residue(
    contract: &Contract,
    entries: &BTreeMap<&str, &ProjectedEntry>,
    files: &BTreeSet<String>,
    cargo_topology: &CargoTopology,
) -> Result<(), ScrapeError> {
    for rule in &contract.rewrite {
        match rule {
            RewriteRule::ManagedBlockRemoveV1 {
                id, paths, marker, ..
            } => {
                let (begin, end) = managed_markers(marker);
                for path in paths {
                    let Some(entry) = entries.get(path.as_str()) else {
                        continue;
                    };
                    let bytes = entry.bytes.as_deref().ok_or_else(|| {
                        fail(format!("projected managed target `{path}` has no bytes"))
                    })?;
                    if find_subslice(bytes, &begin).is_some()
                        || find_subslice(bytes, &end).is_some()
                    {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered managed marker `{marker}` in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::RustSpecmarkStripV1 {
                id,
                patterns,
                exclude,
                forms,
                ..
            } => {
                let forms = forms
                    .iter()
                    .map(|form| {
                        match form {
                            RustForm::Scope => "scope",
                            RustForm::Spec => "spec",
                            RustForm::Verifies => "verifies",
                            RustForm::Cell => "cell",
                        }
                        .to_owned()
                    })
                    .collect::<BTreeSet<_>>();
                for path in selected_paths(files, patterns, exclude)? {
                    if !path.ends_with(".rs") {
                        continue;
                    }
                    let bytes = projected_bytes(entries, &path)?;
                    let rust_aliases =
                        contracted_specmark_aliases_for_source(contract, cargo_topology, &path)?;
                    if prepare_rust(bytes, &rust_aliases, &forms)?.1 != 0 {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered Rust metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                ..
            } => {
                let selected_manifests = selected_paths(files, manifests, &[])?;
                for path in &selected_manifests {
                    if cargo_document_contains_identity(projected_bytes(entries, path)?, package)? {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves Cargo package `{package}` in `{path}`"
                        )));
                    }
                }
                for lockfile in
                    cargo_topology.owned_locks(selected_manifests.iter().map(String::as_str))?
                {
                    match prepare_cargo_lock(projected_bytes(entries, &lockfile)?, package) {
                        Ok(_) => {}
                        Err(ScrapeError::Blocked(message)) => {
                            return Err(fail(format!(
                                "rewrite `{id}` leaves unresolved Cargo.lock identity in `{lockfile}`: {message}"
                            )));
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::NodePackageRemoveV1 {
                id,
                package_json,
                lockfile,
                manager,
                packages,
                script_paths,
                config_paths,
                ..
            } => {
                let Some(_) = entries.get(package_json.as_str()) else {
                    continue;
                };
                let bytes = projected_bytes(entries, package_json)?;
                let (_, count, _, _) =
                    prepare_node_manifest(bytes, packages, script_paths, config_paths)?;
                if count != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered Node manifest identities in `{package_json}`"
                    )));
                }
                let lock_bytes = projected_bytes(entries, lockfile)?;
                match prepare_node_lock(lock_bytes, *manager, packages) {
                    Ok(_) => {}
                    Err(ScrapeError::Blocked(message)) => {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves unresolved Node lock identity in `{lockfile}`: {message}"
                        )));
                    }
                    Err(error) => return Err(error),
                }
            }
            RewriteRule::GoModuleRemoveV1 {
                id,
                go_mod,
                go_sum,
                modules,
                ..
            } => {
                let Some(_) = entries.get(go_mod.as_str()) else {
                    continue;
                };
                if prepare_go_mod(projected_bytes(entries, go_mod)?, modules)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered Go module identities in `{go_mod}`"
                    )));
                }
                if let Some(go_sum) = go_sum
                    && entries.contains_key(go_sum.as_str())
                {
                    match prepare_go_sum(projected_bytes(entries, go_sum)?, modules) {
                        Ok(_) => {}
                        Err(ScrapeError::Blocked(message)) => {
                            return Err(fail(format!(
                                "rewrite `{id}` leaves unresolved Go checksum identity in `{go_sum}`: {message}"
                            )));
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::TomlArrayValuesRemoveV1 {
                id,
                path,
                table,
                key,
                values,
                ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if prepare_toml_array(projected_bytes(entries, path)?, table, key, values)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered TOML values in `{path}`"
                    )));
                }
            }
            RewriteRule::TypeScriptSpecCommentsStripV1 {
                id,
                patterns,
                exclude,
                ..
            } => {
                for path in selected_paths(files, patterns, exclude)? {
                    if prepare_typescript(projected_bytes(entries, &path)?, path.ends_with(".tsx"))?
                        .1
                        != 0
                    {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered TypeScript metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::GoSpecDirectivesStripV1 {
                id,
                patterns,
                exclude,
                ..
            } => {
                for path in selected_paths(files, patterns, exclude)? {
                    if prepare_go_directives(projected_bytes(entries, &path)?)?.1 != 0 {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered Go metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::JsonMemberRemoveV1 {
                id,
                path,
                object,
                members,
                ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if prepare_json_members(projected_bytes(entries, path)?, object, members)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered JSON members in `{path}`"
                    )));
                }
            }
            RewriteRule::TextExactReplaceV1 {
                id, path, before, ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if find_subslice(projected_bytes(entries, path)?, before.as_bytes()).is_some() {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves its registered preimage in `{path}`"
                    )));
                }
            }
        }
    }
    Ok(())
}
