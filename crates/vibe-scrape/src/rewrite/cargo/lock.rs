use super::text::*;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CargoLockPackageId {
    name: String,
    version: String,
    source: Option<String>,
}

#[derive(Debug)]
struct CargoLockDependency {
    array_index: usize,
    target: usize,
}

#[derive(Debug)]
struct CargoLockNode {
    id: CargoLockPackageId,
    dependencies: Vec<CargoLockDependency>,
}

#[derive(Debug)]
struct CargoLockGraph {
    nodes: Vec<CargoLockNode>,
    roots: Vec<usize>,
}

type CargoLockOutput = (RewriteOutput, Option<NativeLockEvidence>);

fn cargo_lock_blocked(message: impl Into<String>) -> ScrapeError {
    ScrapeError::blocked(message)
}

fn cargo_lock_package_id(
    table: &toml_edit::Table,
    index: usize,
) -> Result<CargoLockPackageId, ScrapeError> {
    let field = |name: &str| {
        table
            .get(name)
            .and_then(toml_edit::Item::as_str)
            .ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has no string `{name}`"
                ))
            })
    };
    let source = match table.get("source") {
        None => None,
        Some(item) => {
            let value = item.as_str().ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has a non-string `source`"
                ))
            })?;
            if value.is_empty() {
                return Err(cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has an empty `source`"
                )));
            }
            Some(value.to_owned())
        }
    };
    Ok(CargoLockPackageId {
        name: field("name")?.to_owned(),
        version: field("version")?.to_owned(),
        source,
    })
}

fn cargo_lock_dependency_selector(
    value: &str,
) -> Result<(&str, Option<&str>, Option<&str>), ScrapeError> {
    let mut fields = value.split_ascii_whitespace();
    let name = fields
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| cargo_lock_blocked("Cargo.lock contains an empty dependency selector"))?;
    let Some(version) = fields.next() else {
        return Ok((name, None, None));
    };
    let rest = fields.collect::<Vec<_>>();
    if rest.is_empty() {
        return Ok((name, Some(version), None));
    }
    if rest.len() != 1 {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` has an unsupported shape"
        )));
    }
    let source = rest[0]
        .strip_prefix('(')
        .and_then(|source| source.strip_suffix(')'))
        .filter(|source| !source.is_empty())
        .ok_or_else(|| {
            cargo_lock_blocked(format!(
                "Cargo.lock dependency selector `{value}` has an invalid source"
            ))
        })?;
    Ok((name, Some(version), Some(source)))
}

fn resolve_cargo_lock_dependency(
    value: &str,
    identities: &[CargoLockPackageId],
) -> Result<usize, ScrapeError> {
    let (name, version, source) = cargo_lock_dependency_selector(value)?;
    let candidates = identities
        .iter()
        .enumerate()
        .filter(|(_, identity)| {
            identity.name == name
                && version.is_none_or(|version| identity.version == version)
                && source.is_none_or(|source| identity.source.as_deref() == Some(source))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [only] => Ok(*only),
        [] => Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` resolves to no [[package]]"
        ))),
        _ => Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` is ambiguous"
        ))),
    }
}

fn parse_cargo_lock_graph(
    document: &toml_edit::DocumentMut,
) -> Result<CargoLockGraph, ScrapeError> {
    match document
        .get("version")
        .and_then(toml_edit::Item::as_integer)
    {
        Some(3 | 4) => {}
        Some(version) => {
            return Err(cargo_lock_blocked(format!(
                "Cargo.lock format version {version} is unsupported by schema-1 reconciliation"
            )));
        }
        None => {
            return Err(cargo_lock_blocked(
                "Cargo.lock has no supported integer format version",
            ));
        }
    }
    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| cargo_lock_blocked("Cargo.lock has no [[package]] graph"))?;
    if packages.is_empty() {
        return Err(cargo_lock_blocked(
            "Cargo.lock has an empty [[package]] graph",
        ));
    }
    let identities = packages
        .iter()
        .enumerate()
        .map(|(index, table)| cargo_lock_package_id(table, index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = BTreeSet::new();
    if let Some(duplicate) = identities
        .iter()
        .find(|identity| !unique.insert((*identity).clone()))
    {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock repeats package identity `{}`",
            cargo_lock_identity(duplicate)
        )));
    }
    let mut indegree = vec![0_usize; identities.len()];
    let mut nodes = Vec::with_capacity(identities.len());
    for (index, table) in packages.iter().enumerate() {
        let mut dependencies = Vec::new();
        let mut dependency_targets = BTreeSet::new();
        if let Some(item) = table.get("dependencies") {
            let array = item.as_array().ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock package `{}` has non-array `dependencies`",
                    cargo_lock_identity(&identities[index])
                ))
            })?;
            for (array_index, value) in array.iter().enumerate() {
                let selector = value.as_str().ok_or_else(|| {
                    cargo_lock_blocked(format!(
                        "Cargo.lock package `{}` has a non-string dependency",
                        cargo_lock_identity(&identities[index])
                    ))
                })?;
                let target = resolve_cargo_lock_dependency(selector, &identities)?;
                if !dependency_targets.insert(target) {
                    return Err(cargo_lock_blocked(format!(
                        "Cargo.lock package `{}` repeats dependency target `{selector}`",
                        cargo_lock_identity(&identities[index])
                    )));
                }
                indegree[target] = indegree[target].checked_add(1).ok_or_else(|| {
                    cargo_lock_blocked("Cargo.lock dependency indegree overflows usize")
                })?;
                dependencies.push(CargoLockDependency {
                    array_index,
                    target,
                });
            }
        }
        nodes.push(CargoLockNode {
            id: identities[index].clone(),
            dependencies,
        });
    }
    let roots = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect();
    Ok(CargoLockGraph { nodes, roots })
}

fn cargo_lock_identity(identity: &CargoLockPackageId) -> String {
    let source = identity.source.as_deref().unwrap_or("");
    format!(
        "n{}:{}|v{}:{}|s{}:{}",
        identity.name.len(),
        identity.name,
        identity.version.len(),
        identity.version,
        source.len(),
        source
    )
}

fn cargo_lock_graph_evidence(graph: &CargoLockGraph) -> Vec<String> {
    let mut evidence = graph
        .nodes
        .iter()
        .map(|node| format!("node|{}", cargo_lock_identity(&node.id)))
        .collect::<Vec<_>>();
    evidence.extend(graph.nodes.iter().flat_map(|node| {
        node.dependencies.iter().map(|dependency| {
            format!(
                "edge|{}|{}",
                cargo_lock_identity(&node.id),
                cargo_lock_identity(&graph.nodes[dependency.target].id)
            )
        })
    }));
    evidence.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    evidence
}

fn cargo_lock_package_spans(
    source: &[u8],
    expected_packages: usize,
) -> Result<Vec<std::ops::Range<usize>>, ScrapeError> {
    let starts = line_spans(source)
        .into_iter()
        .filter_map(|(line_start, content_end, _)| {
            (source[line_start..content_end].trim_ascii() == b"[[package]]").then_some(line_start)
        })
        .collect::<Vec<_>>();
    if starts.len() != expected_packages {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock has {expected_packages} parsed packages but {} canonical [[package]] headers; annotated or noncanonical headers are unsupported",
            starts.len()
        )));
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(index, start)| *start..starts.get(index + 1).copied().unwrap_or(source.len()))
        .collect())
}

fn cargo_lock_value_span(
    source: &[u8],
    package_span: &std::ops::Range<usize>,
    value: &toml_edit::Value,
    label: &str,
) -> Result<ByteSpan, ScrapeError> {
    let span = value.span().or_else(|| {
        let rendered = value.to_string();
        unique_subslice_span(&source[package_span.clone()], rendered.as_bytes())
            .map(|span| package_span.start + span.start..package_span.start + span.end)
    });
    let span = span.filter(|span| {
        span.start >= package_span.start && span.end <= package_span.end && span.start <= span.end
    });
    let span = span.ok_or_else(|| {
        cargo_lock_blocked(format!(
            "{label} has no unambiguous source span inside its package table"
        ))
    })?;
    Ok(ByteSpan {
        start: u64::try_from(span.start)
            .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
        end: u64::try_from(span.end)
            .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
        node: label.to_owned(),
    })
}

fn cargo_lock_reachable(
    graph: &CargoLockGraph,
    root: usize,
    cut_root_edges: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut reachable = BTreeSet::from([root]);
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        for dependency in &graph.nodes[node].dependencies {
            if node == root && cut_root_edges.contains(&dependency.array_index) {
                continue;
            }
            if reachable.insert(dependency.target) {
                pending.push(dependency.target);
            }
        }
    }
    reachable
}

pub(super) fn prepare_cargo_lock(
    before: &[u8],
    package: &str,
) -> Result<CargoLockOutput, ScrapeError> {
    let source =
        std::str::from_utf8(before).map_err(|_| cargo_lock_blocked("Cargo.lock is not UTF-8"))?;
    let mut document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| cargo_lock_blocked(format!("cannot parse Cargo.lock: {error}")))?;
    let before_graph = parse_cargo_lock_graph(&document)?;
    let targets = before_graph
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node.id.name == package).then_some(index))
        .collect::<BTreeSet<_>>();
    if targets.is_empty() {
        return Ok(((before.to_vec(), 0, Vec::new(), Vec::new()), None));
    }
    let root = match before_graph.roots.as_slice() {
        [root] => *root,
        [] => {
            return Err(cargo_lock_blocked(
                "Cargo.lock graph has no unique root package",
            ));
        }
        roots => {
            return Err(cargo_lock_blocked(format!(
                "Cargo.lock graph has {} root packages; schema-1 reconciliation supports exactly one",
                roots.len()
            )));
        }
    };
    if before_graph.nodes[root].id.source.is_some() {
        return Err(cargo_lock_blocked(
            "Cargo.lock unique graph root is registry/source-backed rather than a local project package",
        ));
    }
    if targets.contains(&root) {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock root package is the requested removal identity `{package}`"
        )));
    }
    let cut_root_edges = before_graph.nodes[root]
        .dependencies
        .iter()
        .filter_map(|dependency| {
            targets
                .contains(&dependency.target)
                .then_some(dependency.array_index)
        })
        .collect::<BTreeSet<_>>();
    if cut_root_edges.is_empty() {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock package `{package}` is not a direct dependency of the unique root; manifest-to-lock authorization is ambiguous"
        )));
    }
    let reachable = cargo_lock_reachable(&before_graph, root, &cut_root_edges);
    if targets.iter().any(|target| reachable.contains(target)) {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock package `{package}` remains reachable through a retained dependency"
        )));
    }
    let removed_indices = (0..before_graph.nodes.len())
        .filter(|index| !reachable.contains(index))
        .collect::<BTreeSet<_>>();
    let mut removed = removed_indices
        .iter()
        .map(|index| cargo_lock_identity(&before_graph.nodes[*index].id))
        .collect::<Vec<_>>();
    removed.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .expect("graph parser established [[package]]");
    let package_spans = cargo_lock_package_spans(before, packages.len())?;
    let root_dependencies = packages
        .get(root)
        .expect("root index came from the parsed graph")
        .get("dependencies")
        .and_then(toml_edit::Item::as_array)
        .expect("cut edges establish a root dependency array");
    let mut spans = cut_root_edges
        .iter()
        .map(|index| {
            cargo_lock_value_span(
                before,
                &package_spans[root],
                root_dependencies
                    .get(*index)
                    .expect("cut dependency index came from the parsed graph"),
                "Cargo.lock root dependency removal",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    for index in &removed_indices {
        let span = package_spans
            .get(*index)
            .expect("removed index came from the parsed graph");
        spans.push(ByteSpan {
            start: u64::try_from(span.start)
                .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
            end: u64::try_from(span.end)
                .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
            node: format!(
                "Cargo.lock package `{}`",
                cargo_lock_identity(&before_graph.nodes[*index].id)
            ),
        });
    }
    spans.sort_by_key(|span| (span.start, span.end));
    for pair in spans.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(cargo_lock_blocked(
                "Cargo.lock graph evidence spans overlap",
            ));
        }
    }

    let packages = document
        .get_mut("package")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
        .expect("graph parser established mutable [[package]]");
    let root_table = packages
        .get_mut(root)
        .expect("root index came from the parsed graph");
    let dependencies_empty = {
        let dependencies = root_table
            .get_mut("dependencies")
            .and_then(toml_edit::Item::as_array_mut)
            .expect("cut edges establish a mutable root dependency array");
        for index in cut_root_edges.iter().rev() {
            dependencies.remove(*index);
        }
        dependencies.is_empty()
    };
    if dependencies_empty {
        root_table.remove("dependencies");
    }
    for index in removed_indices.iter().rev() {
        packages.remove(*index);
    }
    let after = document.to_string().into_bytes();
    let after_document = std::str::from_utf8(&after)
        .expect("toml_edit emits UTF-8")
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| cargo_lock_blocked(format!("rewritten Cargo.lock is invalid: {error}")))?;
    let after_graph = parse_cargo_lock_graph(&after_document)?;
    if after_graph.nodes.iter().any(|node| node.id.name == package) {
        return Err(cargo_lock_blocked(format!(
            "rewritten Cargo.lock still contains package `{package}`"
        )));
    }
    let nodes = removed
        .iter()
        .map(|identity| format!("cargo-lock:removed:{identity}"))
        .collect::<Vec<_>>();
    let matches = cut_root_edges.len() + removed_indices.len();
    Ok((
        (after, matches, nodes, spans),
        Some(NativeLockEvidence {
            manager: "cargo",
            before_graph: cargo_lock_graph_evidence(&before_graph),
            after_graph: cargo_lock_graph_evidence(&after_graph),
            removed,
        }),
    ))
}

pub(super) fn cargo_contains_identity(
    table: &toml_edit::Table,
    package: &str,
    aliases: &BTreeSet<String>,
) -> bool {
    fn dependencies_contain(
        table: Option<&toml_edit::Table>,
        package: &str,
        aliases: &BTreeSet<String>,
    ) -> bool {
        table.is_some_and(|table| {
            table.iter().any(|(key, item)| {
                key == package
                    || aliases.contains(key)
                    || item.get("package").and_then(toml_edit::Item::as_str) == Some(package)
            })
        })
    }

    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if dependencies_contain(
            table.get(name).and_then(toml_edit::Item::as_table),
            package,
            aliases,
        ) {
            return true;
        }
    }
    if dependencies_contain(
        table
            .get("workspace")
            .and_then(toml_edit::Item::as_table)
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml_edit::Item::as_table),
        package,
        aliases,
    ) {
        return true;
    }
    if let Some(targets) = table.get("target").and_then(toml_edit::Item::as_table) {
        for target in targets.iter().filter_map(|(_, item)| item.as_table()) {
            for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if dependencies_contain(
                    target.get(name).and_then(toml_edit::Item::as_table),
                    package,
                    aliases,
                ) {
                    return true;
                }
            }
        }
    }
    if table
        .get("patch")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|patches| {
            patches
                .iter()
                .filter_map(|(_, item)| item.as_table())
                .any(|registry| dependencies_contain(Some(registry), package, aliases))
        })
    {
        return true;
    }
    if table
        .get("replace")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|replace| {
            replace
                .iter()
                .any(|(key, _)| key.split_once(':').map_or(key, |(name, _)| name) == package)
        })
    {
        return true;
    }
    table
        .get("features")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|features| {
            features.iter().any(|(_, item)| {
                item.as_array().is_some_and(|array| {
                    array
                        .iter()
                        .filter_map(toml_edit::Value::as_str)
                        .any(|edge| {
                            aliases.iter().any(|alias| {
                                edge == alias
                                    || edge == format!("dep:{alias}")
                                    || edge.starts_with(&format!("{alias}/"))
                                    || edge.starts_with(&format!("{alias}?/"))
                            })
                        })
                })
            })
        })
}
