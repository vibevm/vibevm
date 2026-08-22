//! Fact-element recognition and `<facts>` group descent.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::{Ev, Parser};
use super::xml_support::{kind, only_attrs, status_from_attrs};
use crate::doc::{Block, Fact, Unit};
use crate::{Error, Result};

impl<'a> Parser<'a> {
    pub(super) fn facts_block(
        &mut self,
        attrs: &[(String, String)],
        at: (usize, usize),
        was_empty: bool,
    ) -> Result<Block> {
        only_attrs(attrs, &["ordered"], "facts", at, self)?;
        let Some((_, value)) = attrs.iter().find(|(key, _)| key == "ordered") else {
            return Err(self.err(
                at,
                "the <facts> element requires an `ordered` attribute (\"true\"|\"false\")".into(),
            ));
        };
        let ordered = match value.as_str() {
            "true" => true,
            "false" => false,
            other => {
                return Err(self.err(
                    at,
                    format!("the `ordered` attribute is \"true\"|\"false\", found `{other}`"),
                ));
            }
        };
        let mut items = Vec::new();
        if !was_empty {
            loop {
                self.skip_ws_text()?;
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            "unexpected end of input — <facts> never closed",
                        ));
                    }
                    Some(Ev::End(name)) if name == "facts" => {
                        self.i += 1;
                        break;
                    }
                    Some(_) => {}
                }
                let (name, child_attrs, child_at, child_empty) = self.take_start()?;
                if !self.is_fact_element(&name, &child_attrs, child_at)? {
                    return Err(self.err(
                        child_at,
                        format!(
                            "<facts> accepts only fact elements; mixed lists stay in <list> — found <{name}>"
                        ),
                    ));
                }
                let (fact, text) = self.fact(&name, &child_attrs, child_at, false, child_empty)?;
                items.push(Unit {
                    fact: Some(fact),
                    text: text.trim().to_string(),
                });
            }
        }
        if items.is_empty() {
            return Err(self.err(
                at,
                "a <facts> needs at least one fact — an empty list is not Markdown-expressible"
                    .into(),
            ));
        }
        Ok(Block::List { ordered, items })
    }

    pub(super) fn is_fact_element(
        &self,
        element_name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
    ) -> Result<bool> {
        let discriminator = attrs
            .iter()
            .find(|(key, _)| key == "fact")
            .map(|(_, value)| value.as_str());
        if let Some(value) = discriminator
            && value != "true"
        {
            return Err(self.err(
                at,
                format!(
                    "the `fact` discriminator on <{element_name}> must be \"true\", found `{value}`"
                ),
            ));
        }
        Ok(element_name == "fact" || discriminator == Some("true"))
    }

    /// One generic `<fact>` or named fact element: the closed attribute set,
    /// then text-only content. Returns the fact and its text (the unit's own
    /// text lives INSIDE the element). Consumes through the matching end tag.
    pub(super) fn fact(
        &mut self,
        element_name: &str,
        attrs: &[(String, String)],
        at: (usize, usize),
        in_cell: bool,
        was_empty: bool,
    ) -> Result<(Fact, String)> {
        let named = element_name != "fact";
        let allowed = if named {
            &[
                "fact",
                "status",
                "action",
                "actionstage",
                "audience",
                "comment",
                "ref",
            ][..]
        } else {
            &[
                "id",
                "fact",
                "status",
                "action",
                "actionstage",
                "audience",
                "comment",
                "ref",
            ][..]
        };
        only_attrs(attrs, allowed, element_name, at, self)?;
        if named && !super::xml_out::anchor_is_elementable(element_name) {
            return Err(self.err(
                at,
                format!(
                    "named fact <{element_name}> is not elementable — use <fact id=\"{element_name}\">"
                ),
            ));
        }
        let id = if named {
            Some(element_name.to_string())
        } else {
            attrs
                .iter()
                .find(|(key, _)| key == "id")
                .map(|(_, value)| value.clone())
        };
        if id.is_none() && !in_cell {
            return Err(self.err(
                at,
                "a <fact> needs an `id` — only a table cell may be marked without one (the cell exemption)".into(),
            ));
        }
        let status = match attrs.iter().find(|(key, _)| key == "status") {
            Some(_) => Some(status_from_attrs(attrs, at, self)?),
            None => None,
        };
        if let Some(id) = &id {
            self.mint_fact(id.clone(), at)?;
        }
        let mut text = String::new();
        if !was_empty {
            loop {
                match self.evs.get(self.i) {
                    None => {
                        return Err(Error::at(
                            0,
                            format!("unexpected end of input — <{element_name}> never closed"),
                        ));
                    }
                    Some(Ev::End(name)) if name == element_name => {
                        self.i += 1;
                        break;
                    }
                    Some(Ev::Text(value)) => {
                        text.push_str(value);
                        self.i += 1;
                    }
                    Some(other) => {
                        let child_at = self.poss[self.i];
                        return Err(self.err(
                            child_at,
                            format!(
                                "a <{element_name}> fact holds only text — found {}",
                                kind(other)
                            ),
                        ));
                    }
                }
            }
        }
        Ok((Fact { id, status }, text))
    }
}
