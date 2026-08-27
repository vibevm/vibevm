//! `[compile]` — consumer-side compiler controls.
//!
//! One table, one member for now: `trace`, the LLVM `-print-after-all`
//! switch of the spec compiler (PROP-054 `##OBS-TRACE`). It is **consumer**
//! configuration, never a package-role declaration — a `[package]`-rooted
//! dev checkout carries exactly the compile controls a `[project]`-rooted
//! one does (PROP-024 `##MANIFEST-ROLES-ARE-EQUIPOTENT`), and a virtual
//! `[workspace]` coordinator carries them too. Consumer-side code therefore
//! reads [`Manifest::compile_trace_enabled`] and never branches on the role.
//!
//! Scope is the selected root: a dependency package's `[compile]` table is
//! its own business and never activates tracing for the host above it. The
//! setting is deliberately absent from package metadata, lockfile records,
//! index records and extension activation, so there is nowhere for it to
//! leak upward from.
//!
//! Absent and `trace = false` are one value. The serializer skips the
//! default table, so a manifest that never asked for tracing keeps no
//! `[compile]` on disk, and `trace = true` writes back the table it was
//! read from.
//!
//! This atom is the manifest rung only. The later resolution is
//! `--trace-compile || manifest.compile.trace`; neither the CLI flag nor a
//! user-config rung exists yet.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use serde::{Deserialize, Serialize};

use super::document::Manifest;

/// `[compile]` — how this node compiles its own documents.
///
/// ```
/// use vibe_core::manifest::CompileSection;
///
/// let absent = CompileSection::default();
/// assert!(!absent.trace);
///
/// // An absent table and an explicit `trace = false` are the same value.
/// let explicit: CompileSection = toml::from_str("trace = false").unwrap();
/// assert_eq!(explicit, absent);
///
/// let asked: CompileSection = toml::from_str("trace = true").unwrap();
/// assert!(asked.trace);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileSection {
    /// Dump the compiler IR after every pass and time the passes
    /// (PROP-054 `##OBS-TRACE`). Absent → `false`.
    #[serde(default)]
    pub trace: bool,
}

impl CompileSection {
    /// `true` when the table asks for nothing beyond the defaults — the
    /// signal the serializer uses to omit `[compile]` entirely.
    ///
    /// ```
    /// use vibe_core::manifest::CompileSection;
    ///
    /// assert!(CompileSection::default().is_default());
    /// assert!(!CompileSection { trace: true }.is_default());
    /// ```
    pub fn is_default(&self) -> bool {
        !self.trace
    }
}

impl Manifest {
    /// The resolved `[compile] trace` switch of this root, read role-blind.
    ///
    /// Every role answers from the same field: a project, a package, a
    /// virtual `[workspace]` coordinator, and a root that combines a role
    /// with `[workspace]`. Callers never branch on which one they hold.
    ///
    /// ```
    /// use vibe_core::manifest::Manifest;
    ///
    /// let project = Manifest::parse_str(concat!(
    ///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
    ///     "[compile]\ntrace = true\n",
    /// ))
    /// .unwrap();
    /// let package = Manifest::parse_str(concat!(
    ///     "[package]\ngroup = \"org.x\"\nname = \"wal\"\n",
    ///     "kind = \"flow\"\nversion = \"1.0.0\"\n\n",
    ///     "[compile]\ntrace = true\n",
    /// ))
    /// .unwrap();
    /// let coordinator = Manifest::parse_str("[workspace]\nmembers = []\n").unwrap();
    ///
    /// assert!(project.compile_trace_enabled());
    /// assert!(package.compile_trace_enabled()); // roles are equipotent
    /// assert!(!coordinator.compile_trace_enabled()); // absent table → false
    /// ```
    pub fn compile_trace_enabled(&self) -> bool {
        self.compile.trace
    }
}

#[cfg(test)]
#[path = "compile/tests.rs"]
mod tests;
