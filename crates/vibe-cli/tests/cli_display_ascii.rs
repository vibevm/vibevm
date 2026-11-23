//! A dumb terminal draws the ASCII glyph set (PROP-067 `#plain`): the
//! gutter glyphs of a refusal and the rules of a listing.

mod common;

use common::UserScratch;

fn dumb(user: &UserScratch, home: &std::path::Path, args: &[&str]) -> std::process::Output {
    user.vibe()
        .current_dir(home)
        .env("HOME", home)
        .env("TERM", "dumb")
        .env_remove("VIBEVM_HOME")
        .env_remove("NO_COLOR")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn ascii_glyphs_on_a_dumb_terminal_reach_stderr_and_stdout() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let refused = dumb(&user, home.path(), &["agent", "remove", "nope"]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    let line = stderr
        .lines()
        .find(|line| line.contains("agent_unknown"))
        .unwrap_or_else(|| panic!("no refusal line in {stderr}"));
    assert!(line.starts_with("x agent_unknown  "), "{stderr}");

    let listed = dumb(&user, home.path(), &["agent", "list"]);
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let stdout = String::from_utf8_lossy(&listed.stdout);
    assert!(
        stdout.lines().any(|line| line.starts_with("-- launchers ")),
        "{stdout}"
    );
    assert!(!stdout.contains('─') && !stdout.contains('✻'), "{stdout}");
}
