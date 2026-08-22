//! Block and leaf descent for the closed XML dialect: `<p>`/`<quote>`/
//! `<list>`/`<table>`/`<fence>`, generic and named fact wrappers, and the
//! `<status>` element. The document/section spine lives in `doc`.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use super::facts::{fact_element, facts_block, is_fact_element};
use super::reader::{Ev, Parser, Violation, attr, only_attrs};
use super::{XBlock, XFact, XStatus, XUnit};

pub(super) fn block(
    p: &mut Parser,
    name: &str,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
) -> Result<XBlock, Violation> {
    match name {
        "p" | "quote" => {
            only_attrs(attrs, &[], name, at)?;
            let u = unit(p, name, was_empty, false)?;
            Ok(if name == "p" {
                XBlock::Para {
                    unit: u,
                    line: at as u32,
                }
            } else {
                XBlock::Quote(u)
            })
        }
        "fence" => fence(p, attrs, at, was_empty),
        "facts" => facts_block(p, attrs, at, was_empty),
        "list" => list(p, attrs, at, was_empty),
        "table" => table(p, at, was_empty),
        other => Err(Violation::at(
            at,
            format!("the dialect has no <{other}> element — the vocabulary is closed"),
        )),
    }
}

fn list(
    p: &mut Parser,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
) -> Result<XBlock, Violation> {
    only_attrs(attrs, &["ordered"], "list", at)?;
    let ordered = match attr(attrs, "ordered") {
        Some("true") => true,
        Some("false") => false,
        Some(other) => {
            return Err(Violation::at(
                at,
                format!("the `ordered` attribute is \"true\"|\"false\", found `{other}`"),
            ));
        }
        None => {
            return Err(Violation::at(
                at,
                "the <list> element requires an `ordered` attribute (\"true\"|\"false\")",
            ));
        }
    };
    let mut items = Vec::new();
    if !was_empty {
        loop {
            p.skip_ws_text()?;
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        "unexpected end of input — <list> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "list" => {
                    p.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (n, a, item_at, item_empty) = p.take_start()?;
            if n != "item" {
                return Err(Violation::at(
                    item_at,
                    format!("the dialect has no <{n}> element (inside <list>) — only <item>"),
                ));
            }
            only_attrs(&a, &[], "item", item_at)?;
            items.push(unit(p, "item", item_empty, false)?);
        }
    }
    if items.is_empty() {
        return Err(Violation::at(
            at,
            "a <list> needs at least one <item> — an empty list is not Markdown-expressible",
        ));
    }
    Ok(XBlock::List { ordered, items })
}

fn table(p: &mut Parser, at: usize, was_empty: bool) -> Result<XBlock, Violation> {
    let mut rows: Vec<Vec<XUnit>> = Vec::new();
    if !was_empty {
        loop {
            p.skip_ws_text()?;
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        "unexpected end of input — <table> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "table" => {
                    p.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (n, a, tr_at, tr_empty) = p.take_start()?;
            if n != "tr" {
                return Err(Violation::at(
                    tr_at,
                    format!("the dialect has no <{n}> element (inside <table>) — only <tr>"),
                ));
            }
            only_attrs(&a, &[], "tr", tr_at)?;
            let mut row = Vec::new();
            if !tr_empty {
                loop {
                    p.skip_ws_text()?;
                    match p.evs.get(p.i) {
                        None => {
                            return Err(Violation::at(
                                0,
                                "unexpected end of input — <tr> never closed",
                            ));
                        }
                        Some(Ev::End(n2)) if n2 == "tr" => {
                            p.i += 1;
                            break;
                        }
                        Some(_) => {}
                    }
                    let (n2, a2, td_at, td_empty) = p.take_start()?;
                    if n2 != "td" {
                        return Err(Violation::at(
                            td_at,
                            format!("the dialect has no <{n2}> element (inside <tr>) — only <td>"),
                        ));
                    }
                    only_attrs(&a2, &[], "td", td_at)?;
                    // The cell exemption: a marked cell may be id-less.
                    row.push(unit(p, "td", td_empty, true)?);
                }
            }
            if row.is_empty() {
                return Err(Violation::at(tr_at, "a <tr> needs at least one <td>"));
            }
            rows.push(row);
        }
    }
    if rows.is_empty() {
        return Err(Violation::at(
            at,
            "a <table> needs at least one <tr> — an empty table is not Markdown-expressible",
        ));
    }
    Ok(XBlock::Table { rows })
}

/// A fence: `lang`/`fact` attributes, verbatim content (text and CDATA —
/// the one place CDATA is legal). Carries its native line for the
/// binding-law diagnostic.
fn fence(
    p: &mut Parser,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
) -> Result<XBlock, Violation> {
    only_attrs(attrs, &["lang", "fact"], "fence", at)?;
    let lang = attr(attrs, "lang").map(str::to_string);
    let fact = attr(attrs, "fact").map(str::to_string);
    let mut text = String::new();
    if !was_empty {
        loop {
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        "unexpected end of input — <fence> never closed",
                    ));
                }
                Some(Ev::End(n)) if n == "fence" => {
                    p.i += 1;
                    break;
                }
                Some(Ev::Text(t)) => {
                    text.push_str(t);
                    p.i += 1;
                }
                Some(Ev::CData(c)) => {
                    text.push_str(c);
                    p.i += 1;
                }
                Some(other) => {
                    return Err(Violation::at(
                        p.here(),
                        format!(
                            "a <fence> holds only text and CDATA — found {}",
                            other.what()
                        ),
                    ));
                }
            }
        }
    }
    Ok(XBlock::Fence {
        lang,
        fact,
        text,
        line: at as u32,
    })
}

/// One unit-bearing leaf (`p`, `item`, `quote`, `td`): bare text, or exactly
/// one wrapping generic or named fact. `in_cell` permits an id-less generic
/// marked cell.
fn unit(p: &mut Parser, tag: &str, was_empty: bool, in_cell: bool) -> Result<XUnit, Violation> {
    let mut text = String::new();
    let mut fact: Option<XFact> = None;
    if !was_empty {
        loop {
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        format!("unexpected end of input — <{tag}> never closed"),
                    ));
                }
                Some(Ev::End(n)) if n == tag => {
                    p.i += 1;
                    break;
                }
                Some(Ev::Text(t)) => {
                    if t.trim().is_empty() {
                        // whitespace around a fact (or an empty leaf)
                        p.i += 1;
                        continue;
                    }
                    if fact.is_some() {
                        return Err(Violation::at(
                            p.here(),
                            format!(
                                "text beside <fact> inside <{tag}> — a {tag} is ONE unit: bare text or one fact"
                            ),
                        ));
                    }
                    text.push_str(t);
                    p.i += 1;
                }
                Some(Ev::CData(_)) => {
                    return Err(Violation::at(
                        p.here(),
                        "CDATA is allowed only inside <fence>",
                    ));
                }
                Some(Ev::Start(n, a)) if fact.is_none() => {
                    let element_name = n.clone();
                    let attrs = a.clone();
                    let at = p.poss[p.i];
                    if !is_fact_element(&element_name, &attrs, at)? {
                        return Err(Violation::at(
                            at,
                            format!("the dialect has no <{element_name}> element (inside <{tag}>)"),
                        ));
                    }
                    if !text.trim().is_empty() {
                        return Err(Violation::at(
                            p.here(),
                            format!(
                                "text and <fact> cannot mix inside <{tag}> — a {tag} is ONE unit"
                            ),
                        ));
                    }
                    p.i += 1;
                    let (f, content) = fact_element(p, &element_name, &attrs, at, in_cell, false)?;
                    fact = Some(f);
                    text = content;
                }
                Some(Ev::Empty(n, a)) if fact.is_none() => {
                    let element_name = n.clone();
                    let attrs = a.clone();
                    let at = p.poss[p.i];
                    if !is_fact_element(&element_name, &attrs, at)? {
                        return Err(Violation::at(
                            at,
                            format!(
                                "the dialect has no <{element_name}/> element (inside <{tag}>)"
                            ),
                        ));
                    }
                    if !text.trim().is_empty() {
                        return Err(Violation::at(
                            p.here(),
                            format!(
                                "text and <fact> cannot mix inside <{tag}> — a {tag} is ONE unit"
                            ),
                        ));
                    }
                    p.i += 1;
                    let (f, content) = fact_element(p, &element_name, &attrs, at, in_cell, true)?;
                    fact = Some(f);
                    text = content;
                }
                Some(other) => {
                    return Err(Violation::at(
                        p.here(),
                        format!("the dialect has no {} (inside <{tag}>)", other.what()),
                    ));
                }
            }
        }
    }
    let text = text.trim().to_string();
    if tag == "td" && (text.contains('|') || text.contains('\n')) {
        return Err(Violation::at(
            p.poss[p.i.saturating_sub(1)],
            "a <td> cannot hold `|` or a newline — the Markdown table form cannot express it",
        ));
    }
    if let Some(f) = fact.as_ref()
        && !f.is_meaningful()
    {
        return Err(Violation::at(
            p.poss[p.i.saturating_sub(1)],
            "an empty <fact> carries nothing — give it an id or a status",
        ));
    }
    if fact.is_some() {
        return Ok(XUnit { fact, text });
    }
    if text.is_empty() && !in_cell {
        return Err(Violation::at(
            p.poss[p.i.saturating_sub(1)],
            format!("an empty <{tag}> — the Markdown form cannot express it"),
        ));
    }
    Ok(XUnit { fact: None, text })
}

/// The text of a bare-text leaf (`<title>`): text-only content, verbatim.
pub(super) fn leaf_text(p: &mut Parser, tag: &str, was_empty: bool) -> Result<String, Violation> {
    let mut text = String::new();
    if !was_empty {
        loop {
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        format!("unexpected end of input — <{tag}> never closed"),
                    ));
                }
                Some(Ev::End(n)) if n == tag => {
                    p.i += 1;
                    break;
                }
                Some(Ev::Text(t)) => {
                    text.push_str(t);
                    p.i += 1;
                }
                Some(other) => {
                    return Err(Violation::at(
                        p.here(),
                        format!("a <{tag}> holds only text — found {}", other.what()),
                    ));
                }
            }
        }
    }
    Ok(text.trim().to_string())
}

/// The `<status>` element: stage and state required, the closed extras, no
/// children. Values pass through verbatim — the vocabulary check is the
/// authoring frontend's; this reader renders.
pub(super) fn status_element(
    p: &mut Parser,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
) -> Result<XStatus, Violation> {
    only_attrs(
        attrs,
        &[
            "stage",
            "state",
            "action",
            "actionstage",
            "audience",
            "comment",
            "ref",
        ],
        "status",
        at,
    )?;
    if attr(attrs, "stage").is_none() || attr(attrs, "state").is_none() {
        return Err(Violation::at(
            at,
            "the <status> element requires both `stage` and `state`",
        ));
    }
    let el = status_from_attrs(attrs, at)?;
    if !was_empty {
        match p.evs.get(p.i) {
            Some(Ev::End(n)) if n == "status" => p.i += 1,
            Some(other) => {
                return Err(Violation::at(
                    p.here(),
                    format!("a <status> element is empty — found {}", other.what()),
                ));
            }
            None => {
                return Err(Violation::at(
                    0,
                    "unexpected end of input — <status> never closed",
                ));
            }
        }
    }
    Ok(el)
}

pub(super) fn status_from_attrs(
    attrs: &[(String, String)],
    at: usize,
) -> Result<XStatus, Violation> {
    for name in ["comment", "ref"] {
        if let Some(value) = attr(attrs, name)
            && (value.contains('"') || value.contains('\r') || value.contains('\n'))
        {
            return Err(Violation::at(
                at,
                format!(
                    "the `{name}` value is not Markdown-expressible: status attributes cannot contain quotes or newlines"
                ),
            ));
        }
    }
    let (stage, state) = match attr(attrs, "status") {
        Some(v) => match v.split_once('/') {
            Some((s, t)) => (s.to_string(), t.to_string()),
            None => {
                return Err(Violation::at(
                    at,
                    format!(
                        "the `status` attribute is `<stage>/<state>` (e.g. impl/done), found `{v}`"
                    ),
                ));
            }
        },
        None => (
            attr(attrs, "stage").unwrap_or_default().to_string(),
            attr(attrs, "state").unwrap_or_default().to_string(),
        ),
    };
    let audience = attr(attrs, "audience")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(XStatus {
        stage,
        state,
        action: attr(attrs, "action").map(str::to_string),
        actionstage: attr(attrs, "actionstage").map(str::to_string),
        audience,
        comment: attr(attrs, "comment").map(str::to_string),
        r#ref: attr(attrs, "ref").map(str::to_string),
    })
}
