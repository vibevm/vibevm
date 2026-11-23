//! The one spelling of the system-prompt summary (PROP-076
//! `#SYS-OPERATIONS-PATH-AND-FLAGS`, PROP-067 `#block`, `#trailer`).

use super::*;
use std::path::PathBuf;
use vibe_workspace::boot_artifacts::PromptPaths;

const WIDTH: usize = 100;

fn paths(root: &std::path::Path) -> PromptPaths {
    let boot = root.join(vibe_core::layout::current_boot_dir());
    PromptPaths {
        system: boot.join(vibe_core::layout::SYSTEM_MD),
        system_len: 5,
        lane: boot.join(vibe_core::layout::AGENT_LANE_MD),
        lane_len: 7,
    }
}

#[test]
fn system_prompt_summary_has_one_spelling() {
    let root = tempfile::tempdir().unwrap();
    let built = PromptSummary::of_build(&Ok(PromptBuild::Rebuilt(paths(root.path()))));
    assert_eq!(
        built.row_at(WIDTH),
        "✓ system prompt  rebuilt · SYSTEM.md 5 bytes · AGENT-LANE.md 7 bytes"
    );
    assert_eq!(
        built.text(),
        "system prompt rebuilt · SYSTEM.md 5 bytes · AGENT-LANE.md 7 bytes"
    );

    // The snapshot an install takes says the same words about the same bytes.
    let before = Snapshot::of(root.path());
    let boot = root.path().join(vibe_core::layout::current_boot_dir());
    std::fs::create_dir_all(&boot).unwrap();
    std::fs::write(boot.join(vibe_core::layout::SYSTEM_MD), "12345").unwrap();
    std::fs::write(boot.join(vibe_core::layout::AGENT_LANE_MD), "1234567").unwrap();
    let installed = before.summary(root.path()).expect("a prefix appeared");
    assert_eq!(installed.row_at(WIDTH), built.row_at(WIDTH));
    assert_eq!(installed.boot_dir, Some(boot.clone()));

    let unchanged = Snapshot::of(root.path()).summary(root.path()).unwrap();
    let rebuilt_unchanged =
        PromptSummary::of_build(&Ok(PromptBuild::Unchanged(paths(root.path()))));
    assert_eq!(unchanged.row_at(WIDTH), rebuilt_unchanged.row_at(WIDTH));
    assert_eq!(
        unchanged.row_at(WIDTH),
        "✓ system prompt  unchanged · vibevm/vibespecs/boot/SYSTEM.md"
    );

    let opted_out = PromptSummary::of_build(&Ok(PromptBuild::OptedOut));
    assert_eq!(
        opted_out.row_at(WIDTH),
        "○ system prompt  not opted in · no vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md"
    );
    assert_eq!(opted_out.boot_dir, None::<PathBuf>);
}
