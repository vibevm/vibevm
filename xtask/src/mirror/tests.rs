use super::{check_tail, normalize_url, push_args, remotes_matching};

fn remote(name: &str, url: &str) -> (String, String) {
    (name.to_string(), url.to_string())
}

#[test]
fn push_args_never_force() {
    // The marquee invariant of the whole mirror system (PROP-016 §4,
    // CLAUDE.md force-push rule): the fan-out NEVER force-pushes. Guarding every
    // ref shape keeps a future edit from quietly slipping `--force` in.
    for git_ref in ["main", "tags", "release", "v1.0"] {
        let args = push_args("git@host:org/repo.git", git_ref);
        assert!(
            !args
                .iter()
                .any(|a| a == "--force" || a == "-f" || a.starts_with('+')),
            "fan-out push for `{git_ref}` must never force: {args:?}"
        );
        assert_eq!(args[0], "push", "first arg is always the push verb");
    }
}

#[test]
fn push_args_shape_per_ref_kind() {
    // A branch ref pushes that branch by name; `tags` fans every tag.
    assert_eq!(
        push_args("URL", "main"),
        vec!["push", "URL", "refs/heads/main:refs/heads/main"]
    );
    assert_eq!(push_args("URL", "tags"), vec!["push", "URL", "--tags"]);
}

#[test]
fn normalize_url_strips_git_suffix_and_trailing_slash() {
    assert_eq!(
        normalize_url("git@github.com:vibevm/vibevm.git"),
        "git@github.com:vibevm/vibevm"
    );
    assert_eq!(
        normalize_url("https://gitverse.ru/vibevm/vibevm.git/"),
        "https://gitverse.ru/vibevm/vibevm"
    );
    // Already bare — unchanged, and the leading `git@` is never touched
    // (only the tail is trimmed).
    assert_eq!(
        normalize_url("git@gitverse.ru:vibevm/vibevm"),
        "git@gitverse.ru:vibevm/vibevm"
    );
}

#[test]
fn matching_remote_found_despite_git_suffix_difference() {
    // mirrors.toml carries the `.git` form; a remote may not, or vice
    // versa — normalisation makes the two compare equal.
    let remotes = vec![
        remote("origin", "git@gitverse.ru:vibevm/vibevm.git"),
        remote("github", "git@github.com:vibevm/vibevm"),
    ];
    assert_eq!(
        remotes_matching(&remotes, "git@gitverse.ru:vibevm/vibevm"),
        vec!["origin"]
    );
    assert_eq!(
        remotes_matching(&remotes, "git@github.com:vibevm/vibevm.git"),
        vec!["github"]
    );
}

#[test]
fn no_matching_remote_when_url_is_unknown() {
    // A target with no configured remote (the push-by-URL-only case):
    // nothing to refresh, no spurious match.
    let remotes = vec![remote("origin", "git@gitverse.ru:vibevm/vibevm.git")];
    assert!(remotes_matching(&remotes, "git@example.com:someone/other.git").is_empty());
}

#[test]
fn both_remotes_at_same_url_match_in_order() {
    // Two remotes pointing at one host: both tracking refs must move,
    // and the input order is preserved.
    let remotes = vec![
        remote("origin", "git@gitverse.ru:vibevm/vibevm.git"),
        remote("alias", "git@gitverse.ru:vibevm/vibevm.git"),
    ];
    assert_eq!(
        remotes_matching(&remotes, "git@gitverse.ru:vibevm/vibevm.git"),
        vec!["origin", "alias"]
    );
}

#[test]
fn check_tail_all_sync_still_says_so() {
    // The one state that earns the green line.
    assert_eq!(
        check_tail(&[], &[]).unwrap(),
        "mirror --check: all targets in sync."
    );
}

#[test]
fn check_tail_behind_only_names_the_count_and_stays_green() {
    // B-090: Behind alone keeps exit 0, but the tail names it — "all
    // targets in sync" over BEHIND lines was the lie this fix removes.
    let behind = vec!["gitverse", "github"];
    assert_eq!(
        check_tail(&[], &behind).unwrap(),
        "mirror --check: local ahead -- 2 target(s) behind, fast-forward needed (push with \
             cargo xtask mirror)."
    );
}

#[test]
fn check_tail_drift_without_behind_is_the_unchanged_red_verdict() {
    assert_eq!(
        check_tail(&["hub-a"], &[]).unwrap_err(),
        "mirror --check: 1 target(s) drifted from mainline: hub-a"
    );
}

#[test]
fn check_tail_mixed_drift_and_behind_names_both_counts() {
    // Drift is the verdict, but a riding-along Behind is named too — the
    // red tail may not silently drop a class the lines above reported.
    let verdict = check_tail(&["hub-a"], &["hub-b", "hub-c"]).unwrap_err();
    assert!(
        verdict.starts_with("mirror --check: 1 target(s) drifted from mainline: hub-a;"),
        "drift leads the verdict: {verdict}"
    );
    assert!(
        verdict.contains("2 target(s) behind (fast-forward catches up): hub-b, hub-c"),
        "behind class and count named: {verdict}"
    );
}

#[test]
fn check_tail_never_claims_in_sync_over_behind_or_drift() {
    // The invariant the whole fix exists for, swept over every non-green
    // class mix: no tail may assert what the per-target lines deny.
    let empty: Vec<&str> = Vec::new();
    let drifted = vec!["hub-a"];
    let lagging = vec!["hub-b", "hub-c"];
    for (drift, behind) in [
        (drifted.as_slice(), empty.as_slice()),
        (empty.as_slice(), lagging.as_slice()),
        (drifted.as_slice(), lagging.as_slice()),
    ] {
        let tail = match check_tail(drift, behind) {
            Ok(t) => t,
            Err(v) => v,
        };
        assert!(
            !tail.contains("in sync"),
            "tail over drift/behind must not claim sync: {tail}"
        );
    }
}
