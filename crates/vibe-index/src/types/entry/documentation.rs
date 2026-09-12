//! Documentation relations and the card carried by a
//! [`VersionEntry`](super::VersionEntry) — re-exports of the generated
//! wire types (shared vocabulary; JTD is the source of truth).
//!
//! These four ride in the index for one reason, and PROP-057
//! `##REL-INDEX-FIELDS` states it: the reverse questions — «who
//! documents X», «which translations does Y have» — and officiality are
//! answered by ONE fold over `primary.jsonl`, in memory, at build time
//! (`##REL-REVERSE-QUERIES-SITE-SIDE`). The index therefore gains
//! fields and no routes: a shelf card, a language selector and an
//! officiality badge must render without downloading a single package,
//! and the local reader offline takes the same path as the site.
//!
//! `documents` is the only repeatable one — one documentation may
//! document several subjects — and, like every other projection here,
//! an empty one is absence on the wire, never `[]`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#relation");

pub use vibe_wire::generated::shared::{
    DocumentationEntry, DocumentsEntry, MediaEntry, TranslatesEntry,
};
