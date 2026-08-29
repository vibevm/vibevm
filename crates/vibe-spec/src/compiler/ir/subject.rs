//! The immutable per-document selector subject (R4-TRANSFORM-PLAN-ABI §5).
//!
//! A source/document transform runs once per addressed document, and its
//! compiled selector needs a subject to judge that document by. This cell owns
//! that subject: the typed provider its `packages` dimension matches and the
//! declared path its `paths` dimension matches, carried on
//! [`super::ArtifactInput`] and reaching [`super::SourceIr`], with
//! [`super::DocumentIr`] reading it through its source.
//!
//! Four properties are load-bearing.
//!
//! **The subject is CARRIED, never recovered from a display string.**
//! `ContributionMeta.origin` stays display/provenance and gains no parsed
//! identity, so [`DocumentProvider`] is a `vibe-spec`-owned typed value built
//! from already-typed coordinate components. Its coordinate arms mirror the
//! extension kernel's two identity shapes exactly, so the match-time adapter
//! the selector atom lands reconstructs the typed identity component for
//! component — never by parsing a rendered spelling.
//!
//! **Why `vibe-spec` owns the type, and why that is NOT a public-API
//! argument.** [`DocumentSubject`] and [`DocumentProvider`] are `pub(crate)`,
//! `ArtifactInput.subject` is a private field, `crate::lib` re-exports neither
//! type, and a private field's type is not part of a Rust public API — so the
//! kernel's `DependencyProviderId`/`HostIdentity` in this seat would have
//! leaked nothing. Two arguments that do hold:
//!
//! 1. The subject is a JTD wire shape, and a wire shape is spelled from types
//!    this crate owns. The kernel nests
//!    `HostIdentity::Coordinate(DependencyProviderId)` where the schema wants
//!    flat arms, so owning the type decouples a frozen epoch-1 schema from a
//!    kernel refactor.
//! 2. `vibe_core` is the workspace's shared scalar vocabulary; the extension
//!    registry is a sibling subsystem, and an `ir/ → vibe_extension_registry`
//!    edge would put the compiler IR wire downstream of the extension kernel.
//!
//! The `vibe_core` import below is itself the FIRST edge from
//! `crates/vibe-spec/src/compiler/ir/**` to any lower crate: that subtree's
//! product code depended on nothing outside `vibe-spec` before this cell.
//!
//! **Two absences, two arms — never one `Option`.** A document with no
//! coordinate is in one of two states, and they are not the same claim.
//! [`DocumentProvider::Unclaimed`] is permanent and correct: no contribution
//! row declared this document into this artifact, so there is no owner to
//! claim. [`DocumentProvider::Undetermined`] is temporary: a row DID declare
//! it, and the producer could not say which typed provider that row names. The
//! kernel's `dimension_matches` answers an authored `packages` dimension with
//! `false` for a subject that carries no provider, which is the right answer
//! for `Unclaimed` and a silently wrong one for `Undetermined` — the selector
//! adapter must be able to refuse the second while matching-nothing on the
//! first, and a single `None` cannot give it both.
//!
//! **Per document, never per artifact.** One subject belongs to one addressed
//! document. A document a contribution DECLARED carries the declaring row's
//! path — which may legitimately differ from the address' own `doc_path`. A
//! document the compiler REACHED through `#use`, `#source` or `#embed` was
//! declared by no row at all, so [`DocumentSubject::reached`] states the only
//! honest identity it has: its own document path, and an `Unclaimed` provider.

use std::fmt;

use vibe_core::{Group, PackageName};

use super::DocumentAddress;

/// The provider identity one document subject carries.
///
/// Total by construction: every document has an answer here, and the two
/// answers that are not a coordinate are named apart rather than fused into
/// one absence. The coordinate arms hold validated components, so nothing here
/// is a parsed display string and nothing renders one to decide identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocumentProvider {
    /// An installed dependency provider's versionless coordinate.
    Dependency { group: Group, name: PackageName },
    /// A host project with no group, named exactly as authored — the one
    /// spelling that has no coordinate, mirroring the kernel's ungrouped host.
    HostUngrouped { name: String },
    /// A grouped project or package-role host: the same validated coordinate a
    /// dependency carries, in the host seat.
    HostCoordinate { group: Group, name: PackageName },
    /// A coordinator that may control dependencies but declares none.
    HostVirtualWorkspace,
    /// No contribution row declared this document into this artifact.
    ///
    /// A permanent, correct answer, and the one every REACHED document has:
    /// the address' authority is the package that OWNS the document, which is
    /// not the question a `packages` dimension asks. An authored `packages`
    /// dimension matching nothing is the right verdict here.
    Unclaimed,
    /// The producer of this subject could not determine a provider.
    ///
    /// A row DID declare this document; today's producers receive that row's
    /// owner as a display string, so no typed provider exists at the point the
    /// subject is born. Temporary, and the state every document written by
    /// today's producers is in until the owner-view adapter supplies real
    /// coordinates. A selector adapter may not silently read it as "matches
    /// nothing" — the honest reading is that the answer is not yet known.
    Undetermined,
}

impl fmt::Display for DocumentProvider {
    /// The provider's own honest spelling, so a refusal that names it reads as
    /// identity rather than as a Rust field dump.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dependency { group, name } => write!(formatter, "dependency {group}/{name}"),
            Self::HostUngrouped { name } => write!(formatter, "host {name}"),
            Self::HostCoordinate { group, name } => write!(formatter, "host {group}/{name}"),
            Self::HostVirtualWorkspace => formatter.write_str("host virtual-workspace"),
            Self::Unclaimed => formatter.write_str("unclaimed"),
            Self::Undetermined => formatter.write_str("undetermined"),
        }
    }
}

/// The immutable selector subject of exactly one addressed document.
///
/// Private fields with read-only accessors: the value is evidence a transform
/// is judged against, so no member is writable once the document exists, and
/// the inter-pass verifier refuses a source/document transform that returns a
/// different one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentSubject {
    provider: DocumentProvider,
    declared_path: String,
}

impl DocumentSubject {
    /// The subject of a document a contribution DECLARED.
    ///
    /// `declared_path` is that row's own already-validated path; it is not
    /// derived from the document address, and the two may differ (a boot row
    /// declaring `boot/alpha.md` may seed `spec://org.demo/alpha/boot/entry`).
    pub(crate) fn declared(provider: DocumentProvider, declared_path: impl Into<String>) -> Self {
        Self {
            provider,
            declared_path: declared_path.into(),
        }
    }

    /// The subject of a document nothing declared — one the compiler REACHED
    /// through `#use`, `#source` or `#embed`.
    ///
    /// No contribution row named it, so the only honest path identity it has
    /// is its own: a spec address' `doc_path`, or a static entry's path. The
    /// provider is [`DocumentProvider::Unclaimed`] rather than merely unknown:
    /// there is no declaring owner to determine later, so an authored
    /// `packages` dimension matching nothing is a final verdict, not a
    /// placeholder.
    pub(crate) fn reached(address: &DocumentAddress) -> Self {
        let declared_path = match address {
            DocumentAddress::Spec(address) => address.doc_path.clone(),
            DocumentAddress::StaticEntry { path, .. } => path.clone(),
        };
        Self {
            provider: DocumentProvider::Unclaimed,
            declared_path,
        }
    }

    /// The typed provider a `packages` selector dimension is matched against.
    pub(crate) fn provider(&self) -> &DocumentProvider {
        &self.provider
    }

    /// The declared path a `paths` selector dimension is matched against.
    pub(crate) fn declared_path(&self) -> &str {
        &self.declared_path
    }

    /// Does `path` obey the separator half of the `paths` selector contract?
    ///
    /// The kernel compiles an authored `paths` glob with
    /// `require_literal_separator: true` and `case_sensitive: true`
    /// (`vibe-extension-registry/src/selector.rs`), so `\` is not a path
    /// separator to any pattern it can be matched against. A backslashed
    /// `declared_path` therefore does not match the wrong rows — it matches
    /// NOTHING, silently, and the symptom is a transform that quietly never
    /// applies. Separator spelling is semantic, so it is a refusal, not a
    /// normalisation: rewriting `\` to `/` here would invent an identity the
    /// producer did not state.
    ///
    /// This predicate is the whole rule, in one place. Each boundary that
    /// already refuses a blank `declared_path` — the artifact plan, the wire
    /// scalar gate, the inter-pass verifier — refuses a backslashed one
    /// through this same function, in its own error vocabulary.
    pub(crate) fn path_is_forward_slashed(path: &str) -> bool {
        !path.contains('\\')
    }
}

impl fmt::Display for DocumentSubject {
    /// Both members in one line, each in its own honest spelling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} declaring `{}`",
            self.provider, self.declared_path
        )
    }
}
