//! Pure-function tests for the shell git backend — the inline tar
//! extractor and the locale-stable stderr classifier. Out-of-line per
//! the file-length budget; needs neither live git nor the fixtures.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend");

use specmark::verifies;

use super::tar::parse_octal;
use super::*;

/// Run the stderr classifier with placeholder URL/ref — the wrapper the
/// fixtures module carried before these tests moved out of it.
#[cfg(test)]
fn classify(stderr: &str) -> Option<GitError> {
    classify_stderr_message(stderr, "URL".into(), "REF".into())
}

/// Build a USTar archive from `(name, bytes)` pairs. Plenty for our
/// tests; not a complete tar implementation. Moved out of the fixtures
/// module with the extractor tests it feeds.
#[cfg(test)]
fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in entries {
        let mut header = vec![0u8; 512];
        // Name: bytes 0..100, NUL-terminated.
        let nb = name.as_bytes();
        let len = nb.len().min(100);
        header[0..len].copy_from_slice(&nb[..len]);
        // Mode: bytes 100..108 — "0000644\0".
        header[100..108].copy_from_slice(b"0000644\0");
        // UID/GID: bytes 108..116 / 116..124 — "0000000\0".
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        // Size: bytes 124..136 — octal, 11 chars + NUL.
        let size_str = format!("{:011o}\0", data.len());
        header[124..136].copy_from_slice(size_str.as_bytes());
        // Mtime: bytes 136..148 — "00000000000\0".
        header[136..148].copy_from_slice(b"00000000000\0");
        // Checksum: bytes 148..156 — fill with spaces, compute later.
        for b in &mut header[148..156] {
            *b = b' ';
        }
        // Typeflag: byte 156 — '0' (regular file).
        header[156] = b'0';
        // Magic: bytes 257..263 — "ustar\0".
        header[257..263].copy_from_slice(b"ustar\0");
        // Version: bytes 263..265 — "00".
        header[263..265].copy_from_slice(b"00");
        // Compute checksum: sum of all bytes treating chksum field
        // as spaces (already done above).
        let cksum: u32 = header.iter().map(|b| *b as u32).sum();
        let cksum_str = format!("{cksum:06o}\0 ");
        header[148..156].copy_from_slice(cksum_str.as_bytes());

        out.extend_from_slice(&header);
        out.extend_from_slice(data);
        // Pad to 512.
        let pad = (512 - (data.len() % 512)) % 512;
        out.extend(std::iter::repeat_n(0, pad));
    }
    // Two empty 512-byte blocks to terminate.
    out.extend(std::iter::repeat_n(0, 1024));
    out
}

#[test]
fn extract_single_file_from_tar_picks_match() {
    // Hand-build a minimal tar with two files; verify we extract the
    // requested one by name, ignoring the other.
    let tar = build_tar(&[("a.txt", b"AAA\n"), ("vibe.toml", b"hello world\n")]);
    let got = extract_single_file_from_tar(&tar, "vibe.toml").expect("file extracted");
    assert_eq!(got, b"hello world\n");

    let absent = extract_single_file_from_tar(&tar, "nope.txt");
    assert!(absent.is_none());
}

#[test]
fn extract_single_file_from_tar_handles_dot_slash_prefix() {
    let tar = build_tar(&[("./vibe.toml", b"prefixed\n")]);
    let got = extract_single_file_from_tar(&tar, "vibe.toml").unwrap();
    assert_eq!(got, b"prefixed\n");
}

#[test]
fn parse_octal_handles_padded_sizes() {
    assert_eq!(parse_octal(b"00000000027\0").unwrap(), 0o27);
    assert_eq!(parse_octal(b"  144 \0").unwrap(), 0o144);
    assert_eq!(parse_octal(b"\0\0\0\0\0\0\0\0\0\0\0\0").unwrap(), 0);
}

#[test]
fn classify_repo_not_found_message() {
    // GitHub / Gitea-shape 404 surfaces a verbatim "Repository not
    // found." line from the remote helper; that's the substring
    // the classifier locks onto.
    assert!(matches!(
        classify("remote: Repository not found.\nfatal: ...").unwrap(),
        GitError::RepoNotFound { .. }
    ));
    assert!(matches!(
        classify("fatal: 'x' does not appear to be a git repository").unwrap(),
        GitError::RepoNotFound { .. }
    ));
}

#[test]
fn classify_auth_failed_message() {
    assert!(matches!(
        classify("git@github.com: Permission denied (publickey).").unwrap(),
        GitError::AuthFailed { .. }
    ));
    assert!(matches!(
        classify("remote: HTTP Basic: Access denied\nfatal: Authentication failed for ...")
            .unwrap(),
        GitError::AuthFailed { .. }
    ));
}

#[test]
fn classify_credential_prompt_failure_after_silencing() {
    // PROP-002 §2.2.1: when our credential helpers are silenced
    // (`-c credential.helper=` + `GIT_TERMINAL_PROMPT=0`), git
    // can't ask anyone for a username/password and emits
    // `fatal: could not read Username for '...'`. This is what
    // the original opencode walk against GitVerse produced.
    // Must classify as AuthFailed so the resolver can apply the
    // per-`auth` walk-vs-halt rule (§2.3.1).
    for stderr in [
        "fatal: User cancelled dialog.\nfatal: could not read Username for 'https://gitverse.ru': terminal prompts disabled",
        "fatal: could not read Password for 'https://example.invalid': terminal prompts disabled",
    ] {
        assert!(
            matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
            "expected AuthFailed for: {stderr}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn force_silence_wins_over_tty_and_env() {
    // PROP-002 §2.2.1: a public (`auth = "none"`) backend forces the
    // silencing layer on. The `force_silence` branch returns before any
    // env / TTY probe, so this is deterministic — it does not read the
    // process environment, and it holds even on an interactive terminal.
    assert!(should_silence_credential_helpers(true));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn forced_apply_common_env_shuts_every_interactive_channel() {
    // With `force_silence`, every channel git could use to prompt for
    // credentials is closed on the spawned invocation (PROP-002 §2.2.1):
    // empty `-c credential.helper=` and `-c core.askPass=` on the argv,
    // `GCM_INTERACTIVE=Never` in the env, plus the always-on
    // `GIT_TERMINAL_PROMPT=0`. So a public-host 401 returns immediately
    // as an error instead of blocking on a GCM popup.
    let mut cmd = Command::new("git");
    apply_common_env(&mut cmd, true);

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == "credential.helper="),
        "expected `-c credential.helper=` on argv; got {args:?}"
    );
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == "core.askPass="),
        "expected `-c core.askPass=` on argv; got {args:?}"
    );

    let envs: std::collections::HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert_eq!(
        envs.get("GCM_INTERACTIVE"),
        Some(&Some("Never".to_string()))
    );
    assert_eq!(
        envs.get("GIT_TERMINAL_PROMPT"),
        Some(&Some("0".to_string()))
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn shellgit_yields_an_anonymous_public_variant() {
    // A source `ShellGit` is interactive by default and hands out an
    // anonymous-posture variant for public registries — the backend
    // `MultiRegistryResolver::from_manifest` wires to every
    // `auth = "none"` registry (PROP-002 §2.2.1).
    let base = ShellGit::new();
    assert!(
        !base.force_anonymous,
        "the default backend stays interactive"
    );
    assert!(
        base.anonymized_for_public().is_some(),
        "ShellGit must expose an anonymous variant for public registries"
    );
}

#[test]
fn classify_http_status_codes() {
    // Direct HTTP transport errors — when the host returns a
    // structured response without redirecting through the
    // credential layer (some proxies, some CI runners).
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 401 Unauthorized",
        "fatal: unable to access 'https://x/y.git/': HTTP 401",
        "fatal: unable to access 'https://x/y.git/': The requested URL returned error: 403 Forbidden",
        "fatal: unable to access 'https://x/y.git/': HTTP 403",
    ] {
        assert!(
            matches!(classify(stderr).unwrap(), GitError::AuthFailed { .. }),
            "expected AuthFailed for: {stderr}"
        );
    }
}

#[test]
fn classify_network_unreachable_classic_substrings() {
    // Substrings that the M1.1 classifier already recognised.
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': Could not resolve host: x",
        "fatal: Could not read from remote repository.",
        "ssh: connect to host x port 22: Network is unreachable",
    ] {
        assert!(
            matches!(
                classify(stderr).unwrap(),
                GitError::NetworkUnreachable { .. }
            ),
            "expected NetworkUnreachable for: {stderr}"
        );
    }
}

#[test]
fn classify_network_unreachable_connect_failure_substrings() {
    // The shapes M1.6 Scenario B4 surfaced — connect-failure on a
    // dead host. git 2.5x with libcurl 8.x emits the verbatim
    // strings below; the Scenario B4 walk on 2026-05-04 produced
    // the third one (`Could not connect to server`).
    for stderr in [
        "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443 after 2123 ms: Could not connect to server",
        "fatal: unable to access 'https://x/y.git/': Failed to connect to x port 443: Connection refused",
        "fatal: unable to access 'https://x/y.git/': Connection timed out after 30001 ms",
        "fatal: unable to access 'https://x/y.git/': Operation timed out after 30001 ms",
    ] {
        assert!(
            matches!(
                classify(stderr).unwrap(),
                GitError::NetworkUnreachable { .. }
            ),
            "expected NetworkUnreachable for: {stderr}"
        );
    }
}

#[test]
fn classify_ref_not_found_message() {
    assert!(matches!(
        classify("fatal: Remote branch no-such-branch not found in upstream origin").unwrap(),
        GitError::RefNotFound { .. }
    ));
    assert!(matches!(
        classify("fatal: couldn't find remote ref refs/tags/v9.9.9").unwrap(),
        GitError::RefNotFound { .. }
    ));
}

#[test]
fn classify_unknown_message_falls_through() {
    assert!(classify("error: something we have never seen before").is_none());
}

/// A failed [`Output`] carrying `stderr`, so the classifier can be driven
/// without a live `git`. `ExitStatus` has no portable constructor; both
/// platform extensions offer a safe `from_raw`.
#[cfg(test)]
fn failed_output(stderr: &str) -> Output {
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(128)
    };
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(128)
    };
    Output {
        status,
        stdout: Vec::new(),
        stderr: stderr.as_bytes().to_vec(),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn without_userinfo_strips_the_credential_and_nothing_else() {
    // The shape `credentialed_url` / `inject_token` produce.
    assert_eq!(
        without_userinfo("https://x-access-token:SECRET@github.com/o/r.git"),
        "https://github.com/o/r.git"
    );
    assert_eq!(
        without_userinfo("https://SECRET@gitverse.ru/o/r.git"),
        "https://gitverse.ru/o/r.git"
    );
    // No path to keep.
    assert_eq!(
        without_userinfo("https://x-access-token:SECRET@github.com"),
        "https://github.com"
    );
    // An ssh URL's userinfo goes too — it could hold a password, and the
    // account name is not information an error needs.
    assert_eq!(
        without_userinfo("ssh://git@host.example/org/repo.git"),
        "ssh://host.example/org/repo.git"
    );
    // Nothing to strip: left exactly as git received it. The scp form in
    // particular must keep its `git@`, or the URL stops being the URL.
    for url in [
        "https://github.com/o/r.git",
        "git@github.com:vibespecs/org.vibevm.wal.git",
        "file:///tmp/registry/org.vibevm.wal",
        "",
    ] {
        assert_eq!(without_userinfo(url), url, "must pass through: {url}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn without_userinfo_in_arg_reaches_a_url_behind_a_flag() {
    // `git archive` carries the URL inside `--remote=`; `git clone`
    // carries it bare after `--`.
    assert_eq!(
        without_userinfo_in_arg("--remote=https://x-access-token:SECRET@github.com/o/r.git"),
        "--remote=https://github.com/o/r.git"
    );
    assert_eq!(
        without_userinfo_in_arg("https://x-access-token:SECRET@github.com/o/r.git"),
        "https://github.com/o/r.git"
    );
    // Every other argv element is left alone, including the ones that
    // happen to carry an `=` or an `@`.
    for arg in [
        "--format=tar",
        "credential.helper=",
        "core.askPass=",
        "+refs/tags/v1.0.0:refs/tags/v1.0.0",
        "--no-tags",
        "vibe.toml",
    ] {
        assert_eq!(
            without_userinfo_in_arg(arg),
            arg,
            "must pass through: {arg}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn a_classified_failure_names_the_url_without_its_credential() {
    // The URL git is given carries the token; the error git's failure
    // produces must not. Both the classified variants (which carry the
    // URL in a field) and the unclassified fallback (which carries the
    // whole argv) go through the same scrub.
    let args = [
        "clone",
        "--",
        "https://x-access-token:SECRET@github.com/o/r.git",
    ];

    let classified = classify_failure(&args, &failed_output("remote: Repository not found.\n"));
    assert!(
        matches!(classified, GitError::RepoNotFound { .. }),
        "expected RepoNotFound, got: {classified:?}"
    );
    let shown = classified.to_string();
    assert!(!shown.contains("SECRET"), "credential leaked: {shown}");
    assert!(
        shown.contains("https://github.com/o/r.git"),
        "the plain URL must still be named: {shown}"
    );

    let unclassified = classify_failure(&args, &failed_output("error: something new\n"));
    assert!(
        matches!(unclassified, GitError::CommandFailed { .. }),
        "expected CommandFailed, got: {unclassified:?}"
    );
    let shown = unclassified.to_string();
    assert!(
        !shown.contains("SECRET"),
        "credential leaked through the rendered argv: {shown}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn a_rendered_argv_is_safe_to_log() {
    // The argv is rendered into `Io.cmd`, into `CommandFailed.cmd`, and
    // into the `running git` debug line — one renderer, so all three are
    // covered at once.
    let args = [
        "archive",
        "--remote=https://x-access-token:SECRET@github.com/o/r.git",
        "--format=tar",
        "v1.0.0",
        "--",
        "vibe.toml",
    ];
    for rendered in [
        render_argv(Path::new("git"), &args),
        render_argv_for_display(&args),
    ] {
        assert!(
            !rendered.contains("SECRET"),
            "credential leaked: {rendered}"
        );
        assert!(
            rendered.contains("--remote=https://github.com/o/r.git"),
            "the plain URL must still be shown: {rendered}"
        );
        assert!(
            rendered.contains("--format=tar"),
            "argv mangled: {rendered}"
        );
        assert!(rendered.contains("vibe.toml"), "argv mangled: {rendered}");
    }
}

#[test]
fn classify_specific_matchers_win_over_unable_to_access() {
    // `unable to access` is a wrapper that frames many other
    // failures; it must NOT shadow the inner connect-failure or
    // auth-failed classification.
    let stderr =
        "fatal: unable to access 'https://x/y.git/': Authentication failed for 'https://x/'";
    assert!(matches!(
        classify(stderr).unwrap(),
        GitError::AuthFailed { .. }
    ));
}
