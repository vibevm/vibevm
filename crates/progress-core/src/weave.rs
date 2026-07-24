//! `weave` — stitch the observed corpus into whole-context LLM input
//! (PROP-043 §5): full form with token-budget sharding, or the digest map.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#weave");

use crate::doc::ParsedDoc;

/// Rough token estimate (bytes/4) — deliberate, dependency-free; the
/// budget is a planning aid, not an exact tokenizer.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

pub struct WeaveShard {
    pub index: usize,
    pub body: String,
    pub token_estimate: usize,
    pub files: Vec<String>,
}

/// Full weave: every file verbatim under a `=== path ===` banner, split
/// into shards of at most `max_tokens` (one oversized file still becomes
/// its own shard — never silently truncated, PROP-043 quality law).
pub fn weave_full(files: &[(String, String)], max_tokens: Option<usize>) -> Vec<WeaveShard> {
    let mut shards: Vec<WeaveShard> = Vec::new();
    let mut body = String::new();
    let mut names: Vec<String> = Vec::new();
    let flush = |shards: &mut Vec<WeaveShard>, body: &mut String, names: &mut Vec<String>| {
        if !body.is_empty() {
            shards.push(WeaveShard {
                index: shards.len(),
                token_estimate: estimate_tokens(body),
                body: std::mem::take(body),
                files: std::mem::take(names),
            });
        }
    };
    for (path, text) in files {
        let section = format!("\n\n=== {path} ===\n\n{text}");
        if let Some(budget) = max_tokens
            && !body.is_empty()
            && estimate_tokens(&body) + estimate_tokens(&section) > budget
        {
            flush(&mut shards, &mut body, &mut names);
        }
        body.push_str(&section);
        names.push(path.clone());
    }
    flush(&mut shards, &mut body, &mut names);
    shards
}

/// Digest weave: headings + markers + unmarked counters per file — the
/// "map of the theater" that always fits one window.
pub fn weave_digest<'a>(docs: impl IntoIterator<Item = &'a ParsedDoc>) -> String {
    let mut s = String::from("# Progress digest\n");
    for doc in docs {
        s.push_str(&format!(
            "\n## {} — {} markers, {}/{} facts unmarked\n",
            doc.path,
            doc.markers.len(),
            doc.unmarked_facts.len(),
            doc.fact_count
        ));
        for u in &doc.units {
            let anchor = u
                .anchor
                .as_deref()
                .map(|a| format!(" {{#{a}}}"))
                .unwrap_or_default();
            s.push_str(&format!(
                "{} {}{} (l.{}-{})\n",
                "#".repeat(u.level.min(6)),
                u.heading,
                anchor,
                u.line_start,
                u.line_end
            ));
        }
        for m in &doc.markers {
            s.push_str(&format!(
                "  - l.{} [{:?}] {}/{}{}\n",
                m.line,
                m.granularity,
                m.stage,
                m.state,
                m.action.map(|a| format!(" action={a}")).unwrap_or_default()
            ));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharding_respects_budget_and_never_drops_files() {
        let a = ("a.md".to_string(), "x".repeat(4000));
        let b = ("b.md".to_string(), "y".repeat(4000));
        let c = ("c.md".to_string(), "z".repeat(4000));
        let shards = weave_full(&[a, b, c], Some(1500));
        let all: Vec<&String> = shards.iter().flat_map(|s| s.files.iter()).collect();
        assert_eq!(all.len(), 3, "no file dropped");
        assert!(shards.len() >= 2, "budget forced sharding");
        let unsharded = weave_full(&[("a.md".into(), "hi".into())], None);
        assert_eq!(unsharded.len(), 1);
    }
}
