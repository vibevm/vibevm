//! `vibe prompt flags` beside its flags line (PROP-067 `#inline`, `#refusal`).

use super::*;

#[test]
fn prompt_flags_notes_are_dim_hints() {
    let text = hints(
        &["`--workers`: this launch passes no lane, so no worker flag is printed".to_owned()],
        "claude",
        "launch",
        false,
    );
    assert_eq!(
        text,
        "  --workers: this launch passes no lane, so no worker flag is printed"
    );
}

#[test]
fn prompt_flags_with_nothing_to_pass_say_why() {
    assert_eq!(
        hints(&[], "pi", "read", true),
        "  pi reads its boot files itself · delivery read · no flags to pass"
    );
    assert_eq!(hints(&[], "claude", "launch", false), "");
}
