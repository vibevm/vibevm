//! The one human layout every remap view uses (`PROP-066#display`).
//!
//! A header block per source, one numbered block per row, and a footer
//! summarising targets per host. Nothing is abbreviated to fit: a long
//! target wraps under its label rather than being cut, and `--json` is the
//! escape for machines. The same fields appear in the same order here and
//! in the `vibe-remap/1` documents, so a graphical client renders the same
//! blocks.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#display");

use vibe_wire::generated::remap_explanation as explanation;
use vibe_wire::generated::remap_listing as wire;

/// The width the indented labels are padded to, so `to`, `why`, `scope`,
/// `from` and `note` line up down the block.
const LABEL: usize = 7;

/// `vibe remap list` (`PROP-066#display`).
pub(super) fn listing(document: &wire::RemapListing) -> String {
    let mut out = String::new();
    if document.sources.is_empty() {
        out.push_str("no remap rows are active\n");
    }
    for source in &document.sources {
        out.push_str(&format!(
            "{}  [{}, unmatched = {}]\n",
            source.source,
            layer(&source.layer),
            policy(&source.policy)
        ));
        if source.rows.is_empty() {
            out.push_str("  (no rows)\n");
        }
        for row in &source.rows {
            out.push_str(&row_block(row));
        }
        out.push('\n');
    }
    out.push_str(&footer(document));
    out
}

/// One numbered row block: `N. <key kind>  <what>`, then the indented
/// labels the row carries.
fn row_block(row: &wire::Row) -> String {
    let mut out = format!("  {}. {}  {}\n", row.number, kind(&row.kind), row.what);
    out.push_str(&labelled(
        "to",
        &format!("{}  [{}, {}]", row.to, row.scheme, reach(&row.reach)),
    ));
    if let Some(why) = &row.why {
        out.push_str(&labelled("why", why));
    }
    if let Some(scope) = &row.scope {
        out.push_str(&labelled("scope", &scope_text(scope)));
    }
    if let Some(from) = &row.from {
        out.push_str(&labelled("from", from));
    }
    for note in &row.notes {
        out.push_str(&labelled("note", note_text(note)));
    }
    out
}

/// `vibe remap explain` (`PROP-066#precedence`): the declared origin, the
/// matching rows in winning order, the chosen target and the digest.
pub(super) fn explain(document: &explanation::RemapExplanation) -> String {
    let mut out = format!("{} fetch\n", document.kind);
    out.push_str(&labelled(
        "origin",
        document.origin.as_deref().unwrap_or("(none declared)"),
    ));
    out.push_str(&labelled(
        "digest",
        document.digest.as_deref().unwrap_or("(none declared)"),
    ));
    out.push_str(&labelled("policy", explain_policy(&document.policy)));
    out.push('\n');
    if document.matches.is_empty() {
        out.push_str("  no remap row matches this fetch\n");
    }
    for found in &document.matches {
        out.push_str(&format!(
            "  {}. {}  {}\n",
            found.order,
            explain_kind(&found.kind),
            found.what
        ));
        out.push_str(&labelled(
            "to",
            &format!(
                "{}  [{}, {}]",
                found.to,
                found.scheme,
                explain_reach(&found.reach)
            ),
        ));
        out.push_str(&labelled(
            "from",
            &format!(
                "row {} of `{}` ({})",
                found.index + 1,
                found.source,
                explain_layer(&found.layer)
            ),
        ));
        if let Some(why) = &found.why {
            out.push_str(&labelled("why", why));
        }
        if let Some(scope) = &found.scope {
            out.push_str(&labelled(
                "scope",
                &scope_text(&wire::Scope {
                    package: scope.package.clone(),
                    step: scope.step.clone(),
                }),
            ));
        }
        for note in &found.notes {
            out.push_str(&labelled("note", explain_note(note)));
        }
    }
    out.push('\n');
    match (&document.chosen, document.denied) {
        (_, true) => out.push_str("  refused before any connection (remap_denied)\n"),
        (Some(chosen), false) => out.push_str(&format!("  chosen  {chosen}\n")),
        (None, false) => out.push_str("  nothing to fetch\n"),
    }
    out
}

/// One indented `label   value` line, with a long value wrapped under its
/// label rather than cut.
fn labelled(label: &str, value: &str) -> String {
    let indent = " ".repeat(4 + LABEL + 2);
    let wrapped = value.replace('\n', &format!("\n{indent}"));
    format!("    {label:<LABEL$}  {wrapped}\n")
}

/// `3 rows → nexus.corp.example (https, private); 1 row → local path;
/// 1 deny`.
fn footer(document: &wire::RemapListing) -> String {
    let mut parts: Vec<String> = document
        .hosts
        .iter()
        .map(|host| {
            let count = format!("{} row{}", host.rows, if host.rows == 1 { "" } else { "s" });
            // The local group has no host and no scheme worth naming —
            // `PROP-066#display`'s own example reads `1 row → local path`.
            if host.reach == wire::Reach::Local {
                return format!("{count} → {}", host.host);
            }
            format!(
                "{count} → {} ({}, {})",
                host.host,
                host.scheme,
                reach(&host.reach)
            )
        })
        .collect();
    if document.denies > 0 {
        parts.push(format!("{} deny", document.denies));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("{}\n", parts.join("; "))
}

fn scope_text(scope: &wire::Scope) -> String {
    match &scope.step {
        Some(step) => format!("{}, step {step}", scope.package),
        None => scope.package.clone(),
    }
}

fn kind(kind: &wire::KeyKind) -> &'static str {
    match kind {
        wire::KeyKind::Sha256 => "sha256  ",
        wire::KeyKind::Package => "package ",
        wire::KeyKind::Registry => "registry",
        wire::KeyKind::Prefix => "prefix  ",
    }
}

fn explain_kind(kind: &explanation::KeyKind) -> &'static str {
    match kind {
        explanation::KeyKind::Sha256 => "sha256  ",
        explanation::KeyKind::Package => "package ",
        explanation::KeyKind::Registry => "registry",
        explanation::KeyKind::Prefix => "prefix  ",
    }
}

fn layer(layer: &wire::Layer) -> &'static str {
    match layer {
        wire::Layer::UserPackage => "user-package",
        wire::Layer::ProjectPackage => "project-package",
        wire::Layer::Local => "local",
        wire::Layer::Invocation => "invocation",
    }
}

fn explain_layer(layer: &explanation::Layer) -> &'static str {
    match layer {
        explanation::Layer::UserPackage => "user-package",
        explanation::Layer::ProjectPackage => "project-package",
        explanation::Layer::Local => "local",
        explanation::Layer::Invocation => "invocation",
    }
}

fn reach(reach: &wire::Reach) -> &'static str {
    match reach {
        wire::Reach::Local => "local",
        wire::Reach::Private => "private",
        wire::Reach::Public => "public",
        wire::Reach::Unknown => "unknown",
    }
}

fn explain_reach(reach: &explanation::Reach) -> &'static str {
    match reach {
        explanation::Reach::Local => "local",
        explanation::Reach::Private => "private",
        explanation::Reach::Public => "public",
        explanation::Reach::Unknown => "unknown",
    }
}

fn policy(policy: &wire::UnmatchedPolicy) -> &'static str {
    match policy {
        wire::UnmatchedPolicy::Allow => "allow",
        wire::UnmatchedPolicy::Deny => "deny",
    }
}

fn explain_policy(policy: &explanation::UnmatchedPolicy) -> &'static str {
    match policy {
        explanation::UnmatchedPolicy::Allow => "allow",
        explanation::UnmatchedPolicy::Deny => "deny",
    }
}

fn note_text(note: &wire::Note) -> &'static str {
    match note {
        wire::Note::Unencrypted => "the target is plain http — bytes cross the network unencrypted",
        wire::Note::Broad => "a prefix row over a public host rewrites every URL under it",
        wire::Note::Unknown => "the target could not be classified from here",
    }
}

fn explain_note(note: &explanation::Note) -> &'static str {
    match note {
        explanation::Note::Unencrypted => {
            "the target is plain http — bytes cross the network unencrypted"
        }
        explanation::Note::Broad => "a prefix row over a public host rewrites every URL under it",
        explanation::Note::Unknown => "the target could not be classified from here",
    }
}
