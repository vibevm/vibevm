//! Lifecycle-owned executable plan values over the pure registry kernel.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::ops::Deref;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;

use super::{ExtensionRegistry, ExtensionRegistryRow, SelectorSubject};

/// One owned effective contribution, labelled with its lifecycle phase.
///
/// Adapters retain this after the source registry has dropped; dispatch never
/// re-collects or re-sorts it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone)]
pub struct ExecutableContribution {
    /// Canonical lifecycle phase spelling supplied with the planned point.
    pub phase: String,
    /// Complete retained declaration/provider/control row.
    pub row: ExtensionRegistryRow,
}

/// Owned canonical execution plan shared by every future surface adapter.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, Default)]
pub struct ExecutablePlan(Vec<ExecutableContribution>);

impl ExecutablePlan {
    /// Select points in caller-supplied phase order while preserving the
    /// registry's closed tier order within each point.
    #[must_use]
    pub fn from_points<I>(
        registry: &ExtensionRegistry,
        points: I,
        subject: SelectorSubject<'_>,
    ) -> Self
    where
        I: IntoIterator<Item = (String, ExtensionPoint)>,
    {
        let mut rows = Vec::new();
        for (phase, point) in points {
            rows.extend(registry.plan(point, subject).into_iter().map(|row| {
                ExecutableContribution {
                    phase: phase.clone(),
                    row: row.clone(),
                }
            }));
        }
        Self(rows)
    }

    /// Number of contributions selected for one phase spelling.
    #[must_use]
    pub fn count_for(&self, phase: &str) -> usize {
        self.0.iter().filter(|row| row.phase == phase).count()
    }
}

impl Deref for ExecutablePlan {
    type Target = [ExecutableContribution];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
