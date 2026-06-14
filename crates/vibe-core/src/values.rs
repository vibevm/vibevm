//! Typed-value tags carried on graph edges.
//!
//! Spec: `VIBEVM-SPEC.md` §5.3. Each tag here names a value type that may flow
//! between nodes. The actual Rust type carrying the value lives wherever it
//! makes sense (typically near the producing crate); this enum is used for
//! graph-build-time validation that an edge's input type matches the upstream
//! output type.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#typed-value-system");

use std::fmt;

/// Names of the typed values from §5.3. Keep in sync with the spec table.
///
/// ```
/// use vibe_core::values::ValueTag;
///
/// // Each tag has a stable string name used in graph-build validation.
/// assert_eq!(ValueTag::Lockfile.as_str(), "Lockfile");
/// assert_eq!(ValueTag::Wal.to_string(), "WAL"); // Display mirrors as_str
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueTag {
    ProjectConfig,
    Lockfile,
    PackageRef,
    PackageContents,
    WriteSpec,
    WritePlan,
    Approval,
    EffectiveSpec,
    Wal,
    WalVerdict,
    StackSpec,
    FeatSpec,
    BuildPlan,
    CodeFiles,
    CommandResult,
    Report,
}

impl ValueTag {
    pub const fn as_str(self) -> &'static str {
        match self {
            ValueTag::ProjectConfig => "ProjectConfig",
            ValueTag::Lockfile => "Lockfile",
            ValueTag::PackageRef => "PackageRef",
            ValueTag::PackageContents => "PackageContents",
            ValueTag::WriteSpec => "WriteSpec",
            ValueTag::WritePlan => "WritePlan",
            ValueTag::Approval => "Approval",
            ValueTag::EffectiveSpec => "EffectiveSpec",
            ValueTag::Wal => "WAL",
            ValueTag::WalVerdict => "WALVerdict",
            ValueTag::StackSpec => "StackSpec",
            ValueTag::FeatSpec => "FeatSpec",
            ValueTag::BuildPlan => "BuildPlan",
            ValueTag::CodeFiles => "CodeFiles",
            ValueTag::CommandResult => "CommandResult",
            ValueTag::Report => "Report",
        }
    }
}

impl fmt::Display for ValueTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
