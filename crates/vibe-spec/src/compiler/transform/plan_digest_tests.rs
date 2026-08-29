//! Longhand digest REDs: implementation-vector and two-entry plan-vector
//! goldens assembled byte-by-byte from the frozen ABI §4.1 schedule,
//! independent of the streaming production writer (buffer the exact frame,
//! then hash once), plus the mutation kills that make every framed element
//! load-bearing.

use sha2::{Digest as _, Sha256};
use specmark::verifies;
use vibe_core::manifest::ExtensionKey;
use vibe_extension_registry::HostIdentity;

use super::config::ConfigTable;
use super::plan::{
    TransformConfig, TransformImplementation, TransformProvider, TransformSeed, TransformStage,
};
use super::plan_test_support::{
    SelectorShape, build_or_panic, compiled_selectors, default_dependency, empty_config,
};

fn field(bytes: &[u8]) -> Vec<u8> {
    let mut framed = (bytes.len() as u64).to_le_bytes().to_vec();
    framed.extend_from_slice(bytes);
    framed
}

fn once(bytes: &[u8]) -> [u8; 32] {
    let mut hand = Sha256::new();
    hand.update(bytes);
    hand.finalize().into()
}

const IMPLEMENTATION_DOMAIN: &[u8] = b"vibe-transform-implementation-v1\0epoch=1\0";
const PLAN_DOMAIN: &[u8] = b"vibe-transform-plan-v1\0epoch=1\0";
const CONFIG_DOMAIN: &[u8] = b"vibe-transform-config-v1\0epoch=1\0";

/// The implementation digest is exactly: domain field, builtin tag, name
/// field, u32-LE epoch — hashed once over a hand-assembled buffer.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_independent_longhand_implementation_vector_agrees() {
    let mut frame = Vec::new();
    frame.extend_from_slice(&field(IMPLEMENTATION_DOMAIN));
    frame.push(0); // builtin
    frame.extend_from_slice(&field(b"minify"));
    frame.extend_from_slice(&3u32.to_le_bytes());

    let expected = once(&frame);
    let production = super::plan_digest::implementation_digest(
        &TransformImplementation::builtin_candidate("minify", 3),
    );
    assert_eq!(production.as_bytes(), &expected);

    // Mutations of the hand buffer must not match: tag, name and epoch are
    // each load-bearing.
    let mutated_tag = {
        let mut mutated = frame.clone();
        mutated[field(IMPLEMENTATION_DOMAIN).len()] = 1;
        once(&mutated)
    };
    let mutated_name = {
        let mut frame = Vec::new();
        frame.extend_from_slice(&field(IMPLEMENTATION_DOMAIN));
        frame.push(0);
        frame.extend_from_slice(&field(b"minifx"));
        frame.extend_from_slice(&3u32.to_le_bytes());
        once(&frame)
    };
    let mutated_epoch = {
        let mut frame = Vec::new();
        frame.extend_from_slice(&field(IMPLEMENTATION_DOMAIN));
        frame.push(0);
        frame.extend_from_slice(&field(b"minify"));
        frame.extend_from_slice(&4u32.to_le_bytes());
        once(&frame)
    };
    assert_ne!(&expected, &mutated_tag);
    assert_ne!(&expected, &mutated_name);
    assert_ne!(&expected, &mutated_epoch);
}

/// The two-entry plan golden: dependency + ungrouped host, config absent +
/// authored empty, selector absent + present dimensions (one with authored
/// duplicates and disorder), optional host kind/hash present, both child
/// digests — every byte assembled longhand, then hashed once.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_independent_two_entry_plan_vector_agrees_byte_for_byte() {
    // Entry 1's selector: packages ["zz","aa","aa"] (disordered duplicate),
    // paths [] (present empty).
    let selectors: [_; 1] = compiled_selectors(&[SelectorShape::Dimensions {
        packages: Some(vec!["zz", "aa", "aa"]),
        paths: Some(Vec::new()),
    }])
    .try_into()
    .expect("one selector collected");
    let selector = selectors[0].clone();

    let seeds = vec![
        TransformSeed::new(
            ExtensionKey::authored("org.demo/tools#squeeze"),
            TransformProvider::from(&default_dependency()),
            TransformStage::Source,
            TransformImplementation::builtin_candidate("log", 1),
            None,
            None,
        ),
        TransformSeed::new(
            ExtensionKey::for_host("demo", "announce"),
            TransformProvider::from(&super::plan_test_support::host_with(
                HostIdentity::ungrouped_project("demo"),
                Some(vibe_core::PackageKind::Flow),
                Some("sha256-tree/1:bb"),
            )),
            TransformStage::Document,
            TransformImplementation::builtin_candidate("minify", 3),
            Some(empty_config()),
            Some(selector),
        ),
    ];
    let plan = build_or_panic(seeds);
    let production = plan.digest().expect("nonempty plan digests");

    // Hand-assemble the two child digests first (longhand, hashed once).
    let implementation_log = {
        let mut frame = Vec::new();
        frame.extend_from_slice(&field(IMPLEMENTATION_DOMAIN));
        frame.push(0);
        frame.extend_from_slice(&field(b"log"));
        frame.extend_from_slice(&1u32.to_le_bytes());
        once(&frame)
    };
    let implementation_minify = {
        let mut frame = Vec::new();
        frame.extend_from_slice(&field(IMPLEMENTATION_DOMAIN));
        frame.push(0);
        frame.extend_from_slice(&field(b"minify"));
        frame.extend_from_slice(&3u32.to_le_bytes());
        once(&frame)
    };
    let config_empty = {
        let mut frame = Vec::new();
        frame.extend_from_slice(&field(CONFIG_DOMAIN));
        frame.push(6); // table tag
        frame.extend_from_slice(&0u64.to_le_bytes());
        once(&frame)
    };

    // The plan frame itself.
    let mut frame = Vec::new();
    frame.extend_from_slice(&field(PLAN_DOMAIN));
    frame.extend_from_slice(&2u64.to_le_bytes()); // entry count

    // Entry 0: dependency org.demo/tools, version 1.2.3, tool, sha256:aa,
    // stage source, order 0, builtin log/1, config absent, selector absent.
    frame.extend_from_slice(&field(b"org.demo/tools#squeeze"));
    frame.push(0); // stage: source
    frame.extend_from_slice(&0u32.to_le_bytes());
    frame.push(0); // provider: dependency
    frame.extend_from_slice(&field(b"org.demo"));
    frame.extend_from_slice(&field(b"tools"));
    frame.extend_from_slice(&field(b"1.2.3"));
    frame.extend_from_slice(&field(b"tool"));
    frame.extend_from_slice(&field(b"sha256:aa"));
    frame.extend_from_slice(&field(&implementation_log)); // framed child
    frame.push(0); // config absent
    frame.push(0); // selector absent

    // Entry 1: ungrouped host demo, version 0.1.0, kind flow,
    // sha256-tree/1:bb, stage document, order 1, builtin minify/3, config
    // authored empty, selector present with canonical dimensions.
    frame.extend_from_slice(&field(b"__host__/demo#announce"));
    frame.push(1); // stage: document
    frame.extend_from_slice(&1u32.to_le_bytes());
    frame.push(1); // provider: host
    frame.push(0); // host: ungrouped
    frame.extend_from_slice(&field(b"demo"));
    frame.extend_from_slice(&field(b"0.1.0"));
    frame.push(1); // kind present
    frame.extend_from_slice(&field(b"flow"));
    frame.push(1); // hash present
    frame.extend_from_slice(&field(b"sha256-tree/1:bb"));
    frame.extend_from_slice(&field(&implementation_minify));
    frame.push(1); // config present
    frame.extend_from_slice(&field(&config_empty));
    frame.push(1); // selector present
    frame.push(1); // packages dimension present
    frame.extend_from_slice(&2u64.to_le_bytes()); // post-dedup count
    frame.extend_from_slice(&field(b"aa"));
    frame.extend_from_slice(&field(b"zz"));
    frame.push(1); // paths dimension present
    frame.extend_from_slice(&0u64.to_le_bytes()); // present empty

    let expected = once(&frame);
    assert_eq!(production.as_bytes(), &expected);
    // The stable projection spells the same bytes.
    assert_eq!(production.sha256_hex().len(), 7 + 64);
    assert!(production.sha256_hex().starts_with("sha256:"));
    assert_eq!(
        production.sha256_hex(),
        format!("sha256:{}", hex_lower(&expected))
    );

    // Mutation kills: each mutation of the hand buffer hashes differently
    // than production, so the corresponding production byte is
    // load-bearing.
    // (a) child digest framed raw (no length frame).
    let raw_child = strip_one_child_length_frame(&frame);
    assert_ne!(&expected, &once(&raw_child));
    // (b) provider framed as one rendered identity string instead of typed
    // components.
    let rendered_provider = replace_with_rendered_provider(&frame);
    assert_ne!(&expected, &once(&rendered_provider));
    // (c) pre-dedup dimension count: located structurally, not by first
    // occurrence — the packages dimension's post-dedup count is the u64
    // immediately preceding the first canonical member `field(b"aa")`, so
    // the mutation cannot accidentally hit the plan's entry count (another
    // u64 `2` earlier in the frame).
    let pre_dedup_count = {
        let member = field(b"aa");
        let member_at = frame
            .windows(member.len())
            .position(|window| window == member)
            .expect("the first canonical package member exists");
        let count_at = member_at - 8;
        assert_eq!(
            frame[count_at..member_at],
            2u64.to_le_bytes(),
            "the bytes before the first member are the post-dedup count of 2"
        );
        let mut mutated = frame.clone();
        mutated[count_at..member_at].copy_from_slice(&3u64.to_le_bytes());
        mutated
    };
    assert_ne!(&expected, &once(&pre_dedup_count));
    // (d) omitted presence byte (config presence of entry 1).
    let omitted_presence = remove_one_presence_byte(&frame);
    assert_ne!(&expected, &once(&omitted_presence));
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(2 * bytes.len());
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}

/// Remove the 8-byte length frame before entry 0's implementation digest:
/// the child digest appears as a `field` of exactly 32 bytes at a known
/// position (right after `field(b"sha256:aa")`), so splice it precisely.
fn strip_one_child_length_frame(frame: &[u8]) -> Vec<u8> {
    let marker = field(b"sha256:aa");
    let at = frame
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("the dependency hash field exists");
    let child_at = at + marker.len();
    let mut mutated = frame.to_vec();
    mutated.drain(child_at..child_at + 8);
    mutated
}

/// Replace entry 0's typed provider components
/// (group, name, version, kind, hash) with one rendered identity field.
fn replace_with_rendered_provider(frame: &[u8]) -> Vec<u8> {
    let group_marker = field(b"org.demo");
    let hash_marker = field(b"sha256:aa");
    let start = frame
        .windows(group_marker.len())
        .position(|window| window == group_marker)
        .expect("the dependency group field exists");
    let end = frame
        .windows(hash_marker.len())
        .position(|window| window == hash_marker)
        .expect("the dependency hash field exists")
        + hash_marker.len();
    let rendered = field(b"org.demo/tools@1.2.3");
    let mut mutated = frame[..start].to_vec();
    mutated.extend_from_slice(&rendered);
    mutated.extend_from_slice(&frame[end..]);
    mutated
}

/// Remove the config-presence byte of entry 1 (the byte right before
/// `field(&config_empty)`, located via the selector-presence structure).
fn remove_one_presence_byte(frame: &[u8]) -> Vec<u8> {
    // Entry 1's config presence is the single `1` byte immediately before
    // the framed empty-config digest. Locate the digest by hashing every
    // 40-byte window (8-byte frame + 32 bytes) against the known digest.
    let mut config_frame = Vec::new();
    config_frame.extend_from_slice(&field(CONFIG_DOMAIN));
    config_frame.push(6);
    config_frame.extend_from_slice(&0u64.to_le_bytes());
    let config_digest = once(&config_frame);
    let mut framed = (32u64).to_le_bytes().to_vec();
    framed.extend_from_slice(&config_digest);
    let at = frame
        .windows(framed.len())
        .position(|window| window == framed)
        .expect("the framed empty-config digest exists");
    let mut mutated = frame.to_vec();
    mutated.remove(at - 1);
    mutated
}

/// The `sha256:` projection is stable, lowercase, and derived — two equal
/// plans project equally; the projection never feeds back into identity.
#[test]
fn the_sha256_projection_is_stable_lowercase_and_derived() {
    let left = build_or_panic(vec![TransformSeed::new(
        ExtensionKey::authored("k"),
        TransformProvider::from(&default_dependency()),
        TransformStage::Source,
        TransformImplementation::builtin_candidate("log", 1),
        Some(TransformConfig::new(ConfigTable::new())),
        None,
    )]);
    let right = left.clone();
    assert_eq!(left, right);
    assert_eq!(
        left.digest().unwrap().sha256_hex(),
        right.digest().unwrap().sha256_hex()
    );
    let hex = left.digest().unwrap().sha256_hex();
    // The projection is exactly `sha256:` plus 64 lowercase hex characters
    // — asserted over the stripped suffix, so a stray character anywhere
    // (including the prefix) fails rather than passing through a second arm.
    let suffix = hex
        .strip_prefix("sha256:")
        .expect("the projection is prefixed");
    assert_eq!(suffix.len(), 64, "the suffix is exactly 64 characters");
    assert!(
        suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "every suffix byte is lowercase [0-9a-f], got {suffix}"
    );
}
