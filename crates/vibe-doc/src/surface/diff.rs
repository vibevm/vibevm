//! Two surfaces compared, and the answer turned into pages
//! (PROP-057 `##OBS-SURFACE-SNAPSHOTS`, `MAINTENANCE.md` §2.5).
//!
//! The comparison itself is set arithmetic over the snapshot's own lists,
//! and it is the boring half. The half that earns the tool is the second
//! step: every change is put through the three relations a documentation
//! package already has with the product it documents —
//!
//! * a page CITES a rule, so a changed obligation reaches the pages whose
//!   `rule` blocks name its address;
//! * a page DERIVES its text from a command, a manifest field or a
//!   schema, so a changed flag reaches the pages whose `derived` blocks
//!   quote that command;
//! * a page OWES an audience a promise, so a NEW obligation nobody cites
//!   is not silence — it is a page that does not exist yet.
//!
//! Anything that reaches none of the three is listed as needing a page.
//! An empty list is an answer as well: it says the contract moved in a
//! way the manual does not describe, which is a thing worth knowing
//! rather than a run that found nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

use std::collections::BTreeMap;
use std::path::Path;

use vibe_specdoc::doc::DerivedKind;
use vibe_wire::generated::doc_surface::{
    DocSurface, SurfaceCommand, SurfaceFact, SurfaceFlag, SurfaceFormat, SurfaceSchema,
    SurfaceSchemaField,
};
use vibe_wire::generated::doc_surface_diff::{
    ChangeKind, ChangeVerb, DocSurfaceDiff, PageToUpdate, SurfaceChange, UnplacedChange,
};

use crate::citations::SpecSources;
use crate::coverage;
use crate::derived;
use crate::error::Result;

/// The schema version a diff written today carries.
pub const SCHEMA_VERSION: u32 = 1;

/// Compare two surfaces and name the pages to update.
///
/// `corpus_root` is the tree the obligations were observed in — an
/// obligation's address is relative to it, exactly as the coverage gate
/// reads one.
pub fn diff(
    old: &DocSurface,
    new: &DocSurface,
    package_dir: &Path,
    coordinate: &str,
    sources: &SpecSources,
    corpus_root: &Path,
) -> Result<DocSurfaceDiff> {
    let changes = compare(old, new);
    let index = Index::read(package_dir, coordinate, sources, corpus_root)?;
    let (pages, needs_pages) = index.place(&changes, new);
    Ok(DocSurfaceDiff {
        schema_version: SCHEMA_VERSION,
        from: old.version.clone(),
        to: new.version.clone(),
        changes,
        pages,
        needs_pages,
    })
}

/// Every difference between two surfaces, in a stable order.
pub fn compare(old: &DocSurface, new: &DocSurface) -> Vec<SurfaceChange> {
    let mut out = Vec::new();
    commands(&mut out, old, new);
    strings(
        &mut out,
        ChangeKind::ManifestField,
        &old.manifest_fields,
        &new.manifest_fields,
    );
    strings(
        &mut out,
        ChangeKind::LockField,
        &old.lock_fields,
        &new.lock_fields,
    );
    schemas(&mut out, old, new);
    facts(&mut out, old, new);
    formats(&mut out, old, new);
    out
}

/// One change, spelled once so every producer below reads the same.
fn change(kind: ChangeKind, verb: ChangeVerb, subject: &str, detail: &str) -> SurfaceChange {
    SurfaceChange {
        kind,
        change: verb,
        subject: subject.to_owned(),
        detail: detail.to_owned(),
    }
}

/// Commands and the flags under them.
fn commands(out: &mut Vec<SurfaceChange>, old: &DocSurface, new: &DocSurface) {
    let before: BTreeMap<&str, &SurfaceCommand> =
        old.commands.iter().map(|c| (c.path.as_str(), c)).collect();
    let after: BTreeMap<&str, &SurfaceCommand> =
        new.commands.iter().map(|c| (c.path.as_str(), c)).collect();
    for (path, command) in &after {
        let Some(was) = before.get(path) else {
            out.push(change(ChangeKind::Command, ChangeVerb::Added, path, ""));
            // Every flag of a new command arrives with it; listing them
            // one by one would bury the command that carries them.
            continue;
        };
        if was.summary != command.summary {
            out.push(change(
                ChangeKind::Command,
                ChangeVerb::Changed,
                path,
                "its summary was rewritten",
            ));
        }
        flags(out, path, was, command);
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            out.push(change(ChangeKind::Command, ChangeVerb::Removed, path, ""));
        }
    }
}

/// The flags of one command that exists on both sides.
fn flags(out: &mut Vec<SurfaceChange>, path: &str, was: &SurfaceCommand, now: &SurfaceCommand) {
    let before: BTreeMap<&str, &SurfaceFlag> =
        was.flags.iter().map(|f| (f.name.as_str(), f)).collect();
    let after: BTreeMap<&str, &SurfaceFlag> =
        now.flags.iter().map(|f| (f.name.as_str(), f)).collect();
    for (name, flag) in &after {
        let subject = format!("{path} {name}");
        match before.get(name) {
            None => out.push(change(ChangeKind::Flag, ChangeVerb::Added, &subject, "")),
            Some(had) if had.value != flag.value => out.push(change(
                ChangeKind::Flag,
                ChangeVerb::Changed,
                &subject,
                &format!("it takes `{}` where it took `{}`", flag.value, had.value),
            )),
            Some(had) if had.summary != flag.summary => out.push(change(
                ChangeKind::Flag,
                ChangeVerb::Changed,
                &subject,
                "its summary was rewritten",
            )),
            Some(_) => {}
        }
    }
    for name in before.keys() {
        if !after.contains_key(name) {
            out.push(change(
                ChangeKind::Flag,
                ChangeVerb::Removed,
                &format!("{path} {name}"),
                "",
            ));
        }
    }
}

/// Two sorted string lists — the manifest and the lock file.
fn strings(out: &mut Vec<SurfaceChange>, kind: ChangeKind, old: &[String], new: &[String]) {
    for field in new {
        if !old.contains(field) {
            out.push(change(kind.clone(), ChangeVerb::Added, field, ""));
        }
    }
    for field in old {
        if !new.contains(field) {
            out.push(change(kind.clone(), ChangeVerb::Removed, field, ""));
        }
    }
}

/// The published schemas, member by member.
fn schemas(out: &mut Vec<SurfaceChange>, old: &DocSurface, new: &DocSurface) {
    let before: BTreeMap<&str, &SurfaceSchema> =
        old.schemas.iter().map(|s| (s.id.as_str(), s)).collect();
    let after: BTreeMap<&str, &SurfaceSchema> =
        new.schemas.iter().map(|s| (s.id.as_str(), s)).collect();
    for (id, schema) in &after {
        let Some(was) = before.get(id) else {
            out.push(change(ChangeKind::Schema, ChangeVerb::Added, id, ""));
            continue;
        };
        let name = |f: &SurfaceSchemaField| {
            if f.form.is_empty() {
                f.name.clone()
            } else {
                format!("{}.{}", f.form, f.name)
            }
        };
        let had: BTreeMap<String, &SurfaceSchemaField> =
            was.fields.iter().map(|f| (name(f), f)).collect();
        let has: BTreeMap<String, &SurfaceSchemaField> =
            schema.fields.iter().map(|f| (name(f), f)).collect();
        for (member, field) in &has {
            match had.get(member) {
                None => out.push(change(
                    ChangeKind::Schema,
                    ChangeVerb::Changed,
                    id,
                    &format!("member `{member}` appeared"),
                )),
                Some(old_field) if old_field.required != field.required => {
                    out.push(change(
                        ChangeKind::Schema,
                        ChangeVerb::Changed,
                        id,
                        &format!(
                            "member `{member}` became {}",
                            if field.required {
                                "required"
                            } else {
                                "optional"
                            }
                        ),
                    ));
                }
                Some(old_field) if old_field.shape != field.shape => out.push(change(
                    ChangeKind::Schema,
                    ChangeVerb::Changed,
                    id,
                    &format!(
                        "member `{member}` is now `{}` where it was `{}`",
                        field.shape, old_field.shape
                    ),
                )),
                Some(_) => {}
            }
        }
        for member in had.keys() {
            if !has.contains_key(member) {
                out.push(change(
                    ChangeKind::Schema,
                    ChangeVerb::Changed,
                    id,
                    &format!("member `{member}` is gone"),
                ));
            }
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            out.push(change(ChangeKind::Schema, ChangeVerb::Removed, id, ""));
        }
    }
}

/// The documentation obligations of the specifications.
fn facts(out: &mut Vec<SurfaceChange>, old: &DocSurface, new: &DocSurface) {
    let before: BTreeMap<&str, &SurfaceFact> =
        old.facts.iter().map(|f| (f.address.as_str(), f)).collect();
    let after: BTreeMap<&str, &SurfaceFact> =
        new.facts.iter().map(|f| (f.address.as_str(), f)).collect();
    for (address, fact) in &after {
        match before.get(address) {
            None => out.push(change(
                ChangeKind::Fact,
                ChangeVerb::Added,
                address,
                &format!("promised to {}", fact.audiences.join(", ")),
            )),
            Some(was) if was.audiences != fact.audiences => out.push(change(
                ChangeKind::Fact,
                ChangeVerb::Changed,
                address,
                &format!(
                    "promised to {} where it was promised to {}",
                    fact.audiences.join(", "),
                    was.audiences.join(", ")
                ),
            )),
            Some(was) if was.text != fact.text => out.push(change(
                ChangeKind::Fact,
                ChangeVerb::Changed,
                address,
                "its text was rewritten",
            )),
            Some(_) => {}
        }
    }
    for address in before.keys() {
        if !after.contains_key(address) {
            out.push(change(ChangeKind::Fact, ChangeVerb::Removed, address, ""));
        }
    }
}

/// The format registry.
fn formats(out: &mut Vec<SurfaceChange>, old: &DocSurface, new: &DocSurface) {
    let before: BTreeMap<&str, &SurfaceFormat> =
        old.formats.iter().map(|f| (f.id.as_str(), f)).collect();
    let after: BTreeMap<&str, &SurfaceFormat> =
        new.formats.iter().map(|f| (f.id.as_str(), f)).collect();
    for (id, format) in &after {
        let Some(was) = before.get(id) else {
            out.push(change(ChangeKind::Format, ChangeVerb::Added, id, ""));
            continue;
        };
        let mut moved: Vec<String> = Vec::new();
        if was.epoch != format.epoch {
            moved.push(format!("epoch {} → {}", was.epoch, format.epoch));
        }
        if was.recoverable != format.recoverable {
            moved.push(format!(
                "recoverable {} → {}",
                was.recoverable, format.recoverable
            ));
        }
        if was.foreign_parsers != format.foreign_parsers {
            moved.push(format!(
                "foreign parsers {} → {}",
                was.foreign_parsers, format.foreign_parsers
            ));
        }
        if was.sunset != format.sunset {
            moved.push(format!("sunset {} → {}", was.sunset, format.sunset));
        }
        if !moved.is_empty() {
            out.push(change(
                ChangeKind::Format,
                ChangeVerb::Changed,
                id,
                &moved.join("; "),
            ));
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            out.push(change(ChangeKind::Format, ChangeVerb::Removed, id, ""));
        }
    }
}

/// The three relations a page has with the product, read once.
struct Index {
    /// `<canonical file>#<anchor>` → the pages that cite it.
    cited: BTreeMap<String, Vec<String>>,
    /// `(page, kind, reference)` for every `derived` block.
    derived: Vec<(String, DerivedKind, String)>,
    corpus_root: std::path::PathBuf,
}

impl Index {
    fn read(
        package_dir: &Path,
        coordinate: &str,
        sources: &SpecSources,
        corpus_root: &Path,
    ) -> Result<Index> {
        Ok(Index {
            cited: coverage::cited_index(package_dir, coordinate, sources)?,
            derived: derived::references(package_dir)?,
            corpus_root: corpus_root.to_path_buf(),
        })
    }

    /// Put every change through the three relations.
    fn place(
        &self,
        changes: &[SurfaceChange],
        new: &DocSurface,
    ) -> (Vec<PageToUpdate>, Vec<UnplacedChange>) {
        let mut reached: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut unplaced: Vec<UnplacedChange> = Vec::new();
        for entry in changes {
            let pages = match entry.kind {
                ChangeKind::Fact => self.citing(&entry.subject),
                ChangeKind::Command | ChangeKind::Flag => self.quoting_command(&entry.subject),
                ChangeKind::ManifestField | ChangeKind::LockField => {
                    self.quoting_field(&entry.subject)
                }
                ChangeKind::Schema | ChangeKind::Format => self.quoting_schema(&entry.subject, new),
            };
            let reason = reason(entry);
            if pages.is_empty() {
                unplaced.push(UnplacedChange {
                    subject: entry.subject.clone(),
                    reason: format!("{reason} — no page answers to it yet"),
                });
                continue;
            }
            for page in pages {
                let reasons = reached.entry(page).or_default();
                if !reasons.contains(&reason) {
                    reasons.push(reason.clone());
                }
            }
        }
        let pages = reached
            .into_iter()
            .map(|(page, reasons)| PageToUpdate { page, reasons })
            .collect();
        (pages, unplaced)
    }

    /// The pages that cite one obligation by its address.
    fn citing(&self, address: &str) -> Vec<String> {
        let Some((path, anchor)) = address.rsplit_once('#') else {
            return Vec::new();
        };
        coverage::obligation_key(&self.corpus_root, path, anchor)
            .and_then(|key| self.cited.get(&key).cloned())
            .unwrap_or_default()
    }

    /// The pages whose `cli-help` quotes a command. The subject of a flag
    /// change is `<command> <flag>`, and a flag belongs to the help of
    /// its own command, so both kinds enter here and the flag's tail is
    /// cut off first.
    fn quoting_command(&self, subject: &str) -> Vec<String> {
        let command = match subject.split_once(" -") {
            Some((command, _)) => command,
            None => subject,
        }
        .trim();
        self.derived
            .iter()
            .filter(|(_, kind, reference)| {
                *kind == DerivedKind::CliHelp && quotes_command(reference, command)
            })
            .map(|(page, _, _)| page.clone())
            .collect()
    }

    /// The pages whose `manifest-field` quotes a field. A block that
    /// shows the WHOLE manifest shows every field in it, so it answers to
    /// any field change; a block that names one field answers to that
    /// field and to the tables above it.
    fn quoting_field(&self, subject: &str) -> Vec<String> {
        self.derived
            .iter()
            .filter(|(_, kind, reference)| {
                *kind == DerivedKind::ManifestField && quotes_field(reference, subject)
            })
            .map(|(page, _, _)| page.clone())
            .collect()
    }

    /// The pages whose `jtd-schema` quotes a format, by its registry id
    /// or by the path of the schema that id points at.
    fn quoting_schema(&self, id: &str, new: &DocSurface) -> Vec<String> {
        let path = new
            .formats
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.schema.clone())
            .unwrap_or_default();
        self.derived
            .iter()
            .filter(|(_, kind, reference)| {
                *kind == DerivedKind::JtdSchema
                    && (reference == id || (!path.is_empty() && reference == &path))
            })
            .map(|(page, _, _)| page.clone())
            .collect()
    }
}

/// Does a `cli-help` reference quote this command? `vibe list --help`
/// quotes `vibe list` and `vibe`, and nothing quotes a command it does
/// not open with.
fn quotes_command(reference: &str, command: &str) -> bool {
    let quoted: Vec<&str> = reference
        .split_whitespace()
        .take_while(|word| !word.starts_with('-'))
        .collect();
    let wanted: Vec<&str> = command.split_whitespace().collect();
    quoted.len() >= wanted.len() && quoted[..wanted.len()] == wanted[..]
}

/// Does a `manifest-field` reference quote this dotted path?
fn quotes_field(reference: &str, field: &str) -> bool {
    let named = match reference.split_once('#') {
        Some((_, named)) => named.trim(),
        // A coordinate alone is the whole manifest; a bare word with no
        // slash is one field of the documenting package's own.
        None if reference.contains('/') => return true,
        None => reference.trim(),
    };
    // `[]` marks an array of tables in a surface path and is not written
    // in a reference, so it is taken out before the two are compared.
    let bare = field.replace("[]", "");
    bare == named
        || bare
            .strip_prefix("package.")
            .is_some_and(|tail| tail == named)
        || bare.starts_with(&format!("{named}."))
}

/// One change as an author reads it.
fn reason(entry: &SurfaceChange) -> String {
    let kind = match entry.kind {
        ChangeKind::Command => "command",
        ChangeKind::Flag => "flag",
        ChangeKind::ManifestField => "manifest field",
        ChangeKind::LockField => "lock-file field",
        ChangeKind::Schema => "schema",
        ChangeKind::Fact => "rule",
        ChangeKind::Format => "format",
    };
    let verb = match entry.change {
        ChangeVerb::Added => "appeared",
        ChangeVerb::Removed => "is gone",
        ChangeVerb::Changed => "changed",
    };
    if entry.detail.is_empty() {
        format!("{kind} `{}` {verb}", entry.subject)
    } else {
        format!("{kind} `{}` {verb}: {}", entry.subject, entry.detail)
    }
}

/// The diff as a person reads it, in the week's report.
pub fn render_md(document: &DocSurfaceDiff) -> String {
    let mut out = format!(
        "# Pages to update between `{}` and `{}`\n\n",
        document.from, document.to
    );
    if document.changes.is_empty() {
        out.push_str(
            "The two surfaces are the same. Nothing in the contract moved, so no page \
             is owed an edit by this comparison.\n",
        );
        return out;
    }
    out.push_str(&format!(
        "{} change(s) in the contract, reaching {} page(s).\n\n",
        document.changes.len(),
        document.pages.len()
    ));
    for page in &document.pages {
        out.push_str(&format!("## `{}`\n\n", page.page));
        for reason in &page.reasons {
            out.push_str(&format!("- {reason}\n"));
        }
        out.push('\n');
    }
    if !document.needs_pages.is_empty() {
        out.push_str("## Needs a new page\n\n");
        for unplaced in &document.needs_pages {
            out.push_str(&format!("- {}\n", unplaced.reason));
        }
        out.push('\n');
    }
    out
}

/// The diff as a machine reads it.
pub fn to_json(document: &DocSurfaceDiff) -> String {
    // Generated from the schema, so it holds only JSON scalars and
    // sequences and cannot fail to serialise.
    let mut text = serde_json::to_string_pretty(document).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
