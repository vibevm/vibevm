//! The one shared id namespace: the title anchor, section ids and fact
//! ids all mint into it, and a second definition of the same id is an
//! error naming both lines (the DuplicateId law, progress-core's message
//! verbatim).
//!
//! Minting and checking are one cell because they are one invariant: a
//! mint that did not reach the check would be an id nobody validated.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use super::xml_in::Parser;
use crate::{Error, Result};

impl<'a> Parser<'a> {
    pub(super) fn mint_heading(&mut self, id: String, at: (usize, usize)) -> Result<()> {
        if id.is_empty() || id.trim() != id || id.contains(['}', '\r', '\n']) {
            return Err(self.err(
                at,
                format!(
                    "id `{id}` is not a Markdown-expressible heading anchor — use a non-empty single-line value without `}}`"
                ),
            ));
        }
        self.ids.push((id, at));
        Ok(())
    }

    pub(super) fn mint_fact(&mut self, id: String, at: (usize, usize)) -> Result<()> {
        let spelling = format!("@fact:{id} body");
        let (parsed, _) = progress_core::parse::take_fact_id(&spelling, 0, spelling.len());
        if parsed.as_deref() != Some(id.as_str()) {
            return Err(self.err(
                at,
                format!(
                    "id `{id}` is not a Markdown-expressible fact/anchor id — use the shared progress-core anchor grammar"
                ),
            ));
        }
        self.ids.push((id, at));
        Ok(())
    }

    pub(super) fn check_ids(&mut self) -> Result<()> {
        let mut seen: Vec<&(String, (usize, usize))> = Vec::new();
        for def in &self.ids {
            if let Some((_, first)) = seen.iter().find(|(s, _)| s.as_str() == def.0.as_str()) {
                return Err(Error::at(
                    def.1.0,
                    format!(
                        "fact id `@fact:{}` is defined twice in this file: lines {} and {}",
                        def.0, first.0, def.1.0
                    ),
                ));
            }
            seen.push(def);
        }
        Ok(())
    }
}
