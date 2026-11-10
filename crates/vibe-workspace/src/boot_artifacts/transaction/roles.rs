//! The journal's role algebra: every stage role, every production target,
//! and the **one** enumeration of a journal's entries that every by-name
//! list now derives from.
//!
//! The role set was closed and validated before the system-prompt artifacts
//! joined it, and spike S2 of the sys-prompt campaign measured what adding
//! to it costs: twelve files refuse to compile, and **five** further edits
//! enumerate journal entries by name and are not compile-forced
//! (`roll_forward_core`, `rollback_all`, `discard_assets`,
//! `preflight_journal_paths`, and the operation-level undo image). An
//! extension that compiles could therefore still never roll the new roles
//! forward, never roll them back, leak their stage files on failure and
//! leave them out of the undo image — silence, not volume, being the risk.
//!
//! So the fix here is structural rather than careful: [`entries`] and
//! [`states`] are the single enumeration, and the by-name lists are gone.
//! A sixth role added later is reachable from every one of those sites the
//! moment it joins [`PROMPT_MEMBERS`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-TRANSACTION-ROLES");

use super::journal::{Journal, JournalEntry, RecordedState};

/// Every stage role a journal may name. One closed list, read by the stage
/// naming, the owned-namespace validation and the transaction identity
/// parser, so none of the three can drift from the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageRole {
    IndexBefore,
    IndexAfter,
    SelectedBefore,
    SelectedAfter,
    StaleBefore,
    CommitIntent,
    RollbackIntent,
    RedirectClaude,
    RedirectAgents,
    RedirectGemini,
    GitignoreBefore,
    GitignoreAfter,
    SystemBefore,
    SystemAfter,
    SystemMetaBefore,
    SystemMetaAfter,
    AgentLaneBefore,
    AgentLaneAfter,
}

impl StageRole {
    /// The role's wire spelling, which is also the stage filename's suffix.
    ///
    /// No spelling here may be a `-`-prefixed suffix of another, or
    /// [`super::journal::owned_stage_transaction`] could attribute one
    /// role's stage to another; a unit test pins that.
    pub(super) fn wire(self) -> &'static str {
        match self {
            StageRole::IndexBefore => "index-before",
            StageRole::IndexAfter => "index-after",
            StageRole::SelectedBefore => "selected-before",
            StageRole::SelectedAfter => "selected-after",
            StageRole::StaleBefore => "stale-before",
            StageRole::CommitIntent => "commit-intent",
            StageRole::RollbackIntent => "rollback-intent",
            StageRole::RedirectClaude => "redirect-claude",
            StageRole::RedirectAgents => "redirect-agents",
            StageRole::RedirectGemini => "redirect-gemini",
            StageRole::GitignoreBefore => "gitignore-before",
            StageRole::GitignoreAfter => "gitignore-after",
            StageRole::SystemBefore => "system-before",
            StageRole::SystemAfter => "system-after",
            StageRole::SystemMetaBefore => "system-meta-before",
            StageRole::SystemMetaAfter => "system-meta-after",
            StageRole::AgentLaneBefore => "agent-lane-before",
            StageRole::AgentLaneAfter => "agent-lane-after",
        }
    }

    /// Every role, for the two namespace validators.
    pub(super) const ALL: [StageRole; 18] = [
        StageRole::IndexBefore,
        StageRole::IndexAfter,
        StageRole::SelectedBefore,
        StageRole::SelectedAfter,
        StageRole::StaleBefore,
        StageRole::CommitIntent,
        StageRole::RollbackIntent,
        StageRole::RedirectClaude,
        StageRole::RedirectAgents,
        StageRole::RedirectGemini,
        StageRole::GitignoreBefore,
        StageRole::GitignoreAfter,
        StageRole::SystemBefore,
        StageRole::SystemAfter,
        StageRole::SystemMetaBefore,
        StageRole::SystemMetaAfter,
        StageRole::AgentLaneBefore,
        StageRole::AgentLaneAfter,
    ];
}

/// One optional prompt member of a journal: the single filename it may
/// target, how to reach it, and its two stage roles.
pub(super) struct PromptMember {
    /// The one name this member's `target` may hold.
    pub(super) target: &'static str,
    /// The member's place in a journal.
    pub(super) access: fn(&Journal) -> Option<&JournalEntry>,
    /// Stage role of the before-image.
    pub(super) before: StageRole,
    /// Stage role of the after-image.
    pub(super) after: StageRole,
}

/// The prompt members, **ignore file first**.
///
/// The order is the publication order and it is load-bearing: an artifact
/// that exists before its ignore line is an artifact a `git add -A` commits,
/// so the ignore file is written first and — in [`removals`] — removed last
/// (`PROP-076#SYS-ARTIFACTS-NOT-TRACKED`).
pub(super) const PROMPT_MEMBERS: [PromptMember; 4] = [
    PromptMember {
        target: vibe_core::layout::BOOT_GITIGNORE,
        access: |journal| journal.gitignore.as_ref(),
        before: StageRole::GitignoreBefore,
        after: StageRole::GitignoreAfter,
    },
    PromptMember {
        target: vibe_core::layout::SYSTEM_MD,
        access: |journal| journal.system.as_ref(),
        before: StageRole::SystemBefore,
        after: StageRole::SystemAfter,
    },
    PromptMember {
        target: vibe_core::layout::SYSTEM_META_JSON,
        access: |journal| journal.system_meta.as_ref(),
        before: StageRole::SystemMetaBefore,
        after: StageRole::SystemMetaAfter,
    },
    PromptMember {
        target: vibe_core::layout::AGENT_LANE_MD,
        access: |journal| journal.agent_lane.as_ref(),
        before: StageRole::AgentLaneBefore,
        after: StageRole::AgentLaneAfter,
    },
];

/// The prompt entries this journal actually carries, in ignore-first order.
pub(super) fn prompt_entries(journal: &Journal) -> impl Iterator<Item = &JournalEntry> {
    PROMPT_MEMBERS
        .iter()
        .filter_map(|member| (member.access)(journal))
}

/// The prompt entries whose after-image is **present** — this generation
/// writes them — in ignore-first order.
pub(super) fn writes(journal: &Journal) -> impl Iterator<Item = &JournalEntry> {
    prompt_entries(journal).filter(|entry| entry.after.present)
}

/// The prompt entries whose after-image is **absent** — this generation
/// removes them — in ignore-**last** order, so an artifact is never left
/// behind by an interrupted removal with its ignore line already gone.
pub(super) fn removals(journal: &Journal) -> impl Iterator<Item = &JournalEntry> {
    prompt_entries(journal)
        .filter(|entry| !entry.after.present)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}

/// Every entry a journal carries, prompt members included. The **one**
/// enumeration: `preflight_journal_paths`, `discard_assets` and the stage
/// uniqueness check all read it, so none of them can forget a role.
pub(super) fn entries(journal: &Journal) -> impl Iterator<Item = &JournalEntry> {
    [&journal.index, &journal.selected, &journal.stale]
        .into_iter()
        .chain(prompt_entries(journal))
}

/// Every recorded state a journal carries, derived from [`entries`].
pub(super) fn states(journal: &Journal) -> impl Iterator<Item = &RecordedState> {
    entries(journal).flat_map(|entry| [&entry.before, &entry.after])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_role_spelling_is_a_dash_prefixed_suffix_of_another() {
        for role in StageRole::ALL {
            for other in StageRole::ALL {
                if role == other {
                    continue;
                }
                assert!(
                    !other.wire().ends_with(&format!("-{}", role.wire())),
                    "`{}` would swallow `{}`",
                    other.wire(),
                    role.wire()
                );
            }
        }
    }

    #[test]
    fn prompt_members_are_four_distinct_targets_and_eight_distinct_roles() {
        let targets: std::collections::BTreeSet<_> =
            PROMPT_MEMBERS.iter().map(|member| member.target).collect();
        assert_eq!(targets.len(), PROMPT_MEMBERS.len());
        let roles: std::collections::BTreeSet<_> = PROMPT_MEMBERS
            .iter()
            .flat_map(|member| [member.before.wire(), member.after.wire()])
            .collect();
        assert_eq!(roles.len(), PROMPT_MEMBERS.len() * 2);
        // The ignore file leads, because its line must exist before the
        // artifact it covers.
        assert_eq!(PROMPT_MEMBERS[0].target, vibe_core::layout::BOOT_GITIGNORE);
    }
}
