use super::rust_ast::rust_span_offsets;
use super::rust_lexer::*;
use super::text::*;
use super::*;

pub(super) fn rust_import_edits_syn(
    file: &syn::File,
    text: &str,
    bytes: &[u8],
    crate_aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RustImportOutput, ScrapeError> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;

    #[derive(Default)]
    struct Uses(Vec<syn::ItemUse>);
    impl<'ast> syn::visit::Visit<'ast> for Uses {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            self.0.push(node.clone());
        }
    }

    let mut all_uses = Uses::default();
    all_uses.visit_file(file);
    let uses = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item) => Some(item.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let top_level_spans = uses
        .iter()
        .filter_map(|item| rust_span_offsets(text, item.span()))
        .collect::<BTreeSet<_>>();
    for item in &all_uses.0 {
        let relevant = match &item.tree {
            syn::UseTree::Path(path) => crate_aliases.contains(&path.ident.to_string()),
            syn::UseTree::Name(name) => crate_aliases.contains(&name.ident.to_string()),
            syn::UseTree::Rename(rename) => crate_aliases.contains(&rename.ident.to_string()),
            _ => false,
        };
        if relevant {
            let span = rust_span_offsets(text, item.span())
                .ok_or_else(|| fail("syn ItemUse has no stable source span"))?;
            if !top_level_spans.contains(&span) {
                return Err(fail(
                    "non-top-level Specmark import has lexical scope the schema-1 adapter does not resolve",
                ));
            }
        }
    }
    let mut edits = Vec::new();
    let mut imported_forms = BTreeSet::new();
    let mut qualified_aliases = crate_aliases.clone();
    for item in uses {
        let inherited = matches!(item.vis, syn::Visibility::Inherited);
        let (root, tail) = match &item.tree {
            syn::UseTree::Path(path) if crate_aliases.contains(&path.ident.to_string()) => {
                (path.ident.to_string(), Some(path.tree.as_ref()))
            }
            syn::UseTree::Name(name) if crate_aliases.contains(&name.ident.to_string()) => {
                (name.ident.to_string(), None)
            }
            syn::UseTree::Rename(rename) if crate_aliases.contains(&rename.ident.to_string()) => {
                if inherited {
                    qualified_aliases.insert(rename.rename.to_string());
                }
                (rename.ident.to_string(), None)
            }
            _ => continue,
        };
        if !inherited {
            return Err(fail(format!(
                "Rust macro re-export from Specmark alias `{root}` is not erasable"
            )));
        }
        let (item_start, item_end) = rust_span_offsets(text, item.span())
            .ok_or_else(|| fail("syn ItemUse has no stable source span"))?;
        let whole_edit = || {
            let (line_start, line_end, whole) = containing_line(bytes, item_start, item_end);
            Edit {
                start: if whole { line_start } else { item_start },
                end: if whole { line_end } else { item_end },
                replacement: Vec::new(),
            }
        };
        let Some(tail) = tail else {
            qualified_aliases.insert(root);
            edits.push(whole_edit());
            continue;
        };
        match tail {
            syn::UseTree::Name(name) if forms.contains(&name.ident.to_string()) => {
                imported_forms.insert(name.ident.to_string());
                edits.push(whole_edit());
            }
            syn::UseTree::Rename(rename) if forms.contains(&rename.ident.to_string()) => {
                imported_forms.insert(rename.rename.to_string());
                edits.push(whole_edit());
            }
            syn::UseTree::Glob(_) => {
                return Err(fail(format!(
                    "glob import from Specmark alias `{root}` is ambiguous"
                )));
            }
            syn::UseTree::Group(group) => {
                let mut leaves = Vec::new();
                let mut selected = Vec::new();
                for (index, leaf) in group.items.iter().enumerate() {
                    let (start, end) = rust_span_offsets(text, leaf.span())
                        .ok_or_else(|| fail("syn grouped import leaf has no source span"))?;
                    leaves.push((start, end));
                    match leaf {
                        syn::UseTree::Name(name) if name.ident == "self" => {
                            qualified_aliases.insert(root.clone());
                            selected.push(index);
                        }
                        syn::UseTree::Rename(rename) if rename.ident == "self" => {
                            qualified_aliases.insert(rename.rename.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Name(name) if forms.contains(&name.ident.to_string()) => {
                            imported_forms.insert(name.ident.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Rename(rename)
                            if forms.contains(&rename.ident.to_string()) =>
                        {
                            imported_forms.insert(rename.rename.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Glob(_) => {
                            return Err(fail(format!(
                                "glob import from Specmark alias `{root}` is ambiguous"
                            )));
                        }
                        syn::UseTree::Path(_) | syn::UseTree::Group(_) => {
                            return Err(fail(format!(
                                "nested grouped import from Specmark alias `{root}` is ambiguous"
                            )));
                        }
                        _ => {}
                    }
                }
                if selected.is_empty() {
                    continue;
                }
                if selected.len() == leaves.len() {
                    edits.push(whole_edit());
                    continue;
                }
                let selected = selected.into_iter().collect::<BTreeSet<_>>();
                let mut index = 0;
                while index < leaves.len() {
                    if !selected.contains(&index) {
                        index += 1;
                        continue;
                    }
                    let first = index;
                    let mut last = index;
                    while last + 1 < leaves.len() && selected.contains(&(last + 1)) {
                        last += 1;
                    }
                    let (start, end) = if last + 1 < leaves.len() {
                        let gap = &bytes[leaves[last].1..leaves[last + 1].0];
                        let comma = gap.iter().position(|byte| *byte == b',').ok_or_else(|| {
                            fail("grouped Specmark import leaf has no following comma")
                        })?;
                        if gap[..comma].iter().any(|byte| !byte.is_ascii_whitespace()) {
                            return Err(fail(
                                "grouped Specmark import trivia before comma is not safely owned",
                            ));
                        }
                        (leaves[first].0, leaves[last].1 + comma + 1)
                    } else {
                        let gap = &bytes[leaves[first - 1].1..leaves[first].0];
                        let comma =
                            gap.iter().rposition(|byte| *byte == b',').ok_or_else(|| {
                                fail("grouped Specmark import leaf has no preceding comma")
                            })?;
                        if gap[comma + 1..]
                            .iter()
                            .any(|byte| !byte.is_ascii_whitespace())
                        {
                            return Err(fail(
                                "grouped Specmark import trivia after comma is not safely owned",
                            ));
                        }
                        (leaves[first - 1].1 + comma, leaves[last].1)
                    };
                    if bytes[start..end]
                        .windows(2)
                        .any(|window| window == b"//" || window == b"/*")
                    {
                        return Err(fail(
                            "grouped Specmark import carries comment trivia; exact ownership is ambiguous",
                        ));
                    }
                    edits.push(Edit {
                        start,
                        end,
                        replacement: Vec::new(),
                    });
                    index = last + 1;
                }
            }
            syn::UseTree::Path(_) => {
                return Err(fail(format!(
                    "nested Specmark import from alias `{root}` is ambiguous"
                )));
            }
            _ => {}
        }
    }
    Ok((edits, imported_forms, qualified_aliases))
}

#[cfg(any())]
fn rust_import_edits(
    bytes: &[u8],
    tokens: &[RustToken],
    crate_aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RustImportOutput, ScrapeError> {
    let mut edits = Vec::new();
    let mut imported_forms = BTreeSet::new();
    let mut qualified_aliases = crate_aliases.clone();
    let mut index = 0;
    while index < tokens.len() {
        let is_pub = token_ident(tokens.get(index)) == Some("pub")
            && token_ident(tokens.get(index + 1)) == Some("use");
        let use_at = if is_pub { index + 1 } else { index };
        if token_ident(tokens.get(use_at)) != Some("use") {
            index += 1;
            continue;
        }
        let Some(root) = token_ident(tokens.get(use_at + 1)) else {
            index += 1;
            continue;
        };
        if !crate_aliases.contains(root) {
            index += 1;
            continue;
        }
        if is_pub {
            return Err(fail(format!(
                "Rust macro re-export from Specmark alias `{root}` is not erasable"
            )));
        }
        let semi = (use_at + 2..tokens.len())
            .find(|at| token_punct(tokens.get(*at), b';'))
            .ok_or_else(|| fail("unterminated Rust use item"))?;
        if (use_at + 2..semi).any(|at| token_punct(tokens.get(at), b'*')) {
            return Err(fail(format!(
                "glob import from Specmark alias `{root}` is ambiguous"
            )));
        }
        let start = tokens[index].start;
        let end = tokens[semi].end;
        let (line_start, line_end, whole_line) = containing_line(bytes, start, end);
        let removal = |replacement: Vec<u8>| {
            if replacement.is_empty() && whole_line {
                Edit {
                    start: line_start,
                    end: line_end,
                    replacement,
                }
            } else {
                Edit {
                    start,
                    end,
                    replacement,
                }
            }
        };

        // `use specmark as sm;`
        if token_ident(tokens.get(use_at + 2)) == Some("as") {
            let local = token_ident(tokens.get(use_at + 3))
                .ok_or_else(|| fail("malformed renamed Specmark import"))?;
            qualified_aliases.insert(local.to_owned());
            edits.push(removal(Vec::new()));
            index = semi + 1;
            continue;
        }
        if !token_punct(tokens.get(use_at + 2), b':') || !token_punct(tokens.get(use_at + 3), b':')
        {
            return Err(fail("unsupported Specmark import form"));
        }
        if let Some(form) = token_ident(tokens.get(use_at + 4)) {
            if forms.contains(form) {
                let local = if token_ident(tokens.get(use_at + 5)) == Some("as") {
                    token_ident(tokens.get(use_at + 6)).unwrap_or(form)
                } else {
                    form
                };
                imported_forms.insert(local.to_owned());
                edits.push(removal(Vec::new()));
            }
            index = semi + 1;
            continue;
        }
        if !token_punct(tokens.get(use_at + 4), b'{') {
            index = semi + 1;
            continue;
        }
        let close = matching_delimiter(tokens, use_at + 4)
            .ok_or_else(|| fail("malformed grouped Specmark import"))?;
        let mut remove_ranges = Vec::new();
        let mut cursor = use_at + 5;
        while cursor < close {
            while cursor < close && token_punct(tokens.get(cursor), b',') {
                cursor += 1;
            }
            if cursor >= close {
                break;
            }
            let leaf_start = cursor;
            while cursor < close && !token_punct(tokens.get(cursor), b',') {
                cursor += 1;
            }
            let leaf_end = cursor;
            let Some(form) = token_ident(tokens.get(leaf_start)) else {
                return Err(fail(
                    "nested or malformed grouped Specmark import is ambiguous",
                ));
            };
            if form == "self" {
                let local = if token_ident(tokens.get(leaf_start + 1)) == Some("as") {
                    token_ident(tokens.get(leaf_start + 2)).unwrap_or(root)
                } else {
                    root
                };
                qualified_aliases.insert(local.to_owned());
                remove_ranges.push((leaf_start, leaf_end));
            } else if forms.contains(form) {
                let local = if token_ident(tokens.get(leaf_start + 1)) == Some("as") {
                    token_ident(tokens.get(leaf_start + 2)).unwrap_or(form)
                } else {
                    form
                };
                imported_forms.insert(local.to_owned());
                remove_ranges.push((leaf_start, leaf_end));
            }
        }
        if !remove_ranges.is_empty() {
            let retained = (use_at + 5..close)
                .filter(|at| {
                    !remove_ranges
                        .iter()
                        .any(|(start, end)| at >= start && at < end)
                        && !token_punct(tokens.get(*at), b',')
                })
                .count();
            if retained == 0 {
                edits.push(removal(Vec::new()));
            } else {
                let mut pieces = Vec::new();
                let mut cursor = use_at + 5;
                while cursor < close {
                    while cursor < close && token_punct(tokens.get(cursor), b',') {
                        cursor += 1;
                    }
                    if cursor >= close {
                        break;
                    }
                    let leaf_start = cursor;
                    while cursor < close && !token_punct(tokens.get(cursor), b',') {
                        cursor += 1;
                    }
                    let leaf_end = cursor;
                    if !remove_ranges.contains(&(leaf_start, leaf_end)) {
                        pieces.push(
                            trim_ascii(&bytes[tokens[leaf_start].start..tokens[leaf_end - 1].end])
                                .to_vec(),
                        );
                    }
                }
                let mut replacement = Vec::new();
                replacement.extend_from_slice(&bytes[start..tokens[use_at + 4].end]);
                for (piece_index, piece) in pieces.iter().enumerate() {
                    if piece_index > 0 {
                        replacement.extend_from_slice(b", ");
                    }
                    replacement.extend_from_slice(piece);
                }
                replacement.extend_from_slice(&bytes[tokens[close].start..end]);
                edits.push(removal(replacement));
            }
        }
        index = semi + 1;
    }
    Ok((edits, imported_forms, qualified_aliases))
}
