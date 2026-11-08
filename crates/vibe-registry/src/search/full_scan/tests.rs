//! Existing full-scan pure controls, relocated without changed assertions.

use super::requests::decode_base64;
use super::*;

#[test]
fn detect_github_org_extracts_first_path_segment() {
    assert_eq!(
        detect_github_org("https://github.com/vibespecs"),
        Some("vibespecs".into())
    );
    assert_eq!(
        detect_github_org("https://github.com/vibespecs/"),
        Some("vibespecs".into())
    );
    assert_eq!(
        detect_github_org("https://github.com/vibespecs/extra/path"),
        Some("vibespecs".into())
    );
}

#[test]
fn detect_github_org_returns_none_for_non_github_hosts() {
    assert!(detect_github_org("https://gitverse.ru/anarchic").is_none());
    assert!(detect_github_org("https://gitlab.com/foo").is_none());
}

#[test]
fn detect_github_org_returns_none_when_org_segment_is_missing() {
    assert!(detect_github_org("https://github.com").is_none());
    assert!(detect_github_org("https://github.com/").is_none());
}

#[test]
fn parse_next_link_extracts_next_url() {
    let header = r#"<https://api.github.com/orgs/x/repos?page=2>; rel="next", <https://api.github.com/orgs/x/repos?page=5>; rel="last""#;
    assert_eq!(
        parse_next_link(header).as_deref(),
        Some("https://api.github.com/orgs/x/repos?page=2")
    );
}

#[test]
fn parse_next_link_returns_none_when_no_next_rel() {
    let header = r#"<https://api.github.com/orgs/x/repos?page=5>; rel="last""#;
    assert!(parse_next_link(header).is_none());
}

#[test]
fn decode_base64_roundtrips_simple_input() {
    let body = b"hello world";
    // standard base64 of "hello world" is "aGVsbG8gd29ybGQ="
    let encoded = "aGVsbG8gd29ybGQ=";
    let decoded = decode_base64(encoded).unwrap();
    assert_eq!(decoded, body);
}

#[test]
fn decode_base64_handles_double_pad() {
    let body = b"any";
    let encoded = "YW55"; // 3 bytes => no pad
    assert_eq!(decode_base64(encoded).unwrap(), body);
    let body2 = b"a";
    let encoded2 = "YQ=="; // 1 byte => double pad
    assert_eq!(decode_base64(encoded2).unwrap(), body2);
}

/// Manual encoder mirrors the integration-test fixture so that a
/// regression in either side surfaces locally instead of in
/// tests/cli_search.rs.
fn encode_base64_for_test(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if i + 1 < input.len() {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

#[test]
fn decode_base64_round_trips_arbitrary_byte_ranges() {
    for body in [
        b"".to_vec(),
        b"a".to_vec(),
        b"hi".to_vec(),
        b"abc".to_vec(),
        b"hello world".to_vec(),
        b"vibe.toml".to_vec(),
        (0..=255u8).collect::<Vec<u8>>(),
    ] {
        let encoded = encode_base64_for_test(&body);
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, body, "round-trip failed for {body:?}");
    }
}
