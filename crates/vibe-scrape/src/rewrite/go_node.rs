use super::javascript::{
    JsonParser, collect_nodes, json_member_edits, json_object_at, parse_tree, syntax_fingerprint,
};
use super::rust_lexer::containing_line;
use super::text::*;
use super::*;

fn valid_go_directive(line: &[u8]) -> bool {
    let Some(payload) = trim_ascii(line).strip_prefix(b"//spec:") else {
        return false;
    };
    let payload = trim_ascii(payload);
    valid_registered_reference(payload)
}

pub(super) fn prepare_go_directives(before: &[u8]) -> Result<RewriteOutput, ScrapeError> {
    let language: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();
    let before_tree = parse_tree(before, language.clone(), "Go")?;
    let mut comments = Vec::new();
    collect_nodes(before_tree.root_node(), "comment", &mut comments);
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for (comment_start, comment_end) in comments {
        let (start, end, whole_line) = containing_line(before, comment_start, comment_end);
        if whole_line && valid_go_directive(&before[comment_start..comment_end]) {
            edits.push(Edit {
                start,
                end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(start).map_err(|_| fail("Go directive span exceeds u64"))?,
                end: u64::try_from(end).map_err(|_| fail("Go directive span exceeds u64"))?,
                node: format!("go:comment-group-line@{comment_start}..{comment_end}"),
            });
            nodes.push(format!(
                "go:comment-group-line@{comment_start}..{comment_end}"
            ));
        }
    }
    let count = edits.len();
    let after = apply_edits(before, edits)?;
    let after_tree = parse_tree(&after, language, "rewritten Go")?;
    let mut erased_before = Vec::new();
    let mut parsed_after = Vec::new();
    syntax_fingerprint(before_tree.root_node(), before, &mut erased_before);
    syntax_fingerprint(after_tree.root_node(), &after, &mut parsed_after);
    if erased_before != parsed_after {
        return Err(fail(
            "Go registered-metadata erasure changed the parsed product tree",
        ));
    }
    Ok((after, count, nodes, spans))
}

pub(super) fn go_module_on_line(line: &str, module: &str, block: Option<&str>) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return false;
    }
    let words = trimmed.split_ascii_whitespace().collect::<Vec<_>>();
    match block {
        Some("require" | "exclude" | "tool") => words.first().copied() == Some(module),
        Some("replace") => words
            .split(|word| *word == "=>")
            .any(|side| side.first().copied() == Some(module)),
        _ => match words.as_slice() {
            [directive, name, ..] if matches!(*directive, "require" | "exclude" | "tool") => {
                *name == module
            }
            ["replace", rest @ ..] => rest
                .split(|word| *word == "=>")
                .any(|side| side.first().copied() == Some(module)),
            _ => false,
        },
    }
}

pub(super) fn prepare_go_mod(
    before: &[u8],
    modules: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    std::str::from_utf8(before).map_err(|_| fail("go.mod is not UTF-8"))?;
    let mut block: Option<&str> = None;
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    let lines = line_spans(before);
    let mut block_start = None;
    let mut block_rows = 0usize;
    let mut block_removed = 0usize;
    let mut pending_empty_blocks = Vec::new();
    for (start, content_end, end) in &lines {
        let line = std::str::from_utf8(&before[*start..*content_end]).expect("validated UTF-8");
        let trimmed = line.trim();
        if block.is_none() {
            for directive in ["require", "exclude", "replace", "tool"] {
                if trimmed == format!("{directive} (") {
                    block = Some(directive);
                    block_start = Some(*start);
                    block_rows = 0;
                    block_removed = 0;
                    break;
                }
            }
            if block.is_none() {
                for module in modules {
                    if go_module_on_line(line, module, None) {
                        edits.push(Edit {
                            start: *start,
                            end: *end,
                            replacement: Vec::new(),
                        });
                        spans.push(ByteSpan {
                            start: u64::try_from(*start)
                                .map_err(|_| fail("go.mod span exceeds u64"))?,
                            end: u64::try_from(*end)
                                .map_err(|_| fail("go.mod span exceeds u64"))?,
                            node: format!("go.mod:{module}@{start}"),
                        });
                        nodes.push(format!("go.mod:{module}@{start}"));
                    }
                }
            }
            continue;
        }
        if trimmed == ")" {
            if block_rows == block_removed {
                pending_empty_blocks.push((block_start.expect("open block"), *end));
            }
            block = None;
            block_start = None;
            continue;
        }
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            block_rows += 1;
            for module in modules {
                if go_module_on_line(line, module, block) {
                    edits.push(Edit {
                        start: *start,
                        end: *end,
                        replacement: Vec::new(),
                    });
                    spans.push(ByteSpan {
                        start: u64::try_from(*start)
                            .map_err(|_| fail("go.mod span exceeds u64"))?,
                        end: u64::try_from(*end).map_err(|_| fail("go.mod span exceeds u64"))?,
                        node: format!("go.mod:{module}@{start}"),
                    });
                    nodes.push(format!("go.mod:{module}@{start}"));
                    block_removed += 1;
                }
            }
        }
    }
    if block.is_some() {
        return Err(fail("unterminated go.mod directive block"));
    }
    for (start, end) in pending_empty_blocks {
        edits.retain(|edit| edit.end <= start || edit.start >= end);
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
    }
    // A line cannot validly name two requested module identities.  Catching it
    // above as two edits would otherwise look like an internal overlap.
    edits.sort_by_key(|edit| (edit.start, edit.end));
    edits.dedup_by_key(|edit| (edit.start, edit.end));
    let count = nodes.len();
    let after = apply_edits(before, edits)?;
    let text = std::str::from_utf8(&after).expect("validated UTF-8");
    if modules.iter().any(|module| {
        text.lines()
            .any(|line| go_module_on_line(line, module, None))
    }) {
        return Err(fail("requested Go module identity remains in go.mod"));
    }
    Ok((after, count, nodes, spans))
}

pub(super) fn prepare_go_sum(
    before: &[u8],
    modules: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    std::str::from_utf8(before).map_err(|_| fail("go.sum is not UTF-8"))?;
    for (start, content_end, _) in line_spans(before) {
        let line = std::str::from_utf8(&before[start..content_end]).expect("validated UTF-8");
        let Some(name) = line.split_ascii_whitespace().next() else {
            continue;
        };
        if modules
            .iter()
            .any(|module| name == module || name == format!("{module}/go.mod"))
        {
            return Err(ScrapeError::blocked(format!(
                "go.sum graph reconciliation for `{name}` requires the sealed manager-native Go resolver; syntax-only checksum deletion is forbidden"
            )));
        }
    }
    Ok((before.to_vec(), 0, Vec::new(), Vec::new()))
}

fn json_mentions_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_mentions_key(value, key))
        }
        serde_json::Value::Array(array) => array.iter().any(|value| json_mentions_key(value, key)),
        _ => false,
    }
}

pub(super) fn prepare_node_lock(
    before: &[u8],
    manager: NodeManager,
    packages: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    if manager != NodeManager::Npm {
        return Err(ScrapeError::blocked(format!(
            "selected {:?} lock reconciliation requires a sealed deterministic manager-native plan before execution",
            manager
        )));
    }
    let value: serde_json::Value = serde_json::from_slice(before)
        .map_err(|error| fail(format!("cannot parse npm lockfile: {error}")))?;
    for package in packages {
        if json_mentions_key(&value, package)
            || json_mentions_key(&value, &format!("node_modules/{package}"))
        {
            return Err(ScrapeError::blocked(format!(
                "npm lock graph reconciliation for `{package}` requires the sealed manager-native npm resolver; syntax-only lock deletion is forbidden"
            )));
        }
    }
    Ok((before.to_vec(), 0, Vec::new(), Vec::new()))
}

pub(super) fn prepare_node_manifest(
    before: &[u8],
    packages: &[String],
    script_paths: &[Vec<String>],
    config_paths: &[Vec<String>],
) -> Result<RewriteOutput, ScrapeError> {
    let root = JsonParser::parse(before)?;
    let mut targets = BTreeMap::<Vec<String>, BTreeSet<String>>::new();
    let mut count = 0;
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for table in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        for package in packages {
            targets
                .entry(vec![table.to_owned()])
                .or_default()
                .insert(package.clone());
        }
    }
    for path in script_paths.iter().chain(config_paths) {
        let (member, object) = path
            .split_last()
            .ok_or_else(|| fail("Node script/config member path is empty"))?;
        targets
            .entry(object.to_vec())
            .or_default()
            .insert(member.clone());
    }
    let mut edits = Vec::new();
    for (object, members) in targets {
        let members = members.into_iter().collect::<Vec<_>>();
        match json_object_at(&root, &object) {
            Ok(_) => {
                let (mut local_edits, local_count, mut local_nodes, mut local_spans) =
                    json_member_edits(&root, &object, &members)?;
                count += local_count;
                edits.append(&mut local_edits);
                nodes.append(&mut local_nodes);
                spans.append(&mut local_spans);
            }
            Err(_) if object.len() == 1 => {}
            Err(error) => return Err(error),
        }
    }
    let after = apply_edits(before, edits)?;
    JsonParser::parse(&after)?;
    Ok((after, count, nodes, spans))
}

pub(super) fn virtual_bytes(
    project: &Project,
    path: &str,
    inventory: &BTreeMap<String, &InventoryEntry>,
    current: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, ScrapeError> {
    if let Some(bytes) = current.get(path) {
        return Ok(bytes.clone());
    }
    let entry = inventory
        .get(path)
        .ok_or_else(|| fail(format!("rewrite target `{path}` is absent from inventory")))?;
    read_candidate(project, entry)
}

pub(super) fn candidate(
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    _nodes: Vec<String>,
    spans: Vec<ByteSpan>,
) -> Candidate {
    Candidate {
        path,
        before,
        after,
        matches,
        spans,
        native_lock_evidence: None,
    }
}

pub(super) fn cargo_lock_candidate(
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    spans: Vec<ByteSpan>,
    evidence: NativeLockEvidence,
) -> Candidate {
    Candidate {
        path,
        before,
        after,
        matches,
        spans,
        native_lock_evidence: Some(evidence),
    }
}

pub(super) fn prepare_record(id: &str, kind: &str, candidate: Candidate) -> PreparedRewrite {
    let before_sha256 = digest(&candidate.before);
    let after_sha256 = digest(&candidate.after);
    let native_lock_change = candidate
        .native_lock_evidence
        .map(|evidence| NativeLockChange {
            manager: evidence.manager.to_owned(),
            path: candidate.path.clone(),
            before_sha256: before_sha256.clone(),
            after_sha256: after_sha256.clone(),
            before_graph: evidence.before_graph,
            after_graph: evidence.after_graph,
            removed: evidence.removed,
            authorizing_rewrite_id: id.to_owned(),
        });
    PreparedRewrite {
        id: id.to_owned(),
        kind: kind.to_owned(),
        path: candidate.path,
        adapter_epoch: 1,
        spans: candidate.spans,
        before_sha256,
        before_bytes: candidate.before.len() as u64,
        after_bytes: candidate.after,
        after_sha256,
        matches: candidate.matches as u64,
        reason: format!("schema-1 `{kind}` registered metadata/dependency removal"),
        native_lock_change,
    }
}

pub(super) fn validate_relocations(
    contract: &Contract,
    inventory: &[InventoryEntry],
) -> Result<(), ScrapeError> {
    let paths = inventory
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    for relocation in &contract.relocate {
        let source_exists = paths.contains(relocation.from.as_str())
            || paths
                .iter()
                .any(|path| path.starts_with(&(relocation.from.clone() + "/")));
        if relocation.required && !source_exists {
            return Err(fail(format!(
                "required relocation `{}` source `{}` is absent",
                relocation.id, relocation.from
            )));
        }
        if !source_exists {
            continue;
        }
        if paths.contains(relocation.to.as_str())
            || paths
                .iter()
                .any(|path| path.starts_with(&(relocation.to.clone() + "/")))
        {
            return Err(fail(format!(
                "relocation `{}` destination `{}` already exists",
                relocation.id, relocation.to
            )));
        }
        let mut ancestor = relocation.to.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if inventory
                .iter()
                .any(|entry| entry.path == parent && entry.kind == EntryKind::File)
            {
                return Err(fail(format!(
                    "relocation `{}` destination `{}` has file `{parent}` as an ancestor",
                    relocation.id, relocation.to
                )));
            }
            ancestor = parent;
        }
        if relocation.from == ".git"
            || relocation.from.starts_with(".git/")
            || relocation.to == ".git"
            || relocation.to.starts_with(".git/")
        {
            return Err(fail(format!(
                "relocation `{}` addresses protected .git metadata",
                relocation.id
            )));
        }
        for source_path in paths.iter().filter(|path| {
            *path == &relocation.from.as_str() || path.starts_with(&(relocation.from.clone() + "/"))
        }) {
            let suffix = source_path
                .strip_prefix(&relocation.from)
                .expect("selected relocation descendant has source prefix");
            let mapped = format!("{}{}", relocation.to, suffix);
            if paths.contains(mapped.as_str()) {
                return Err(fail(format!(
                    "relocation `{}` mapped descendant `{mapped}` already exists",
                    relocation.id
                )));
            }
            let mut mapped_ancestor = mapped.as_str();
            while let Some((parent, _)) = mapped_ancestor.rsplit_once('/') {
                if inventory
                    .iter()
                    .any(|entry| entry.path == parent && entry.kind == EntryKind::File)
                {
                    return Err(fail(format!(
                        "relocation `{}` mapped descendant `{mapped}` has file `{parent}` as an ancestor",
                        relocation.id
                    )));
                }
                mapped_ancestor = parent;
            }
            let mut explicitly_kept = false;
            let mut selected_for_deletion = false;
            for rule in &contract.classify {
                if !any_pattern(rule.patterns(), source_path)? {
                    continue;
                }
                explicitly_kept |= matches!(rule, crate::contract::ClassifyRule::Keep { .. });
                selected_for_deletion |= matches!(
                    rule,
                    crate::contract::ClassifyRule::Delete { .. }
                        | crate::contract::ClassifyRule::Generated { .. }
                );
            }
            if selected_for_deletion && !explicitly_kept {
                return Err(fail(format!(
                    "relocation `{}` source member `{source_path}` is not effectively kept",
                    relocation.id
                )));
            }
        }
        for rule in &contract.classify {
            let deleting = matches!(
                rule,
                crate::contract::ClassifyRule::Delete { .. }
                    | crate::contract::ClassifyRule::Generated { .. }
            );
            if deleting
                && (any_pattern(rule.patterns(), &relocation.to)?
                    || any_pattern(rule.patterns(), &(relocation.to.clone() + "/probe"))?)
            {
                return Err(fail(format!(
                    "relocation `{}` destination `{}` remains inside a deletion selector",
                    relocation.id, relocation.to
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn cargo_path_prefix_present(table: &toml_edit::Table, prefixes: &[String]) -> bool {
    for (key, item) in table.iter() {
        if toml_item_path_prefix_present(Some(key), item, prefixes) {
            return true;
        }
    }
    false
}

fn toml_item_path_prefix_present(
    key: Option<&str>,
    item: &toml_edit::Item,
    prefixes: &[String],
) -> bool {
    if key == Some("path")
        && item
            .as_str()
            .is_some_and(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
    {
        return true;
    }
    if let Some(table) = item.as_table() {
        return cargo_path_prefix_present(table, prefixes);
    }
    if let Some(array) = item.as_array_of_tables() {
        return array
            .iter()
            .any(|table| cargo_path_prefix_present(table, prefixes));
    }
    item.as_value()
        .is_some_and(|value| toml_value_path_prefix_present(key, value, prefixes))
}

fn toml_value_path_prefix_present(
    key: Option<&str>,
    value: &toml_edit::Value,
    prefixes: &[String],
) -> bool {
    if key == Some("path")
        && value
            .as_str()
            .is_some_and(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
    {
        return true;
    }
    if let Some(table) = value.as_inline_table() {
        return table.iter().any(|(child_key, child)| {
            toml_value_path_prefix_present(Some(child_key), child, prefixes)
        });
    }
    value.as_array().is_some_and(|array| {
        array
            .iter()
            .any(|child| toml_value_path_prefix_present(None, child, prefixes))
    })
}

pub(super) fn assert_language_absent(
    bytes: &[u8],
    language: Language,
    rust_aliases: &BTreeSet<String>,
    tsx: bool,
) -> Result<bool, ScrapeError> {
    match language {
        Language::TypeScript => Ok(prepare_typescript(bytes, tsx)?.1 == 0),
        Language::Go => Ok(prepare_go_directives(bytes)?.1 == 0),
        Language::Rust => {
            if rust_aliases.is_empty() {
                return Ok(true);
            }
            let forms = ["scope", "spec", "verifies", "cell"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            Ok(prepare_rust(bytes, rust_aliases, &forms)?.1 == 0)
        }
    }
}
