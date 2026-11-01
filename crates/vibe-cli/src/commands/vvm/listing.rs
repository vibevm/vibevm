//! The human shape of `vibe self ls` and `vibe self current` (PROP-067
//! `#list`, `#meta`): one row per instance, its columns measured over the
//! whole list, the state words in brackets at the end of the row.
//!
//! Every field the old `key=value` line carried is still on the row — the
//! selector, origin and profile as columns, the commit shortened as a
//! column, the source and payload digest as metadata — so the listing
//! keeps what a reader copied from it and loses only the punctuation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use super::builder::short_commit;
use super::model::InstallRecord;
use super::provenance::SourceExecution;
use crate::output::language::{Cell, List, Meta, Role, Row, rule};

/// Which of the two states an instance row carries.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Marks {
    pub(super) active: bool,
    pub(super) running: bool,
}

/// One installed instance as a row.
///
/// ```text
///   branch:main#106  external  4f2a91c0de  debug  /src/vibevm  [active]  [running]
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn record_row(record: &InstallRecord, marks: Marks) -> Row {
    let mut meta = vec![Meta::note(record.source_label())];
    if let Some(sha) = &record.payload_sha256 {
        meta.push(Meta::note(format!("sha256 {sha}")));
    }
    if marks.active {
        meta.push(Meta::State("active".into()));
    }
    if marks.running {
        meta.push(Meta::State("running".into()));
    }
    Row::new(vec![
        Cell::new(record.selector().to_string(), Role::Accent),
        Cell::text(record.origin.as_str()),
        Cell::new(short_commit(&record.commit), Role::Dim),
        Cell::text(record.profile.as_str()),
    ])
    .meta(meta)
}

/// The executable running straight from a source tree, which no selector
/// names: its row says so in the name column.
pub(super) fn source_row(source: &SourceExecution) -> Row {
    Row::new(vec![
        Cell::new("source", Role::Accent),
        Cell::text("external"),
        Cell::new(short_commit(&source.commit), Role::Dim),
        Cell::text("-"),
    ])
    .meta(vec![
        Meta::note(source.source.display().to_string()),
        Meta::note(format!("executable {}", source.executable.display())),
        Meta::State("running".into()),
    ])
}

/// The rows as one list block, then a source execution under a rule of
/// its own, because it is not one of the installed instances.
pub(super) fn render(rows: Vec<Row>, source: Option<&SourceExecution>) -> String {
    let mut blocks = Vec::new();
    if !rows.is_empty() {
        blocks.push(List::new(rows).meta_column().render());
    }
    if let Some(source) = source {
        blocks.push(rule("running from a source tree", None));
        blocks.push(List::new(vec![source_row(source)]).meta_column().render());
    }
    blocks.join("\n\n")
}
