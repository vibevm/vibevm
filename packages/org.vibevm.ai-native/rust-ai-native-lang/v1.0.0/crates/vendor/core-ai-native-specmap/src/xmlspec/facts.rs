//! Fact recognition and descent for the closed XML dialect: the
//! generic `<fact id>` wrapper and the named `<FACT-ID fact="true">`
//! form (PROP-045 ##NAMED-FACT-ELEMENTS), split from `blocks` along
//! the file-length seam.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use super::XFact;
use super::blocks::status_from_attrs;
use super::reader::{Ev, Parser, Violation, attr, only_attrs};
use specmark_grammar::is_valid_fact_id;

pub(super) fn is_fact_element(
    element_name: &str,
    attrs: &[(String, String)],
    at: usize,
) -> Result<bool, Violation> {
    let discriminator = attr(attrs, "fact");
    if let Some(value) = discriminator
        && value != "true"
    {
        return Err(Violation::at(
            at,
            format!(
                "the `fact` discriminator on <{element_name}> must be \"true\", found `{value}`"
            ),
        ));
    }
    Ok(element_name == "fact" || discriminator == Some("true"))
}

/// One generic `<fact>` or named fact: the closed attribute set, then
/// text-only content. The fact carries the element's native line.
pub(super) fn fact_element(
    p: &mut Parser,
    element_name: &str,
    attrs: &[(String, String)],
    at: usize,
    in_cell: bool,
    was_empty: bool,
) -> Result<(XFact, String), Violation> {
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
    only_attrs(attrs, allowed, element_name, at)?;
    if named && !super::doc::anchor_is_elementable(element_name) {
        return Err(Violation::at(
            at,
            format!(
                "named fact <{element_name}> is not elementable — use <fact id=\"{element_name}\">"
            ),
        ));
    }
    if named && !is_valid_fact_id(element_name) {
        return Err(Violation::at(
            at,
            format!("named fact id `{element_name}` does not match the shared fact-id grammar"),
        ));
    }
    let id = if named {
        Some(element_name.to_string())
    } else {
        attr(attrs, "id").map(str::to_string)
    };
    if id.is_none() && !in_cell {
        return Err(Violation::at(
            at,
            "a <fact> needs an `id` — only a table cell may be marked without one (the cell exemption)",
        ));
    }
    let status = if attr(attrs, "status").is_some() {
        Some(status_from_attrs(attrs, at)?)
    } else {
        None
    };
    let mut text = String::new();
    if !was_empty {
        loop {
            match p.evs.get(p.i) {
                None => {
                    return Err(Violation::at(
                        0,
                        format!("unexpected end of input — <{element_name}> never closed"),
                    ));
                }
                Some(Ev::End(n)) if n == element_name => {
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
                        format!(
                            "a <{element_name}> fact holds only text — found {}",
                            other.what()
                        ),
                    ));
                }
            }
        }
    }
    Ok((
        XFact {
            id,
            status,
            line: at as u32,
        },
        text,
    ))
}
