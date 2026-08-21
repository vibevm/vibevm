//! Block and leaf descent for the closed XML dialect.

use super::xml_in::{Ev, Parser};
use super::xml_support::{kind, only_attrs, status_from_attrs};
use crate::doc::{Block, Fact, StatusEl, Unit};
use crate::{Error, Result};

impl<'a> Parser<'a> {
    pub(super) fn block(
        &mut self,
        name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        match name {
            "p" | "quote" => {
                only_attrs(attrs, &[], name, at, self)?;
                let u = self.unit(name, was_empty, false)?;
                Ok(if name == "p" {
                    Block::Paragraph(u)
                } else {
                    Block::Quote(u)
                })
            }
            "fence" => self.fence(attrs, at, was_empty),
            "list" => {
                only_attrs(attrs, &["ordered"], "list", at, self)?;
                let Some((_, v)) = attrs.iter().find(|(k, _)| k == "ordered") else {
                    return Err(self.err(
                        at,
                        "the <list> element requires an `ordered` attribute (\"true\"|\"false\")"
                            .into(),
                    ));
                };
                let ordered = match v.as_str() {
                    "true" => true,
                    "false" => false,
                    other => {
                        return Err(self.err(
                            at,
                            format!(
                                "the `ordered` attribute is \"true\"|\"false\", found `{other}`"
                            ),
                        ));
                    }
                };
                let mut items = Vec::new();
                if !was_empty {
                    'items: loop {
                        self.skip_ws_text()?;
                        match self.evs.get(self.i) {
                            None => {
                                return Err(Error::at(
                                    0,
                                    "unexpected end of input — <list> never closed",
                                ));
                            }
                            Some(Ev::End(n)) if n == "list" => {
                                self.i += 1;
                                break 'items;
                            }
                            Some(_) => {}
                        }
                        let (n, a, at, item_empty) = self.take_start()?;
                        if n != "item" {
                            return Err(self.err(
                                at,
                                format!(
                                    "the dialect has no <{n}> element (inside <list>) — only <item>"
                                ),
                            ));
                        }
                        only_attrs(&a, &[], "item", at, self)?;
                        items.push(self.unit("item", item_empty, false)?);
                    }
                }
                if items.is_empty() {
                    return Err(self.err(
                        at,
                        "a <list> needs at least one <item> — an empty list is not Markdown-expressible".into(),
                    ));
                }
                Ok(Block::List { ordered, items })
            }
            "table" => {
                only_attrs(attrs, &[], "table", at, self)?;
                let mut rows = Vec::new();
                if !was_empty {
                    'rows: loop {
                        self.skip_ws_text()?;
                        match self.evs.get(self.i) {
                            None => {
                                return Err(Error::at(
                                    0,
                                    "unexpected end of input — <table> never closed",
                                ));
                            }
                            Some(Ev::End(n)) if n == "table" => {
                                self.i += 1;
                                break 'rows;
                            }
                            Some(_) => {}
                        }
                        let (n, a, at, tr_empty) = self.take_start()?;
                        if n != "tr" {
                            return Err(self.err(
                                at,
                                format!(
                                    "the dialect has no <{n}> element (inside <table>) — only <tr>"
                                ),
                            ));
                        }
                        only_attrs(&a, &[], "tr", at, self)?;
                        let mut row = Vec::new();
                        if !tr_empty {
                            'cells: loop {
                                self.skip_ws_text()?;
                                match self.evs.get(self.i) {
                                    None => {
                                        return Err(Error::at(
                                            0,
                                            "unexpected end of input — <tr> never closed",
                                        ));
                                    }
                                    Some(Ev::End(n)) if n == "tr" => {
                                        self.i += 1;
                                        break 'cells;
                                    }
                                    Some(_) => {}
                                }
                                let (n, a, at, td_empty) = self.take_start()?;
                                if n != "td" {
                                    return Err(self.err(
                                        at,
                                        format!(
                                            "the dialect has no <{n}> element (inside <tr>) — only <td>"
                                        ),
                                    ));
                                }
                                only_attrs(&a, &[], "td", at, self)?;
                                // The cell exemption: a marked cell may be id-less.
                                row.push(self.unit("td", td_empty, true)?);
                            }
                        }
                        if row.is_empty() {
                            return Err(self.err(at, "a <tr> needs at least one <td>".into()));
                        }
                        rows.push(row);
                    }
                }
                if rows.is_empty() {
                    return Err(self.err(
                        at,
                        "a <table> needs at least one <tr> — an empty table is not Markdown-expressible".into(),
                    ));
                }
                Ok(Block::Table { rows })
            }
            other => Err(self.err(
                at,
                format!("the dialect has no <{other}> element — the vocabulary is closed"),
            )),
        }
    }

    /// A fence: `lang`/`fact` attributes, verbatim content (text and
    /// CDATA — the one place CDATA is legal).
    fn fence(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs(attrs, &["lang", "fact"], "fence", at, self)?;
        let lang = attrs
            .iter()
            .find(|(k, _)| k == "lang")
            .map(|(_, v)| v.clone());
        let fact = attrs
            .iter()
            .find(|(k, _)| k == "fact")
            .map(|(_, v)| v.clone());
        let mut text = String::new();
        if !was_empty {
            loop {
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            "unexpected end of input — <fence> never closed",
                        ));
                    }
                    Some(Ev::End(n)) if n == "fence" => {
                        self.i += 1;
                        break;
                    }
                    Some(Ev::Text(t)) => {
                        text.push_str(t);
                        self.i += 1;
                    }
                    Some(Ev::CData(c)) => {
                        text.push_str(c);
                        self.i += 1;
                    }
                    Some(other) => {
                        let at = self.poss[self.i];
                        return Err(self.err(
                            at,
                            format!(
                                "a <fence> holds only text and CDATA — found {}",
                                kind(other)
                            ),
                        ));
                    }
                }
            }
        }
        Ok(Block::Fence { lang, fact, text })
    }

    /// One unit-bearing leaf (`p`, `item`, `quote`, `td`): bare text, or
    /// exactly one wrapping `<fact>`. `allow_empty` permits the empty
    /// table cell; `in_cell` permits the id-less marked cell.
    fn unit(&mut self, tag: &str, was_empty: bool, in_cell: bool) -> Result<Unit> {
        let mut text = String::new();
        let mut fact: Option<Fact> = None;
        if !was_empty {
            loop {
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            format!("unexpected end of input — <{tag}> never closed"),
                        ));
                    }
                    Some(Ev::End(n)) if n == tag => {
                        self.i += 1;
                        break;
                    }
                    Some(Ev::Text(t)) => {
                        if t.trim().is_empty() {
                            // whitespace around a fact (or an empty leaf)
                            self.i += 1;
                            continue;
                        }
                        if fact.is_some() {
                            let at = self.poss[self.i];
                            return Err(self.err(
                                at,
                                format!(
                                    "text beside <fact> inside <{tag}> — a {tag} is ONE unit: bare text or one fact"
                                ),
                            ));
                        }
                        text.push_str(t);
                        self.i += 1;
                    }
                    Some(Ev::CData(_)) => {
                        let at = self.poss[self.i];
                        return Err(self.err(at, "CDATA is allowed only inside <fence>".into()));
                    }
                    Some(Ev::Start(n, a)) if n == "fact" && fact.is_none() => {
                        if !text.trim().is_empty() {
                            let at = self.poss[self.i];
                            return Err(self.err(
                                at,
                                format!(
                                    "text and <fact> cannot mix inside <{tag}> — a {tag} is ONE unit"
                                ),
                            ));
                        }
                        let at = self.poss[self.i];
                        let attrs = a.clone();
                        self.i += 1;
                        let (f, content) = self.fact(&attrs, at, in_cell, false)?;
                        fact = Some(f);
                        text = content;
                    }
                    Some(Ev::Empty(n, a)) if n == "fact" && fact.is_none() => {
                        if !text.trim().is_empty() {
                            let at = self.poss[self.i];
                            return Err(self.err(
                                at,
                                format!(
                                    "text and <fact> cannot mix inside <{tag}> — a {tag} is ONE unit"
                                ),
                            ));
                        }
                        let at = self.poss[self.i];
                        let attrs = a.clone();
                        self.i += 1;
                        let (f, content) = self.fact(&attrs, at, in_cell, true)?;
                        fact = Some(f);
                        text = content;
                    }
                    Some(other) => {
                        let at = self.poss[self.i];
                        return Err(self.err(
                            at,
                            format!("the dialect has no {} (inside <{tag}>)", kind(other)),
                        ));
                    }
                }
            }
        }
        let text = text.trim().to_string();
        if tag == "td" && (text.contains('|') || text.contains('\n')) {
            let at = self.poss[self.i.saturating_sub(1)];
            return Err(self.err(
                at,
                "a <td> cannot hold `|` or a newline — the Markdown table form cannot express it"
                    .into(),
            ));
        }
        if let Some(f) = &fact {
            if !f.is_meaningful() {
                let at = self.poss[self.i.saturating_sub(1)];
                return Err(self.err(
                    at,
                    "an empty <fact> carries nothing — give it an id or a status".into(),
                ));
            }
            return Ok(Unit {
                fact: Some(f.clone()),
                text,
            });
        }
        if text.is_empty() && !in_cell {
            let at = self.poss[self.i.saturating_sub(1)];
            return Err(self.err(
                at,
                format!("an empty <{tag}> — the Markdown form cannot express it"),
            ));
        }
        Ok(Unit { fact: None, text })
    }

    /// One `<fact>`: the closed attribute set, then text-only content.
    /// Returns the fact and its text (the unit's own text lives INSIDE the
    /// fact element). Consumes through `</fact>`.
    fn fact(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        in_cell: bool,
        was_empty: bool,
    ) -> Result<(Fact, String)> {
        only_attrs(
            attrs,
            &[
                "id",
                "status",
                "action",
                "actionstage",
                "audience",
                "comment",
                "ref",
            ],
            "fact",
            at,
            self,
        )?;
        let id = attrs
            .iter()
            .find(|(k, _)| k == "id")
            .map(|(_, v)| v.clone());
        if id.is_none() && !in_cell {
            return Err(self.err(
                at,
                "a <fact> needs an `id` — only a table cell may be marked without one (the cell exemption)".into(),
            ));
        }
        let status = match attrs.iter().find(|(k, _)| k == "status") {
            Some(_) => Some(status_from_attrs(attrs, at, self)?),
            None => None,
        };
        if let Some(id) = &id {
            self.mint(id.clone(), at)?;
        }
        let mut text = String::new();
        if !was_empty {
            loop {
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            "unexpected end of input — <fact> never closed",
                        ));
                    }
                    Some(Ev::End(n)) if n == "fact" => {
                        self.i += 1;
                        break;
                    }
                    Some(Ev::Text(t)) => {
                        text.push_str(t);
                        self.i += 1;
                    }
                    Some(other) => {
                        let at2 = self.poss[self.i];
                        return Err(self.err(
                            at2,
                            format!("a <fact> holds only text — found {}", kind(other)),
                        ));
                    }
                }
            }
        }
        Ok((Fact { id, status }, text))
    }

    /// The text of a bare-text leaf (`<title>`): text-only content,
    /// verbatim (a title keeps its spacing; it is one line by nature).
    pub(super) fn leaf_text(&mut self, tag: &str, was_empty: bool) -> Result<String> {
        let mut text = String::new();
        if !was_empty {
            loop {
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            format!("unexpected end of input — <{tag}> never closed"),
                        ));
                    }
                    Some(Ev::End(n)) if n == tag => {
                        self.i += 1;
                        break;
                    }
                    Some(Ev::Text(t)) => {
                        text.push_str(t);
                        self.i += 1;
                    }
                    Some(other) => {
                        let at = self.poss[self.i];
                        return Err(self.err(
                            at,
                            format!("a <{tag}> holds only text — found {}", kind(other)),
                        ));
                    }
                }
            }
        }
        Ok(text.trim().to_string())
    }

    /// The `<status>` element: stage and state required, the closed
    /// extras, no children.
    pub(super) fn status_element(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<StatusEl> {
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
            self,
        )?;
        if !attrs.iter().any(|(k, _)| k == "stage") || !attrs.iter().any(|(k, _)| k == "state") {
            return Err(self.err(
                at,
                "the <status> element requires both `stage` and `state`".into(),
            ));
        }
        let el = status_from_attrs(attrs, at, self)?;
        if !was_empty {
            match self.evs.get(self.i) {
                Some(Ev::End(n)) if n == "status" => self.i += 1,
                Some(other) => {
                    let at2 = self.poss[self.i];
                    return Err(self.err(
                        at2,
                        format!("a <status> element is empty — found {}", kind(other)),
                    ));
                }
                None => {
                    return Err(Error::at(
                        0,
                        "unexpected end of input — <status> never closed",
                    ));
                }
            }
        }
        Ok(el)
    }
}
