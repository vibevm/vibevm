//! Read, never computed — and absent, never zero.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CACHE-FACTS",
    r = 1
);

use super::*;

const ANTHROPIC: &str = r#"{"usage":{"input_tokens":1200,
    "cache_creation_input_tokens":278000,"cache_read_input_tokens":0,
    "output_tokens":42}}"#;

const OPENAI: &str = r#"{"id":"chatcmpl-1","usage":{"prompt_tokens":279200,
    "completion_tokens":9,"cached_input_tokens":278000,
    "cache_write_input_tokens":0}}"#;

const OPENAI_CHAT_DETAILS: &str = r#"{"usage":{"prompt_tokens":279200,
    "prompt_tokens_details":{"cached_tokens":278000}}}"#;

#[test]
fn an_anthropic_record_yields_all_three_counters_it_reports() {
    let usage = ProviderUsage::read(ANTHROPIC).expect("the record parses");
    let diagnostics = CacheDiagnostics::from_usage(&usage).expect("a cache counter is present");
    assert_eq!(diagnostics.source(), "anthropic-messages-usage");
    assert_eq!(diagnostics.write_tokens(), Some(278_000));
    assert_eq!(diagnostics.read_tokens(), Some(0), "zero IS a measurement");
    // `input_tokens` already excludes the cached part, so it is the
    // provider's own uncached count rather than a derivation.
    assert_eq!(diagnostics.uncached_tokens(), Some(1_200));
}

#[test]
fn an_openai_compatible_record_reports_no_uncached_count_and_none_is_invented() {
    for body in [OPENAI, OPENAI_CHAT_DETAILS] {
        let usage = ProviderUsage::read(body).expect("the record parses");
        let diagnostics = CacheDiagnostics::from_usage(&usage).expect("a cache counter is present");
        assert_eq!(diagnostics.source(), "openai-compatible-usage");
        assert_eq!(diagnostics.read_tokens(), Some(278_000));
        // `prompt_tokens` carries the cached part INSIDE it, so an uncached
        // figure here could only be `279200 - 278000` — an arithmetic
        // result in a field whose whole contract is that a provider said
        // it. It stays absent.
        assert_eq!(
            diagnostics.uncached_tokens(),
            None,
            "a subtraction was published as a reading: {body}"
        );
    }
}

#[test]
fn a_provider_that_reports_no_caching_yields_no_diagnostics_rather_than_zeros() {
    // Three zeros would read as a measured miss. Nothing was measured.
    let usage = ProviderUsage::read(r#"{"usage":{"prompt_tokens":51,"completion_tokens":9}}"#)
        .expect("the record parses");
    assert_eq!(CacheDiagnostics::from_usage(&usage), None);
    // What the caller needs to read is which numbers their provider DID
    // send, not a restatement that the cache ones are missing.
    assert_eq!(
        usage.counter_names(),
        ["completion_tokens", "prompt_tokens"]
    );
}

#[test]
fn a_response_with_no_usage_object_is_refused() {
    let error = ProviderUsage::read(r#"{"choices":[{"message":{"content":"hi"}}]}"#)
        .expect_err("nothing to read");
    assert!(error.to_string().contains("no `usage` object"), "{error:#}");
    assert!(error.to_string().contains("never computed"), "{error:#}");
}

#[test]
fn a_response_that_does_not_parse_is_refused_naming_that() {
    let error = ProviderUsage::read("not json at all").expect_err("unparseable");
    assert!(error.to_string().contains("does not parse"), "{error:#}");
}

#[test]
fn the_usage_object_is_found_one_level_down_as_well() {
    let usage = ProviderUsage::read(
        r#"{"response":{"usage":{"cache_read_input_tokens":7,"input_tokens":3}}}"#,
    )
    .expect("the record parses");
    assert_eq!(
        CacheDiagnostics::from_usage(&usage)
            .expect("a cache counter is present")
            .read_tokens(),
        Some(7)
    );
}
