//! Query tokenisation for `vibe search` — the free-text query a user
//! types becomes the token list the full-scan scores against, the same
//! lowercase-alnum / drop-stopwords / drop-short rule the index server
//! applies, kept here so the org-walk fallback need not pull the
//! server-side index crate.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

/// Tokenise a free-text query the same way the server does:
/// lowercase ASCII alphanumeric runs, drop tokens shorter than 2
/// characters, drop trivial English stopwords.
///
/// ```
/// use vibe_registry::search::query::tokenise_query;
///
/// // Alphanumeric runs are lowercased; punctuation splits tokens.
/// assert_eq!(tokenise_query("WAL log-store"), vec!["wal", "log", "store"]);
/// // Sub-two-character tokens and stopwords drop out.
/// assert_eq!(tokenise_query("the a WAL"), vec!["wal"]);
/// ```
pub fn tokenise_query(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is",
        "it", "its", "of", "on", "or", "she", "that", "the", "this", "to", "was", "were", "with",
        "you", "your",
    ];
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in query.chars() {
        if c.is_ascii_alphanumeric() {
            buf.push(c.to_ascii_lowercase());
        } else if !buf.is_empty() {
            push_if_keepable(&mut out, std::mem::take(&mut buf), STOPWORDS);
        }
    }
    if !buf.is_empty() {
        push_if_keepable(&mut out, buf, STOPWORDS);
    }
    out
}

fn push_if_keepable(out: &mut Vec<String>, tok: String, stopwords: &[&str]) {
    if tok.len() < 2 {
        return;
    }
    if stopwords.contains(&tok.as_str()) {
        return;
    }
    out.push(tok);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_query_drops_stopwords_and_short_tokens() {
        let toks = tokenise_query("the WAL log discipline");
        assert!(!toks.contains(&"the".into()));
        assert!(toks.contains(&"wal".into()));
        assert!(toks.contains(&"log".into()));
        assert!(toks.contains(&"discipline".into()));
    }
}
