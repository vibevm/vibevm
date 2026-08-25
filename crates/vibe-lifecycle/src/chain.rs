//! Construction of default and clean-prefixed lifecycle step chains.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS");

use specmark::spec;

use crate::{DEFAULT_PHASES, Phase};

/// One executable step in a lifecycle request.
///
/// `Clean` belongs to its own one-step lifecycle; `Default` identifies a phase
/// in the fixed default lifecycle.
///
/// ```
/// use vibe_lifecycle::{LifecycleStep, Phase};
///
/// assert_eq!(LifecycleStep::Clean, LifecycleStep::Clean);
/// assert_eq!(
///     LifecycleStep::Default(Phase::Test),
///     LifecycleStep::Default(Phase::Test),
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleStep {
    /// Enter the separate clean lifecycle before any requested default phases.
    Clean,
    /// Execute one phase of the default lifecycle.
    Default(Phase),
}

/// A parsed request for one of vibe's two lifecycles.
///
/// A default request runs through its named phase. A clean request always runs
/// clean first and may stop there or continue through a named default phase.
///
/// ```
/// use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
///
/// assert_eq!(
///     LifecycleRequest::Default(Phase::Install).steps(),
///     vec![
///         LifecycleStep::Default(Phase::Validate),
///         LifecycleStep::Default(Phase::Install),
///     ],
/// );
/// assert_eq!(
///     LifecycleRequest::Clean { then: None }.steps(),
///     vec![LifecycleStep::Clean],
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleRequest {
    /// Run the default lifecycle through the named phase.
    Default(Phase),
    /// Run clean, optionally followed by the default lifecycle through `then`.
    Clean {
        /// The last default phase to execute after clean, if any.
        then: Option<Phase>,
    },
}

/// Return the canonical default-lifecycle prefix ending at `through`.
///
/// The result is a slice of [`DEFAULT_PHASES`]. Its order is therefore derived
/// solely from the phase's position in that table, never from enum ordinal
/// values.
///
/// ```
/// use vibe_lifecycle::{Phase, inclusive_chain};
///
/// assert_eq!(
///     inclusive_chain(Phase::Build),
///     &[
///         Phase::Validate,
///         Phase::Install,
///         Phase::Generate,
///         Phase::Build,
///     ],
/// );
/// ```
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
pub fn inclusive_chain(through: Phase) -> &'static [Phase] {
    let Some(end) = DEFAULT_PHASES.iter().position(|phase| *phase == through) else {
        unreachable!("every closed Phase variant must occur in DEFAULT_PHASES")
    };
    &DEFAULT_PHASES[..=end]
}

impl LifecycleRequest {
    /// Expand the request into execution order.
    ///
    /// Clean, when requested, is always the first and only clean-lifecycle
    /// step. Default phases then follow in canonical inclusive order.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
    pub fn steps(self) -> Vec<LifecycleStep> {
        match self {
            Self::Default(through) => default_steps(through).collect(),
            Self::Clean { then } => {
                let default_len = then.map_or(0, |phase| inclusive_chain(phase).len());
                let mut steps = Vec::with_capacity(1 + default_len);
                steps.push(LifecycleStep::Clean);
                if let Some(through) = then {
                    steps.extend(default_steps(through));
                }
                steps
            }
        }
    }
}

fn default_steps(through: Phase) -> impl Iterator<Item = LifecycleStep> {
    inclusive_chain(through)
        .iter()
        .copied()
        .map(LifecycleStep::Default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
    fn inclusive_prefix_is_exhaustive_for_every_phase() {
        for (index, phase) in DEFAULT_PHASES.into_iter().enumerate() {
            assert_eq!(inclusive_chain(phase), &DEFAULT_PHASES[..=index]);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
    fn default_request_maps_the_canonical_prefix_to_steps() {
        for phase in DEFAULT_PHASES {
            let expected = inclusive_chain(phase)
                .iter()
                .copied()
                .map(LifecycleStep::Default)
                .collect::<Vec<_>>();
            assert_eq!(LifecycleRequest::Default(phase).steps(), expected);
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
    fn chains_are_monotone_prefixes() {
        for (left_index, left) in DEFAULT_PHASES.into_iter().enumerate() {
            for (right_index, right) in DEFAULT_PHASES.into_iter().enumerate() {
                assert_eq!(
                    left_index <= right_index,
                    inclusive_chain(right).starts_with(inclusive_chain(left)),
                );
            }
        }
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
    fn validate_and_deploy_are_the_default_chain_boundaries() {
        assert_eq!(inclusive_chain(Phase::Validate), &[Phase::Validate]);
        assert_eq!(inclusive_chain(Phase::Deploy), DEFAULT_PHASES);
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
    fn clean_without_continuation_is_exactly_one_step() {
        assert_eq!(
            LifecycleRequest::Clean { then: None }.steps(),
            vec![LifecycleStep::Clean],
        );
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
    fn clean_prefixes_every_possible_default_chain() {
        for phase in DEFAULT_PHASES {
            let steps = LifecycleRequest::Clean { then: Some(phase) }.steps();
            assert_eq!(steps.first(), Some(&LifecycleStep::Clean));
            assert_eq!(steps.len(), inclusive_chain(phase).len() + 1);
            assert_eq!(
                &steps[1..],
                &inclusive_chain(phase)
                    .iter()
                    .copied()
                    .map(LifecycleStep::Default)
                    .collect::<Vec<_>>(),
            );
        }
    }
}
