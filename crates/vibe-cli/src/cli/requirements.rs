//! Argument struct for `vibe requirements` — the R7.5 read-only
//! requirements surface (PROP-054 `##REF-REQUIREMENTS-SURFACES`; R7
//! architecture §6.1). Split from the `cli` hub because that file has
//! almost no line budget left; the hub re-exports it, so
//! `crate::cli::RequirementsArgs` is the address.
//!
//! The grammar is the whole of this file. Every VALUE law — that a
//! prefix is a `spec://` prefix and never a bare fact id, that the row
//! bound is inclusive `1..=256` — belongs to
//! `vibe_requirements::RequirementsQuery::try_new`, which refuses
//! through the wire owner's own validator. Restating either here would
//! create the second grammar `##REF-REQUIREMENTS-SURFACES` exists to
//! prevent: the CLI and the MCP tool would then be free to drift about
//! what a legal question is. clap turns away an unknown option and a
//! value that is not a `u32`; everything else is the one constructor's,
//! and it decides before any filesystem access.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-REQUIREMENTS-SURFACES");

use std::path::PathBuf;

/// `vibe requirements` — one bounded metadata answer about what the
/// selected project's specs declare and what it recorded about them.
///
/// Read-only and algorithmic: no sync, no write, no materialisation, no
/// credential and no network. The four observation axes stay four
/// columns — this surface never combines them into a verdict.
#[derive(Debug, clap::Args)]
pub struct RequirementsArgs {
    /// Scope the answer to one `spec://` address prefix. Absent asks
    /// about every addressed fact. A bare fact id is refused: it names
    /// nothing a second reader could resolve.
    #[arg(long = "address-prefix", value_name = "PREFIX")]
    pub address_prefix: Option<String>,

    /// Maximum number of fact rows, inclusive range 1..=256. There is
    /// no unbounded mode — an answer that does not fit an agent's
    /// context is worthless — and the report restates the bound it was
    /// cut by, so a truncated answer says so.
    #[arg(
        long,
        value_name = "N",
        default_value_t = vibe_requirements::RequirementsQuery::default().limit()
    )]
    pub limit: u32,

    /// Ask for the optional relation-edge enrichment. Absent means
    /// `not-requested`: no specmap config is read and no map is loaded
    /// or built. Enrichment never changes the fact rows — a loss is
    /// typed per source and the base rows still return.
    #[arg(long)]
    pub relations: bool,

    /// Project root — the selected workspace node the answer is about.
    /// Defaults to the current directory. A trusted constructor input:
    /// the path never rides the answer.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
