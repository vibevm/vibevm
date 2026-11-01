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

use crate::output::language::{Role, paint, rule};
use vibe_wire::generated::remap_listing as wire;
use vibe_wire::generated::remap_offer as offer_doc;

/// The width the indented labels are padded to, so `to`, `why`, `scope`,
/// `from` and `note` line up down the block.
const LABEL: usize = 6;

/// Where a row's labels start: under its key kind, past `  N. `.
const UNDER: usize = 5;

/// A library sentence as human output: identifiers lose their backticks
/// (PROP-067 `#inline`).
fn prose(text: &str) -> String {
    text.replace('`', "")
}

/// `vibe remap list` (`PROP-066#display`).
pub(super) fn listing(document: &wire::RemapListing) -> String {
    let mut out = String::new();
    if document.sources.is_empty() {
        out.push_str("no remap rows are active\n");
    }
    // PROP-067 `#heading`: each source opens with a rule naming it, its
    // layer, its policy and its state.
    for source in &document.sources {
        let state = source
            .state
            .as_ref()
            .map(|state| format!(", {}", package_state(state)))
            .unwrap_or_default();
        let label = format!(
            "{}  {}, unmatched = {}{state}",
            source.source,
            layer(&source.layer),
            policy(&source.policy)
        );
        out.push_str(&rule(&label, None));
        out.push_str("\n\n");
        if source.rows.is_empty() {
            out.push_str(&format!("  {}\n", paint(Role::Dim, "no rows")));
        }
        for row in &source.rows {
            out.push_str(&row_block(row));
        }
        out.push('\n');
    }
    out.push_str(&footer(document));
    out.trim_end().to_owned()
}

/// A row's numbered gutter line: `N. <kind>  <what>`, the key in the
/// accent role, a deny row's key in the alert role.
fn numbered(number: impl std::fmt::Display, kind: &str, what: &str, deny: bool) -> String {
    let role = if deny { Role::Alert } else { Role::Accent };
    format!("  {number}. {kind}  {}\n", paint(role, what))
}

/// One numbered row block: `N. <key kind>  <what>`, then the indented
/// labels the row carries.
fn row_block(row: &wire::Row) -> String {
    let deny = row.to == "deny";
    let mut out = numbered(row.number, kind(&row.kind), &row.what, deny);
    out.push_str(&labelled(
        "to",
        &format!(
            "{}  {}",
            row.to,
            paint(
                Role::Dim,
                &format!("[{}, {}]", row.scheme, reach(&row.reach))
            )
        ),
    ));
    if let Some(why) = &row.why {
        out.push_str(&labelled("why", &prose(why)));
    }
    if let Some(scope) = &row.scope {
        out.push_str(&labelled("scope", &scope_text(scope)));
    }
    if let Some(from) = &row.from {
        out.push_str(&labelled("from", from));
    }
    for note in &row.notes {
        out.push_str(&labelled("note", &paint(Role::Warn, note_text(note))));
    }
    // Which packages offer this row (`PROP-066#providers`): one name reads
    // differently from three, and uninstall's question is decided on
    // exactly this list.
    if !row.providers.is_empty() {
        out.push_str(&labelled("by", &row.providers.join(", ")));
    }
    if let Some(probed) = &row.probed {
        out.push_str(&labelled("probed", probed));
    }
    out
}

/// The word a package source's state prints as.
fn package_state(state: &wire::PackageState) -> &'static str {
    match state {
        wire::PackageState::Offered => "offered",
        wire::PackageState::Accepted => "accepted",
        wire::PackageState::Rejected => "rejected",
        wire::PackageState::Disabled => "disabled",
    }
}

/// `vibe remap explain` (`PROP-066#precedence`): the declared origin, the
/// matching rows in winning order, the chosen target and the digest.
pub(super) fn explain(document: &explanation::RemapExplanation) -> String {
    let mut out = format!("{}\n\n", rule(&format!("{} fetch", document.kind), None));
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
        out.push_str(&numbered(
            found.order,
            explain_kind(&found.kind),
            &found.what,
            found.to == "deny",
        ));
        out.push_str(&labelled(
            "to",
            &format!(
                "{}  {}",
                found.to,
                paint(
                    Role::Dim,
                    &format!("[{}, {}]", found.scheme, explain_reach(&found.reach))
                )
            ),
        ));
        out.push_str(&labelled(
            "from",
            &format!(
                "row {} of {} ({})",
                found.index + 1,
                found.source,
                explain_layer(&found.layer)
            ),
        ));
        if let Some(why) = &found.why {
            out.push_str(&labelled("why", &prose(why)));
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
            out.push_str(&labelled("note", &paint(Role::Warn, explain_note(note))));
        }
    }
    out.push('\n');
    match (&document.chosen, document.denied) {
        (_, true) => out.push_str(&format!(
            "  {}\n",
            paint(Role::Alert, "refused before any connection (remap_denied)")
        )),
        (Some(chosen), false) => out.push_str(&format!(
            "  {}  {}\n",
            paint(Role::Dim, "chosen"),
            paint(Role::Accent, chosen)
        )),
        (None, false) => out.push_str("  nothing to fetch\n"),
    }
    out.trim_end().to_owned()
}

/// One indented `label   value` line under a row's key kind, the label
/// dim, a long value wrapped under its label rather than cut.
fn labelled(label: &str, value: &str) -> String {
    let indent = " ".repeat(UNDER + LABEL + 2);
    let wrapped = value.replace('\n', &format!("\n{indent}"));
    format!(
        "{}{}  {wrapped}\n",
        " ".repeat(UNDER),
        paint(Role::Dim, &format!("{label:<LABEL$}"))
    )
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
    format!("{}\n", parts.join(" · "))
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

/// The consent question and the offer document, in the same layout and
/// the same field order as `list` (`PROP-066#display`,
/// `PROP-066#display-packages`): a header block carrying the package's
/// key, version, description and state, one numbered block per row, and
/// the per-host footer.
pub(super) fn offer(document: &offer_doc::RemapOffer) -> String {
    let mut out = String::new();
    for (index, package) in document.packages.iter().enumerate() {
        // The packages are the body of the question's block (PROP-067
        // `#question`): each one starts at column 2, a blank line apart.
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "  {}@{}  {}\n",
            paint(Role::Accent, &package.package),
            package.version,
            paint(Role::Dim, &format!("[{}]", offer_state(&package.state)))
        ));
        if let Some(description) = &package.description {
            out.push_str(&labelled("about", description));
        }
        if let Some(registry) = &package.registry {
            out.push_str(&labelled("from", registry));
        }
        if let Some(policy) = &package.policy {
            out.push_str(&labelled("policy", &format!("unmatched = {policy}")));
        }
        out.push_str(&labelled("digest", &package.content_hash));
        if package.rows.is_empty() {
            out.push_str(&format!("  {}\n", paint(Role::Dim, "no rows")));
        }
        for row in &package.rows {
            out.push_str(&offer_row_block(row));
        }
        if !package.hosts.is_empty() {
            out.push_str(&format!("  {}\n", package.hosts.join("; ")));
        }
    }
    out
}

/// One numbered row block of the question.
fn offer_row_block(row: &offer_doc::OfferedRow) -> String {
    let mut out = numbered(
        row.number,
        offer_kind_label(&row.kind),
        &row.what,
        row.to == "deny",
    );
    out.push_str(&labelled(
        "to",
        &format!(
            "{}  {}",
            row.to,
            paint(
                Role::Dim,
                &format!("[{}, {}]", row.scheme, offer_reach_label(&row.reach))
            )
        ),
    ));
    if let Some(why) = &row.why {
        out.push_str(&labelled("why", &prose(why)));
    }
    if let Some(scope) = &row.scope {
        out.push_str(&labelled("scope", scope));
    }
    for note in &row.notes {
        out.push_str(&labelled("note", &paint(Role::Warn, offer_note_text(note))));
    }
    out
}

/// The offer document's own spelling of a key kind — the same four words
/// `list` prints, in the vocabulary the offer schema generated.
pub(super) fn offer_kind(key: &vibe_core::remap::RemapKey) -> offer_doc::KeyKind {
    use vibe_core::remap::RemapKey;
    match key {
        RemapKey::Sha256(_) => offer_doc::KeyKind::Sha256,
        RemapKey::Package(_) => offer_doc::KeyKind::Package,
        RemapKey::Registry(_) => offer_doc::KeyKind::Registry,
        RemapKey::Prefix(_) => offer_doc::KeyKind::Prefix,
    }
}

/// The offer document's spelling of a reach.
pub(super) fn offer_reach(reach: wire::Reach) -> offer_doc::Reach {
    match reach {
        wire::Reach::Local => offer_doc::Reach::Local,
        wire::Reach::Private => offer_doc::Reach::Private,
        wire::Reach::Public => offer_doc::Reach::Public,
        wire::Reach::Unknown => offer_doc::Reach::Unknown,
    }
}

/// The offer document's spelling of a note.
pub(super) fn offer_note(note: wire::Note) -> offer_doc::Note {
    match note {
        wire::Note::Unencrypted => offer_doc::Note::Unencrypted,
        wire::Note::Broad => offer_doc::Note::Broad,
        wire::Note::Unknown => offer_doc::Note::Unknown,
    }
}

fn offer_kind_label(kind: &offer_doc::KeyKind) -> &'static str {
    match kind {
        offer_doc::KeyKind::Sha256 => "sha256  ",
        offer_doc::KeyKind::Package => "package ",
        offer_doc::KeyKind::Registry => "registry",
        offer_doc::KeyKind::Prefix => "prefix  ",
    }
}

fn offer_reach_label(reach: &offer_doc::Reach) -> &'static str {
    match reach {
        offer_doc::Reach::Local => "local",
        offer_doc::Reach::Private => "private",
        offer_doc::Reach::Public => "public",
        offer_doc::Reach::Unknown => "unknown",
    }
}

fn offer_note_text(note: &offer_doc::Note) -> &'static str {
    match note {
        offer_doc::Note::Unencrypted => {
            "the target is plain http — bytes cross the network unencrypted"
        }
        offer_doc::Note::Broad => "a prefix row over a public host rewrites every URL under it",
        offer_doc::Note::Unknown => "the target could not be classified from here",
    }
}

/// The word an offer's state prints as.
pub(super) fn offer_state(state: &offer_doc::OfferState) -> &'static str {
    match state {
        offer_doc::OfferState::Offered => "offered",
        offer_doc::OfferState::Accepted => "accepted",
        offer_doc::OfferState::Rejected => "rejected",
        offer_doc::OfferState::Disabled => "disabled",
    }
}
