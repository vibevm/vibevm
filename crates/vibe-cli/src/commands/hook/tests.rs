//! The shape of the managed block each shell gets
//! (PROP-075 `#ACTIVATION-HOOK`). Running it is `cli_hook_linux.rs`; this
//! pins what is generated, including the properties that make the cost
//! budget checkable by reading.

use super::*;

fn names() -> Vec<String> {
    vec![
        ".nvmrc".to_owned(),
        ".node-version".to_owned(),
        ".tool-versions".to_owned(),
    ]
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn every_block_bakes_the_name_list_and_calls_vibe_at_most_once() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh] {
        let body = block_body(shell, &names());
        for name in names() {
            assert!(body.contains(&name), "{shell:?} must bake `{name}`: {body}");
        }
        // Exactly one `vibe app env` call site, so the budget of "at most
        // one `vibe` invocation on a `cd`" is a property of the text.
        assert_eq!(
            body.matches("vibe app env --project --shell").count(),
            1,
            "{shell:?}: {body}"
        );
        // And it is always the `--project` form, which does no network, no
        // registry call and no index-server read.
        assert!(!body.contains("vibe install"), "{shell:?}: {body}");
        assert!(!body.contains("app use"), "{shell:?}: {body}");
        // The remembered directory is what makes an unchanged prompt free.
        assert!(body.contains("__vibe_version_dir") || body.contains("VibeVersionDir"));
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn each_shell_registers_its_own_prompt_mechanism() {
    assert!(block_body(Shell::Bash, &names()).contains("PROMPT_COMMAND"));
    assert!(block_body(Shell::Zsh, &names()).contains("precmd_functions"));
    assert!(block_body(Shell::Fish, &names()).contains("--on-variable PWD"));
    let pwsh = block_body(Shell::Pwsh, &names());
    assert!(pwsh.contains("function global:prompt"), "{pwsh}");
    assert!(pwsh.contains("Invoke-Expression"), "{pwsh}");
}

#[test]
fn a_name_with_a_quote_is_quoted_for_its_shell() {
    let odd = vec!["it's".to_owned()];
    assert!(block_body(Shell::Bash, &odd).contains("'it'\\''s'"));
    assert!(block_body(Shell::Fish, &odd).contains("'it\\'s'"));
    assert!(block_body(Shell::Pwsh, &odd).contains("'it''s'"));
}

#[test]
fn a_home_that_declares_no_version_file_still_gets_a_well_formed_block() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Pwsh] {
        let body = block_body(shell, &[]);
        assert!(!body.is_empty(), "{shell:?}");
        assert_eq!(body.matches("vibe app env --project --shell").count(), 1);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_startup_file_is_the_one_each_shell_reads() {
    let home = std::path::Path::new("/home/dev");
    assert_eq!(rc_path(Shell::Bash, home), home.join(".bashrc"));
    assert_eq!(rc_path(Shell::Zsh, home), home.join(".zshrc"));
    assert_eq!(
        rc_path(Shell::Fish, home),
        home.join(".config/fish/config.fish")
    );
    // PowerShell's profile is not `.profile`.
    let profile = rc_path(Shell::Pwsh, home);
    assert!(
        profile.ends_with("Microsoft.PowerShell_profile.ps1"),
        "{}",
        profile.display()
    );
}

#[test]
fn the_hook_block_markers_are_its_own() {
    assert_ne!(HOOK_BLOCK.begin, vibe_app::ENV_BLOCK.begin);
    assert_ne!(HOOK_BLOCK.end, vibe_app::ENV_BLOCK.end);
}
