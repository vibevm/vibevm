//! The continuation plan — what a runner reads *after* the prefix.
//!
//! The plan is the design's load-bearing boundary. An INDEX row's bytes are
//! never frozen into the prefix, however unconditional the row, because
//! `kind = "static"` describes read timing and not linkage
//! (`#SYS-NO-FROZEN-INDEX`). What is fixed at generation time is the rows'
//! *identity, order and conditions*; their bytes are read per request and
//! witnessed there (`#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
//!
//! Two bands, and only one kind of gate can reach the second. `installed:`
//! is decided while artifacts are generated, and so is `target:` — the
//! platform the product addresses is a property of the project, never of a
//! session (PROP-076 `#SYS-HOST-AND-TARGET`). Only an `os:` gate survives,
//! and it is evaluated against an explicitly supplied session target, never
//! against the operating system of whatever machine generated the prefix
//! (`#SYS-SESSION-TARGET`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-BANDS");

use vibe_core::manifest::{HostChoice, WhenCondition};

use crate::boot::EffectiveBoot;

/// Which continuation band a row belongs to.
///
/// ```
/// use vibe_core::manifest::{TargetOs, WhenCondition};
/// use vibe_workspace::system_prompt::ContinuationBand;
///
/// assert!(ContinuationBand::Unconditional.is_unconditional());
/// let gated = ContinuationBand::Gated(WhenCondition::Os(TargetOs::Windows));
/// assert_eq!(gated.when().map(ToString::to_string).as_deref(), Some("os:windows"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationBand {
    /// Band U — read on every request, in plan order.
    Unconditional,
    /// Band G — read only when the condition holds for an explicitly
    /// supplied session target. The condition is carried verbatim, because
    /// the runner, not the generator, decides it.
    Gated(WhenCondition),
}

impl ContinuationBand {
    /// `true` for band U.
    pub fn is_unconditional(&self) -> bool {
        matches!(self, ContinuationBand::Unconditional)
    }

    /// The gate, for a band-G row.
    pub fn when(&self) -> Option<&WhenCondition> {
        match self {
            ContinuationBand::Unconditional => None,
            ContinuationBand::Gated(when) => Some(when),
        }
    }
}

/// One continuation row: a file to read and the band that decides whether
/// this request reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationRow {
    /// The path exactly as the generated `INDEX.md` row spells it, which is
    /// how a reading agent resolves it today.
    ///
    /// That spelling is relative to the **workspace root**, not to the
    /// selected node, and the plan keeps it unchanged rather than
    /// re-basing it: a dependency row names a path under `vibedeps/` that
    /// does not lie beneath a member node at all, so a node-relative
    /// spelling could not express it. Re-basing would also make the plan
    /// disagree with the artifact a session actually reads.
    pub path: String,
    /// Band U or band G.
    pub band: ContinuationBand,
}

/// The ordered rows of bands U and G — the live half of the boot the prefix
/// does not carry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContinuationPlan {
    /// The rows in `INDEX.md` order, which is the order a session reads
    /// them. Band U and band G are not separated here on purpose: the order
    /// is one sequence, and splitting it would lose the interleaving a
    /// reader follows.
    pub rows: Vec<ContinuationRow>,
}

impl ContinuationPlan {
    /// Derive the plan from a node's computed effective boot.
    ///
    /// The source is `EffectiveBoot::dynamic_entries()` in composed order —
    /// the exact iterator the INDEX renderer maps — so the plan's order is
    /// the artifact's order by construction rather than by agreement. The
    /// classification is the same one expression: a residual `when` is band
    /// G, its absence band U.
    pub fn of(boot: &EffectiveBoot) -> Self {
        Self {
            rows: boot
                .dynamic_entries()
                .map(|entry| ContinuationRow {
                    path: entry.path.clone(),
                    band: match &entry.when {
                        Some(when) => ContinuationBand::Gated(when.clone()),
                        None => ContinuationBand::Unconditional,
                    },
                })
                .collect(),
        }
    }

    /// Resolve `os:` rows against the node's declared `host`.
    ///
    /// `portable` — the default — changes nothing: the rows stay in band G
    /// and are evaluated per session against an explicitly supplied session
    /// target. A **fixed** host is a project declaration read from the
    /// node's own manifest, so it is resolved here, at build time: a
    /// matching row becomes an ordinary band-U row and a non-matching one is
    /// dropped entirely, which is what leaves a fixed-host prefix with an
    /// empty band G and lets `prepare` need no session target for it
    /// (`#SYS-HOST-AND-TARGET`).
    ///
    /// Nothing is detected. The OS name comes from the manifest exactly like
    /// `entrypoint` or `recipe`, so the artifact stays a pure function of
    /// its declared inputs and `#SYS-NO-VOLATILE` holds unchanged.
    ///
    /// ```
    /// use vibe_core::manifest::{HostChoice, TargetOs};
    /// use vibe_workspace::system_prompt::ContinuationPlan;
    ///
    /// let plan = ContinuationPlan::default();
    /// // An empty plan resolves to an empty plan under either host.
    /// assert_eq!(plan.resolve_host(HostChoice::Portable), plan);
    /// assert_eq!(plan.resolve_host(HostChoice::Fixed(TargetOs::Linux)), plan);
    /// ```
    pub fn resolve_host(&self, host: HostChoice) -> Self {
        let Some(fixed) = host.fixed() else {
            return self.clone();
        };
        Self {
            rows: self
                .rows
                .iter()
                .filter_map(
                    |row| match row.band.when().and_then(WhenCondition::session_os) {
                        None => Some(row.clone()),
                        Some(os) if os == fixed => Some(ContinuationRow {
                            path: row.path.clone(),
                            band: ContinuationBand::Unconditional,
                        }),
                        Some(_) => None,
                    },
                )
                .collect(),
        }
    }

    /// The band-U rows, in plan order — the material a second cache
    /// breakpoint covers (`#SYS-CONTINUATION-CACHEABLE`).
    pub fn unconditional(&self) -> impl Iterator<Item = &ContinuationRow> {
        self.rows.iter().filter(|row| row.band.is_unconditional())
    }

    /// The band-G rows, in plan order — the ones a runner can only resolve
    /// against an explicitly supplied session target.
    pub fn gated(&self) -> impl Iterator<Item = &ContinuationRow> {
        self.rows.iter().filter(|row| !row.band.is_unconditional())
    }
}
