//! Fact-owned terminal-artifact annotation parsing.

use crate::doc::{Issue, IssueCode, ParsedDoc, Severity};
use crate::element;
use crate::model::{ArtifactKind, ArtifactRequirements, MarkerForm, Stage, State};

struct TailStatus {
    start: usize,
    form: MarkerForm,
    state: State,
    refs: Vec<String>,
}

enum TailAnnotation {
    None,
    Valid {
        start: usize,
        requirements: ArtifactRequirements,
    },
    Invalid {
        start: usize,
        message: String,
    },
}

pub(super) fn scan_requirements(doc: &mut ParsedDoc) {
    for bi in 0..doc.blocks.len() {
        for fi in 0..doc.blocks[bi].facts.len() {
            let block = &doc.blocks[bi];
            let fact = &block.facts[fi];
            let visible = mask_link_destinations(&block.scan_text[fact.span.0..fact.span.1]);
            let raw = &block.source_text[fact.span.0..fact.span.1];
            let line = fact.line;
            let id_present = fact.id.is_some();
            let (requirements, mut issues) =
                parse_fact_requirements(&visible, raw, line, id_present);
            doc.blocks[bi].facts[fi].requirements = requirements;
            doc.issues.append(&mut issues);
        }
    }
}

fn parse_fact_requirements(
    visible: &str,
    raw: &str,
    line: usize,
    id_present: bool,
) -> (Option<ArtifactRequirements>, Vec<Issue>) {
    let mut issues = Vec::new();
    let tail = tail_status(visible, raw);
    let cluster_end = tail.as_ref().map_or_else(
        || malformed_status_start(visible).unwrap_or(visible.len()),
        |s| s.start,
    );
    let (token_start, requirements) = match tail_annotation(&visible[..cluster_end]) {
        TailAnnotation::None => return (None, issues),
        TailAnnotation::Invalid { start, message } => {
            push_issue(&mut issues, line_at(visible, line, start), &message);
            return (None, issues);
        }
        TailAnnotation::Valid {
            start,
            requirements,
        } => (start, requirements),
    };

    let Some(status) = tail else {
        push_issue(
            &mut issues,
            line_at(visible, line, token_start),
            "`@requires:` needs a final fact status marker",
        );
        return (None, issues);
    };

    if !id_present {
        push_issue(
            &mut issues,
            line_at(visible, line, token_start),
            "`@requires:` belongs only to an explicitly addressed fact",
        );
    }
    if status.state == State::Void {
        push_issue(
            &mut issues,
            line_at(visible, line, token_start),
            "a void fact cannot carry terminal artifact requirements",
        );
    }
    if requirements.contains(ArtifactKind::External)
        && (status.form != MarkerForm::Point
            || status.refs.len() != 1
            || status.refs[0].trim().is_empty())
    {
        push_issue(
            &mut issues,
            line_at(visible, line, status.start),
            "an `external` requirement needs exactly one non-empty `ref` on the final status point marker",
        );
    }
    (Some(requirements), issues)
}

fn tail_annotation(s: &str) -> TailAnnotation {
    let trimmed = s.trim_end();
    let (start, token) = last_token(trimmed);
    if token.starts_with("@requires") {
        let prior = trimmed[..start].trim_end();
        let (_, previous) = last_token(prior);
        if previous.starts_with("@requires") || split_attempt_start(prior).is_some() {
            return TailAnnotation::Invalid {
                start,
                message: "a fact may carry exactly one `@requires:` annotation".into(),
            };
        }
        let Some(csv) = token.strip_prefix("@requires:") else {
            return malformed_annotation(start);
        };
        return match ArtifactRequirements::parse_csv(csv) {
            Ok(requirements) => TailAnnotation::Valid {
                start,
                requirements,
            },
            Err(message) => TailAnnotation::Invalid {
                start,
                message: format!("invalid `@requires:` annotation: {message}"),
            },
        };
    }

    // Backtrack a comma-shaped split-member chain of any length. Only that
    // exact trailing shape is reserved: `@requires:plan remains prose` does
    // not match, while `@requires:plan, verification, documentation` does.
    split_attempt_start(trimmed).map_or(TailAnnotation::None, malformed_annotation)
}

fn split_attempt_start(s: &str) -> Option<usize> {
    let trimmed = s.trim_end();
    let (mut cursor, closest) = last_token(trimmed);
    if closest.is_empty() {
        return None;
    }
    loop {
        let prior = trimmed[..cursor].trim_end();
        let (prior_start, previous) = last_token(prior);
        if previous == "@requires"
            || previous == "@requires:"
            || (previous.starts_with("@requires:") && previous.ends_with(','))
        {
            return Some(prior_start);
        }
        if !previous.ends_with(',') {
            return None;
        }
        cursor = prior_start;
    }
}

fn malformed_annotation(start: usize) -> TailAnnotation {
    TailAnnotation::Invalid {
        start,
        message: "malformed requirements annotation — expected `@requires:<kind>[,<kind>…]`".into(),
    }
}

fn last_token(s: &str) -> (usize, &str) {
    let start = s
        .rfind(char::is_whitespace)
        .map_or(0, |at| at + s[at..].chars().next().unwrap().len_utf8());
    (start, &s[start..])
}

fn tail_status(visible: &str, raw: &str) -> Option<TailStatus> {
    let end = visible.trim_end().len();
    let text = &visible[..end];
    if let Some(at) = text.rfind("<status")
        && let Some(el) = element::lex_element(raw, at)
        && el.self_closing
        && at + el.tag_len == end
    {
        let decoded = element::decode_attrs(&el.attrs);
        return Some(TailStatus {
            start: at,
            form: MarkerForm::Point,
            state: decoded.state?,
            refs: el
                .attrs
                .iter()
                .filter(|(name, _)| name == "ref")
                .map(|(_, value)| value.clone())
                .collect(),
        });
    }
    let at = text.rfind('@')?;
    let shorthand = element::lex_shorthand(text, at)?;
    (at + shorthand.len == end).then_some(TailStatus {
        start: at,
        form: MarkerForm::Shorthand,
        state: shorthand.state,
        refs: Vec::new(),
    })
}

/// A qualified status-shaped final token that failed the real status lexer.
/// It delimits the terminal cluster but never supplies a status.
fn malformed_status_start(s: &str) -> Option<usize> {
    let text = s.trim_end();
    let (start, token) = last_token(text);
    if token.starts_with("@status:") {
        return Some(start);
    }
    if legacy_status_like(token) {
        return Some(start);
    }
    final_xml_status_like(text)
}

fn final_xml_status_like(text: &str) -> Option<usize> {
    let at = text.rfind("<status")?;
    let suffix = &text[at..];
    // This occurrence precedes the requirements annotation; it cannot be the
    // final status suffix that follows the annotation.
    if suffix.contains("@requires:") {
        return None;
    }
    match element::lex_element(text, at) {
        Some(status) if status.self_closing && at + status.tag_len == text.len() => Some(at),
        Some(status)
            if !status.self_closing
                && (at + status.tag_len == text.len()
                    || suffix.trim_end().ends_with("</status>")) =>
        {
            Some(at)
        }
        None if !suffix.contains('>') => Some(at),
        _ => None,
    }
}

fn legacy_status_like(token: &str) -> bool {
    let Some(body) = token.strip_prefix('@') else {
        return false;
    };
    let mut parts = body.split('/');
    let stage = parts.next().unwrap_or_default();
    if stage.is_empty() || !stage.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let stage_legal = Stage::parse(stage).is_some();
    match parts.next() {
        // A malformed single bare word is indistinguishable from an external
        // mention. Legal bare stages were already consumed by `tail_status`.
        None => false,
        Some(state) => {
            if state.is_empty()
                || !state.chars().all(|c| c.is_ascii_alphanumeric())
                || parts.next().is_some()
            {
                return false;
            }
            let state_legal = State::parse(state).is_some();
            stage_legal || state_legal
        }
    }
}

fn push_issue(issues: &mut Vec<Issue>, line: usize, message: &str) {
    issues.push(Issue {
        severity: Severity::Error,
        line,
        code: IssueCode::TerminalRequirements,
        message: message.into(),
    });
}

fn line_at(text: &str, first: usize, at: usize) -> usize {
    first + text[..at].matches('\n').count()
}

/// Blank inline link destinations while preserving every byte offset.
pub(super) fn mask_link_destinations(s: &str) -> String {
    let mut bytes = s.as_bytes().to_vec();
    let original = s.as_bytes();
    let mut i = 0usize;
    while i + 1 < original.len() {
        if original[i] == b']'
            && original[i + 1] == b'('
            && !is_escaped(s, i)
            && has_open_label(s, i)
            && let Some((close, after)) = destination_close(s, i + 2)
        {
            for at in i + 2..close {
                if original[at] != b'\n' {
                    bytes[at] = b' ';
                }
            }
            i = after;
        } else {
            i += 1;
        }
    }
    String::from_utf8(bytes).expect("blanking whole UTF-8 characters preserves UTF-8")
}

fn is_escaped(s: &str, at: usize) -> bool {
    s.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}

fn has_open_label(s: &str, before: usize) -> bool {
    let mut depth = 0usize;
    let mut escaped = false;
    for (_, c) in s[..before].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

/// `(closing parenthesis byte, byte after it)` for one balanced destination.
fn destination_close(s: &str, start: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut j = start;
    let mut escaped = false;
    while j < s.len() && depth > 0 {
        let c = s[j..].chars().next().expect("j is a character boundary");
        if escaped {
            escaped = false;
            j += c.len_utf8();
            continue;
        }
        match c {
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        j += c.len_utf8();
    }
    (depth == 0).then_some((j - 1, j))
}

#[cfg(test)]
mod tests {
    use crate::parse::parse_document;

    fn only_fact(source: &str) -> crate::doc::Fact {
        parse_document("x.md", source).blocks[0].facts[0].clone()
    }

    #[test]
    fn parses_and_canonicalises_the_set() {
        let fact = only_fact(
            "@fact:A body @requires:external,specification,implementation <status stage=\"impl\" state=\"done\" ref=\"ticket:1\"/>",
        );
        assert_eq!(
            fact.requirements.unwrap().to_csv(),
            "specification,implementation,external"
        );
    }

    #[test]
    fn every_markdown_fact_carrier_keeps_the_contract() {
        let source = "@fact:PARA paragraph @requires:verification,specification @status:test/done\n\n\
                      - @fact:ITEM item @requires:verification,specification @status:test/done\n\n\
                      | H |\n|---|\n| @fact:CELL cell @requires:verification,specification @status:test/done |\n\n\
                      > @fact:QUOTE quote @requires:verification,specification @status:test/done\n\n\
                      @fact/code:CODE typed @requires:verification,specification @status:test/done\n\n\
                      ```rust\nassert!(true);\n```";
        let doc = parse_document("x.md", source);
        assert_eq!(doc.error_count(), 0, "{:#?}", doc.issues);
        let facts = doc
            .blocks
            .iter()
            .flat_map(|block| &block.facts)
            .filter(|fact| fact.requirements.is_some())
            .collect::<Vec<_>>();
        assert_eq!(facts.len(), 5, "{:#?}", doc.blocks);
        assert!(facts.iter().all(|fact| {
            fact.requirements.as_ref().unwrap().to_csv() == "specification,verification"
        }));
    }

    #[test]
    fn every_admitted_final_status_spelling_accepts_requirements() {
        for status in [
            "@status:spec/done",
            "@status:spec",
            "@spec/done",
            "@spec",
            "<status stage=\"spec\" state=\"done\"/>",
        ] {
            let source = format!("@fact:A body @requires:plan {status}");
            let doc = parse_document("x.md", &source);
            assert_eq!(doc.error_count(), 0, "{source}: {:#?}", doc.issues);
            assert_eq!(
                doc.blocks[0].facts[0]
                    .requirements
                    .as_ref()
                    .unwrap()
                    .to_csv(),
                "plan"
            );
        }
    }

    #[test]
    fn invalid_terminal_clusters_are_loud() {
        for (source, needle) in [
            ("@fact:A body @requires:specification", "needs a final"),
            (
                "@fact:A body @requires:specification @requires:plan @status:spec/done",
                "exactly one",
            ),
            (
                "@fact:A body @requires:plan, verification @requires:research @status:spec/done",
                "exactly one",
            ),
            ("@fact:A body @requires: @status:spec/done", "list is empty"),
            (
                "@fact:A body @requires:,plan @status:spec/done",
                "empty member",
            ),
            (
                "@fact:A body @requires:plan, @status:spec/done",
                "empty member",
            ),
            (
                "@fact:A body @requires:plan,plan @status:spec/done",
                "duplicate",
            ),
            (
                "@fact:A body @requires:spaceship @status:spec/done",
                "unknown required",
            ),
            (
                "@fact:A body @requires=plan @status:spec/done",
                "malformed requirements",
            ),
            (
                "@fact:A body @requires:plan, verification @status:spec/done",
                "malformed requirements",
            ),
            (
                "@fact:A body @requires:plan, verification, documentation @status:spec/done",
                "malformed requirements",
            ),
            (
                "@fact:A body @requires plan @status:spec/done",
                "malformed requirements",
            ),
            (
                "@fact:A body @requires:plan @status:specc/done",
                "needs a final fact status",
            ),
            (
                "@fact:A body @requires:plan, verification @status:specc/done",
                "malformed requirements",
            ),
            ("@fact:A body @requires:plan @status:spec/void", "void fact"),
            (
                "@fact:A body @requires:external @status:impl/done",
                "non-empty `ref`",
            ),
            (
                "body @requires:plan @status:spec/done",
                "explicitly addressed",
            ),
            (
                "@fact:A body @requires:external <status stage=\"impl\" state=\"done\" ref=\"\"/>",
                "non-empty `ref`",
            ),
            (
                "@fact:A body @requires:external <status stage=\"impl\" state=\"done\" ref=\"one\" ref=\"two\"/>",
                "non-empty `ref`",
            ),
        ] {
            let doc = parse_document("x.md", source);
            assert!(
                doc.issues
                    .iter()
                    .any(|issue| issue.message.contains(needle)),
                "{source:?}: {:#?}",
                doc.issues
            );
        }
    }

    #[test]
    fn malformed_legacy_status_like_tails_leave_requirements_invalid() {
        for tail in ["@specc/done", "@spec/finished"] {
            let source = format!("@fact:A body @requires:plan {tail}");
            let doc = parse_document("x.md", &source);
            assert!(
                doc.issues.iter().any(|issue| {
                    issue.code == crate::doc::IssueCode::TerminalRequirements
                        && issue.message.contains("needs a final fact status")
                }),
                "{source}: {:#?}",
                doc.issues
            );
        }
    }

    #[test]
    fn earlier_status_forms_do_not_hide_a_trailing_requirement() {
        for source in [
            "@fact:A <status stage=\"spec\" state=\"done\">fragment</status> @requires:plan",
            "@fact:A <status stage=\"spec\" state=\"done\"/> body @requires:plan",
            "@fact:A body @requires:plan <status stage=\"spec\" state=\"done\">",
        ] {
            let doc = parse_document("x.md", source);
            assert!(
                doc.issues.iter().any(|issue| {
                    issue.code == crate::doc::IssueCode::TerminalRequirements
                        && issue.message.contains("needs a final fact status")
                }),
                "{source}: {:#?}",
                doc.issues
            );
        }
    }

    #[test]
    fn ordinary_mentions_after_requires_remain_unclassified_prose() {
        for mention in ["@specc", "@bob", "@alice", "@alice/bob"] {
            let source = format!("@fact:A body @requires:plan {mention}");
            let doc = parse_document("x.md", &source);
            assert_eq!(doc.error_count(), 0, "{source}: {:#?}", doc.issues);
            assert!(doc.blocks[0].facts[0].requirements.is_none(), "{source}");
        }

        let source =
            "@fact:A body @requires:plan, remains prose @requires:research @status:spec/done";
        let doc = parse_document("x.md", source);
        assert_eq!(doc.error_count(), 0, "{:#?}", doc.issues);
        assert_eq!(
            doc.blocks[0].facts[0]
                .requirements
                .as_ref()
                .unwrap()
                .to_csv(),
            "research"
        );
    }

    #[test]
    fn opaque_and_incidental_lookalikes_stay_prose() {
        let doc = parse_document(
            "x.md",
            "@fact:A `@requires:plan` [link](https://x/é\\)@requires:plan) prose @requires:specification stays ordinary @requires:research too @status:spec/done\n\n```md\n@requires:plan\n```",
        );
        assert_eq!(doc.error_count(), 0, "{:#?}", doc.issues);
        assert!(doc.blocks[0].facts[0].requirements.is_none());
    }

    #[test]
    fn presence_and_authored_order_move_content_identity() {
        let absent = "@fact:A body @status:spec/done";
        let first = "@fact:A body @requires:specification,plan @status:spec/done";
        let reordered = "@fact:A body @requires:plan,specification @status:spec/done";
        assert_ne!(
            crate::parse::content_hash(absent),
            crate::parse::content_hash(first)
        );
        assert_ne!(
            crate::parse::content_hash(first),
            crate::parse::content_hash(reordered)
        );
        let canonical = parse_document("x.md", reordered).blocks[0].facts[0]
            .requirements
            .as_ref()
            .unwrap()
            .to_csv();
        assert_eq!(canonical, "specification,plan");
    }
}
