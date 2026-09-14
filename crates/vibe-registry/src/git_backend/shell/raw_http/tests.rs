//! Pure tests for the HTTPS read fast path — which repository URLs it
//! addresses and where, when a miss may be believed, and how each host's
//! body is read. No network and no `git`: every function under test here
//! is a decision made before a socket is opened, or after the bytes are
//! already in hand.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf");

use specmark::verifies;

use super::*;

/// The read URL a repository URL maps to, with production bases.
fn url_for(repo_url: &str, refname: &str, path: &str) -> Option<String> {
    address(None, repo_url, refname, path).map(|r| r.url)
}

/// The token a read would carry in its `Authorization` header.
fn token_for(repo_url: &str) -> Option<String> {
    address(None, repo_url, "v1.0.0", "vibe.toml").and_then(|r| r.token)
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn github_https_repositories_map_to_the_raw_host() {
    // With and without the `.git` suffix, with and without userinfo, and
    // with a trailing slash — all the same file.
    let expect = "https://raw.githubusercontent.com/vibespecs/org.vibevm.wal/v1.0.0/vibe.toml";
    for repo_url in [
        "https://github.com/vibespecs/org.vibevm.wal.git",
        "https://github.com/vibespecs/org.vibevm.wal",
        "https://github.com/vibespecs/org.vibevm.wal/",
        "https://x@github.com/vibespecs/org.vibevm.wal.git",
        "https://x-access-token:ghp_secret@github.com/vibespecs/org.vibevm.wal.git",
    ] {
        assert_eq!(
            url_for(repo_url, "v1.0.0", "vibe.toml").as_deref(),
            Some(expect),
            "unexpected mapping for {repo_url}"
        );
    }
}

#[test]
fn github_takes_branches_and_shas_verbatim() {
    // The raw host resolves whatever ref shape it is given; a branch with
    // a slash in it stays one path tail, not an escaped segment.
    assert_eq!(
        url_for("https://github.com/o/r.git", "feature/x", "vibe.toml").as_deref(),
        Some("https://raw.githubusercontent.com/o/r/feature/x/vibe.toml")
    );
    let sha = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(
        url_for("https://github.com/o/r.git", sha, "docs/vibe.toml"),
        Some(format!(
            "https://raw.githubusercontent.com/o/r/{sha}/docs/vibe.toml"
        ))
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn gitverse_https_repositories_map_to_the_contents_api() {
    assert_eq!(
        url_for(
            "https://gitverse.ru/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml"
        )
        .as_deref(),
        Some(
            "https://gitverse.ru/api/repos/vibespecs/org.vibevm.wal/contents/vibe.toml?ref=v1.0.0"
        )
    );
    // The ref is a query value here, so a branch needs no special
    // handling; a nested path keeps its separators as real segments.
    assert_eq!(
        url_for("https://gitverse.ru/o/r", "feature/x", "docs/vibe.toml").as_deref(),
        Some("https://gitverse.ru/api/repos/o/r/contents/docs/vibe.toml?ref=feature%2Fx")
    );
}

#[test]
fn ssh_http_and_unknown_hosts_are_not_fast_path() {
    for repo_url in [
        // ssh, in both spellings — nothing to read over HTTPS.
        "git@github.com:vibespecs/org.vibevm.wal.git",
        "ssh://git@github.com/vibespecs/org.vibevm.wal.git",
        "git@gitverse.ru:vibespecs/org.vibevm.wal.git",
        "ssh://git@gitverse.ru/vibespecs/org.vibevm.wal.git",
        // plaintext http — a token must never cross it.
        "http://github.com/o/r.git",
        // hosts with no row in the table.
        "https://gitlab.com/o/r.git",
        "https://example.invalid/o/r.git",
        // a host that merely ends in one we know.
        "https://notgithub.com/o/r.git",
        "https://evil-github.com/o/r.git",
        // file, and the `git+` transport wrapper.
        "file:///tmp/registry/org.vibevm.wal",
        "git+https://github.com/o/r.git",
    ] {
        assert_eq!(
            url_for(repo_url, "v1.0.0", "vibe.toml"),
            None,
            "{repo_url} must fall back to git"
        );
    }
}

#[test]
fn only_owner_and_repository_paths_are_addressable() {
    for repo_url in [
        // too few and too many path segments.
        "https://github.com/only-owner",
        "https://github.com/o/r/extra",
        "https://gitverse.ru/o/r/tree/main",
        // empty halves.
        "https://github.com//r.git",
        "https://github.com/o/.git",
        // a query or fragment means this is not a plain repository URL.
        "https://github.com/o/r.git?x=1",
        "https://github.com/o/r.git#frag",
    ] {
        assert_eq!(
            url_for(repo_url, "v1.0.0", "vibe.toml"),
            None,
            "{repo_url} must fall back to git"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn a_token_is_taken_from_userinfo_and_never_left_in_the_url() {
    // The credentialed shape `inject_token` produces: the secret is the
    // half after the colon.
    assert_eq!(
        token_for("https://x-access-token:ghp_secret@github.com/o/r.git").as_deref(),
        Some("ghp_secret")
    );
    // A bare `<token>@host` form carries the whole userinfo.
    assert_eq!(
        token_for("https://ghp_bare@gitverse.ru/o/r.git").as_deref(),
        Some("ghp_bare")
    );
    // No userinfo, no token; an empty one is no token either.
    assert_eq!(token_for("https://github.com/o/r.git"), None);
    assert_eq!(token_for("https://@github.com/o/r.git"), None);

    // Whatever the userinfo held, the read URL and the URL named in
    // errors are both free of it.
    let read = address(
        None,
        "https://x-access-token:ghp_secret@github.com/o/r.git",
        "v1.0.0",
        "vibe.toml",
    )
    .expect("a credentialed github URL is addressable");
    assert!(!read.url.contains("ghp_secret"), "read URL: {}", read.url);
    assert!(!read.url.contains('@'), "read URL: {}", read.url);
    assert_eq!(read.plain_url, "https://github.com/o/r.git");
}

#[test]
fn refs_and_paths_that_could_steer_the_url_fall_back_to_git() {
    for (refname, path) in [
        // traversal, in either position.
        ("v1.0.0", "../../etc/passwd"),
        ("../../../other", "vibe.toml"),
        ("v1.0.0", "./vibe.toml"),
        // query and fragment injection.
        ("v1.0.0", "vibe.toml?ref=main"),
        ("v1.0.0#x", "vibe.toml"),
        ("v1.0.0", "vibe.toml#x"),
        // absolute, empty and malformed shapes.
        ("v1.0.0", "/vibe.toml"),
        ("v1.0.0", ""),
        ("", "vibe.toml"),
        ("v1.0.0", "a//vibe.toml"),
        // anything with a space or a control character in it.
        ("v1 0", "vibe.toml"),
        ("v1.0.0", "vibe file.toml"),
    ] {
        assert_eq!(
            url_for("https://github.com/o/r.git", refname, path),
            None,
            "ref {refname:?} + path {path:?} must fall back to git"
        );
    }
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator",
    r = 1
)]
fn a_miss_is_believed_only_for_a_non_manifest_file_at_a_fixed_ref() {
    let sha = "0123456789abcdef0123456789abcdef01234567";

    // The ordinary case this path exists for: no redirect marker, at a
    // release tag or an exact commit. Answerable without git.
    assert!(absence_is_authoritative("v1.0.0", "vibe-redirect.toml"));
    assert!(absence_is_authoritative(
        "v0.1.0-rc.1",
        "vibe-redirect.toml"
    ));
    assert!(absence_is_authoritative(sha, "vibe-redirect.toml"));

    // The manifest never is: only git can say whether this was a missing
    // file, a missing ref, a missing repository, or a repository we are
    // not allowed to see. Including when it sits in a subdirectory.
    assert!(!absence_is_authoritative("v1.0.0", "vibe.toml"));
    assert!(!absence_is_authoritative(sha, "nested/vibe.toml"));

    // A moving ref never is either — a branch tip is not evidence about
    // a file, and a mistyped branch misses for its own reasons.
    for refname in [
        "main",
        "feature/x",
        "v1.0.0-not-semver-after-all+",
        "release",
        "0123456789abcdef",                         // short SHA
        "0123456789abcdef0123456789abcdef0123456z", // 40 chars, not hex
    ] {
        assert!(
            !absence_is_authoritative(refname, "vibe-redirect.toml"),
            "{refname} is not a fixed ref"
        );
    }
}

#[test]
fn github_serves_the_file_itself() {
    let github = &HOSTS[0];
    assert_eq!(github.host, "github.com");
    assert_eq!((github.read_body)(b"[package]\n").unwrap(), b"[package]\n");
    assert!((github.is_miss)(404, b""));
    for status in [400, 401, 403, 500, 502] {
        assert!(
            !(github.is_miss)(status, b""),
            "{status} is not a github miss"
        );
    }
}

#[test]
fn gitverse_serves_base64_json_and_reports_a_miss_in_the_body() {
    let gitverse = &HOSTS[1];
    assert_eq!(gitverse.host, "gitverse.ru");

    // 200: base64 in `content`, wrapped across lines as the route sends it.
    let body =
        br#"{"encoding":"base64","content":"W3BhY2thZ2Vd\nCg==","size":11,"path":"vibe.toml"}"#;
    assert_eq!((gitverse.read_body)(body).unwrap(), b"[package]\n");

    // 200 with a shape this reader does not know → ask git, don't guess.
    for unknown in [
        &br#"{"encoding":"utf-8","content":"[package]"}"#[..],
        // the route answers an array for a directory.
        &br#"[{"path":"vibe.toml"}]"#[..],
        &br#"{"content":"W3BhY2thZ2Vd"}"#[..],
        &b"not json at all"[..],
    ] {
        assert!(
            (gitverse.read_body)(unknown).is_none(),
            "unreadable body must fall back to git: {}",
            String::from_utf8_lossy(unknown)
        );
    }

    // A miss is 400 + code 4305, and nothing else is.
    assert!((gitverse.is_miss)(
        400,
        br#"{"code":4305,"message":"File: vibe.toml not found"}"#
    ));
    assert!(!(gitverse.is_miss)(
        400,
        br#"{"code":4004,"message":"doesn't have repository"}"#
    ));
    assert!(!(gitverse.is_miss)(400, b"{}"));
    assert!(!(gitverse.is_miss)(404, br#"{"code":4305}"#));
    for status in [401, 403, 500, 502] {
        assert!(
            !(gitverse.is_miss)(status, br#"{"code":4305}"#),
            "{status} is not a gitverse miss"
        );
    }
}

#[test]
fn decode_base64_handles_padding_and_line_wrapping() {
    assert_eq!(decode_base64("").unwrap(), b"");
    assert_eq!(decode_base64("YQ==").unwrap(), b"a");
    assert_eq!(decode_base64("YWI=").unwrap(), b"ab");
    assert_eq!(decode_base64("YWJj").unwrap(), b"abc");
    assert_eq!(
        decode_base64("W3BhY2thZ2Vd\r\nCg==").unwrap(),
        b"[package]\n"
    );
    assert!(decode_base64("YWJ").is_err());
    assert!(decode_base64("YQ*=").is_err());
}
