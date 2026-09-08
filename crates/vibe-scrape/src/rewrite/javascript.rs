use super::text::*;
use super::*;

#[derive(Debug, Clone)]
enum JsonKind {
    Object(Vec<JsonMember>),
    Array,
    Scalar,
}

#[derive(Debug, Clone)]
pub(super) struct JsonNode {
    end: usize,
    kind: JsonKind,
}

#[derive(Debug, Clone)]
pub(super) struct JsonMember {
    key: String,
    key_start: usize,
    value: JsonNode,
    comma_before: Option<usize>,
    comma_after: Option<usize>,
}

pub(super) struct JsonParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    pub(super) fn parse(bytes: &'a [u8]) -> Result<JsonNode, ScrapeError> {
        serde_json::from_slice::<serde_json::Value>(bytes)
            .map_err(|error| fail(format!("invalid JSON rewrite target: {error}")))?;
        let mut parser = Self { bytes, cursor: 0 };
        parser.ws();
        let node = parser.value()?;
        parser.ws();
        if parser.cursor != bytes.len() {
            return Err(fail("JSON contains trailing non-whitespace bytes"));
        }
        Ok(node)
    }

    fn ws(&mut self) {
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.cursor += 1;
        }
    }

    fn value(&mut self) -> Result<JsonNode, ScrapeError> {
        self.ws();
        let kind = match self.bytes.get(self.cursor).copied() {
            Some(b'{') => self.object()?,
            Some(b'[') => self.array()?,
            Some(b'"') => {
                self.string()?;
                JsonKind::Scalar
            }
            Some(b't') => {
                self.literal(b"true")?;
                JsonKind::Scalar
            }
            Some(b'f') => {
                self.literal(b"false")?;
                JsonKind::Scalar
            }
            Some(b'n') => {
                self.literal(b"null")?;
                JsonKind::Scalar
            }
            Some(b'-' | b'0'..=b'9') => {
                self.number()?;
                JsonKind::Scalar
            }
            _ => return Err(fail("invalid JSON value")),
        };
        Ok(JsonNode {
            end: self.cursor,
            kind,
        })
    }

    fn object(&mut self) -> Result<JsonKind, ScrapeError> {
        self.cursor += 1;
        self.ws();
        let mut members = Vec::new();
        let mut prior_comma = None;
        if self.bytes.get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(JsonKind::Object(members));
        }
        loop {
            self.ws();
            let key_start = self.cursor;
            let key_span = self.string()?;
            let key = serde_json::from_slice::<String>(&self.bytes[key_span.0..key_span.1])
                .map_err(|error| fail(format!("invalid JSON object key: {error}")))?;
            self.ws();
            if self.bytes.get(self.cursor) != Some(&b':') {
                return Err(fail("JSON object key has no colon"));
            }
            self.cursor += 1;
            let value = self.value()?;
            self.ws();
            let comma_after = if self.bytes.get(self.cursor) == Some(&b',') {
                let comma = self.cursor;
                self.cursor += 1;
                Some(comma)
            } else {
                None
            };
            members.push(JsonMember {
                key,
                key_start,
                value,
                comma_before: prior_comma,
                comma_after,
            });
            if comma_after.is_none() {
                if self.bytes.get(self.cursor) != Some(&b'}') {
                    return Err(fail(
                        "JSON object member is not followed by comma or close brace",
                    ));
                }
                self.cursor += 1;
                break;
            }
            prior_comma = comma_after;
        }
        Ok(JsonKind::Object(members))
    }

    fn array(&mut self) -> Result<JsonKind, ScrapeError> {
        self.cursor += 1;
        self.ws();
        let mut values = Vec::new();
        if self.bytes.get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(JsonKind::Array);
        }
        loop {
            values.push(self.value()?);
            self.ws();
            match self.bytes.get(self.cursor) {
                Some(b',') => {
                    self.cursor += 1;
                    self.ws();
                }
                Some(b']') => {
                    self.cursor += 1;
                    break;
                }
                _ => {
                    return Err(fail(
                        "JSON array value is not followed by comma or close bracket",
                    ));
                }
            }
        }
        let _ = values;
        Ok(JsonKind::Array)
    }

    fn string(&mut self) -> Result<(usize, usize), ScrapeError> {
        let start = self.cursor;
        if self.bytes.get(self.cursor) != Some(&b'"') {
            return Err(fail("JSON string expected"));
        }
        self.cursor += 1;
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return Ok((start, self.cursor));
                }
                b'\\' => {
                    self.cursor += 2;
                    if self.cursor > self.bytes.len() {
                        return Err(fail("truncated JSON escape"));
                    }
                }
                0..=0x1f => return Err(fail("control byte in JSON string")),
                _ => self.cursor += 1,
            }
        }
        Err(fail("unterminated JSON string"))
    }

    fn literal(&mut self, expected: &[u8]) -> Result<(), ScrapeError> {
        if self.bytes.get(self.cursor..self.cursor + expected.len()) != Some(expected) {
            return Err(fail("invalid JSON literal"));
        }
        self.cursor += expected.len();
        Ok(())
    }

    fn number(&mut self) -> Result<(), ScrapeError> {
        let start = self.cursor;
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E')
        }) {
            self.cursor += 1;
        }
        serde_json::from_slice::<serde_json::Number>(&self.bytes[start..self.cursor])
            .map(|_| ())
            .map_err(|error| fail(format!("invalid JSON number: {error}")))
    }
}

pub(super) fn json_object_at<'a>(
    node: &'a JsonNode,
    path: &[String],
) -> Result<&'a [JsonMember], ScrapeError> {
    if path.is_empty() {
        return match &node.kind {
            JsonKind::Object(members) => Ok(members),
            _ => Err(fail("selected JSON path is not an object")),
        };
    }
    let JsonKind::Object(members) = &node.kind else {
        return Err(fail(format!(
            "JSON path component `{}` descends through a non-object",
            path[0]
        )));
    };
    let matches = members
        .iter()
        .filter(|member| member.key == path[0])
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(fail(format!(
            "JSON object path component `{}` occurs {} times",
            path[0],
            matches.len()
        )));
    }
    json_object_at(&matches[0].value, &path[1..])
}

pub(super) fn prepare_json_members(
    before: &[u8],
    object: &[String],
    members: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    let root = JsonParser::parse(before)?;
    let (edits, count, nodes, spans) = json_member_edits(&root, object, members)?;
    let after = apply_edits(before, edits)?;
    let parsed_after = JsonParser::parse(&after)?;
    let residual = json_object_at(&parsed_after, object)?;
    if residual.iter().any(|entry| members.contains(&entry.key)) {
        return Err(fail("JSON registered member remains after preparation"));
    }
    Ok((after, count, nodes, spans))
}

pub(super) fn json_member_edits(
    root: &JsonNode,
    object: &[String],
    members: &[String],
) -> Result<JsonEditOutput, ScrapeError> {
    let object_members = json_object_at(root, object)?;
    let wanted = members.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    for (index, member) in object_members.iter().enumerate() {
        if wanted.contains(member.key.as_str()) {
            if object_members[..index]
                .iter()
                .any(|earlier| earlier.key == member.key)
            {
                return Err(fail(format!(
                    "duplicate JSON member `{}` is ambiguous",
                    member.key
                )));
            }
            selected.push(index);
        }
    }
    let count = selected.len();
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < selected.len() {
        let first = selected[cursor];
        let mut last = first;
        cursor += 1;
        while cursor < selected.len() && selected[cursor] == last + 1 {
            last = selected[cursor];
            cursor += 1;
        }
        let (start, end) = if last + 1 < object_members.len() {
            (
                object_members[first].key_start,
                object_members[last]
                    .comma_after
                    .ok_or_else(|| fail("JSON removal run has no following comma"))?
                    + 1,
            )
        } else if first > 0 {
            (
                object_members[first]
                    .comma_before
                    .ok_or_else(|| fail("JSON removal run has no preceding comma"))?,
                object_members[last].value.end,
            )
        } else {
            (
                object_members[first].key_start,
                object_members[last].value.end,
            )
        };
        let identities = object_members[first..=last]
            .iter()
            .map(|member| member.key.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let node = format!("json:{}.[{identities}]", object.join("."));
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
        spans.push(ByteSpan {
            start: u64::try_from(start).map_err(|_| fail("JSON span exceeds u64"))?,
            end: u64::try_from(end).map_err(|_| fail("JSON span exceeds u64"))?,
            node: node.clone(),
        });
        nodes.push(node);
    }
    Ok((edits, count, nodes, spans))
}

fn registered_ts_tag(line: &[u8]) -> Option<&'static str> {
    let mut line = trim_ascii(line);
    if let Some(rest) = line.strip_prefix(b"*") {
        line = trim_ascii(rest);
    }
    let rest = line.strip_prefix(b"@")?;
    for tag in ["spec", "verifies", "cell", "scope"] {
        let tag_bytes = tag.as_bytes();
        if rest.starts_with(tag_bytes)
            && rest
                .get(tag_bytes.len())
                .is_some_and(u8::is_ascii_whitespace)
            && valid_registered_reference(trim_ascii(&rest[tag_bytes.len()..]))
        {
            return Some(tag);
        }
    }
    None
}

pub(super) fn parse_tree(
    bytes: &[u8],
    language: tree_sitter::Language,
    label: &str,
) -> Result<tree_sitter::Tree, ScrapeError> {
    std::str::from_utf8(bytes).map_err(|_| fail(format!("{label} source is not UTF-8")))?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| fail(format!("cannot load {label} grammar: {error}")))?;
    let tree = parser
        .parse(bytes, None)
        .ok_or_else(|| fail(format!("{label} parser returned no tree")))?;
    if tree.root_node().has_error() || tree.root_node().is_missing() {
        return Err(fail(format!("{label} source contains a parse error")));
    }
    Ok(tree)
}

pub(super) fn collect_nodes(
    node: tree_sitter::Node<'_>,
    kind: &str,
    out: &mut Vec<(usize, usize)>,
) {
    if node.kind() == kind {
        out.push((node.start_byte(), node.end_byte()));
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_nodes(cursor.node(), kind, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

pub(super) fn syntax_fingerprint(node: tree_sitter::Node<'_>, bytes: &[u8], out: &mut Vec<u8>) {
    if node.kind() == "comment" {
        return;
    }
    out.extend_from_slice(node.kind().as_bytes());
    out.push(b'(');
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            syntax_fingerprint(cursor.node(), bytes, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    } else {
        out.extend_from_slice(&bytes[node.byte_range()]);
    }
    out.push(b')');
}

fn is_attached_jsdoc(root: tree_sitter::Node<'_>, bytes: &[u8], comment_end: usize) -> bool {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    let mut next: Option<tree_sitter::Node<'_>> = None;
    while let Some(node) = stack.pop() {
        if node.is_named()
            && node.kind() != "comment"
            && node.start_byte() >= comment_end
            && next.is_none_or(|known| node.start_byte() < known.start_byte())
        {
            next = Some(node);
        }
        cursor.reset(node);
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    let Some(next) = next else { return false };
    if !bytes[comment_end..next.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        return false;
    }
    let kind = next.kind();
    kind.ends_with("declaration")
        || kind.ends_with("signature")
        || matches!(
            kind,
            "class"
                | "interface_declaration"
                | "enum_declaration"
                | "method_definition"
                | "public_field_definition"
                | "lexical_declaration"
                | "variable_declaration"
                | "export_statement"
        )
}

pub(super) fn prepare_typescript(before: &[u8], tsx: bool) -> Result<RewriteOutput, ScrapeError> {
    let language: tree_sitter::Language = if tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    };
    let before_tree = parse_tree(before, language.clone(), "TypeScript")?;
    let mut comments = Vec::new();
    collect_nodes(before_tree.root_node(), "comment", &mut comments);
    let mut edits = Vec::new();
    let mut count = 0;
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for (comment_start, comment_end) in comments {
        let comment = &before[comment_start..comment_end];
        if !comment.starts_with(b"/**")
            || !is_attached_jsdoc(before_tree.root_node(), before, comment_end)
        {
            continue;
        }
        let mut line_edits = Vec::new();
        for (local_start, local_content_end, local_end) in line_spans(comment) {
            if let Some(tag) = registered_ts_tag(&comment[local_start..local_content_end]) {
                let start = comment_start + local_start;
                let end = comment_start + local_end;
                line_edits.push((start, end));
                spans.push(ByteSpan {
                    start: u64::try_from(start)
                        .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                    end: u64::try_from(end).map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                    node: format!("typescript-jsdoc:@{tag}@{start}"),
                });
                nodes.push(format!("typescript-jsdoc:@{tag}@{start}"));
                count += 1;
            }
        }
        if line_edits.is_empty() && !comment.contains(&b'\n') && comment.len() >= 5 {
            let body_start = 3;
            let body_end = comment.len() - 2;
            let body = &comment[body_start..body_end];
            for at in 0..body.len() {
                if body[at] != b'@' {
                    continue;
                }
                if let Some(tag) = registered_ts_tag(trim_ascii(&body[at..])) {
                    let start = comment_start + body_start + at;
                    let end = comment_start + body_end;
                    line_edits.push((start, end));
                    spans.push(ByteSpan {
                        start: u64::try_from(start)
                            .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                        end: u64::try_from(end)
                            .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                        node: format!("typescript-jsdoc:@{tag}@{start}"),
                    });
                    nodes.push(format!("typescript-jsdoc:@{tag}@{start}"));
                    count += 1;
                    break;
                }
            }
        }
        if line_edits.is_empty() {
            continue;
        }
        let local = line_edits
            .iter()
            .map(|(start, end)| Edit {
                start: start - comment_start,
                end: end - comment_start,
                replacement: Vec::new(),
            })
            .collect();
        let rewritten = apply_edits(comment, local)?;
        let body = if rewritten.len() >= 5 {
            trim_ascii(&rewritten[3..rewritten.len() - 2])
        } else {
            &rewritten[..]
        };
        let (start, end, replacement) = if body.is_empty() || body == b"*" {
            // The comment contained metadata only.  Remove its complete line
            // when possible, otherwise remove only the comment span.
            let line_start = before[..comment_start]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |at| at + 1);
            let line_end = before[comment_end..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(before.len(), |at| comment_end + at + 1);
            if trim_ascii(&before[line_start..comment_start]).is_empty()
                && trim_ascii(&before[comment_end..line_end]).is_empty()
            {
                (line_start, line_end, Vec::new())
            } else {
                (comment_start, comment_end, Vec::new())
            }
        } else {
            (comment_start, comment_end, rewritten)
        };
        edits.push(Edit {
            start,
            end,
            replacement,
        });
    }
    let after = apply_edits(before, edits)?;
    let after_tree = parse_tree(&after, language, "rewritten TypeScript")?;
    let mut erased_before = Vec::new();
    let mut parsed_after = Vec::new();
    syntax_fingerprint(before_tree.root_node(), before, &mut erased_before);
    syntax_fingerprint(after_tree.root_node(), &after, &mut parsed_after);
    if erased_before != parsed_after {
        return Err(fail(
            "TypeScript registered-metadata erasure changed the parsed product tree",
        ));
    }
    Ok((after, count, nodes, spans))
}
