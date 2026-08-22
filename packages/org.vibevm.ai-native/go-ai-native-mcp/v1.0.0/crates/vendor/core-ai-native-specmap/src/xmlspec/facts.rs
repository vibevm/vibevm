//! Fact recognition and descent for the closed XML dialect: the
//! generic `<fact id>` wrapper and the named `<FACT-ID fact="true">`
//! form (PROP-045 ##NAMED-FACT-ELEMENTS), split from `blocks` along
//! the file-length seam.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use super::blocks::status_from_attrs;
use super::reader::{Ev, Parser, Violation, attr, only_attrs};
use super::{XBlock, XFact, XUnit};
use specmark_grammar::is_valid_fact_id;

pub(super) fn facts_block(
    p: &mut Parser,
    attrs: &[(String, String)],
    at: usize,
    was_empty: bool,
) -> Result<XBlock, Violation> {
    only_attrs(attrs, &["ordered"], "facts", at)?;
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
                "the <facts> element requires an `ordered` attribute (\"true\"|\"false\")",
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
                        "unexpected end of input — <facts> never closed",
                    ));
                }
                Some(Ev::End(name)) if name == "facts" => {
                    p.i += 1;
                    break;
                }
                Some(_) => {}
            }
            let (name, child_attrs, child_at, child_empty) = p.take_start()?;
            if !is_fact_element(&name, &child_attrs, child_at)? {
                return Err(Violation::at(
                    child_at,
                    format!(
                        "<facts> accepts only fact elements; mixed lists stay in <list> — found <{name}>"
                    ),
                ));
            }
            let (fact, text) = fact_element(p, &name, &child_attrs, child_at, false, child_empty)?;
            items.push(XUnit {
                fact: Some(fact),
                text: text.trim().to_string(),
            });
        }
    }
    if items.is_empty() {
        return Err(Violation::at(
            at,
            "a <facts> needs at least one fact — an empty list is not Markdown-expressible",
        ));
    }
    Ok(XBlock::List { ordered, items })
}

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

#[cfg(test)]
mod tests {
    use super::super::parse_units;

    const NS: &str = "project";

    #[test]
    fn facts_group_gives_the_same_units_as_legacy_list_carriers() {
        let facts = concat!(
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
            "  <title id=\"root\">T</title>\n",
            "  <facts ordered=\"false\">\n",
            "    <FIRST fact=\"true\" status=\"impl/done\">first</FIRST>\n",
            "    <SECOND fact=\"true\" status=\"spec/work\">second</SECOND>\n",
            "  </facts>\n",
            "</spec>\n",
        );
        let list = concat!(
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
            "  <title id=\"root\">T</title>\n",
            "  <list ordered=\"false\">\n",
            "    <item><FIRST fact=\"true\" status=\"impl/done\">first</FIRST></item>\n",
            "    <item><SECOND fact=\"true\" status=\"spec/work\">second</SECOND></item>\n",
            "  </list>\n",
            "</spec>\n",
        );
        let (facts_units, facts_warnings) = parse_units("spec/T.xml", facts, NS);
        let (list_units, list_warnings) = parse_units("spec/T.xml", list, NS);
        assert!(facts_warnings.is_empty());
        assert!(list_warnings.is_empty());
        assert_eq!(facts_units.len(), list_units.len());
        for (facts_unit, list_unit) in facts_units.iter().zip(list_units.iter()) {
            assert_eq!(facts_unit.uri, list_unit.uri);
            assert_eq!(facts_unit.anchor, list_unit.anchor);
            assert_eq!(facts_unit.heading, list_unit.heading);
            assert_eq!(facts_unit.contentHash, list_unit.contentHash);
        }
        assert!(!super::super::doc::anchor_is_elementable("facts"));
    }

    #[test]
    fn non_fact_inside_facts_group_is_a_dialect_warning() {
        let xml = concat!(
            "<spec xmlns=\"https://vibevm.org/spec/1\">\n",
            "  <facts ordered=\"false\"><p>plain</p></facts>\n",
            "</spec>\n",
        );
        let (units, warnings) = parse_units("spec/T.xml", xml, NS);
        assert!(units.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "xml-dialect");
        assert!(warnings[0].message.contains("mixed lists stay in <list>"));
    }
}
