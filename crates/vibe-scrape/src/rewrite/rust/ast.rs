use super::rust_imports::*;
use super::rust_lexer::*;
use super::text::*;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustAstNode {
    form: String,
    start: usize,
    end: usize,
    attribute: bool,
    top_level: bool,
}

pub(super) fn rust_span_offsets(text: &str, span: proc_macro2::Span) -> Option<(usize, usize)> {
    fn at(text: &str, location: proc_macro2::LineColumn) -> Option<usize> {
        if location.line == 0 {
            return None;
        }
        let mut offset = 0usize;
        for line in text.split_inclusive('\n').take(location.line - 1) {
            offset = offset.checked_add(line.len())?;
        }
        offset
            .checked_add(location.column)
            .filter(|offset| *offset <= text.len())
    }
    Some((at(text, span.start())?, at(text, span.end())?))
}

fn registered_rust_path(
    path: &syn::Path,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Option<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [form] if imported_forms.contains(form) && forms.contains(form) => Some(form.clone()),
        [root, form] if qualified_aliases.contains(root) && forms.contains(form) => {
            Some(form.clone())
        }
        _ => None,
    }
}

fn rust_ast_nodes(
    file: &syn::File,
    text: &str,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Vec<RustAstNode> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;
    struct Collector<'a> {
        text: &'a str,
        qualified_aliases: &'a BTreeSet<String>,
        imported_forms: &'a BTreeSet<String>,
        forms: &'a BTreeSet<String>,
        nodes: Vec<RustAstNode>,
        depth: usize,
    }
    impl<'ast> syn::visit::Visit<'ast> for Collector<'_> {
        fn visit_item(&mut self, node: &'ast syn::Item) {
            if let syn::Item::Mod(module) = node
                && let Some((_, items)) = &module.content
            {
                for attribute in &module.attrs {
                    self.visit_attribute(attribute);
                }
                self.depth += 1;
                for item in items {
                    self.visit_item(item);
                }
                self.depth -= 1;
                return;
            }
            syn::visit::visit_item(self, node);
        }

        fn visit_block(&mut self, node: &'ast syn::Block) {
            self.depth += 1;
            syn::visit::visit_block(self, node);
            self.depth -= 1;
        }

        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if let Some(form) = registered_rust_path(
                &node.path,
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            ) && let Some((start, end)) = rust_span_offsets(self.text, node.span())
            {
                self.nodes.push(RustAstNode {
                    form,
                    start,
                    end,
                    attribute: false,
                    top_level: self.depth == 0,
                });
            }
            syn::visit::visit_macro(self, node);
        }

        fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
            if let Some(form) = registered_rust_path(
                node.path(),
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            ) && let Some((start, end)) = rust_span_offsets(self.text, node.span())
            {
                self.nodes.push(RustAstNode {
                    form,
                    start,
                    end,
                    attribute: true,
                    top_level: self.depth == 0,
                });
            }
            syn::visit::visit_attribute(self, node);
        }
    }
    let mut collector = Collector {
        text,
        qualified_aliases,
        imported_forms,
        forms,
        nodes: Vec::new(),
        depth: 0,
    };
    collector.visit_file(file);
    collector.nodes
}

fn opaque_rust_macro_ranges(
    file: &syn::File,
    text: &str,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Vec<(usize, usize)> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;
    struct Collector<'a> {
        text: &'a str,
        qualified_aliases: &'a BTreeSet<String>,
        imported_forms: &'a BTreeSet<String>,
        forms: &'a BTreeSet<String>,
        ranges: Vec<(usize, usize)>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Collector<'_> {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if registered_rust_path(
                &node.path,
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            )
            .is_none()
                && let Some(range) = rust_span_offsets(self.text, node.tokens.span())
            {
                self.ranges.push(range);
            }
            // Macro token trees are opaque syntax. Deliberately do not recurse.
        }
    }
    let mut collector = Collector {
        text,
        qualified_aliases,
        imported_forms,
        forms,
        ranges: Vec::new(),
    };
    collector.visit_file(file);
    collector.ranges
}

fn rust_erasure_oracle(
    file: &syn::File,
    text: &str,
    bytes: &[u8],
    aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<syn::File, ScrapeError> {
    let (mut edits, imported_forms, qualified_aliases) =
        rust_import_edits_syn(file, text, bytes, aliases, forms)?;
    let nodes = rust_ast_nodes(file, text, &qualified_aliases, &imported_forms, forms);
    for node in nodes {
        if !node.top_level {
            return Err(fail(format!(
                "non-top-level Specmark `{}` metadata is outside the erasure oracle's lexical scope",
                node.form
            )));
        }
        let (start, end) = if node.attribute {
            (node.start, node.end)
        } else {
            let mut syntactic_end = node.end;
            while bytes
                .get(syntactic_end)
                .is_some_and(|byte| matches!(*byte, b' ' | b'\t'))
            {
                syntactic_end += 1;
            }
            if bytes.get(syntactic_end) == Some(&b';') {
                syntactic_end += 1;
            }
            let (line_start, line_end, whole) = containing_line(bytes, node.start, syntactic_end);
            if !whole {
                return Err(fail(format!(
                    "Specmark `{}` macro is not a complete metadata-only statement in the erasure oracle",
                    node.form
                )));
            }
            (line_start, line_end)
        };
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
    }
    let erased = apply_edits(bytes, edits)?;
    let erased = std::str::from_utf8(&erased).expect("Rust source was validated UTF-8");
    syn::parse_file(erased).map_err(|error| {
        fail(format!(
            "independent Rust registered-metadata erasure does not parse: {error}"
        ))
    })
}

pub(super) fn prepare_rust(
    before: &[u8],
    aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RewriteOutput, ScrapeError> {
    let text = std::str::from_utf8(before).map_err(|_| fail("Rust source is not UTF-8"))?;
    let parsed_before = syn::parse_file(text).map_err(|error| {
        fail(format!(
            "Rust source does not parse before erasure: {error}"
        ))
    })?;
    let tokens = rust_tokens(before)?;
    let (mut edits, imported_forms, qualified_aliases) =
        rust_import_edits_syn(&parsed_before, text, before, aliases, forms)?;
    let ast_nodes = rust_ast_nodes(
        &parsed_before,
        text,
        &qualified_aliases,
        &imported_forms,
        forms,
    );
    if let Some(node) = ast_nodes.iter().find(|node| !node.top_level) {
        return Err(fail(format!(
            "non-top-level Specmark `{}` metadata has lexical scope the schema-1 adapter does not resolve",
            node.form
        )));
    }
    let opaque_macros = opaque_rust_macro_ranges(
        &parsed_before,
        text,
        &qualified_aliases,
        &imported_forms,
        forms,
    );
    let import_spans = edits
        .iter()
        .map(|edit| (edit.start, edit.end))
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let Some(name) = token_ident(Some(token)) else {
            continue;
        };
        if qualified_aliases.contains(name)
            && !import_spans
                .iter()
                .any(|(start, end)| token.start >= *start && token.end <= *end)
            && !(token_punct(tokens.get(index + 1), b':')
                && token_punct(tokens.get(index + 2), b':'))
        {
            return Err(fail(format!(
                "Rust Specmark alias `{name}` is shadowed or used as a value"
            )));
        }
    }
    let mut nodes = edits
        .iter()
        .map(|edit| format!("rust:syn-item-use@{}..{}", edit.start, edit.end))
        .collect::<Vec<_>>();
    let mut spans = edits
        .iter()
        .map(|edit| {
            Ok(ByteSpan {
                start: u64::try_from(edit.start)
                    .map_err(|_| fail("Rust import span exceeds u64"))?,
                end: u64::try_from(edit.end).map_err(|_| fail("Rust import span exceeds u64"))?,
                node: format!("rust:syn-item-use@{}..{}", edit.start, edit.end),
            })
        })
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let mut count = 0;
    let mut index = 0;
    while index < tokens.len() {
        if opaque_macros
            .iter()
            .any(|(start, end)| tokens[index].start >= *start && tokens[index].end <= *end)
        {
            index += 1;
            continue;
        }
        if import_spans
            .iter()
            .any(|(start, end)| tokens[index].start >= *start && tokens[index].end <= *end)
        {
            index += 1;
            continue;
        }
        let mut form = None;
        let mut bang_or_attr = None;
        if let Some(root) = token_ident(tokens.get(index)) {
            if qualified_aliases.contains(root)
                && token_punct(tokens.get(index + 1), b':')
                && token_punct(tokens.get(index + 2), b':')
            {
                if let Some(candidate) = token_ident(tokens.get(index + 3))
                    && forms.contains(candidate)
                {
                    form = Some(candidate.to_owned());
                    bang_or_attr = Some(index + 4);
                }
            } else if imported_forms.contains(root) {
                form = Some(root.to_owned());
                bang_or_attr = Some(index + 1);
            }
        }
        let Some(form) = form else {
            index += 1;
            continue;
        };
        let marker_at = bang_or_attr.expect("form has suffix");
        if token_punct(tokens.get(marker_at), b'!') {
            let open_at = marker_at + 1;
            let close_at = matching_delimiter(&tokens, open_at)
                .ok_or_else(|| fail(format!("invalid Specmark `{form}!` macro grammar")))?;
            let arguments = &text[tokens[open_at].end..tokens[close_at].start];
            validate_rust_arguments(&form, arguments, true)?;
            if !ast_nodes.iter().any(|node| {
                !node.attribute
                    && node.form == form
                    && node.start == tokens[index].start
                    && node.end >= tokens[close_at].end
            }) {
                return Err(fail(format!(
                    "Specmark `{form}!` candidate is not a matching syn macro node"
                )));
            }
            let mut end = tokens[close_at].end;
            if token_punct(tokens.get(close_at + 1), b';') {
                end = tokens[close_at + 1].end;
            }
            let (line_start, line_end, whole_line) =
                containing_line(before, tokens[index].start, end);
            if !whole_line {
                return Err(fail(format!(
                    "Specmark `{form}!` is not a complete metadata-only statement"
                )));
            }
            edits.push(Edit {
                start: line_start,
                end: line_end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(line_start).map_err(|_| fail("Rust span exceeds u64"))?,
                end: u64::try_from(line_end).map_err(|_| fail("Rust span exceeds u64"))?,
                node: format!("rust-macro:{form}@{}", tokens[index].start),
            });
            nodes.push(format!("rust-macro:{form}@{}", tokens[index].start));
            count += 1;
            index = close_at + 1;
            continue;
        }
        // Attributes begin with `#[`; the token currently found is their path.
        if index >= 2
            && token_punct(tokens.get(index - 2), b'#')
            && token_punct(tokens.get(index - 1), b'[')
        {
            let close_at = matching_delimiter(&tokens, index - 1)
                .ok_or_else(|| fail(format!("invalid Specmark `{form}` attribute grammar")))?;
            let start = tokens[index - 2].start;
            let end = tokens[close_at].end;
            if !ast_nodes.iter().any(|node| {
                node.attribute && node.form == form && node.start == start && node.end == end
            }) {
                return Err(fail(format!(
                    "Specmark `{form}` candidate is not a matching syn attribute node"
                )));
            }
            let arguments = if token_punct(tokens.get(marker_at), b'(') {
                let argument_close = matching_delimiter(&tokens, marker_at)
                    .ok_or_else(|| fail(format!("invalid Specmark `{form}` attribute grammar")))?;
                &text[tokens[marker_at].end..tokens[argument_close].start]
            } else {
                ""
            };
            validate_rust_arguments(&form, arguments, false)?;
            edits.push(Edit {
                start,
                end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(start).map_err(|_| fail("Rust span exceeds u64"))?,
                end: u64::try_from(end).map_err(|_| fail("Rust span exceeds u64"))?,
                node: format!("rust-attribute:{form}@{start}"),
            });
            nodes.push(format!("rust-attribute:{form}@{start}"));
            count += 1;
            index = close_at + 1;
            continue;
        }
        index += 1;
    }
    let after = apply_edits(before, edits)?;
    let after_text = std::str::from_utf8(&after).expect("validated UTF-8");
    let parsed_after = syn::parse_file(after_text).map_err(|error| {
        fail(format!(
            "Rust source does not parse after metadata erasure: {error}"
        ))
    })?;
    let erased_before_ast = rust_erasure_oracle(&parsed_before, text, before, aliases, forms)?;
    use quote::ToTokens as _;
    if erased_before_ast.into_token_stream().to_string()
        != parsed_after.into_token_stream().to_string()
    {
        return Err(fail(
            "Rust erase_registered_metadata(parse(before)) differs from parse(after)",
        ));
    }
    let after_tokens = rust_tokens(&after)?;
    for (index, token) in after_tokens.iter().enumerate() {
        let Some(name) = token_ident(Some(token)) else {
            continue;
        };
        if qualified_aliases.contains(name)
            && token_punct(after_tokens.get(index + 1), b':')
            && token_punct(after_tokens.get(index + 2), b':')
        {
            return Err(fail(format!(
                "unresolved Specmark alias reference `{name}::...` remains after erasure"
            )));
        }
        if imported_forms.contains(name)
            && (token_punct(after_tokens.get(index + 1), b'!')
                || (index >= 2
                    && token_punct(after_tokens.get(index - 2), b'#')
                    && token_punct(after_tokens.get(index - 1), b'[')))
        {
            return Err(fail(format!(
                "unresolved imported Specmark form `{name}` remains after erasure"
            )));
        }
    }
    Ok((after, count, nodes, spans))
}
