//! Rendering: one [`Rec`] back to a logfmt line (quotes added when a
//! value contains spaces or quotes).

use crate::parse::Rec;

/// Renders a record to one logfmt line.
pub fn render(rec: &Rec) -> String {
    let mut parts = Vec::new();
    for (k, v) in &rec.pairs {
        if v.contains(' ') || v.contains('"') {
            let escaped = v.replace('"', "\\\"");
            parts.push(format!("{}={:?}", k, escaped).replace("\\\\\"", "\\\""));
        } else {
            parts.push(format!("{}={}", k, v));
        }
    }
    parts.join(" ")
}

/// A one-line human summary: `<n> pairs: k1, k2, …`.
pub fn summary(rec: &Rec) -> String {
    let keys: Vec<String> = rec.pairs.iter().map(|(k, _)| format!("{}", k)).collect();
    format!("{} pairs: {}", rec.pairs.len(), keys.join(", "))
}
