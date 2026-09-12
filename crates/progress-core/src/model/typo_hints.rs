//! Nearest-legal-value hints for a typo'd token (PROP-043 §3.2).
//!
//! The vocabularies in [`super`] are closed, so a token outside them is
//! an error; this is what the error says next.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#attributes");

/// Nearest legal value for a typo'd token, for `check` hints
/// (PROP-043 §3.2 — "typos like `rewrok` die in CI").
pub fn nearest<'a>(input: &str, legal: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    legal
        .into_iter()
        .map(|cand| (levenshtein(input, cand), cand))
        .filter(|(d, _)| *d <= 3)
        .min_by_key(|(d, _)| *d)
        .map(|(_, cand)| cand)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Action;

    #[test]
    fn nearest_catches_the_famous_typo() {
        assert_eq!(
            nearest("rewrok", Action::ALL.iter().map(|a| a.as_str())),
            Some("rework")
        );
        assert_eq!(
            nearest("zzzzzz", Action::ALL.iter().map(|a| a.as_str())),
            None
        );
    }
}
