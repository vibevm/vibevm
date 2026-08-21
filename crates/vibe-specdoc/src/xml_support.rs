//! Shared decoding and validation helpers for the XML frontend.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use crate::doc::{Block, StatusEl};
use crate::{Error, Result};
use progress_core::model::{Action, Audience, Stage, State, nearest};
use quick_xml::escape::unescape;
use quick_xml::events::BytesStart;

use super::xml_in::{Ev, Parser};

pub(super) fn name_of(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

pub(super) fn decode_attrs(
    e: &BytesStart<'_>,
    at: (usize, usize),
) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| Error::at(at.0, format!("ill-formed attribute: {err}")))?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let raw = String::from_utf8_lossy(&a.value).into_owned();
        let value = unescape(&raw)
            .map_err(|err| Error::at(at.0, format!("bad escape in attribute `{key}`: {err}")))?
            .into_owned();
        out.push((key, value));
    }
    Ok(out)
}

pub(super) fn push_text(
    evs: &mut Vec<Ev>,
    s: String,
    poss: &mut Vec<(usize, usize)>,
    at: (usize, usize),
) {
    if let Some(Ev::Text(prev)) = evs.last_mut() {
        prev.push_str(&s);
        return;
    }
    evs.push(Ev::Text(s));
    poss.push(at);
}

pub(super) fn kind(e: &Ev) -> String {
    match e {
        Ev::Start(n, _) => format!("<{n}>"),
        Ev::End(n) => format!("</{n}>"),
        Ev::Empty(n, _) => format!("<{n}/>"),
        Ev::Text(t) => format!("text `{t}`"),
        Ev::CData(_) => "CDATA".to_string(),
    }
}

/// 1-based line and byte column of an offset in the source.
pub(super) fn pos_of(xml: &str, off: usize) -> (usize, usize) {
    let off = off.min(xml.len());
    let before = &xml[..off];
    let line = before.matches('\n').count() + 1;
    let col = before.rfind('\n').map(|nl| off - nl).unwrap_or(off) + 1;
    (line, col)
}

pub(super) fn only_attrs(
    attrs: &[(String, String)],
    allowed: &[&str],
    el: &str,
    at: (usize, usize),
    p: &Parser<'_>,
) -> Result<()> {
    for (k, _) in attrs {
        if !allowed.contains(&k.as_str()) {
            return Err(p.err(
                at,
                format!(
                    "the <{el}> element has no `{k}` attribute — the dialect's vocabulary is closed"
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn last_unit_fact_id(b: &Block) -> Option<&str> {
    let unit = match b {
        Block::Paragraph(u) | Block::Quote(u) => Some(u),
        Block::List { items, .. } => items.last(),
        _ => None,
    };
    unit?.fact.as_ref()?.id.as_deref()
}

/// Decode a status from the attribute pairs — the fact form spells the
/// pair as one `status="stage/state"`, the element form as `stage` +
/// `state`.
pub(super) fn status_from_attrs(
    attrs: &[(String, String)],
    at: (usize, usize),
    p: &Parser<'_>,
) -> Result<StatusEl> {
    let get = |k: &str| {
        attrs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };
    for name in ["comment", "ref"] {
        if let Some(value) = get(name)
            && (value.contains('"') || value.contains('\r') || value.contains('\n'))
        {
            return Err(p.err(
                at,
                format!(
                    "the `{name}` value is not Markdown-expressible: status attributes cannot contain quotes or newlines"
                ),
            ));
        }
    }
    let (stage, state) = if let Some(v) = get("status") {
        let Some((s, t)) = v.split_once('/') else {
            return Err(p.err(
                at,
                format!(
                    "the `status` attribute is `<stage>/<state>` (e.g. impl/done), found `{v}`"
                ),
            ));
        };
        (parse_stage(s, at, p)?, parse_state(t, at, p)?)
    } else {
        (
            parse_stage(&get("stage").unwrap_or_default(), at, p)?,
            parse_state(&get("state").unwrap_or_default(), at, p)?,
        )
    };
    let mut el = StatusEl {
        stage,
        state,
        action: None,
        actionstage: None,
        audience: Vec::new(),
        comment: get("comment"),
        r#ref: get("ref"),
    };
    if let Some(v) = get("action") {
        el.action = Some(parse_vocab("action", &v, at, p, Action::parse)?);
    }
    if let Some(v) = get("actionstage") {
        el.actionstage = Some(parse_stage(&v, at, p)?);
    }
    if let Some(v) = get("audience") {
        for part in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            el.audience
                .push(parse_vocab("audience", part, at, p, Audience::parse)?);
        }
    }
    Ok(el)
}

fn parse_stage(s: &str, at: (usize, usize), p: &Parser<'_>) -> Result<Stage> {
    parse_vocab("stage", s, at, p, Stage::parse)
}

fn parse_state(s: &str, at: (usize, usize), p: &Parser<'_>) -> Result<State> {
    parse_vocab("state", s, at, p, State::parse)
}

fn parse_vocab<T>(
    what: &str,
    s: &str,
    at: (usize, usize),
    p: &Parser<'_>,
    parse: fn(&str) -> Option<T>,
) -> Result<T> {
    if let Some(v) = parse(s) {
        return Ok(v);
    }
    let hint = match what {
        "stage" | "actionstage" => nearest(s, Stage::ALL.iter().map(|s| s.as_str())),
        "state" => nearest(s, State::ALL.iter().map(|s| s.as_str())),
        "action" => nearest(s, Action::ALL.iter().map(|s| s.as_str())),
        "audience" => nearest(s, Audience::ALL.iter().map(|s| s.as_str())),
        _ => None,
    };
    match hint {
        Some(h) => Err(p.err(
            at,
            format!("unknown {what} value `{s}` — did you mean `{h}`?"),
        )),
        None => Err(p.err(at, format!("unknown {what} value `{s}`"))),
    }
}
