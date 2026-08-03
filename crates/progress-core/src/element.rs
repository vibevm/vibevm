//! Lexing of the `<status>` element and the `@stage/state` shorthand.
//!
//! Hand-rolled (no regex dependency): the grammar is tiny and the error
//! surface must be precise. Foreign inline grammars (`@spec://…`, `#use`,
//! `<!-- REVIEW -->`) are opaque text by law — the shorthand recognizer
//! must refuse `@spec://` by lookahead (PROP-043 §3.7).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#markup");

use crate::model::{Action, Audience, Stage, State};

/// A lexed `<status …>` element, attributes still raw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawElement {
    /// Raw `name="value"` pairs in source order.
    pub attrs: Vec<(String, String)>,
    pub self_closing: bool,
    /// Byte length of the opening tag (`<status …>` or `<status …/>`).
    pub tag_len: usize,
    /// Lexer-level problems (bad attribute syntax etc.).
    pub errors: Vec<String>,
}

/// A recognized shorthand token: `@stage` or `@stage/state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawShorthand {
    pub stage: Stage,
    pub state: State,
    /// Byte length of the token.
    pub len: usize,
}

/// Attribute values decoded into the closed vocabularies, with every
/// violation reported (never silently dropped).
#[derive(Debug, Clone, Default)]
pub struct DecodedAttrs {
    pub stage: Option<Stage>,
    pub state: Option<State>,
    pub action: Option<Action>,
    pub actionstage: Option<Stage>,
    pub audience: Vec<Audience>,
    pub comment: Option<String>,
    pub r#ref: Option<String>,
    /// (attribute, offending value, optional nearest-legal hint)
    pub violations: Vec<(String, String, Option<String>)>,
}

/// Try to lex a `<status` element starting exactly at `at` in `s`.
pub fn lex_element(s: &str, at: usize) -> Option<RawElement> {
    let rest = &s[at..];
    let tag = "<status";
    if !rest.starts_with(tag) {
        return None;
    }
    let after = &rest[tag.len()..];
    // Must be followed by whitespace, `>`, or `/>` — not `<statusx`.
    let mut chars = after.char_indices().peekable();
    match chars.peek() {
        Some((_, c)) if c.is_whitespace() || *c == '>' || *c == '/' => {}
        _ => return None,
    }

    let mut attrs = Vec::new();
    let mut errors = Vec::new();
    let bytes_consumed;
    let mut i = 0usize;
    let b = after.as_bytes();
    loop {
        // Skip whitespace.
        while i < b.len() && (b[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return None; // Tag never closed on this text span.
        }
        if after[i..].starts_with("/>") {
            bytes_consumed = i + 2;
            return Some(RawElement {
                attrs,
                self_closing: true,
                tag_len: tag.len() + bytes_consumed,
                errors,
            });
        }
        if after[i..].starts_with('>') {
            bytes_consumed = i + 1;
            return Some(RawElement {
                attrs,
                self_closing: false,
                tag_len: tag.len() + bytes_consumed,
                errors,
            });
        }
        // Attribute name.
        let name_start = i;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'-') {
            i += 1;
        }
        if i == name_start {
            errors.push(format!(
                "unexpected character `{}` inside <status> tag",
                &after[i..]
                    .chars()
                    .next()
                    .map(String::from)
                    .unwrap_or_default()
            ));
            return Some(RawElement {
                attrs,
                self_closing: false,
                tag_len: tag.len() + i,
                errors,
            });
        }
        let name = after[name_start..i].to_string();
        if !after[i..].starts_with('=') {
            errors.push(format!("attribute `{name}` has no `=\"value\"`"));
            continue;
        }
        i += 1;
        if !after[i..].starts_with('"') {
            errors.push(format!("attribute `{name}` value is not double-quoted"));
            continue;
        }
        i += 1;
        let val_start = i;
        while i < b.len() && b[i] != b'"' {
            i += 1;
        }
        if i >= b.len() {
            errors.push(format!("attribute `{name}` value never closes its quote"));
            return Some(RawElement {
                attrs,
                self_closing: false,
                tag_len: tag.len() + i,
                errors,
            });
        }
        attrs.push((name, after[val_start..i].to_string()));
        i += 1;
    }
}

/// Decode raw attribute pairs into the closed vocabularies (PROP-043 §3.2).
pub fn decode_attrs(raw: &[(String, String)]) -> DecodedAttrs {
    let mut d = DecodedAttrs::default();
    for (name, value) in raw {
        match name.as_str() {
            "stage" => match Stage::parse(value) {
                Some(v) => d.stage = Some(v),
                None => d.violations.push((
                    "stage".into(),
                    value.clone(),
                    crate::model::nearest(value, Stage::ALL.iter().map(|s| s.as_str()))
                        .map(String::from),
                )),
            },
            "state" => match State::parse(value) {
                Some(v) => d.state = Some(v),
                None => d.violations.push((
                    "state".into(),
                    value.clone(),
                    crate::model::nearest(value, State::ALL.iter().map(|s| s.as_str()))
                        .map(String::from),
                )),
            },
            "action" => match Action::parse(value) {
                Some(v) => d.action = Some(v),
                None => d.violations.push((
                    "action".into(),
                    value.clone(),
                    crate::model::nearest(value, Action::ALL.iter().map(|s| s.as_str()))
                        .map(String::from),
                )),
            },
            "actionstage" => match Stage::parse(value) {
                Some(v) => d.actionstage = Some(v),
                None => d.violations.push((
                    "actionstage".into(),
                    value.clone(),
                    crate::model::nearest(value, Stage::ALL.iter().map(|s| s.as_str()))
                        .map(String::from),
                )),
            },
            "audience" => {
                for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    match Audience::parse(part) {
                        Some(v) => d.audience.push(v),
                        None => d.violations.push((
                            "audience".into(),
                            part.to_string(),
                            crate::model::nearest(part, Audience::ALL.iter().map(|a| a.as_str()))
                                .map(String::from),
                        )),
                    }
                }
            }
            "comment" => d.comment = Some(value.clone()),
            "ref" => d.r#ref = Some(value.clone()),
            other => d.violations.push((other.to_string(), value.clone(), None)),
        }
    }
    d
}

/// Try to lex a shorthand starting exactly at `at` (which must point at `@`).
///
/// Refusals (return `None`): `@spec://…` (the foreign directive — `://`
/// lookahead), any `@word` whose word is not a legal stage, a stage
/// followed by `/notastate` (that is a candidate typo — the caller decides
/// whether position makes it marker-shaped enough to flag).
pub fn lex_shorthand(s: &str, at: usize) -> Option<RawShorthand> {
    let rest = &s[at..];
    if !rest.starts_with('@') {
        return None;
    }
    let body = &rest[1..];
    let word_end = body
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric())
        .map(|(i, _)| i)
        .unwrap_or(body.len());
    let word = &body[..word_end];
    let stage = Stage::parse(word)?;
    let after = &body[word_end..];
    // Foreign-grammar guard: `@spec://…` is not ours (PROP-043 §3.7).
    if after.starts_with("://") {
        return None;
    }
    if let Some(after_slash) = after.strip_prefix('/') {
        let st_end = after_slash
            .char_indices()
            .find(|(_, c)| !c.is_ascii_alphanumeric())
            .map(|(i, _)| i)
            .unwrap_or(after_slash.len());
        let st_word = &after_slash[..st_end];
        let state = State::parse(st_word)?;
        return Some(RawShorthand {
            stage,
            state,
            len: 1 + word_end + 1 + st_end,
        });
    }
    // Bare shorthand: default state=work; the one exception is
    // @unknown → hold (PROP-043 §3.7).
    let state = if stage == Stage::Unknown {
        State::Hold
    } else {
        State::Work
    };
    Some(RawShorthand {
        stage,
        state,
        len: 1 + word_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_point_element() {
        let el = lex_element(r#"<status stage="impl" state="work"/>"#, 0).expect("element");
        assert!(el.self_closing);
        assert_eq!(el.attrs.len(), 2);
        let d = decode_attrs(&el.attrs);
        assert_eq!(d.stage, Some(Stage::Impl));
        assert_eq!(d.state, Some(State::Work));
        assert!(d.violations.is_empty());
    }

    #[test]
    fn flags_typo_with_hint() {
        let el = lex_element(r#"<status stage="impl" state="work" action="rewrok"/>"#, 0)
            .expect("element");
        let d = decode_attrs(&el.attrs);
        assert_eq!(d.violations.len(), 1);
        assert_eq!(d.violations[0].2.as_deref(), Some("rework"));
    }

    #[test]
    fn shorthand_defaults_and_exception() {
        let sh = lex_shorthand("@impl", 0).expect("shorthand");
        assert_eq!((sh.stage, sh.state), (Stage::Impl, State::Work));
        let sh = lex_shorthand("@unknown", 0).expect("shorthand");
        assert_eq!((sh.stage, sh.state), (Stage::Unknown, State::Hold));
        let sh = lex_shorthand("@test/plan", 0).expect("shorthand");
        assert_eq!((sh.stage, sh.state), (Stage::Test, State::Plan));
    }

    #[test]
    fn shorthand_refuses_foreign_spec_directive() {
        // `@spec://…` is the in-place spec-citation grammar — never ours.
        assert_eq!(
            lex_shorthand("@spec://org.vibevm.core/vibevm/modules/x#y", 0),
            None
        );
        // Plain `@spec` (no `://`) IS ours.
        assert!(lex_shorthand("@spec", 0).is_some());
        // Unknown words are not shorthand at all.
        assert_eq!(lex_shorthand("@vasya", 0), None);
    }
}
