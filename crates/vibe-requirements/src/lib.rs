//! # vibe-requirements — the R7.5 read-only requirements query library
//!
//! ONE bounded answer about what a project's specs declare and what it
//! recorded about them, assembled in one place for every surface
//! (PROP-054 `##FACT-QUERY-CONTRACT`; R7 architecture §5). The CLI and
//! MCP are thin parsers/renderers over [`query`]: they build a
//! [`RequirementsQuery`] and a [`QueryContext`], call the one function,
//! and receive the generated
//! [`vibe_wire::generated::requirements_report::RequirementsReport`] —
//! nothing else assembles report members.
//!
//! The library is read-only and credential-free by construction: it
//! enumerates the selected host source and the lock-selected
//! materialised package slots through public `vibe-workspace` /
//! `vibe-core` APIs, reads each authored source exactly once through
//! the A2a `vibe-facts` one-read seams (`observe_authored_source`,
//! `Registry::load_with_witnesses`), joins adoption through the A1
//! `join_adoption`, and never syncs, writes, re-walks or re-reads a
//! source. `observed_at` is injected, never clocked inside; the
//! selected root is a trusted constructor input, never a wire member;
//! the optional lifecycle run id is injected by the surface through the
//! existing read-only lifecycle peek — this crate does not depend on
//! the lifecycle engine.
//!
//! Optional relation enrichment is an injected [`RelationProvider`]
//! value, called at most once per query and only when the query asked
//! for relations; provenance and wire `current|carried` states are
//! DERIVED here from the base source kind, never chosen by a provider.
//!
//! ```
//! use std::path::Path;
//! use vibe_requirements::{QueryContext, RequirementsQuery};
//! use vibe_wire::generated::shared::Timestamp;
//!
//! let root = tempfile::TempDir::new().unwrap();
//! std::fs::write(root.path().join("vibe.toml"),
//!     "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
//! let specs = root.path().join(vibe_core::layout::current_specs_root());
//! std::fs::create_dir_all(&specs).unwrap();
//! std::fs::write(specs.join("RULE.md"),
//!     "# Rules\n\n@fact:FIRST First. @status:impl/done\n").unwrap();
//!
//! let report = vibe_requirements::query(
//!     &RequirementsQuery::default(),
//!     &QueryContext {
//!         selected_root: root.path().to_path_buf(),
//!         observed_at: "2026-01-01T00:00:00Z".parse::<Timestamp>().unwrap(),
//!         lifecycle_run_id: None,
//!     },
//!     None,
//! )
//! .unwrap();
//! assert_eq!(report.rows.len(), 1);
//! assert_eq!(report.rows[0].address, "spec://org.example/demo/RULE#FIRST");
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT");

// Implementation modules stay PRIVATE: the public API is exactly the
// explicit re-exports below plus `text`. Nothing accidental is frozen
// as public surface.
mod digest;
mod provider;
mod query;
mod rows;
mod sources;
pub mod text;

mod error;
pub use error::QueryError;
pub use provider::{
    ProviderError, ProviderOutcome, ProviderSource, RelationProvider, RelationRequest,
};
pub use query::{QueryContext, RequirementsQuery, query};
pub use vibe_wire::generated::shared::Timestamp;

#[cfg(test)]
mod tests_digest;
#[cfg(test)]
mod tests_followup;
#[cfg(test)]
mod tests_provider;
#[cfg(test)]
mod tests_query;
