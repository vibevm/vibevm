//! Report rendering: XML (native), Markdown table, and the five
//! resolution views (PROP-043 §5 `report`).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#report");

use crate::doc::ParsedDoc;
use crate::model::{Audience, Granularity, Marker, Stage, State};
use crate::rollup::DocRollup;
use serde::{Deserialize, Serialize};

/// The five resolution views — filters over one model (PROP-043 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Done,
    Todo,
    Qa,
    Remove,
    Doc,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        match s {
            "done" => Some(View::Done),
            "todo" => Some(View::Todo),
            "qa" => Some(View::Qa),
            "remove" => Some(View::Remove),
            "doc" => Some(View::Doc),
            _ => None,
        }
    }

    /// The filter predicate of each view.
    pub fn matches(self, m: &Marker) -> bool {
        match self {
            View::Done => m.state == State::Done,
            View::Todo => m.action == Some(crate::model::Action::Continue),
            View::Qa => {
                m.stage == Stage::Test && (m.state == State::Plan || m.state == State::Work)
            }
            View::Remove => m.action == Some(crate::model::Action::Remove),
            View::Doc => m.actionstage == Some(Stage::Doc),
        }
    }
}

/// Does a marker speak to `audience`? Absent audience list ⇒ `dev`.
pub fn audience_matches(m: &Marker, audience: Audience) -> bool {
    if m.audience.is_empty() {
        audience == Audience::Dev
    } else {
        m.audience.contains(&audience)
    }
}

/// One row of the rendered report.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub path: String,
    pub line: usize,
    pub granularity: Granularity,
    pub stage: Stage,
    pub state: State,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

pub fn rows<'a>(
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    view: Option<View>,
    audience: Option<Audience>,
) -> Vec<Row> {
    let mut out = Vec::new();
    for doc in docs {
        for m in &doc.markers {
            if let Some(v) = view
                && !v.matches(m)
            {
                continue;
            }
            if let Some(a) = audience
                && !audience_matches(m, a)
            {
                continue;
            }
            let action = m.actionstage.map_or_else(
                || m.action.map(|a| a.to_string()),
                |ast| m.action.map(|a| format!("{a}+{ast}")),
            );
            out.push(Row {
                path: doc.path.clone(),
                line: m.line,
                granularity: m.granularity,
                stage: m.stage,
                state: m.state,
                action,
                comment: m.comment.clone(),
            });
        }
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The native XML report (PROP-043 §5: XML is the native output).
pub fn render_xml(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<progress-report schema=\"1\">\n",
    );
    s.push_str("  <files>\n");
    for (path, r) in rollups {
        let eff = r
            .effective
            .map(|(st, sta)| format!(" stage=\"{st}\" state=\"{sta}\""))
            .unwrap_or_default();
        s.push_str(&format!(
            "    <file path=\"{}\"{} markers=\"{}\" facts=\"{}\" unmarked=\"{}\"/>\n",
            xml_escape(path),
            eff,
            r.marker_count,
            r.fact_count,
            r.unmarked_facts
        ));
    }
    s.push_str("  </files>\n  <markers>\n");
    for row in rows {
        s.push_str(&format!(
            "    <marker path=\"{}\" line=\"{}\" granularity=\"{:?}\" stage=\"{}\" state=\"{}\"{}{}/>\n",
            xml_escape(&row.path),
            row.line,
            row.granularity,
            row.stage,
            row.state,
            row.action
                .as_deref()
                .map(|a| format!(" action=\"{}\"", xml_escape(a)))
                .unwrap_or_default(),
            row.comment
                .as_deref()
                .map(|c| format!(" comment=\"{}\"", xml_escape(c)))
                .unwrap_or_default(),
        ));
    }
    s.push_str("  </markers>\n</progress-report>\n");
    s
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// The Markdown table render (source · stage · state · action · comment).
pub fn render_md(rows: &[Row], rollups: &[(String, DocRollup)]) -> String {
    let mut s =
        String::from("| source | stage | state | action | comment |\n|---|---|---|---|---|\n");
    for (path, r) in rollups {
        if let Some((st, sta)) = r.effective {
            s.push_str(&format!(
                "| **{}** ({} markers, {}/{} unmarked) | {} | {} |  |  |\n",
                md_escape(path),
                r.marker_count,
                r.unmarked_facts,
                r.fact_count,
                st,
                sta
            ));
        } else {
            s.push_str(&format!(
                "| **{}** (no markers, {} facts) | — | — |  |  |\n",
                md_escape(path),
                r.fact_count
            ));
        }
    }
    for row in rows {
        s.push_str(&format!(
            "| {}:{} | {} | {} | {} | {} |\n",
            md_escape(&row.path),
            row.line,
            row.stage,
            row.state,
            row.action.as_deref().map(md_escape).unwrap_or_default(),
            row.comment.as_deref().map(md_escape).unwrap_or_default(),
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn views_filter_one_model() {
        let text = "\
<status stage=\"impl\" state=\"done\"/>

# T {#t}

<status stage=\"test\" state=\"plan\"/>

Body. <status stage=\"impl\" state=\"work\" action=\"continue\" actionstage=\"doc\" audience=\"user\"/>
";
        let doc = parse_document("x.md", text);
        let all = rows([&doc], None, None);
        assert_eq!(all.len(), 3);
        assert_eq!(rows([&doc], Some(View::Done), None).len(), 1);
        assert_eq!(rows([&doc], Some(View::Qa), None).len(), 1);
        assert_eq!(rows([&doc], Some(View::Todo), None).len(), 1);
        assert_eq!(rows([&doc], Some(View::Doc), None).len(), 1);
        // audience=user matches only the explicitly-user marker.
        assert_eq!(rows([&doc], None, Some(Audience::User)).len(), 1);
        // default audience is dev.
        assert_eq!(rows([&doc], None, Some(Audience::Dev)).len(), 2);
    }
}
