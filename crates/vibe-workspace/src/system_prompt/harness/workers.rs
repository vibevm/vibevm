//! The flag that hands a launch's composed lane to every subagent it
//! starts, so same-type workers of one wave share one prefix
//! (`#SYS-WORKER-PREFIX-DELIVERY`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY");

use std::path::Path;

use specmark::spec;

use super::HarnessKind;

/// The flags that append the composed lane file to every subagent a launch
/// starts. `lane` is the same file the parent launch names, so the workers'
/// prefix is never a second composition. Empty for every kind with no such
/// flag.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY")]
pub fn worker_prefix_flags(kind: HarnessKind, lane: &Path) -> Vec<String> {
    match kind {
        HarnessKind::Claude => vec![
            "--append-subagent-system-prompt-file".to_owned(),
            lane.display().to_string(),
        ],
        HarnessKind::Codex
        | HarnessKind::OpenCode
        | HarnessKind::Qwen
        | HarnessKind::Hermes
        | HarnessKind::Pi => Vec::new(),
    }
}
